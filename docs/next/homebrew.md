# Homebrew installation

The shared tap is [jsonmartin/homebrew-tap](https://github.com/jsonmartin/homebrew-tap). Its formula uses versioned assets from [Superherdr releases](https://github.com/jsonmartin/superherdr/releases).

```sh
brew install jsonmartin/tap/superherdr
herdr --version
superherdr --version
```

Both commands report `superherdr 0.1.0`. The formula installs the `superherdr` binary and a `herdr` symlink.

The 0.1.0 formula supports macOS Apple Silicon, declares macOS 13 as its minimum, and was tested on macOS 27. Its macOS 27 bottle is assembled from the verified release binary. Linux users should use the release archive or shell installer.

## Existing Herdr installations

If the original Homebrew `herdr` formula is linked, first run:

```sh
brew unlink herdr
brew install jsonmartin/tap/superherdr
```

Unlinking retains the original keg. Check `command -v herdr` if an older standalone executable appears earlier on PATH. Changing the executable does not migrate state or restart a running server. Superherdr uses separate state directories by default.

## Updates

```sh
brew update
brew upgrade jsonmartin/tap/superherdr
```

On Homebrew versions with `brew trust`, run `brew trust --formula jsonmartin/tap/superherdr` once so an unqualified `brew upgrade` includes this formula.

Homebrew owns this installation; do not update it with the shell installer. Building source does not update a Homebrew keg. Binary self-update is disabled. A running server may continue using the previous executable until a compatible handoff or an appropriate restart; preserve active sessions when updating.
