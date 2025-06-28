import os
import re
import subprocess
from pathlib import Path

import click


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

    click.echo(click.style("SSH Hosts available:", fg="green"))
    for i, host in enumerate(hosts, 1):
        click.echo(f"[{i}] {host}")

    while True:
        try:
            choice_str = click.prompt(
                "Enter the number of the host to connect to (or 'q' to quit)", type=str
            )

            if choice_str.lower() == "q":
                click.echo("Exiting.")
                break

            choice = int(choice_str)
            if 1 <= choice <= len(hosts):
                selected_host = hosts[choice - 1]
                click.echo(
                    click.style(f"Connecting to {selected_host}...", fg="yellow")
                )

                # Use subprocess.run to execute the ssh command
                # This will hand over control of the terminal to the ssh process
                try:
                    # Using os.execvp is better as it replaces the current process
                    # with ssh, making it feel like a native command.
                    # However, it's less portable and doesn't return.
                    # subprocess.run is a safer, more universal choice.
                    subprocess.run(["ssh", selected_host], check=True)
                except FileNotFoundError:
                    click.echo(
                        click.style(
                            "Error: 'ssh' command not found. Is OpenSSH client installed and in your PATH?",
                            fg="red",
                        )
                    )
                except subprocess.CalledProcessError as e:
                    # This error is often just the user exiting the ssh session
                    # with a non-zero status, so we can often ignore it.
                    click.echo(
                        click.style(
                            f"SSH session for {selected_host} ended.", fg="blue"
                        )
                    )
                break  # Exit loop after attempting connection
            else:
                click.echo(click.style("Invalid number. Please try again.", fg="red"))
        except ValueError:
            click.echo(
                click.style("Invalid input. Please enter a number or 'q'.", fg="red")
            )
        except (EOFError, KeyboardInterrupt):
            click.echo("\nExiting.")
            break


if __name__ == "__main__":
    cli()
