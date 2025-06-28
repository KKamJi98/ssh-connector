# SSH Connector

### Example

Let's assume your `~/.ssh/config` file contains the following entries:

```
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

```
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

A CLI tool to simplify SSH connections by providing an interactive menu of hosts from your `~/.ssh/config` file.

## Features

- Lists all hosts defined in your `~/.ssh/config`.
- Allows you to filter and select a host to connect to.
- Automatically connects to the selected host using the `ssh` command.

## Installation

### Prerequisites

- Python 3.13+
- `uv` (recommended)

### Using pip

```bash
pip install .
```

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

## Development

### Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/KKamJi98/ssh-connector.git
   cd ssh-connector
   ```

2. Create a virtual environment and install dependencies:
   ```bash
   python -m venv .venv
   source .venv/bin/activate
   uv pip install -e ".[dev]"
   ```

### Running Tests

This project uses `pytest` for testing. To run the tests, use the following command:

```bash
pytest
```

### Formatting

This project uses `black` and `isort` for code formatting. To format the code, run:

```bash
black .
isort .
```