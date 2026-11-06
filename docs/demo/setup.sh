#!/usr/bin/env bash
# Source this file in a subshell. It never reads your real SSH/AWS configuration.
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
binary="${SSH_CONNECTOR_DEMO_BIN:-$repo_root/target/release/ssh-connector}"
if [[ ! -x "$binary" ]]; then
    printf 'Build the binary first: cargo build --release --locked\n' >&2
    return 1 2>/dev/null || exit 1
fi
DEMO_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/ssh-connector-demo.XXXXXX")"
export DEMO_ROOT
mkdir -p "$DEMO_ROOT/home/.ssh/config.d" "$DEMO_ROOT/home/.aws" "$DEMO_ROOT/bin" "$DEMO_ROOT/work"
cp "$binary" "$DEMO_ROOT/bin/ssh-connector"
cat > "$DEMO_ROOT/home/.ssh/config" <<'CONFIG'
# Groups come from file names, not comments.
Include config.d/*

Host web-01
    HostName web-01.example.invalid
    User demo
Host db-primary
    HostName db-primary.example.invalid
    User demo
Host db-replica
    HostName db-replica.example.invalid
    User demo
Host bastion
    HostName bastion.example.invalid
    User demo
CONFIG
cat > "$DEMO_ROOT/home/.ssh/config.d/lab" <<'CONFIG'
Host web-02
    HostName web-02.example.invalid
    User demo
Host web-03
    HostName web-03.example.invalid
    User demo
Host api-01
    HostName api-01.example.invalid
    User demo
Host worker-01
    HostName worker-01.example.invalid
    User demo
CONFIG
cat > "$DEMO_ROOT/home/.aws/config" <<'CONFIG'
[profile demo]
region = us-east-1
CONFIG
cat > "$DEMO_ROOT/bin/ssh" <<'SSH'
#!/bin/sh
printf '[demo ssh] argv:'
for arg do printf ' <%s>' "$arg"; done
printf '\n[demo ssh] Connected to a fictional host. No network used.\n'
SSH
chmod +x "$DEMO_ROOT/bin/ssh"
# A plain prompt, no personal startup files, no agent socket or credentials.
export HOME="$DEMO_ROOT/home" USER=demo LOGNAME=demo
export PS1='$ ' PS2='> ' HISTFILE=/dev/null
unset PROMPT_COMMAND SSH_AUTH_SOCK SSH_AGENT_PID BASH_ENV ENV
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
git init --quiet --bare "$DEMO_ROOT/origin.git"
(cd "$DEMO_ROOT/work" && git init --quiet && git remote add origin "$DEMO_ROOT/origin.git")
export PATH="$DEMO_ROOT/bin:$PATH"
