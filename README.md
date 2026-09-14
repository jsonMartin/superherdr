# Superherdr

Superherdr is an open-source terminal project built on [Herdr](https://github.com/herdrdev/herdr) with built-in Focus and Snooze. It tracks upstream Herdr and keeps its native terminal runtime, agent status detection, CLI, and socket API.

Focus shows the project or workspace you want to work on in one client. Snooze hides a project or workspace across clients until its wake time, without stopping its terminals. See the [accepted Focus/Snooze behavior](docs/next/focus-snooze-fork.md).

## Install from this checkout

Superherdr does not yet have a published package or release feed. The upstream Herdr installers install Herdr, not Superherdr. Release-candidate preparation for 0.1.0 is described in [docs/next/superherdr-0.1.0.md](docs/next/superherdr-0.1.0.md).

With Rust and Zig 0.15.2 installed, run from the source checkout:

```sh
cargo install --path . --locked
superherdr
```

The executable is `superherdr` (`superherdr.exe` on Windows). Installing it does not overwrite `herdr`. To use Superherdr as your default `herdr` command, preserve the original executable as `herdr-og`, then point a `herdr` symlink at `superherdr` in a directory earlier on PATH. A command alias does not change the separate state directories below. A release build uses `~/.config/superherdr` and `~/.local/state/superherdr` on Unix; a debug build uses `superherdr-dev`. XDG overrides and Windows APPDATA/LOCALAPPDATA roots are supported.

Existing Herdr sessions and configuration are not moved or stopped. Superherdr starts with separate state. Do not point both products at the same writable session directory. Binary self-update is disabled so an upstream release cannot replace Superherdr. Rebuild from this checkout to update.

For the local Homebrew setup and command aliases, see [Homebrew installation notes](docs/next/homebrew.md).

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

[Upstream documentation](https://herdr.dev/docs/) describes the inherited behavior; use `superherdr` for command examples and the project’s directories above. Historical release snapshots, changelogs, distribution payloads, and the upstream stable skill remain upstream reference material, not Superherdr releases. No Superherdr GitHub destination or hosted installer has been established.

## Development

```sh
cargo build --locked
cargo test --locked --test superherdr_rename
```

Read [AGENTS.md](AGENTS.md) and [CONTRIBUTING.md](CONTRIBUTING.md) before working here. Upstream contribution and release rules do not authorize publishing Superherdr to upstream.

## License

Superherdr retains Herdr's [Apache License 2.0](LICENSE), upstream copyright notices, and [sponsor acknowledgments](SPONSORS.md).
