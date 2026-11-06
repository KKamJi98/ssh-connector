//! Ctrl-C handling, checked by sending SIGINT to the process. The expected
//! behaviour is what the Python version did (see docs/rust-migration.md).
#![cfg(unix)]

mod common;

use common::Sandbox;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn sigint(child: &Child) {
    let ok = Command::new("kill")
        .arg("-INT")
        .arg(child.id().to_string())
        .status()
        .unwrap();
    assert!(ok.success());
}

/// Read stdout until `needle` shows up, so the signal lands at a known point.
fn wait_for(child: &mut Child, needle: &str) -> String {
    let stdout = child.stdout.as_mut().unwrap();
    let mut seen = Vec::new();
    let mut buf = [0u8; 4096];
    let deadline = Instant::now() + Duration::from_secs(10);
    while !String::from_utf8_lossy(&seen).contains(needle) {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {needle:?}"
        );
        let n = stdout.read(&mut buf).unwrap();
        assert!(n > 0, "stdout closed before {needle:?}");
        seen.extend_from_slice(&buf[..n]);
    }
    String::from_utf8(seen).unwrap()
}

fn finish(mut child: Child, mut seen: String) -> (String, String, i32) {
    let mut rest = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut rest)
        .unwrap();
    seen.push_str(&rest);
    let mut err = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut err)
        .unwrap();
    let code = child.wait().unwrap().code().unwrap_or(-1);
    (seen, err, code)
}

#[test]
fn interrupt_at_prompt_aborts_with_exit_1() {
    let sb = Sandbox::new(true);
    sb.write(".ssh/config", b"Host alpha\n");
    let mut child = sb
        .command()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let seen = wait_for(&mut child, "'f' to filter): ");
    std::thread::sleep(Duration::from_millis(100));
    sigint(&child);
    let (_, err, code) = finish(child, seen);
    assert_eq!(err, "Aborted!\n");
    assert_eq!(code, 1);
}

#[test]
fn interrupt_while_ssh_runs_stops_ssh_and_exits_0() {
    let sb = Sandbox::new(true);
    sb.write(".ssh/config", b"Host alpha\n");
    let mut child = sb
        .command()
        .env("FAKE_SSH_SLEEP_MS", "5000")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(b"1\n").unwrap();
    let seen = wait_for(&mut child, "Connecting to alpha...\n");
    std::thread::sleep(Duration::from_millis(300));
    let start = Instant::now();
    sigint(&child);
    let (out, _, code) = finish(child, seen);
    assert!(
        start.elapsed() < Duration::from_secs(3),
        "ssh was not stopped"
    );
    assert!(
        out.ends_with("Connecting to alpha...\n\nExiting.\n"),
        "{out:?}"
    );
    assert_eq!(code, 0);
}

#[test]
fn ssh_killed_by_a_signal_exits_with_128_plus_signal() {
    let sb = Sandbox::new(true);
    sb.write(".ssh/config", b"Host alpha\n");
    let mut child = sb
        .command()
        .env("FAKE_SSH_SLEEP_MS", "5000")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(b"1\n").unwrap();
    let seen = wait_for(&mut child, "Connecting to alpha...\n");
    std::thread::sleep(Duration::from_millis(300));
    // Terminate only ssh (the fake), not ssh-connector.
    let ok = Command::new("pkill")
        .args(["-TERM", "-P", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(ok.success());
    let (out, _, code) = finish(child, seen);
    assert!(out.ends_with("SSH session for alpha ended.\n"), "{out:?}");
    assert_eq!(code, 128 + 15);
}
