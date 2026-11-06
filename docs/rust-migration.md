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

## Phase 1: parity port

### How parity is checked

- `tests/golden/cases.json` holds 91 cases recorded from PyPI
  `ssh-connector==0.3.1` with `click==8.5.0` and `rich==15.0.0`
  (`scripts/golden.py generate`). Each case has its own HOME, a PATH with
  only a fake ssh (`examples/fake-ssh.rs`), piped stdin and stdout.
  `cargo test` replays them; stdout, stderr and the exit code must match
  byte for byte. CI does not need Python.
- `scripts/compare-tty.py` runs both versions on a pseudo-terminal for what
  pipes cannot show: colour from isatty, `^C` and `^D` echo, Ctrl-C to the
  process group while ssh runs, `NO_COLOR`, `TERM=dumb`, a 12-column
  terminal. All 13 scenarios matched byte for byte on macOS (2026-11-06).
- `tests/signals.rs` checks the two Ctrl-C paths with SIGINT in CI.

### Inputs left out of the golden set, and why

| Input | Python 0.3.1 | Rust | Why it is left out |
|---|---|---|---|
| Host or file name that rich reads as markup (`a[/b]`, `[bold]x`) | crash (`MarkupError`) or styled text | printed literally | A crash is not behaviour to keep (bug 12). |
| Number typed with non-ASCII digits (`٢`) or underscores (`1_0`) | accepted by `int()` | "Invalid input" | Accidental: `int()` quirks, not a feature. |
| Error text for usage errors other than unknown option and extra arguments (`--hel` suggestions, `--help=x`) | click wording | close but not identical | Wording only; stream and exit code match. |
| `FORCE_COLOR` for click's one-line messages | ignored by click | ignored | Covered; listed so it is not "fixed" by accident. |
| Grapheme clusters (combining marks, ZWJ emoji) in host names | rich 15 measures graphemes | measured per code point | Not seen in ssh configs. |
| Whitespace characters `\x1c`-`\x1f` around config lines | stripped by `str.strip()` | kept | Control characters; not seen in ssh configs. |
| `~user/...` in `Include` | expanded from passwd | left as is | Needs a passwd lookup for a rare form. |
| Files that are not UTF-8 on Windows | decoded with the ANSI code page | decoded as UTF-8 | The Python behaviour depended on the locale. |

### Measurements (macOS arm64, 20 runs, median, `q` on stdin)

| Case | Python 0.3.1 | Rust (release) |
|---|---|---|
| 4 hosts | 47.1 ms | 4.2 ms |
| Ethan's real `~/.ssh/config` | 42.5 ms | 4.3 ms |
| 6000 hosts | 504.0 ms | 12.0 ms |
| Install size | 8.6 MB venv plus a Python interpreter | 686,224-byte binary |

For an interactive picker the speed difference is not noticeable; the gain is
one static binary per platform and no Python runtime.

## Phase 2: CI and release

- `ci.yml`: fmt, clippy `-D warnings` and `cargo test --locked` on ubuntu,
  macos and windows; cargo-deny (`deny.toml`).
- `release-please.yml` (push to main): the same checks as a gate, then
  release-please with the GitHub App token and `release-type: rust`. It no
  longer publishes anything.
- `release.yml` (push of a `v*` tag): tag vs `Cargo.toml` check, five
  targets, archives + `.sha256` + build provenance attached to the release
  that release-please opened (by tag name, which also fixes the old
  `gh release list --limit 1` lookup), `Formula/ssh-connector.rb` pushed to
  `KKamJi98/homebrew-tap` with the deploy key `HOMEBREW_TAP_DEPLOY_KEY`, and
  `cargo publish` through crates.io Trusted Publishing in the `release`
  environment. The publish step skips a version that is already on
  crates.io, because the first version is published by hand.
- The crate is `kkamji-ssh-connector` (`ssh-connector` on crates.io belongs to
  github.com/chojs23/ssh-connector, which also installs an `ssh-connector`
  binary). The binary, the formula and the shell integration keep the name
  `ssh-connector`.
- Python packaging (`pyproject.toml`, `uv.lock`, PyPI jobs) is gone. PyPI
  0.3.1 stays published as the way back; the `pypi` environment is left in
  place.

## Phase 3: bug fixes

Each fix is its own commit with a test that failed before it. Golden cases
whose output changed are listed in `tests/golden/changes.json` with the bug
number and the reason; `cases.json` still holds what Python printed.

| # | Fix | Golden cases changed |
|---|---|---|
| 3.1 | Exit with ssh's exit code (128 + signal when ssh is killed) | ssh-exit-1, ssh-exit-130, ssh-exit-255 |
| 3.2 | Missing ssh: message on stderr, exit 127 | ssh-missing |
| 3.3 | End of input quits like `q`; Ctrl-C exits 130 (also while ssh runs) | pick-eof, filter-eof; `tests/signals.rs` |
| 3.4 | Hosts starting with `-` are not listed; ssh gets `-- <host>` | every case that runs ssh, ssh-dash-host |
| 3.5 | Negated patterns (`!name`) are not listed | config-wildcards |
| 3.6 | `?` patterns are not listed | config-wildcards |
| 3.7 | A word starting with `#` starts a comment | config-trailing-comment |
| 3.8 | `Keyword=value` lines | config-keyword-forms, include-equals-form |
| 3.9 | Quotes and backslash escapes as in OpenSSH's `argv_split`; names with whitespace are not listed | config-quoted |
| 3.10 | Relative `Include` resolves against `~/.ssh` | include-nested-relative, unix-symlinked-config |
| 3.11 | `${VAR}` in `Include` paths | include-env-var |
| 3.12 | Names print literally (rich markup crash) | none: Python crashed; `tests/cli.rs` |
| 3.13 | A non-UTF-8 byte no longer drops the whole file | config-non-utf8-* |
| 3.14 | Same-named included files keep separate groups, labelled by path | groups-same-basename, include-order-sorted |
| 3.15 | README drift (`-abort`, Python 3.13) | fixed by the Phase 2 README rewrite |

Checked against the real tools:

- `ssh -G` (OpenSSH 10.2p1, macOS) for 3.5-3.9 and the `--` separator.
- `ssh.exe -G -- localhost` with OpenSSH_for_Windows_9.5p2 accepts `--` too.
- 3.10 follows the ssh_config(5) text; it was not run against the real
  `~/.ssh` (see the plan's note on `~` expansion).

Found while doing this: the golden case for a missing ssh still had
`/usr/bin` on PATH, so the Python recording had run the real ssh against
the made-up host `alpha` (it failed DNS). The harness now leaves PATH empty
for that case and the fixture was re-recorded (commit "keep the
missing-ssh golden case away from the real ssh").

Development note: `cargo publish --dry-run` verifies the packaged copy in
the shared `target/` and leaves `target/debug/ssh-connector` built from
`target/package/`, after which cargo considers it fresh even when `src/`
changes. Run `cargo clean` after a dry run, or give the dry run its own
`--target-dir`.

## Phase 4: P1 additions

- `-h`, `-V`/`--version` (clap defaults; click only had `--help`).
- `ssh-connector <QUERY>`: a host name (case-insensitive) connects at once;
  anything else opens the menu filtered by it.
- `--list`: host names, one per line, in menu order (normal hosts by group,
  then jump hosts), each once; filtered by QUERY when given.
- `-F`/`--config FILE`: read another config; ssh gets `-F FILE` as well.
  Relative `Include` paths still resolve against `~/.ssh`, as ssh does for
  a `-F` file.
- Shell completion: clap_complete's environment-driven engine
  (`COMPLETE=<shell> ssh-connector`), candidates from the same list as
  `--list`. It needs clap_complete's `unstable-dynamic` feature; the exact
  version is pinned by Cargo.lock. Homebrew installs it with
  `generate_completions_from_executable(..., shell_parameter_format: :clap)`.
- The program name in usage and error text is fixed to `ssh-connector`
  (`bin_name`). clap took it from argv[0] by default, so on Windows it read
  `ssh-connector.exe` and the golden error cases failed on `windows-latest`
  (CI run 42235363547). A renamed or symlinked binary now prints the same
  text as well.
- P2 and P3 ideas are in `docs/ROADMAP.md`.

## Pre-merge review (2026-11-06)

An independent review of the whole branch found the following. What was
changed and what was left:

- Windows printed `ssh-connector.exe` in usage text: fixed with `bin_name`
  (see Phase 4).
- release-please tag format: the reviewer read release-please 17.x, where
  the rust strategy takes the crate name as component and includes it in
  the tag by default, which would give `kkamji-ssh-connector-v0.4.0` and
  never start `release.yml`. The action at v5.0.0 declares
  `include-component-in-tag` with `default: false` in `action.yml` and
  passes it on, so the tag should already be `v0.4.0`. The input is now set
  to `false` explicitly so the tag format does not depend on that default.
- Signal tests waited a fixed 300 ms for ssh to start. They now wait until
  the fake ssh prints its argv to stderr.
- `ExitCode::from(code as u8)` truncated codes above 255. Windows ssh can end
  with an NTSTATUS such as `0xC000013A`, and a multiple of 256 would have
  become 0. Such codes now go to `std::process::exit` unchanged; a
  Windows-only test covers 300 and 512.
- Left as is, recorded in `docs/ROADMAP.md`: `%` tokens in `Include` paths
  (the Python version did not expand them either) and completion that
  ignores `-F`.
- The first `crates-io` job of 0.4.0 is expected to fail, because Trusted
  Publishing can only be set up for a crate that exists. The plan covers
  it: publish 0.4.0 by hand, register the trusted publisher, re-run the job.

## Release 0.4.0 (2026-11-06)

What happened, where it differed from the plan:

- PR #9 was squash-merged as `af4e251` with a `Release-As: 0.4.0` footer.
  release-please opened PR #11 at 0.4.0 (Cargo.toml, Cargo.lock,
  CHANGELOG.md), with tag `v0.4.0`, not `kkamji-ssh-connector-v0.4.0`.
  PR #11 was merged as `76f294a`; release-please created the tag and the
  GitHub release.
- `release.yml` run 42239884922, attempt 1: the `aarch64-apple-darwin`
  build waited 15 minutes for a `macos-latest` runner and GitHub cancelled
  it ("The job was not acquired by Runner of type hosted even after
  multiple attempts"; notice: "Due to capacity constraints, jobs targeting
  macOS arm64 runners may experience longer queue times"). The same label
  had given runners to the `x86_64-apple-darwin` build and to CI minutes
  earlier, and three other repositories were running macOS jobs at the same
  time. `gh run rerun --failed` (attempt 2) built it and ran the publishing
  jobs. If this recurs, re-run first; building both macOS targets in one
  job would need fewer macOS runners but only applies to the next tag.
- crates.io: the crate did not exist, so 0.4.0 was published by hand from
  the tag commit (`cargo publish --locked`, separate target dir). The
  trusted publisher was then registered on the crate's settings page
  (GitHub, `KKamJi98/ssh-connector`, owner id 72260110, workflow
  `release.yml`, environment `release`). "Require trusted publishing" was
  left off. In attempt 2 the `crates-io` job found 0.4.0 (HTTP 200) and
  skipped, so Trusted Publishing will first be used by the next release.
- WSL: the shared plan said none of the tools was installed there. In fact
  `~/.local/bin/ssh-connector` was a hand-written script running the Python
  0.3.1 code from `~/.local/opt/migration-personal-cli`, and `~/.local/bin`
  comes before linuxbrew in PATH. After `brew install`, only that script was
  moved to `~/.local/state/rust-migration/backup/ssh-connector.python-shim`;
  the shared directory (it also serves `awspick`) was left alone.
- Mac: `uv tool uninstall ssh-connector` (was 0.3.1) after `brew install`
  and after `/opt/homebrew/bin/ssh-connector --version` printed 0.4.0.
