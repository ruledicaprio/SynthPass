# Filler or letter per name cell: on a verified grid no letter is classed a filler, but strict names reach 17 / 45 against 15 — no-go

**Date:** 2026-10-01 · **MAIN:** `048695e` (the base of all three OCR runs; the prototype was uncommitted at run time, see Evidence and limits) · **DATA:** `396b22f` · **Evidence:** Observed (three local release OCR runs of the 45 name-scorable real specimens, one per chargrid arm, with the prototype run on their pixels; one synthetic sweep and one tuning run, both on generated TD3 renders) plus Observed by eye (grid (a) on all 32 Tier-1 name lines with pixels) plus Derived (per-cell and per-document comparison with the reviewed fixtures, and the ceiling) · **Status:** current

**2026-10-01.** This is the go/no-go note that
[#575](https://github.com/ruledicaprio/SynthPass/issues/575) item C asks for. The prototype was
a bench-only example, `filler_cells`. After this no-go the owner chose not to keep it in the tree,
so its source lives only in the history of
[PR #650](https://github.com/ruledicaprio/SynthPass/pull/650) (commit `4c88d58`). It classes each name cell
of a Tier-1 hit as `filler`, `letter`, `uncertain` or `occluded` from that cell's pixels, on two
grids kept apart, and compares the class with the reviewed fixture's zone, never with OCR. It
then applies the classes to the read and scores the names. Nothing is wired into the product, no
default changes, and [ADR-0014](../decisions/ADR-0014-per-cell-ocrb-classification.md) stays
Proposed. Documents are named by country, year and side, and cells by count. No zone text appears
here.

## The answer

**No-go.** The first gate fails and the second holds.

| Gate (#575, from ADR-0014 `:185`) | Needed | Measured | Verdict |
| --- | --- | --- | --- |
| Strict names ≥ best chargrid arm + 10 pp on the 45 name-scorable real documents | ≥ 20 / 45 (best arm 15 / 45, plus 5) | grid (a) **17 / 45**, grid (b) **15 / 45** | fails |
| No correct→wrong field on any document | 0 | 0 on grid (a), 0 on grid (b) | holds |

- **The chargrid arms, same binary, same run** (Observed): `off` 15 / 45, `control` 15 / 45, `on`
  14 / 45. `on` breaks the Serbia 2012 passport and fixes nothing. The best arm is 15 / 45
  (33.3%). Grid (a) is +2 documents (+4.4 pp), grid (b) +0.
- **No filler-or-letter decision on these reads can reach the gate** (Derived). Take every name
  cell where an edit alignment pairs a printed `<` with a read letter, and set it back to `<` from
  the fixture. That is a perfect decision, and it reaches **18 / 45**. It leaves 15 Tier-1 hits
  wrong. 9 of them carry a read shifted by a cell against the print, 3 of those also with a letter
  read as another letter. 4 more carry only a letter read as another letter, and 1 carries letters
  read as `<`. The last, the Slovenia 2022 ID card back, has no pass pixels to classify. The other
  12 of the 45 are misses. A filler-or-letter decision touches none of these errors.
- **On the hand-verified grid (a), no letter is ever classed a filler** (Observed): 0 of 517 letter
  cells. 658 of 722 filler cells are classed `filler` (91.1%), 28 `letter` and 36 `uncertain`.
- **On production's chargrid grid (b), the classifier mostly fails** (Observed): 105 of 460 filler
  cells are classed `filler` and 290 `letter`. Grid (b) exists on 20 of the 33 Tier-1 hits. On
  all 20 its cell 0 sits to the right of grid (a)'s, by 0.06 to 1.07 cells.
- **The misalignment guard is what keeps the second gate** (Observed). The prototype leaves a line
  alone when a cell it classes `letter` was read as `<`. On grid (a) the guard stopped 16 of 32
  lines. 10 of them were reads shifted against the printed cells, and without the guard grid (a)
  turns 4 exact names wrong.
- **On synthetic renders no letter is classed a filler at any step except blur sigma 6** (5 of
  1,372 cells), on held-out seeds and the exact grid (Observed). Fillers are classed `filler` on
  all 2,528 cells at every resolution from 22 to 6 px per cell except 8 and 5 px, at every
  contrast, at rotations up to 4 degrees and at blur up to sigma 2.5.

## The two grids

**Grid (a), the hand-verified grid.** It is fitted from the line's own ink: the format's cell
count, the first and last inked columns, and the pitch and origin whose cell boundaries cross the
least ink, each boundary charged the ink within 20% of a pitch of it (the prototype's `ink_grid`). The fixture
supplies nothing but the cell count, which the format already fixes. Each line was drawn with
its grid and with the fixture's filler cells marked, in two halves at about 2x, and checked by
eye. **On all 32 lines every boundary lies in the gap between two glyphs, and every marked filler
cell lies on a printed `<`** (Observed by eye). None needed correcting. The 33rd Tier-1 hit,
the Slovenia 2022 ID card back, has no grid: its zone was assembled from the page text after the
retry loop ran out, so no single pass's pixels hold it.

**Grid (b), production's fit.** This is `fit_chargrid_name_line`'s grid, as
`SYNTHPASS_OCR_CHARGRID=control` captures it on the accepted pass. The accepted pass's image was
read back from `SYNTHPASS_OCR_DUMP_VARIANTS`, resized where the capture says it was, and its band
matched the capture's `band_sha256` on all 20 lines. The other 13 hits have no grid (b): chargrid
skipped them, with `no_line_match` 7, `line_match_ambiguous` 3, `grid_fit_failed` 2 and
`no_source_image` 1.

**Grid (b) against grid (a), on the 20 lines that have both** (Derived, in cells of grid (a)'s
pitch). Cell 0's left edge is to the right of grid (a)'s on all 20, from 0.06 to 1.07 cells,
median 0.21. The pitch ratio (b)/(a) runs from 0.893 to 1.008. At the line's right end the
offset is at least a quarter of a cell on 17 of 20 lines. This note does not say why. That
question is #575 item E.

## Per cell

Name cells of the Tier-1 hits (cells 5-43 of a TD3 or MRV-A name line, cells 0-29 of a TD1 third
line), with truth from the fixture.

| Grid | Truth | → `filler` | → `letter` | → `uncertain` | → `occluded` | Cells |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| (a) hand-verified | `<` | **658** | 28 | 36 | 0 | 722 |
| (a) hand-verified | letter | **0** | 496 | 20 | 1 | 517 |
| (b) chargrid fit | `<` | 105 | 290 | 65 | 0 | 460 |
| (b) chargrid fit | letter | **0** | 300 | 11 | 0 | 311 |

Grid (b) covers fewer cells because it exists on 20 lines, not 32.

**OCR's read of the same lines, edit-aligned against the fixture** (Derived), over the 32 lines:

| Edit | Cells | Lines |
| --- | ---: | ---: |
| printed `<` read as a letter | 89 | 12 |
| printed letter read as `<` | 6 | 2 |
| printed letter read as another letter | 10 | 7 |
| cell missing from the read | 14 | 13 |
| cell added to the read | 14 | 13 |

## Per document

The 33 Tier-1 hits of the 45. The other 12 are misses in every arm, and the prototype never
touches a miss: the Afghanistan 2016, Czechia 2005, Germany 2024, Hong Kong 2007, Hong Kong 2019,
Romania 2024 and Russian Federation 2019 passports, and the Belgium 2021, Croatia 2021, France
2020, Italy 2022 and Sweden 2022 ID card backs. All of these are local results, and they match the
committed baseline's 33 hits and 15 strict documents one for one.

"Guard" is the misalignment guard (see Method). "Ceiling" is the perfect decision above. The last
two columns are the edit alignment's counts for that line.

| Document | Format | `off` | chargrid `on` | Grid (a) | Grid (b) | Ceiling | `<` read as letter | Cells missing + added |
| --- | --- | --- | --- | --- | --- | --- | ---: | ---: |
| Angola passport 2026 (rotated 90°) | TD3 | wrong | wrong | changed 3 → wrong | no grid | wrong | 2 | 2 |
| Azerbaijan passport 2022 (black and white) | TD3 | exact | exact | unchanged → exact | guard → exact | exact | 0 | 0 |
| Bangladesh passport 2023 (overprinted) | TD3 | wrong | wrong | changed 7 → wrong | no grid | wrong | 21 | 2 |
| Canada passport 2013 | TD3 | wrong | wrong | guard → wrong | guard → wrong | wrong | 0 | 2 |
| Canada passport 2023 (highlighted) | TD3 | wrong | wrong | changed 1 → wrong | guard → wrong | exact | 2 | 0 |
| Canada passport 2023 (wide crop) | TD3 | wrong | wrong | guard → wrong | no grid | wrong | 6 | 0 |
| Cetis sample passport 2022 (inner page) | TD3 | wrong | wrong | changed 18 → wrong | changed 16 → wrong | wrong | 18 | 2 |
| China passport 2012 | TD3 | exact | exact | unchanged → exact | guard → exact | exact | 0 | 0 |
| Croatia passport 2009 | TD3 | exact | exact | unchanged → exact | no grid | exact | 0 | 0 |
| Cyprus passport 2010 | TD3 | wrong | wrong | guard → wrong | no grid | wrong | 19 | 0 |
| Cyprus passport 2020 | TD3 | exact | exact | unchanged → exact | no grid | exact | 0 | 0 |
| Cyprus passport 2026 | TD3 | exact | exact | unchanged → exact | no grid | exact | 0 | 0 |
| Djibouti passport 2017 (partly censored) | TD3 | wrong | wrong | guard → wrong | guard → wrong | wrong | 13 | 2 |
| Djibouti passport 2019 | TD3 | exact | exact | guard → exact | guard → exact | exact | 0 | 0 |
| Dominican Republic passport 2020 | TD3 | wrong | wrong | guard → wrong | guard → wrong | wrong | 4 | 0 |
| Estonia passport 2020 | TD3 | wrong | wrong | **changed 1 → exact** | guard → wrong | exact | 1 | 0 |
| India passport 2022 | TD3 | wrong | wrong | changed 1 → wrong | no grid | wrong | 1 | 0 |
| India passport 2023 | TD3 | exact | exact | guard → exact | guard → exact | exact | 0 | 2 |
| Indonesia passport 2024 (rotated) | TD3 | wrong | wrong | guard → wrong | guard → wrong | wrong | 0 | 2 |
| Nepal passport 2019 | TD3 | exact | exact | unchanged → exact | no grid | exact | 0 | 0 |
| Nigeria passport 2022 | TD3 | wrong | wrong | guard → wrong | guard → wrong | wrong | 0 | 2 |
| Pakistan passport 2024 (rotated) | TD3 | exact | exact | guard → exact | guard → exact | exact | 0 | 0 |
| Serbia ID card 2008, back | TD1 | exact | exact | guard → exact | guard → exact | exact | 0 | 2 |
| Serbia passport 2012 | TD3 | exact | **wrong** | guard → exact | guard → exact | exact | 0 | 2 |
| Slovakia passport 2005 | TD3 | exact | exact | guard → exact | guard → exact | exact | 0 | 2 |
| Slovakia passport 2014 | TD3 | wrong | wrong | unchanged → wrong | no grid | wrong | 0 | 0 |
| Slovenia ID card 2022, back | — | wrong | wrong | no pass pixels | no pass pixels | wrong | — | — |
| Somalia passport 2023 | TD3 | exact | exact | unchanged → exact | guard → exact | exact | 0 | 0 |
| Somaliland passport 2023 (non-ISO) | TD3 | exact | exact | unchanged → exact | unchanged → exact | exact | 0 | 0 |
| Spain passport 2013 | TD3 | wrong | wrong | guard → wrong | no grid | wrong | 1 | 4 |
| Spain passport 2015 (highlighted) | TD3 | wrong | wrong | **changed 1 → exact** | guard → wrong | exact | 1 | 0 |
| United Arab Emirates passport 2011 | TD3 | wrong | wrong | guard → wrong | no grid | wrong | 0 | 2 |
| Uzbekistan passport 2013 | TD3 | exact | exact | guard → exact | guard → exact | exact | 0 | 0 |

| Arm | Strict names / 45 | Name correct→wrong | Any field correct→wrong |
| --- | ---: | ---: | ---: |
| `off` | 15 | — | — |
| chargrid `control` | 15 | 0 | 0 |
| chargrid `on` | 14 | 1 | 1 (the given names) |
| prototype, grid (a) | **17** | **0** | **0** |
| prototype, grid (b) | 15 | 0 | 0 |
| *counterfactual: grid (a) without the guard* | *13* | *4* | *4 (the names)* |
| *counterfactual: grid (b) without the guard* | *16* | *0* | *0* |
| *ceiling: the perfect filler-or-letter decision* | *18* | *0* | *0* |

Rows in italics are bounds and counterfactuals. They are not arms and are not gated. Without the
guard, grid (a) breaks the India 2023, Serbia 2012 and Slovakia 2005 passports and the Serbia 2008
ID card back, all of them reads shifted against the print. Grid (b) gains the Spain 2015 passport
(highlighted) and breaks nothing.

**Where the 16 guard stops on grid (a) came from** (Derived). 10 were reads shifted against the
printed cells (4 of them exact under `off`, 6 wrong). The other 6 were unshifted reads where a
filler classed `letter` or a letter read as `<` tripped the guard (3 exact, 3 wrong). None of those
3 wrong ones reaches the ceiling either, so on these 45 documents the guard cost no strict name
the ceiling allows.

**The one document between 17 and the ceiling's 18** is the Canada 2023 passport (highlighted
crop), at about 12 px per cell. Its read has two fillers read as letters, and the prototype
classes one of them `filler` and the other not. Under an earlier version of grid (a), whose
boundary cost read a single column, it classed both, and the arm reached 18 / 45. That version's
overlays also passed the eye check (see Rejected on the way).

## On synthetic renders

The sweep uses TD3 seeds 0-99, the generator's exact cell grid, and one degradation at a time on
#433's axes. It reuses the glyph atlas's steps and its rotation and contrast definitions. Name
cells 5-43 give 2,528 filler and 1,372 letter cells per step. The thresholds were chosen on
seeds 100-199, so these seeds are held out (Observed).

| Axis | Step | `<` → filler | `<` → letter | `<` → uncertain | letter → filler | letter → uncertain | letter → occluded | Lines all right / 100 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| resolution | 22 (clean) | 2,528 | 0 | 0 | 0 | 5 | 0 | 96 |
| resolution | 18 | 2,528 | 0 | 0 | 0 | 3 | 0 | 97 |
| resolution | 15 | 2,528 | 0 | 0 | 0 | 0 | 0 | 100 |
| resolution | 13 | 2,528 | 0 | 0 | 0 | 0 | 0 | 100 |
| resolution | 11 | 2,528 | 0 | 0 | 0 | 68 | 0 | 69 |
| resolution | 10 | 2,528 | 0 | 0 | 0 | 68 | 0 | 39 |
| resolution | 9 | 2,528 | 0 | 0 | 0 | 23 | 0 | 78 |
| resolution | 8 | 900 | 662 | 966 | 0 | 8 | 0 | 0 |
| resolution | 7 | 2,528 | 0 | 0 | 0 | 0 | 0 | 100 |
| resolution | 6 | 2,528 | 0 | 0 | 0 | 0 | 0 | 100 |
| resolution | 5 | 2,050 | 0 | 478 | 0 | 87 | 0 | 4 |
| resolution | 4 | 0 | 818 | 1,710 | 0 | 30 | 7 | 0 |
| contrast | 0.8 to 0.1 (8 steps) | 2,528 each | 0 | 0 | 0 | 5 each | 0 | 96 each |
| rotation | 0.25 | 2,528 | 0 | 0 | 0 | 3 | 0 | 98 |
| rotation | 0.5 | 2,528 | 0 | 0 | 0 | 4 | 0 | 97 |
| rotation | 1 | 2,528 | 0 | 0 | 0 | 2 | 0 | 98 |
| rotation | 1.5 | 2,528 | 0 | 0 | 0 | 3 | 0 | 98 |
| rotation | 2 | 2,528 | 0 | 0 | 0 | 15 | 0 | 87 |
| rotation | 3 | 2,528 | 0 | 0 | 0 | 55 | 0 | 47 |
| rotation | 4 | 2,528 | 0 | 0 | 0 | 66 | 0 | 51 |
| rotation | 5 | 2,229 | 0 | 299 | 0 | 79 | 0 | 0 |
| blur | 0.5 | 2,528 | 0 | 0 | 0 | 3 | 0 | 97 |
| blur | 1 | 2,528 | 0 | 0 | 0 | 3 | 0 | 97 |
| blur | 1.5 | 2,528 | 0 | 0 | 0 | 1 | 0 | 99 |
| blur | 2 | 2,528 | 0 | 0 | 0 | 8 | 4 | 91 |
| blur | 2.5 | 2,528 | 0 | 0 | 0 | 12 | 171 | 10 |
| blur | 3 | 2,363 | 0 | 165 | 0 | 28 | 217 | 6 |
| blur | 4 | 2,417 | 0 | 111 | 0 | 81 | 376 | 0 |
| blur | 5 | 2,523 | 0 | 5 | 0 | 128 | 525 | 0 |
| blur | 6 | 1,926 | 4 | 598 | **5** | 216 | 822 | 0 |

The contrast rows are identical at every step, because Otsu's threshold rescales with the ink.
From blur sigma 2.5 on, `occluded` mostly means a letter blurred into a blob dense enough to trip
the covered-cell guard, not a covered cell. The resolution axis is not monotonic: 8 and 5 px fail
where 7 and 6 px pass (Observed; the run does not say why).

## Method

- **Population.** These are the 45 name-scorable documents of the committed real-specimen
  baseline: outcome rows with a `names_exact` verdict in the scored population (33 hits, 10
  `checksum_failed`, 2 `no_mrz_found`). They come from the public `samples-data` corpus at
  `396b22f`, the baseline's own DATA. Cell truth is the reviewed fixture's zone
  (`samples/ocr_fixtures/<stem>.json`). The truth line and the read line always had the same
  format.
- **OCR runs.** These were three release runs of `filler_cells real`, one per arm (`off`,
  `control`, `on`), on one Linux container with 4 vCPUs. All three ran concurrently with
  `RTEN_NUM_THREADS=1` and `SYNTHPASS_OCR_MAX_SECONDS=600`, so the wall-clock retry budget could
  not cut a pass. Each document goes through `decode_image`, is written to PNG, and is read by
  `recognize_detailed_traced`, the way `provider-bench` hands a specimen to OCR. The Tier-1 read
  is `synthpass_die::read_tier1` on the page text, with the line-1 selector at its default. A hit
  is a checksum-valid read whose document number equals the fixture's. Names are scored with
  `classify_names`, as `provider-bench` scores them. The models were the pinned pair (detection
  `f15cfb56bd02…`, recognition `e484866d4cce…`). `off` and `control` agree on every document, and
  `off` agrees with the committed baseline's hit and strict verdict on every document.
- **Features.** The band is the line box padded 15% each way, then cut to its text rows: the
  contiguous rows around the inkiest row holding at least 10% of its ink. Cells are binarised with
  Otsu's threshold over the band. Per cell, `v` is the longest vertical run of ink over the line's
  90th-percentile `v`. The glyph's ink box is cut into thirds each way, and each zone's density is
  taken over the box's mean. `corner` is the denser of the two left corner zones, `apex` the
  middle-left zone, and `ends` the sparser of the two right corner zones.
- **The decision.** A cell is `occluded` when its dark fraction is ≥ 0.60 or ≥ 2.5 times the
  line's 90th-percentile cell, and `uncertain` when it is below 0.01. It is `filler` when it has
  an ink box with `corner ≤ 0.75`, `apex ≥ 1.0`, `ends ≥ 0.3` and `v ≤ 0.80`. It is `letter` when
  `corner ≥ 1.00` or `v ≥ 0.95`, and `uncertain` otherwise.
- **How each threshold was chosen.** The four filler bounds come from an exhaustive search on
  synthetic renders only: TD3 seeds 100-199 at every sweep step, 92,500 filler and 51,800 letter
  cells. The search kept the combination classing the most fillers `filler` while classing **no
  letter** `filler`: 86,821 fillers (93.9%), 0 letters. Every chosen value is inside its searched
  range. The letter bounds sit 0.25 (`corner`) and 0.15 (`v`) above the filler bounds. Those two
  margins and the three guards (0.01, 0.60, 2.5) were chosen, not fitted. **No threshold was tuned
  on the real fixtures.** The feature set was not chosen blind to them, though: see Rejected on the
  way.
- **Applying the classes.** Only the `off` read of a Tier-1 hit is touched, and only its name
  cells. A cell classed `filler` but read as a letter becomes `<`. A `letter` cell is never
  rewritten, and `uncertain` and `occluded` cells keep OCR's read (ADR-0026: a covered cell is
  never reconstructed). The **guard** leaves the whole line alone when any cell classed `letter`
  was read as `<`. A change is kept only when the re-parse is checksum-valid and every field but
  the two names equals the original read. Correct→wrong is counted on every field the fixture
  states.
- **The ceiling.** An edit alignment from the fixture's name line to the read (Levenshtein, ties
  to a diagonal step) pairs printed and read cells across a missing or added cell. Every name cell
  it pairs a printed `<` with a read letter is set to `<`, and the result is re-parsed through the
  same gate as the prototype.

## Evidence and limits

- **The runs.** All three OCR runs ran from one binary built at `048695e` plus the uncommitted
  example. The `real` subcommand was not edited after that build. The branch was then rebased
  onto `b233bef`. The two commits in between change `provider-bench`'s own hint and dump parse
  (#634) and a `tools/` script (#641). Neither touches OCR or `read_tier1`, which these runs use. The
  classifier and the analysis were revised after the OCR runs, which is possible because the runs
  keep the accepted pass's pixels, and every number here comes from the committed code. The sweep
  and the tuning set are deterministic, and a second sweep run reproduced `sweep.json`
  byte-for-byte.
- **Not CI.** This is one local run per arm, on a Linux container. CI's real-specimen gate runs
  elsewhere with other thread settings, and inference is floating-point. The local `off` arm
  matched the committed baseline on all 45 verdicts, but that is one run.
- **Kept local.** The run outputs (`artifacts/filler-cells/`: per-arm JSONL with the read zones,
  the variant dumps, the overlays) hold document text and stay on the machine. This note uses
  their counts only.
- **45 documents.** The gate is decided by single documents: +2 on grid (a), and a one-document
  sensitivity to a sub-pixel grid change on a 12 px pitch.

## What this does not show

- **That a per-cell classifier is worthless.** On a verified grid, no letter was classed a filler
  on real print or on any synthetic step but the last blur step. The no-go comes from the
  ceiling: these reads' remaining name errors are shifted cells and letter confusions, which a
  filler-or-letter decision cannot touch.
- **That grid (a) would work in the product.** It needs no fixture, but it was only checked by eye
  on 32 lines. It was never run as a production grid or measured on synthetic grids. The sweep
  uses the generator's exact grid.
- **Why grid (b) sits to the right.** The offsets are measured, not explained. That is #575
  item E.
- **ADR-0026 on real covered cells.** `occluded` fired once on real print (one letter cell). The
  `redacted_mrz` documents are not in the name-scorable population, so covered cells were not
  measured.
- **Latency, determinism across machines, and composed degradations.** None was measured. The
  synthetic sweep is TD3 only, with one font and one degradation at a time.
- **Anything about ADR-0014's own five-threshold go/no-go.** This note measures #575's narrower
  binary decision against ADR-0014's strict-name threshold, and changes nothing in the ADR.

## Rejected on the way

- **Inked height as a feature.** It was dropped before any real classification. On clean renders
  the vendored font's filler spans 0.91 of the reference height against 0.95 for the lowest
  letter. The
  [filler geometry note](ocrb-filler-geometry-2026-09-23.md) measures the printed filler at about
  one cap height.
- **Thresholds fitted on clean renders alone (`v`, `h`, `r`).** On clean renders every filler is
  the same pixels, and the bounds fitted there failed on the held-out sweep. Every filler became
  `letter` from blur sigma 1 and at 11 px per cell. On the real crops this version was run twice,
  before and after the band was cut to its text rows, before the zone features existed. At best
  156 of 722 fillers were classed `filler` (404 `letter`), and 0 of 517 letters `filler`. The zone
  features replaced it after that result had been seen. Their
  thresholds come from the synthetic search alone, but the choice of feature was not made blind to
  that one real run.
- **The horizontal run `h` as letter evidence.** Under blur, neighbouring fillers merge into one
  run (the filler's p95 is 0.64 on the tuning set), so `h` would class blurred fillers `letter`.
  It is still measured and reported by `tune`.
- **A single-column boundary cost for grid (a).** With wide gaps, many pitches cross zero ink, and
  ties went to the smallest pitch. A unit test on a regular synthetic grid caught it. The
  committed cost charges each boundary the ink within 20% of a pitch. The earlier version's
  overlays also passed the eye check. On the real lines it classed 665 / 722 fillers `filler`
  (now 658) and gave 18 / 45 strict (now 17), with the same 0 letters classed `filler` and 0
  correct→wrong.
