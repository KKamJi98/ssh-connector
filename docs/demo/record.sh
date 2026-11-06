#!/usr/bin/env bash
# Run from any directory. Only the renderer has access to system tools.
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"
if [[ -z "${SSH_CONNECTOR_DEMO_BIN:-}" ]]; then
    cargo build --release --locked
fi
renderer="$(command -v vhs)"
if [[ "$renderer" != /* || ! -x "$renderer" ]]; then
    printf 'Required executable not found: vhs\n' >&2
    exit 1
fi
# Resolve tools before setup changes HOME/PATH; custom installations work too.
tools=(ttyd ffmpeg bash clear stty env uname)
resolved=()
for tool in "${tools[@]}"; do
    candidate="$(command -v "$tool")"
    if [[ "$candidate" != /* || ! -x "$candidate" ]]; then
        printf 'Required executable not found: %s\n' "$tool" >&2
        exit 1
    fi
    resolved+=("$candidate")
done
source docs/demo/setup.sh
trap 'rm -rf -- "$DEMO_ROOT"' EXIT
for ((i = 0; i < ${#tools[@]}; i++)); do
    ln -s "${resolved[$i]}" "$DEMO_ROOT/bin/${tools[$i]}"
done
# Relative tape outputs point only at the approved artifact directory.
mkdir -p "$DEMO_ROOT/work/docs"
ln -s "$repo_root/docs/demo" "$DEMO_ROOT/work/docs/demo"
cd "$DEMO_ROOT/work"
# Start both the renderer and its shell with a clean, fictional identity.
env -i HOME="$HOME" USER=demo LOGNAME=demo \
    PATH="$DEMO_ROOT/bin" TMPDIR="${TMPDIR:-/tmp}" \
    PS1='$ ' PS2='> ' HISTFILE=/dev/null TERM=xterm-256color \
    GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 GIT_ALLOW_PROTOCOL=file \
    "$renderer" "$repo_root/docs/demo/ssh-connector.tape"
