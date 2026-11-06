//! Helpers shared by the integration tests.
#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;

pub const BIN: &str = env!("CARGO_BIN_EXE_ssh-connector");

/// The fake ssh built from `examples/fake-ssh.rs`. `cargo test` builds
/// examples next to the test binaries.
pub fn fake_ssh() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let debug_dir = exe.parent().unwrap().parent().unwrap();
    let name = format!("fake-ssh{}", std::env::consts::EXE_SUFFIX);
    let p = debug_dir.join("examples").join(name);
    assert!(
        p.exists(),
        "fake ssh not built at {}; run `cargo test` (it builds examples)",
        p.display()
    );
    p
}

/// A scratch HOME plus a bin directory that holds the fake ssh.
pub struct Sandbox {
    pub dir: tempfile::TempDir,
}

impl Sandbox {
    pub fn new(with_ssh: bool) -> Sandbox {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("home")).unwrap();
        std::fs::create_dir_all(dir.path().join("bin")).unwrap();
        if with_ssh {
            let name = format!("ssh{}", std::env::consts::EXE_SUFFIX);
            std::fs::copy(fake_ssh(), dir.path().join("bin").join(name)).unwrap();
        }
        Sandbox { dir }
    }

    pub fn home(&self) -> PathBuf {
        self.dir.path().join("home")
    }

    pub fn write(&self, rel: &str, bytes: &[u8]) {
        let p = self.home().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    /// A command with a clean environment: HOME (USERPROFILE on Windows)
    /// and a PATH that holds only the fake ssh.
    pub fn command(&self) -> Command {
        let mut c = Command::new(BIN);
        c.env_clear();
        let home = self.home();
        c.env("HOME", &home);
        c.env("USERPROFILE", &home);
        let bin = self.dir.path().join("bin");
        let has_ssh = bin
            .join(format!("ssh{}", std::env::consts::EXE_SUFFIX))
            .exists();
        // Without the fake, PATH holds nothing else: /usr/bin has a real ssh.
        if cfg!(windows) || !has_ssh {
            c.env("PATH", &bin);
            if let Some(root) = std::env::var_os("SystemRoot") {
                c.env("SystemRoot", root);
            }
        } else {
            let mut path = bin.into_os_string();
            path.push(":/usr/bin:/bin");
            c.env("PATH", path);
        }
        c
    }

    /// Replace the sandbox HOME (raw and canonical) with `<HOME>`.
    pub fn redact(&self, s: &str) -> String {
        let mut homes = vec![self.home().to_string_lossy().to_string()];
        if let Ok(c) = self.home().canonicalize() {
            homes.push(c.to_string_lossy().to_string());
        }
        homes.sort_by_key(|h| std::cmp::Reverse(h.len()));
        homes
            .into_iter()
            .fold(s.to_string(), |acc, h| acc.replace(&h, "<HOME>"))
    }
}
