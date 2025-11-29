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

- Lists all hosts defined in your `~/.ssh/config` and follows `Include` directives (e.g., `Include ~/.ssh/config.d/*`).
- Allows you to filter and select a host to connect to.
- Automatically connects to the selected host using the `ssh` command.
- Groups hosts by the source file (main config shown as `Default`) and renders `jump` hosts together at the bottom with their originating group.

## Installation

### Prerequisites

- Python 3.13+
- `uv` (recommended)

### Using uv

```bash
uv pip install .
```

## Usage

Simply run the following command in your terminal:

```bash
ssh-connector
```

This will display a list of your configured SSH hosts. You can then select a host to connect to.

### How hosts are discovered

- The tool reads `~/.ssh/config` and follows any `Include` directives it finds (globs are supported).
- If you want to split entries into `~/.ssh/config.d/`, ensure your main config includes them, for example:

```text
Include ~/.ssh/config.d/*

Host base
    HostName 192.168.0.10
```

- Hosts are grouped by their source file name when displayed; entries from the main config appear under `Default`. Hostnames containing `jump` (case-insensitive) are also shown in a dedicated `JUMP-HOSTS` section with their source group.

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
