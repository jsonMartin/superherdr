# Homebrew installation

The shared tap is [jsonmartin/homebrew-tap](https://github.com/jsonmartin/homebrew-tap). Its formula uses versioned assets from [Superherdr releases](https://github.com/jsonmartin/superherdr/releases).

```sh
brew install jsonmartin/tap/superherdr
herdr --version
```

The formula installs a single `herdr` command, which prints `herdr 0.9.0 (superherdr 0.9.0.1)`. It supports macOS Apple Silicon, declares macOS 13 as its minimum, and is tested on macOS 27. Its macOS 27 bottle is assembled from the verified release binary. Linux users should use the release archive or shell installer.

## Existing Herdr installations

Superherdr uses Herdr's configuration, plugins, and sessions, so they carry over. Both formulae provide `herdr`, so if the original Homebrew `herdr` formula is linked, first run:

```sh
brew unlink herdr
brew install jsonmartin/tap/superherdr
```

Unlinking retains the original keg. Check `command -v herdr` if an older standalone executable appears earlier on PATH. Installing does not restart a running server; a running Herdr server keeps serving clients until you stop it.

## Updates

```sh
brew update
brew upgrade jsonmartin/tap/superherdr
```

On Homebrew versions with `brew trust`, run `brew trust --formula jsonmartin/tap/superherdr` once so an unqualified `brew upgrade` includes this formula.

Homebrew owns this installation; do not update it with the shell installer. Building source does not update a Homebrew keg. Binary self-update is disabled. A running server keeps using the previous executable until it restarts; preserve active sessions when updating.
