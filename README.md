# SSH Connector

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
   git clone https://github.com/your-username/ssh-connector.git
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