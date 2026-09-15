# superherdr-compatibility Specification

## Purpose
Run Superherdr as a drop-in `herdr` that shares Herdr's command, configuration and state while identifying itself in version output and staying compatible with Herdr's clients, plugins, integrations and upstream merges. This is a Full specification because it governs executable identity, persisted state locations, environment and wire contracts.

## Requirements

### Requirement: Executable identity
Superherdr SHALL build, install and run as a single command named `herdr`. No distribution channel (release archive, shell installer, Homebrew formula, Arch Linux package or source install) SHALL create a `superherdr` command or alias. The Cargo package SHALL remain `superherdr` with a binary target named `herdr`. Superherdr versions SHALL be `<Herdr base version>.<revision>`, with the revision starting at 1 for each Herdr base. `herdr --version` SHALL print `herdr <base> (superherdr <base>.<revision>)`, where a preview build MAY append a `-` suffix to the base; the first two words SHALL match upstream Herdr's output format, and the parenthesized part identifies Superherdr to users, the shell installer and scripts. The Cargo package version SHALL be the three-part Herdr base. Help output, usage and error messages, `completion` output including `#compdef herdr`, `--default-config` comments and `--skill` SHALL use Herdr's wording and the `herdr` command, except where Superherdr's behavior differs: self-update is disabled and background version checks are off by default. `herdr status client --json` SHALL keep Herdr's fields (`version`, `channel`, `protocol`, `endpoint_protocol_generation`, `endpoint_capabilities`, `binary`, `session`) with the three-part Herdr base in `version`, and SHALL add the optional field `superherdr_version` with the four-part version; the socket API `Pong` response SHALL carry the same optional `superherdr_version`, so server status reports the same pair. The stale-server check SHALL compare `superherdr_version` when both client and server report it, and `version` otherwise.

#### Scenario: Version output
- **WHEN** a user runs `herdr --version` on a release build
- **THEN** the output SHALL be `herdr <base> (superherdr <base>.<revision>)`, for example `herdr 0.9.0 (superherdr 0.9.0.1)`

#### Scenario: Upgrade within one Herdr base
- **WHEN** a Superherdr 0.9.0.2 client checks a running Superherdr 0.9.0.1 server
- **THEN** status SHALL report the server binary as stale even though both report `version` 0.9.0

#### Scenario: Help, completions and skill name the command
- **WHEN** a user runs `herdr --help`, `herdr completion zsh` or `herdr --skill`
- **THEN** the output SHALL refer to `herdr` and SHALL NOT mention a `superherdr` command

#### Scenario: Source install
- **WHEN** a user runs `cargo install --path . --locked`
- **THEN** a `herdr` executable SHALL be installed and no `superherdr` executable SHALL be created

### Requirement: Herdr configuration and state
Release builds SHALL use Herdr's configuration and state directories, `~/.config/herdr` and `~/.local/state/herdr` on Unix, honoring `XDG_CONFIG_HOME` and `XDG_STATE_HOME`; debug builds SHALL use `herdr-dev`. Every derived path (`config.toml`, `session.json`, `plugins.json` and `plugins/`, `sessions/<name>/`, `herdr.sock` and `herdr-client.sock`, agent-detection overrides, the endpoint catalog, client preferences and Superherdr's `snooze.json`) SHALL follow those directories, so an existing Herdr configuration, keybindings, plugins, integrations and sessions are used unchanged. The default `worktrees.directory` SHALL be Herdr's `~/.herdr/worktrees`. Superherdr SHALL NOT read, migrate or delete the `superherdr` directories used by Superherdr 0.1.0. Superherdr SHALL write only what Herdr tolerates: its configuration additions are the six `keys.*` bindings `focus_project`, `clear_project_focus`, `snooze_workspace`, `snooze_project`, `show_snoozed` and `reset_focus_snooze`, which Herdr reports as unknown keys while applying the rest of the file; `session.json` SHALL keep `SNAPSHOT_VERSION` 3 with only the optional `lifetime_id` field added; Snooze state SHALL live in a separate `snooze.json` that Herdr ignores. Superherdr SHALL NOT stop a running Herdr server: the server that owns the session socket keeps it, and a second server exits reporting the busy socket. A separate setup SHALL be available only through existing overrides: `HERDR_CONFIG_PATH` for a separate configuration file, `XDG_CONFIG_HOME` and `XDG_STATE_HOME` for fully separate directories, and `--session <name>` for a separate session and socket that shares the configuration.

#### Scenario: Herdr and Superherdr installed together
- **WHEN** a user with an existing Herdr configuration and sessions starts Superherdr's `herdr` with default settings and no Herdr server running
- **THEN** Superherdr SHALL apply that configuration and keybindings, list those sessions and restore their state, and SHALL store Snooze state beside them in `snooze.json`

#### Scenario: Herdr server already running
- **WHEN** a Herdr server owns the default session socket and the user starts Superherdr's `herdr`
- **THEN** Superherdr SHALL attach as a client when the private protocol version matches, SHALL NOT stop or replace the server, and SHALL disable only the Superherdr-specific actions that server lacks

#### Scenario: Switch back to Herdr
- **WHEN** a user replaces Superherdr with Herdr after setting Superherdr keybindings and snoozing work
- **THEN** Herdr SHALL start with the same sessions, SHALL warn about the unknown `keys.*` entries while applying the rest of the configuration, and SHALL leave `snooze.json` in place for a later Superherdr

#### Scenario: Separate setup by override
- **WHEN** a user starts `herdr` with `XDG_CONFIG_HOME` and `XDG_STATE_HOME` pointing at dedicated directories
- **THEN** Superherdr SHALL read and write only those directories and SHALL NOT touch `~/.config/herdr` or `~/.local/state/herdr`

#### Scenario: Superherdr 0.1.0 directories
- **WHEN** a user upgrades from Superherdr 0.1.0 with data in `~/.config/superherdr`
- **THEN** Superherdr SHALL neither read nor delete that directory, and files the user wants are copied into `~/.config/herdr` by hand

### Requirement: Inherited integration names
`HERDR_*` environment variables, socket filenames, integration asset names and API/schema identifiers SHALL keep their Herdr names, and the bundled integration assets SHALL invoke `herdr`, which is the installed command. Sockets SHALL live inside the Herdr directories above. For SSH, `HERDR_REMOTE_BINARY` SHALL name a local build to install on the remote host. Default remote discovery SHALL look for `herdr` on the remote login shell's `PATH` and in Herdr's known install locations, and SHALL use the first candidate that passes the existing endpoint generation and capability checks, whether it is Herdr or Superherdr; Superherdr-specific shared features are unavailable on a Herdr remote. When no compatible remote `herdr` exists, Superherdr SHALL offer, after the existing interactive confirmation, to install the local Superherdr binary or the `HERDR_REMOTE_BINARY` build at `~/.local/bin/herdr` on the remote host, and SHALL NOT download Herdr or Superherdr release assets.

#### Scenario: Agent integration inside a pane
- **WHEN** an agent integration reads `HERDR_SOCKET_PATH` inside a Superherdr pane
- **THEN** it SHALL reach the Superherdr server that owns the pane

#### Scenario: Remote host runs Herdr
- **WHEN** `herdr --remote <host>` finds an upstream Herdr on the remote host that passes the endpoint checks
- **THEN** Superherdr SHALL attach to it without installing anything and SHALL disable only Snooze for that host

#### Scenario: Remote host has no compatible herdr
- **WHEN** the remote `herdr` is missing or fails the endpoint checks
- **THEN** Superherdr SHALL ask before installing the local binary at `~/.local/bin/herdr` and SHALL NOT download a release asset

### Requirement: Endpoint and protocol compatibility
Superherdr SHALL keep Herdr's client endpoint generation 1 and its frozen codecs, fixtures and method shapes unchanged. New shared features SHALL be additive and optional, so a Herdr client or server without them loses only the affected action. Changing the private same-install protocol incompatibly SHALL increase `PROTOCOL_VERSION` once per published protocol.

#### Scenario: Herdr client without shared Snooze
- **WHEN** a client without Superherdr's optional Snooze feature connects
- **THEN** the connection SHALL succeed and that client SHALL simply not hide snoozed work

### Requirement: Plugin compatibility level
Superherdr SHALL report the Herdr plugin compatibility level it inherits (`0.9.0` for Superherdr 0.1.0), independent of Superherdr's own version, so existing Herdr plugins keep working.

#### Scenario: Install an existing Herdr plugin
- **WHEN** a user installs a plugin that requires Herdr plugin compatibility 0.9.0
- **THEN** Superherdr SHALL accept it

### Requirement: Upstream agent-detection catalog
While Superherdr's detection engine and rules match Herdr's, release builds of Superherdr SHALL fetch agent-detection rule updates from Herdr's published catalog at startup when `update.manifest_check` is enabled, which is the default. It SHALL reject rules that require a newer detection engine and SHALL fall back to bundled rules when the catalog is unreachable. Setting `update.manifest_check = false` SHALL stop all catalog requests. A change that makes Superherdr's detection engine or rules diverge from Herdr's SHALL move the catalog to Superherdr-hosted infrastructure in the same change.

#### Scenario: Catalog unreachable
- **WHEN** the catalog cannot be fetched at startup
- **THEN** agent detection SHALL continue with the bundled rules

#### Scenario: Opt out
- **WHEN** a user sets `update.manifest_check = false`
- **THEN** Superherdr SHALL make no request to the catalog
