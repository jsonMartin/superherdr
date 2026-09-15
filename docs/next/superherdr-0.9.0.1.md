# Superherdr 0.9.0.1 release notes

Superherdr 0.9.0.1 is the first Superherdr release, based on [Herdr](https://github.com/herdrdev/herdr) 0.9.0. It installs as the `herdr` command and is a drop-in replacement for Herdr.

## What 0.9.0.1 adds

- **Focus is client-local.** Each client picks the project or workspace to show; other clients keep their own Focus.
- **Snooze is shared.** A snoozed project or workspace stays hidden on every client until its wake time, without stopping its terminals. Snooze never stops processes, mutes notices, or acknowledges requests; runtime behavior is unaffected.
- **All / Top-level Agents toggle.** Top-level hides agents in linked worktree workspaces when the same endpoint has a non-linked parent with the same worktree key. Parent, standalone, and orphan agents stay visible. Notices and runtime behavior are unchanged.

## Drop-in Herdr

- The only command is `herdr`. Help, completions, and `herdr --skill` use Herdr's wording.
- Configuration, keybindings, plugins, integrations, and sessions use Herdr's directories (`~/.config/herdr`, `~/.local/state/herdr`; `herdr-dev` for debug builds). Superherdr adds six optional keybindings (`focus_project`, `clear_project_focus`, `snooze_workspace`, `snooze_project`, `show_snoozed`, `reset_focus_snooze`); Herdr warns about them if you switch back and applies the rest of the file. Snooze state is kept in a separate `snooze.json` that Herdr ignores.
- Separate setups are opt-in: `HERDR_CONFIG_PATH`, `XDG_CONFIG_HOME` and `XDG_STATE_HOME`, or `--session <name>`.
- `herdr --version` prints `herdr 0.9.0 (superherdr 0.9.0.1)`. `herdr status client --json` reports `version` `0.9.0` and adds `superherdr_version` `0.9.0.1`.
- SSH remote discovery uses any compatible `herdr`, Herdr or Superherdr.

## Artifacts and platforms

- `superherdr-0.9.0.1-macos-aarch64.tar.gz`: macOS Apple Silicon. It declares a macOS 13.0 minimum matching its dependency, but is exercised only on macOS 27.
- `superherdr-0.9.0.1-linux-x86_64.tar.gz`: Linux x86_64, statically linked with musl: a static-pie executable with no interpreter, no `DT_NEEDED` entries, and no glibc symbol dependencies.
- `superherdr-0.9.0.1.arm64_golden_gate.bottle.tar.gz`: the Homebrew bottle, assembled from the macOS archive's binary.
- Linux aarch64, Intel macOS, Windows, and Android/Termux have no build.
- Each archive contains `herdr`, `LICENSE`, and `licenses/`. `SHA256SUMS` lists every asset.
- Binary self-update is disabled. Superherdr does not read Herdr update manifests.
- Existing plugins keep the Herdr 0.9.0 compatibility level.
- `HERDR_*` environment variables, socket names, and protocol identifiers keep their Herdr names.

## Shell installer

`install.sh` (source: `distribution/install.sh`) installs from Superherdr's own GitHub releases on Linux x86_64 and macOS Apple Silicon:

- It downloads `SHA256SUMS` and `superherdr-<version>-<target>.tar.gz` from the `superherdr-v<version>` release over HTTPS and verifies the checksum before extracting.
- It installs one `herdr` file into `~/.local/bin` (override with `SUPERHERDR_INSTALL_DIR`), and the licenses into `${XDG_DATA_HOME:-~/.local/share}/superherdr/licenses`. It needs no root.
- The version defaults to 0.9.0.1; pass another version as the first argument or through `SUPERHERDR_VERSION`.
- It updates only its own install: a `herdr` whose `--version` names Superherdr. It refuses upstream Herdr's `herdr`, non-executable files, foreign symlinks, and any `herdr` or `superherdr` in another `PATH` directory, including directories later on `PATH` than the install directory.
- It stages the new binary inside the install directory and renames it into place, so an interrupted install leaves the previous binary intact.
- It does not touch configuration, state, or running sessions.

## Building release artifacts

Both binaries use the repository-pinned Rust 1.96.1 toolchain (invoke rustup's `cargo`, not a Homebrew Rust earlier on `PATH`), Zig 0.15.2 for the vendored libghostty-vt, `--locked` dependencies, and `HERDR_BUILD_CHANNEL=stable`. The GitHub release names the exact commit.

- **Linux:** the manually triggered `Build Linux release` workflow (`.github/workflows/build-linux-release.yml`) builds `x86_64-unknown-linux-musl` with `-j4` and `RUST_MIN_STACK=268435456` (rustc crashed at the final crate with 128 MiB; the cause was not diagnosed), verifies the binary is static, runs a server and PTY smoke test, and uploads the archive.
- **macOS:** built on Apple Silicon with `MACOSX_DEPLOYMENT_TARGET=13.0`. The stock Zig 0.15.2 download cannot link against the macOS 26.5 or 27 SDKs, and Homebrew's patched `zig@0.15` cannot compile against the macOS 27 SDK, so the build uses `zig@0.15` with `xcrun --sdk macosx --show-sdk-path` pointed at the macOS 26.5 SDK. The Homebrew bottle is assembled from the archive's binary, not produced by `brew bottle`.

The release repository is [jsonmartin/superherdr](https://github.com/jsonmartin/superherdr), and the Homebrew formula is in [jsonmartin/homebrew-tap](https://github.com/jsonmartin/homebrew-tap). Release tags are `superherdr-v<version>`, so inherited Herdr tags remain unchanged.
