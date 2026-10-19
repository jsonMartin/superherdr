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
# Cargo.toml, Cargo.lock, SUPERHERDR_REVISION, and the CHANGELOG release-notes
# graft are resolved mechanically; any other conflict stays for a human.
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
merge_output="$(git merge --no-ff --no-commit "$upstream_commit" 2>&1)" || merge_status=$?
if ! git rev-parse -q --verify MERGE_HEAD >/dev/null; then
    echo "error: git merge failed before creating a merge (exit $merge_status):" >&2
    echo "$merge_output" >&2
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
    if ! git cat-file -e "HEAD:$path" 2>/dev/null; then
        echo "warning: .upstream-sync/ours path is not tracked, skipping: $path" >&2
        continue
    fi
    if [ -n "$base" ] && ! git diff --quiet "$base" "$upstream_commit" -- "$path"; then
        ported+=("$path")
    fi
    git checkout HEAD -- "$path"
    git add -- "$path"
done < <(list_entries .upstream-sync/ours)

# Mechanical release bookkeeping: Cargo.toml, Cargo.lock, SUPERHERDR_REVISION,
# and the CHANGELOG graft. Upstream release tags sit on branches, so the merge
# base often carries an older version than either side; the version line is
# normalized to the tag's version in both merge parents before the 3-way merge
# so the name/version difference never conflicts. Only runs where Superherdr's
# Cargo.toml exists, so minimal fixtures and --check are unaffected.
unresolved=()
resolved=()
graft_ran=0
if git cat-file -e HEAD:Cargo.toml 2>/dev/null; then
    old_base="$(git show HEAD:Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -1 || true)"
    new_base="$(git show "$upstream_commit:Cargo.toml" 2>/dev/null | sed -n 's/^version = "\(.*\)"$/\1/p' | head -1 || true)"
    old_rev="$(git show HEAD:src/build_info.rs 2>/dev/null | sed -n 's/^pub const SUPERHERDR_REVISION: u32 = \([0-9]\{1,\}\);$/\1/p' | head -1 || true)"
    new_version="$new_base.1"
    if [ "v$new_base" != "$tag" ]; then
        unresolved+=("upstream Cargo.toml version ${new_base:-<missing>} does not match $tag")
    fi

    set_version() {
        awk -v v="$new_base" '!done && /^version = "/ { print "version = \"" v "\""; done = 1; next } { print }'
    }

    if [ -n "$(git ls-files -u -- Cargo.toml)" ] && [ -n "$base" ] \
        && git cat-file -e "$base:Cargo.toml" 2>/dev/null \
        && git cat-file -e "$upstream_commit:Cargo.toml" 2>/dev/null; then
        tmp="$(mktemp -d)"
        git show "$base:Cargo.toml" | set_version > "$tmp/base"
        git show "HEAD:Cargo.toml" | set_version > "$tmp/ours"
        git show "$upstream_commit:Cargo.toml" > "$tmp/theirs"
        if git merge-file -p "$tmp/ours" "$tmp/base" "$tmp/theirs" > "$tmp/out"; then
            cat "$tmp/out" > Cargo.toml
            git add Cargo.toml
            resolved+=("Cargo.toml: merged with the version line normalized to $new_base")
        fi
        rm -rf "$tmp"
    fi

    lock_unmerged=0
    if [ -n "$(git ls-files -u -- Cargo.lock)" ]; then
        lock_unmerged=1
        # Upstream's lock has no superherdr entry; take theirs and re-lock the
        # workspace members, which rewrites the root package entry.
        git checkout --theirs -- Cargo.lock
    fi
    if [ -z "$(git ls-files -u -- Cargo.toml)" ] \
        && { [ "$new_base" != "$old_base" ] || [ "$lock_unmerged" -eq 1 ]; }; then
        if cargo update --workspace --quiet; then
            git add Cargo.lock
            resolved+=("Cargo.lock: took upstream's and regenerated workspace entries with cargo update --workspace")
        else
            unresolved+=("Cargo.lock: cargo update --workspace failed")
            git checkout -m -- Cargo.lock
        fi
    fi

    if [ "$new_base" != "$old_base" ]; then
        if [ -n "$(git ls-files -u -- src/build_info.rs)" ]; then
            unresolved+=("reset SUPERHERDR_REVISION to 1 after resolving src/build_info.rs")
        elif [ -f src/build_info.rs ]; then
            tmp="$(mktemp)"
            sed 's/^pub const SUPERHERDR_REVISION: u32 = [0-9]\{1,\};$/pub const SUPERHERDR_REVISION: u32 = 1;/' src/build_info.rs > "$tmp"
            cat "$tmp" > src/build_info.rs
            rm -f "$tmp"
            git add src/build_info.rs
            resolved+=("SUPERHERDR_REVISION reset to 1")
        fi

        graft_ran=1
        found_old_base=0
        graft_versions=()
        if git cat-file -e "$upstream_commit:CHANGELOG.md" 2>/dev/null; then
            while IFS= read -r v; do
                if [ "$v" = "$old_base" ]; then
                    found_old_base=1
                    break
                fi
                graft_versions+=("$v")
            done < <({ git show "$upstream_commit:CHANGELOG.md" || true; } | awk '/^## \[/ && !/Unreleased/ { v = $0; sub(/^## \[/, "", v); sub(/\].*/, "", v); print v }')
        fi
        if [ "$found_old_base" -eq 0 ]; then
            unresolved+=("CHANGELOG.md: graft Herdr notes by hand (upstream CHANGELOG lacks $old_base)")
        elif [ "${graft_versions[0]:-}" != "$new_base" ]; then
            unresolved+=("CHANGELOG.md: graft Herdr notes by hand (upstream CHANGELOG lacks $new_base)")
        else
            inherited=""
            grafted_list=""
            for v in "${graft_versions[@]}"; do
                grep -qxF "### Inherited from Herdr $v" CHANGELOG.md && continue
                # git show must survive awk's early exit on the section's end.
                body="$({ git show "$upstream_commit:CHANGELOG.md" || true; } | awk -v want="$v" '
                    BEGIN { on = 0; n = 0 }
                    /^## / {
                        if (on) { exit }
                        v = $0
                        sub(/^## \[/, "", v)
                        sub(/\].*/, "", v)
                        if (v == want) { on = 1 }
                        next
                    }
                    on {
                        if (match($0, /^#+[ \t]+/)) { t = substr($0, RLENGTH + 1); $0 = toupper(substr(t, 1, 1)) tolower(substr(t, 2)) ":" }
                        gsub(/\*\*/, "")
                        while (match($0, /\[[^]]*\]\([^)]*\)/)) {
                            l = substr($0, RSTART, RLENGTH)
                            sub(/\]\(.*/, "", l)
                            $0 = substr($0, 1, RSTART - 1) substr(l, 2) substr($0, RSTART + RLENGTH)
                        }
                        lines[++n] = $0
                    }
                    END {
                        while (n > 0 && lines[n] == "") n--
                        started = 0
                        for (i = 1; i <= n; i++) { if (lines[i] != "" || started) { started = 1; print lines[i] } }
                    }')"
                inherited+="### Inherited from Herdr $v"$'\n\n'"$body"$'\n\n'
                grafted_list="${grafted_list:+$grafted_list, }$v"
            done
            if [ -n "$inherited" ]; then
                tmp="$(mktemp)"
                line="$(grep -n '^### Inherited from Herdr ' CHANGELOG.md | head -1 | cut -d: -f1 || true)"
                if [ -n "$line" ]; then
                    head -n $((line - 1)) CHANGELOG.md > "$tmp"
                    printf '%s' "$inherited" >> "$tmp"
                    tail -n +"$line" CHANGELOG.md >> "$tmp"
                else
                    cat CHANGELOG.md > "$tmp"
                    printf '\n%s' "$inherited" >> "$tmp"
                fi
                cat "$tmp" > CHANGELOG.md
                rm -f "$tmp"
                resolved+=("Herdr release notes grafted from the tagged upstream CHANGELOG: $grafted_list")
            fi

            based="Based on Herdr $new_base (release notes: https://github.com/herdrdev/herdr/releases/tag/$tag)."
            new_heading="## [$new_version] - $(date -u +%Y-%m-%d)"
            tmp="$(mktemp)"
            first="$(grep -n '^## \[' CHANGELOG.md | head -1 | cut -d: -f1 || true)"
            if [ -n "$first" ]; then
                heading_line="$(sed -n "${first}p" CHANGELOG.md)"
                heading_version="$(printf '%s' "$heading_line" | sed 's/^## \[//; s/\].*//')"
                case "$heading_line" in
                    *"Unreleased"*)
                        if [ "$heading_version" != "$old_base.$old_rev" ]; then
                            # The pending section was never released; it becomes
                            # this sync's release, keeping its entries.
                            { printf '%s\n\n%s\n\n' "$new_heading" "$based"; tail -n +$((first + 1)) CHANGELOG.md; } > "$tmp"
                        else
                            { head -n $((first - 1)) CHANGELOG.md; printf '%s\n\n%s\n\n' "$new_heading" "$based"; tail -n +"$first" CHANGELOG.md; } > "$tmp"
                        fi
                        ;;
                    *)
                        { head -n $((first - 1)) CHANGELOG.md; printf '%s\n\n%s\n\n' "$new_heading" "$based"; tail -n +"$first" CHANGELOG.md; } > "$tmp"
                        ;;
                esac
            else
                { cat CHANGELOG.md; printf '\n%s\n\n%s\n\n' "$new_heading" "$based"; } > "$tmp"
            fi
            cat "$tmp" > CHANGELOG.md
            rm -f "$tmp"
            resolved+=("dated Superherdr section $new_heading")
        fi
        git add CHANGELOG.md
    fi
fi

conflicts="$(git diff --name-only --diff-filter=U)"
added="$(git diff --cached --name-only --diff-filter=A HEAD || true)"

write_report() {
    echo "## Merge Herdr $tag"
    echo
    echo "Upstream release: https://github.com/herdrdev/herdr/releases/tag/$tag"
    echo
    if [ -n "$conflicts" ] || [ "${#unresolved[@]}" -gt 0 ]; then
        echo "### Conflicts to resolve"
        echo
        if [ -n "$conflicts" ]; then
            echo "$conflicts" | sed 's/^/- `/; s/$/`/'
            echo
        fi
        if [ "${#unresolved[@]}" -gt 0 ]; then
            printf -- '- %s\n' "${unresolved[@]}"
            echo
        fi
    fi
    if [ "${#resolved[@]}" -gt 0 ]; then
        echo "### Resolved automatically"
        echo
        printf -- '- %s\n' "${resolved[@]}"
        echo
        echo "Superherdr version after merge: $new_version"
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
        for path in "${ported[@]}"; do
            if [ "$path" = "CHANGELOG.md" ] && [ "$graft_ran" -eq 1 ]; then
                echo "- \`CHANGELOG.md\` (inherited sections grafted automatically)"
            else
                printf -- '- `%s`\n' "$path"
            fi
        done
        echo
        echo "Inspect with \`git diff $base $upstream_commit -- <file>\`."
        echo
    fi
    if [ -n "$added" ]; then
        echo "### New upstream files (add unwanted ones to .upstream-sync/exclude)"
        echo
        echo "$added" | sed 's/^/- `/; s/$/`/'
        echo
    fi
}

if [ -n "$report" ]; then
    write_report > "$report"
fi
write_report

if [ -n "$conflicts" ] || [ "${#unresolved[@]}" -gt 0 ]; then
    echo "Conflicts remain. Resolve them, run 'git add', then 'git commit'." >&2
    exit 2
fi

check_exclusions
git commit --quiet -m "chore(upstream): merge herdr $tag"
echo "Merged Herdr $tag. Review the report above, run just check, then open a pull request."
