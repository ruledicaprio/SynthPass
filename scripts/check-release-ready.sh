#!/usr/bin/env bash
# Answer one question: is this commit a *release*, or a staged version bump?
#
#   scripts/check-release-ready.sh                        is the workspace ready?
#   scripts/check-release-ready.sh --scope mrz            is crates/mrz ready?
#   scripts/check-release-ready.sh --scope mrz --version 0.8.0
#                                                         ...and is it that version?
#
# The problem this exists to solve. A version field moving means the release
# *window is open*. It does not mean the release is *ready*, and conflating the
# two published nothing and queued the wrong thing: crates/mrz opened its 0.8.0
# breaking window by bumping the version in the PR that made the first breaking
# change, because CI's `semver` job requires exactly that. release.yml read the
# bump as "release now", tagged mrz-v0.8.0 on that commit, and left a crates.io
# publish waiting on approval -- aimed at a commit that predates every other
# 0.8 change. Twenty-three commits later the tag still names it.
#
# So readiness needs its own signal. It does not need a new flag for a human to
# remember to set, because the evidence is already in the repository:
#
#   1. the manifest version is the version being released
#   2. the CHANGELOG carries an assembled section for that exact version
#   3. [Unreleased] is empty -- the fragments went into the version's section,
#      not somewhere a reader has to go looking for them
#   4. no fragments are left pending for this scope
#
# A staged bump fails 2 and 4 by construction. That is the whole design: the
# same fragments that already derive the version number (next-version.sh) also
# witness whether it has been released, so the gate reads the repository rather
# than trusting a field, or a human to remember a flag.
#
# Deliberately *not* checked here: anything needing cargo, git or the network.
# `cargo package`, `cargo publish --dry-run` and the tag/SHA binding are the
# release workflow's job -- they cannot run offline and cannot be unit tested.
# Everything this script does is repository content, so it stays pure bash and
# tools/test_release_ready.py exercises it against synthetic fixture trees.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

scope=workspace
want_version=""
manifest=""
changelog=""
frag_dir=""

while [ $# -gt 0 ]; do
    case "$1" in
        --scope)        scope="${2:?--scope needs a value}"; shift 2 ;;
        --version)      want_version="${2:?--version needs a value}"; shift 2 ;;
        --manifest)     manifest="${2:?--manifest needs a path}"; shift 2 ;;
        --changelog)    changelog="${2:?--changelog needs a path}"; shift 2 ;;
        --fragment-dir) frag_dir="${2:?--fragment-dir needs a path}"; shift 2 ;;
        -h|--help)      sed -n '2,8p' "$0"; exit 0 ;;
        *)              echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

# Scope defaults, every one of them overridable so the test suite can aim the
# whole gate at a fixture tree. The overrides have no other user.
case "$scope" in
    workspace)
        manifest="${manifest:-Cargo.toml}"
        changelog="${changelog:-CHANGELOG.md}"
        frag_dir="${frag_dir:-changelog.d}"
        section='[workspace.package]'
        ;;
    mrz)
        manifest="${manifest:-crates/mrz/Cargo.toml}"
        changelog="${changelog:-crates/mrz/CHANGELOG.md}"
        frag_dir="${frag_dir:-changelog.d/mrz}"
        section='[package]'
        ;;
    *)
        echo "unknown scope '$scope' (expected: workspace, mrz)" >&2; exit 2 ;;
esac

status=0
fail() { printf '  FAIL: %s\n' "$1" >&2; status=1; }

for required in "$manifest" "$changelog"; do
    [ -f "$required" ] || { echo "no such file: $required" >&2; exit 2; }
done
[ -d "$frag_dir" ] || { echo "no such directory: $frag_dir" >&2; exit 2; }

# Read a '## [x.y.z]' section body, up to the next '## ' heading. The version is
# escaped before it becomes a regex: unescaped, '0.8.0' also matches '0-8-0' and
# -- the one that would actually bite -- the prefix of '0.8.01'.
section_body() {  # <file> <version>
    awk -v ver="$2" '
        BEGIN { gsub(/\./, "\\.", ver) }
        $0 ~ "^## \\[" ver "\\]" { f = 1; next }
        f && /^## / { exit }
        f { print }
    ' "$1"
}

bullets() {  # count '- ' bullets on stdin
    grep -cE '^[[:space:]]*- ' || true
}

# Whether the heading exists at all, which an empty section body cannot tell us
# apart from its absence. Reporting "no section" for a section that is present
# but empty sends a reader to fix the wrong thing.
has_section() {  # <file> <version>
    awk -v ver="$2" '
        BEGIN { gsub(/\./, "\\.", ver) }
        $0 ~ "^## \\[" ver "\\]" { found = 1; exit }
        END { exit(found ? 0 : 1) }
    ' "$1"
}

# ------------------------------------------------------------------ check 1
# The awk mirrors next-version.sh's parser deliberately -- same expression, so
# the two scripts cannot come to disagree about what the manifest says.
echo "==> manifest version ($manifest)"
current="$(awk -v sect="$section" '
    $0 == sect { f = 1; next }
    f && /^\[/ { exit }
    f && /^version[[:space:]]*=/ { gsub(/[^0-9.]/, "", $0); print; exit }
' "$manifest")"

if ! printf '%s' "$current" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "could not parse a version out of $manifest (got '$current')" >&2
    exit 2
fi

version="$current"
if [ -n "$want_version" ] && [ "$want_version" != "$current" ]; then
    fail "asked about $want_version but $manifest declares $current"
    version="$want_version"
else
    echo "    $scope is at $current"
fi

# ------------------------------------------------------------------ check 2
echo "==> assembled changelog section ($changelog)"
body="$(section_body "$changelog" "$version" || true)"
count="$(printf '%s\n' "$body" | bullets)"

if ! has_section "$changelog" "$version"; then
    fail "no '## [$version]' section in $changelog -- the fragments have not been
        assembled. scripts/assemble-changelog.sh --scope $scope --write splices
        them into [Unreleased]; renaming that heading to '## [$version]' is what
        makes this commit a release."
elif [ "$count" -eq 0 ]; then
    fail "'## [$version]' in $changelog has no entries -- an empty section is not a release"
else
    echo "    [$version] section found ($count bullets)"
fi

# ------------------------------------------------------------------ check 3
# An [Unreleased] section still holding bullets means assemble-changelog.sh ran
# and its heading was never renamed. The entries exist; nobody looking this
# version up will find them.
echo "==> [Unreleased] is empty"
if [ "$(section_body "$changelog" "Unreleased" | bullets)" -gt 0 ]; then
    fail "[Unreleased] in $changelog still holds entries -- rename that heading to
        '## [$version]' and open a fresh empty [Unreleased] above it"
else
    echo "    nothing pending under [Unreleased]"
fi

# ------------------------------------------------------------------ check 4
# The same glob next-version.sh uses: '*' does not cross a '/', so
# changelog.d/*.md is workspace-scoped without a filter that could drift from
# the directory layout.
echo "==> fragments consumed ($frag_dir)"
shopt -s nullglob
pending=()
for f in "$frag_dir"/*.md; do
    [ "$(basename "$f")" = "README.md" ] && continue
    pending+=( "$(basename "$f")" )
done

if [ "${#pending[@]}" -gt 0 ]; then
    fail "${#pending[@]} fragment(s) still pending in $frag_dir -- either they belong in
        this release's changelog section, or they belong to the next release and
        this commit is not one:"
    printf '          %s\n' "${pending[@]}" >&2
else
    echo "    none pending"
fi

echo
if [ "$status" -eq 0 ]; then
    echo "READY: $scope $version is assembled and its fragments are consumed."
else
    cat >&2 <<EOF
NOT READY: this commit does not release $scope $version.

Two ordinary situations both land here, and neither is an error:

  - a staged version bump, with the breaking window open and the release still
    to come -- the fragments name changes this version has not yet claimed;
  - an ordinary commit after a release, with fragments accumulating toward the
    next version.

The release workflow treats both the same way: it tags and publishes nothing.
This becomes READY on the commit where the release PR assembles the changelog
and consumes the fragments. RELEASING.md has the sequence.
EOF
fi
exit "$status"
