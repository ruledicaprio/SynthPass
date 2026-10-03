# Elimination probe, items 1 and 2, on 46 located public specimens: a digit's top edge stands 0.07 pitch above a letter's (median AUC 0.997 at the 2% edge) but one threshold separates them in only 28 of 56 lines; the left-quarter ink separates all 23 undisputed `K` cells from all 1,466 fillers at native and down to 9 px per cell

**Date:** 2026-10-03 · **MAIN:** `a321045` ([#671](https://github.com/ruledicaprio/SynthPass/pull/671)'s merge; the run was made on 2026-10-01 at its pre-merge head `9e8246e`, which is `origin/main` `9bd5d3e` plus the probe, and the probe's locate and measure paths are unchanged on `main` through `16f6865`: [#679](https://github.com/ruledicaprio/SynthPass/pull/679) added only the `--sweep` envelope) · **DATA:** the public corpus at `samples-data` `65a5040`, the pin of `main`'s committed baseline since [#658](https://github.com/ruledicaprio/SynthPass/pull/658); the probe verified every image against `samples/corpus.jsonl`'s sha256; 67 documents carry a reviewed fixture and 46 were located · **Evidence:** Observed on public real specimens, ink-only segmentation, reviewed-fixture truth · **Status:** current

**2026-10-03.** These are items 1 and 2 of the measurement plan in
[`mrz-geometric-elimination.md`](../research/mrz-geometric-elimination.md) ("What would have to be measured"), taken by
[#671](https://github.com/ruledicaprio/SynthPass/pull/671)'s bench-only [`elimination_probe`](../../crates/synthpass-bench/examples/elimination_probe.rs) on the public
real specimens against their reviewed fixtures. The probe runs no OCR: it finds each MRZ line from ink alone, accepts a
line only when every cell holds exactly one connected component, measures each cell on its own component's box, and
records classes and geometry only. Item 1 asks whether a cell's top edge separates printed digits from printed letters
within one line, and down to what resolution. Item 2 asks whether the share of a glyph's ink in its left quarter
separates the filler `<` from `K`, the one pair no check digit can tell apart. **No decision follows from this note**:
nothing in the product reads these signals, [ADR-0014](../decisions/ADR-0014-per-cell-ocrb-classification.md) stays
Proposed, and no code, baseline or default changes with it. The numbers were held back until every located overlay had
been checked by eye (below).

**The run.**

| | |
|---|---|
| Tool | `elimination_probe` at #671's head `9e8246e`, run 2026-10-01 22:3x UTC on the Web session's container (a debug build; 75 s over the 67 documents) |
| Command | `cargo run -p synthpass-bench --example elimination_probe -- --samples-root <public corpus> --out <local dir> --overlays` |
| Fractions | the top edge read at 0.02, 0.05, 0.10 and 0.20 of each cell's ink, as the plan asks ("report the fraction alongside the separation") |
| Steps | each located line re-measured at 20, 16, 13, 11, 9, 8, 7, 6, 5 and 4 px per cell, at the steps below its own pitch, on the cell boxes found at native resolution: the segmentation is held fixed and only the signal degrades |
| Output | 36,146 cell rows in a local `cells.jsonl` (4,068 at native, the rest at the steps) and 46 overlay images; both stay local. Every number below was recomputed from the rows and agrees with the probe's own tables |
| Overlay gate | 7 documents checked by eye by the Web session on 2026-10-02; all 46 by the desktop lane on 2026-10-03, on its own run of the probe on `main` `0e47197` with the same samples, which located the same 46 documents and refused the same 21 (its report is in the project library) |

**Population.** 67 documents carry a reviewed fixture directly under `samples/ocr_fixtures/`. **46 were located** (36 TD3
pages of 2 x 44 cells, 10 TD1 cards of 3 x 30; 102 lines) and **21 were refused, every one of them `no_zone`**: the
locator found fewer regular lines of exactly the format's width than the format has. Three of the 21 are rotated pages
by file name, seven are under 320 px wide (a 44-cell line in a 270 px image is about 6 px per cell, the glyph floor), and
the other eleven were not examined beyond Poland 2023, whose fillers touch (#671). The list is at the end. **The 46 are
the documents the ink-only locator accepts, and every figure below is about them, not about the corpus** (Observed).

**The overlay gate, and three facts it fixed.** An overlay draws each located zone with its cell boxes coloured by
fixture class and the 5% top edge in red. Both eye-checks judged geometry only (one box per printed glyph, no
off-by-one, no drift along a line, the zone correctly bounded) and transcribed nothing. The desktop lane's check of all
46: **46 OK, 0 off-by-one, 0 drift, 0 wrong bound**; the 7 checked here agree, including the Azerbaijan 2022 strip,
whose visible skew every box tracks (Observed). Three facts from the overlays bear on the numbers.

- **The boxes are the components' own ink boxes, not fixed-pitch cells.** `measure_cells` measures each cell on its
  component's box, widened by a quarter pitch above and below only. So every per-cell share below is a share of the
  glyph's own box width: that is what makes the left quarter a shape measure (`K`'s stem fills it, a filler's vertex
  barely does), and it also means a narrow glyph has a narrow box. The rise is measured against the line's skew, so only
  differences within one line mean anything; no baseline is fitted (Observed, from the code).
- **On noisy backgrounds some boxes are taller** (Czechia 2005, Russia 2019): a stray mark joined the component, and the
  top edge is the measurement that moves when that happens. Russia 2019 line 2 is among the four lowest-AUC lines below;
  whether the two are linked was not tested (Observed; the link Hypothesized).
- **The check sees boxes on glyphs, not classes.** Classes come from the fixture. One fixture cell is disputed: on the
  Slovenia 2022 ID card back, line 1 column 28, the print shows a filler where the reviewed fixture has `K` (#671's smoke
  run recorded it; the fixture is Rusmir's to settle). It matters for item 2 below.

## Item 1: the top edge, digit against letter, within one line

A cell's **rise** is its top edge, read at fraction `f` of its ink, measured upward against the line's skew in cells
of the line's pitch. A line **qualifies** when it has at least two digits and two letters (`K` counts as a letter). Per
line and per `f`: the **gap** is the median digit rise minus the median letter rise; the **margin** is the lowest digit
rise minus the highest letter rise (above zero, one threshold separates every digit from every letter in that line);
the **AUC** is the share of digit-letter pairs in which the digit sits higher, ties counting half. **56 of the 102 lines
qualify**: all 36 TD3 second lines and both of TD1's first two lines (10 each); TD3 line 1 and TD1 line 3 carry no digits.
They hold 1,377 digit cells and 324 letter cells.

| Fraction `f` | Qualifying lines | Margin > 0 | Median gap, cells of pitch | Lowest margin | Median AUC | Lines with AUC at least 0.9 | Lowest AUC |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 0.02 | 56 | 28 | 0.072 | -0.180 | 0.997 | 46 | 0.354 |
| 0.05 | 56 | 16 | 0.072 | -0.182 | 0.963 | 45 | 0.346 |
| 0.1 | 56 | 8 | 0.076 | -0.236 | 0.891 | 25 | 0.167 |
| 0.2 | 56 | 3 | 0.072 | -0.296 | 0.823 | 13 | 0.000 |

1. **The ordering holds in distribution.** At `f` 0.02 the median line has AUC 0.997 and 46 of 56 lines are at or above
   0.9; the median gap is 0.072 of the pitch, and it is 0.072 to 0.076 at every fraction (Observed).
2. **The gap is the standard's.** ISO 1073-2's constant-strokewidth table gives digits 2.66 mm and capitals 2.46 mm above
   the baseline, a difference of 0.20 mm, which is 0.079 of the 2.54 mm pitch; ECMA-11's letterpress table gives 2.60
   against 2.46, 0.055 of the pitch ([glyph-dimensions.md](../ocrb/glyph-dimensions.md)). The measured median, 0.072,
   sits between the two and nearer ISO's (Derived).
3. **But one threshold per line holds in only 28 of 56 lines** at the best fraction, and in fewer at the others. The
   gap is sub-pixel at the corpus's resolution: at the lines' own pitches it is a median 1.07 px (the qualifying lines'
   median pitch is 15.3 px per cell). The 28 lines with a positive margin have a median pitch of 20.1 px per cell; the
   28 without, 12.2 (Observed). That the margin is resolution-bound rather than print-bound is Hypothesized: the floors
   below point the same way, and the per-line table shows every line under 12 px per cell except five failing the margin.
4. **The 2% edge is the best of the four fractions on every measure.** At 0.2 the edge sits inside the glyph body,
   where digits and letters do not differ, and the lowest-AUC line reaches 0.000 (Observed).

**The resolution floor.** A line's floor is the finest step it reaches from native with its margin above zero at every
step along the way; a line whose margin fails at the first step below native holds at native only.

| Fraction `f` | Lines with a positive margin at native | Floor at 4 px | 5 to 9 px | 11 to 20 px | Holds at native only | Median floor, px per cell |
|---:|---:|---:|---:|---:|---:|---:|
| 0.02 | 28 | 3 | 10 | 10 | 5 | 9.5 |
| 0.05 | 16 | 3 | 6 | 4 | 3 | 7.5 |
| 0.1 | 8 | 2 | 5 | 1 | 0 | 7.0 |
| 0.2 | 3 | 2 | 0 | 0 | 1 | 4.0 |

Per step, over the lines whose native pitch is above that step:

| Step, px per cell | Lines measured | Margin > 0 at `f` 0.02 | Median AUC at 0.02 | Margin > 0 at `f` 0.05 | Median AUC at 0.05 |
|---|---:|---:|---:|---:|---:|
| native | 56 | 28 | 0.997 | 16 | 0.963 |
| 20 | 16 | 14 | 1.000 | 8 | 0.995 |
| 16 | 27 | 18 | 1.000 | 10 | 0.980 |
| 13 | 31 | 18 | 1.000 | 11 | 0.950 |
| 11 | 43 | 18 | 0.981 | 11 | 0.962 |
| 9 | 55 | 19 | 0.974 | 9 | 0.933 |
| 8 | 56 | 18 | 0.961 | 14 | 0.945 |
| 7 | 56 | 10 | 0.934 | 11 | 0.912 |
| 6 | 56 | 7 | 0.906 | 7 | 0.889 |
| 5 | 56 | 5 | 0.919 | 7 | 0.897 |
| 4 | 55 | 9 | 0.833 | 8 | 0.833 |

5. **Among the 28 lines with a positive margin at native (`f` 0.02), the median floor is 9.5 px per cell**; 3 lines
   hold to 4 px, 10 to between 5 and 9, 10 to between 11 and 20, and 5 hold at native only. The coarsest floor is 20 px
   per cell (Slovenia 2022 line 2, native 30.2). The share of measured lines with a positive margin falls to 10 of 56 at
   7 px, 7 at 6 and 5 at 5; the rise to 9 of 55 at 4 px is noise over 55 lines. The median AUC stays at or above 0.9
   down to 6 px per cell and is 0.833 at 4 (Observed).
6. **Outliers.** Turkiye 2025 line 2 has AUC 0.354 and a gap of -0.003: its digits do not stand above its letters at
   all, and its fillers also carry the highest left-quarter shares of any document (0.246-0.247 against a corpus median
   of 0.148), which points at a different glyph design; not examined further, and no character was read (Observed;
   the design Hypothesized). France ID 2020, at 136 px per cell, holds at `f` 0.02 (AUC 1.000 on both lines) but line 1
   falls to 0.750 at 0.05 and 0.167 at 0.1: at that resolution the fractions reach deep into the glyph (Observed; the
   cause Hypothesized). Uzbekistan 2013 (0.737), Russia 2019 (0.779) and Nigeria 2022 (0.822) are the other lines under
   0.85 at `f` 0.02 (Observed).

## Item 2: left-quarter ink, `K` against the filler, pooled over lines

The **left quarter** is the share of a cell's ink in the first two of eight equal columns of its own box. The cells of
all lines are pooled at each step. The pooled **margin** is the lowest `K` minus the highest filler; the pooled **AUC**
is the share of `K`-filler pairs in which the `K` has the larger share. The last two columns leave out the one disputed
fixture cell (Slovenia 2022 line 1 column 28, above).

| Step, px per cell | `K` cells | `K` left quarter, min / median / max | Filler cells | Filler left quarter, min / median / max | Margin, all `K` | AUC, all `K` | Margin without the disputed cell | AUC without it |
|---|---:|---|---:|---|---:|---:|---:|---:|
| native | 24 | 0.130 / 0.412 / 0.474 | 1466 | 0.076 / 0.148 / 0.262 | -0.132 | 0.967 | +0.065 | 1.000 |
| 20 | 6 | 0.129 / 0.442 / 0.462 | 370 | 0.100 / 0.137 / 0.251 | -0.123 | 0.871 | +0.130 | 1.000 |
| 16 | 12 | 0.130 / 0.432 / 0.450 | 633 | 0.083 / 0.141 / 0.256 | -0.127 | 0.936 | +0.113 | 1.000 |
| 13 | 13 | 0.126 / 0.409 / 0.448 | 752 | 0.088 / 0.142 / 0.255 | -0.129 | 0.935 | +0.108 | 1.000 |
| 11 | 20 | 0.131 / 0.392 / 0.452 | 1117 | 0.080 / 0.146 / 0.258 | -0.127 | 0.961 | +0.061 | 1.000 |
| 9 | 24 | 0.124 / 0.361 / 0.450 | 1432 | 0.074 / 0.146 / 0.257 | -0.134 | 0.964 | +0.037 | 1.000 |
| 8 | 24 | 0.136 / 0.360 / 0.410 | 1466 | 0.067 / 0.148 / 0.311 | -0.175 | 0.970 | -0.006 | 1.000 |
| 7 | 24 | 0.141 / 0.352 / 0.410 | 1466 | 0.072 / 0.148 / 0.390 | -0.250 | 0.973 | -0.096 | 0.999 |
| 6 | 24 | 0.127 / 0.330 / 0.406 | 1466 | 0.059 / 0.147 / 0.393 | -0.266 | 0.964 | -0.121 | 0.999 |
| 5 | 24 | 0.119 / 0.315 / 0.399 | 1466 | 0.046 / 0.153 / 0.326 | -0.208 | 0.961 | -0.076 | 0.999 |
| 4 | 24 | 0.185 / 0.309 / 0.371 | 1464 | 0.000 / 0.163 / 0.309 | -0.124 | 0.986 | -0.091 | 0.997 |

7. **With every fixture cell as it stands, the pooled margin is negative at every step** (-0.132 at native) and the AUC
   is 0.967. The whole of the negative margin is one cell: the disputed Slovenia 2022 cell, whose left quarter is 0.130,
   below the filler median of 0.148 and exactly where a printed filler lands. The other 23 `K` cells run from 0.327 to
   0.474 (Observed).
8. **Without that cell, one threshold separates every `K` from every filler at native**: the lowest `K` is 0.327 against
   the highest filler 0.262, a margin of +0.065 and AUC 1.000 over 23 by 1,466 cells. The pooled margin stays positive
   at 20, 16, 13, 11 and 9 px per cell and turns negative at 8 (one filler at 0.311 against the lowest `K` at 0.305), so
   **the pooled floor is 9 px per cell** (Derived, conditional on the fixture question; with the cell as it stands there
   is no floor).
9. **The left quarter is a `K`-against-filler measure, not a class measure.** Letters' left quarter has a median of
   0.306 and a maximum of 0.709, digits' 0.230 and 0.431, so it does not separate letters from fillers in general; the
   research note scoped it to the one pair the check digits cannot see, and that is where it works (Observed).

## What this does not show

- **No decision.** Nothing in the product reads a top edge or an ink profile; ADR-0014 stays Proposed. The research
  note's items 3 (grid offset, [#674](https://github.com/ruledicaprio/SynthPass/pull/674)) and 4 (the operating envelope, [#679](https://github.com/ruledicaprio/SynthPass/pull/679)) have their own tools and
  are not quoted here.
- **The population is the 46 documents the ink-only locator accepts.** The 21 it refuses include the rotated, the
  smallest and the touching-glyph specimens, so these figures describe documents that segment cleanly, not the
  corpus. Public specimens only; the private and local tracks were never read.
- **The margins are per line and uncorrected.** Fifty-six lines at four fractions give many chances for one cell to
  cross; the AUC is the robust figure, the margin the strict one.
- **The floors resample the band on the native cell boxes.** The segmentation is held fixed, so a floor says where the
  signal fails, not where the locator does; the envelope (#679) measures the other case.
- **One run, one container.** The probe is deterministic and the desktop lane's independent run located the same 46
  and refused the same 21, but the measurement itself was not repeated elsewhere.
- **The disputed fixture cell is reported both ways and settled by neither.** The fixture is Rusmir's.
- **No character from any overlay or fixture appears here**: classes, geometry, counts and public asset ids only.

## Line by line: the 56 qualifying lines at native

Rises are in cells of the line's own pitch; the gap in pixels is the gap times the pitch. The floor is at `f` 0.02.

| Document (fixture stem) | Line | Pitch, px per cell | Digits | Letters | Gap at `f` 0.02, cells | Gap, px | Margin at 0.02 | AUC at 0.02 | AUC at 0.05 | Floor at 0.02, px per cell |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `Afghanistan_Passport_Specimen_PO_AFG_2016_mrz` | 2 | 11.8 | 23 | 5 | +0.052 | +0.61 | -0.005 | 0.948 | 0.948 | none |
| `Argentina_Passport_Specimen_P0_ARG_2021_mrz_child` | 2 | 14.8 | 32 | 7 | +0.065 | +0.96 | -0.086 | 0.969 | 0.951 | none |
| `Argentina_Passport_Specimen_P0_ARG_2026_mrz` | 2 | 11.7 | 33 | 7 | +0.088 | +1.03 | -0.063 | 0.948 | 0.948 | none |
| `Azerbaijan_Passport_Specimen_PC_AZE_2022_mrz_black_and_white` | 2 | 8.7 | 28 | 9 | +0.105 | +0.91 | -0.019 | 0.992 | 0.988 | none |
| `Belgium_ID_Specimen_2021_back_mrz` | 1 | 26.4 | 13 | 5 | +0.088 | +2.33 | +0.045 | 1.000 | 1.000 | 9 |
| `Belgium_ID_Specimen_2021_back_mrz` | 2 | 26.4 | 26 | 4 | +0.074 | +1.95 | +0.028 | 1.000 | 1.000 | 16 |
| `Bosnia_and_Herzegovina_ID_Specimen_2013_back_mrz` | 1 | 9.5 | 14 | 7 | +0.050 | +0.47 | +0.002 | 1.000 | 0.980 | native only |
| `Bosnia_and_Herzegovina_ID_Specimen_2013_back_mrz` | 2 | 9.5 | 15 | 4 | +0.070 | +0.66 | +0.007 | 1.000 | 1.000 | native only |
| `Canada_Passport_Specimen_PP_CAN_2023_mrz_highlight` | 2 | 12.2 | 23 | 7 | +0.075 | +0.92 | -0.075 | 0.832 | 0.932 | none |
| `Canada_Passport_Specimen_PP_CAN_2023_mrz_wide` | 2 | 9.5 | 23 | 7 | +0.079 | +0.75 | -0.052 | 0.913 | 0.876 | none |
| `Cetis_Sample_Passport_Specimen_P0_TRC_2022_mrz_inner_page` | 2 | 34.7 | 24 | 6 | +0.064 | +2.22 | -0.015 | 0.965 | 0.944 | none |
| `China_Passport_Specimen_2012_mrz` | 2 | 16.4 | 26 | 18 | +0.067 | +1.10 | +0.040 | 1.000 | 1.000 | 4 |
| `Colombia_Passport_Specimen_PP_COL_2026_mrz` | 2 | 27.0 | 30 | 8 | +0.081 | +2.18 | +0.049 | 1.000 | 1.000 | 8 |
| `Croatia_ID_Specimen_2021_back_mrz` | 1 | 22.5 | 10 | 5 | +0.077 | +1.74 | +0.060 | 1.000 | 1.000 | 4 |
| `Croatia_ID_Specimen_2021_back_mrz` | 2 | 22.5 | 16 | 3 | +0.087 | +1.97 | +0.080 | 1.000 | 1.000 | 6 |
| `Croatia_Passport_Specimen_P0_HRV_2009_mrz` | 2 | 12.0 | 26 | 4 | +0.101 | +1.22 | -0.063 | 0.962 | 0.913 | none |
| `Cyprus_Passport_Specimen_P0_CYP_2020_mrz` | 2 | 12.2 | 25 | 5 | +0.079 | +0.96 | -0.061 | 0.944 | 0.936 | none |
| `Cyprus_Passport_Specimen_PP_CYP_2026_mrz` | 2 | 14.6 | 34 | 6 | +0.092 | +1.34 | -0.106 | 0.877 | 0.819 | none |
| `Czechia_Passport_Specimen_P0_CZE_2005_mrz` | 2 | 12.2 | 35 | 4 | +0.083 | +1.02 | -0.008 | 0.971 | 0.971 | none |
| `Djibouti_Passport_Specimen_P0_DJI_2017_mrz_partly_censored` | 2 | 11.0 | 24 | 6 | +0.090 | +0.99 | -0.040 | 0.882 | 0.875 | none |
| `Dominican_Republic_Passport_Specimen_P0_DOM_2020_mrz` | 2 | 10.6 | 35 | 6 | +0.087 | +0.93 | -0.044 | 0.962 | 0.943 | none |
| `Estonia_Passport_Specimen_P0_EST_2020_mrz` | 2 | 12.2 | 35 | 6 | +0.070 | +0.85 | +0.023 | 1.000 | 0.981 | 11 |
| `France_ID_Specimen_2020_back_mrz` | 1 | 136.3 | 3 | 12 | +0.051 | +6.90 | +0.016 | 1.000 | 0.750 | 13 |
| `France_ID_Specimen_2020_back_mrz` | 2 | 136.4 | 15 | 4 | +0.073 | +9.95 | +0.022 | 1.000 | 0.967 | 13 |
| `Germany_Passport_Specimen_P0_D00_2018_mrz` | 2 | 15.9 | 27 | 6 | +0.065 | +1.03 | +0.000 | 1.000 | 0.932 | native only |
| `India_Passport_Specimen_P0_IND_2013_mrz_boxed` | 2 | 34.9 | 23 | 5 | +0.102 | +3.58 | +0.050 | 1.000 | 1.000 | 9 |
| `India_Passport_Specimen_P0_IND_2022_mrz` | 2 | 19.4 | 37 | 5 | +0.106 | +2.07 | -0.081 | 0.881 | 0.881 | none |
| `India_Passport_Specimen_P0_IND_2023_mrz` | 2 | 16.5 | 37 | 5 | +0.093 | +1.54 | -0.086 | 0.881 | 0.951 | none |
| `Indonesia_Passport_Specimen_P0_IDN_2011_mrz` | 2 | 36.6 | 23 | 5 | +0.064 | +2.33 | +0.006 | 1.000 | 1.000 | 8 |
| `Nepal_Passport_Specimen_P0_NPL_2019_mrz` | 2 | 10.8 | 36 | 4 | +0.066 | +0.71 | -0.016 | 0.986 | 0.972 | none |
| `Nigeria_Passport_Specimen_P0_NGA_2022_mrz` | 2 | 12.8 | 36 | 5 | +0.048 | +0.62 | -0.151 | 0.822 | 0.806 | none |
| `Russian_Federation_ID_Specimen_2013_back_mrz` | 1 | 18.3 | 10 | 5 | +0.069 | +1.26 | +0.031 | 1.000 | 1.000 | 16 |
| `Russian_Federation_ID_Specimen_2013_back_mrz` | 2 | 18.4 | 15 | 4 | +0.101 | +1.87 | +0.075 | 1.000 | 1.000 | 7 |
| `Russian_Federation_Passport_Specimen_P0_RUS_2019_mrz` | 2 | 16.1 | 26 | 4 | +0.114 | +1.83 | -0.180 | 0.779 | 0.817 | none |
| `Serbia_ID_Specimen_2008_back_with_mrz` | 1 | 10.8 | 23 | 5 | +0.048 | +0.52 | -0.020 | 0.957 | 0.922 | none |
| `Serbia_ID_Specimen_2008_back_with_mrz` | 2 | 10.8 | 15 | 4 | +0.070 | +0.76 | +0.015 | 1.000 | 0.950 | native only |
| `Serbia_Passport_Specimen_P0_SRB_2012_mrz` | 2 | 21.1 | 39 | 4 | +0.075 | +1.57 | +0.024 | 1.000 | 0.987 | 11 |
| `Slovakia_Passport_Specimen_P0_SVK_2005_mrz` | 2 | 11.8 | 24 | 5 | +0.041 | +0.48 | +0.004 | 1.000 | 0.983 | 8 |
| `Slovakia_Passport_Specimen_PS_SVK_2014_mrz` | 2 | 12.0 | 34 | 6 | +0.048 | +0.57 | -0.003 | 0.985 | 0.882 | none |
| `Slovenia_ID_Specimen_2022_back_mrz` | 1 | 30.2 | 21 | 7 | +0.077 | +2.32 | +0.036 | 1.000 | 1.000 | 11 |
| `Slovenia_ID_Specimen_2022_back_mrz` | 2 | 30.2 | 15 | 4 | +0.067 | +2.02 | +0.002 | 1.000 | 0.983 | 20 |
| `Somalia_Passport_Specimen_P0_SOM_2023_mrz` | 2 | 14.0 | 39 | 5 | +0.093 | +1.29 | +0.014 | 1.000 | 0.959 | 9 |
| `Somaliland_Passport_Specimen_P0_RSL_2023_mrz_non_ISO` | 2 | 9.5 | 32 | 5 | +0.072 | +0.68 | -0.075 | 0.887 | 0.931 | none |
| `Spain_Passport_Specimen_P0_ESP_2013_mrz` | 2 | 12.3 | 35 | 8 | +0.054 | +0.67 | -0.042 | 0.932 | 0.921 | none |
| `Spain_Passport_Specimen_P0_ESP_2015_mrz_highlight` | 2 | 12.1 | 33 | 8 | +0.048 | +0.58 | -0.030 | 0.970 | 0.981 | none |
| `Sweden_ID_Specimen_2022_back_mrz` | 1 | 18.1 | 16 | 7 | +0.069 | +1.24 | -0.009 | 0.991 | 0.991 | none |
| `Sweden_ID_Specimen_2022_back_mrz` | 2 | 18.1 | 15 | 4 | +0.068 | +1.23 | +0.030 | 1.000 | 1.000 | 13 |
| `Switzerland_ID_Specimen_2023_back_mrz` | 1 | 57.6 | 6 | 8 | +0.079 | +4.53 | +0.048 | 1.000 | 1.000 | 8 |
| `Switzerland_ID_Specimen_2023_back_mrz` | 2 | 57.6 | 15 | 4 | +0.092 | +5.31 | +0.072 | 1.000 | 1.000 | 4 |
| `Turkiye_ID_Specimen_2020_back_mrz` | 1 | 19.1 | 19 | 6 | +0.054 | +1.03 | -0.012 | 0.912 | 0.851 | none |
| `Turkiye_ID_Specimen_2020_back_mrz` | 2 | 19.1 | 15 | 4 | +0.071 | +1.37 | +0.013 | 1.000 | 0.950 | 16 |
| `Turkiye_Passport_Specimen_P0_TUR_2010_mrz` | 2 | 9.8 | 31 | 5 | +0.090 | +0.88 | +0.024 | 1.000 | 0.948 | native only |
| `Turkiye_Passport_Specimen_P0_TUR_2025_mrz` | 2 | 17.5 | 26 | 5 | -0.003 | -0.06 | -0.055 | 0.354 | 0.346 | none |
| `United_Arab_Emirates_Passport_Specimen_P0_ARE_2011_mrz` | 2 | 21.3 | 22 | 7 | +0.056 | +1.20 | -0.006 | 0.994 | 0.968 | none |
| `United_Kingdom_Passport_Specimen_P0_GBR_2021_mrz` | 2 | 9.6 | 26 | 4 | +0.065 | +0.62 | +0.026 | 1.000 | 1.000 | 6 |
| `Uzbekistan_Passport_Specimen_P0_UZB_2013_mrz` | 2 | 10.5 | 38 | 6 | +0.049 | +0.52 | -0.108 | 0.737 | 0.781 | none |

## The 24 `K` cells

| Document (fixture stem) | Line | Column | Pitch, px per cell | Left quarter at native |
|---|---:|---:|---:|---:|
| `Slovenia_ID_Specimen_2022_back_mrz` | 1 | 28 | 30.2 | 0.130 |
| `Turkiye_Passport_Specimen_P0_TUR_2010_mrz` | 1 | 9 | 9.8 | 0.327 |
| `Estonia_Passport_Specimen_P0_EST_2020_mrz` | 1 | 18 | 12.2 | 0.343 |
| `United_Kingdom_Passport_Specimen_P0_GBR_2021_mrz` | 1 | 17 | 9.6 | 0.348 |
| `Uzbekistan_Passport_Specimen_P0_UZB_2013_mrz` | 1 | 21 | 10.5 | 0.353 |
| `Russian_Federation_Passport_Specimen_P0_RUS_2019_mrz` | 1 | 14 | 16.2 | 0.373 |
| `Estonia_Passport_Specimen_P0_EST_2020_mrz` | 2 | 0 | 12.2 | 0.374 |
| `India_Passport_Specimen_P0_IND_2013_mrz_boxed` | 1 | 13 | 34.9 | 0.390 |
| `Slovakia_Passport_Specimen_P0_SVK_2005_mrz` | 1 | 4 | 11.7 | 0.398 |
| `Slovakia_Passport_Specimen_PS_SVK_2014_mrz` | 1 | 4 | 12.0 | 0.402 |
| `Bosnia_and_Herzegovina_ID_Specimen_2013_back_mrz` | 3 | 0 | 9.5 | 0.411 |
| `Slovakia_Passport_Specimen_PS_SVK_2014_mrz` | 2 | 12 | 12.0 | 0.412 |
| `Germany_Passport_Specimen_P0_D00_2018_mrz` | 1 | 9 | 15.9 | 0.413 |
| `China_Passport_Specimen_2012_mrz` | 2 | 30 | 16.4 | 0.426 |
| `Slovakia_Passport_Specimen_P0_SVK_2005_mrz` | 2 | 12 | 11.8 | 0.429 |
| `Russian_Federation_ID_Specimen_2013_back_mrz` | 3 | 3 | 18.5 | 0.434 |
| `Estonia_Passport_Specimen_P0_EST_2020_mrz` | 1 | 16 | 12.2 | 0.439 |
| `Turkiye_Passport_Specimen_P0_TUR_2025_mrz` | 1 | 9 | 17.5 | 0.444 |
| `Russian_Federation_ID_Specimen_2013_back_mrz` | 3 | 17 | 18.5 | 0.453 |
| `Turkiye_Passport_Specimen_P0_TUR_2025_mrz` | 1 | 20 | 17.5 | 0.456 |
| `Indonesia_Passport_Specimen_P0_IDN_2011_mrz` | 1 | 9 | 36.7 | 0.461 |
| `United_Arab_Emirates_Passport_Specimen_P0_ARE_2011_mrz` | 2 | 1 | 21.3 | 0.468 |
| `United_Arab_Emirates_Passport_Specimen_P0_ARE_2011_mrz` | 2 | 3 | 21.3 | 0.468 |
| `United_Arab_Emirates_Passport_Specimen_P0_ARE_2011_mrz` | 1 | 7 | 21.3 | 0.474 |

## The 21 refused documents

All 21 are `no_zone`: the locator found fewer regular lines of exactly the format's width than the format has. The
cause column is this note's reading of the file name and the image size; only Poland 2023 was examined (#671).

| Document (file) | Shape | Image, px | Regular lines found | Cause offered here |
|---|---|---|---:|---|
| `Angola_Passport_Specimen_PN_AGO_2026_mrz_rotated_counterclockwise_90_deg.png` | 2x44 | 852 x 629 | 0 | rotated page (by file name) |
| `Argentina_Passport_Specimen_P0_ARG_2026_mrz_blur.jpg` | 2x44 | 282 x 206 | 1 | under 320 px wide |
| `Bangladesh_Passport_Specimen_P0_BGD_2023_mrz_overwritten_with_font.png` | 2x44 | 857 x 692 | 1 | not examined |
| `Canada_Passport_Specimen_P0_CAN_2013_mrz.jpg` | 2x44 | 520 x 729 | 1 | not examined |
| `Cyprus_Passport_Specimen_P0_CYP_2010_mrz.png` | 2x44 | 272 x 182 | 0 | under 320 px wide |
| `Djibouti_Passport_Specimen_P0_DJI_2019_mrz.jpg` | 2x44 | 465 x 430 | 1 | not examined |
| `Germany_Passport_Specimen_P0_D00_2024_mrz.jpg` | 2x44 | 665 x 472 | 0 | not examined |
| `Ghana_Passport_Specimen_P0_GHA_2019_mrz.jpg` | 2x44 | 309 x 214 | 0 | under 320 px wide |
| `Hong_Kong_Passport_Specimen_P0_HKG_2007_mrz.png` | 2x44 | 267 x 181 | 0 | under 320 px wide |
| `Hong_Kong_Passport_Specimen_P0_HKG_2019_mrz.png` | 2x44 | 265 x 181 | 0 | under 320 px wide |
| `India_Passport_Specimen_P0_IND_2013_mrz.jpg` | 2x44 | 293 x 199 | 0 | under 320 px wide |
| `Indonesia_Passport_Specimen_P0_IDN_2024_mrz_rotated.png` | 2x44 | 537 x 730 | 0 | rotated page (by file name) |
| `Italy_ID_Specimen_2022_back_mrz.jpg` | 3x30 | 637 x 403 | 0 | not examined |
| `Korea_Republic_of_Korea_Passport_Specimen_PM_KOR_2020_mrz.jpg` | 2x44 | 593 x 843 | 0 | not examined |
| `Korea_Republic_of_Korea_Passport_Specimen_PM_KOR_2022_mrz.png` | 2x44 | 593 x 843 | 0 | not examined |
| `Mauritania_Passport_Specimen_P0_MRT_2010_mrz.png` | 2x44 | 1652 x 1056 | 0 | not examined |
| `Pakistan_Passport_Specimen_P0_PAK_2024_mrz_rotated.png` | 2x44 | 531 x 765 | 0 | rotated page (by file name) |
| `Poland_Passport_Specimen_P0_POL_2023_mrz.jpg` | 2x44 | 547 x 365 | 0 | fillers touch (examined in #671) |
| `Romania_Passport_Specimen_PE_ROU_2024_mrz.jpg` | 2x44 | 2100 x 1468 | 0 | not examined |
| `Switzerland_ID_Specimen_ID_CHE_2003_back_mrz.jpg` | 3x30 | 232 x 146 | 2 | under 320 px wide |
| `Turkiye_Passport_Specimen_P0_TUR_2024_mrz.webp` | 2x44 | 467 x 347 | 0 | not examined |
