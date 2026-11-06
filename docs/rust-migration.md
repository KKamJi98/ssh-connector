# Rust migration notes

This file records what was checked, decided and measured while moving
ssh-connector from Python (PyPI `ssh-connector` 0.3.1, click + rich) to
Rust. The plan it follows was written on 2026-11-06; where the plan and the
real state of tools or APIs differed, the real state won and the reason is
written here.

## Phase 0: spike results (2026-11-06)

### Release tooling: hand-written GitHub Actions, not dist

- dist (axodotdev) publishes Homebrew formulas with a personal access token
  stored as `HOMEBREW_TAP_TOKEN` (dist book, "Homebrew" installer page). The
  plan pushes to the tap with a per-repo SSH deploy key
  (`HOMEBREW_TAP_DEPLOY_KEY`) and never with a PAT.
- dist creates the GitHub Release itself. Here release-please already
  creates it, so dist would have to be configured around a release that
  exists, and its crates.io story is a custom job anyway.
- The prjump migration, which would have been the reference, had not
  recorded its own Phase 0 result yet (`prjump/docs/rust-migration.md` did
  not exist on 2026-11-06).
- Decision: hand-written workflows. Every job is small (build five targets,
  upload, attest, update one formula, `cargo publish`).

### release-please with `release-type: rust`

- release-please v17.11.2 (action v5.0.0) `src/strategies/rust.ts` updates
  `Cargo.toml` (`CargoToml` updater) and `Cargo.lock` (`CargoLock`
  updater), so `cargo publish --locked` stays consistent after a release PR.
- The App installation token is what lets release-please's tag start other
  workflows. GitHub does not start workflow runs for events caused by
  `GITHUB_TOKEN`; it does for an App token. The existing release workflow
  already relied on this for the release PR's CI.
- release-please creates the tag through the GitHub Releases API. The new
  publish workflow listens to `push: tags: ['v*']`. This was not proven
  before the first release; if a release-please tag ever fails to start it,
  the fallback is `release: types: [published]`, which also runs on the tag
  ref and so passes the `release` environment's tag-only policy.

### Ctrl-C and ssh (macOS, verified by running both versions)

- The Python version does not exec ssh. It runs it as a child
  (`subprocess.run`), with the terminal inherited.
- Ctrl-C at a prompt: click turns it into `Aborted!` on stderr, exit 1.
- Ctrl-C while ssh runs: Python gets KeyboardInterrupt, waits up to 0.25 s
  for ssh, kills it, prints a blank line and `Exiting.`, exit 0.
- SIGQUIT is not handled by Python (default action). The plan said to ignore
  SIGQUIT while ssh runs; parity wins, so it keeps the default action.
- The Rust version installs one `ctrlc` handler and decides by state:
  prompt, ssh running, or elsewhere. A handler (not `SIG_IGN`) is used so
  ssh starts with the default SIGINT disposition.

### Home directory

- Python `Path.home()`: `$HOME`, then the passwd entry on Unix;
  `USERPROFILE`, then `HOMEDRIVE` + `HOMEPATH` on Windows.
- `std::env::home_dir` changed on Windows in Rust 1.86 and its exact lookup
  order differs between doc versions, so the port does not rely on it for
  Windows. It reads the variables itself and uses `home_dir` only for the
  Unix passwd fallback.

### Windows (partly verified)

- Ethan's Windows host (reached through WSL interop) has
  `C:\Windows\System32\OpenSSH\ssh.exe`, `OpenSSH_for_Windows_9.5p2`.
- That `ssh.exe` does process a client-side `Include` with a Windows path
  and a glob: `ssh -F <config> -G host` expanded `Include ...\sub\*.conf`
  and opened the matched file. It then refused the file with "Bad
  permissions" because files created from WSL or PowerShell in `%TEMP%`
  carried an unknown SID in their ACL, so the parsed result could not be
  checked. Upstream Win32-OpenSSH issues #2192, #2398 and #2019 are about
  this area.
- Not verified on Windows: Ctrl-C delivery to the console process group
  while `ssh.exe` runs. CI runs the unit, golden and integration tests on
  `windows-latest` with a fake `ssh.exe`.

### Table rendering

- rich 15.0.0's layout was read from source (`Table._calculate_column_widths`,
  `_collapse_widths`, `ratio_reduce`, `Text.wrap`, `divide_line`,
  `chop_cells`, `set_cell_size`) and reproduced for the subset used: a
  `DOUBLE_EDGE` box, a centred title, a bold header and one cell of padding.
- rich writes every padding and content piece as its own styled segment, so
  even the ANSI bytes can be compared exactly (with `FORCE_COLOR=1`).
