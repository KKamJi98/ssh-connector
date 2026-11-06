//! End-to-end checks that are not golden cases: behaviour where the Python
//! version crashed or that it did not have.

mod common;

use common::Sandbox;
use std::io::Write;
use std::process::{Output, Stdio};

fn run(sb: &Sandbox, args: &[&str], stdin: &str) -> Output {
    let mut child = sb
        .command()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

/// Bug 3.12: rich read host names as markup. `a[/b]` crashed the Python
/// version with MarkupError and `[bold]x` was shown as a bold "x".
#[test]
fn host_names_are_printed_literally() {
    let sb = Sandbox::new(true);
    sb.write(".ssh/config", b"Host [bold]x\nHost a[/b]\n");
    let out = run(&sb, &[], "a[/B]\n");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("\u{2502} [bold]x \u{2551}"), "{stdout}");
    assert!(stdout.contains("\u{2502} a[/b]   \u{2551}"), "{stdout}");
    assert!(stdout.contains("Connecting to a[/b]..."), "{stdout}");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains(r#"argv=["--", "a[/b]"]"#), "{stderr}");
    assert_eq!(out.status.code(), Some(0));
}

/// Group names come from file names and were markup too.
#[test]
fn group_names_are_printed_literally() {
    let sb = Sandbox::new(true);
    sb.write(".ssh/config", b"Include d/*\n");
    sb.write(".ssh/d/[x]", b"Host h\n");
    let out = run(&sb, &[], "q\n");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("[x] (1)"), "{stdout}");
    assert_eq!(out.status.code(), Some(0));
}

// Phase 4: command line additions. Each has a normal and a wrong-input case.

const CONFIG: &[u8] = b"Host alpha\nHost jump-a\nHost beta\nHost x-ignore\nInclude more\n";
const MORE: &[u8] = b"Host gamma\nHost alpha\n";

fn sandbox() -> Sandbox {
    let sb = Sandbox::new(true);
    sb.write(".ssh/config", CONFIG);
    sb.write(".ssh/more", MORE);
    sb
}

fn text(b: Vec<u8>) -> String {
    String::from_utf8(b).unwrap()
}

#[test]
fn version_flags_print_the_version() {
    let sb = sandbox();
    for flag in ["--version", "-V"] {
        let out = run(&sb, &[flag], "");
        assert_eq!(
            text(out.stdout),
            format!("ssh-connector {}\n", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(out.status.code(), Some(0));
    }
    let out = run(&sb, &["-v"], "");
    assert_eq!(out.status.code(), Some(2));
    assert!(text(out.stderr).contains("unexpected argument '-v'"));
}

#[test]
fn short_help_prints_usage() {
    let sb = sandbox();
    let out = run(&sb, &["-h"], "");
    assert!(text(out.stdout).starts_with("A CLI tool"));
    assert_eq!(out.status.code(), Some(0));
    let out = run(&sb, &["--hlep"], "");
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn list_prints_hosts_in_menu_order() {
    let sb = sandbox();
    let out = run(&sb, &["--list"], "");
    assert_eq!(text(out.stdout), "alpha\nbeta\ngamma\njump-a\n");
    assert_eq!(out.status.code(), Some(0));

    let out = run(&sb, &["--list", "A"], "");
    assert_eq!(text(out.stdout), "alpha\nbeta\ngamma\njump-a\n");
    let out = run(&sb, &["--list", "mm"], "");
    assert_eq!(text(out.stdout), "gamma\n");
    let out = run(&sb, &["--list", "nothing"], "");
    assert_eq!(text(out.stdout), "");
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn query_naming_a_host_connects_directly() {
    let sb = sandbox();
    let out = run(&sb, &["GAMMA"], "");
    assert_eq!(text(out.stdout), "Connecting to gamma...\n");
    assert_eq!(text(out.stderr), "fake-ssh argv=[\"--\", \"gamma\"]\n");
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn query_that_is_not_a_host_filters_the_menu() {
    let sb = sandbox();
    let out = run(&sb, &["alp"], "1\n");
    let stdout = text(out.stdout);
    assert!(stdout.contains("alpha"), "{stdout}");
    assert!(!stdout.contains("beta"), "{stdout}");
    assert!(stdout.ends_with("Connecting to alpha...\n"), "{stdout}");

    let out = run(&sb, &["zzz"], "q\n");
    let stdout = text(out.stdout);
    assert!(
        stdout.starts_with("No hosts found matching the filter.\n"),
        "{stdout}"
    );
    assert!(stdout.contains("beta"), "{stdout}");
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn config_flag_reads_that_file_and_passes_it_to_ssh() {
    let sb = sandbox();
    sb.write("other/ssh_config", b"Host only-here\n");
    let path = sb.home().join("other").join("ssh_config");
    let path_str = path.to_string_lossy().to_string();

    let out = run(&sb, &["-F", &path_str, "--list"], "");
    assert_eq!(text(out.stdout), "only-here\n");

    let out = run(&sb, &["--config", &path_str, "only-here"], "");
    assert_eq!(
        text(out.stderr),
        format!("fake-ssh argv={:?}\n", ["-F", &path_str, "--", "only-here"])
    );
    assert_eq!(out.status.code(), Some(0));

    let missing = sb.home().join("missing").to_string_lossy().to_string();
    let out = run(&sb, &["-F", &missing], "");
    assert_eq!(
        text(out.stderr),
        format!("Error: SSH config file not found at {missing}\n")
    );
    assert_eq!(out.status.code(), Some(1));
    let out = run(&sb, &["-F", &missing, "--list"], "");
    assert_eq!(out.status.code(), Some(1));
    let out = run(&sb, &["-F"], "");
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn completion_offers_host_names() {
    let sb = sandbox();
    let out = sb
        .command()
        .env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "1")
        .env("_CLAP_COMPLETE_COMP_TYPE", "9")
        .env("_CLAP_COMPLETE_SPACE", "true")
        .env("_CLAP_IFS", "\n")
        .args(["--", "ssh-connector", "a"])
        .output()
        .unwrap();
    assert_eq!(text(out.stdout), "alpha");
    assert_eq!(out.status.code(), Some(0));

    let out = sb.command().env("COMPLETE", "zsh").output().unwrap();
    assert!(text(out.stdout).starts_with("#compdef ssh-connector"));

    let out = sb
        .command()
        .env("COMPLETE", "no-such-shell")
        .output()
        .unwrap();
    assert_ne!(out.status.code(), Some(0));
}
