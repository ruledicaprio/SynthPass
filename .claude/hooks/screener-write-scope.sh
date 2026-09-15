#!/usr/bin/env bash
# PreToolUse guard on Write|Edit for the synthpass-screener subagent. A screener judges the
# candidates of one scouting cycle and records its proposals; it does not touch the corpus,
# code, docs, or anything git tracks.
#
# Allowed targets:
#   work/scouting/**          the cycle's packet (.md/.json), the ledger, cycle notes
#   the session scratchpad    (…/Temp/claude/…)
# Everything else is denied — samples/, knowledge/, tools/, crates/ included. A change the
# screener thinks one of those needs goes in its report for the calling session.
# Reads the hook payload from stdin; prints a deny decision and exits 0 when it matches.

in=$(cat)
path=$(printf '%s' "$in" | grep -oE '"file_path":[[:space:]]*"([^"\\]|\\.)*"' | head -1 \
  | sed -E 's/^"file_path":[[:space:]]*"//; s/"$//; s/\\\\/\//g; s#\\/#/#g')

deny() {
  printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"synthpass-screener writes only under work/scouting/ (the cycle packet, the ledger, cycle notes): %s. Put anything else in the report as a recommendation for the calling session."}}' "$1"
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
  work/scouting/*) exit 0 ;;
  *) deny "$path is outside the screener's write scope" ;;
esac
