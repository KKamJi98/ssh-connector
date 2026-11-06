#!/usr/bin/env python3
"""Golden fixtures: what the Python version (PyPI ssh-connector 0.3.1) prints.

    # Record the reference output (needs uv and a built fake ssh):
    cargo build --examples
    python3 scripts/golden.py generate

    # Compare any binary against the recorded fixtures:
    python3 scripts/golden.py compare target/debug/ssh-connector

    # After a deliberate behaviour change, record the new output and why:
    python3 scripts/golden.py bless target/debug/ssh-connector \
        --bug 3.1 --reason "ssh's exit code is passed through"

`generate` installs ssh-connector 0.3.1 with the exact click and rich
versions below into a throwaway venv, runs every case in CASES, and writes
tests/golden/cases.json. `cargo test` replays that file against the Rust
binary (tests/golden.rs), so CI needs neither Python nor network access.

Every case runs with a fresh HOME, a PATH that holds only the fake ssh (or
nothing), stdin and stdout as pipes, and no environment beyond what the case
sets. The temporary HOME is written as <HOME> in the recorded output.

cases.json is only ever written by `generate`: it is what Python printed.
Intended differences (bug fixes) live in changes.json, one entry per case
with the bug number, the reason and the new expected output, so every line
that no longer matches Python is accounted for.
"""

from __future__ import annotations

import argparse
import base64
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "tests" / "golden" / "cases.json"
CHANGES = ROOT / "tests" / "golden" / "changes.json"
REFERENCE = {"ssh-connector": "0.3.1", "click": "8.5.0", "rich": "15.0.0"}


def case(name, config=None, stdin="q\n", **kw):
    files = dict(kw.pop("files", {}))
    if config is not None:
        files[".ssh/config"] = config
    return {"name": name, "files": files, "stdin": stdin, **kw}


BASIC = "Host alpha\n  HostName 10.0.0.1\nHost beta gamma\nHost jump-a\n"
JUMPS = "Host web-1\nHost jump-prod\nInclude config.d/*\n"
LONG = "a" * 120

CASES = [
    # Command line.
    case("args-help", args=["--help"], stdin=""),
    case("args-help-extra", args=["--help", "extra"], stdin=""),
    case("args-short-h", args=["-h"], stdin=""),
    case("args-version", args=["--version"], stdin=""),
    case("args-unknown", args=["--bogus"], stdin=""),
    case("args-help-then-unknown", args=["--help", "--bogus"], stdin=""),
    case("args-extra-one", args=["extra"], stdin=""),
    case("args-extra-two", args=["a", "b"], stdin=""),
    case("args-dash", args=["-"], stdin=""),
    case("args-double-dash", args=["--"], config=BASIC),
    # Choosing a host.
    case("pick-quit", config=BASIC),
    case("pick-quit-upper", config=BASIC, stdin="Q\n"),
    case("pick-number", config=BASIC, stdin="2\n"),
    case("pick-name-case", config=BASIC, stdin="GAMMA\n"),
    case("pick-jump-number", config=BASIC, stdin="4\n"),
    case("pick-name-is-number", config="Host alpha\nHost beta\nHost 1\n", stdin="1\n"),
    case("pick-whitespace-number", config=BASIC, stdin=" 2 \n"),
    case("pick-plus-number", config=BASIC, stdin="+1\n"),
    case("pick-zero", config=BASIC, stdin="0\nq\n"),
    case("pick-negative", config=BASIC, stdin="-1\nq\n"),
    case("pick-too-big", config=BASIC, stdin="99\nq\n"),
    case("pick-huge", config=BASIC, stdin="99999999999999999999999999\nq\n"),
    case("pick-garbage", config=BASIC, stdin="nope\nq\n"),
    case("pick-space-only", config=BASIC, stdin=" \nq\n"),
    case("pick-empty-reprompt", config=BASIC, stdin="\n\nq\n"),
    case("pick-eof", config=BASIC, stdin=""),
    case("pick-eof-partial", config=BASIC, stdin="2"),
    case("filter-match", config=BASIC, stdin="f\nAL\n1\n"),
    case("filter-jump-only", config=BASIC, stdin="f\njump\nq\n"),
    case("filter-none", config=BASIC, stdin="f\nzzz\nq\n"),
    case("filter-empty-reprompt", config=BASIC, stdin="f\n\nbe\nq\n"),
    case("filter-eof", config=BASIC, stdin="f\n"),
    case("filter-then-number", config=BASIC, stdin="f\nma\n1\n"),
    # Running ssh.
    case("ssh-exit-1", config=BASIC, stdin="1\n", env={"FAKE_SSH_EXIT": "1"}),
    case("ssh-exit-255", config=BASIC, stdin="1\n", env={"FAKE_SSH_EXIT": "255"}),
    case("ssh-exit-130", config=BASIC, stdin="1\n", env={"FAKE_SSH_EXIT": "130"}),
    case("ssh-missing", config=BASIC, stdin="1\n", ssh=False),
    case("ssh-dash-host", config="Host -V\n", stdin="1\n"),
    # Reading the config.
    case("config-missing", config=None),
    case("config-empty", config=""),
    case(
        "config-only-ignored",
        config="Host x-ignore\nHost github.com\nHost my-bitbucket\n",
    ),
    case("config-abort-suffix", config="Host keep-abort\nHost drop-ignore\n"),
    case(
        "config-wildcards",
        config="Host *\nHost *.corp a.*\nHost q?wild\nHost neg !excluded\n",
    ),
    case(
        "config-keyword-forms",
        config="host lower\nHOST upper\n  Host indented\n\tHost\ttabbed\nHostName nothost\nHost=eq\nHosts plural\n",
    ),
    case("config-trailing-comment", config="Host trailing # comment here\n"),
    case("config-quoted", config='Host "quoted host" plain\n'),
    case("config-crlf", config="Host one\r\nHost two\r\n"),
    case("config-lone-cr", config="Host one\rHost two\r"),
    case("config-bom", config="﻿Host first\nHost second\n"),
    case("config-hangul", config="Host 서버-한글\nHost ok\n"),
    case(
        "config-duplicates",
        config="Host a\nHost a b\nInclude other\n",
        files={".ssh/other": "Host a\n"},
    ),
    case(
        "config-non-utf8-main",
        config_b64=base64.b64encode(b"# caf\xe9\nHost one\n").decode(),
    ),
    case(
        "config-non-utf8-start-byte",
        config_b64=base64.b64encode(b"Host one\n\xff\n").decode(),
    ),
    case(
        "config-non-utf8-truncated",
        config_b64=base64.b64encode(b"Host one\n\xe2\x82").decode(),
    ),
    case(
        "config-non-utf8-after-8k",
        config_b64=base64.b64encode(b"Host one\n" + b"#" * 8491 + b"\xff\n").decode(),
    ),
    case(
        "config-non-utf8-split-char",
        config_b64=base64.b64encode(
            b"Host one\n" + b"#" * 8181 + "\ud55c".encode() + b"#" * 100 + b"\xff\n"
        ).decode(),
    ),
    case(
        "config-non-utf8-include",
        config="Host main1\nInclude inc\n",
        files_b64={".ssh/inc": base64.b64encode(b"Host x\n# \xff\n").decode()},
    ),
    case(
        "include-nested-relative",
        config="Include config.d/*\nHost base1\n",
        files={
            ".ssh/config.d/01": "Host inc1\nInclude nested/*\n",
            ".ssh/config.d/nested/n": "Host nested-rel-to-file\n",
            ".ssh/nested/n": "Host nested-rel-to-ssh\n",
        },
    ),
    case(
        "include-order-sorted",
        config="Include b/* a/*\nHost m\n",
        files={".ssh/a/x": "Host from-a\n", ".ssh/b/x": "Host from-b\n"},
    ),
    case(
        "include-tilde",
        config="Include ~/.ssh/extra\n",
        files={".ssh/extra": "Host tilde-host\n"},
    ),
    case(
        "include-env-var",
        config="Include ${HOME}/.ssh/extra\nHost m\n",
        files={".ssh/extra": "Host env-host\n"},
    ),
    case("include-missing", config="Include nothing/*\nInclude also-missing\nHost m\n"),
    case(
        "include-cycle",
        config="Include config\nInclude loop\nHost m\n",
        files={".ssh/loop": "Include config\nHost l\n"},
    ),
    case(
        "include-hidden-glob",
        config="Include d/*\n",
        files={".ssh/d/.hidden": "Host hidden\n", ".ssh/d/shown": "Host shown\n"},
    ),
    case(
        "include-recursive-glob",
        config="Include d/**/*.conf\n",
        files={".ssh/d/a.conf": "Host top\n", ".ssh/d/x/y/b.conf": "Host deep\n"},
    ),
    case(
        "include-equals-form",
        config="Include=other\nHost m\n",
        files={".ssh/other": "Host o\n"},
    ),
    case(
        "include-in-host-block",
        config="Host m\n  Include other\n",
        files={".ssh/other": "Host o\n"},
    ),
    case(
        "groups-jump-across-files",
        config=JUMPS,
        files={
            ".ssh/config.d/10-dev": "Host dev-1\nHost jump-dev\n",
            ".ssh/config.d/20-only-jump": "Host JUMP-x\n",
        },
    ),
    case(
        "groups-same-basename",
        config="Host m1\nInclude a/hosts b/hosts Main\n",
        files={
            ".ssh/a/hosts": "Host a1\nHost m1\n",
            ".ssh/b/hosts": "Host b1\n",
            ".ssh/Main": "Host main-file\n",
        },
    ),
    case(
        "groups-long-title",
        config="Include config.d/*\n",
        files={
            ".ssh/config.d/a-very-long-include-file-name-for-wrapping.conf": "Host x\n"
        },
    ),
    case(
        "groups-long-title-spaces",
        config="Include config.d/*\n",
        files={".ssh/config.d/a b c d e f": "Host x\n"},
    ),
    case(
        "unix-permission-denied",
        config="Host m\nInclude locked\n",
        files={".ssh/locked": "Host secret\n"},
        modes={".ssh/locked": 0o000},
        unix_only=True,
    ),
    case(
        "unix-symlinked-config",
        config=None,
        files={
            "dotfiles/ssh/config": "Host linked\nInclude extra\n",
            "dotfiles/ssh/extra": "Host beside-target\n",
        },
        symlinks={".ssh/config": "../dotfiles/ssh/config"},
        unix_only=True,
    ),
    # Layout and colour.
    case("width-long-host-80", config=f"Host {LONG}\nHost short\n"),
    case(
        "width-long-host-30", config=f"Host {LONG}\nHost short\n", env={"COLUMNS": "30"}
    ),
    case(
        "width-long-host-200",
        config=f"Host {LONG}\nHost short\n",
        env={"COLUMNS": "200"},
    ),
    case("width-jump-30", config=f"Host jump-{LONG}\n", env={"COLUMNS": "30"}),
    case(
        "width-jump-group-30",
        config="Include config.d/*\n",
        files={".ssh/config.d/" + "g" * 40: "Host jump-" + "h" * 30 + "\n"},
        env={"COLUMNS": "30"},
    ),
    case("width-hangul-30", config="Host " + "한" * 20 + "\n", env={"COLUMNS": "30"}),
    case("width-many-hosts", config="".join(f"Host host-{i}\n" for i in range(1, 121))),
    case("width-columns-and-lines", config=BASIC, env={"COLUMNS": "40", "LINES": "10"}),
    case("width-columns-not-number", config=BASIC, env={"COLUMNS": "wide"}),
    case("color-force", config=BASIC, stdin="nope\nq\n", env={"FORCE_COLOR": "1"}),
    case("color-force-empty", config=BASIC, env={"FORCE_COLOR": ""}),
    case(
        "color-force-no-color", config=BASIC, env={"FORCE_COLOR": "1", "NO_COLOR": "1"}
    ),
    case("color-force-dumb", config=BASIC, env={"FORCE_COLOR": "1", "TERM": "dumb"}),
    case(
        "color-force-dumb-columns",
        config=BASIC,
        env={"FORCE_COLOR": "1", "TERM": "dumb", "COLUMNS": "30"},
    ),
    case("color-tty-compatible", config=BASIC, env={"TTY_COMPATIBLE": "1"}),
    case(
        "color-tty-compatible-off",
        config=BASIC,
        env={"TTY_COMPATIBLE": "0", "FORCE_COLOR": "1"},
    ),
    case(
        "color-force-long-title",
        config="Include config.d/*\n",
        files={
            ".ssh/config.d/a-very-long-include-file-name-for-wrapping.conf": "Host x\n"
        },
        env={"FORCE_COLOR": "1"},
    ),
    case(
        "color-force-truncated",
        config=f"Host {LONG}\n",
        env={"FORCE_COLOR": "1", "COLUMNS": "30"},
    ),
]


def setup_home(home: Path, c: dict) -> None:
    files = dict(c.get("files", {}))
    for rel, text in files.items():
        p = home / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(text.encode())
    for rel, b64 in c.get("files_b64", {}).items():
        p = home / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(base64.b64decode(b64))
    if "config_b64" in c:
        p = home / ".ssh" / "config"
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(base64.b64decode(c["config_b64"]))
    for rel, target in c.get("symlinks", {}).items():
        p = home / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.symlink_to(target)
    for rel, mode in c.get("modes", {}).items():
        (home / rel).chmod(mode)


def run_case(binary: str, c: dict, fake_ssh: Path) -> dict:
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        home = tmp / "home"
        home.mkdir()
        bindir = tmp / "bin"
        bindir.mkdir()
        if c.get("ssh", True):
            shutil.copy(fake_ssh, bindir / "ssh")
        setup_home(home, c)
        env = {"HOME": str(home), "PATH": f"{bindir}:/usr/bin:/bin"}
        env.update(c.get("env", {}))
        try:
            p = subprocess.run(
                [binary, *c.get("args", [])],
                input=c["stdin"].encode(),
                capture_output=True,
                env=env,
                timeout=20,
            )
        finally:
            for rel in c.get("modes", {}):
                (home / rel).chmod(0o644)
        out, err = p.stdout.decode(), p.stderr.decode()
        for h in sorted({str(home), str(home.resolve())}, key=len, reverse=True):
            out, err = out.replace(h, "<HOME>"), err.replace(h, "<HOME>")
        return {"stdout": out, "stderr": err, "code": p.returncode}


def fake_ssh_path() -> Path:
    p = ROOT / "target" / "debug" / "examples" / "fake-ssh"
    if not p.exists():
        sys.exit("build the fake ssh first: cargo build --examples")
    return p


def generate(args) -> None:
    fake = fake_ssh_path()
    with tempfile.TemporaryDirectory() as venv_root:
        venv = Path(venv_root) / "venv"
        subprocess.run(["uv", "venv", "-q", "-p", "3.13", str(venv)], check=True)
        pins = [f"{k}=={v}" for k, v in REFERENCE.items()]
        subprocess.run(
            [
                "uv",
                "pip",
                "install",
                "-q",
                "--python",
                str(venv / "bin" / "python"),
                *pins,
            ],
            check=True,
        )
        ref = str(venv / "bin" / "ssh-connector")
        cases = []
        for c in CASES:
            result = run_case(ref, c, fake)
            cases.append({**c, "expected": result})
    FIXTURE.parent.mkdir(parents=True, exist_ok=True)
    doc = {"reference": REFERENCE, "cases": cases}
    FIXTURE.write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")
    print(f"wrote {len(cases)} cases to {FIXTURE.relative_to(ROOT)}")


def load_changes() -> dict:
    if CHANGES.exists():
        return json.loads(CHANGES.read_text())["cases"]
    return {}


def compare(args) -> None:
    fake = fake_ssh_path()
    doc = json.loads(FIXTURE.read_text())
    changes = load_changes()
    failed = 0
    for c in doc["cases"]:
        got = run_case(args.binary, c, fake)
        want = changes.get(c["name"], {}).get("expected", c["expected"])
        if got != want:
            failed += 1
            print(f"--- {c['name']}")
            for k in ("code", "stdout", "stderr"):
                if got[k] != want[k]:
                    print(f"  {k}: expected {want[k]!r}")
                    print(f"  {k}:      got {got[k]!r}")
    print(f"{len(doc['cases']) - failed}/{len(doc['cases'])} cases match")
    sys.exit(1 if failed else 0)


def bless(args) -> None:
    fake = fake_ssh_path()
    doc = json.loads(FIXTURE.read_text())
    changes = load_changes()
    for c in doc["cases"]:
        name = c["name"]
        got = run_case(args.binary, c, fake)
        current = changes.get(name)
        if got == c["expected"]:
            if current is not None:
                print(f"back to the Python output: {name}")
                del changes[name]
            continue
        if current is not None and current["expected"] == got:
            continue
        bugs = [] if current is None else current["bug"].split(", ")
        reasons = [] if current is None else current["reason"].split("; ")
        if args.bug not in bugs:
            bugs.append(args.bug)
            reasons.append(args.reason)
        changes[name] = {"bug": ", ".join(bugs), "reason": "; ".join(reasons), "expected": got}
        print(f"changed: {name}")
    out = {"cases": dict(sorted(changes.items()))}
    CHANGES.write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")


def main() -> None:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(required=True)
    g = sub.add_parser("generate")
    g.set_defaults(func=generate)
    c = sub.add_parser("compare")
    c.add_argument("binary")
    c.set_defaults(func=compare)
    b = sub.add_parser("bless")
    b.add_argument("binary")
    b.add_argument("--bug", required=True)
    b.add_argument("--reason", required=True)
    b.set_defaults(func=bless)
    args = ap.parse_args()
    args.func(args)


if __name__ == "__main__":
    os.umask(0o022)
    main()
