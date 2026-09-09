#!/usr/bin/env bash
# Derive the next version from the pending changelog fragments.
#
#   scripts/next-version.sh                        next workspace version
#   scripts/next-version.sh --scope mrz            next crates/mrz version
#   scripts/next-version.sh --explain              show which fragments drove the bump
#   scripts/next-version.sh --check 1.5.0          exit 1 unless 1.5.0 is what's implied
#   scripts/next-version.sh --current              print the current version, nothing else
#
# The point: a version number should be a *consequence* of what changed, not a
# number someone typed. Every user-visible change already drops a fragment in
# changelog.d/ (CONTRIBUTING.md, "Changelog fragments"). Those fragments carry
# enough information to say which slot must move -- so the release process reads
# the bump off them rather than asking a human to remember the semver rules at
# the exact moment they are least likely to.
#
# Fragment grammar (changelog.d/README.md):
#
#   changelog.d/<id>.<category>.md         workspace-scoped
#   changelog.d/mrz/<id>.<category>.md     crates/mrz, its own published line
#   changelog.d/<id>.<category>!.md        breaking -- "!" is the discriminator
#
# Bump rules, which differ by scope because the two lines have different
# contracts:
#
#   workspace (1.x)  breaking -> major   added -> minor   otherwise -> patch
#   mrz       (0.x)  breaking -> MINOR   added -> patch   otherwise -> patch
#
# The mrz row is not a typo. crates/mrz is pre-1.0 and published, so cargo
# treats the *minor* slot as the breaking slot -- `^0.7` resolves 0.7.x but
# never 0.8.0. CONTRIBUTING.md's "Semver policy" section is the long form, and
# the `semver` CI job (cargo-semver-checks against the crates.io baseline) is
# what independently proves an API change was classified honestly.
#
# Pure bash. Nothing to install.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

scope=workspace
explain=0
current_only=0
check=""
frag_dir_override=""

while [ $# -gt 0 ]; do
    case "$1" in
        --scope)        scope="${2:?--scope needs a value}"; shift 2 ;;
        --explain)      explain=1; shift ;;
        --current)      current_only=1; shift ;;
        --check)        check="${2:?--check needs a version}"; shift 2 ;;
        --fragment-dir) frag_dir_override="${2:?--fragment-dir needs a path}"; shift 2 ;;
        -h|--help)      sed -n '2,30p' "$0"; exit 0 ;;
        *)              echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

case "$scope" in
    workspace)
        manifest="Cargo.toml"
        frag_dir="changelog.d"
        # The workspace version lives under [workspace.package], not [package].
        current="$(awk '/^\[workspace\.package\]/ { f = 1; next }
                        f && /^\[/            { exit }
                        f && /^version[[:space:]]*=/ {
                            gsub(/[^0-9.]/, "", $0); print; exit }' "$manifest")"
        ;;
    mrz)
        manifest="crates/mrz/Cargo.toml"
        frag_dir="changelog.d/mrz"
        current="$(awk '/^\[package\]/ { f = 1; next }
                        f && /^\[/     { exit }
                        f && /^version[[:space:]]*=/ {
                            gsub(/[^0-9.]/, "", $0); print; exit }' "$manifest")"
        ;;
    *)
        echo "unknown scope '$scope' (expected: workspace, mrz)" >&2; exit 2 ;;
esac

[ -n "${frag_dir_override}" ] && frag_dir="$frag_dir_override"

if ! printf '%s' "$current" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "could not parse a version out of $manifest (got '$current')" >&2
    exit 1
fi

if [ "$current_only" -eq 1 ]; then
    printf '%s\n' "$current"
    exit 0
fi

major="${current%%.*}"
rest="${current#*.}"
minor="${rest%%.*}"
patch="${rest##*.}"

# Collect this scope's fragments. A plain glob is deliberate: `*` does not cross
# a `/`, so changelog.d/*.md picks up workspace fragments without ever reaching
# changelog.d/mrz/. The scoping falls out of the filesystem rather than needing
# a filter that could drift from it.
shopt -s nullglob
fragments=( "$frag_dir"/*.md )
breaking=() additive=() other=()

for f in "${fragments[@]}"; do
    base="$(basename "$f")"
    [ "$base" = "README.md" ] && continue
    # <id>.<category>[!].md
    if [[ "$base" =~ \.(added|changed|deprecated|removed|fixed|security)(!)?\.md$ ]]; then
        category="${BASH_REMATCH[1]}"
        bang="${BASH_REMATCH[2]:-}"
        if [ -n "$bang" ]; then
            breaking+=( "$base" )
        elif [ "$category" = "added" ]; then
            additive+=( "$base" )
        else
            other+=( "$base" )
        fi
    fi
done

if [ "${#breaking[@]}" -gt 0 ]; then
    reason="breaking change"
    if [ "$major" -eq 0 ]; then
        next="${major}.$((minor + 1)).0"        # pre-1.0: minor is the breaking slot
        slot="minor (pre-1.0 breaking slot)"
    else
        next="$((major + 1)).0.0"
        slot="major"
    fi
elif [ "${#additive[@]}" -gt 0 ] && [ "$major" -gt 0 ]; then
    reason="new functionality, no breaking change"
    next="${major}.$((minor + 1)).0"
    slot="minor"
elif [ "${#fragments[@]}" -gt 0 ] || [ "${#other[@]}" -gt 0 ] || [ "${#additive[@]}" -gt 0 ]; then
    reason="fixes and changes only"
    next="${major}.${minor}.$((patch + 1))"
    slot="patch"
else
    # No fragments at all. A release still needs *a* number; patch is the honest
    # floor, and the release script refuses an empty release separately.
    reason="no pending fragments"
    next="${major}.${minor}.$((patch + 1))"
    slot="patch"
fi

if [ "$explain" -eq 1 ]; then
    printf 'scope     : %s (%s)\n' "$scope" "$manifest"
    printf 'fragments : %s\n' "$frag_dir"
    printf 'current   : %s\n' "$current"
    printf 'bump      : %s -- %s\n' "$slot" "$reason"
    printf 'next      : %s\n' "$next"
    if [ "${#breaking[@]}" -gt 0 ]; then
        printf '\nbreaking (forces the bump):\n'
        printf '  %s\n' "${breaking[@]}"
    fi
    if [ "${#additive[@]}" -gt 0 ]; then
        printf '\nadditive:\n'
        printf '  %s\n' "${additive[@]}"
    fi
    if [ "${#other[@]}" -gt 0 ]; then
        printf '\nother:\n'
        printf '  %s\n' "${other[@]}"
    fi
    exit 0
fi

if [ -n "$check" ]; then
    if [ "$check" = "$next" ]; then
        printf 'OK: %s is the version the pending %s fragments imply\n' "$check" "$scope"
        exit 0
    fi
    cat >&2 <<EOF
Version mismatch for scope '$scope'.

  proposed : $check
  implied  : $next   (current $current, bump: $slot -- $reason)

The pending fragments in $frag_dir/ imply a different version than the one in
$manifest. Either the bump is wrong, or a fragment is mis-categorised -- a
breaking change must be named with a trailing "!" (e.g. api-rename.changed!.md),
which is what makes it visible to this check.

Run 'scripts/next-version.sh --scope $scope --explain' to see which fragments
drove the decision.
EOF
    exit 1
fi

printf '%s\n' "$next"
