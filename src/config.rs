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

/// `~/.ssh`.
pub fn user_dir() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".ssh"))
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

/// `${NAME}` environment references in an `Include` path, which ssh expands
/// (ssh_config(5), ENVIRONMENT VARIABLES). `None` for an undefined variable
/// or an unclosed `${`.
fn expand_env(pattern: &str) -> Option<String> {
    let mut out = String::new();
    let mut rest = pattern;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after.find('}')?;
        out.push_str(&std::env::var(&after[..end]).ok()?);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// Hosts the menu never shows.
pub fn is_ignored_host(host: &str) -> bool {
    let lowered = host.to_lowercase();
    lowered.ends_with("-ignore") || lowered.contains("github") || lowered.contains("bitbucket")
}

/// Split a config line into its keyword and the rest, as OpenSSH does: the
/// keyword ends at whitespace or '=', and one '=' may stand between the
/// keyword and its arguments (`Host=web`, `Host = web`).
fn split_keyword(line: &str) -> (&str, &str) {
    let end = line
        .find(|c: char| c.is_whitespace() || c == '=')
        .unwrap_or(line.len());
    let (keyword, rest) = line.split_at(end);
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('=').unwrap_or(rest).trim_start();
    (keyword, rest)
}

/// The arguments after the keyword, split as OpenSSH's `argv_split` does:
/// words separated by spaces or tabs, `"..."` or `'...'` quoting (quotes are
/// removed), backslash escapes for quotes, backslash and space, and a word
/// that starts with '#' begins a comment to the end of the line. `None`
/// means an unclosed quote; ssh rejects such a line.
fn arguments(rest: &str) -> Option<Vec<String>> {
    let chars: Vec<char> = rest.chars().collect();
    let mut args = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ' ' || chars[i] == '\t' {
            i += 1;
            continue;
        }
        if chars[i] == '#' {
            break;
        }
        let mut arg = String::new();
        let mut quote: Option<char> = None;
        while i < chars.len() {
            let c = chars[i];
            let next = chars.get(i + 1).copied();
            if c == '\\' {
                match next {
                    Some(n @ ('\'' | '"' | '\\')) => {
                        arg.push(n);
                        i += 1;
                    }
                    Some(' ') if quote.is_none() => {
                        arg.push(' ');
                        i += 1;
                    }
                    _ => arg.push(c),
                }
            } else if quote.is_none() && (c == ' ' || c == '\t') {
                break;
            } else if quote.is_none() && (c == '"' || c == '\'') {
                quote = Some(c);
            } else if quote == Some(c) {
                quote = None;
            } else {
                arg.push(c);
            }
            i += 1;
        }
        if quote.is_some() {
            return None;
        }
        args.push(arg);
    }
    Some(args)
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
    /// Where relative `Include` paths point: `~/.ssh` for a user config,
    /// whichever file the `Include` sits in (ssh_config(5)).
    user_dir: PathBuf,
    /// Error lines in the order Python printed them.
    pub errors: Vec<String>,
}

impl Parser {
    fn new(user_dir: &Path) -> Parser {
        Parser {
            visited: HashSet::new(),
            user_dir: user_dir.to_path_buf(),
            errors: Vec::new(),
        }
    }

    /// Every match of every pattern on one `Include` line, de-duplicated and
    /// sorted together (as the Python version did).
    fn expand_includes(patterns: &[String], base_dir: &Path) -> Vec<PathBuf> {
        let mut found = BTreeSet::new();
        let options = glob::MatchOptions {
            case_sensitive: true,
            require_literal_separator: true,
            require_literal_leading_dot: true,
        };
        for pat in patterns {
            // ssh fails the Include on an unknown variable; list nothing.
            let Some(pat) = expand_env(pat) else {
                continue;
            };
            let expanded = expand_user(&pat);
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

        let bytes = match std::fs::read(&real_path) {
            Ok(b) => b,
            Err(e) => {
                self.report(io_error_message(&e, &real_path));
                return Vec::new();
            }
        };
        // A byte that is not UTF-8 (a Latin-1 comment, say) must not hide the
        // rest of the file. ssh itself does not care about the encoding.
        let text = String::from_utf8_lossy(&bytes);

        let mut hosts = Vec::new();
        let mut includes: Vec<Vec<String>> = Vec::new();
        for raw in universal_lines(&text) {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (keyword, rest) = split_keyword(line);
            if keyword.eq_ignore_ascii_case("Include") {
                let parts = arguments(rest).unwrap_or_default();
                if !parts.is_empty() {
                    includes.push(parts);
                }
                continue;
            }
            if keyword.eq_ignore_ascii_case("Host") {
                for value in arguments(rest).unwrap_or_default() {
                    // '*' and '?' make a pattern, not a host name. A name
                    // starting with '-' would reach ssh as an option. A
                    // negated pattern ('!name') matches no host by itself.
                    // ssh refuses host names with whitespace in them. A name
                    // with bytes that were not UTF-8 cannot be passed on as
                    // written, so it is left out too.
                    if !value.contains(['*', '?', char::REPLACEMENT_CHARACTER])
                        && !value.contains(char::is_whitespace)
                        && !value.starts_with('-')
                        && !value.starts_with('!')
                    {
                        hosts.push(Entry {
                            host: value,
                            source: real_path.clone(),
                        });
                    }
                }
            }
        }
        for parts in includes {
            for inc in Self::expand_includes(&parts, &self.user_dir) {
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
pub fn grouped_hosts(config_path: &Path, user_dir: &Path) -> (Groups, Vec<String>) {
    let mut groups = Groups::default();
    if !config_path.is_file() {
        return (groups, Vec::new());
    }
    let mut parser = Parser::new(user_dir);
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
    let mut parser = Parser::new(config_path.parent().unwrap());
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
        // A relative Include is relative to ~/.ssh, not to the file that
        // holds it (ssh_config(5)). The Python test expected config.d/nested.
        write(&ssh, "config.d/nested/10-not-read", "Host wrong-dir\n");
        write(&ssh, "nested/10-nested-host", "Host nested1\n");
        assert_eq!(hosts(&cfg), vec!["base1", "inc1", "inc2", "nested1"]);
    }

    #[test]
    fn keyword_split_like_openssh() {
        assert_eq!(split_keyword("Host foo"), ("Host", "foo"));
        assert_eq!(split_keyword("HOST\tfoo bar"), ("HOST", "foo bar"));
        assert_eq!(split_keyword("Host=foo"), ("Host", "foo"));
        assert_eq!(split_keyword("Host = foo"), ("Host", "foo"));
        assert_eq!(split_keyword("HostName foo").0, "HostName");
        assert_eq!(split_keyword("Host-foo bar").0, "Host-foo");
        assert_eq!(split_keyword("Host"), ("Host", ""));
    }

    #[test]
    fn comment_ends_the_arguments() {
        assert_eq!(arguments("a b # c d").unwrap(), vec!["a", "b"]);
        assert_eq!(arguments("a#b").unwrap(), vec!["a#b"]);
    }

    #[test]
    fn quotes_and_escapes_like_openssh() {
        assert_eq!(
            arguments(r#""quoted host" plain"#).unwrap(),
            vec!["quoted host", "plain"]
        );
        assert_eq!(arguments("'a b'c d").unwrap(), vec!["a bc", "d"]);
        assert_eq!(arguments(r"a\ b c").unwrap(), vec!["a b", "c"]);
        assert_eq!(arguments(r#"a\"b"#).unwrap(), vec![r#"a"b"#]);
        assert_eq!(arguments(r"a\x").unwrap(), vec![r"a\x"]);
        assert_eq!(
            arguments(r##""# not a comment" x"##).unwrap(),
            vec!["# not a comment", "x"]
        );
        assert_eq!(arguments(r#""unclosed"#), None);
    }

    #[test]
    fn env_references_expand() {
        let path = std::env::var("PATH").unwrap();
        assert_eq!(expand_env("${PATH}/x").unwrap(), format!("{path}/x"));
        assert_eq!(expand_env("$PATH/x").unwrap(), "$PATH/x");
        assert_eq!(expand_env("${SSH_CONNECTOR_SURELY_UNSET_VAR}"), None);
        assert_eq!(expand_env("${PATH"), None);
    }

    #[test]
    fn universal_newlines() {
        let v: Vec<&str> = universal_lines("a\r\nb\rc\nd").collect();
        assert_eq!(v, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn non_utf8_bytes_do_not_hide_hosts() {
        let d = tempfile::tempdir().unwrap();
        let cfg = d.path().join("config");
        fs::write(&cfg, b"# caf\xe9\nHost one\nHost tw\xffo\n").unwrap();
        assert_eq!(hosts(&cfg), vec!["one"]);
    }
}
