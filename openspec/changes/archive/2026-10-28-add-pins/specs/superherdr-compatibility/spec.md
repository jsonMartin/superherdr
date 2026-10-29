## MODIFIED Requirements

### Requirement: Herdr configuration and state
Release builds SHALL use Herdr's configuration and state directories, `~/.config/herdr` and `~/.local/state/herdr` on Unix, honoring `XDG_CONFIG_HOME` and `XDG_STATE_HOME`; debug builds SHALL use `herdr-dev`. Every derived path (`config.toml`, `session.json`, `plugins.json` and `plugins/`, `sessions/<name>/`, `herdr.sock` and `herdr-client.sock`, agent-detection overrides, the endpoint catalog, client preferences and Superherdr's `snooze.json` and `pins.json`) SHALL follow those directories, so an existing Herdr configuration, keybindings, plugins, integrations and sessions are used unchanged. The default `worktrees.directory` SHALL be Herdr's `~/.herdr/worktrees`. Superherdr SHALL NOT read, migrate or delete the `superherdr` directories used by Superherdr 0.1.0. Superherdr SHALL write only what Herdr tolerates: its configuration additions are the six `keys.*` bindings `focus_project`, `clear_project_focus`, `snooze_workspace`, `snooze_project`, `show_snoozed` and `reset_focus_snooze`, which Herdr reports as unknown keys while applying the rest of the file; `session.json` SHALL keep `SNAPSHOT_VERSION` 3 with only the optional `lifetime_id` field added; Snooze state SHALL live in a separate `snooze.json` and pin state in a separate `pins.json`, both of which Herdr ignores. Superherdr SHALL NOT stop a running Herdr server: the server that owns the session socket keeps it, and a second server exits reporting the busy socket. A separate setup SHALL be available only through existing overrides: `HERDR_CONFIG_PATH` for a separate configuration file, `XDG_CONFIG_HOME` and `XDG_STATE_HOME` for fully separate directories, and `--session <name>` for a separate session and socket that shares the configuration.

#### Scenario: Herdr and Superherdr installed together
- **WHEN** a user with an existing Herdr configuration and sessions starts Superherdr's `herdr` with default settings and no Herdr server running
- **THEN** Superherdr SHALL apply that configuration and keybindings, list those sessions and restore their state, and SHALL store Snooze state and pins beside them in `snooze.json` and `pins.json`

#### Scenario: Herdr server already running
- **WHEN** a Herdr server owns the default session socket and the user starts Superherdr's `herdr`
- **THEN** Superherdr SHALL attach as a client when the private protocol version matches, SHALL NOT stop or replace the server, and SHALL disable only the Superherdr-specific actions that server lacks, such as Snooze and pins

#### Scenario: Switch back to Herdr
- **WHEN** a user replaces Superherdr with Herdr after setting Superherdr keybindings, snoozing work and pinning work
- **THEN** Herdr SHALL start with the same sessions, SHALL warn about the unknown `keys.*` entries while applying the rest of the configuration, and SHALL leave `snooze.json` and `pins.json` in place for a later Superherdr

#### Scenario: Separate setup by override
- **WHEN** a user starts `herdr` with `XDG_CONFIG_HOME` and `XDG_STATE_HOME` pointing at dedicated directories
- **THEN** Superherdr SHALL read and write only those directories and SHALL NOT touch `~/.config/herdr` or `~/.local/state/herdr`

#### Scenario: Superherdr 0.1.0 directories
- **WHEN** a user upgrades from Superherdr 0.1.0 with data in `~/.config/superherdr`
- **THEN** Superherdr SHALL neither read nor delete that directory, and files the user wants are copied into `~/.config/herdr` by hand

### Requirement: Inherited integration names
`HERDR_*` environment variables, socket filenames, integration asset names and API/schema identifiers SHALL keep their Herdr names, and the bundled integration assets SHALL invoke `herdr`, which is the installed command. Sockets SHALL live inside the Herdr directories above. For SSH, `HERDR_REMOTE_BINARY` SHALL name a local build to install on the remote host. Default remote discovery SHALL look for `herdr` on the remote login shell's `PATH` and in Herdr's known install locations, and SHALL use the first candidate that passes the existing endpoint generation and capability checks, whether it is Herdr or Superherdr; Superherdr-specific shared features, Snooze and pins, are unavailable on a Herdr remote. When no compatible remote `herdr` exists, Superherdr SHALL offer, after the existing interactive confirmation, to install the local Superherdr binary or the `HERDR_REMOTE_BINARY` build at `~/.local/bin/herdr` on the remote host, and SHALL NOT download Herdr or Superherdr release assets.

#### Scenario: Agent integration inside a pane
- **WHEN** an agent integration reads `HERDR_SOCKET_PATH` inside a Superherdr pane
- **THEN** it SHALL reach the Superherdr server that owns the pane

#### Scenario: Remote host runs Herdr
- **WHEN** `herdr --remote <host>` finds an upstream Herdr on the remote host that passes the endpoint checks
- **THEN** Superherdr SHALL attach to it without installing anything and SHALL disable only Snooze and pins for that host

#### Scenario: Remote host has no compatible herdr
- **WHEN** the remote `herdr` is missing or fails the endpoint checks
- **THEN** Superherdr SHALL ask before installing the local binary at `~/.local/bin/herdr` and SHALL NOT download a release asset

### Requirement: Endpoint and protocol compatibility
Superherdr SHALL keep Herdr's client endpoint generation 1 and its frozen codecs, fixtures and method shapes unchanged. New shared features SHALL be additive and optional, so a Herdr client or server without them loses only the affected action. Changing the private same-install protocol incompatibly SHALL increase `PROTOCOL_VERSION` once per published protocol.

#### Scenario: Herdr client without shared Snooze
- **WHEN** a client without Superherdr's optional Snooze feature connects
- **THEN** the connection SHALL succeed and that client SHALL simply not hide snoozed work

#### Scenario: Herdr client without pins
- **WHEN** a client without Superherdr's optional pin feature connects
- **THEN** the connection SHALL succeed and that client SHALL show its usual ordering without pins
