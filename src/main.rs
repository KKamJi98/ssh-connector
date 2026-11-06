mod app;
mod config;
mod table;
mod term;

use clap::{CommandFactory, Parser};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use std::ffi::OsStr;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

/// A CLI tool to list hosts from ~/.ssh/config and connect to them.
#[derive(Parser)]
#[command(name = "ssh-connector", version, disable_help_subcommand = true)]
struct Cli {
    /// Connect to QUERY if it is a host name; otherwise open the menu
    /// filtered by it
    #[arg(add = ArgValueCompleter::new(complete_host))]
    query: Option<String>,

    /// Print the host names, one per line, in menu order, and exit
    #[arg(long)]
    list: bool,

    /// Read this ssh config instead of ~/.ssh/config (ssh gets it as -F too)
    #[arg(short = 'F', long = "config", value_name = "FILE")]
    config: Option<PathBuf>,
}

/// Shell completion for QUERY: the same names `--list` prints.
fn complete_host(current: &OsStr) -> Vec<CompletionCandidate> {
    let prefix = current.to_string_lossy().to_lowercase();
    let Ok((groups, _)) = app::load(&app::Options::default()) else {
        return Vec::new();
    };
    app::menu_order(&groups)
        .into_iter()
        .filter(|h| h.to_lowercase().starts_with(&prefix))
        .map(CompletionCandidate::new)
        .collect()
}

fn main() -> ExitCode {
    // `COMPLETE=<shell> ssh-connector` prints the shell's completion script;
    // the script calls back with COMPLETE set to get the candidates.
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();
    let options = app::Options {
        query: cli.query,
        config: cli.config,
    };
    if cli.list {
        return ExitCode::from(app::list(&options) as u8);
    }

    std::panic::set_hook(Box::new(|info| {
        let _ = writeln!(std::io::stderr(), "Error: internal error: {info}");
    }));
    app::install_interrupt_handler();
    let term = term::Term::detect();
    let result = std::panic::catch_unwind(|| app::run(&term, &options));
    let code = match result {
        Ok(Ok(code)) => code,
        Ok(Err(app::Exit(code))) => code,
        // The panic hook has already reported it.
        Err(_) => 1,
    };
    ExitCode::from(code as u8)
}
