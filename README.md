# Superherdr

Superherdr is an open-source terminal project built on [Herdr](https://github.com/herdrdev/herdr) with built-in Focus and Snooze. It tracks upstream Herdr and keeps its native terminal runtime, agent status detection, CLI, and socket API.

Focus shows the project or workspace you want to work on in one client. Snooze hides a project or workspace across clients until its wake time, without stopping its terminals. See [Focus and Snooze](docs/next/focus-snooze-fork.md).

## Install

Superherdr 0.1.0 provides macOS Apple Silicon and Linux x86_64 binaries from [GitHub releases](https://github.com/jsonMartin/superherdr/releases).

### macOS: Homebrew

```sh
brew install jsonmartin/tap/superherdr
herdr
```

The formula provides both `superherdr` and `herdr`. If the original Homebrew `herdr` formula is linked, unlink it first with `brew unlink herdr`; this keeps its installed files. See [Homebrew installation notes](docs/next/homebrew.md).

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

A source install provides `superherdr`; it does not create the `herdr` alias. See [0.1.0 build and installation notes](docs/next/superherdr-0.1.0.md).

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

Superherdr is a fork of [Herdr](https://github.com/herdrdev/herdr), created by Ogulcan Celik and the Herdr contributors. Its terminal runtime, agent detection, CLI, and socket API come from Herdr, and credit for that work belongs to them. Superherdr adds its own features on top and merges upstream releases regularly. [Herdr's documentation](https://herdr.dev/docs/) describes the inherited behavior; use `superherdr` in its command examples.

The `HERDR_*` environment variables, socket filenames, integration asset names, API/schema identifiers, and frozen endpoint codecs keep their upstream names for compatibility. Socket files live inside the separate Superherdr directories.

Agent status detection rules update from Herdr's published catalog at `herdr.dev` in the background when Superherdr starts, so detection fixes arrive as soon as Herdr publishes them. If the catalog is unreachable, Superherdr uses its bundled rules. To use only the bundled rules and make no request to `herdr.dev`, set this in your configuration:

```toml
[update]
manifest_check = false
```

Windows and Nix build files are inherited from Herdr and are not supported Superherdr targets.

## Contributing

Bug fixes, tests, and documentation improvements are welcome as pull requests. New features and material behavior changes start with an OpenSpec proposal. Read [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md) first.

```sh
just test
```

## Support

If Superherdr is useful to you, you can support its development through [GitHub Sponsors](https://github.com/sponsors/jsonMartin) or [Buy Me a Coffee](https://buymeacoffee.com/jsonmartin).

## License

Superherdr is licensed under the [Apache License 2.0](LICENSE), the same license as Herdr. See [NOTICE](NOTICE) for attribution.
