#!/usr/bin/env bash
# PreToolUse guard on Write|Edit for the synthpass-analyst subagent. An analyst measures and
# writes findings; it does not change code, decisions, corpus files, or CI-owned numbers.
#
# Allowed targets:
#   knowledge/benchmarks/**            dated writeups and README.md (the live-numbers home)
#   artifacts/**                       gitignored local bench outputs
#   the session scratchpad             (…/Temp/claude/…)
# Denied even inside the allowed tree:
#   knowledge/benchmarks/real-specimen-mrz-baseline.json — written only by CI
#   anything under knowledge/benchmarks/ that is not .md/.jsonl/.json/.txt/.py
# Reads the hook payload from stdin; prints a deny decision and exits 0 when it matches.

in=$(cat)
path=$(printf '%s' "$in" | grep -oE '"file_path":[[:space:]]*"([^"\\]|\\.)*"' | head -1 \
  | sed -E 's/^"file_path":[[:space:]]*"//; s/"$//; s/\\\\/\//g; s#\\/#/#g')

deny() {
  printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"synthpass-analyst writes only new or updated files under knowledge/benchmarks/ (never the CI-written baseline JSON) and artifacts/: %s. Put a code, ADR, corpus or baseline change in the report as a recommendation for the calling session."}}' "$1"
  exit 0
}

[ -n "$path" ] || deny "could not read file_path from the tool input"

# Matching is on a repo-relative SUFFIX, not on `cwd`-stripping, for the reason recorded in
# docpoet-write-scope.sh: a linked-worktree session reports `cwd` as the MAIN checkout while the
# target lives under D:/Projects/worktrees/..., so a cwd-relative rule denied every allowlisted
# path in a worktree (it did, on 2026-09-24, for a whole knowledge tree). Deny rules come first,
# also by suffix, so they hold wherever the session is rooted.

# 1. Scratchpad is always fine.
case "$path" in
  */Temp/claude/*|*/temp/claude/*) exit 0 ;;
esac

# 2. Deny rules first.
case "$path" in
  *knowledge/benchmarks/real-specimen-mrz-baseline.json)
    deny "the baseline is produced only by CI (gh workflow run real-specimen-gate.yml -f mode=write-baseline)" ;;
esac

# 3. Allow rules, by suffix.
case "$path" in
  artifacts/*|*/artifacts/*) exit 0 ;;
  knowledge/benchmarks/*.md|*/knowledge/benchmarks/*.md) exit 0 ;;
  knowledge/benchmarks/*.jsonl|*/knowledge/benchmarks/*.jsonl) exit 0 ;;
  knowledge/benchmarks/*.json|*/knowledge/benchmarks/*.json) exit 0 ;;
  knowledge/benchmarks/*.txt|*/knowledge/benchmarks/*.txt) exit 0 ;;
  knowledge/benchmarks/*/*.py|*/knowledge/benchmarks/*/*.py) exit 0 ;;
  *) deny "$path is outside the analyst's write scope" ;;
esac
