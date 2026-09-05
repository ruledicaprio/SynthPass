#!/usr/bin/env bash
#
# Run the Tier-2 parity harness and produce a log that says which code made it.
#
#   ./scripts/measure-parity.sh                # both arms, one binary
#   ./scripts/measure-parity.sh --arm baseline # one arm
#   ./scripts/measure-parity.sh --arm holdout
#
# Writes artifacts/parity-<arm>-<timestamp>.log, each with a provenance header.
#
# ---------------------------------------------------------------------------
# Why this script exists rather than a cargo invocation in a shell loop
#
# On 2026-09-05 an ad-hoc runner reported two complete arms of parity numbers
# measured against a STALE BINARY. The release build had failed (`LNK1104:
# cannot open parity-<hash>.exe`), the runner picked its executable with
# `ls -t target/release/deps/parity-*.exe | head -1`, and that glob happily
# returned the previous build. Roughly 55 minutes of output looked entirely
# normal; the numbers were real, reproducible, and answered a question nobody
# had asked. It was caught by comparing two sha256 files by hand.
#
# Three defects made that possible, and this script exists to make each one
# impossible:
#
#   1. The runner continued after a non-zero build.   -> set -euo pipefail, and
#                                                        the build's status is
#                                                        checked explicitly.
#   2. It chose a binary by timestamp glob.           -> the path comes from
#                                                        cargo's own JSON
#                                                        `executable` field.
#                                                        The build names its
#                                                        output; we never guess.
#   3. Two cargo invocations could overlap.           -> a lock directory, plus
#                                                        a refusal to start
#                                                        while a parity binary
#                                                        is still running (that
#                                                        lock is what caused the
#                                                        LNK1104 in the first
#                                                        place).
#
# Concurrent cargo against one target/ dir has bitten this project twice:
# LNK1103 "debugging information corrupt" in a cached rlib, and
# STATUS_DLL_INIT_FAILED compiling synthpass-pipeline. Both looked like flakes
# and were contention. One cargo at a time, always.
# ---------------------------------------------------------------------------
set -euo pipefail

cd "$(dirname "$0")/.."
ARM="${2:-both}"
[ "${1:-}" = "--arm" ] || ARM="both"
case "$ARM" in
  baseline|holdout|both) ;;
  *) echo "usage: $0 [--arm baseline|holdout|both]" >&2; exit 2 ;;
esac

LOCK=".parity-measure.lock"
cleanup() { rmdir "$LOCK" 2>/dev/null || true; }
if ! mkdir "$LOCK" 2>/dev/null; then
  echo "another measure-parity run holds $LOCK — one cargo at a time" >&2
  exit 2
fi
trap cleanup EXIT

# A running parity binary holds its own .exe open on Windows, which is what
# makes the next link fail. Refuse rather than produce an unlinkable build.
if command -v tasklist >/dev/null 2>&1 && tasklist 2>/dev/null | grep -qi "parity-"; then
  echo "a parity binary is still running — wait for it, or the next link fails" >&2
  exit 2
fi

mkdir -p artifacts

echo "==> building the parity test binary (nothing else may run cargo now)"
BUILD_JSON="artifacts/parity-build.json"
cargo test -p synthpass-llm --test parity --release --no-run \
  --message-format=json > "$BUILD_JSON"

# cargo names its own output. Never `ls -t`: after a failed build that glob
# returns a *previous* binary and the run silently measures old code.
BIN=$(python -c '
import json,sys
exe=None
for line in open(sys.argv[1], encoding="utf-8"):
    line=line.strip()
    if not line.startswith("{"): continue
    try: msg=json.loads(line)
    except ValueError: continue
    if msg.get("reason")=="compiler-artifact" and msg.get("executable") \
       and msg.get("target",{}).get("name")=="parity":
        exe=msg["executable"]
print(exe or "")
' "$BUILD_JSON")

if [ -z "$BIN" ] || [ ! -x "$BIN" ]; then
  echo "cargo reported no parity executable — refusing to measure" >&2
  exit 1
fi

GIT_SHA=$(git rev-parse --short HEAD)
GIT_DIRTY=$(git status --porcelain --untracked-files=no | wc -l | tr -d ' ')
BIN_SHA=$(sha256sum "$BIN" | cut -c1-16)

run_arm() {
  local arm="$1" log
  log="artifacts/parity-${arm}-$(date '+%Y%m%d-%H%M').log"
  {
    echo "# arm:        $arm"
    echo "# started:    $(date '+%Y-%m-%d %H:%M:%S')"
    echo "# git:        $GIT_SHA ($GIT_DIRTY tracked file(s) modified)"
    echo "# binary:     $BIN"
    echo "# binary sha: $BIN_SHA"
    echo "#"
    echo "# The harness prints its own 'normalizer vocabulary:' fingerprint"
    echo "# below. If that matches an earlier run, the same vocabulary was"
    echo "# compiled in -- whatever this working tree currently says."
    echo
  } > "$log"

  echo "==> $arm arm -> $log"
  if [ "$arm" = "holdout" ]; then
    SYNTHPASS_PARITY_HOLDOUT=1 "$BIN" --ignored --nocapture --test-threads=1 \
      native_llm_field_accuracy_over_sample_set >> "$log" 2>&1
  else
    "$BIN" --ignored --nocapture --test-threads=1 \
      native_llm_field_accuracy_over_sample_set >> "$log" 2>&1
  fi

  grep -E '^normalizer vocabulary:' "$log" || true
  grep -E '^(reviewed|derived|legacy)' "$log" || true
}

# Both arms from ONE binary, deliberately: a rebuild between arms has hidden a
# real regression on this project before.
if [ "$ARM" = "both" ] || [ "$ARM" = "baseline" ]; then run_arm baseline; fi
if [ "$ARM" = "both" ] || [ "$ARM" = "holdout" ]; then run_arm holdout; fi

echo "==> done"
