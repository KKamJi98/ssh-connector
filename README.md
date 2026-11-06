# SSH Connector

[![CI](https://github.com/KKamJi98/ssh-connector/actions/workflows/ci.yml/badge.svg)](https://github.com/KKamJi98/ssh-connector/actions/workflows/ci.yml)

A CLI tool to simplify SSH connections by providing an interactive menu of hosts from your `~/.ssh/config` file.

It is a single binary with no runtime dependencies. Since 0.4.0 it is written in Rust; versions up to 0.3.1 were a Python package on PyPI (see [Moving from the PyPI version](#moving-from-the-pypi-version)).

### Example

Let's assume your `~/.ssh/config` file contains the following entries:

```text
Host dev-app-1
    Hostname 192.168.1.10
    User devuser

Host dev-db-1
    Hostname 192.168.1.11
    User devuser

Host stg-app-1
    Hostname 192.168.2.10
    User stguser
    ProxyJump jump-stg

Host prod-app-1
    Hostname 10.0.0.10
    User produser
    ProxyJump jump-prod

Host jump-stg
    Hostname 172.16.0.2
    User jumpstg

Host jump-prod
    Hostname 172.16.0.3
    User jumpprod
```

When you run `ssh-connector`, you will see an interactive menu like this:

```text
      Main (4)      
╔═════╤════════════╗
║ No. │ Host       ║
╟─────┼────────────╢
║ 1   │ dev-app-1  ║
║ 2   │ dev-db-1   ║
║ 3   │ stg-app-1  ║
║ 4   │ prod-app-1 ║
╚═════╧════════════╝
      JUMP-HOSTS (2)       
╔═════╤═══════════╤═══════╗
║ No. │ Host      │ Group ║
╟─────┼───────────┼───────╢
║ 5   │ jump-stg  │ Main  ║
║ 6   │ jump-prod │ Main  ║
╚═════╧═══════════╧═══════╝
Enter the number or name of the host to connect to (or 'q' to quit, 'f' to filter):
```

Select a host by its number (e.g. `4`) or its name (e.g. `prod-app-1`). The tool runs `ssh <host>`, so everything in your config (`ProxyJump`, `User`, keys) applies as usual.

## Features

- Lists all hosts defined in your `~/.ssh/config`, follows `Include` directives (e.g. `Include ~/.ssh/config.d/*`), and groups entries by source file (`Main` for the main config).
- Moves host names containing `jump` (case-insensitive) into a "JUMP-HOSTS" table at the bottom, with their source group.
- Hides hosts whose names end with `-ignore` or contain `github`/`bitbucket` (case-insensitive).
- Select a host by number or by name (case-insensitive), or press `f` to filter by substring.
- Connects with the `ssh` command on your `PATH`.

## Installation

### Homebrew (macOS, Linux)

```bash
brew install KKamJi98/tap/ssh-connector
```

### Cargo

```bash
cargo install kkamji-ssh-connector
```

The crate is called `kkamji-ssh-connector` because `ssh-connector` on crates.io belongs to an unrelated project. The installed command is still `ssh-connector`. If you already installed that other crate with `cargo install`, cargo will refuse to overwrite its `ssh-connector` binary; uninstall it first or use Homebrew.

### Prebuilt binaries

Every [GitHub release](https://github.com/KKamJi98/ssh-connector/releases) has archives for:

| Platform | Archive |
|---|---|
| macOS (Apple silicon) | `ssh-connector-<tag>-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `ssh-connector-<tag>-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 (static, musl) | `ssh-connector-<tag>-x86_64-unknown-linux-musl.tar.gz` |
| Linux arm64 (static, musl) | `ssh-connector-<tag>-aarch64-unknown-linux-musl.tar.gz` |
| Windows x86_64 | `ssh-connector-<tag>-x86_64-pc-windows-msvc.zip` |

Each archive has a `.sha256` file next to it and a build provenance attestation:

```bash
gh attestation verify ssh-connector-<tag>-<target>.tar.gz --repo KKamJi98/ssh-connector
```

On Windows the tool uses the OpenSSH client that ships with Windows 10 1809 and later (Settings > Optional features > OpenSSH Client) and reads `%USERPROFILE%\.ssh\config`.

### Moving from the PyPI version

Versions up to 0.3.1 were published to PyPI. New versions are not; 0.3.1 stays on PyPI.

```bash
uv tool uninstall ssh-connector   # or: pipx uninstall ssh-connector
brew install KKamJi98/tap/ssh-connector
```

To go back: `brew uninstall ssh-connector` and then `uv tool install ssh-connector==0.3.1`.

## Usage

```bash
ssh-connector
```

This shows your configured SSH hosts. Enter a host number or name to connect, `f` to filter, `q` to quit.

### How hosts are discovered and displayed

- The tool reads `~/.ssh/config` and follows any `Include` directives it finds (globs are supported).
- If you split entries into `~/.ssh/config.d/`, make sure your main config includes them:

```text
Include ~/.ssh/config.d/*

Host base
    HostName 192.168.0.10
```

- Hosts are grouped by the file they come from (`Main` for `~/.ssh/config`, the file name for included files).
- Any host name containing `jump` (case-insensitive) is collected into a final "JUMP-HOSTS" section, with its source group in a separate column.
- Hosts whose names end with `-ignore` or contain `github`/`bitbucket` are not shown.

## Development

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

`cargo test` includes golden fixtures recorded from the last Python release (`tests/golden/cases.json`); see `scripts/golden.py` and `docs/rust-migration.md`.

### Releases

Commits follow Conventional Commits (`<type>: <summary>`, in English, no scopes). release-please opens a release PR from them; merging it tags the release, and the tag builds the binaries, updates the Homebrew formula and publishes the crate (`.github/workflows/release.yml`).
