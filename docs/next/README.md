# Superherdr

Superherdr is an open-source terminal project built on [Herdr](https://github.com/herdrdev/herdr) with built-in Focus and Snooze. It tracks upstream Herdr and keeps its native terminal runtime, agent status detection, CLI, and socket API.

Focus shows the project or workspace you want to work on in one client. Snooze hides a project or workspace across clients until its wake time, without stopping its terminals. See the [accepted Focus/Snooze behavior](focus-snooze-fork.md).

The [All / Top-level Agents toggle](top-level-agents.md) hides linked-worktree agents while keeping their parent agents and standalone workspaces visible.

## Install

Superherdr 0.1.0 provides macOS Apple Silicon and Linux x86_64 binaries from [GitHub releases](https://github.com/jsonMartin/superherdr/releases).

### macOS: Homebrew

```sh
brew install jsonmartin/tap/superherdr
herdr
```

The formula provides both `superherdr` and `herdr`. If the original Homebrew `herdr` formula is linked, unlink it first with `brew unlink herdr`; this keeps its installed files. See [Homebrew installation notes](homebrew.md).

### Linux x86_64: shell installer

Download the versioned installer, inspect it, then run it:

```sh
curl -fsSL https://github.com/jsonMartin/superherdr/releases/download/superherdr-v0.1.0/install.sh -o install-superherdr.sh
sh install-superherdr.sh
herdr
```

The installer verifies the release archive checksum and installs `superherdr` plus a `herdr` alias into `~/.local/bin`. It exits without changes if any other `herdr` or `superherdr` is on PATH, including the original Herdr; remove or move that first, or extract the release archive by hand. It also supports macOS Apple Silicon; Linux ARM64, Intel Mac, Windows, and Android/Termux are not supported by these release artifacts. The Linux binary is static and was tested on Arch Linux x86_64. The macOS binary targets macOS 13 and was tested on macOS 27.

### Build from source

With the repository-pinned Rust toolchain and Zig 0.15.2 installed:

```sh
cargo install --path . --locked
superherdr
```

A source install provides `superherdr`; it does not create the `herdr` alias. See [0.1.0 build and installation notes](superherdr-0.1.0.md).

### Updates and existing sessions

Use `brew upgrade jsonmartin/tap/superherdr` for Homebrew installations. For direct installations, download the installer from the desired release and rerun it with that version, for example `sh install-superherdr.sh 0.1.0`. Binary self-update is disabled.

Installing Superherdr does not migrate or stop original Herdr sessions. Unix release builds use `~/.config/superherdr` and `~/.local/state/superherdr`; debug builds use `superherdr-dev`. XDG overrides are supported. A command alias does not migrate session state. Replacing a running original Herdr server requires a separate compatible handoff; do not run two servers against the same writable session directory.

## Use

```sh
superherdr --help
superherdr completion zsh
superherdr --skill
superherdr
```

`ctrl+b q` detaches the client. Running the same command reattaches. Focus and Snooze are available in the sidebar and workspace/project context menus.

For SSH, install a matching Superherdr build remotely or provide `HERDR_REMOTE_BINARY` pointing to a Superherdr build for the remote platform. Default discovery and installation use `superherdr`, not stock `herdr`. There is no fallback download of upstream Herdr as a Superherdr binary.

## Compatibility and upstream

The `HERDR_*` environment variables, socket filenames, integration asset names, API/schema identifiers, and frozen endpoint codecs retain their upstream names for compatibility. Socket files live inside the separate Superherdr directories. Existing upstream detection catalogs and plugin services remain intentional upstream integrations.

[Upstream documentation](https://herdr.dev/docs/) describes the inherited behavior; use `superherdr` for command examples and the project’s directories above. Historical release snapshots, changelogs, distribution payloads, and the upstream stable skill remain upstream reference material, not Superherdr releases. The project lives at [jsonmartin/superherdr](https://github.com/jsonmartin/superherdr).

## Development

```sh
cargo build --locked
cargo test --locked --test superherdr_rename
```

Read [AGENTS.md](AGENTS.md) and [CONTRIBUTING.md](CONTRIBUTING.md) before working here. Upstream contribution and release rules do not authorize publishing Superherdr to upstream.

## License

Superherdr retains Herdr's [Apache License 2.0](LICENSE), upstream copyright notices, and [sponsor acknowledgments](SPONSORS.md).
