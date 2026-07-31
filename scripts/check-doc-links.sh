#!/usr/bin/env bash
# Verify documentation links resolve.
#
#   scripts/check-doc-links.sh
#
# Four checks, over git-tracked files only:
#
#   1. Every relative Markdown link `[text](path)` resolves to something that
#      exists.
#   2. Every `knowledge/...` path cited in prose — including `.rs` doc comments,
#      Cargo.toml comments and workflow files — exists.
#   3. No `docs/...` references survive, outside a written allowlist.
#   4. Every `#fragment` on a Markdown link resolves to an actual `##`-`####`
#      heading in the target file (GitHub's heading-slug algorithm), not just
#      the file itself. Check 1 strips the fragment before checking existence,
#      so a link to a real file with a stale or invented anchor previously
#      passed silently.
#
# Check 3 is the one that matters most: the `docs/` -> `knowledge/` rename left
# ~116 references behind, and nothing but this script would notice them rotting
# again. See knowledge/decisions/ADR-0001-knowledge-tree.md.
#
# Fenced code blocks in Markdown are excluded from checks 2 and 3: the design
# notes contain proposed directory trees and shell snippets, which are
# illustrations rather than citations. A path a reader is meant to follow does
# not live inside a ``` fence.
#
# Pure bash + git. Nothing to install.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

self="scripts/check-doc-links.sh"

# Paths that are cited but deliberately do not exist. Each needs a reason.
#
#   docs/V2-DESIGN.md, knowledge/V2-DESIGN.md
#     Cited by section number in 11 source doc comments but never committed to
#     this repository (verified: `git log --all -- docs/V2-DESIGN.md` is empty).
#     Deliberately NOT rewritten to `knowledge/` during the rename — moving a
#     broken link is not fixing it. The `knowledge/` spelling appears only in
#     prose explaining that choice. Tracked in knowledge/technical_debt.md.
#
#   docs/mlis_v2_0_0_preliminary_design.md, docs/REBRAND_MIGRATION.md,
#   docs/rebranding_identra_synthpass.md
#     Removed on purpose once the rebrand completed. Referenced only by
#     historical CHANGELOG.md prose describing them as removed, where the old
#     `docs/` path is the historically accurate one.
#
#   docs/
#     The bare directory name, in the design notes that argue for `knowledge/`
#     over `docs/`. Rewriting those would destroy the argument being made.
#
#   knowledge/architecture/*, knowledge/benchmarking/methodology.md
#     Aspirational paths in KNOWLEDGE.md's original proposal, which was adopted
#     with variations: `architecture/` was folded into ARCHITECTURE.md plus
#     `decisions/`, and `benchmarking/` shipped as `benchmarks/`. The doc is
#     kept in the author's voice with an editor's note pointing at ADR-0001;
#     rewriting the paths would misrepresent what was proposed.
allow_missing=(
  "docs/"
  "knowledge/architecture/vision-provider-interface.md"
  "knowledge/architecture/document-fusion.md"
  "knowledge/benchmarking/methodology.md"
  "docs/V2-DESIGN.md"
  "knowledge/V2-DESIGN.md"
  "docs/mlis_v2_0_0_preliminary_design.md"
  "docs/REBRAND_MIGRATION.md"
  "docs/rebranding_identra_synthpass.md"
)

is_allowed() {
  local needle="$1" a
  for a in "${allow_missing[@]}"; do [ "$needle" = "$a" ] && return 0; done
  return 1
}

# Emit a file with fenced code blocks removed (Markdown only).
prose() {
  case "$1" in
    *.md) awk '/^[[:space:]]*```/ { fence = !fence; next } !fence' "$1" ;;
    *)    cat "$1" ;;
  esac
}

fail=0
report() { printf '  %s\n' "$1"; fail=1; }

# GitHub's heading-slug algorithm, applied to every `##`-`####` heading in a
# file: lowercase, trim the ends, drop anything that isn't a
# letter/digit/space/hyphen/underscore, then turn each remaining space into a
# hyphen INDIVIDUALLY — runs are not collapsed, so a removed character
# flanked by spaces (e.g. "Foo — Bar") yields a double hyphen ("foo--bar"),
# matching what GitHub actually renders. Repeated slugs within one file get
# -1, -2, ... suffixes. One `awk` process per file (fence-toggling, heading
# match, and slug bookkeeping all done in-process) rather than one shell
# subprocess per heading line — the latter was slow enough on Windows/MSYS
# process-spawn overhead to time out across a corpus this size.
heading_slugs() {
  awk '
    /^[[:space:]]*```/ { fence = !fence; next }
    fence { next }
    /^##[[:space:]]/ || /^###[[:space:]]/ || /^####[[:space:]]/ {
      line = $0
      sub(/^#+[[:space:]]+/, "", line)
      gsub(/^[ \t]+|[ \t]+$/, "", line)
      line = tolower(line)
      gsub(/[^a-z0-9 _-]/, "", line)
      gsub(/ /, "-", line)
      if (line in seen) { seen[line]++; print line "-" seen[line] }
      else { seen[line] = 0; print line }
    }
  ' "$1"
}

# Only files that actually mention a path are worth opening: `git grep -l` does
# the filtering in one process instead of 300 shell iterations.
mapfile -t md_files    < <(git ls-files '*.md')
mapfile -t path_files  < <(git grep -lE '(docs|knowledge)/' -- . 2>/dev/null || true)

# ------------------------------------------------ 1. relative Markdown links
echo "==> relative Markdown links"
for f in "${md_files[@]}"; do
  dir="$(dirname "$f")"
  # `changelog.d/` fragments are spliced verbatim into CHANGELOG.md at the repo
  # root by scripts/assemble-changelog.sh, so their links are written
  # root-relative and must be resolved from there, not from changelog.d/.
  case "$f" in changelog.d/*) dir="." ;; esac
  while IFS= read -r target; do
    [ -z "$target" ] && continue
    case "$target" in http://*|https://*|mailto:*|\#*) continue ;; esac
    target="${target%%#*}"
    target="${target%% *}"
    [ -z "$target" ] && continue
    resolved="$(realpath -m --relative-to="$repo_root" "$dir/$target" 2>/dev/null || printf '%s' "$target")"
    [ -e "$repo_root/$resolved" ] && continue
    is_allowed "$resolved" || report "$f -> $target (resolved: $resolved)"
  done < <(grep -oE '\]\([^)]+\)' "$f" 2>/dev/null | sed -e 's/^](//' -e 's/)$//')
done

# ------------------------------------------------ 2. knowledge/ paths in prose
echo "==> knowledge/ paths cited in prose"
for f in "${path_files[@]}"; do
  [ "$f" = "$self" ] && continue
  while IFS= read -r p; do
    [ -z "$p" ] && continue
    p="${p%.}"
    [ -e "$repo_root/$p" ] && continue
    is_allowed "$p" || report "$f cites missing $p"
  done < <(prose "$f" | grep -ohE 'knowledge/[A-Za-z0-9_./-]+' | sort -u)
done

# ------------------------------------------------ 3. stale docs/ references
echo "==> stale docs/ references"
for f in "${path_files[@]}"; do
  [ "$f" = "$self" ] && continue
  while IFS= read -r p; do
    [ -z "$p" ] && continue
    p="${p%.}"
    is_allowed "$p" && continue
    report "$f still references $p (the tree moved to knowledge/ — see ADR-0001)"
  done < <(prose "$f" | grep -ohE 'docs/[A-Za-z0-9_./-]*' | sort -u)
done

# ------------------------------------------------ 4. markdown anchor fragments
echo "==> markdown anchor fragments"
# Cache each target file's heading slugs (newline-joined) so a file linked
# from many places — e.g. ARCHITECTURE.md — only gets its headings parsed
# once, not once per inbound link.
declare -A slug_cache=()
for f in "${md_files[@]}"; do
  dir="$(dirname "$f")"
  case "$f" in changelog.d/*) dir="." ;; esac
  while IFS= read -r target; do
    [ -z "$target" ] && continue
    case "$target" in http://*|https://*|mailto:*) continue ;; esac
    case "$target" in *"#"*) ;; *) continue ;; esac
    path="${target%%#*}"
    frag="${target#*#}"
    frag="${frag%% *}"
    [ -z "$frag" ] && continue
    if [ -z "$path" ]; then
      target_file="$repo_root/$f"
    else
      path="${path%% *}"
      resolved="$(realpath -m --relative-to="$repo_root" "$dir/$path" 2>/dev/null || printf '%s' "$path")"
      target_file="$repo_root/$resolved"
    fi
    [ -f "$target_file" ] || continue # check 1 already reports a missing file
    case "$target_file" in *.md) ;; *) continue ;; esac
    if [ -z "${slug_cache[$target_file]+x}" ]; then
      slug_cache[$target_file]="$(heading_slugs "$target_file")"
    fi
    grep -qxF -- "$frag" <<<"${slug_cache[$target_file]}" ||
      report "$f -> $target (no ## heading slugs to #$frag in $target_file)"
  done < <(grep -oE '\]\([^)]+\)' "$f" 2>/dev/null | sed -e 's/^](//' -e 's/)$//')
done

if [ "$fail" -ne 0 ]; then
  echo
  echo "doc-link check FAILED. Fix the paths above, or — if a citation is" >&2
  echo "deliberately dangling — add it to allow_missing[] in this script" >&2
  echo "with a written reason." >&2
  exit 1
fi

echo "all documentation links resolve."
