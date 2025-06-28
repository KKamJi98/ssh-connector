import os
import re
import subprocess
from pathlib import Path

import click
from rich.console import Console
from rich.table import Table


def get_ssh_hosts():
    """
    Parses the ~/.ssh/config file to extract hostnames.

    Returns:
        A list of hostnames, excluding any with wildcards.
    """
    ssh_config_path = Path.home() / ".ssh" / "config"
    if not ssh_config_path.is_file():
        click.echo(
            click.style(
                f"Error: SSH config file not found at {ssh_config_path}", fg="red"
            )
        )
        return []

    hosts = []
    try:
        with open(ssh_config_path, "r") as f:
            for line in f:
                line = line.strip()
                if re.match(r"^\s*Host\s+", line, re.IGNORECASE):
                    # Extract host value, stripping "Host " prefix
                    host_values = line.split()[1:]
                    for host in host_values:
                        # Exclude wildcard hosts
                        if "*" not in host:
                            hosts.append(host)
    except Exception as e:
        click.echo(
            click.style(f"Error reading or parsing SSH config file: {e}", fg="red")
        )
        return []

    return list(dict.fromkeys(hosts))  # Return unique hosts in order of appearance


@click.command()
def cli():
    """
    A CLI tool to list hosts from ~/.ssh/config and connect to them.
    """
    hosts = get_ssh_hosts()

    if not hosts:
        click.echo("No valid SSH hosts found.")
        return

    console = Console()

    while True:
        table = Table(title="SSH Hosts available:")
        table.add_column("No.", style="cyan", no_wrap=True)
        table.add_column("Host", style="magenta")

        display_hosts = []
        for i, host in enumerate(hosts, 1):
            display_hosts.append((str(i), host))

        for row in display_hosts:
            table.add_row(*row)

        console.print(table)

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
                hosts = [host for host in hosts if filter_term.lower() in host.lower()]
                if not hosts:
                    click.echo(
                        click.style("No hosts found matching the filter.", fg="red")
                    )
                continue  # Restart the loop to display filtered hosts

            choice = int(choice_str)
            if 1 <= choice <= len(hosts):
                selected_host = hosts[choice - 1]
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
                except subprocess.CalledProcessError as e:
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
