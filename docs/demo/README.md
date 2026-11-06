# Reproduce the demo

The GIF and MP4 record the real release-mode binary. Only `ssh` is a stub.
It prints its exact arguments and a fictional connection banner; it never
opens a socket. The fixture has eight fictional aliases across `Main` and
an included file named `lab`. Names and addresses are invented, and all
addresses end in `.example.invalid`.

Prerequisites: Rust, Bash, Git, [VHS](https://github.com/charmbracelet/vhs),
`ttyd`, and `ffmpeg`. This tape was recorded with VHS 0.12.1 on macOS.
The `Menlo` font is available on macOS; select an installed monospace font
in the tape on other platforms.

```bash
bash docs/demo/record.sh
```

The script builds with `cargo build --release --locked`, makes a temporary
HOME and working directory, creates fictional SSH/AWS configs and a local
bare Git remote, and puts a fake `ssh` first on PATH. Only resolved rendering
tools are linked into that PATH; the user's full PATH is not inherited.
VHS starts with an empty environment plus explicitly listed variables.
The tape sets `$ ` in the live shell before recording and clears that hidden
setup. It never replaces the CLI's output. No real username, path, clock,
key, token, or agent socket is displayed. The temporary tree is removed when
recording exits. The two output files stay in this folder.

To inspect the same fixture without recording, use a subshell so your real
HOME and PATH are unchanged afterward:

```bash
cargo build --release --locked
(
    source docs/demo/setup.sh
    trap 'rm -rf -- "$DEMO_ROOT"' EXIT
    ssh-connector --list
    ssh-connector web-02
)
```

Do not substitute a real SSH config or the system SSH client for this demo.
Review extracted frames after every re-recording before publishing.
