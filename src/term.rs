//! Terminal capabilities, decided the way the Python version's libraries did.
//!
//! Two libraries wrote to the terminal in the Python version and they did not
//! agree with each other, so both rule sets are kept:
//!
//! * the tables followed rich: colour when stdout is a terminal (or when
//!   `TTY_COMPATIBLE` / `FORCE_COLOR` say so), nothing at all on `TERM=dumb`,
//!   and `NO_COLOR` drops colours but keeps bold and italic;
//! * the one-line messages followed click: colour only when stdout is a
//!   terminal, with no environment overrides.

use std::env;
use std::io::IsTerminal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
}

impl Color {
    fn code(self) -> u8 {
        match self {
            Color::Red => 31,
            Color::Green => 32,
            Color::Yellow => 33,
            Color::Blue => 34,
            Color::Magenta => 35,
            Color::Cyan => 36,
        }
    }
}

/// A text style. Only the attributes this tool uses exist.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub color: Option<Color>,
}

impl Style {
    pub const NONE: Style = Style {
        bold: false,
        italic: false,
        color: None,
    };

    pub fn fg(color: Color) -> Style {
        Style {
            color: Some(color),
            ..Style::NONE
        }
    }

    pub fn bold() -> Style {
        Style {
            bold: true,
            ..Style::NONE
        }
    }

    pub fn italic() -> Style {
        Style {
            italic: true,
            ..Style::NONE
        }
    }

    /// The SGR parameter list, in the order rich writes them.
    fn sgr(self, with_color: bool) -> String {
        let mut codes: Vec<String> = Vec::new();
        if self.bold {
            codes.push("1".into());
        }
        if self.italic {
            codes.push("3".into());
        }
        if with_color {
            if let Some(c) = self.color {
                codes.push(c.code().to_string());
            }
        }
        codes.join(";")
    }
}

#[derive(Clone, Debug)]
pub struct Term {
    /// rich would emit escape sequences at all.
    rich_ansi: bool,
    /// rich's `NO_COLOR` handling: keep attributes, drop colours.
    rich_no_color: bool,
    /// click would keep the colour of `click.style` text.
    click_ansi: bool,
    /// The same, for text written to stderr.
    click_ansi_err: bool,
    /// The width rich lays tables out against.
    pub width: usize,
}

fn env_nonempty(name: &str) -> Option<String> {
    env::var(name).ok()
}

fn is_ascii_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

impl Term {
    pub fn detect() -> Term {
        let stdout_tty = std::io::stdout().is_terminal();

        let rich_terminal = match env_nonempty("TTY_COMPATIBLE").as_deref() {
            Some("0") => false,
            Some("1") => true,
            _ => match env::var("FORCE_COLOR") {
                Ok(v) => !v.is_empty(),
                Err(_) => stdout_tty,
            },
        };
        let term = env::var("TERM").unwrap_or_default().to_lowercase();
        let dumb = rich_terminal && (term == "dumb" || term == "unknown");
        let rich_no_color = env::var("NO_COLOR").map(|v| !v.is_empty()).unwrap_or(false);

        let columns = env::var("COLUMNS").ok().filter(|v| is_ascii_digits(v));
        let lines = env::var("LINES").ok().filter(|v| is_ascii_digits(v));
        let width = match (&columns, &lines) {
            // rich stores both at start-up and then never looks at the
            // terminal again.
            (Some(c), Some(_)) => c.parse().unwrap_or(80),
            _ if dumb => 80,
            _ => {
                let mut w = terminal_width().unwrap_or(0);
                if let Some(c) = &columns {
                    w = c.parse().unwrap_or(w);
                }
                if w == 0 { 80 } else { w }
            }
        };

        Term {
            rich_ansi: rich_terminal && !dumb,
            rich_no_color,
            click_ansi: stdout_tty,
            click_ansi_err: std::io::stderr().is_terminal(),
            width,
        }
    }

    /// rich-styled text.
    pub fn rich(&self, text: &str, style: Style) -> String {
        if !self.rich_ansi || text.is_empty() {
            return text.to_string();
        }
        let sgr = style.sgr(!self.rich_no_color);
        if sgr.is_empty() {
            text.to_string()
        } else {
            format!("\x1b[{sgr}m{text}\x1b[0m")
        }
    }

    /// click-styled text (`click.style(text, fg=...)` then `click.echo`).
    pub fn click(&self, text: &str, color: Color) -> String {
        self.click_to(text, color, self.click_ansi)
    }

    /// click-styled text for stderr.
    pub fn click_err(&self, text: &str, color: Color) -> String {
        self.click_to(text, color, self.click_ansi_err)
    }

    fn click_to(&self, text: &str, color: Color, ansi: bool) -> String {
        if ansi {
            format!("\x1b[{}m{text}\x1b[0m", color.code())
        } else {
            text.to_string()
        }
    }
}

/// rich asks stdin, stdout and stderr in turn on Unix, and only the two
/// output streams on Windows.
fn terminal_width() -> Option<usize> {
    use terminal_size::{Width, terminal_size_of};
    #[cfg(unix)]
    let sizes = [
        terminal_size_of(std::io::stdin()),
        terminal_size_of(std::io::stdout()),
        terminal_size_of(std::io::stderr()),
    ];
    #[cfg(windows)]
    let sizes = [
        terminal_size_of(std::io::stdout()),
        terminal_size_of(std::io::stderr()),
    ];
    sizes
        .into_iter()
        .flatten()
        .next()
        .map(|(Width(w), _)| w as usize)
}
