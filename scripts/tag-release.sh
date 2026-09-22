#!/usr/bin/env bash
# Create a release tag, or prove the one that exists names this exact commit.
#
#   scripts/tag-release.sh --tag mrz-v0.8.0 --sha <40-hex>
#   scripts/tag-release.sh --tag v1.5.1 --sha <40-hex> --verify-only
#
# There are three states a release tag can be in, and the one that matters is
# the one release.yml used to treat as success:
#
#   absent        -> create it, annotated, on the given commit, and push it
#   present here  -> a notice; re-running a release is not an error
#   present THERE -> loud failure
#
# "Present there" means refs/tags/<tag> exists and peels to a different commit
# than the one being released. The old workflow said `$tag already exists;
# nothing to do` and exited 0, which is how mrz-v0.8.0 came to sit on the commit
# that opened the breaking window while the publish job downstream of it kept
# reporting green. A tag that names the wrong commit is the single most
# expensive state this repository can reach -- every `git log <tag>..` and every
# GitHub compare built on it is quietly wrong -- so it fails closed and asks for
# a human.
#
# Repairing one is deliberately not automated: moving a published tag rewrites
# what a consumer already fetched. scripts/repair-tags.sh exists for the one-off,
# and RELEASING.md describes when it applies.
#
# Unlike the other scripts here this one does not cd to the repository root: it
# touches no repo-relative path, only git, and git resolves the repository from
# the working directory on its own. That keeps it usable against any checkout,
# which is what lets tools/test_release_ready.py drive it over throwaway repos.
set -euo pipefail

tag=""
sha=""
message=""
verify_only=0

while [ $# -gt 0 ]; do
    case "$1" in
        --tag)         tag="${2:?--tag needs a value}"; shift 2 ;;
        --sha)         sha="${2:?--sha needs a value}"; shift 2 ;;
        --message)     message="${2:?--message needs a value}"; shift 2 ;;
        --verify-only) verify_only=1; shift ;;
        -h|--help)     sed -n '2,8p' "$0"; exit 0 ;;
        *)             echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

[ -n "$tag" ] || { echo "--tag is required" >&2; exit 2; }
[ -n "$sha" ] || { echo "--sha is required" >&2; exit 2; }
message="${message:-$tag}"

# A short sha, a branch name or a tag would all "work" here and bind the release
# to something that can move. Only a full object name is accepted.
if ! printf '%s' "$sha" | grep -qE '^[0-9a-f]{40}$'; then
    echo "--sha must be a full 40-character commit sha (got '$sha')" >&2
    exit 2
fi
if ! git cat-file -e "${sha}^{commit}" 2>/dev/null; then
    echo "--sha $sha is not a commit in this repository" >&2
    exit 2
fi

if ! git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
    if [ "$verify_only" -eq 1 ]; then
        echo "would create $tag on $sha"
        exit 0
    fi
    git tag -a "$tag" -m "$message" "$sha"
    git push origin "$tag"
    echo "created $tag on $sha"
    exit 0
fi

# Peel it: `git rev-list -n 1` resolves an annotated tag object through to the
# commit, and leaves a lightweight tag alone. Both kinds exist in this repo's
# history, so neither may be assumed.
existing="$(git rev-list -n 1 "$tag")"

if [ "$existing" = "$sha" ]; then
    echo "$tag already exists and names $sha -- nothing to do"
    exit 0
fi

describe() {  # <sha> -- one line, or the bare sha if the object is missing
    git log -1 --format='%h %ad  %s' --date=short "$1" 2>/dev/null || printf '%s\n' "$1"
}

# --left-right counts both directions. A plain `existing..sha` reports 0 when
# the tag is simply *ahead* of the release, which reads as "they are the same
# commit" -- the exact misreading this whole script exists to prevent.
distance="$(git rev-list --left-right --count "$existing...$sha" 2>/dev/null || printf '? ?')"
behind="${distance%%[[:space:]]*}"
ahead="${distance##*[[:space:]]}"

cat >&2 <<EOF
FAIL: $tag already exists, and it does not name the commit being released.

  tag names : $existing
              $(describe "$existing")
  releasing : $sha
              $(describe "$sha")
  distance  : $behind commit(s) only the tag has, $ahead only the release has

Nothing has been tagged, pushed or published. This is never resolved by
re-running: a tag that is already public has been fetched by someone, so moving
it rewrites history they hold. Decide explicitly --

  - release the next version instead, leaving $tag where it is; or
  - repair $tag deliberately (scripts/repair-tags.sh force-pushes tags; read its
    header first) and re-run this release against the repaired tag.

RELEASING.md, "Repairing the historical tags", is the long form.
EOF
exit 1
