import os
import sys

import pytest

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))
from unittest.mock import mock_open, patch

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
