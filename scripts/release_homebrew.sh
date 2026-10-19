#!/bin/sh
# Point the Homebrew formula at one Superherdr release: url, version, sha256,
# and the test block's expected version string. Keeps each line's existing
# indentation and refuses to move the tap backwards. Used by
# publish-release.yml.
# Usage: release_homebrew.sh <formula> <version> <base> <macos-sha256>
set -eu

formula="$1"
version="$2"
base="$3"
hash="$4"

# lowercase owner, matching the live formula, so a same-version re-run is a
# byte-identical no-op instead of a case-only commit
url="https://github.com/jsonmartin/superherdr/releases/download/superherdr-v$version/superherdr-$version-macos-aarch64.tar.gz"

cur="$(sed -n 's/^[[:space:]]*version "\(.*\)"$/\1/p' "$formula")"
if [ -z "$cur" ]; then
    echo "cannot read the formula's current version" >&2
    exit 1
fi
# never downgrade; a matching version rewrites to the same bytes (a no-op)
if [ "$(printf '%s\n' "$cur" "$version" | sort -V | tail -1)" != "$version" ]; then
    echo "formula already newer ($cur); leaving it alone"
    exit 0
fi

sed -E \
    -e "s|^([[:space:]]*url \")[^\"]*(\")|\1$url\2|" \
    -e "s|^([[:space:]]*version \")[^\"]*(\")|\1$version\2|" \
    -e "s|^([[:space:]]*sha256 \")[^\"]*(\")|\1$hash\2|" \
    -e "s|^([[:space:]]*assert_equal \")[^\"]*(\",)|\1herdr $base (superherdr $version)\2|" \
    "$formula" > "$formula.new"
mv "$formula.new" "$formula"

# assert every rewritten line landed, ignoring indentation
grep -qE "^[[:space:]]*url \"$url\"$" "$formula"
grep -qE "^[[:space:]]*version \"$version\"$" "$formula"
grep -qE "^[[:space:]]*sha256 \"$hash\"$" "$formula"
grep -qE "^[[:space:]]*assert_equal \"herdr $base \\(superherdr $version\\)\"," "$formula"
