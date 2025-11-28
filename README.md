# SSH Connector

[![CI](https://github.com/KKamJi98/ssh-connector/actions/workflows/ci.yml/badge.svg)](https://github.com/KKamJi98/ssh-connector/actions/workflows/ci.yml)

A CLI tool to simplify SSH connections by providing an interactive menu of hosts from your `~/.ssh/config` file.

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

Host stg-db-1
    Hostname 192.168.2.11
    User stguser
    ProxyJump jump-stg

Host prod-app-1
    Hostname 10.0.0.10
    User produser
    ProxyJump jump-prod

Host prod-db-1
    Hostname 10.0.0.11
    User produser
    ProxyJump jump-prod

Host jump-dev
    Hostname 172.16.0.1
    User jumpdev

Host jump-stg
    Hostname 172.16.0.2
    User jumpstg

Host jump-prod
    Hostname 172.16.0.3
    User jumpprod
```

When you run `ssh-connector`, you will see an interactive menu like this:

```text
      SSH Hosts
╔═════╤═════════════╗
║ No. │ Host        ║
╟─────┼─────────────╢
║ 1   │ dev-app-1   ║
║ 2   │ dev-db-1    ║
║ 3   │ stg-app-1   ║
║ 4   │ stg-db-1    ║
║ 5   │ prod-app-1  ║
║ 6   │ prod-db-1   ║
╚═════╧═════════════╝
     JUMP-HOSTS
╔═════╤═════════════╗
║ No. │ Host        ║
╟─────┼─────────────╢
║ 7   │ jump-dev    ║
║ 8   │ jump-stg    ║
║ 9   │ jump-prod   ║
╚═════╧═════════════╝
Enter the number of the host to connect to (or 'q' to quit, 'f' to filter):
```

After selecting a host (e.g., `prod-app-1`), the tool will execute the appropriate SSH command, handling `ProxyJump` automatically if configured.

## Features

- Lists all hosts defined in your `~/.ssh/config`, follows `Include` directives (e.g., `Include ~/.ssh/config.d/*`), and groups entries by source file (`Default` for the main config).
- Automatically separates hostnames containing `jump` (case-insensitive) into a dedicated "JUMP-HOSTS" table at the bottom with their source group.
- Ignores hosts whose names end with `-abort` (case-insensitive) so they do not appear in the menu.
- Allows you to filter and select a host to connect to.
- Automatically connects to the selected host using the `ssh` command.

## Installation

### Prerequisites

- Python 3.13+
- `uv` (recommended) or `pipx`

### Recommended: uv tool (global shim)

Creates an isolated environment and installs a shim (usually in `~/.local/bin`) so you can run `ssh-connector` from any directory. Ensure that directory is on your `PATH`.

```bash
uv tool install .
ssh-connector --help
```

### Alternative: uv pip (user install)

```bash
uv pip install . --user
ssh-connector --help
```

### Alternative: pipx

```bash
pipx install .
ssh-connector --help
```

## Usage

Simply run the following command in your terminal:

```bash
ssh-connector
```

This will display a list of your configured SSH hosts. You can then select a host to connect to.

### How hosts are discovered and displayed

- The tool reads `~/.ssh/config` and follows any `Include` directives it finds (globs are supported).
- If you want to split entries into `~/.ssh/config.d/`, ensure your main config includes them, for example:

```text
Include ~/.ssh/config.d/*

Host base
    HostName 192.168.0.10
```

- Hosts are grouped by the file they come from (`Default` for `~/.ssh/config`, and the filename for included configs).
- Any host name containing `jump` (case-insensitive) is collected into a final "JUMP-HOSTS" section, with its source group shown in a separate column.
- Hosts whose names end with `-abort` are filtered out entirely.
- Press `f` to filter by substring (case-insensitive) before selecting a host.

## Development

### Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/KKamJi98/ssh-connector.git
   cd ssh-connector
   ```

2. Sync dev dependencies with uv:
   ```bash
   uv sync --group dev
   ```

### Running Tests

Run tests via uv:

```bash
uv run --group dev pytest -q
```

### Formatting

This project uses `ruff` for linting and formatting.

```bash
# Lint
uv run --group dev ruff check .

# Format in-place
uv run --group dev ruff format

# Format check (CI-style)
uv run --group dev ruff format --check .
```

### Commit Messages

This repository uses Conventional Commits and release-please.
Use `<type>: <summary>` in English without scopes/emojis.
See `AGENTS.md` for details.
