//! Replays tests/golden/cases.json (recorded from the Python version by
//! scripts/golden.py) against the Rust binary. stdout, stderr and the exit
//! code must match byte for byte, except for the cases listed in
//! tests/golden/changes.json: deliberate fixes, each with its bug number,
//! reason and new expected output.

mod common;

use common::Sandbox;
use serde_json::Value;
use std::io::Write;
use std::process::Stdio;

fn b64(s: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for c in s.bytes().filter(|&c| c != b'=') {
        let v = ALPHABET.iter().position(|&a| a == c).expect("base64") as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    out
}

fn str_map(v: &Value, key: &str) -> Vec<(String, String)> {
    v.get(key)
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn run(c: &Value) -> Option<(String, String, i32)> {
    if c.get("unix_only").and_then(Value::as_bool).unwrap_or(false) && !cfg!(unix) {
        return None;
    }
    let sandbox = Sandbox::new(c.get("ssh").and_then(Value::as_bool).unwrap_or(true));
    for (rel, text) in str_map(c, "files") {
        sandbox.write(&rel, text.as_bytes());
    }
    for (rel, data) in str_map(c, "files_b64") {
        sandbox.write(&rel, &b64(&data));
    }
    if let Some(data) = c.get("config_b64").and_then(Value::as_str) {
        sandbox.write(".ssh/config", &b64(data));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for (rel, target) in str_map(c, "symlinks") {
            let p = sandbox.home().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(target, p).unwrap();
        }
        if let Some(modes) = c.get("modes").and_then(Value::as_object) {
            for (rel, mode) in modes {
                let p = sandbox.home().join(rel);
                std::fs::set_permissions(
                    p,
                    std::fs::Permissions::from_mode(mode.as_u64().unwrap() as u32),
                )
                .unwrap();
            }
        }
    }

    let mut cmd = sandbox.command();
    if let Some(args) = c.get("args").and_then(Value::as_array) {
        cmd.args(args.iter().map(|a| a.as_str().unwrap()));
    }
    for (k, v) in str_map(c, "env") {
        cmd.env(k, v);
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    let stdin = c["stdin"].as_str().unwrap().as_bytes().to_vec();
    let mut pipe = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || {
        let _ = pipe.write_all(&stdin);
    });
    let out = child.wait_with_output().unwrap();
    writer.join().unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Some(modes) = c.get("modes").and_then(Value::as_object) {
            for rel in modes.keys() {
                let _ = std::fs::set_permissions(
                    sandbox.home().join(rel),
                    std::fs::Permissions::from_mode(0o644),
                );
            }
        }
    }
    Some((
        sandbox.redact(&String::from_utf8(out.stdout).unwrap()),
        sandbox.redact(&String::from_utf8(out.stderr).unwrap()),
        out.status.code().unwrap_or(-1),
    ))
}

#[test]
fn matches_python_reference() {
    let doc: Value = serde_json::from_str(include_str!("golden/cases.json")).unwrap();
    let changes: Value = serde_json::from_str(include_str!("golden/changes.json")).unwrap();
    let mut failures = Vec::new();
    let mut ran = 0;
    for c in doc["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let Some((stdout, stderr, code)) = run(c) else {
            continue;
        };
        ran += 1;
        let e = changes["cases"]
            .get(name)
            .map(|ch| &ch["expected"])
            .unwrap_or(&c["expected"]);
        if stdout != e["stdout"].as_str().unwrap()
            || stderr != e["stderr"].as_str().unwrap()
            || i64::from(code) != e["code"].as_i64().unwrap()
        {
            failures.push(format!(
                "{name}\n  expected code={} stdout={:?} stderr={:?}\n  got      code={code} stdout={stdout:?} stderr={stderr:?}",
                e["code"], e["stdout"], e["stderr"]
            ));
        }
    }
    assert!(ran > 0);
    assert!(
        failures.is_empty(),
        "{} of {ran} golden cases differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
