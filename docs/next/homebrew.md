# Local Homebrew installation

Superherdr currently uses a machine-local Homebrew tap. It is not available from a published tap or Homebrew core.

On a machine with the local tap and its archives already prepared:

```sh
brew install local/superherdr/superherdr local/superherdr/herdr-og
```

The Superherdr formula installs `superherdr` and a `herdr` executable symlink:

```ruby
bin.install "superherdr"
bin.install_symlink bin/"superherdr" => "herdr"
```

`herdr-og` preserves a snapshot of the original executable. The original Homebrew `herdr` package remains installed but unlinked. Existing absolute executable paths remain intact for running sessions.

The local shell puts the Superherdr formula's `bin` directory before the old standalone executable. Check a new shell with:

```sh
command -v herdr
herdr --version
superherdr --version
herdr-og --version
```

Both `herdr` and `superherdr` should report `superherdr`; `herdr-og` should report `herdr`. Command names do not change configuration directories or inherited `HERDR_*` session overrides. Existing panes continue to address their existing session.

A Homebrew formula alias changes package lookup, not executable names. The executable symlink above supplies the command alias. There is no generic `brew install --as` option.

## Updating and sharing

These packages are local snapshots. Keep the local formulas and archives available for reinstalls. Building new source does not update the installed package, and `brew upgrade` does not fetch unpublished project changes. Refresh the binary archive, checksum and bottle before reinstalling an updated local package.

Do not commit machine-specific `file://` URLs, binary archives, Homebrew receipts, or shell backups. A shared formula needs an established release location and checksums for actual published artifacts. The shared formula is being prepared in [jsonmartin/homebrew-tap](https://github.com/jsonmartin/homebrew-tap), using assets from [jsonmartin/superherdr](https://github.com/jsonmartin/superherdr). Its release is still a draft. After publication and installation verification, the package command will be `brew install jsonmartin/tap/superherdr`; it will provide both `superherdr` and `herdr`. The personal `herdr-og` snapshot stays local.
