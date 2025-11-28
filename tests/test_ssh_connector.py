import pytest
from unittest.mock import mock_open

from pathlib import Path

from ssh_connector import get_ssh_hosts


@pytest.fixture
def mock_ssh_config_file(monkeypatch):
    def _mock_file(content):
        m = mock_open(read_data=content)
        monkeypatch.setattr("pathlib.Path.is_file", lambda self: True)
        monkeypatch.setattr("builtins.open", m)

    return _mock_file


def test_get_ssh_hosts_with_valid_config(mock_ssh_config_file):
    config_content = """
    Host server1
        HostName 192.168.1.1

    Host server2
        HostName 192.168.1.2

    Host *.example.com
    """
    mock_ssh_config_file(config_content)
    hosts = get_ssh_hosts()
    assert hosts == ["server1", "server2"]


def test_get_ssh_hosts_with_no_config_file(monkeypatch):
    monkeypatch.setattr("pathlib.Path.is_file", lambda self: False)
    hosts = get_ssh_hosts()
    assert hosts == []


def test_get_ssh_hosts_with_empty_config(mock_ssh_config_file):
    mock_ssh_config_file("")
    hosts = get_ssh_hosts()
    assert hosts == []


def test_get_ssh_hosts_excludes_ignore_and_vcs_hosts(mock_ssh_config_file):
    config_content = """
    Host server1-ignore
    Host server2
    Host github-enterprise
    Host BITBUCKET-jump
    """
    mock_ssh_config_file(config_content)
    hosts = get_ssh_hosts()
    assert hosts == ["server2"]


def test_get_ssh_hosts_with_include(tmp_path: Path):
    # Create directory structure
    ssh_dir = tmp_path / ".ssh"
    config_dir = ssh_dir / "config.d"
    config_dir.mkdir(parents=True)

    # Base config includes all files in config.d and has one host
    base_config = (
        "Include config.d/*\n# Comment line\nHost base1\n    HostName 10.0.0.1\n"
    )
    (ssh_dir / "config").write_text(base_config)

    # Included configs
    (config_dir / "01-hosts").write_text("Host inc1 inc2\n    HostName example\n")
    # A wildcard host should be ignored
    (config_dir / "02-wild").write_text("Host *.example.com\n")

    # Another layer of include from included file
    nested_dir = config_dir / "nested"
    nested_dir.mkdir()
    (config_dir / "03-include-nested").write_text("Include nested/*\n")
    (nested_dir / "10-nested-host").write_text("Host nested1\n")

    hosts = get_ssh_hosts(ssh_dir / "config")
    # Order should follow discovery order: base -> 01 -> 02(ignored) -> 03 -> nested
    assert hosts == ["base1", "inc1", "inc2", "nested1"]
