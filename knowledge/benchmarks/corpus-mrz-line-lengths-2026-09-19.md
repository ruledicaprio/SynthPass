# Corpus MRZ-band line-length distribution

**Date:** 2026-09-19 · **MAIN:** `4510672` · **DATA:** `396b22f3b97ee36275a5329667ee37cbd0449b40` · **Evidence:** Observed (release native line-extractor sweep over 295 corpus images) · **Status:** current

This measurement asks whether the truncated groups seen on Russia and Germany are common across the
corpus. The release probe ran detection, line grouping, and recognition once per image. For each
image it selected the highest-scoring contiguous two- or three-line candidate, even when that
candidate was below the production acceptance threshold, and recorded the longest recognized line
in that candidate. This keeps null-band documents measurable while making the selection rule explicit.
Expected widths are TD1 30, TD2/MRV-B 36, and TD3/MRV-A 44. Only geometry and cell counts are
reported; no MRZ text, holder name, or character values are included.

## Observed

The probe processed **295 images**. The requested scored outcome buckets contain 140 hits, 10
`checksum_failed` documents, and 2 `no_mrz_found` documents.

| outcome | documents | reached or exceeded expected width | exactly expected | below expected |
| :--- | ---: | ---: | ---: | ---: |
| hit | 140 | 127 | 125 | 13 |
| checksum_failed | 10 | 6 | 6 | 4 |
| no_mrz_found | 2 | 2 | 0 | 0 |

### Hits

The 140 hits are overwhelmingly full-width: **125/140 (89.3%)** have a longest candidate line
exactly at the expected width. One is one cell over and one is two cells over; the remaining 13 are
below width. The shortfall distribution is: one hit at −1, two at −2, one at −5, three at −6, two
at −8, one at −10, one at −15, and two at −44 cells.

### Checksum failures

Six of ten reach the expected width exactly. Four are short: two by one cell, one by five, and one
by eight. The two cases that motivated this probe sit in this bucket: Germany reaches 36/44 and
Russia 25/30; both are among the short reads.

### No-MRZ-found outcomes

Both scored `no_mrz_found` rows produce candidate lines longer than their TD1 width—32/30 and
31/30. Because their candidates are below the production band threshold or otherwise unusable,
this is evidence that a long recognized line alone does not establish a valid MRZ band.

## Hypothesized

The corpus does not show systematic truncation among successful reads: 89.3% of hits reach exactly
the expected width, and only 13/140 are short under the candidate-region rule. Truncation is much
more concentrated among checksum failures (4/10 short), including the two null-band cases that
motivated the measurement.

The result supports a narrow line-grouping weakness on difficult or perspective-heavy documents,
rather than a corpus-wide fragility. The two over-width `no_mrz_found` candidates also caution
against treating line length as sufficient evidence; grouping, score, checksum validation, and
format agreement must remain separate signals.

The candidate-region rule is deliberately broader than the production detector because it retains
sub-threshold windows. These counts measure the extractor's best contiguous candidates, not a new
acceptance rule or a change to the real-specimen baseline.
