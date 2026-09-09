#!/usr/bin/env bash
# One-shot: re-point the orphaned release tags onto main, and backfill the
# missing GitHub Releases.
#
#   scripts/repair-tags.sh                 dry run — report the plan, change nothing
#   scripts/repair-tags.sh --apply         rewrite the tags locally
#   scripts/repair-tags.sh --apply --push  ...and force-push them + create Releases
#
# ⚠ --push force-updates public tag refs. Read this whole header first.
#
# THE PROBLEM
#
# Every release tag v0.4.0..v1.4.0 and mrz-v0.4.0/0.5.0 sits on a history that
# shares NO COMMON ANCESTOR with main. The repository's history was rewritten
# once and the tags were never moved, so both root commits differ (dc697ee on
# main, 85423b6 on the tag line) while carrying identical subjects and dates.
#
#   $ git merge-base v1.4.0 HEAD ; echo $?
#   1
#   $ git describe --tags
#   fatal: No tags can describe 'f735dc2'.
#
# Consequences: `git log v1.3.0..v1.4.0` is fatal, GitHub's tag compare is
# meaningless, and any tag-driven automation would be built on sand.
#
# THE REPAIR
#
# For 14 of the 15 tags, main contains a commit whose TREE HASH IS IDENTICAL to
# the tagged commit's — the content is the same, only the commit object differs.
# So the mapping is exact rather than approximate: match on tree, not on message
# or date, and a tag lands on a commit whose contents are byte-for-byte what it
# always named.
#
# v1.3.0 is the exception. No commit on main has its exact tree (main's history
# around that point composes differently — the samples-data reconciliation).
# It falls back to the commit that INTRODUCED version = "1.3.0" into
# Cargo.toml on main, which is the honest answer to "where did 1.3.0 begin".
# That one substitution is reported explicitly rather than silently.
#
# WHY FORCE-PUSHING TAGS IS ACCEPTABLE HERE
#
# Only one GitHub Release exists (v1.4.0), and `mrz` consumers resolve from
# crates.io rather than from git tags, so nothing downstream pins these refs.
# The alternative — leaving them — permanently breaks `git describe` and every
# tag-range diff on a repository that is meant to be sold.
#
# This script is kept for provenance, not for reuse. .github/workflows/release.yml
# is what prevents a recurrence: it tags from a version change on main, so a tag
# can only ever land on a commit that is on main. See RELEASING.md.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

apply=0
push=0
for arg in "$@"; do
    case "$arg" in
        --apply) apply=1 ;;
        --push)  push=1 ;;
        -h|--help) sed -n '2,50p' "$0"; exit 0 ;;
        *) echo "unknown argument: $arg" >&2; exit 2 ;;
    esac
done
[ "$push" -eq 1 ] && [ "$apply" -eq 0 ] && { echo "--push requires --apply" >&2; exit 2; }

main_ref=main
git rev-parse --verify --quiet "$main_ref" >/dev/null || { echo "no '$main_ref' branch" >&2; exit 1; }

echo "Building tree -> commit index for $main_ref ..."
declare -A tree_to_commit=()
while read -r sha tree; do
    # First (newest) wins; releases are the last commit with their tree.
    [ -n "${tree_to_commit[$tree]:-}" ] || tree_to_commit["$tree"]="$sha"
done < <(git log --format='%H %T' "$main_ref")
echo "  indexed ${#tree_to_commit[@]} trees"
echo

# The commit on main that first set [workspace.package] version = $1.
introduced_version() {
    local want="$1" sha found=""
    while read -r sha; do
        local v
        v="$(git show "$sha:Cargo.toml" 2>/dev/null | awk '
            /^\[workspace\.package\]/ { f = 1; next }
            f && /^\[/ { exit }
            f && /^version[[:space:]]*=/ { gsub(/[^0-9.]/, "", $0); print; exit }')"
        [ "$v" = "$want" ] && found="$sha"
    done < <(git log --format='%H' "$main_ref" -- Cargo.toml)
    printf '%s' "$found"
}

plan=()
printf '%-14s %-10s %-42s %s\n' TAG METHOD "TARGET ON MAIN" SUBJECT
printf '%.0s-' {1..110}; echo

for tag in $(git tag -l --sort=creatordate | grep -E '^(v|mrz-v)[0-9]'); do
    tree="$(git rev-parse "$tag^{tree}")"
    target="${tree_to_commit[$tree]:-}"
    method=tree

    if [ -z "$target" ]; then
        case "$tag" in
            v*) target="$(introduced_version "${tag#v}")"; method=version ;;
        esac
    fi

    if [ -z "$target" ]; then
        printf '%-14s %-10s %s\n' "$tag" UNRESOLVED "*** no match on main — skipped ***"
        continue
    fi

    # Already correct? (Reachable from main and pointing at the target.)
    current="$(git rev-list -1 "$tag")"
    if [ "$current" = "$target" ]; then
        printf '%-14s %-10s %-42s %s\n' "$tag" ok "$target" "already on main"
        continue
    fi

    printf '%-14s %-10s %-42s %s\n' "$tag" "$method" "$target" \
        "$(git log -1 --format='%cs %s' "$target" | cut -c1-46)"
    plan+=( "$tag=$target" )
done

echo
if [ "${#plan[@]}" -eq 0 ]; then
    echo "Nothing to repair."
    exit 0
fi
echo "${#plan[@]} tag(s) would be re-pointed."

if [ "$apply" -eq 0 ]; then
    echo
    echo "Dry run. Re-run with --apply (and --push to publish) to act."
    exit 0
fi

echo
for entry in "${plan[@]}"; do
    tag="${entry%%=*}"; target="${entry#*=}"
    # Preserve the original annotation, which carries the release's own title.
    msg="$(git tag -l --format='%(contents)' "$tag")"
    [ -z "$msg" ] && msg="$tag"
    git tag -f -a "$tag" -m "$msg" "$target" >/dev/null
    echo "  re-pointed $tag -> $target"
done

if [ "$push" -eq 0 ]; then
    echo
    echo "Local tags rewritten. Not pushed. Verify, then re-run with --push:"
    echo "    git describe --tags $main_ref"
    echo "    git log --oneline v1.3.0..v1.4.0 | head"
    exit 0
fi

echo
echo "Force-pushing ${#plan[@]} tag(s) ..."
for entry in "${plan[@]}"; do
    git push --force origin "refs/tags/${entry%%=*}"
done

if command -v gh >/dev/null 2>&1; then
    echo
    echo "Backfilling GitHub Releases ..."
    for tag in $(git tag -l --sort=creatordate | grep -E '^v[0-9]'); do
        if gh release view "$tag" >/dev/null 2>&1; then
            echo "  $tag: release already exists"
            continue
        fi
        notes="$(awk -v ver="${tag#v}" '
            $0 ~ "^## \\[" ver "\\]" { f = 1; print; next }
            f && /^## / { exit }
            f { print }' CHANGELOG.md)"
        if [ -z "$notes" ]; then
            echo "  $tag: no CHANGELOG section — skipped"
            continue
        fi
        # --latest=false matters: gh marks the most recently *created* release as
        # "Latest" by default, so backfilling history in chronological order ends
        # with an old release badged Latest on the repo front page. Observed on
        # the real run — v1.3.0 took the badge from v1.4.0 and had to be undone
        # with `gh release edit v1.4.0 --latest`.
        printf '%s\n' "$notes" \
            | gh release create "$tag" --title "$tag" --notes-file - --verify-tag --latest=false
        echo "  $tag: release created"
    done
fi

echo
echo "Done. Verify:"
echo "    git describe --tags $main_ref"
echo "    git merge-base v1.4.0 $main_ref"
echo "    gh release list"
