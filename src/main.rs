mod app;
mod config;
mod table;
mod term;

use clap::Parser;
use std::io::Write;
use std::process::ExitCode;

/// The command line. The Python version (click) had `--help` and nothing
/// else: no `-h`, no `--version`, no positional arguments.
#[derive(Parser)]
#[command(
    name = "ssh-connector",
    disable_help_flag = true,
    disable_version_flag = true,
    disable_help_subcommand = true
)]
struct Cli {
    #[arg(long = "help")]
    help: bool,

    /// Collected only to report them the way click did.
    #[arg(hide = true, num_args = 0..)]
    extra: Vec<String>,
}

fn prog_name() -> String {
    std::env::args_os()
        .next()
        .and_then(|a| {
            std::path::Path::new(&a)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
        })
        // click on Windows saw the console script name; drop the suffix so
        // usage lines read the same everywhere.
        .map(|n| n.strip_suffix(".exe").map(str::to_string).unwrap_or(n))
        .unwrap_or_else(|| "ssh-connector".to_string())
}

fn help_text(prog: &str) -> String {
    format!(
        "Usage: {prog} [OPTIONS]\n\n  A CLI tool to list hosts from ~/.ssh/config and connect to them.\n\nOptions:\n  --help  Show this message and exit.\n"
    )
}

/// click's usage error: usage line, hint, blank line, message; exit 2.
fn usage_error(prog: &str, message: &str) -> ExitCode {
    let _ = write!(
        std::io::stderr(),
        "Usage: {prog} [OPTIONS]\nTry '{prog} --help' for help.\n\nError: {message}\n"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let prog = prog_name();
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            use clap::error::{ContextKind, ContextValue, ErrorKind};
            let arg = match e.get(ContextKind::InvalidArg) {
                Some(ContextValue::String(s)) => s.clone(),
                _ => String::new(),
            };
            let arg = arg.split('=').next().unwrap_or_default().to_string();
            let message = match e.kind() {
                ErrorKind::UnknownArgument if !arg.is_empty() => format!("No such option '{arg}'."),
                _ => e
                    .to_string()
                    .trim_start_matches("error: ")
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string(),
            };
            return usage_error(&prog, &message);
        }
    };
    if cli.help {
        let mut out = std::io::stdout();
        if out
            .write_all(help_text(&prog).as_bytes())
            .and_then(|_| out.flush())
            .is_err()
        {
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }
    if !cli.extra.is_empty() {
        let (noun, list) = if cli.extra.len() == 1 {
            ("argument", cli.extra[0].clone())
        } else {
            ("arguments", cli.extra.join(" "))
        };
        return usage_error(&prog, &format!("Got unexpected extra {noun} ({list})"));
    }

    std::panic::set_hook(Box::new(|info| {
        let _ = writeln!(std::io::stderr(), "Error: internal error: {info}");
    }));
    app::install_interrupt_handler();
    let term = term::Term::detect();
    let result = std::panic::catch_unwind(|| app::run(&term));
    let code = match result {
        Ok(Ok(code)) => code,
        Ok(Err(app::Exit(code))) => code,
        // The panic hook has already reported it.
        Err(_) => 1,
    };
    ExitCode::from(code as u8)
}
