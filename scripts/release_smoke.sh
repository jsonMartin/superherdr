#!/bin/sh
# Smoke-test a release binary in isolation: its own XDG directories, a named
# "smoke" session, one workspace, and a PTY roundtrip. Inherits nothing from
# the environment, so it never touches an existing server or the user's config.
# Used by the build workflows and `just release-smoke`.
# Usage: release_smoke.sh <path-to-herdr-binary>
set -eu

if [ "$#" -ne 1 ] || [ ! -x "$1" ]; then
    echo "usage: $0 <path-to-herdr-binary>" >&2
    exit 2
fi
bin="$1"

# short path: the session socket lives under here and macOS socket paths cap
# at 104 bytes
tmp="$(mktemp -d /tmp/hs.XXXXXX)"
export XDG_CONFIG_HOME="$tmp/config" XDG_RUNTIME_DIR="$tmp/run"
# release binaries use the "herdr" config dir, debug binaries "herdr-dev"
mkdir -p "$XDG_CONFIG_HOME/herdr" "$XDG_CONFIG_HOME/herdr-dev" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
echo 'onboarding = false' > "$XDG_CONFIG_HOME/herdr/config.toml"
echo 'onboarding = false' > "$XDG_CONFIG_HOME/herdr-dev/config.toml"
unset HERDR_SOCKET_PATH HERDR_CLIENT_SOCKET_PATH HERDR_ENV
"$bin" --session smoke server > "$tmp/server.log" 2>&1 &
server_pid=$!
trap 'kill "$server_pid" 2>/dev/null || true; rm -rf "$tmp"' EXIT
socket=""
for _ in $(seq 100); do
    for dir in herdr herdr-dev; do
        if [ -S "$XDG_CONFIG_HOME/$dir/sessions/smoke/herdr.sock" ]; then
            socket="$XDG_CONFIG_HOME/$dir/sessions/smoke/herdr.sock"
            break
        fi
    done
    [ -n "$socket" ] && break
    sleep 0.2
done
[ -n "$socket" ] || { cat "$tmp/server.log"; exit 1; }
export HERDR_SOCKET_PATH="$socket"
"$bin" workspace create --cwd "$tmp" > "$tmp/created.json"
pane="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["root_pane"]["pane_id"])' "$tmp/created.json")"
"$bin" pane run "$pane" "printf 'SMOKE_%s\\n' OK"
for _ in $(seq 50); do
    "$bin" pane read "$pane" --source recent > "$tmp/pane.txt" || true
    if grep -q SMOKE_OK "$tmp/pane.txt"; then echo "PTY roundtrip ok"; exit 0; fi
    sleep 0.2
done
cat "$tmp/pane.txt" "$tmp/server.log"
exit 1
