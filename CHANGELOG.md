# Changelog

Superherdr's release history. Superherdr is a fork of [Herdr](https://github.com/herdrdev/herdr); changes inherited from upstream are described in [Herdr's releases](https://github.com/herdrdev/herdr/releases).

## [0.9.0.1] - 2026-09-15

First Superherdr release, based on Herdr 0.9.0. Superherdr versions are the Herdr base version plus a Superherdr revision.

### Added
- Focus shows the project or workspace selected in each client; other clients keep their own Focus.
- Snooze hides a project or workspace on every client until its wake time, without stopping its terminals.
- The All / Top-level Agents toggle hides linked-worktree agents when their parent exists on the same endpoint.
- Release binaries for macOS Apple Silicon and Linux x86_64, a Homebrew formula (`jsonmartin/tap/superherdr`), and a checksum-verifying shell installer.

### Changed
- Superherdr installs and runs as the `herdr` command and uses Herdr's configuration, state, plugins, and sessions, so it is a drop-in replacement. Separate setups are opt-in with `HERDR_CONFIG_PATH`, `XDG_CONFIG_HOME`/`XDG_STATE_HOME`, or `--session`.
- `herdr --version` prints the Herdr base and the Superherdr version, for example `herdr 0.9.0 (superherdr 0.9.0.1)`.
- Binary self-update is disabled.
- New worktrees default to `~/.herdr/worktrees`, as in Herdr.
- Connecting to a remote host finds and installs `herdr` in the same places Herdr does. Superherdr does not download release binaries for remote hosts; set `HERDR_REMOTE_BINARY` or install herdr there.
