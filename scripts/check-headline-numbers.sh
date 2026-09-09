#!/usr/bin/env bash
# Verify the README's headline accuracy figure matches the committed CI baseline.
#
#   scripts/check-headline-numbers.sh
#
# Why this exists: the same accuracy numbers used to be restated in four documents
# (README.md, ROADMAP.md, benchmarks/README.md, MRZ_SEQUENCE_COMPLETENESS.md) with
# nothing keeping them in step. README.md spent the whole v1.4.0 cycle advertising
# ~42-46% when the measured rate was 52.0%, and naming `checksum_failed` as the
# dominant miss when `no_mrz_found` had overtaken it 3.5:1. A reader who trusted
# the front page was told the wrong number *and* pointed at the wrong bottleneck.
#
# The rule that replaced it: knowledge/benchmarks/README.md is the only document
# carrying live numbers, and README.md carries exactly one headline figure. This
# script is what makes that rule load-bearing rather than aspirational -- it reads
# the CI-written baseline and fails if README.md disagrees.
#
# The baseline itself is written only by CI:
#   gh workflow run real-specimen-gate.yml -f mode=write-baseline
# Local runs differ (OCR-inference float variance), so never hand-edit the counts.
#
# Pure bash + a JSON field grep. Nothing to install.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

baseline="knowledge/benchmarks/real-specimen-mrz-baseline.json"
readme="README.md"
status=0

fail() {
    printf '  %s\n' "$1" >&2
    status=1
}

[ -f "$baseline" ] || { echo "FAIL: $baseline is missing" >&2; exit 1; }
[ -f "$readme" ] || { echo "FAIL: $readme is missing" >&2; exit 1; }

# Pull the three numbers we assert on. The baseline is machine-written and flat,
# so a field grep is sufficient and avoids a jq dependency.
json_int() {
    local key="$1"
    local v
    v="$(grep -oE "\"$key\"[[:space:]]*:[[:space:]]*[0-9]+" "$baseline" | grep -oE '[0-9]+$' | head -1)"
    [ -n "$v" ] || { echo "FAIL: could not read \"$key\" from $baseline" >&2; exit 1; }
    printf '%s' "$v"
}

scored="$(json_int scored)"
documents="$(json_int documents)"
hits="$(json_int tier1_hits)"
no_mrz="$(json_int no_mrz_found)"
checksum="$(json_int checksum_failed)"

# Optional buckets: absent from an older baseline, so default to 0 rather than
# aborting the way `json_int` does for the required ones.
json_int_or_zero() {
    local v
    v="$(grep -oE "\"$1\"[[:space:]]*:[[:space:]]*[0-9]+" "$baseline" | grep -oE '[0-9]+$' | head -1)"
    printf '%s' "${v:-0}"
}
false_positives="$(json_int_or_zero false_positive_mrz)"

# One decimal place, rounded half-up, matching how the README states it.
rate="$(awk -v h="$hits" -v s="$scored" 'BEGIN { printf "%.1f", (h * 100.0) / s }')"
# The corpus-level rate: the same hits over every document in the corpus,
# including the ones no pipeline could read. Published alongside the first so
# neither can be accused of flattering by exclusion.
corpus_rate="$(awk -v h="$hits" -v d="$documents" 'BEGIN { printf "%.1f", (h * 100.0) / d }')"

echo "baseline: ${hits}/${scored} = ${rate}% scored, ${hits}/${documents} = ${corpus_rate}% of corpus"
echo "          (no_mrz_found ${no_mrz}, checksum_failed ${checksum}, unattackable $((documents - scored)))"

# 1. The README must state BOTH rates, each as "<hits> / <denominator> = <rate>%".
#    Tolerate any run of spaces around the slash so a reflow does not fail the build.
check_rate() { # <denominator> <rate> <what>
    if ! grep -qE "${hits}[[:space:]]*/[[:space:]]*$1[[:space:]]*=[[:space:]]*$2%" "$readme"; then
        fail "README.md does not state the $3 rate '${hits} / $1 = $2%'."
        fail "Found instead: $(grep -oE '[0-9]+[[:space:]]*/[[:space:]]*[0-9]+[[:space:]]*=[[:space:]]*[0-9.]+%' "$readme" | head -3 | tr '\n' ' ')"
    fi
}
check_rate "$scored" "$rate" "scored"
check_rate "$documents" "$corpus_rate" "corpus-level"

# 2. The README must name the dominant miss kind with its current count. Which kind
#    dominates is derived here, not assumed -- if the ordering flips back, this
#    script starts demanding the other one rather than silently passing.
if [ "$no_mrz" -ge "$checksum" ]; then
    dominant="no_mrz_found"; dominant_n="$no_mrz"
else
    dominant="checksum_failed"; dominant_n="$checksum"
fi

if ! grep -qE "\`${dominant}\`[^0-9]*${dominant_n}" "$readme"; then
    fail "README.md does not name the dominant miss as \`${dominant}\` (${dominant_n} of ${scored})."
fi

# 3. Guard against the specific way this went wrong: the README naming the
#    *other* kind as dominant, or quoting a superseded count for it.
if [ "$dominant" = "no_mrz_found" ]; then
    if grep -qiE 'largest.{0,40}checksum_failed|checksum_failed.{0,40}ahead of' "$readme"; then
        fail "README.md still claims checksum_failed is the largest miss; no_mrz_found (${no_mrz}) overtook it."
    fi
fi

# 4. The README states how far ahead the dominant miss is, as a "N.N×"
#    multiplier. That sentence is prose around two numbers this script already
#    knows, so it can go stale on its own -- it did: the ratio was 3.5x while
#    the counts said 1.8x, because 42 documents that carry no MRZ at all were
#    being counted as detection failures. Checked only when the README actually
#    states a multiplier, so removing the sentence is allowed; stating a wrong
#    one is not.
#    Scoped to the two lines around the dominant-miss sentence, not the whole
#    file: README.md also states a "~2.5×" GPU speedup, and a file-wide search
#    for a multiplier finds that one first.
if [ "$checksum" -gt 0 ]; then
    ratio="$(awk -v a="$dominant_n" -v b="$checksum" 'BEGIN { printf "%.1f", a / b }')"
    stated="$(grep -A2 -E "\`${dominant}\`" "$readme" | grep -oE '[0-9]+\.[0-9]+×' | head -1 || true)"
    if [ -n "$stated" ] && [ "$stated" != "${ratio}×" ]; then
        fail "README.md states the dominant miss is ${stated} the other; the baseline says ${ratio}× (${dominant_n} vs ${checksum})."
    fi
fi

# 5. A committed baseline must never bless a false positive. `false_positive_mrz`
#    is a checksum-valid MRZ returned for a document that carries none -- a
#    hallucinated record, or a mislabelled corpus file. Either way, freezing one
#    into the baseline makes it the accepted state and the gate stops objecting.
if [ "$false_positives" -gt 0 ]; then
    fail "the committed baseline records ${false_positives} false positive(s) (\`false_positive_mrz\`)."
    fail "a checksum-valid MRZ was returned for a document tagged as carrying none — find out which"
    fail "(\`provider-bench --real-specimens --mrz-only --verbose\`) before this is baselined as normal."
fi

if [ "$status" -eq 0 ]; then
    echo "OK: README.md headline numbers match $baseline"
else
    cat >&2 <<EOF

The README's headline accuracy claim is out of step with the committed baseline.
Update README.md's Accuracy section to the numbers printed above, and put any
additional figures in knowledge/benchmarks/README.md rather than a second
document -- that duplication is what this check exists to prevent.
EOF
fi

exit "$status"
