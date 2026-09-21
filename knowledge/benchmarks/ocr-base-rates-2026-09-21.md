# OCR base rates

**Date:** 2026-09-21 · **MAIN:** `bd90506` · **DATA:** `396b22f3b97ee36275a5329667ee37cbd0449b40` · **Evidence:** Observed (release `provider-bench` at `cd4f519`, 20 commits behind main) · **Status:** current

The run used `--real-specimens --mrz-only --dump-ocr --dump-ocr-hits`. The dump is at the
gitignored path `artifacts/provider-bench-miss-ocr-dump.jsonl`; it is not a repository artifact.
The stale tree reproduced the committed Tier-1 baseline exactly: 261 documents, 152 scored, 140
hits, and the expected miss buckets (10 `checksum_failed`, 19 `checksum_failed_specimen`, 2
`no_mrz_found`, 51 `no_mrz_expected`, 39 `redacted_mrz`, all other listed buckets zero). Strict
names were also unchanged at 12 strict hits and 33 name-scorable hits of 45. The prior 31-record
miss-only dump matched the new run on all six legacy keys for all 31 records.

The key stratified result is that **78% of substitutions on damaged-name hits were the filler
glyph**. The blended rate must be read with the clean-hit and damaged-hit strata below.

## Method

Only records with a ground-truth fixture contribute character cells. A cell is counted when both
the recovered and transcribed lines contain that position. The dump had 171 records: 140 hits and
31 misses. Sixty-four records had ground truth; 107 did not and contributed no cells.

## Cells and glyph base rates

There were **5,326 compared cells** from 64 documents: 2,908 on hits and 2,418 on misses. The
following counts are printed glyphs and correctly recovered glyphs.

| glyph | printed | correct |
| :---: | ---: | ---: |
| 0 | 549 | 452 |
| 1 | 246 | 207 |
| 2 | 198 | 167 |
| 3 | 130 | 107 |
| 4 | 123 | 97 |
| 5 | 128 | 100 |
| 6 | 87 | 70 |
| 7 | 123 | 96 |
| 8 | 145 | 128 |
| 9 | 131 | 113 |
| A | 189 | 132 |
| B | 35 | 28 |
| C | 61 | 48 |
| D | 56 | 42 |
| E | 122 | 82 |
| F | 42 | 35 |
| G | 36 | 28 |
| H | 54 | 43 |
| I | 99 | 68 |
| J | 16 | 11 |
| K | 40 | 30 |
| L | 59 | 37 |
| M | 109 | 69 |
| N | 116 | 80 |
| O | 74 | 57 |
| P | 112 | 100 |
| Q | 2 | 2 |
| R | 92 | 59 |
| S | 93 | 70 |
| T | 48 | 31 |
| U | 50 | 32 |
| V | 23 | 15 |
| W | 5 | 3 |
| X | 4 | 3 |
| Y | 16 | 13 |
| Z | 33 | 24 |
| `<` | 1,880 | 1,515 |

The first-pass whole-fixture rates were **0: 97/549 = 17.7% wrong** and **`<`: 365/1,880 = 19.4%
wrong**. Those are naive positional rates: they count a shifted line as a character error. They
are retained here because the conditioning and alignment traps must remain visible.

## C47 alignment correction

The first pass compared equal columns directly. A Levenshtein backtrace was then applied per line,
with diagonal substitutions counted as recognition errors and insertions/deletions counted as
separate shift events. Under that alignment, the three rates are:

| glyph | miss-only | naive-positional | substitution-only after alignment |
| :---: | ---: | ---: | ---: |
| 0 | 17.9% | 17.7% (97/549) | **16.1% (88/546)** |
| `<` | 12.3% | 19.4% (365/1,880) | **17.0% (316/1,856)** |

The aligned denominators include only truth cells paired by the backtrace; cells represented by an
insertion or deletion are reported as shifts, not substitutions. The dump contains 64 fixture
documents. Shift runs ranged from one cell to 90 cells, so the shift population is not a
single-cell nuisance.

The 15% split is diagnostic context for the bimodal positional result, not a threshold used to
produce these rates. The substitution-only figures come from the alignment, not from discarding a
bucket.

The blended substitution rate is not representative of either population. Split by outcome, the
hit-side rate is 6.05% and the miss-side rate is 22.37%. The per-glyph strata are:

| glyph | hits | misses |
| :---: | ---: | ---: |
| 0 | 1.1% | 35.0% |
| 1 | 0.0% | 31.5% |
| 2 | 0.0% | 35.2% |
| 3 | 2.8% | 34.5% |
| 4 | 0.0% | 40.4% |
| 5 | 0.0% | 42.6% |
| 6 | 0.0% | 44.1% |
| 7 | 0.0% | 50.0% |
| 8 | 0.0% | 22.9% |
| 9 | 0.0% | 32.6% |
| A | 2.4% | 8.7% |
| B | 3.7% | 12.5% |
| C | 6.5% | 6.9% |
| D | 2.9% | 13.6% |
| E | 0.0% | 8.8% |
| F | 0.0% | 15.0% |
| G | 0.0% | 8.7% |
| H | 0.0% | 3.3% |
| I | 3.4% | 12.5% |
| J | 7.7% | 33.3% |
| K | 11.8% | 17.4% |
| L | 0.0% | 10.7% |
| M | 9.8% | 28.3% |
| N | 4.1% | 16.3% |
| O | 5.7% | 10.3% |
| P | 1.4% | 12.2% |
| Q | 0.0% | 0.0% |
| R | 5.1% | 17.3% |
| S | 3.4% | 16.1% |
| T | 0.0% | 24.0% |
| U | 0.0% | 18.8% |
| V | 15.4% | 37.5% |
| W | 50.0% | 33.3% |
| X | 0.0% | 50.0% |
| Y | 0.0% | 0.0% |
| Z | 5.3% | 15.4% |
| `<` | 13.0% | 18.8% |

Within hits, 24 clean documents contributed 2,114 cells at 2.08% substitutions; the other nine
contributed 794 cells at 16.62%. In the damaged-hit stratum, 103 of 132 substitutions (78%) were
the filler glyph. Digits stayed at 0.0–1.3% in the clean-hit stratum, while `<` was 3.9% there and
33.1% in the damaged-hit stratum. This is the strongest evidence that the remaining problem is
filler-run segmentation in the name region, not general image quality.

The shift-count discrepancy is **unresolved**, and the explanation previously given here was
withdrawn as unsupported. Two independent runs report **458 operations across 36 documents** and
**92 operations across 26 documents** for what both describe as the same quantity: individual
inserted or deleted cells produced by a per-line Levenshtein backtrace. A definitional difference
was proposed -- that one figure groups operations into events and the other does not -- but neither
implementation performs such grouping, so that explanation does not hold.

The obvious structural cause was tested and refuted. Of the 171 dumped records, 107 carry no ground
truth, 3 carry no recovered lines, **2** differ in line count (both misses, two truth lines read as
three, +2 cells each) and **none** differ in per-line length; the remaining 59 are fully comparable.
Including or excluding those two documents moves the operation count by at most 4, not by a factor
of five.

Until one implementation is committed and re-run against the other, the per-glyph substitution rates
in this report should be read as the load-bearing result and the shift-operation count as
provisional. The two do not depend on each other: substitutions are counted from diagonal
backtrace steps and are unaffected by how insert/delete steps are tallied.

## Checksum-valid hits with surviving mismatches

Twenty-six of the 33 labelled hits carried a non-empty mismatch map. Their differing-cell counts
were distributed as follows:

| differing cells | hits |
| ---: | ---: |
| 1 | 4 |
| 2 | 2 |
| 3 | 1 |
| 5 | 2 |
| 6 | 3 |
| 7 | 3 |
| 9 | 1 |
| 11 | 1 |
| 17 | 1 |
| 18 | 1 |
| 20 | 1 |
| 23 | 1 |
| 25 | 1 |
| 29 | 1 |
| 32 | 1 |
| 33 | 1 |
| 37 | 1 |

Those surviving mismatches total 314 cells: 308 in **Uncovered** fields, 5 under
**OwnCheckDigit**, and 1 under **CompositeOnly**. By field, the counts were name 285, issuing_country
13, document_code 8, personal_number 5, nationality 2, and optional_data_1 1. The hit-side
distribution is therefore dominated by uncovered fields, like the miss-side measurement, while
the total population and field mix differ.

## Unresolved

The committed outcome-ledger SHA was not regenerated by this run, so it was not possible to claim
that the local ledger hash matches `6b84366ba64b58351688b992d480881cbf400daca052259bc841483460ed805a`.
All baseline counts and strict-name counts matched exactly. The worktree remains 20 commits behind
`main`; the run demonstrates behavioral equivalence on the gate dimensions and does not remove that
provenance caveat.

The hit-side checksum-covered mismatches remain unresolved in meaning: five cells are in
OwnCheckDigit fields and one is in a CompositeOnly field even though the documents passed their
checks. They may be compensating errors or a layout/transcription edge case and are retained as
counts, not explained away. Eight hits have a wrong `document_code` cell (the format-gate field),
which independently corroborates the format-gate finding.
