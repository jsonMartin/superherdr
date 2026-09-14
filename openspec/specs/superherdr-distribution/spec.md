# superherdr-distribution Specification

## Purpose
Define how Superherdr is released and installed: versioned GitHub release assets, a checksum-verifying shell installer, and a Homebrew formula. Installing never takes over another installation or touches session state.

## Requirements

### Requirement: Release assets
Each release SHALL be published under the tag `superherdr-v<version>` in `jsonMartin/superherdr`, with an archive per supported target named `superherdr-<version>-<target>.tar.gz`, an `install.sh`, and a `SHA256SUMS` file listing every asset. Supported targets SHALL be `macos-aarch64` and `linux-x86_64`. Each archive SHALL contain the `superherdr` binary, `LICENSE` and `licenses/`.

#### Scenario: Verify a download
- **WHEN** a user downloads an archive and `SHA256SUMS` from a release
- **THEN** the archive's SHA-256 digest SHALL match its `SHA256SUMS` entry

### Requirement: Shell installer
The installer SHALL support Linux x86_64 and macOS Apple Silicon and SHALL refuse other platforms before downloading. It SHALL verify the archive against `SHA256SUMS` before extracting. It SHALL install `superherdr` and a `herdr` symlink into `~/.local/bin` or `SUPERHERDR_INSTALL_DIR`, and licenses into `${XDG_DATA_HOME:-~/.local/share}/superherdr/licenses`, without root. The version SHALL default to the installer's release and MAY be set by argument or `SUPERHERDR_VERSION`.

#### Scenario: Checksum mismatch
- **WHEN** the downloaded archive does not match `SHA256SUMS`
- **THEN** the installer SHALL exit with an error and leave any existing installation unchanged

#### Scenario: Unsupported platform
- **WHEN** the installer runs on Linux aarch64, Intel macOS or Android
- **THEN** it SHALL exit with an error before downloading anything

### Requirement: Installation ownership
The installer SHALL update only its own previous installation: a regular `superherdr` file in the install directory with a `herdr` symlink pointing at exactly that file. It SHALL refuse, before downloading, a symlinked `superherdr`, a `herdr` symlink or file it does not own, a `superherdr` without its owned alias, and any `superherdr` or `herdr` found on `PATH` outside the install directory. It SHALL stage the new binary inside the install directory and rename it into place. It SHALL NOT modify configuration, state or running sessions.

#### Scenario: Original Herdr on PATH
- **WHEN** a `herdr` executable from another installation is on `PATH`
- **THEN** the installer SHALL exit without changes and name the conflicting path

#### Scenario: Interrupted replacement
- **WHEN** replacing the binary fails after download
- **THEN** the previous binary and alias SHALL remain intact and the staging directory SHALL be removed

### Requirement: Homebrew formula
The `jsonmartin/tap/superherdr` formula SHALL install the release's macOS Apple Silicon binary with `superherdr` and `herdr` commands, SHALL declare a conflict with the `herdr` formula, and SHALL be updated with `brew upgrade`.

#### Scenario: Linked original formula
- **WHEN** the original Homebrew `herdr` formula is linked
- **THEN** installation SHALL report the conflict, and `brew unlink herdr` SHALL allow installation while keeping the original keg

### Requirement: No self-update
Superherdr SHALL NOT replace its own binary. `superherdr update` SHALL fail with a message that self-update is disabled, and background version checks SHALL be off by default.

#### Scenario: Run update
- **WHEN** a user runs `superherdr update`
- **THEN** the command SHALL exit unsuccessfully and state that self-update is disabled for Superherdr
