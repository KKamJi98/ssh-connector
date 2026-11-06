//! Reading host names out of an OpenSSH client config.
//!
//! This is a port of the Python version's parser and keeps its rules on
//! purpose, including where they differ from OpenSSH. Fixing those is a
//! separate step so that any change in output can be traced to a decision.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

/// A host name and the file (canonical path) it was found in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub host: String,
    pub source: PathBuf,
}

/// Hosts grouped by source file, in discovery order.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Groups {
    pub order: Vec<String>,
    pub hosts: Vec<Vec<String>>,
}

impl Groups {
    pub fn is_empty(&self) -> bool {
        self.hosts.iter().all(Vec::is_empty)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &[String])> {
        self.order
            .iter()
            .map(String::as_str)
            .zip(self.hosts.iter().map(Vec::as_slice))
    }
}

/// The user's home directory, found the way Python's `Path.home()` finds it.
pub fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(p) = std::env::var_os("USERPROFILE") {
            return Some(PathBuf::from(p));
        }
        let drive = std::env::var_os("HOMEDRIVE").unwrap_or_default();
        let path = std::env::var_os("HOMEPATH")?;
        let mut p = PathBuf::from(drive);
        p.push(path);
        Some(p)
    }
    #[cfg(not(windows))]
    {
        if let Some(h) = std::env::var_os("HOME") {
            return Some(PathBuf::from(h));
        }
        std::env::home_dir()
    }
}

pub fn default_config_path() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".ssh").join("config"))
}

/// Python `os.path.expanduser` for `~` and `~/...`. `~user` is left as is.
fn expand_user(pattern: &str) -> String {
    let Some(rest) = pattern.strip_prefix('~') else {
        return pattern.to_string();
    };
    let is_sep = |c: char| c == '/' || (cfg!(windows) && c == '\\');
    if !(rest.is_empty() || rest.starts_with(is_sep)) {
        return pattern.to_string();
    }
    let Some(home) = home_dir() else {
        return pattern.to_string();
    };
    let home = home.to_string_lossy().to_string();
    let home = home.trim_end_matches(is_sep);
    let joined = format!("{home}{rest}");
    if joined.is_empty() {
        "/".to_string()
    } else {
        joined
    }
}

/// Hosts the menu never shows.
pub fn is_ignored_host(host: &str) -> bool {
    let lowered = host.to_lowercase();
    lowered.ends_with("-ignore") || lowered.contains("github") || lowered.contains("bitbucket")
}

/// `re.match(r"^<kw>\b", line, re.IGNORECASE)`.
fn starts_with_keyword(line: &str, keyword: &str) -> bool {
    let Some(head) = line.get(..keyword.len()) else {
        return false;
    };
    if !head.eq_ignore_ascii_case(keyword) {
        return false;
    }
    match line[keyword.len()..].chars().next() {
        None => true,
        Some(c) => !(c.is_alphanumeric() || c == '_'),
    }
}

/// Python universal-newline line splitting: `\r\n`, `\r` and `\n`.
fn universal_lines(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        match rest.find(['\r', '\n']) {
            Some(i) => {
                let line = &rest[..i];
                let skip = if rest[i..].starts_with("\r\n") { 2 } else { 1 };
                rest = &rest[i + skip..];
                Some(line)
            }
            None => {
                let line = rest;
                rest = "";
                Some(line)
            }
        }
    })
}

/// Python's message for a failed strict UTF-8 decode of `buf`, where the
/// error position is counted from the start of `buf`.
fn decode_error_message(buf: &[u8], err: std::str::Utf8Error) -> String {
    let start = err.valid_up_to();
    let len = err.error_len().unwrap_or(buf.len() - start);
    let reason = match err.error_len() {
        None => "unexpected end of data",
        Some(_) if matches!(buf[start], 0xC2..=0xF4) => "invalid continuation byte",
        Some(_) => "invalid start byte",
    };
    if len <= 1 {
        format!(
            "'utf-8' codec can't decode byte 0x{:02x} in position {start}: {reason}",
            buf[start]
        )
    } else {
        format!(
            "'utf-8' codec can't decode bytes in position {start}-{}: {reason}",
            start + len - 1
        )
    }
}

/// Decode a file the way Python's text-mode file iteration does: 8192-byte
/// chunks through an incremental decoder. Only the error message depends on
/// this, because Python counts the error position from the start of the
/// chunk (plus any partial character carried over), not of the file.
fn decode_like_python(bytes: &[u8]) -> Result<&str, String> {
    const CHUNK: usize = 8192;
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(text);
    }
    let mut pending_start = 0;
    let mut pos = 0;
    loop {
        let chunk_end = (pos + CHUNK).min(bytes.len());
        let buf = &bytes[pending_start..chunk_end];
        let at_eof = chunk_end == bytes.len();
        match std::str::from_utf8(buf) {
            Ok(_) => pending_start = chunk_end,
            Err(e) => match e.error_len() {
                Some(_) => return Err(decode_error_message(buf, e)),
                None if at_eof => {
                    // The final decode only sees the incomplete tail.
                    let tail = &buf[e.valid_up_to()..];
                    let err = std::str::from_utf8(tail).unwrap_err();
                    return Err(decode_error_message(tail, err));
                }
                None => pending_start += e.valid_up_to(),
            },
        }
        if at_eof {
            // Not reached: the whole input failed to decode above.
            return Err(String::new());
        }
        pos = chunk_end;
    }
}

/// Python `repr()` of a path string, enough for error messages.
fn py_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

fn io_error_message(err: &std::io::Error, path: &Path) -> String {
    let p = py_repr(&path.to_string_lossy());
    match (err.raw_os_error(), err.kind()) {
        (Some(code), std::io::ErrorKind::PermissionDenied) => {
            format!("[Errno {code}] Permission denied: {p}")
        }
        (Some(code), _) => format!("[Errno {code}] {err}: {p}"),
        (None, _) => err.to_string(),
    }
}

/// `Path.resolve()`: the canonical path, without the `\\?\` prefix Windows
/// adds (Python does not, and glob patterns built on it would not match).
fn canonical(p: &Path) -> std::io::Result<PathBuf> {
    let c = p.canonicalize()?;
    #[cfg(windows)]
    {
        let s = c.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            if !rest.starts_with("UNC\\") {
                return Ok(PathBuf::from(rest.to_string()));
            }
        }
    }
    Ok(c)
}

pub struct Parser {
    visited: HashSet<PathBuf>,
    /// Error lines in the order Python printed them.
    pub errors: Vec<String>,
}

impl Parser {
    fn new() -> Parser {
        Parser {
            visited: HashSet::new(),
            errors: Vec::new(),
        }
    }

    /// `_expand_include_patterns`: every match of every pattern on one
    /// `Include` line, de-duplicated and sorted together.
    fn expand_includes(patterns: &[String], base_dir: &Path) -> Vec<PathBuf> {
        let mut found = BTreeSet::new();
        let options = glob::MatchOptions {
            case_sensitive: true,
            require_literal_separator: true,
            require_literal_leading_dot: true,
        };
        for pat in patterns {
            let expanded = expand_user(pat);
            let path = Path::new(&expanded);
            let full = if path.is_absolute() {
                path.to_path_buf()
            } else {
                base_dir.join(path)
            };
            let full_str = full.to_string_lossy().to_string();
            match glob::glob_with(&full_str, options) {
                Ok(paths) => {
                    for m in paths.flatten() {
                        if m.is_file() {
                            found.insert(m);
                        }
                    }
                }
                Err(_) => {
                    // Python's glob treats a malformed pattern literally.
                    if full.is_file() {
                        found.insert(full);
                    }
                }
            }
        }
        found.into_iter().collect()
    }

    /// `_parse_config_file`.
    fn parse_file(&mut self, path: &Path) -> Vec<Entry> {
        let Ok(real_path) = canonical(path) else {
            return Vec::new();
        };
        if !self.visited.insert(real_path.clone()) || !real_path.is_file() {
            return Vec::new();
        }
        let base_dir = real_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();

        let bytes = match std::fs::read(&real_path) {
            Ok(b) => b,
            Err(e) => {
                self.report(io_error_message(&e, &real_path));
                return Vec::new();
            }
        };
        let text = match decode_like_python(&bytes) {
            Ok(t) => t,
            Err(message) => {
                self.report(message);
                return Vec::new();
            }
        };

        let mut hosts = Vec::new();
        let mut includes: Vec<Vec<String>> = Vec::new();
        for raw in universal_lines(text) {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if starts_with_keyword(line, "Include") {
                let parts: Vec<String> =
                    line.split_whitespace().skip(1).map(String::from).collect();
                if !parts.is_empty() {
                    includes.push(parts);
                }
                continue;
            }
            if starts_with_keyword(line, "Host") {
                for value in line.split_whitespace().skip(1) {
                    // A name starting with '-' would reach ssh as an option.
                    if !value.contains('*') && !value.starts_with('-') {
                        hosts.push(Entry {
                            host: value.to_string(),
                            source: real_path.clone(),
                        });
                    }
                }
            }
        }
        for parts in includes {
            for inc in Self::expand_includes(&parts, &base_dir) {
                hosts.extend(self.parse_file(&inc));
            }
        }
        hosts
    }

    fn report(&mut self, message: String) {
        self.errors.push(format!(
            "Error reading or parsing SSH config file: {message}"
        ));
    }
}

/// `get_ssh_hosts_grouped`: hosts grouped by source file ("Main" for the main
/// config, the file name for included files). Also returns parse errors.
pub fn grouped_hosts(config_path: &Path) -> (Groups, Vec<String>) {
    let mut groups = Groups::default();
    if !config_path.is_file() {
        return (groups, Vec::new());
    }
    let mut parser = Parser::new();
    let entries = parser.parse_file(config_path);
    let main = canonical(config_path).ok();
    // (group, host) pairs already listed; a set keeps large configs linear.
    let mut seen: HashSet<(usize, String)> = HashSet::new();
    for Entry { host, source } in entries {
        if is_ignored_host(&host) {
            continue;
        }
        let group = if Some(&source) == main.as_ref() {
            "Main".to_string()
        } else {
            source
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        };
        let idx = match groups.order.iter().position(|g| *g == group) {
            Some(i) => i,
            None => {
                groups.order.push(group);
                groups.hosts.push(Vec::new());
                groups.order.len() - 1
            }
        };
        if seen.insert((idx, host.clone())) {
            groups.hosts[idx].push(host);
        }
    }
    (groups, parser.errors)
}

/// `get_ssh_hosts`: unique host names in discovery order.
#[cfg(test)]
pub fn hosts(config_path: &Path) -> Vec<String> {
    if !config_path.is_file() {
        return Vec::new();
    }
    let mut parser = Parser::new();
    let mut out: Vec<String> = Vec::new();
    for e in parser.parse_file(config_path) {
        if !is_ignored_host(&e.host) && !out.contains(&e.host) {
            out.push(e.host);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(dir: &Path, rel: &str, content: &str) -> PathBuf {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, content).unwrap();
        p
    }

    // Ported from tests/test_ssh_connector.py.

    #[test]
    fn valid_config() {
        let d = tempfile::tempdir().unwrap();
        let cfg = write(
            d.path(),
            "config",
            "\n    Host server1\n        HostName 192.168.1.1\n\n    Host server2\n        HostName 192.168.1.2\n\n    Host *.example.com\n    ",
        );
        assert_eq!(hosts(&cfg), vec!["server1", "server2"]);
    }

    #[test]
    fn no_config_file() {
        let d = tempfile::tempdir().unwrap();
        assert!(hosts(&d.path().join("missing")).is_empty());
    }

    #[test]
    fn empty_config() {
        let d = tempfile::tempdir().unwrap();
        let cfg = write(d.path(), "config", "");
        assert!(hosts(&cfg).is_empty());
    }

    #[test]
    fn excludes_ignore_and_vcs_hosts() {
        let d = tempfile::tempdir().unwrap();
        let cfg = write(
            d.path(),
            "config",
            "\n    Host server1-ignore\n    Host server2\n    Host github-enterprise\n    Host BITBUCKET-jump\n    ",
        );
        assert_eq!(hosts(&cfg), vec!["server2"]);
    }

    #[test]
    fn follows_include() {
        let d = tempfile::tempdir().unwrap();
        let ssh = d.path().join(".ssh");
        let cfg = write(
            &ssh,
            "config",
            "Include config.d/*\n# Comment line\nHost base1\n    HostName 10.0.0.1\n",
        );
        write(
            &ssh,
            "config.d/01-hosts",
            "Host inc1 inc2\n    HostName example\n",
        );
        write(&ssh, "config.d/02-wild", "Host *.example.com\n");
        write(&ssh, "config.d/03-include-nested", "Include nested/*\n");
        write(&ssh, "config.d/nested/10-nested-host", "Host nested1\n");
        assert_eq!(hosts(&cfg), vec!["base1", "inc1", "inc2", "nested1"]);
    }

    #[test]
    fn keyword_needs_word_boundary() {
        assert!(starts_with_keyword("Host foo", "Host"));
        assert!(starts_with_keyword("HOST\tfoo", "Host"));
        assert!(starts_with_keyword("Host=foo", "Host"));
        assert!(!starts_with_keyword("HostName foo", "Host"));
        assert!(!starts_with_keyword("Hos", "Host"));
    }

    #[test]
    fn universal_newlines() {
        let v: Vec<&str> = universal_lines("a\r\nb\rc\nd").collect();
        assert_eq!(v, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn decode_messages_match_python() {
        let msg = |b: &[u8]| decode_like_python(b).unwrap_err();
        assert_eq!(
            msg(b"# caf\xe9\nHost one\n"),
            "'utf-8' codec can't decode byte 0xe9 in position 5: invalid continuation byte"
        );
        assert_eq!(
            msg(b"x\xffy"),
            "'utf-8' codec can't decode byte 0xff in position 1: invalid start byte"
        );
        assert_eq!(
            msg(b"Host one\n\xe2\x82"),
            "'utf-8' codec can't decode bytes in position 0-1: unexpected end of data"
        );
        let mut big = vec![b'#'; 9000];
        big[8500] = 0xff;
        assert_eq!(
            msg(&big),
            "'utf-8' codec can't decode byte 0xff in position 308: invalid start byte"
        );
    }
}
