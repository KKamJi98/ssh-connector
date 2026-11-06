//! A table renderer that reproduces the subset of rich's `Table` the Python
//! version used: a `DOUBLE_EDGE` box, a centred title, a bold header row,
//! one space of padding on each side of every cell, and the column width and
//! overflow rules of rich 15 (`_calculate_column_widths`, `divide_line`,
//! `set_cell_size`).
//!
//! Cell text is always taken literally. rich parsed it as markup, which was a
//! bug (a host such as `a[/b]` crashed the Python version).

use crate::term::{Style, Term};
use unicode_width::UnicodeWidthChar;

pub struct Column {
    pub header: &'static str,
    pub style: Style,
    pub no_wrap: bool,
}

pub struct Table {
    pub title: String,
    pub title_style: Style,
    pub border_style: Style,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
}

const ELLIPSIS: char = '\u{2026}';

fn char_width(c: char) -> usize {
    c.width().unwrap_or(0)
}

pub fn cell_len(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

/// rich `set_cell_size`: crop or pad to exactly `total` cells. A wide
/// character cut in half becomes a space.
fn set_cell_size(s: &str, total: usize) -> String {
    let mut out = String::new();
    let mut size = 0;
    for c in s.chars() {
        let w = char_width(c);
        if size + w > total {
            if size < total {
                out.push(' ');
                size += 1;
            }
            break;
        }
        out.push(c);
        size += w;
    }
    while size < total {
        out.push(' ');
        size += 1;
    }
    out
}

/// rich `Text.truncate` with `overflow="ellipsis"`, `pad=True`.
fn fit_ellipsis(s: &str, width: usize) -> String {
    let len = cell_len(s);
    if len > width {
        if width == 0 {
            return String::new();
        }
        let mut t = set_cell_size(s, width - 1);
        t.push(ELLIPSIS);
        t
    } else {
        let mut t = s.to_string();
        t.push_str(&" ".repeat(width - len));
        t
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Measurement {
    min: usize,
    max: usize,
}

impl Measurement {
    fn normalize(self) -> Measurement {
        let min = self.min.min(self.max);
        Measurement {
            min,
            max: min.max(self.max),
        }
    }

    fn with_maximum(self, width: usize) -> Measurement {
        Measurement {
            min: self.min.min(width),
            max: self.max.min(width),
        }
    }
}

/// rich `Text.__rich_measure__` for single-line text.
fn measure_text(s: &str) -> Measurement {
    let max = cell_len(s);
    let min = s.split_whitespace().map(cell_len).max().unwrap_or(max);
    Measurement { min, max }
}

/// `Measurement.get` of a cell wrapped in `Padding(0, 1)`.
fn measure_cell(s: &str, max_width: usize) -> Measurement {
    if max_width < 1 {
        return Measurement { min: 0, max: 0 };
    }
    // Padding.__rich_measure__
    let padded = if max_width < 3 {
        Measurement {
            min: max_width,
            max: max_width,
        }
    } else {
        let inner_width = max_width - 2;
        let inner = measure_text(s).normalize().with_maximum(inner_width);
        let inner = if inner.max < 1 {
            Measurement { min: 0, max: 0 }
        } else {
            inner.normalize()
        };
        Measurement {
            min: inner.min + 2,
            max: inner.max + 2,
        }
        .with_maximum(max_width)
    };
    let m = padded.normalize().with_maximum(max_width);
    if m.max < 1 {
        Measurement { min: 0, max: 0 }
    } else {
        m.normalize()
    }
}

/// rich `ratio_reduce`.
fn ratio_reduce(
    total: usize,
    ratios: &[usize],
    maximums: &[usize],
    values: &[usize],
) -> Vec<usize> {
    let ratios: Vec<usize> = ratios
        .iter()
        .zip(maximums)
        .map(|(&r, &m)| if m > 0 { r } else { 0 })
        .collect();
    let mut total_ratio: usize = ratios.iter().sum();
    if total_ratio == 0 {
        return values.to_vec();
    }
    let mut total_remaining = total as f64;
    let mut result = Vec::with_capacity(values.len());
    for ((&ratio, &maximum), &value) in ratios.iter().zip(maximums).zip(values) {
        if ratio > 0 && total_ratio > 0 {
            let share = py_round(ratio as f64 * total_remaining / total_ratio as f64);
            let distributed = (maximum as f64).min(share);
            result.push((value as f64 - distributed) as isize as usize);
            total_remaining -= distributed;
            total_ratio -= ratio;
        } else {
            result.push(value);
        }
    }
    result
}

/// Python's `round()` (banker's rounding).
fn py_round(x: f64) -> f64 {
    let r = x.round();
    if (x - x.trunc()).abs() == 0.5 {
        2.0 * (x / 2.0).round()
    } else {
        r
    }
}

/// rich `Table._collapse_widths`.
fn collapse_widths(mut widths: Vec<usize>, wrapable: &[bool], max_width: usize) -> Vec<usize> {
    let mut total: usize = widths.iter().sum();
    let mut excess = total as isize - max_width as isize;
    if wrapable.iter().any(|&w| w) {
        while total > 0 && excess > 0 {
            let max_column = widths
                .iter()
                .zip(wrapable)
                .filter(|(_, w)| **w)
                .map(|(&x, _)| x)
                .max()
                .unwrap_or(0);
            let second_max = widths
                .iter()
                .zip(wrapable)
                .map(|(&x, &w)| if w && x != max_column { x } else { 0 })
                .max()
                .unwrap_or(0);
            let difference = max_column - second_max;
            let ratios: Vec<usize> = widths
                .iter()
                .zip(wrapable)
                .map(|(&x, &w)| usize::from(x == max_column && w))
                .collect();
            if ratios.iter().all(|&r| r == 0) || difference == 0 {
                break;
            }
            let max_reduce = vec![(excess as usize).min(difference); widths.len()];
            widths = ratio_reduce(excess as usize, &ratios, &max_reduce, &widths);
            total = widths.iter().sum();
            excess = total as isize - max_width as isize;
        }
    }
    widths
}

impl Table {
    fn cells(&self, col: usize) -> impl Iterator<Item = &str> {
        std::iter::once(self.columns[col].header)
            .chain(self.rows.iter().map(move |r| r[col].as_str()))
    }

    fn measure_column(&self, col: usize, max_width: usize) -> Measurement {
        if max_width < 1 {
            return Measurement { min: 0, max: 0 };
        }
        let ms: Vec<Measurement> = self
            .cells(col)
            .map(|c| measure_cell(c, max_width))
            .collect();
        Measurement {
            min: ms.iter().map(|m| m.min).max().unwrap_or(1),
            max: ms.iter().map(|m| m.max).max().unwrap_or(max_width),
        }
        .with_maximum(max_width)
    }

    /// rich `Table._calculate_column_widths` (no expand, no fixed widths).
    fn column_widths(&self, max_width: usize) -> Vec<usize> {
        let n = self.columns.len();
        let ranges: Vec<Measurement> = (0..n).map(|i| self.measure_column(i, max_width)).collect();
        let mut widths: Vec<usize> = ranges
            .iter()
            .map(|r| if r.max == 0 { 1 } else { r.max })
            .collect();
        let table_width: usize = widths.iter().sum();
        if table_width > max_width {
            let wrapable: Vec<bool> = self.columns.iter().map(|c| !c.no_wrap).collect();
            widths = collapse_widths(widths, &wrapable, max_width);
            let table_width: usize = widths.iter().sum();
            if table_width > max_width {
                let excess = table_width - max_width;
                widths = ratio_reduce(excess, &vec![1; n], &widths.clone(), &widths);
            }
            widths = widths
                .iter()
                .enumerate()
                .map(|(i, &w)| self.measure_column(i, w).max)
                .collect();
        }
        widths
    }

    /// Render the table to lines (without trailing newlines).
    pub fn render(&self, term: &Term) -> Vec<String> {
        let n = self.columns.len();
        let extra_width = 2 + (n - 1);
        let max_width = term.width.saturating_sub(extra_width);
        let widths = self.column_widths(max_width);
        let table_width = widths.iter().sum::<usize>() + extra_width;

        let mut out = Vec::new();
        for line in wrap_center(&self.title, table_width) {
            out.push(render_title_line(
                &line,
                table_width,
                self.title_style,
                term,
            ));
        }

        let border = |s: String| term.rich(&s, self.border_style);
        let rule = |left: char, fill: char, mid: char, right: char| {
            let mut s = String::new();
            s.push(left);
            for (i, &w) in widths.iter().enumerate() {
                if i > 0 {
                    s.push(mid);
                }
                s.extend(std::iter::repeat_n(fill, w));
            }
            s.push(right);
            s
        };
        let row = |cells: Vec<(&str, Style)>| {
            let mut s = border("\u{2551}".into());
            for (i, ((text, style), &w)) in cells.into_iter().zip(&widths).enumerate() {
                if i > 0 {
                    s.push_str(&border("\u{2502}".into()));
                }
                s.push_str(&render_cell(text, w, style, term));
            }
            s.push_str(&border("\u{2551}".into()));
            s
        };

        out.push(border(rule('\u{2554}', '\u{2550}', '\u{2564}', '\u{2557}')));
        out.push(row(self
            .columns
            .iter()
            .map(|c| (c.header, Style::bold()))
            .collect()));
        out.push(border(rule('\u{255f}', '\u{2500}', '\u{253c}', '\u{2562}')));
        for r in &self.rows {
            out.push(row(r
                .iter()
                .zip(&self.columns)
                .map(|(t, c)| (t.as_str(), c.style))
                .collect()));
        }
        out.push(border(rule('\u{255a}', '\u{2550}', '\u{2567}', '\u{255d}')));
        out
    }
}

/// One cell: left pad, content padded or cut to the inner width, right pad.
/// Each part is its own styled segment, as rich writes it.
fn render_cell(text: &str, width: usize, style: Style, term: &Term) -> String {
    if width < 3 {
        // Padding no longer fits; rich gives the whole width to the padding.
        return term.rich(&" ".repeat(width), style);
    }
    let inner = width - 2;
    let content = fit_ellipsis(text, inner);
    let mut s = term.rich(" ", style);
    s.push_str(&term.rich(&content, style));
    s.push_str(&term.rich(" ", style));
    s
}

fn render_title_line(line: &str, width: usize, title_style: Style, term: &Term) -> String {
    let len = cell_len(line);
    let left = width.saturating_sub(len) / 2;
    let right = width.saturating_sub(len + left);
    let pad_style = Style::italic();
    let text_style = Style {
        italic: true,
        ..title_style
    };
    let mut s = String::new();
    if left > 0 {
        s.push_str(&term.rich(&" ".repeat(left), pad_style));
    }
    s.push_str(&term.rich(line, text_style));
    if right > 0 {
        s.push_str(&term.rich(&" ".repeat(right), pad_style));
    }
    s
}

/// rich `chop_cells`.
fn chop_cells(s: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    let mut size = 0;
    for c in s.chars() {
        let w = char_width(c);
        if size + w > width && !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
            size = 0;
        }
        cur.push(c);
        size += w;
    }
    if size > 0 || !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// Words as rich's `re_word` (`\s*\S+\s*`) finds them, with byte offsets.
fn words(s: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut pos = 0;
    let bytes_len = s.len();
    while pos < bytes_len {
        let rest = &s[pos..];
        let lead = rest.len() - rest.trim_start().len();
        let after_lead = &rest[lead..];
        if after_lead.is_empty() {
            break;
        }
        let word_len = after_lead
            .find(char::is_whitespace)
            .unwrap_or(after_lead.len());
        let after_word = &after_lead[word_len..];
        let trail = after_word.len() - after_word.trim_start().len();
        let end = pos + lead + word_len + trail;
        out.push((pos, &s[pos..end]));
        pos = end;
    }
    out
}

/// rich `divide_line` with `fold=True`, returning byte offsets.
fn divide_line(s: &str, width: usize) -> Vec<usize> {
    let mut breaks = Vec::new();
    let mut cell_offset = 0usize;
    for (start, word) in words(s) {
        let word_length = cell_len(word.trim_end());
        let remaining = width as isize - cell_offset as isize;
        if remaining >= word_length as isize {
            cell_offset += cell_len(word);
        } else if word_length > width {
            let folded = chop_cells(word, width);
            let mut start = start;
            let last_index = folded.len().saturating_sub(1);
            for (i, line) in folded.iter().enumerate() {
                if start > 0 {
                    breaks.push(start);
                }
                if i == last_index {
                    cell_offset = cell_len(line);
                } else {
                    start += line.len();
                }
            }
        } else if cell_offset > 0 && start > 0 {
            breaks.push(start);
            cell_offset = cell_len(word);
        }
    }
    breaks
}

/// rich's title rendering: wrap with `overflow="fold"`, then centre each
/// line (`Lines.justify("center")` strips trailing whitespace first).
fn wrap_center(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut prev = 0;
    for b in divide_line(text, width)
        .into_iter()
        .chain(std::iter::once(text.len()))
    {
        let mut piece = text[prev..b].to_string();
        // Text.rstrip_end(width): drop trailing whitespace beyond width.
        let len = piece.chars().count();
        if len > width {
            let ws = piece.len() - piece.trim_end().len();
            let excess = len - width;
            let crop = ws.min(excess);
            for _ in 0..crop {
                piece.pop();
            }
        }
        let piece = piece.trim_end().to_string();
        let piece = if cell_len(&piece) > width {
            set_cell_size(&piece, width)
        } else {
            piece
        };
        lines.push(piece);
        prev = b;
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ellipsis_cuts_to_width() {
        assert_eq!(fit_ellipsis("abcdef", 4), "abc\u{2026}");
        assert_eq!(fit_ellipsis("ab", 4), "ab  ");
    }

    #[test]
    fn wide_char_cut_in_half_becomes_space() {
        assert_eq!(set_cell_size("\u{c11c}\u{bc84}", 3), "\u{c11c} ");
    }

    #[test]
    fn title_folds_long_words() {
        assert_eq!(
            wrap_center("abcdefghij (1)", 6),
            vec!["abcdef", "ghij", "(1)"]
        );
    }

    #[test]
    fn python_round_is_bankers() {
        assert_eq!(py_round(0.5), 0.0);
        assert_eq!(py_round(1.5), 2.0);
        assert_eq!(py_round(2.5), 2.0);
    }
}
