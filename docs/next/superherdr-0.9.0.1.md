# Superherdr 0.1.0 release notes

Superherdr 0.1.0 is the first Superherdr release. Superherdr is a project built on [Herdr](https://github.com/herdrdev/herdr), with its own release versioning.

## What 0.1.0 adds

- **Focus is client-local.** Each client picks the project or workspace to show; other clients keep their own Focus.
- **Snooze is shared.** A snoozed project or workspace stays hidden on every client until its wake time, without stopping its terminals. Snooze never stops processes, mutes notices, or acknowledges requests; runtime behavior is unaffected.
- **All / Top-level Agents toggle.** Top-level hides agents in linked worktree workspaces when the same endpoint has a non-linked parent with the same worktree key. Parent, standalone, and orphan agents stay visible. Notices and runtime behavior are unchanged.

## Artifacts and platforms

- `superherdr-0.1.0-macos-aarch64.tar.gz`: macOS Apple Silicon. It declares a macOS 13.0 minimum matching its dependency, but has been exercised only on macOS 27.
- `superherdr-0.1.0-linux-x86_64.tar.gz`: Linux x86_64, statically linked with musl. It is a static-pie executable with no interpreter, no `DT_NEEDED` entries, and no glibc symbol dependencies. The release binary was exercised on GitHub's Ubuntu x86_64 runner (headless server, named session, PTY command roundtrip), and an earlier 0.1.0 build was also run on Arch Linux x86_64, including the TUI. Other distributions and the SSH remote attach flow have not been exercised.
- `superherdr-0.1.0.arm64_golden_gate.bottle.tar.gz`: the Homebrew bottle, assembled from the macOS archive's binary.
- Linux aarch64, Intel macOS, Windows, and Android/Termux have no 0.1.0 build.
- Each archive contains `superherdr`, `LICENSE`, and `licenses/`. `SHA256SUMS` lists every asset.
- Binary self-update is disabled. Superherdr does not read Herdr update manifests.
- Existing plugins retain the Herdr 0.9.0 compatibility level; Superherdr release numbers are separate.
- `HERDR_*` environment variables, socket names, and protocol identifiers retain their inherited names for compatibility.

## Shell installer

`install.sh` (source: `distribution/install.sh`) installs from Superherdr's own GitHub releases on Linux x86_64 and macOS Apple Silicon:

- It downloads `SHA256SUMS` and `superherdr-<version>-<target>.tar.gz` from the `superherdr-v<version>` release over HTTPS and verifies the checksum before extracting.
- It installs `superherdr` and a `herdr` symlink into `~/.local/bin` (override with `SUPERHERDR_INSTALL_DIR`), and the licenses into `${XDG_DATA_HOME:-~/.local/share}/superherdr/licenses`. It needs no root.
- The version defaults to 0.1.0; pass another version as the first argument or through `SUPERHERDR_VERSION`.
- It updates only its own previous install: a regular `superherdr` binary plus a `herdr` symlink pointing at exactly that binary. It refuses symlinked or unpaired `superherdr` files, foreign `herdr` symlinks or files, and any `superherdr` or `herdr` found on `PATH` outside the install directory, such as a Homebrew or original Herdr installation.
- It stages the new binary inside the install directory and renames it into place, so an interrupted install leaves the previous binary intact.
- It does not touch configuration, state, or running sessions, and does not migrate original Herdr state.

## Building release artifacts

Both binaries use the repository-pinned Rust 1.96.1 toolchain, Zig 0.15.2 for the vendored libghostty-vt, `--locked` dependencies, and `HERDR_BUILD_CHANNEL=stable`. The release notes name the exact commits.

- **Linux:** the manually triggered `Build Linux release` workflow (`.github/workflows/build-linux-release.yml`) builds `x86_64-unknown-linux-musl` with `-j4` and `RUST_MIN_STACK=268435456` (rustc crashed at the final crate with 128 MiB; the cause was not diagnosed), verifies the binary is static, runs a server and PTY smoke test, and uploads the archive.
- **macOS:** built on Apple Silicon with `MACOSX_DEPLOYMENT_TARGET=13.0`. The stock Zig 0.15.2 download cannot link against the macOS 26.5 or 27 SDKs, and Homebrew's patched `zig@0.15` cannot compile against the macOS 27 SDK, so the build used `zig@0.15` with `xcrun --sdk macosx --show-sdk-path` pointed at the macOS 26.5 SDK. The Homebrew bottle is assembled from the archive's binary, not produced by `brew bottle`.

The release repository is [jsonmartin/superherdr](https://github.com/jsonmartin/superherdr), and the Homebrew formula is in [jsonmartin/homebrew-tap](https://github.com/jsonmartin/homebrew-tap). The release tag is `superherdr-v0.1.0`, so inherited Herdr tags remain unchanged.
