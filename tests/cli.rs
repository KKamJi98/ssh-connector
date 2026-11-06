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
