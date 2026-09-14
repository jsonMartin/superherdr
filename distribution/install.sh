#!/bin/sh
set -eu

BIN="superherdr"
ALIAS="herdr"
DEFAULT_VERSION="0.1.0"
# Fixed to Superherdr's own GitHub releases; tests stub curl rather than
# re-pointing the installer at another base.
RELEASE_URL_BASE="https://github.com/jsonMartin/superherdr/releases/download"
INSTALL_DIR="${SUPERHERDR_INSTALL_DIR:-$HOME/.local/bin}"
# tolerate trailing slashes (e.g. SUPERHERDR_INSTALL_DIR=~/.local/bin/)
# without collapsing the root directory to an empty string
while [ "$INSTALL_DIR" != "/" ] && [ "${INSTALL_DIR%/}" != "$INSTALL_DIR" ]; do
    INSTALL_DIR="${INSTALL_DIR%/}"
done
LICENSE_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/superherdr/licenses"
# both are mktemp-created by this run and removed by the EXIT trap; nothing
# else is ever deleted
TMP=""
STAGE=""

main() {
    echo ""
    echo "  superherdr installer (terminal agent runtime built on Herdr)"
    echo ""

    VERSION="${SUPERHERDR_VERSION:-${1:-$DEFAULT_VERSION}}"
    if ! printf '%s' "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
        err "invalid version '$VERSION': expected N.N.N (e.g. 0.1.0)"
    fi

    # detect platform; only release targets that actually ship are supported
    OS="$(uname -s)"
    case "$OS" in
        Linux)  os="linux" ;;
        Darwin) os="macos" ;;
        *)      err "unsupported OS: $OS (Superherdr releases Linux x86_64 and macOS Apple Silicon)" ;;
    esac
    # Termux reports Linux but uses bionic libc and a different filesystem
    # layout; no Superherdr build targets it
    if [ "$os" = "linux" ] && [ "$(uname -o 2>/dev/null || true)" = "Android" ]; then
        err "Android/Termux is not supported by Superherdr release binaries. SSH to a supported host instead."
    fi

    ARCH="$(uname -m)"
    case "$ARCH" in
        x86_64|amd64)   arch="x86_64" ;;
        aarch64|arm64)  arch="aarch64" ;;
        *)              err "unsupported architecture: $ARCH" ;;
    esac

    case "${os}-${arch}" in
        linux-x86_64)  target="linux-x86_64" ;;
        macos-aarch64) target="macos-aarch64" ;;
        linux-aarch64) err "no Superherdr release build exists for Linux aarch64 yet" ;;
        macos-x86_64)  err "no Superherdr release build exists for Intel macOS yet" ;;
        *)             err "unsupported platform: ${os}-${arch}" ;;
    esac
    log "detected ${target}"

    need curl
    need awk
    need tar

    ARCHIVE="superherdr-${VERSION}-${target}.tar.gz"
    BASE_URL="${RELEASE_URL_BASE}/superherdr-v${VERSION}"

    # refuse conflicting installs before downloading anything; never touch
    # another installation (package-managed binaries, upstream Herdr, or a
    # Homebrew superherdr) and never disturb existing config, state, or sessions
    check_destinations

    # fetch the published SHA256SUMS and this archive's expected digest
    log "fetching checksums for v${VERSION}..."
    SUMS="$(curl -fsSL --retry 3 --connect-timeout 10 --max-time 20 "${BASE_URL}/SHA256SUMS")" \
        || err "can't download ${BASE_URL}/SHA256SUMS. The release must be published (draft assets are not anonymously downloadable)."
    SHA256="$(printf '%s\n' "$SUMS" | awk -v name="$ARCHIVE" '
        $2 == name || $2 == "*" name { print $1; exit }
    ')"
    if ! printf '%s\n' "$SHA256" | grep -Eq '^[0-9a-fA-F]{64}$'; then
        err "SHA256SUMS does not list ${ARCHIVE}"
    fi
    SHA256="$(printf '%s\n' "$SHA256" | awk '{ print tolower($0) }')"

    if command -v sha256sum >/dev/null 2>&1; then
        SHA256_TOOL="sha256sum"
    elif command -v shasum >/dev/null 2>&1; then
        SHA256_TOOL="shasum"
    elif command -v openssl >/dev/null 2>&1; then
        SHA256_TOOL="openssl"
    else
        err "SHA-256 verification requires sha256sum, shasum, or openssl"
    fi

    log "downloading ${ARCHIVE}..."
    TMP="$(mktemp -d)"
    trap cleanup EXIT

    if ! curl -fsSL --retry 3 --connect-timeout 10 --max-time 300 "${BASE_URL}/${ARCHIVE}" -o "${TMP}/${ARCHIVE}"; then
        err "download failed from ${BASE_URL}/${ARCHIVE}"
    fi

    case "$SHA256_TOOL" in
        sha256sum) ACTUAL_SHA256="$(sha256sum < "${TMP}/${ARCHIVE}" | awk '{ print $1 }')" ;;
        shasum)    ACTUAL_SHA256="$(shasum -a 256 < "${TMP}/${ARCHIVE}" | awk '{ print $1 }')" ;;
        openssl)   ACTUAL_SHA256="$(openssl dgst -sha256 < "${TMP}/${ARCHIVE}" | awk '{ print $NF }')" ;;
    esac
    if [ "$ACTUAL_SHA256" != "$SHA256" ]; then
        err "downloaded Superherdr checksum did not match"
    fi

    mkdir "${TMP}/pkg"
    if ! tar -xzf "${TMP}/${ARCHIVE}" -C "${TMP}/pkg"; then
        err "could not extract ${ARCHIVE}"
    fi
    if [ ! -f "${TMP}/pkg/${BIN}" ]; then
        err "archive does not contain ${BIN}"
    fi

    # install into the user-local directory (no root); re-running this
    # installer is the direct-install update path. The new binary is staged
    # inside INSTALL_DIR and chmodded before the final rename so the last
    # move is a same-filesystem rename and an interrupted copy can never
    # truncate the previous binary.
    mkdir -p "$INSTALL_DIR"
    STAGE="$(mktemp -d "${INSTALL_DIR}/.superherdr-stage.XXXXXX")"
    mv "${TMP}/pkg/${BIN}" "${STAGE}/${BIN}"
    chmod 755 "${STAGE}/${BIN}"
    mv -f "${STAGE}/${BIN}" "${INSTALL_DIR}/${BIN}"
    rm -rf "${STAGE}"
    STAGE=""

    # herdr alias so inherited HERDR_* tooling keeps working; only reached
    # when check_destinations confirmed the name is ours or free
    rm -f "${INSTALL_DIR}/${ALIAS}"
    ln -s "${BIN}" "${INSTALL_DIR}/${ALIAS}"

    # distribute the bundled licenses alongside the binary
    mkdir -p "$LICENSE_DIR"
    if [ -f "${TMP}/pkg/LICENSE" ]; then
        cp "${TMP}/pkg/LICENSE" "${LICENSE_DIR}/LICENSE"
    fi
    if [ -d "${TMP}/pkg/licenses" ]; then
        cp "${TMP}/pkg/licenses/"* "$LICENSE_DIR/"
    fi

    log "installed ${BIN} to ${INSTALL_DIR}/${BIN}"
    log "created alias ${INSTALL_DIR}/${ALIAS} -> ${BIN}"
    log "licenses copied to ${LICENSE_DIR}"

    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            echo ""
            warn "${INSTALL_DIR} is not in your PATH"
            echo "  add it to your shell config:"
            echo ""
            echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
            echo ""
            ;;
    esac

    echo ""
    log "ready. run 'superherdr' (or 'herdr') to get started."
    log "to update later: re-run this installer. it does not self-update."
    echo ""
}

cleanup() {
    for path in "$STAGE" "$TMP"; do
        if [ -n "$path" ]; then
            rm -rf "$path"
        fi
    done
}

# Fail safely on any existing destination this installer did not create:
# package-managed symlinks, Homebrew superherdr, or upstream Herdr's `herdr`.
# A previous direct install is recognized only by its exact owned pairing: a
# regular ${INSTALL_DIR}/superherdr plus a ${INSTALL_DIR}/herdr symlink whose
# target is exactly that binary (relative or absolute). An external symlink is
# never mistaken for ownership, even when its target ends in /superherdr.
check_destinations() {
    LINK_TARGET=""
    if [ -L "${INSTALL_DIR}/${BIN}" ]; then
        err "${INSTALL_DIR}/${BIN} is a symlink; refusing to overwrite it (it may be package-managed). Remove it manually if it is yours."
    fi
    if [ -L "${INSTALL_DIR}/${ALIAS}" ]; then
        LINK_TARGET="$(readlink "${INSTALL_DIR}/${ALIAS}")"
        case "$LINK_TARGET" in
            "$BIN"|"${INSTALL_DIR}/${BIN}") ;;
            *) err "${INSTALL_DIR}/${ALIAS} is a symlink to ${LINK_TARGET}; refusing to overwrite it (it may be package-managed or another install)." ;;
        esac
    elif [ -e "${INSTALL_DIR}/${ALIAS}" ]; then
        err "${INSTALL_DIR}/${ALIAS} already exists as a regular file; it may be the original Herdr. Remove or move it yourself, or set SUPERHERDR_INSTALL_DIR."
    fi
    if [ -e "${INSTALL_DIR}/${BIN}" ]; then
        case "$LINK_TARGET" in
            "$BIN"|"${INSTALL_DIR}/${BIN}") ;;
            *) err "${INSTALL_DIR}/${BIN} already exists but ${INSTALL_DIR}/${ALIAS} does not point at it; this installer did not create it. Remove it manually if it is yours." ;;
        esac
    fi
    for name in "$BIN" "$ALIAS"; do
        if FOUND="$(command -v "$name" 2>/dev/null)"; then
            case "$FOUND" in
                "${INSTALL_DIR}/${name}") ;;
                *)
                    err "existing ${name} found at ${FOUND} (outside ${INSTALL_DIR}). Installing would shadow or conflict with it. Remove it first (e.g. 'brew uninstall' or your package manager); this installer never takes over another installation."
                    ;;
            esac
        fi
    done
}

log()  { printf '  \033[32m>\033[0m %s\n' "$1"; }
warn() { printf '  \033[33m!\033[0m %s\n' "$1"; }
err()  { printf '  \033[31m✗\033[0m %s\n' "$1" >&2; exit 1; }

need() {
    if ! command -v "$1" >/dev/null 2>&1; then
        err "requires '$1' — install it first, or download a release archive manually from https://github.com/jsonMartin/superherdr/releases"
    fi
}

main "$@"
