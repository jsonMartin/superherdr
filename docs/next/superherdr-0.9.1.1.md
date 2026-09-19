# Superherdr 0.9.1.1 release notes

Superherdr 0.9.1.1 is based on [Herdr](https://github.com/herdrdev/herdr) 0.9.1. It installs as the `herdr` command and is a drop-in replacement for Herdr. Focus, shared Snooze, and the All / Top-level Agents view carry over unchanged.

## What this release inherits

All of Herdr 0.9.1, including saved-machine CLI control (`herdr --machine`), cursor-aware text editing with word-wise selection, Ctrl-click link highlighting, per-machine workspace navigation with collapsed groups, large SSH bandwidth and idle-CPU reductions, and the rest of [Herdr's 0.9.1 release notes](https://github.com/herdrdev/herdr/releases/tag/v0.9.1).

Superherdr's Focus/Snooze/agent-scope behavior was merged through upstream's new agent-view projection and per-endpoint collapsed groups; the fork's hiding rules keep applying wherever they applied before.

## Drop-in Herdr

- `herdr --version` prints `herdr 0.9.1 (superherdr 0.9.1.1)`. `herdr status client --json` reports `version` `0.9.1` and adds `superherdr_version` `0.9.1.1`.
- Configuration, keybindings, plugins, integrations, and sessions stay in Herdr's directories. Binary self-update stays disabled; Superherdr does not read Herdr update manifests.
- Existing plugins keep the Herdr 0.9.0 compatibility level (`HERDR_PLUGIN_COMPATIBILITY_VERSION` is unchanged this release).

## Artifacts and platforms

- `superherdr-0.9.1.1-macos-aarch64.tar.gz`: macOS Apple Silicon, built with `MACOSX_DEPLOYMENT_TARGET=13.0`.
- `superherdr-0.9.1.1-linux-x86_64.tar.gz`: Linux x86_64, statically linked with musl.
- `superherdr-0.9.1.1.arm64_golden_gate.bottle.tar.gz`: the Homebrew bottle, assembled from the macOS archive's binary.
- Each archive contains `herdr`, `LICENSE`, and `licenses/`. `SHA256SUMS` lists every asset.

## Building release artifacts

Both binaries use the repository-pinned Rust 1.96.1 toolchain, **Zig 0.16.0** for the vendored libghostty-vt (a hard requirement this release — 0.15.x is rejected by `build.rs`), `--locked` dependencies, and `HERDR_BUILD_CHANNEL=stable`.

- **Linux:** the manually triggered `Build Linux release` workflow builds `x86_64-unknown-linux-musl` with `-j4` and `RUST_MIN_STACK=268435456`, verifies the binary is static, runs a server and PTY smoke test, and uploads the archive.
- **macOS:** built by the `Build macOS release` workflow on a GitHub macOS runner with `MACOSX_DEPLOYMENT_TARGET=13.0`. Building on a local Mac stopped working for this release: macOS 27's dyld rejects proc-macro dylibs the new Xcode 27 linker produces ("mis-aligned LINKEDIT string pool"). The GitHub runner toolchain links cleanly. The Homebrew bottle is assembled from the archive's binary, not produced by `brew bottle`.

The release repository is [jsonmartin/superherdr](https://github.com/jsonmartin/superherdr), and the Homebrew formula is in [jsonmartin/homebrew-tap](https://github.com/jsonmartin/homebrew-tap). Release tags are `superherdr-v<version>`, so inherited Herdr tags remain unchanged.
