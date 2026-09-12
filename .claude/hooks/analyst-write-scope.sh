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

# Normalise to a path relative to the repo root when absolute.
cwd=$(printf '%s' "$in" | grep -oE '"cwd":[[:space:]]*"([^"\\]|\\.)*"' | head -1 \
  | sed -E 's/^"cwd":[[:space:]]*"//; s/"$//; s/\\\\/\//g; s#\\/#/#g')
case "$path" in
  "$cwd"/*) [ -n "$cwd" ] && path=${path#"$cwd"/} ;;
esac

case "$path" in
  */Temp/claude/*|*/temp/claude/*) exit 0 ;;
  artifacts/*) exit 0 ;;
  knowledge/benchmarks/real-specimen-mrz-baseline.json) deny "the baseline is produced only by CI (gh workflow run real-specimen-gate.yml -f mode=write-baseline)" ;;
  knowledge/benchmarks/*.md|knowledge/benchmarks/*.jsonl|knowledge/benchmarks/*.json|knowledge/benchmarks/*.txt|knowledge/benchmarks/*/*.md|knowledge/benchmarks/*/*.py|knowledge/benchmarks/*/*.json) exit 0 ;;
  *) deny "$path is outside the analyst's write scope" ;;
esac
