# Superherdr

Superherdr is an open-source terminal project built on [Herdr](https://github.com/herdrdev/herdr) with built-in Focus and Snooze. It tracks upstream Herdr and keeps its native terminal runtime, agent status detection, CLI, and socket API.

Focus shows the project or workspace you want to work on in one client. Snooze hides a project or workspace across clients until its wake time, without stopping its terminals. See [Focus and Snooze](docs/next/focus-snooze-fork.md).

## Install

Superherdr installs as the `herdr` command and is a drop-in replacement for Herdr: it reads Herdr's configuration, keybindings, plugins, and sessions from the same directories. Releases provide macOS Apple Silicon and Linux x86_64 binaries on [GitHub releases](https://github.com/jsonMartin/superherdr/releases).

### macOS: Homebrew

```sh
brew install jsonmartin/tap/superherdr
herdr
```

If the original Homebrew `herdr` formula is linked, unlink it first with `brew unlink herdr`; this keeps its installed files. See [Homebrew installation notes](docs/next/homebrew.md).

### Linux x86_64: shell installer

Download the versioned installer, inspect it, then run it:

```sh
curl -fsSL https://github.com/jsonMartin/superherdr/releases/download/superherdr-v0.9.0.1/install.sh -o install-superherdr.sh
sh install-superherdr.sh
herdr
```

The installer verifies the release archive checksum and installs `herdr` into `~/.local/bin`. It exits without changes if another `herdr` is installed, including the original Herdr in `~/.local/bin` or anywhere on `PATH`; remove or move that first, or extract the release archive by hand. It also supports macOS Apple Silicon; Linux ARM64, Intel Mac, Windows, and Android/Termux are not supported by these release artifacts. The Linux binary is statically linked; the macOS binary targets macOS 13 and is tested on macOS 27.

### Build from source

With the repository-pinned Rust toolchain and Zig 0.15.2 installed:

```sh
cargo install --path . --locked
herdr
```

See [release and build notes](docs/next/superherdr-0.9.0.1.md).

### Versions

Superherdr versions are the Herdr version they are based on plus a Superherdr revision: `0.9.0.1` is the first Superherdr release on Herdr 0.9.0. `herdr --version` prints both, for example `herdr 0.9.0 (superherdr 0.9.0.1)`, so tools that read Herdr's version keep working.

### Updates and existing sessions

Use `brew upgrade jsonmartin/tap/superherdr` for Homebrew installations. For direct installations, download the installer from the desired release and rerun it. Binary self-update is disabled.

Superherdr uses Herdr's directories: `~/.config/herdr` and `~/.local/state/herdr` for release builds, `herdr-dev` for debug builds, honoring `XDG_CONFIG_HOME` and `XDG_STATE_HOME`. Your existing Herdr sessions, configuration, and plugins carry over, and switching back to Herdr keeps them too. Only one server runs per session: if a Herdr server is already running, Superherdr attaches to it as a client and Snooze is unavailable until that server is restarted with Superherdr.

To keep a separate setup instead, opt in with an override: `HERDR_CONFIG_PATH=/path/to/config.toml herdr` for a separate configuration file, `XDG_CONFIG_HOME` and `XDG_STATE_HOME` for fully separate directories, or `herdr --session <name>` for a separate session.

## Use

```sh
herdr --help
herdr completion zsh
herdr --skill
herdr
```

`ctrl+b q` detaches the client. Running the same command reattaches. Focus and Snooze are available in the sidebar and workspace/project context menus.

For SSH, `herdr --remote <host>` uses a compatible `herdr` on the remote host, whether Herdr or Superherdr; Snooze needs Superherdr on the remote. If none is installed, it offers to install your local binary, or the build named by `HERDR_REMOTE_BINARY`, at `~/.local/bin/herdr`. It never downloads release binaries.

## Compatibility and upstream

Superherdr is a fork of [Herdr](https://github.com/herdrdev/herdr), created by Ogulcan Celik and the Herdr contributors. Its terminal runtime, agent detection, CLI, and socket API come from Herdr, and credit for that work belongs to them. Superherdr adds its own features on top and merges upstream releases regularly. [Herdr's documentation](https://herdr.dev/docs/) describes the inherited behavior, and its command examples work as written.

The `HERDR_*` environment variables, socket filenames, integration asset names, API/schema identifiers, and frozen endpoint codecs keep their upstream names for compatibility. Socket files live in the same Herdr directories.

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
