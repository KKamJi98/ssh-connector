# Roadmap

Ideas that were considered during the Rust migration (2026-11) and left
out of 0.4.0. Nothing here is promised.

## P2

- Make the hidden-host rules (`-ignore` suffix, `github`, `bitbucket`)
  configurable in `~/.config/ssh-connector/config.toml` instead of fixed.
- Show `HostName` and `User` columns next to each host.
- Put recently used hosts first (a small history file).
- Pass extra ssh arguments after `--` (`ssh-connector web -- -L 8080:localhost:80`).
- Honour conditional `Include` inside `Host` and `Match` blocks; today an
  `Include` is always read.
- Expand `%` tokens (`%d`, `%u`, ...) in `Include` paths.
- Show a host that is defined in two files once, not once per group.

## P3

- A full-screen picker that filters as you type.
- Preview the effective settings for a host with `ssh -G`.
