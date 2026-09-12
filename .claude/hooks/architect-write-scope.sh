#!/usr/bin/env bash
# PreToolUse guard on Write|Edit for the synthpass-architect subagent. The architect judges
# plans and diffs against the tracked documents and maintains the local strategic compass; it
# does not write ADRs, roadmap changes, code, or benchmark numbers itself — those go into its
# report as drafts for the user.
#
# Allowed targets:
#   intent/intent.md                   the only file it edits (local, gitignored, AI-maintained)
#   the session scratchpad             (…/Temp/claude/…)
# Everything else is denied, tracked or not.
# Reads the hook payload from stdin; prints a deny decision and exits 0 when it matches.

in=$(cat)
path=$(printf '%s' "$in" | grep -oE '"file_path":[[:space:]]*"([^"\\]|\\.)*"' | head -1 \
  | sed -E 's/^"file_path":[[:space:]]*"//; s/"$//; s/\\\\/\//g; s#\\/#/#g')

deny() {
  printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"synthpass-architect edits only intent/intent.md: %s. Put an ADR, roadmap, code or doc change in the report as a draft for the user to apply."}}' "$1"
  exit 0
}

[ -n "$path" ] || deny "could not read file_path from the tool input"

cwd=$(printf '%s' "$in" | grep -oE '"cwd":[[:space:]]*"([^"\\]|\\.)*"' | head -1 \
  | sed -E 's/^"cwd":[[:space:]]*"//; s/"$//; s/\\\\/\//g; s#\\/#/#g')
case "$path" in
  "$cwd"/*) [ -n "$cwd" ] && path=${path#"$cwd"/} ;;
esac

case "$path" in
  */Temp/claude/*|*/temp/claude/*) exit 0 ;;
  intent/intent.md) exit 0 ;;
  *) deny "$path is outside the architect's write scope" ;;
esac
