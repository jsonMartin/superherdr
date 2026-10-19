#!/bin/sh
# Prepare a release directory for publishing: verify the built archives, stamp
# the installer's default version with the release version, build SHA256SUMS,
# and extract this release's notes from CHANGELOG.md. Used by
# publish-release.yml.
# Usage: release_publish.sh <dist-dir> <version>
set -eu

dist="$1"
version="$2"
# the changelog sits beside the caller, not inside dist
changelog="$(pwd)/CHANGELOG.md"

cd "$dist"

# every archive must match the .sha256 file its build job uploaded
sha256sum -c ./*.tar.gz.sha256

# the committed installer carries a placeholder default; the attached copy must
# install this release
sed "s/^DEFAULT_VERSION=\"[^\"]*\"\$/DEFAULT_VERSION=\"$version\"/" install.sh > install.sh.stamped
mv install.sh.stamped install.sh
grep -qxF "DEFAULT_VERSION=\"$version\"" install.sh

# same files and order as the superherdr-v0.9.5.1 release
sha256sum superherdr-*.tar.gz install.sh > SHA256SUMS

# notes: this release's dated Superherdr section, ending at the next section
# heading or the inherited Herdr notes
section="$(awk -v heading="## [$version] - " '
    index($0, heading) == 1 { on = 1; next }
    on && (/^## / || /^### Inherited from Herdr /) { exit }
    on && !started && /^[[:space:]]*$/ { next }
    on { started = 1; print }
' "$changelog")"
if [ -z "$(printf '%s' "$section" | tr -d '[:space:]')" ]; then
    echo "no notes for $version in $changelog: expected a '## [$version] - <date>' section" >&2
    exit 1
fi
{
    echo "# Superherdr $version"
    echo
    printf '%s\n' "$section"
} > notes.md
