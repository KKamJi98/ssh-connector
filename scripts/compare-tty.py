#!/usr/bin/env python3
"""Compare the Python reference and a Rust binary on a real pseudo-terminal.

    cargo build --examples
    python3 scripts/compare-tty.py <python ssh-connector> target/debug/ssh-connector

The golden fixtures (scripts/golden.py) use pipes. This covers what only a
terminal shows: colour decided by isatty, the terminal echo of ^C and ^D,
and Ctrl-C sent to the whole foreground process group while ssh runs.
Unix only. Output is compared byte for byte after the terminal settles.
"""

from __future__ import annotations

import os
import pty
import select
import shutil
import signal
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONFIG = "Host alpha\nHost beta\nHost jump-a\n"

# name: (keys to type with pauses, extra env)
SCENARIOS = {
    "quit": ([b"q\r"], {}),
    "invalid-then-quit": ([b"nope\r", b"q\r"], {}),
    "invalid-number": ([b"9\r", b"q\r"], {}),
    "filter-none": ([b"f\r", b"zzz\r", b"q\r"], {}),
    "connect": ([b"1\r"], {}),
    "connect-fails": ([b"1\r"], {"FAKE_SSH_EXIT": "255"}),
    "ctrl-c-at-prompt": ([b"\x03"], {}),
    "ctrl-d-at-prompt": ([b"\x04"], {}),
    "ctrl-c-at-filter": ([b"f\r", b"\x03"], {}),
    "ctrl-c-while-ssh": ([b"1\r", b"\x03"], {"FAKE_SSH_SLEEP_MS": "3000"}),
    "no-color": ([b"q\r"], {"NO_COLOR": "1"}),
    "dumb": ([b"q\r"], {"TERM": "dumb"}),
    "narrow": ([b"q\r"], {"COLUMNS": "12"}),
}


def run(binary: str, keys, env_extra, fake_ssh: Path):
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        (tmp / "home" / ".ssh").mkdir(parents=True)
        (tmp / "home" / ".ssh" / "config").write_text(CONFIG)
        (tmp / "bin").mkdir()
        shutil.copy(fake_ssh, tmp / "bin" / "ssh")
        env = {
            "HOME": str(tmp / "home"),
            "PATH": f"{tmp / 'bin'}:/usr/bin:/bin",
            "TERM": "xterm-256color",
            **env_extra,
        }
        pid, fd = pty.fork()
        if pid == 0:
            os.execve(binary, [binary], env)
        out = bytearray()

        def drain(seconds):
            end = time.time() + seconds
            while time.time() < end:
                r, _, _ = select.select([fd], [], [], 0.05)
                if r:
                    try:
                        data = os.read(fd, 65536)
                    except OSError:
                        return
                    if not data:
                        return
                    out.extend(data)

        drain(1.0)
        for k in keys:
            os.write(fd, k)
            drain(0.8)
        drain(1.0)
        try:
            _, status = os.waitpid(pid, 0)
        except ChildProcessError:
            status = 0
        drain(0.2)
        os.close(fd)
        return bytes(out), os.waitstatus_to_exitcode(status)


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    ref, rust = sys.argv[1], sys.argv[2]
    fake = ROOT / "target" / "debug" / "examples" / "fake-ssh"
    failed = 0
    for name, (keys, env) in SCENARIOS.items():
        a = run(ref, keys, env, fake)
        b = run(rust, keys, env, fake)
        same = a == b
        failed += not same
        print(f"{'same' if same else 'DIFF'}  {name}  exit={a[1]}/{b[1]}")
        if not same:
            print(f"   python: {a[0][-400:]!r}")
            print(f"   rust:   {b[0][-400:]!r}")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    signal.signal(signal.SIGINT, signal.SIG_DFL)
    main()
