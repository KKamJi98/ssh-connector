import re
import subprocess
from pathlib import Path
from typing import Iterable, List, Set

import click
from rich import box
from rich.console import Console
from rich.table import Table


def _expand_include_patterns(patterns: Iterable[str], base_dir: Path) -> List[Path]:
    """Expand Include patterns relative to base_dir, supporting ~ and globs.

    Returns files sorted lexicographically to be deterministic.
    """
    import glob
    import os

    expanded: Set[Path] = set()
    for pat in patterns:
        pat = os.path.expanduser(pat)
        pat_path = Path(pat)
        if not pat_path.is_absolute():
            pat_path = base_dir / pat
        # Use glob with recursive patterns
        matches = [Path(p) for p in glob.glob(str(pat_path), recursive=True)]
        for m in matches:
            if m.is_file():
                expanded.add(m)
    return sorted(expanded)


def _parse_config_file(path: Path, visited: Set[Path]) -> List[str]:
    """Parse a single ssh config file, following Include directives recursively.

    - Collects Host values (excluding wildcard patterns with '*').
    - Follows Include directives relative to current file directory.
    - Avoids cycles via visited set.
    """
    hosts: List[str] = []
    include_patterns: list[list[str]] = []
    try:
        real_path = path.resolve()
        if real_path in visited:
            return []
        visited.add(real_path)
        if not real_path.is_file():
            return []

        base_dir = real_path.parent
        with open(real_path, "r") as f:
            for raw_line in f:
                line = raw_line.strip()
                if not line or line.startswith("#"):
                    continue
                # Include directive
                if re.match(r"^Include\b", line, re.IGNORECASE):
                    # Record include patterns; defer processing until after local hosts
                    parts = line.split()[1:]
                    if parts:
                        include_patterns.append(parts)
                    continue
                # Host directive
                if re.match(r"^Host\b", line, re.IGNORECASE):
                    host_values = line.split()[1:]
                    for hv in host_values:
                        if "*" not in hv:
                            hosts.append(hv)
        # After collecting local hosts, process includes in order
        for parts in include_patterns:
            include_files = _expand_include_patterns(parts, base_dir)
            for inc in include_files:
                hosts.extend(_parse_config_file(inc, visited))
    except Exception as e:
        click.echo(
            click.style(f"Error reading or parsing SSH config file: {e}", fg="red")
        )
        return []
    return hosts


def get_ssh_hosts(config_path: Path | None = None) -> List[str]:
    """Parse SSH config, following Include directives, and return host list.

    - Default config path: ``~/.ssh/config``.
    - Returns unique hosts preserving discovery order.
    - Excludes wildcard host patterns.
    """
    ssh_config_path = config_path or (Path.home() / ".ssh" / "config")
    if not ssh_config_path.is_file():
        click.echo(
            click.style(
                f"Error: SSH config file not found at {ssh_config_path}", fg="red"
            )
        )
        return []

    visited: Set[Path] = set()
    hosts = _parse_config_file(ssh_config_path, visited)
    # Deduplicate while preserving order
    return list(dict.fromkeys(hosts))


@click.command()
def cli():
    """
    A CLI tool to list hosts from ~/.ssh/config and connect to them.
    """
    all_available_hosts = get_ssh_hosts()
    if not all_available_hosts:
        click.echo("No valid SSH hosts found.")
        return

    console = Console()
    filter_term = ""  # Initialize filter_term

    while True:
        selectable_hosts = []  # Initialize selectable_hosts at the beginning of each loop iteration
        current_display_index = 1  # Initialize current_display_index here as well
        table = Table(
            title="[bold cyan]SSH Hosts[/bold cyan]",
            box=box.DOUBLE_EDGE,
            border_style="cyan",
        )
        table.add_column("No.", style="cyan", no_wrap=True)
        table.add_column("Host", style="magenta")

        # Apply filter if present
        if filter_term:
            current_display_hosts = [
                host
                for host in all_available_hosts
                if filter_term.lower() in host.lower()
            ]
        else:
            current_display_hosts = all_available_hosts

        if not current_display_hosts:
            click.echo(click.style("No hosts found matching the filter.", fg="red"))
            filter_term = ""  # Reset filter if no matches
            continue

        normal_hosts = [
            host for host in current_display_hosts if "jump" not in host.lower()
        ]
        jump_hosts = [host for host in current_display_hosts if "jump" in host.lower()]

        for host in normal_hosts:
            selectable_hosts.append(host)
            table.add_row(str(current_display_index), host)
            current_display_index += 1

        console.print(table)

        if jump_hosts:
            jump_table = Table(
                title="[bold yellow]JUMP-HOSTS[/bold yellow]",
                box=box.DOUBLE_EDGE,
                border_style="yellow",
            )
            jump_table.add_column("No.", style="cyan", no_wrap=True)
            jump_table.add_column("Host", style="magenta")
            for host in jump_hosts:
                selectable_hosts.append(host)
                jump_table.add_row(str(current_display_index), host)
                current_display_index += 1
            console.print(jump_table)

        try:
            choice_str = click.prompt(
                "Enter the number of the host to connect to (or 'q' to quit, 'f' to filter)",
                type=str,
            )

            if choice_str.lower() == "q":
                click.echo("Exiting.")
                break
            elif choice_str.lower() == "f":
                filter_term = click.prompt("Enter filter term", type=str)
                continue

            choice = int(choice_str)
            if 1 <= choice <= len(selectable_hosts):
                selected_host = selectable_hosts[choice - 1]
                click.echo(
                    click.style(f"Connecting to {selected_host}...", fg="yellow")
                )

                try:
                    subprocess.run(["ssh", selected_host], check=True)
                except FileNotFoundError:
                    click.echo(
                        click.style(
                            "Error: 'ssh' command not found. Is OpenSSH client installed and in your PATH?",
                            fg="red",
                        )
                    )
                except subprocess.CalledProcessError:
                    click.echo(
                        click.style(
                            f"SSH session for {selected_host} ended.", fg="blue"
                        )
                    )
                break
            else:
                click.echo(click.style("Invalid number. Please try again.", fg="red"))
        except ValueError:
            click.echo(
                click.style(
                    "Invalid input. Please enter a number, 'q', or 'f'.", fg="red"
                )
            )
        except (EOFError, KeyboardInterrupt):
            click.echo("\nExiting.")
            break


if __name__ == "__main__":
    cli()
