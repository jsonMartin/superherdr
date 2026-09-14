# Changelog

Superherdr's release history. Superherdr is a fork of [Herdr](https://github.com/herdrdev/herdr); changes inherited from upstream are described in [Herdr's releases](https://github.com/herdrdev/herdr/releases).

## [0.1.0] - 2026-09-14

First Superherdr release, based on Herdr 0.9.0.

### Added
- Focus shows the project or workspace selected in each client; other clients keep their own Focus.
- Snooze hides a project or workspace on every client until its wake time, without stopping its terminals.
- The All / Top-level Agents toggle hides linked-worktree agents when their parent exists on the same endpoint.
- Release binaries for macOS Apple Silicon and Linux x86_64, a Homebrew formula (`jsonmartin/tap/superherdr`), and a checksum-verifying shell installer.

### Changed
- The executable is `superherdr`, with a `herdr` alias from Homebrew and the shell installer. Superherdr uses its own configuration and state directories and does not migrate original Herdr sessions.
- Binary self-update is disabled.
