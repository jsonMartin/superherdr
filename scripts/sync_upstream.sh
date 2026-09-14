#!/usr/bin/env bash
# Merge an upstream Herdr release into Superherdr while keeping the fork's
# intentional removals (.upstream-sync/exclude) and rewritten files
# (.upstream-sync/ours).
#
#   scripts/sync_upstream.sh <tag> [--report FILE]  merge vX.Y.Z; commit when clean
#   scripts/sync_upstream.sh --latest               print the newest upstream release tag
#   scripts/sync_upstream.sh --check                fail if an excluded path is tracked
#
# Exit status: 0 merged, already merged, or check passed; 1 error; 2 conflicts
# remain (the merge is left in progress for manual resolution).
#
# Upstream tags are fetched into refs/upstream-tags/, never refs/tags/, so a
# plain `git push` cannot publish them to this repository.
set -euo pipefail

UPSTREAM_URL="${SUPERHERDR_UPSTREAM_URL:-https://github.com/herdrdev/herdr.git}"
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

list_entries() {
    grep -v -e '^[[:space:]]*#' -e '^[[:space:]]*$' "$1"
}

check_exclusions() {
    local found=0 path
    while IFS= read -r path; do
        if [ -n "$(git ls-files -- "$path")" ]; then
            echo "error: excluded upstream path is tracked again: $path" >&2
            found=1
        fi
    done < <(list_entries .upstream-sync/exclude)
    if [ "$found" -ne 0 ]; then
        echo "remove it, or delete its line from .upstream-sync/exclude if it is wanted" >&2
        return 1
    fi
    echo "upstream exclusions: ok"
}

latest_tag() {
    git ls-remote --tags --refs "$UPSTREAM_URL" 'v*' |
        sed 's#.*refs/tags/v##' |
        grep -E '^[0-9]+\.[0-9]+\.[0-9]+$' |
        sort -t. -k1,1n -k2,2n -k3,3n |
        tail -1 | sed 's/^/v/'
}

case "${1:-}" in
    --check)
        check_exclusions
        exit
        ;;
    --latest)
        latest_tag
        exit
        ;;
    v[0-9]*) tag="$1"; shift ;;
    *)
        echo "usage: $0 <vX.Y.Z> [--report FILE] | --latest | --check" >&2
        exit 1
        ;;
esac

report=""
if [ "${1:-}" = "--report" ]; then
    report="${2:?--report needs a file}"
fi
if ! printf '%s' "$tag" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "error: expected a release tag like v0.9.1, got $tag" >&2
    exit 1
fi
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
    echo "error: commit or stash tracked changes before syncing" >&2
    exit 1
fi

upstream_ref="refs/upstream-tags/$tag"
git fetch --quiet --no-tags "$UPSTREAM_URL" "+refs/tags/$tag:$upstream_ref"
upstream_commit="$(git rev-parse "$upstream_ref^{commit}")"

if git merge-base --is-ancestor "$upstream_commit" HEAD; then
    echo "Herdr $tag is already merged"
    exit 0
fi
base="$(git merge-base HEAD "$upstream_commit" || true)"

merge_status=0
git merge --no-ff --no-commit --quiet "$upstream_commit" >/dev/null 2>&1 || merge_status=$?
if ! git rev-parse -q --verify MERGE_HEAD >/dev/null; then
    echo "error: git merge failed before creating a merge (exit $merge_status)" >&2
    exit 1
fi

dropped=()
while IFS= read -r path; do
    if [ -n "$(git ls-files -- "$path")" ]; then
        dropped+=("$path")
        git rm -r -q --force -- "$path" >/dev/null
    fi
done < <(list_entries .upstream-sync/exclude)

ported=()
while IFS= read -r path; do
    if [ -n "$base" ] && ! git diff --quiet "$base" "$upstream_commit" -- "$path"; then
        ported+=("$path")
    fi
    if git cat-file -e "HEAD:$path" 2>/dev/null; then
        git checkout HEAD -- "$path"
        git add -- "$path"
    fi
done < <(list_entries .upstream-sync/ours)

conflicts="$(git diff --name-only --diff-filter=U)"
new_workflows="$(git diff --cached --name-only --diff-filter=A HEAD -- .github/ || true)"

write_report() {
    echo "## Merge Herdr $tag"
    echo
    echo "Upstream release: https://github.com/herdrdev/herdr/releases/tag/$tag"
    echo
    if [ -n "$conflicts" ]; then
        echo "### Conflicts to resolve"
        echo
        echo "$conflicts" | sed 's/^/- `/; s/$/`/'
        echo
    fi
    if [ "${#dropped[@]}" -gt 0 ]; then
        echo "### Excluded paths dropped"
        echo
        printf -- '- `%s`\n' "${dropped[@]}"
        echo
    fi
    if [ "${#ported[@]}" -gt 0 ]; then
        echo "### Upstream changes to Superherdr-owned files (review and port by hand)"
        echo
        printf -- '- `%s`\n' "${ported[@]}"
        echo
        echo "Inspect with \`git diff $base $upstream_commit -- <file>\`."
        echo
    fi
    if [ -n "$new_workflows" ]; then
        echo "### New upstream files under .github (decide whether to keep or exclude)"
        echo
        echo "$new_workflows" | sed 's/^/- `/; s/$/`/'
        echo
    fi
}

if [ -n "$report" ]; then
    write_report > "$report"
fi
write_report

if [ -n "$conflicts" ]; then
    echo "Conflicts remain. Resolve them, run 'git add', then 'git commit'." >&2
    exit 2
fi

check_exclusions
git commit --quiet -m "chore(upstream): merge herdr $tag"
echo "Merged Herdr $tag. Review the report above, run just check, then open a pull request."
