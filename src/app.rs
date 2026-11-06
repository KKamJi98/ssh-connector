//! The interactive menu: show the tables, read a choice, run ssh.

use crate::config::{self, Groups};
use crate::table::{Column, Table};
use crate::term::{Color, Style, Term};
use std::io::{self, BufRead, Write};
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, Instant};

const PROMPT: &str =
    "Enter the number or name of the host to connect to (or 'q' to quit, 'f' to filter): ";
const FILTER_PROMPT: &str = "Enter filter term: ";

/// What a Ctrl-C means right now.
const MODE_OTHER: u8 = 0;
const MODE_PROMPT: u8 = 1;
const MODE_CHILD: u8 = 2;

static MODE: AtomicU8 = AtomicU8::new(MODE_OTHER);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Exit the process with `code` after a write error on stdout. A closed pipe
/// ends the program quietly with 1, as click did.
pub struct Exit(pub i32);

type Res<T> = Result<T, Exit>;

fn out_err(_: io::Error) -> Exit {
    Exit(1)
}

/// Install the Ctrl-C handler. While ssh runs it only takes note (ssh gets
/// the same Ctrl-C and decides for itself); anywhere else it ends the program
/// with 130, the shell's code for "interrupted", after moving to a new line.
///
/// The handler must not touch stdout: the main thread holds its lock for the
/// whole run. Everything written so far has already been flushed, because
/// the only place the main thread waits is after a flush.
pub fn install_interrupt_handler() {
    let _ = ctrlc::set_handler(|| match MODE.load(Ordering::SeqCst) {
        MODE_CHILD => INTERRUPTED.store(true, Ordering::SeqCst),
        _ => {
            let _ = writeln!(io::stderr());
            std::process::exit(INTERRUPTED_EXIT);
        }
    });
}

/// 128 + SIGINT.
const INTERRUPTED_EXIT: i32 = 130;

struct Out<'a> {
    term: &'a Term,
    w: io::StdoutLock<'static>,
}

impl Out<'_> {
    fn line(&mut self, s: &str) -> Res<()> {
        writeln!(self.w, "{s}").map_err(out_err)
    }

    fn click(&mut self, s: &str, color: Color) -> Res<()> {
        let styled = self.term.click(s, color);
        self.line(&styled)?;
        self.w.flush().map_err(out_err)
    }

    fn flush(&mut self) -> Res<()> {
        self.w.flush().map_err(out_err)
    }
}

/// Read one line for a prompt. Empty answers ask again. `None` is the end of
/// input (Ctrl-D), which quits like `q`.
fn prompt(out: &mut Out, text: &str) -> Res<Option<String>> {
    loop {
        write!(out.w, "{text}").map_err(out_err)?;
        out.flush()?;
        MODE.store(MODE_PROMPT, Ordering::SeqCst);
        let mut buf = String::new();
        let n = io::stdin().lock().read_line(&mut buf);
        MODE.store(MODE_OTHER, Ordering::SeqCst);
        match n {
            Ok(0) => return Ok(None),
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error: could not read input: {e}");
                return Err(Exit(1));
            }
        }
        if buf.ends_with('\n') {
            buf.pop();
            if cfg!(windows) && buf.ends_with('\r') {
                buf.pop();
            }
        }
        if !buf.is_empty() {
            return Ok(Some(buf));
        }
    }
}

/// Python `int(s)` for the forms worth supporting: surrounding whitespace, a
/// sign, ASCII digits. `None` means `int()` would raise `ValueError`.
fn parse_int(s: &str) -> Option<i128> {
    let t = s.trim();
    let (neg, digits) = match t.as_bytes().first() {
        Some(b'+') => (false, &t[1..]),
        Some(b'-') => (true, &t[1..]),
        _ => (false, t),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // Anything too long for i128 is out of range either way.
    let v = digits.parse::<i128>().unwrap_or(i128::MAX);
    Some(if neg { -v } else { v })
}

fn host_table(
    term: &Term,
    title: String,
    color: Color,
    rows: Vec<Vec<String>>,
    with_group: bool,
) -> Vec<String> {
    let mut columns = vec![
        Column {
            header: "No.",
            style: Style::fg(Color::Cyan),
            no_wrap: true,
        },
        Column {
            header: "Host",
            style: Style::fg(Color::Magenta),
            no_wrap: false,
        },
    ];
    if with_group {
        columns.push(Column {
            header: "Group",
            style: Style::fg(Color::Green),
            no_wrap: false,
        });
    }
    Table {
        title,
        title_style: Style {
            bold: true,
            italic: false,
            color: Some(color),
        },
        border_style: Style::fg(color),
        columns,
        rows,
    }
    .render(term)
}

/// Run the menu. Returns the process exit code.
pub fn run(term: &Term) -> Res<i32> {
    let stdout: io::StdoutLock<'static> = io::stdout().lock();
    let mut out = Out { term, w: stdout };

    let (groups, errors) = match config::default_config_path() {
        Some(p) => config::grouped_hosts(&p),
        None => (Groups::default(), Vec::new()),
    };
    for e in &errors {
        out.click(e, Color::Red)?;
    }
    if groups.is_empty() {
        out.click_plain("No valid SSH hosts found.")?;
        return Ok(0);
    }

    let mut filter = String::new();
    loop {
        let mut selectable: Vec<String> = Vec::new();
        let mut any_shown = false;
        let mut jump: Vec<(String, String)> = Vec::new();
        let ft = filter.to_lowercase();
        for (group, hosts) in groups.iter() {
            let hosts: Vec<&String> = hosts
                .iter()
                .filter(|h| ft.is_empty() || h.to_lowercase().contains(&ft))
                .collect();
            if hosts.is_empty() {
                continue;
            }
            let (normal, jumps): (Vec<&String>, Vec<&String>) = hosts
                .into_iter()
                .partition(|h| !h.to_lowercase().contains("jump"));
            jump.extend(jumps.into_iter().map(|h| (h.clone(), group.to_string())));
            if normal.is_empty() {
                continue;
            }
            any_shown = true;
            let rows = normal
                .iter()
                .map(|h| {
                    selectable.push((*h).clone());
                    vec![selectable.len().to_string(), (*h).clone()]
                })
                .collect();
            for l in host_table(
                term,
                format!("{group} ({})", normal.len()),
                Color::Cyan,
                rows,
                false,
            ) {
                out.line(&l)?;
            }
        }
        if !jump.is_empty() {
            any_shown = true;
            let rows = jump
                .iter()
                .map(|(h, g)| {
                    selectable.push(h.clone());
                    vec![selectable.len().to_string(), h.clone(), g.clone()]
                })
                .collect();
            for l in host_table(
                term,
                format!("JUMP-HOSTS ({})", jump.len()),
                Color::Yellow,
                rows,
                true,
            ) {
                out.line(&l)?;
            }
        }
        if !any_shown {
            out.click("No hosts found matching the filter.", Color::Red)?;
            filter.clear();
            continue;
        }

        let Some(choice) = prompt(&mut out, PROMPT)? else {
            out.click_plain("\nExiting.")?;
            return Ok(0);
        };
        let lowered = choice.to_lowercase();
        if lowered == "q" {
            out.click_plain("Exiting.")?;
            return Ok(0);
        }
        if lowered == "f" {
            let Some(term) = prompt(&mut out, FILTER_PROMPT)? else {
                out.click_plain("\nExiting.")?;
                return Ok(0);
            };
            filter = term;
            continue;
        }

        let selected = match selectable.iter().find(|h| h.to_lowercase() == lowered) {
            Some(h) => h.clone(),
            None => match parse_int(&choice) {
                Some(n) if n >= 1 && n <= selectable.len() as i128 => {
                    selectable[(n - 1) as usize].clone()
                }
                Some(_) => {
                    out.click("Invalid number. Please try again.", Color::Red)?;
                    continue;
                }
                None => {
                    out.click(
                        "Invalid input. Please enter a number, host name, 'q', or 'f'.",
                        Color::Red,
                    )?;
                    continue;
                }
            },
        };

        out.click(&format!("Connecting to {selected}..."), Color::Yellow)?;
        return connect(&mut out, &selected);
    }
}

impl Out<'_> {
    fn click_plain(&mut self, s: &str) -> Res<()> {
        self.line(s)?;
        self.flush()
    }
}

/// Run `ssh <host>` with the terminal handed over, and report how it ended.
fn connect(out: &mut Out, host: &str) -> Res<i32> {
    INTERRUPTED.store(false, Ordering::SeqCst);
    MODE.store(MODE_CHILD, Ordering::SeqCst);
    let child = Command::new("ssh").arg(host).spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            MODE.store(MODE_OTHER, Ordering::SeqCst);
            let message =
                "Error: 'ssh' command not found. Is OpenSSH client installed and in your PATH?";
            let _ = writeln!(io::stderr(), "{}", out.term.click_err(message, Color::Red));
            // 127: the shell's code for "command not found".
            return Ok(127);
        }
        Err(e) => {
            MODE.store(MODE_OTHER, Ordering::SeqCst);
            eprintln!("Error: failed to run ssh: {e}");
            return Ok(1);
        }
    };

    let status = wait_child(&mut child);
    MODE.store(MODE_OTHER, Ordering::SeqCst);
    if INTERRUPTED.load(Ordering::SeqCst) {
        out.click_plain("\nExiting.")?;
        return Ok(INTERRUPTED_EXIT);
    }
    let code = status.map(exit_code).unwrap_or(1);
    if code != 0 {
        out.click(&format!("SSH session for {host} ended."), Color::Blue)?;
    }
    // ssh's own exit code, so scripts and the shell see why it ended.
    Ok(code)
}

/// The exit code a shell would report for `status`: the code itself, or
/// 128 + the signal number when ssh was killed by a signal.
fn exit_code(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return 128 + sig;
        }
    }
    1
}

/// Wait for ssh. After a Ctrl-C reaches this process, give ssh 0.25 s to
/// finish on its own and then kill it, as Python's `subprocess.run` does.
fn wait_child(child: &mut std::process::Child) -> Option<ExitStatus> {
    let mut interrupted_at: Option<Instant> = None;
    loop {
        match child.try_wait() {
            Ok(Some(s)) => return Some(s),
            Ok(None) => {}
            Err(_) => return child.wait().ok(),
        }
        if INTERRUPTED.load(Ordering::SeqCst) {
            let at = *interrupted_at.get_or_insert_with(Instant::now);
            if at.elapsed() >= Duration::from_millis(250) {
                let _ = child.kill();
                return child.wait().ok();
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
mod tests {
    use super::parse_int;

    #[test]
    fn int_parsing_follows_python_for_ascii() {
        assert_eq!(parse_int(" 2 "), Some(2));
        assert_eq!(parse_int("+1"), Some(1));
        assert_eq!(parse_int("-1"), Some(-1));
        assert_eq!(parse_int("abc"), None);
        assert_eq!(parse_int(""), None);
        assert_eq!(parse_int("+"), None);
        assert_eq!(
            parse_int("99999999999999999999999999999999999999999"),
            Some(i128::MAX)
        );
    }
}
