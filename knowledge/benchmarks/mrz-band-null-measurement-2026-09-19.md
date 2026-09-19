# MRZ-band null measurement: Germany 2024 and Russia 2019

**Date:** 2026-09-19 · **MAIN:** `1db1434` · **DATA:** `396b22f3b97ee36275a5329667ee37cbd0449b40` · **Evidence:** Observed (temporary native geometry probe; synthetic benchmark run) · **Status:** current

This measurement investigates the only two scored misses whose recorded `mrz_band_score` is
`null`. The probe ran the native detector's line scoring on the two named images and enumerated
every contiguous two-line and three-line candidate. The acceptance threshold is
`MRZ_BAND_MIN_AVG_SCORE = 0.15`. No MRZ text, holder name, or character values are recorded here.

## Germany passport specimen, 2024

**Observed:** The page was processed at rotation 0 with 44 recognized lines. No candidate cleared the
threshold. The best two-line candidate was lines **41–42** (0-based, end-exclusive 43), with an
average score of **0.037107**; its line lengths were 36 and 25, with individual scores 0.015413 and
0.058802. The best three-line candidate was lines **40–42** (end-exclusive 43), with an average of
**0.031998**; the added line had length 46 and score 0.021778. Every other two-line and three-line
window scored lower. The detector therefore returned no band.

**Hypothesized:** The candidate windows are in the lower-page region and contain several long lines,
but their OCR text and measured glyph geometry do not jointly satisfy the MRZ score. This is
consistent with ADR-0015's warning that the band detector scores recognized text: a weak read can
make a real band undiscoverable. The downstream read retaining the expected TD3 shape shows that
band absence and format failure are separable outcomes.

## Russian Federation passport specimen, 2019

**Observed:** The page was processed at rotation 0 with 47 recognized lines. No candidate cleared the
threshold. The best two-line candidate was lines **0–1** (end-exclusive 2), average **0.062818**:
line 0 had length 25 and score 0.125637, while line 1 had length 1 and score 0. The best three-line
candidate was lines **0–2** (end-exclusive 3), average **0.041879**; line 2 had length 2 and score 0.
Every other window scored lower. The detector therefore returned no band.

**Hypothesized:** This is a downstream recognition and segmentation failure rather than a marginal
threshold miss. The highest-scoring line is short of every TD target, and the adjacent lines are
single-cell fragments, so no contiguous group can approach the threshold. The resulting three-line
TD1-shaped downstream read is therefore compatible with the null band: the detector is scoring the
wrongly segmented text it received, not rejecting a clean TD3-shaped candidate.

## Finding

Both null scores are explained by the same mechanism at different severities: the detector has no
independent pixel-level MRZ oracle and scores recognized line text. Germany has long lower-page
candidates whose combined text/geometry score remains low; Russia has fragmented short candidates
and loses the format entirely. A threshold change alone would not address the Russian case and
would risk admitting Germany's low-confidence windows. The measured result supports ADR-0015's
premise and does not justify a production change.

## Synthetic CER table (Task 2)

The requested live command was run as written:
`cargo run --release -p synthpass-bench --bin synthpass-bench -- --count 20 --document-type TD3`.
The rendered table was:

```
mean character error rate by field (worst first):
    48.71%  given_names         line 1
    25.39%  surname             line 1
    20.00%  document_type       line 1
    15.00%  issuing_country      line 1
     8.82%  mrz_lines           no single line
     7.86%  personal_number     line 2
     6.67%  nationality         line 2
     5.00%  date_of_birth       line 2
     5.00%  date_of_expiry      line 2
     5.00%  document_number     line 2
     5.00%  sex                 line 2
  line 1 mean 27.28%   line 2 mean 5.75%   ratio 4.7x
```

This matches the report's grouped-by-line result: line 1 is 27.28%, line 2 is 5.75%, and the ratio
is 4.7×. The run produced 19/20 Tier-1 hits and wrote `artifacts/bench-report.json`; those synthetic
results do not alter the real-specimen baseline.
