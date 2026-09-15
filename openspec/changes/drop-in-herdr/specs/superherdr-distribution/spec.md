## ADDED Requirements

### Requirement: Arch Linux package
The `superherdr-bin` AUR package SHALL use the four-part Superherdr version as `pkgver` and SHALL install the Linux x86_64 release binary as `/usr/bin/herdr` only, SHALL provide `superherdr` and `herdr`, and SHALL conflict with `superherdr`, `herdr`, `herdr-bin` and `herdr-git`, so a package that depends on `herdr` accepts Superherdr and no system holds two `herdr` binaries.

#### Scenario: Herdr package installed
- **WHEN** `herdr-bin` is installed and a user installs `superherdr-bin`
- **THEN** pacman SHALL report the conflict and SHALL install `superherdr-bin` only after the user removes `herdr-bin`

## MODIFIED Requirements

### Requirement: Release assets
Each release SHALL be published under the tag `superherdr-v<version>` in `jsonMartin/superherdr`, where `<version>` is the four-part Superherdr version (for example `superherdr-v0.9.0.1`), with an archive per supported target named `superherdr-<version>-<target>.tar.gz`, an `install.sh`, and a `SHA256SUMS` file listing every asset. Supported targets SHALL be `macos-aarch64` and `linux-x86_64`. Each archive SHALL contain the `herdr` binary, `LICENSE` and `licenses/`, and SHALL NOT contain a `superherdr` file.

#### Scenario: Verify a download
- **WHEN** a user downloads an archive and `SHA256SUMS` from a release
- **THEN** the archive's SHA-256 digest SHALL match its `SHA256SUMS` entry

#### Scenario: Archive contents
- **WHEN** a user extracts a release archive
- **THEN** it SHALL contain `herdr`, `LICENSE` and `licenses/` and no `superherdr` entry

### Requirement: Shell installer
The installer SHALL support Linux x86_64 and macOS Apple Silicon and SHALL refuse other platforms before downloading. It SHALL verify the archive against `SHA256SUMS` before extracting. It SHALL install a single `herdr` executable into `~/.local/bin` or `SUPERHERDR_INSTALL_DIR`, and licenses into `${XDG_DATA_HOME:-~/.local/share}/superherdr/licenses`, without root, and SHALL NOT create a `superherdr` file or alias. The version SHALL default to the installer's four-part release version and MAY be set by argument or `SUPERHERDR_VERSION`.

#### Scenario: Checksum mismatch
- **WHEN** the downloaded archive does not match `SHA256SUMS`
- **THEN** the installer SHALL exit with an error and leave any existing installation unchanged

#### Scenario: Unsupported platform
- **WHEN** the installer runs on Linux aarch64, Intel macOS or Android
- **THEN** it SHALL exit with an error before downloading anything

### Requirement: Installation ownership
The installer SHALL update only an installation it created, and SHALL decide before downloading. It SHALL treat `<install dir>/herdr` as its own when it is a regular file whose `--version` output contains `(superherdr `, or when the Superherdr 0.1.0 layout is present: a `herdr` symlink whose target is exactly the regular `<install dir>/superherdr`. Upgrading the 0.1.0 layout SHALL replace the `herdr` symlink with the new binary and then delete the owned `superherdr` file, so no second command remains. It SHALL also treat a regular `superherdr` beside an owned regular `herdr` as a leftover from an interrupted 0.1.0 upgrade and remove it. It SHALL refuse, naming the path: a `herdr` symlink with any other target; a regular `herdr` that is not executable or does not identify itself as Superherdr, such as upstream Herdr's installed binary; a `superherdr` file without its 0.1.0 alias or an owned `herdr` beside it; and an executable `herdr` or `superherdr` in any `PATH` directory other than the install directory, including directories later on `PATH` than the install directory. It SHALL stage the new binary inside the install directory and rename it into place. It SHALL NOT modify configuration, state or running sessions.

#### Scenario: Original Herdr on PATH
- **WHEN** a `herdr` executable from another installation is on `PATH`
- **THEN** the installer SHALL exit without changes and name the conflicting path

#### Scenario: Interrupted replacement
- **WHEN** replacing the binary fails after download
- **THEN** the previous `herdr`, or the previous 0.1.0 pair, SHALL remain intact and the staging directory SHALL be removed

#### Scenario: Upstream Herdr in the install directory
- **WHEN** `<install dir>/herdr` is a regular file whose `--version` output does not contain `(superherdr `
- **THEN** the installer SHALL exit without downloading or changing anything and SHALL name the path

#### Scenario: Herdr shadowed later on PATH
- **WHEN** the install directory is earlier on `PATH` than a directory containing upstream Herdr's `herdr`
- **THEN** the installer SHALL exit without changes and name the shadowed path

#### Scenario: Resume an interrupted 0.1.0 upgrade
- **WHEN** the install directory holds an owned regular `herdr` and a leftover regular `superherdr`
- **THEN** the installer SHALL complete the upgrade and remove `superherdr`

#### Scenario: Upgrade from Superherdr 0.1.0
- **WHEN** the install directory holds a regular `superherdr` and a `herdr` symlink pointing at it
- **THEN** the installer SHALL install the new `herdr` as a regular file, remove `superherdr`, and leave no symlink

### Requirement: Homebrew formula
The `jsonmartin/tap/superherdr` formula SHALL use the four-part Superherdr version and SHALL install the release's macOS Apple Silicon binary as the single `herdr` command, SHALL NOT install a `superherdr` command, SHALL declare a conflict with the `herdr` formula, and SHALL be updated with `brew upgrade`. Upgrading from the 0.1.0 formula SHALL leave no `superherdr` link.

#### Scenario: Linked original formula
- **WHEN** the original Homebrew `herdr` formula is linked
- **THEN** installation SHALL report the conflict, and `brew unlink herdr` SHALL allow installation while keeping the original keg

#### Scenario: Upgrade from 0.1.0
- **WHEN** a user upgrades the formula from 0.1.0
- **THEN** `herdr --version` SHALL print `herdr <base> (superherdr <version>)` and `superherdr` SHALL no longer resolve to a Homebrew link

### Requirement: No self-update
Superherdr SHALL NOT replace its own binary. `herdr update` SHALL fail with a message that self-update is disabled for Superherdr, and background version checks SHALL be off by default.

#### Scenario: Run update
- **WHEN** a user runs `herdr update`
- **THEN** the command SHALL exit unsuccessfully and state that self-update is disabled for Superherdr
