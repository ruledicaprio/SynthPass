#!/usr/bin/env bash
# Verify the README's and ROADMAP's headline accuracy figures match the committed CI baseline.
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
# carrying live numbers, and README.md and ROADMAP.md each carry exactly one
# headline figure. This script is what makes that rule load-bearing rather than
# aspirational -- it reads the CI-written baseline and fails if either disagrees.
#
# knowledge/benchmarks/README.md's own "Current headline numbers" live block is
# checked too (added for T12, tools/rebless.py): it is the document that actually
# carries the numbers README.md and ROADMAP.md only summarize -- both rates, the
# outcomes heading's document count, all six outcome-table bucket counts, and the
# "N of the <documents> specimens cannot produce" sentence. This gap is real: on
# cohort c12 the live block went stale and nobody noticed until a hand check.
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
roadmap="knowledge/ROADMAP.md"
bench_readme="knowledge/benchmarks/README.md"
status=0

fail() {
    printf '  %s\n' "$1" >&2
    status=1
}

[ -f "$baseline" ] || { echo "FAIL: $baseline is missing" >&2; exit 1; }
[ -f "$readme" ] || { echo "FAIL: $readme is missing" >&2; exit 1; }
[ -f "$roadmap" ] || { echo "FAIL: $roadmap is missing" >&2; exit 1; }
[ -f "$bench_readme" ] || { echo "FAIL: $bench_readme is missing" >&2; exit 1; }

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
no_mrz_expected="$(json_int_or_zero no_mrz_expected)"
redacted_mrz="$(json_int_or_zero redacted_mrz)"
checksum_failed_specimen="$(json_int_or_zero checksum_failed_specimen)"

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

# 6. ROADMAP.md obeys the same one-figure rule as README.md: exactly one rate
#    written "<hits> / <denominator> = <rate>%", and it must be the baseline's
#    scored figure. A second rate, or one that trails the baseline, is the stale
#    "119 / 144 = 82.6%" failure this script exists to catch.
roadmap_rates="$(grep -oE '[0-9]+[[:space:]]*/[[:space:]]*[0-9]+[[:space:]]*=[[:space:]]*[0-9.]+%' "$roadmap" || true)"
roadmap_count="$(printf '%s' "$roadmap_rates" | grep -c . || true)"
if [ "$roadmap_count" -eq 0 ]; then
    fail "ROADMAP.md states no rate as '<hits> / <denominator> = <rate>%'; it must state exactly one, '${hits} / ${scored} = ${rate}%'."
elif [ "$roadmap_count" -gt 1 ]; then
    fail "ROADMAP.md states ${roadmap_count} rates; only knowledge/benchmarks/README.md carries live numbers, so it must state exactly one ('${hits} / ${scored} = ${rate}%')."
    fail "Found: $(printf '%s' "$roadmap_rates" | tr '\n' ' ')"
elif ! grep -qE "${hits}[[:space:]]*/[[:space:]]*${scored}[[:space:]]*=[[:space:]]*${rate}%" "$roadmap"; then
    fail "ROADMAP.md's one rate is not the baseline's scored figure '${hits} / ${scored} = ${rate}%'."
    fail "Found instead: ${roadmap_rates}"
fi

# 7. knowledge/benchmarks/README.md's "Current headline numbers" live block must
#    state both rates itself -- it is the document README.md and ROADMAP.md are
#    summarizing, not a third copy of their claim.
check_bench_rate() { # <denominator> <rate> <what>
    if ! grep -qE "${hits}[[:space:]]*/[[:space:]]*$1[[:space:]]*=[[:space:]]*$2%" "$bench_readme"; then
        fail "$bench_readme does not state the $3 rate '${hits} / $1 = $2%' in its live block."
        fail "Found instead: $(grep -oE '[0-9]+[[:space:]]*/[[:space:]]*[0-9]+[[:space:]]*=[[:space:]]*[0-9.]+%' "$bench_readme" | head -5 | tr '\n' ' ')"
    fi
}
check_bench_rate "$scored" "$rate" "scored"
check_bench_rate "$documents" "$corpus_rate" "corpus-level"

# 8. The "Real-specimen outcomes" heading states the current document count.
if ! grep -qE "\(${documents} documents\)" "$bench_readme"; then
    fail "$bench_readme's outcomes heading does not state '(${documents} documents)'."
fi

# 9. The outcome table's per-bucket counts, each `| \`bucket\` | N |` (a count
#    may be bold, e.g. '| `checksum_failed` | **10** |', so the digits are
#    matched with optional surrounding '**' rather than an exact cell).
check_bucket_count() { # <bucket key> <count>
    if ! grep -qE "\`$1\`[^|]*\|[[:space:]]*\*{0,2}$2\*{0,2}[[:space:]]*\|" "$bench_readme"; then
        fail "$bench_readme's outcome table does not show \`$1\` = $2."
    fi
}
check_bucket_count no_mrz_found "$no_mrz"
check_bucket_count checksum_failed "$checksum"
check_bucket_count false_positive_mrz "$false_positives"
check_bucket_count no_mrz_expected "$no_mrz_expected"
check_bucket_count redacted_mrz "$redacted_mrz"
check_bucket_count checksum_failed_specimen "$checksum_failed_specimen"

# 10. The "Why two rates" paragraph's "N of the <documents> specimens cannot
#     produce a Tier-1 hit" sentence -- N is documents - scored, stated
#     explicitly rather than left as arithmetic for the reader.
unattackable=$((documents - scored))
if ! grep -qE "${unattackable}[[:space:]]+of[[:space:]]+the[[:space:]]+${documents}[[:space:]]+specimens[[:space:]]+cannot[[:space:]]+produce" "$bench_readme"; then
    fail "$bench_readme does not state '${unattackable} of the ${documents} specimens cannot produce a Tier-1 hit'."
fi

if [ "$status" -eq 0 ]; then
    echo "OK: README.md, ROADMAP.md and $bench_readme's live block match $baseline"
else
    cat >&2 <<EOF

A headline accuracy claim is out of step with the committed baseline. Update the
one figure in README.md or knowledge/ROADMAP.md to the numbers printed above,
and knowledge/benchmarks/README.md's own live block to match -- that is the
document the other two summarize, and duplication elsewhere is what this check
exists to prevent.
EOF
fi

exit "$status"
