#!/usr/bin/env bash
# PreToolUse guard for executor subagents (wired in .claude/agents/synthpass-engineer.md).
# An executor edits files and runs build tools; the calling session owns git, GitHub and
# publishing. Read-only git stays allowed. Reads the hook payload from stdin; prints a deny
# decision and exits 0 when it matches, exits 0 silently otherwise (normal permission flow).
#
# Also denied: the repo scripts that run git themselves (run-bench.ps1, sync-samples.ps1,
# repair-tags.sh) — the guard sees only the command line, so a script's own commit/push would
# otherwise pass unseen.
#
# Matching is by command boundary, not substring: an unanchored `git commit` would also deny a
# commit message that merely mentions one. Written with grep/sed because jq is not installed
# on the Windows host. Safe read-only forms are scrubbed to a placeholder first, so
# `git branch --show-current` passes while `git branch -D x` does not.

in=$(cat)
# Scan only the tool_input.command string. The payload also carries the model's free-text
# "description" of the call, and a description such as "check whether branch x exists" must
# not deny a read-only command. Falls back to the whole payload if no command string is found.
cmd=$(printf '%s' "$in" | grep -oE '"command":[[:space:]]*"([^"\\]|\\.)*"' | head -1)
[ -n "$cmd" ] || cmd=$in

# Command boundary inside the JSON-escaped payload: start of the "command" string, an escaped
# newline or quote, a shell separator, a path separator (…\\bin\\git.exe, …/bin/git), or a
# wrapper such as xargs/exec/timeout — optionally followed by VAR=value assignments.
b='("command":"|\\n|\\"|"|'"'"'|;|&&|\|\||\||&|\(|`|\$\(|/|\\\\)[[:space:]]*((xargs|exec|command|env|nohup|time|builtin)[[:space:]]+)?(timeout[[:space:]]+[^[:space:]]+[[:space:]]+)?([A-Za-z_][A-Za-z0-9_]*=[^[:space:]]*[[:space:]]+)*'
# The same boundary without path separators — a script name preceded by `scripts/` must sit
# at a real command start, so `grep foo scripts/run-bench.ps1` is a read, not a run.
sb='("command":"|\\n|\\"|"|'"'"'|;|&&|\|\||\||&|\(|`|\$\()[[:space:]]*'
# Token terminator: whitespace, escaped newline, closing quote, or shell separator.
t='([[:space:]]|\\n|"|;|&|\|)'
# Optional escaped closing quote after a quoted executable name (PowerShell: & "git.exe" commit).
q='(\\")?'

# Name the agent in the message so a denial read out of context still says who was bound.
agent=$(printf '%s' "$in" | grep -oE '"agent_type":[[:space:]]*"[A-Za-z0-9_:-]*"' | head -1 | sed -E 's/.*:[[:space:]]*"//; s/"$//')
[ -n "$agent" ] || agent="this subagent"

deny() {
  printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"%s has no git, GitHub or publish rights (%s). Leave the working tree for the calling session to review and commit; put what you need in the handoff report."}}' "$agent" "$1"
  exit 0
}

# --- git ---------------------------------------------------------------------------------
if printf '%s' "$cmd" | grep -qE "${b}git(\.exe)?${q}[[:space:]]"; then
  scrub=$(printf '%s' "$cmd" | sed -E \
    -e 's/branch[[:space:]]+(--show-current|--list|-a|-r|-vv?|--contains|--merged|--no-merged)/SAFE/g' \
    -e 's/branch([[:space:]]*("|;|&|\||\\n))/SAFE\1/g' \
    -e 's/worktree[[:space:]]+list/SAFE/g' \
    -e 's/remote[[:space:]]+(-v|show|get-url)/SAFE/g' \
    -e 's/remote([[:space:]]*("|;|&|\||\\n))/SAFE\1/g' \
    -e 's/stash[[:space:]]+(list|show)/SAFE/g' \
    -e 's/tag[[:space:]]+(-l|--list|-n)/SAFE/g' \
    -e 's/tag([[:space:]]*("|;|&|\||\\n))/SAFE\1/g' \
    -e 's/config[[:space:]]+(--get|--get-all|--get-regexp|--list|-l)/SAFE/g' \
    -e 's/submodule[[:space:]]+status/SAFE/g' \
    -e 's/reflog[[:space:]]+show/SAFE/g' \
    -e 's/reflog([[:space:]]*("|;|&|\||\\n))/SAFE\1/g')
  if printf '%s' "$scrub" | grep -qE "[[:space:]](commit|push|pull|fetch|checkout|switch|restore|reset|rebase|merge|cherry-pick|revert|am|apply|add|rm|mv|clean|stash|tag|notes|config|remote|submodule|bisect|gc|prune|reflog|update-ref|symbolic-ref|filter-branch|replace|init|clone|worktree|branch)${t}"; then
    deny "git subcommand that mutates history, the index, the working tree or the remote"
  fi
fi

# --- gh ----------------------------------------------------------------------------------
if printf '%s' "$cmd" | grep -qE "${b}gh(\.exe)?${q}[[:space:]]"; then
  scrub=$(printf '%s' "$cmd" | sed -E \
    -e 's/gh(\.exe)?(\\")?[[:space:]]+(pr|issue|run|repo|release|workflow|label)[[:space:]]+(view|list|diff|checks|status)/GHSAFE/g' \
    -e 's/gh(\.exe)?(\\")?[[:space:]]+auth[[:space:]]+status/GHSAFE/g' \
    -e 's/gh(\.exe)?(\\")?[[:space:]]+(--version|version)/GHSAFE/g')
  if printf '%s' "$scrub" | grep -qE "${b}gh(\.exe)?${q}[[:space:]]"; then
    deny "gh is limited to view/list/diff/checks/status"
  fi
fi

# --- repo scripts that run git internally --------------------------------------------------
# The guard reads the command line, not what a script does. These commit, push, fetch or
# create worktrees on their own (run-bench.ps1 -> bench-data, sync-samples.ps1 -> samples-data,
# repair-tags.sh -> tags), so invoking them is a git mutation by another name. Matched only in
# an execution position — `grep foo scripts/run-bench.ps1` stays allowed.
if printf '%s' "$cmd" | grep -qE "${sb}(\\\\\")?(bash[[:space:]]+|(pwsh|powershell)(\.exe)?[[:space:]]+(-[A-Za-z]+([[:space:]]+[A-Za-z]+)?[[:space:]]+)*)?(\\\\\"|')?(\./|\.\\\\\\\\)?(scripts[/\\\\]+)?(run-bench\.ps1|sync-samples\.ps1|repair-tags\.sh)"; then
  deny "run-bench.ps1, sync-samples.ps1 and repair-tags.sh commit, push or create worktrees internally — ask the calling session to run them and hand you the output path"
fi

# --- cargo: outward-facing or machine-changing ------------------------------------------
if printf '%s' "$cmd" | grep -qE "${b}cargo(\.exe)?${q}[[:space:]]+(publish|login|logout|owner|yank|install|uninstall|clean)${t}"; then
  deny "cargo publish/login/owner/yank/install/uninstall/clean are the calling session's"
fi

exit 0
