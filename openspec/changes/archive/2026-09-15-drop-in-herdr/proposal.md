# Change: Superherdr becomes a drop-in `herdr`

## Why

### Problem and users

Superherdr 0.1.0 installs as `superherdr` (with a `herdr` symlink from the shell installer and Homebrew) and keeps its own `~/.config/superherdr` and `~/.local/state/superherdr` directories. That protects a side-by-side Herdr install, but it is the wrong default for the people Superherdr is for:

- Omarchy users get `~/.config/herdr/config.toml` seeded by the distribution and expect `herdr` to be the command. With 0.1.0 they must configure Superherdr separately and keep two files in sync.
- Existing Herdr users who switch keep none of their keybindings, plugins, integrations, saved SSH machines or named sessions until they copy them by hand. The agent integrations Herdr installed call `herdr`, so both products have to resolve to the same command anyway.
- Nobody wants two Herdr-shaped setups on one machine. Superherdr is a superset of Herdr; a superset should run in Herdr's place, not beside it.

This belongs in Superherdr, not upstream: it concerns how the fork installs and where it stores state. Herdr's behavior does not change.

### Why now

0.1.0 shipped hours ago and its only known users are the maintainer's machines, which already symlink the `superherdr` directories to Herdr's. Changing the persisted locations before anyone else depends on them avoids a migration later. The change ships as Superherdr 0.9.0.1, the first release under the versioning scheme below.

## What Changes

Superherdr installs and runs as a single command, `herdr`, and uses exactly Herdr's configuration and state directories by default. The package, project, repository, release tag, archive names, tap and formula names stay `superherdr`.

### Before and after

| | 0.1.0 | This change |
| --- | --- | --- |
| Command | `superherdr`, plus a `herdr` symlink from the installer and Homebrew; source builds get only `superherdr` | `herdr` in every channel; no `superherdr` command or alias anywhere |
| Directories, release build | `~/.config/superherdr`, `~/.local/state/superherdr` | `~/.config/herdr`, `~/.local/state/herdr`, honoring `XDG_CONFIG_HOME` and `XDG_STATE_HOME` |
| Directories, debug build | `superherdr-dev` | `herdr-dev`, as upstream |
| Existing Herdr config, keybindings, plugins, integrations, sessions | ignored | used unchanged |
| `herdr --version` | `superherdr 0.1.0` | `herdr 0.9.0 (superherdr 0.9.0.1)` |
| Versions | Superherdr's own `0.1.0` | Herdr's base version plus a Superherdr revision: `0.9.0.1`, `0.9.0.2`, then `0.9.1.1` after merging Herdr 0.9.1 |
| Help, completions, `--skill`, messages | `superherdr ...` | Herdr's wording, `herdr ...` |
| `worktrees.directory` default | `~/.superherdr/worktrees` | `~/.herdr/worktrees`, Herdr's default |
| SSH remote discovery | `command -v superherdr`, refuses anything else | `command -v herdr` and Herdr's known paths; any compatible Herdr or Superherdr is used |
| Installer ownership | regular `superherdr` plus owned `herdr` symlink | single regular `herdr` that identifies itself as Superherdr; the 0.1.0 layout is upgraded in place |

### Decisions

These answer the questions the maintainer left open. Each is a recommendation with reasons; the maintainer accepts or changes them at review.

**1. Versioning and identification (decided by the maintainer).** Every Superherdr release is versioned `<Herdr base version>.<revision>`, starting at `.1` for each Herdr base: `0.9.0.1`, `0.9.0.2`, and `0.9.1.1` after merging Herdr 0.9.1. A bare three-part version therefore always means upstream Herdr. `herdr --version` prints `herdr <base> (superherdr <base>.<revision>)`, for example `herdr 0.9.0 (superherdr 0.9.0.1)`; a preview build may append `-<channel>` to the base as upstream does. The first two words match upstream Herdr's `herdr <version>` exactly, so scripts that read the second word or match `^herdr ([0-9.]+)` see `0.9.0`; the parenthesized part lets users and the shell installer tell Superherdr apart. `Cargo.toml` keeps the three-part base version (`0.9.0`), because Cargo requires SemVer and inherited update code parses `X.Y.Z`; the revision lives in a separate build constant. Release tags, archive names, the Homebrew `version` and the AUR `pkgver` use the four-part version (`superherdr-v0.9.0.1`, `superherdr-0.9.0.1-linux-x86_64.tar.gz`, `0.9.0.1`); AUR `pkgver` forbids hyphens, which this satisfies.

`herdr status client --json` keeps Herdr's fields (`version`, `channel`, `protocol`, `endpoint_protocol_generation`, `endpoint_capabilities`, `binary`, `session`) with the Herdr base in `version`, and adds one optional field, `superherdr_version`, with the four-part version. The running server reports the same pair: the socket API `Pong` response gains the optional `superherdr_version` field, which older clients ignore, and the generated API schema is regenerated. Because Herdr and every Superherdr revision on one base share `version`, the stale-server check compares `superherdr_version` when client and server both report it and falls back to `version` otherwise; without that, upgrading from `0.9.0.1` to `0.9.0.2` would report the old server as current. Help text, usage and error strings, `completion` output (`#compdef herdr`), `--default-config` comments and `--skill` revert to Herdr's wording, keeping only the lines where Superherdr behaves differently: self-update is disabled and `update.version_check` defaults to `false`.

**2. Shell installer.** It installs one regular file, `herdr`. Ownership is decided before any download:

- Owned: `<install dir>/herdr` is a regular file and `"<install dir>/herdr" --version` prints `(superherdr ` after the Herdr version; or the 0.1.0 layout is present, a `herdr` symlink whose target is exactly the regular `<install dir>/superherdr`.
- Upgrading the 0.1.0 layout stages the new binary, renames it over the `herdr` symlink, then deletes the owned `superherdr` file so no second command remains. An interrupted run leaves either the previous pair intact or an owned regular `herdr` beside a leftover `superherdr`; the next run recognizes that second state (owned `herdr` plus a regular `superherdr` in the install directory) and removes the leftover instead of refusing.
- Refused, naming the path: a `herdr` symlink with any other target; a regular `herdr` that is not executable (checked with `-x` before running it) or does not identify itself as Superherdr, which is what upstream Herdr's installer leaves at `~/.local/bin/herdr`; a `superherdr` file without its 0.1.0 alias and without an owned `herdr` beside it; an executable `herdr` or `superherdr` in any `PATH` directory other than the install directory, checked in every `PATH` entry so an install directory earlier on `PATH` cannot hide one. A user who wants to replace upstream Herdr removes or moves that file first, or sets `SUPERHERDR_INSTALL_DIR`; the installer never takes over another installation.
- Running the existing binary to identify it is deliberate: the file is in the user's own install directory, and Herdr's updater and remote-install code already execute discovered binaries. A receipt file was rejected as new persisted state.
- `SUPERHERDR_INSTALL_DIR`, `SUPERHERDR_VERSION`, the license directory `${XDG_DATA_HOME:-~/.local/share}/superherdr/licenses` and the checksum flow are unchanged.

**3. SSH remote.** `HERDR_REMOTE_BINARY` stays what it is upstream: the path of a local build to upload. Discovery on the remote host reverts to Herdr's: `command -v herdr` in the login shell plus Herdr's known locations (`~/.local/bin/herdr`, Homebrew, mise, Nix). The first candidate that passes the existing endpoint generation and capability checks is used, whether it is Herdr or Superherdr. On a Herdr remote, Snooze (the only server-side Superherdr feature) is unavailable for that host and nothing else changes, which is what the endpoint contract already promises. When no compatible `herdr` exists, Superherdr offers, after the existing interactive confirmation, to install the local binary (or `HERDR_REMOTE_BINARY`) at `~/.local/bin/herdr`, exactly where Herdr installs its own newer version. Release-asset download stays refused for both products. Requiring the remote to be Superherdr was rejected: it would refuse working Herdr hosts and needs a product probe.

**4. Cargo and source text.** `[package] name` stays `superherdr`; a `[[bin]]` target named `herdr` with `path = "src/main.rs"` makes `cargo build` and `cargo install --path .` produce `herdr`. Integration tests use `env!("CARGO_BIN_EXE_herdr")`, which is Herdr's text. The `env!("CARGO_PKG_NAME") == "superherdr"` guards that disable self-update and asset download keep working. The rename commit `032aa9a1` rewrote several hundred `herdr` command strings in `src/`, `tests/` and `scripts/` to `superherdr`; this change reverts them to upstream wording, which makes the strings correct for a `herdr` command and removes most future merge conflicts. Deliberate divergences that remain in `src/`: the `--version` product name, the self-update guard and message, the `update.version_check` default and its comment, `HERDR_PLUGIN_COMPATIBILITY_VERSION`, and the remote asset-download refusal. The agent skill returns to upstream's `skills/herdr/SKILL.md`, which leaves `.upstream-sync/exclude`; `skills/superherdr/` is deleted. `tests/superherdr_rename.rs` becomes `tests/drop_in_herdr.rs` and asserts the new identity contract.

**5. Homebrew and AUR.** The `jsonmartin/tap/superherdr` formula installs `bin/"herdr"` only, keeps `conflicts_with "herdr"` because both provide a `herdr` command, and its test asserts `herdr --version` prints `herdr 0.9.0 (superherdr 0.9.0.1)`. Upgrading from 0.1.0 relinks only what the new keg provides, so the old `superherdr` link disappears. The unpublished `superherdr-bin` PKGBUILD installs `/usr/bin/herdr` only and keeps `provides=('superherdr' 'herdr')` and `conflicts=('superherdr' 'herdr' 'herdr-bin' 'herdr-git')`: `provides=herdr` lets packages that depend on `herdr` accept Superherdr, and the conflicts stop two `herdr` binaries from being installed together.

### Non-goals

- No migration, fallback lookup or deletion of the 0.1.0 `superherdr` directories. They are simply not read.
- No new flag, environment variable or configuration key. Separation uses `HERDR_CONFIG_PATH`, `XDG_CONFIG_HOME` and `XDG_STATE_HOME`, and `--session` only.
- No change to Herdr's runtime behavior, `PROTOCOL_VERSION`, endpoint generation or plugin compatibility level `0.9.0`.
- No Superherdr release-asset download for remote hosts; no self-update.
- No rename of the package, repository, release tag, archive names, tap, formula, AUR package or installer variables.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `superherdr-compatibility`: executable identity, configuration and state location, inherited names and remote discovery.
- `superherdr-distribution`: archive contents, shell installer, installation ownership, Homebrew formula, self-update wording; adds the Arch Linux package.

## Alternatives and drawbacks

- **Keep 0.1.0 as is: separate command and directories.** Safe side by side, but every Herdr user starts from scratch, Omarchy's seeded configuration is ignored, and integrations that call `herdr` hit whichever binary the alias points at. Rejected: it is the problem.
- **Share configuration only; keep separate state and sessions.** Needs a new lookup, splits the session directory away from Herdr's socket conventions, and still loses sessions and plugins. Rejected as half a migration.
- **Fallback lookup: `superherdr` directories first, then `herdr`.** Two places to look are two places to be wrong; it needs new code and a persisted "which one" decision. Rejected: nobody but the maintainer has 0.1.0 data.
- **Keep both commands.** Two names for one binary doubles the ownership rules and keeps the `superherdr` strings that conflict with upstream. Rejected by the maintainer.
- **`herdr --version` printing `superherdr <version>` or only `herdr <version>`.** The first breaks scripts that read Herdr's version from the second word; the second cannot be told apart from upstream. Rejected in favor of `herdr <base> (superherdr <version>)`.
- **Independent Superherdr version numbers (0.1.x).** Users and tools could not see which Herdr release a Superherdr build is based on, and the numbers look unrelated to Herdr's. Rejected by the maintainer.
- **Four-part version in `status client --json` `version`.** Would break Herdr clients and tools that parse SemVer there. Rejected in favor of the optional `superherdr_version` field.

Drawbacks of the chosen approach: a Herdr and a Superherdr binary can be told apart only by `herdr --version`; Superherdr's configuration keys make Herdr print an "unknown config key" diagnostic after a switch back; both products share one server socket, so the server one of them started is the server the other attaches to.

## Impact

- **UI:** none beyond product wording in help and notices reverting to Herdr's.
- **CLI:** the command is `herdr`. `--version` prints `herdr <base> (superherdr <version>)`. `status client --json` keeps Herdr's fields and adds optional `superherdr_version`. Help, usage, errors, completions and `--skill` use `herdr`.
- **Configuration:** Herdr's `config.toml` is read. Superherdr's only additions are six `keys.*` bindings with defaults (`focus_project`, `clear_project_focus`, `snooze_workspace`, `snooze_project`, `show_snoozed`, `reset_focus_snooze`); Herdr warns about them when set and applies the rest of the file. `worktrees.directory` defaults to `~/.herdr/worktrees`; worktrees created by 0.1.0 under `~/.superherdr/worktrees` stay where they are and remain tracked by path.
- **Persistence:** `session.json` (`SNAPSHOT_VERSION` 3 with the optional `lifetime_id`), `plugins.json`, `plugins/`, `sessions/<name>/`, agent-detection overrides, the endpoint catalog and client preferences live under `~/.config/herdr` and `~/.local/state/herdr`. Herdr ignores `lifetime_id` and drops it on its next write; Superherdr treats it as optional. Snooze stays in a separate `snooze.json` that Herdr ignores and Superherdr honors again after a switch back. The 0.1.0 `superherdr` directories are neither read nor deleted.
- **Both products on one machine, or swapped:** the default session uses `~/.config/herdr/herdr.sock` (JSON API) and `herdr-client.sock` (binary client protocol). The first server owns it; a second server exits with the inherited "server is already running (socket busy)" error, so Superherdr never stops a Herdr server. Either client attaches to whichever server runs when `PROTOCOL_VERSION` (22 in both Herdr 0.9.0 and Superherdr) matches. Herdr and Superherdr on the same base report the same `version`, so a Superherdr client on a Herdr server does not see a stale-server notice; Superherdr-only actions are already gated by advertised capabilities, so nothing misbehaves. A Superherdr client on a Herdr server keeps Focus and the Top-level Agents view, which are client-local, and loses Snooze; a Herdr client on a Superherdr server does not hide snoozed work. Because the directories are shared, the existing rule to bump `PROTOCOL_VERSION` when the private protocol diverges becomes load-bearing.
- **Downgrade to Herdr:** install Herdr over the Superherdr binary, or `brew uninstall jsonmartin/tap/superherdr` and `brew install herdr`; stop the Superherdr server first or let the Herdr client attach to it. Sessions, plugins and configuration carry over; the six keys warn; `snooze.json` is left alone.
- **Herdr clients and plugins:** unchanged; endpoint generation 1, frozen codecs and plugin compatibility level 0.9.0 stay.
- **Upstream merges:** improve. `src/`, `tests/` and `scripts/` return to upstream text except the listed divergences, and `skills/herdr/SKILL.md` is inherited again. `Cargo.toml` still conflicts on `name` and `version` as documented; the `[[bin]]` block is a small, well-named edit. `distribution/install.sh` and `scripts/test_unix_installer.py` stay in `.upstream-sync/ours`.
- **Distribution:** archives contain `herdr`; the installer, formula and PKGBUILD install one file. Any script that re-runs upstream Herdr's installer, including a distribution updater, overwrites `~/.local/bin/herdr` with Herdr silently because that installer checks nothing; Superherdr cannot prevent it and documents it.
- **Debug builds:** `herdr-dev` is shared with an upstream Herdr debug build on the same machine.
- **Inherited Herdr behavior altered:** none at runtime. This change removes Superherdr's earlier divergence (separate directories and command) rather than adding one.

## Verification

- `cargo build --locked` produces `target/debug/herdr`. `tests/drop_in_herdr.rs` asserts `herdr --version` matches `herdr <base>[-<suffix>] (superherdr <base>.<revision>)`, `status client --json` has a three-part `version` and a four-part `superherdr_version`, `--help` contains `Usage: herdr` and the `herdr-dev` or `herdr` directory name, `completion zsh` contains `#compdef herdr`, `--skill` starts with `name: herdr` and never mentions `superherdr`, and `update` exits non-zero with the self-update message.
- Status unit tests: the stale-server check compares `superherdr_version` when both sides report it (0.9.0.1 client, 0.9.0.2 server is stale) and `version` otherwise.
- Configuration unit tests: `app_dir_name()` returns `herdr` and `herdr-dev`; `config_dir()` and `state_dir()` honor XDG overrides; the default `worktrees.directory` is `~/.herdr/worktrees`; a configuration with only Herdr keys loads without diagnostics.
- Persistence test: a `session.json` written by Superherdr still loads after `lifetime_id` is removed, which is what a Herdr rewrite produces.
- `scripts/test_unix_installer.py`: fresh install creates only `herdr`; upgrade over an owned `herdr`; upgrade over the 0.1.0 pair removes `superherdr`; refusal of a regular `herdr` reporting `herdr 0.9.0`, of a non-executable `herdr`, of a foreign `herdr` symlink, and of `herdr` or `superherdr` elsewhere on `PATH`; checksum mismatch, corrupt archive, archive without `herdr` and injected rename failure preserve the previous layout in both forms; the fake curl sees only the owned release URLs and refusals make no request.
- Remote unit tests in `src/remote/attach.rs`: the discovery script emits Herdr's paths and `command -v herdr`; `HERDR_REMOTE_BINARY` validation; the download refusal message.
- `just check` and `just upstream-exclusions-check` pass.
- Manual, on a machine with Herdr configuration and sessions: `herdr session list` shows Herdr's sessions; `herdr` applies Herdr keybindings; `herdr --remote <host>` against an upstream Herdr host attaches and disables Snooze; after the 0.9.0.1 assets exist, `brew upgrade jsonmartin/tap/superherdr` from 0.1.0 leaves only `herdr`.

## Open questions

- Does any third-party tool compare the whole `herdr --version` line rather than the version word? None is known, and Herdr's own preview builds already append a suffix.
- How does Omarchy install and update `herdr` (a package, its own script, or Herdr's installer)? This decides whether an Omarchy update can silently replace Superherdr. Not verified here.
- Herdr 0.9.x client behavior against a Superherdr server was inferred from the shared code, not observed with a real Herdr binary.
- Whether the Arch Linux package belongs in the spec before it is published. It is included so the promise is recorded, and can be dropped at acceptance.
