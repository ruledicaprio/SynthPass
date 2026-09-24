#!/usr/bin/env bash
# Enforce the changelog-fragment contract.
#
#   scripts/check-changelog.sh                 validate every pending fragment
#   scripts/check-changelog.sh --base origin/main
#                                              ...and apply the PR-scoped rules
#
# Four checks. The first runs anywhere; the rest need a base ref to diff against
# and are what CI runs on a pull request.
#
#   1. Every fragment is named <id>.<category>[!].md with a real category, and
#      its body starts with a "- " bullet. (A fragment that isn't a bullet gets
#      spliced into CHANGELOG.md as a dangling paragraph under someone else's
#      heading -- which has already happened once and nothing noticed.)
#   2. A PR touching crates/** adds at least one fragment. Escape hatch: the
#      `skip-changelog` label, for genuinely invisible internal changes.
#   3. A PR touching crates/mrz/** adds an mrz-scoped fragment, because mrz has
#      its own published version line and its own CHANGELOG. A release PR, which
#      assembles those fragments into crates/mrz/CHANGELOG.md instead, satisfies
#      rules 2 and 3 through that CHANGELOG edit.
#   4. If a PR moves a version in Cargo.toml or crates/mrz/Cargo.toml, that
#      version must equal what the pending fragments imply. This is the
#      discriminator: the number and the diff have to agree, checked rather
#      than trusted.
#
# Check 4 is the one worth being careful about. It does not verify the change is
# *correctly categorised* -- only that the version follows from how it was
# categorised. cargo-semver-checks (the `semver` CI job) is the independent
# check on the categorisation itself for crates/mrz, which is the crate where
# getting it wrong reaches other people.
#
# Pure bash + git. Nothing to install.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

base=""
labels="${PR_LABELS:-}"

while [ $# -gt 0 ]; do
    case "$1" in
        --base)   base="${2:?--base needs a ref}"; shift 2 ;;
        --labels) labels="${2:-}"; shift 2 ;;
        -h|--help) sed -n '2,30p' "$0"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

status=0
fail() { printf '  FAIL: %s\n' "$1" >&2; status=1; }

categories='added|changed|deprecated|removed|fixed|security'

# ------------------------------------------------------------------ check 1
echo "==> fragment grammar and body"
shopt -s nullglob
for f in changelog.d/*.md changelog.d/mrz/*.md; do
    base_name="$(basename "$f")"
    [ "$base_name" = "README.md" ] && continue

    if ! [[ "$base_name" =~ ^[A-Za-z0-9._-]+\.($categories)(!)?\.md$ ]]; then
        fail "$f: name must be <id>.<category>[!].md, category one of ${categories//|/, }"
        continue
    fi

    # First non-blank line must open a Markdown bullet.
    first="$(grep -vE '^[[:space:]]*$' "$f" | head -1 || true)"
    if [ -z "$first" ]; then
        fail "$f: empty fragment"
    elif ! printf '%s' "$first" | grep -qE '^- '; then
        fail "$f: body must start with a '- ' bullet (got: ${first:0:60})"
    fi
done
[ "$status" -eq 0 ] && echo "    all fragments well-formed"

# The remaining checks need something to diff against. Exiting silently here
# reported green for three checks that never ran: a bare local run -- which is
# what CONTRIBUTING and CLAUDE.md tell you to do before a PR -- validated only
# fragment grammar, and a PR that CI then failed on the missing-fragment rule
# passed locally. A local pre-PR run means "compare me against main", so say so
# and do it; when there is genuinely nothing to diff, skip loudly.
if [ -z "$base" ]; then
    if git rev-parse --verify --quiet origin/main >/dev/null; then
        base=origin/main
        echo "    (no --base given; diffing against origin/main)"
    else
        echo "  no --base and no origin/main: PR-scoped checks SKIPPED" >&2
        echo "  this run validated fragment grammar ONLY" >&2
        exit "$status"
    fi
fi

if ! git rev-parse --verify --quiet "$base" >/dev/null; then
    echo "  base ref '$base' not found; skipping PR-scoped checks" >&2
    exit "$status"
fi

# The checks below diff commits ($base...HEAD), not the working tree. A local
# pre-PR run with an uncommitted or untracked change under crates/ or
# changelog.d/ -- exactly what CONTRIBUTING.md and CLAUDE.md tell you to run
# before opening a PR -- diffs a tree that does not yet contain that change,
# so the checks below cannot see it. Reported once: a bare local run this way
# printed "skipped: crates/ changes are documentation only" for an uncommitted,
# non-Markdown crates/ edit, and that false green got quoted in a PR body that
# CI then failed. Warn loudly rather than fail -- running mid-work is normal,
# and it is the loudness, not a refusal, that stops the false green from being
# trusted -- but make it impossible to miss.
dirty="$(git status --porcelain -- crates changelog.d)"
if [ -n "$dirty" ]; then
    dirty_count="$(printf '%s\n' "$dirty" | grep -c .)"
    echo >&2
    echo "  WARNING: $dirty_count uncommitted/untracked change(s) under crates/ or changelog.d/:" >&2
    printf '%s\n' "$dirty" | sed 's/^/    /' >&2
    echo "  the checks below compare $base...HEAD -- committed history -- so they" >&2
    echo "  do NOT see these changes. commit first, then re-run, before quoting this" >&2
    echo "  output as evidence in a PR." >&2
    echo >&2
fi

changed_files="$(git diff --name-only "$base"...HEAD)"
added_files="$(git diff --name-only --diff-filter=A "$base"...HEAD)"

has() { printf '%s\n' "$changed_files" | grep -qE "$1"; }
# An mrz release PR assembles the pending mrz fragments into crates/mrz/CHANGELOG.md and deletes
# them in the same change, so it adds no fragment -- the CHANGELOG edit is its entry. Without
# this, the release flow RELEASING.md documents for mrz could only pass under `skip-changelog`,
# whose meaning ("nobody downstream is affected") is the opposite of a release.
mrz_changelog_edited() { printf '%s\n' "$changed_files" | grep -qx 'crates/mrz/CHANGELOG\.md'; }
# The README.md files documenting changelog.d/ and changelog.d/mrz/ match the
# same globs the fragments do. Counting one as a fragment let a PR that added
# only a directory README satisfy the "you owe a fragment" rule — observed in
# CI on this very PR, which reported "ok: mrz fragment present" for a README.
added_matching() {
    printf '%s\n' "$added_files" | grep -E "$1" | grep -v '/README\.md$' || true
}

# ------------------------------------------------------------------ check 2
echo "==> a code change carries a fragment"
if printf '%s' "$labels" | grep -q 'skip-changelog'; then
    echo "    skipped: 'skip-changelog' label present"
elif ! has '^crates/'; then
    # No crates/ path in the diff at all. Checked first and separately from the
    # documentation-only case below: both an empty diff and an all-Markdown diff
    # make `grep -E '^crates/' | grep -qvE '\.md$'` fail the same way, and
    # collapsing them here once reported "documentation only" for a diff that
    # had nothing in it -- a specific, wrong reason for a change that was never
    # committed.
    echo "    skipped: no crates/ changes"
elif ! printf '%s\n' "$changed_files" | grep -E '^crates/' | grep -qvE '\.md$'; then
    # Only Markdown under crates/ changed — a crate README or CHANGELOG. That
    # cannot alter behaviour, so it owes no entry. Mirrors the same exclusion in
    # real-specimen-gate.yml's path filter.
    echo "    skipped: crates/ changes are documentation only"
elif [ -n "$(added_matching '^changelog\.d/.*\.md$')" ]; then
    echo "    ok: $(added_matching '^changelog\.d/.*\.md$' | wc -l) fragment(s) added"
elif mrz_changelog_edited; then
    echo "    ok: crates/mrz/CHANGELOG.md edited directly (an mrz release PR)"
else
    fail "this PR touches crates/ but adds no changelog.d/ fragment."
    fail "add one (see changelog.d/README.md), or label the PR 'skip-changelog'"
fi

# ------------------------------------------------------------------ check 3
echo "==> an mrz change carries an mrz-scoped fragment"
if printf '%s' "$labels" | grep -q 'skip-changelog'; then
    echo "    skipped: 'skip-changelog' label present"
elif ! has '^crates/mrz/'; then
    # No crates/mrz/ path in the diff at all -- checked and reported separately
    # from the test-only/documentation case below, for the same reason as check 2:
    # an empty diff and a tests-or-docs-only diff both make the exclusion grep
    # below fail the same way, and collapsing them mis-reports an empty diff as
    # "mrz changes are test-only or documentation" when there were no mrz changes
    # to describe.
    echo "    skipped: no crates/mrz/ changes"
elif ! printf '%s\n' "$changed_files" | grep -E '^crates/mrz/' \
     | grep -qvE '^crates/mrz/(tests|fuzz)/|\.md$'; then
    # Test-only and CI-only mrz changes ship no release, per CONTRIBUTING.md's
    # semver policy, so they do not owe a fragment either.
    echo "    skipped: mrz changes are test-only or documentation"
elif [ -n "$(added_matching '^changelog\.d/mrz/.*\.md$')" ]; then
    echo "    ok: mrz fragment present"
elif mrz_changelog_edited; then
    echo "    ok: crates/mrz/CHANGELOG.md edited directly (an mrz release PR)"
else
    fail "this PR touches crates/mrz/ but adds no changelog.d/mrz/ fragment."
    fail "mrz has its own published version line -- its entries do not go in the workspace set"
fi

# ------------------------------------------------------------------ check 4
echo "==> a version bump matches what the fragments imply"
version_of() { # <ref> <manifest> <section>
    git show "$1:$2" 2>/dev/null | awk -v sect="$3" '
        $0 == sect            { f = 1; next }
        f && /^\[/            { exit }
        f && /^version[[:space:]]*=/ { gsub(/[^0-9.]/, "", $0); print; exit }'
}

check_scope() { # <scope> <manifest> <section>
    local scope="$1" manifest="$2" section="$3" before after
    before="$(version_of "$base" "$manifest" "$section" || true)"
    after="$(version_of HEAD "$manifest" "$section" || true)"

    if [ -z "$after" ]; then
        fail "could not read the $scope version from $manifest at HEAD"
        return
    fi
    if [ "$before" = "$after" ]; then
        echo "    $scope: unchanged at $after"
        return
    fi

    # Which fragments justify the bump? The UNION of two sets, because the repo
    # has two legitimate flows and each populates a different side of the diff:
    #
    #   * A release PR runs assemble-changelog.sh, which DELETES the fragments in
    #     the same commit that moves the version. Its evidence is at `base`.
    #   * An mrz API change must "bump the version in that same PR to go green"
    #     (CONTRIBUTING.md, Semver policy), so it ADDS its fragment alongside the
    #     bump. Its evidence is at HEAD.
    #
    # Reading either side alone silently mis-derives the other flow -- taking only
    # `base` rated a breaking mrz change as a patch, which is the exact mistake
    # this check exists to prevent.
    local tmp implied
    tmp="$(mktemp -d)"
    mkdir -p "$tmp/f"
    local path ref
    for ref in "$base" HEAD; do
        while IFS= read -r path; do
            [ -z "$path" ] && continue
            case "$path" in */README.md) continue ;; esac
            # mrz fragments live one level deeper; keep the two scopes from bleeding.
            if [ "$scope" = "mrz" ]; then
                case "$path" in changelog.d/mrz/*.md) ;; *) continue ;; esac
            else
                case "$path" in changelog.d/mrz/*) continue ;; changelog.d/*.md) ;; *) continue ;; esac
            fi
            git show "$ref:$path" > "$tmp/f/$(basename "$path")" 2>/dev/null || true
        done <<< "$(git ls-tree -r --name-only "$ref" -- changelog.d/)"
    done

    # Computed against the last *released* version at the base commit (the topmost
    # `## [X.Y.Z]` in the scope's CHANGELOG), falling back to `$before`. Not the working tree:
    # at HEAD the manifest is already bumped. And not `$before` alone: inside an open breaking
    # window `$before` is already the staged number, so a second bump would pass (0.8.0 ->
    # 0.9.0 with 0.7.1 still the last release). next-version.sh uses the same base.
    local log released
    if [ "$scope" = "mrz" ]; then log="crates/mrz/CHANGELOG.md"; else log="CHANGELOG.md"; fi
    released="$(git show "$base:$log" 2>/dev/null | awk 'match($0, /^## \[[0-9]+\.[0-9]+\.[0-9]+\]/) {
                    v = substr($0, RSTART + 4, RLENGTH - 5); print v; exit }' || true)"
    implied="$(compute_implied "$scope" "${released:-$before}" "$tmp/f")"
    rm -rf "$tmp"

    if [ "$after" = "$implied" ]; then
        echo "    $scope: $before -> $after matches the fragments"
    else
        fail "$scope version $before -> $after, but the fragments imply $implied"
        fail "  run: scripts/next-version.sh --scope $scope --explain"
    fi
}

# Compute the implied next version for an arbitrary base version + fragment dir,
# mirroring next-version.sh's rules. Kept here rather than shelling out because
# next-version.sh reads the version from the working tree, which is the wrong
# side of the diff on a release PR.
compute_implied() { # <scope> <current> <fragment-dir>
    local scope="$1" cur="$2" dir="$3"
    local maj="${cur%%.*}" rest="${cur#*.}"
    local min="${rest%%.*}" pat="${rest##*.}"
    local breaking=0 additive=0
    shopt -s nullglob
    local f b
    for f in "$dir"/*.md; do
        b="$(basename "$f")"
        [ "$b" = "README.md" ] && continue
        if [[ "$b" =~ \.($categories)(!)\.md$ ]]; then breaking=1
        elif [[ "$b" =~ \.added\.md$ ]]; then additive=1
        fi
    done
    if [ "$breaking" -eq 1 ]; then
        if [ "$maj" -eq 0 ]; then printf '%s.%s.0' "$maj" "$((min + 1))"
        else printf '%s.0.0' "$((maj + 1))"; fi
    elif [ "$additive" -eq 1 ] && [ "$maj" -gt 0 ]; then
        printf '%s.%s.0' "$maj" "$((min + 1))"
    else
        printf '%s.%s.%s' "$maj" "$min" "$((pat + 1))"
    fi
}

check_scope workspace Cargo.toml '[workspace.package]'
check_scope mrz crates/mrz/Cargo.toml '[package]'

if [ "$status" -ne 0 ]; then
    echo >&2
    echo "changelog contract check FAILED -- see changelog.d/README.md and RELEASING.md" >&2
fi
exit "$status"
