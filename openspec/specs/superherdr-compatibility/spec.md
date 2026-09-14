# superherdr-compatibility Specification

## Purpose
Keep Superherdr distinguishable from Herdr on the same machine while staying compatible with Herdr's clients, plugins, integrations and upstream merges. This is a Full specification because it governs executable identity, persisted state locations, environment and wire contracts.

## Requirements

### Requirement: Executable identity
The binary SHALL be named `superherdr`. `superherdr --version` SHALL print `superherdr <version>`, where a preview build MAY append a `-` suffix. Help output SHALL use `superherdr`, `completion zsh` SHALL define `#compdef superherdr`, and `--skill` SHALL describe `superherdr` without instructing agents to run `herdr`. Installing Superherdr from source SHALL NOT create or overwrite a `herdr` executable.

#### Scenario: Version output
- **WHEN** a user runs `superherdr --version` on a release build
- **THEN** the output SHALL be `superherdr <version>`

### Requirement: Separate configuration and state
Release builds SHALL use `superherdr` configuration and state directories (`~/.config/superherdr` and `~/.local/state/superherdr` on Unix, honoring XDG overrides), and debug builds SHALL use `superherdr-dev`. Unless a user points an inherited override such as `HERDR_CONFIG_PATH` at Herdr's files, Superherdr SHALL NOT read, migrate or write Herdr's default configuration or state directories, and SHALL NOT stop a running Herdr server.

#### Scenario: Herdr and Superherdr installed together
- **WHEN** a user with existing Herdr sessions starts Superherdr with default settings
- **THEN** Superherdr SHALL start with its own state, and the Herdr sessions SHALL be unchanged

### Requirement: Inherited integration names
`HERDR_*` environment variables, socket filenames, integration asset names and API/schema identifiers SHALL keep their Herdr names. Sockets SHALL live inside Superherdr's own directories. `HERDR_REMOTE_BINARY` SHALL select the remote binary for SSH, and default remote discovery SHALL look for `superherdr`, never downloading upstream Herdr in its place.

#### Scenario: Agent integration inside a pane
- **WHEN** an agent integration reads `HERDR_SOCKET_PATH` inside a Superherdr pane
- **THEN** it SHALL reach the Superherdr server that owns the pane

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
