import re
import subprocess
from pathlib import Path
from typing import Dict, Iterable, List, Set, Tuple

import click
from rich import box
from rich.console import Console
from rich.table import Table


def _is_ignored_host(host: str) -> bool:
    """Return True if host should be skipped from display/selection."""

    lowered = host.lower()
    return lowered.endswith("-ignore") or "github" in lowered or "bitbucket" in lowered


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


def _parse_config_file(path: Path, visited: Set[Path]) -> List[Tuple[str, Path]]:
    """Parse a single ssh config file, following Include directives recursively.

    - Collects Host values (excluding wildcard patterns with '*').
    - Follows Include directives relative to current file directory.
    - Avoids cycles via visited set.
    """
    hosts: List[Tuple[str, Path]] = []
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
                            hosts.append((hv, real_path))
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
    entries = _parse_config_file(ssh_config_path, visited)
    # Deduplicate host names while preserving order
    ordered_unique: Dict[str, None] = {}
    for host, _src in entries:
        if _is_ignored_host(host):
            continue
        if host not in ordered_unique:
            ordered_unique[host] = None
    return list(ordered_unique.keys())


def get_ssh_hosts_grouped(
    config_path: Path | None = None,
) -> Tuple[Dict[str, List[str]], List[str]]:
    """Return hosts grouped by their source file.

    - "Default" group corresponds to hosts defined in the main config file.
    - Included files are grouped by their filename (Path.name).
    - Returns a tuple of (grouped_hosts, group_order).
    """
    ssh_config_path = config_path or (Path.home() / ".ssh" / "config")
    if not ssh_config_path.is_file():
        return {}, []

    visited: Set[Path] = set()
    entries = _parse_config_file(ssh_config_path, visited)

    grouped: Dict[str, List[str]] = {}
    group_order: List[str] = []
    for host, src in entries:
        if _is_ignored_host(host):
            continue
        group = "Default" if src == ssh_config_path.resolve() else src.name
        if group not in grouped:
            grouped[group] = []
            group_order.append(group)
        # Deduplicate within a group while preserving order
        if host not in grouped[group]:
            grouped[group].append(host)

    return grouped, group_order


@click.command()
def cli():
    """
    A CLI tool to list hosts from ~/.ssh/config and connect to them.
    """
    grouped, group_order = get_ssh_hosts_grouped()
    all_available_hosts = [h for g in group_order for h in grouped.get(g, [])]
    if not all_available_hosts:
        click.echo("No valid SSH hosts found.")
        return

    console = Console()
    filter_term = ""  # Initialize filter_term

    while True:
        selectable_hosts: List[str] = []
        current_display_index = 1

        # Build a filtered view of groups → hosts
        def filter_hosts(hosts: List[str]) -> List[str]:
            if not filter_term:
                return hosts
            ft = filter_term.lower()
            return [h for h in hosts if ft in h.lower()]

        any_host_displayed = False

        # First pass: render non-jump hosts per group
        aggregated_jump: List[Tuple[str, str]] = []  # (host, group)
        for group in group_order:
            hosts_in_group = grouped.get(group, [])
            hosts_in_group = filter_hosts(hosts_in_group)
            if not hosts_in_group:
                continue

            # Split
            normal_hosts = [h for h in hosts_in_group if "jump" not in h.lower()]
            jump_hosts = [h for h in hosts_in_group if "jump" in h.lower()]

            # Queue jump hosts for the final section
            aggregated_jump.extend((h, group) for h in jump_hosts)

            if not normal_hosts:
                continue

            any_host_displayed = True
            table = Table(
                title=f"[bold cyan]{group} ({len(normal_hosts)})[/bold cyan]",
                box=box.DOUBLE_EDGE,
                border_style="cyan",
            )
            table.add_column("No.", style="cyan", no_wrap=True)
            table.add_column("Host", style="magenta")

            for host in normal_hosts:
                selectable_hosts.append(host)
                table.add_row(str(current_display_index), host)
                current_display_index += 1
            console.print(table)

        # Second pass: render all jump hosts at the bottom, with group shown
        if aggregated_jump:
            any_host_displayed = True
            jump_table = Table(
                title=f"[bold yellow]JUMP-HOSTS ({len(aggregated_jump)})[/bold yellow]",
                box=box.DOUBLE_EDGE,
                border_style="yellow",
            )
            jump_table.add_column("No.", style="cyan", no_wrap=True)
            jump_table.add_column("Host", style="magenta")
            jump_table.add_column("Group", style="green")
            for host, group in aggregated_jump:
                selectable_hosts.append(host)
                jump_table.add_row(str(current_display_index), host, group)
                current_display_index += 1
            console.print(jump_table)

        if not any_host_displayed:
            click.echo(click.style("No hosts found matching the filter.", fg="red"))
            filter_term = ""
            continue

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
