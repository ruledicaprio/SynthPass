# Russia 2019 MRZ segmentation follow-up

**Date:** 2026-09-19 · **MAIN:** `1db1434` · **DATA:** `396b22f3b97ee36275a5329667ee37cbd0449b40` · **Evidence:** Observed (release native line-extractor probe) · **Status:** current

This follow-up measures the line extractor before MRZ scoring on
`Russian_Federation_Passport_Specimen_P0_RUS_2019_mrz.jpg`. The probe used the release build,
the same detection and recognition models as the native path, and no retry or repair arm. It
reports geometry and recognized-cell counts only; no MRZ text, holder name, or character values are
recorded.

## Observed

The image is **900 × 1286** pixels. Word detection produced **103** words, `find_text_lines`
produced **49** groups, and recognition returned **47** lines.

The two lines that won the scoring probe are not in the MRZ area. They are at the top edge of the
image: line **0** has `x=404.0, y=-1.0, w=367.0, h=19.0` and 25 recognized cells; line **1** has
`x=847.0, y=1.0, w=42.0, h=16.0` and 1 cell. Their vertical placement and geometry identify them
as unrelated upper-page material. This is why line 0's individual score can approach the threshold
while its neighbour contributes zero.

The lower-page region where the printed MRZ is located produced these long groups and fragments:

| 0-based line | x | y | width | height | cells |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 43 | 92.0 | 1102.0 | 314.0 | 22.0 | 20 |
| 44 | 93.0 | 1139.0 | 377.0 | 26.0 | 24 |
| 45 | 469.0 | 1145.0 | 19.0 | 20.0 | 1 |
| 46 | 766.0 | 1149.0 | 31.0 | 19.0 | 4 |

Lines 43 and 44 occupy the expected two-row vertical band, but each is substantially shorter than
a full TD3 row. Lines 45 and 46 overlap the second row vertically as isolated fragments. The
extractor therefore did not return two complete MRZ line groups; it returned two truncated groups and
two same-band fragments.

## Hypothesized

The null band is caused upstream of `mrz_line_score`. The extractor is finding the MRZ's lower-page
region, but perspective and the photographed two-page booklet cause the row text to be split and
truncated: the long groups do not span the full row, while additional word groups are emitted as
separate lines. The scorer then sees short, incomplete candidates and cannot form a qualifying
contiguous pair. The top-edge 25-cell line is a false geometric contender, not a recovered MRZ row.

This explains both observed symptoms without changing the threshold: the best score comes from
unrelated top-page material, while the actual lower-page rows never arrive at scoring as complete
TD3 lines. The next useful experiment is therefore detector/line-group geometry on perspective
images, rather than a scoring-threshold adjustment.
