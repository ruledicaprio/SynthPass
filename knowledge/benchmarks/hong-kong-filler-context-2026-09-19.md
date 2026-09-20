# Hong Kong filler fabrication and ADR-0014

**Date:** 2026-09-19 · **MAIN:** `4510672` · **DATA:** `396b22f3b97ee36275a5329667ee37cbd0449b40` · **Evidence:** Observed (recorded native OCR dump and hand-transcribed fixtures) · **Status:** current

This measurement tests whether the two Hong Kong filler-fabrication cases match ADR-0014's
isolated-versus-in-run suppression profile. The comparison uses the recovered native lines from the
instrumented miss dump and the two committed fixture lines. Only the trailing filler run after the
last non-filler name cell is counted; no MRZ text, holder name, or character values are reproduced.

## Observed

| specimen | truth trailing filler run | fabricated cells | isolated | boundary | interior (run ≥3) |
| :--- | ---: | ---: | ---: | ---: | ---: |
| Hong Kong 2007 | 24 | 15 | 0 | 1 | 14 |
| Hong Kong 2019 | 26 | 26 | 0 | 2 | 24 |

The 2007 fabricated positions are the first cell of the trailing run plus 14 interior cells; the
last run cell remains filler. The 2019 read fills every trailing-run cell, including both run
boundaries and all 24 interior cells. Both recovered name lines are already the target width, so
`crates/mrz`'s one-character-short `restored()` path cannot have manufactured these tails.

## Hypothesized

The distribution does **not** match ADR-0014's suppression profile. ADR-0014 predicts that context
suppresses an isolated filler while an in-run filler is comparatively unaffected. These real reads
contain no fabricated isolated filler and are dominated by interior in-run cells, including a
complete 26-cell tail in 2019. They therefore contradict the claim that these documents are a real
document instance of that measured suppression mechanism.

A long contiguous fabricated tail remains compatible with other mechanisms: the recognizer may be
reading non-filler structure as glyphs, or a geometry/alignment error may be projecting unrelated
content into the trailing run. The full-width recovered lines rule out the specific one-cell-short
padding branch, but the dump alone does not distinguish those alternatives.

The merged twelve-miss report should remove its ADR-0014 attribution. The Hong Kong cases remain a
measured filler-fabrication failure mode; they are not evidence for the ADR's isolated-filler
suppression effect.
