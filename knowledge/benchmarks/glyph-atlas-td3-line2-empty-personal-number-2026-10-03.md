# Glyph atlas on TD3 line 2 with an empty personal number, 100 seeds on the laptop: 98.9% of clean filler cells read as themselves, and a filler read as `0` or `K`, which the check digits cannot see, wins in 233 of 75,600 degraded renders and in none of 1,400 clean ones

**Date:** 2026-10-03 · **MAIN:** `7efeac8` ([#690](https://github.com/ruledicaprio/SynthPass/pull/690)'s squash commit, which adds `glyph_atlas --no-personal-number`). The run was built from **`bc0dd4c`**, #690's head when the sweep started. `git diff bc0dd4c 7efeac8 -- crates Cargo.lock` is empty, so the binary measured behaves as `main`'s; the summary header says `dirty: true`, which is the tree's untracked files only, the tracked tree was clean · **DATA:** none (generated renders, seeds 0-99; the vendored font `crates/synthpass-gen/fonts/ocr-b.ttf`, sha256 `367d876cca94`; models `f15cfb56bd02` (detection) and `e484866d4cce` (recognition), both pinned by `synthpass_ocr::verify`) · **Evidence:** Observed on synthetic renders, vendored font, oracle crop · **Status:** current

**2026-10-03.** [#685](https://github.com/ruledicaprio/SynthPass/pull/685) swept TD3 line 2's document number (cells 0-8)
and personal number (cells 28-41) and found no filler in the personal number, because the generator fills all 14
cells; so the filler-to-letter confusion of [#537](https://github.com/ruledicaprio/SynthPass/issues/537) had n = 0 in the
reverse direction. Real passports often leave the personal number empty: 14 fillers, and a `0` in the check digit of
cell 42. On that field the check digits are **blind to exactly #537's substitution**: the ICAO values
([`char_value`](../../crates/mrz/src/checksum.rs)) are `<` = 0, `0` = 0, `A` = 10, `K` = 20 and `U` = 30, and the
weights 7, 3 and 1 share no factor with 10, so a `<` read as `0`, `A`, `K` or `U` changes no weighted sum mod 10 and
leaves cell 42 and the composite in cell 43 valid. Any other glyph changes a sum, so a read of it is visible to them
(derived from the table above, not measured). How often a filler in that field is read as one of the four is the measurable form of
#537's risk. This run asks it with `glyph_atlas --no-personal-number` (#690), which turns the generator's existing
`include_personal_number` off: the personal number is the generator's last draw, so no other field of any seed moves
(cells 0-8 hold the same truths as in #685, seed by seed). The atlas reads the oracle line crop, bypasses imageprep,
the retry loop and the grid fit, and records no decoded text. It says nothing about real documents and cannot
reproduce the real pass-02 event, only place it in context. No code, ADR, baseline or default changes with this note.

**The run.**

| | |
|---|---|
| Machine | win11-i7-1255U, 12 logical CPUs, Windows, AC power |
| `RTEN_NUM_THREADS` | 4; one run at a time under `slot.ps1` |
| Tool | `glyph_atlas` release build of `bc0dd4c`, `schema` 3, ocrs 0.13.1, rten 0.26.0 |
| Preflight | `--axes blur --seeds 2`, the same cells: exit 0, 20 computed renders in 15.4 s; `truths`: cells 28-41 only `<`, cell 42 only `0` |
| Command | `glyph_atlas --format td3 --line 2 --no-personal-number --cells 0,1,2,3,4,5,6,7,8,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43 --axes resolution,jpeg,rotation,blur,noise,contrast --seeds 100 --seed-start 0` |
| Time | 12:45:29Z to 13:50:14Z, exit 0; 5,500 computed renders (5,900 rendered steps) in 3,882.9 s |
| Output | 19,057 summary rows, one per (cell, truth glyph, axis, step); `records.jsonl` is per-seed and stays local |

A first start of the same command at 11:49:44Z was stopped by hand at about 55 minutes, before it wrote a summary;
its partial `records.jsonl` (seeds 0-83, 127,494 records) was kept locally and is used once below, as a repeat-run
check. The run's 3,883 s against #685's 3,113 s for 5,500 renders is slower; the cause was not investigated, and no
timing enters a result.

**Pooling.** A rate is pooled over the cells and renders named beside it, with its n and a Wilson 95% interval.
**Read as itself** is the first sweep's definition: the winner of the CTC distribution averaged over the cell's
timesteps, blank excluded, equals the truth glyph, over every label the model has. A **floor** is the most severe step
still read as itself in at least 90 of 100 renders, going down from the clean render without a failing step. The 54
degraded steps are the six axes' steps without their identity steps. **Blind** in this note means a `<` read as `0`,
`A`, `K` or `U`.

## The answer

1. **Truth composition (Observed).** Cells 28-41: 1,400 clean renders, **all `<`**. Cell 42: 100 of 100 are `0`.
   Cell 43: five digits, the even ones (0, 2, 4, 6, 8: 18, 25, 23, 17 and 17 of the 100 seeds), which is derived, not
   observed: the weights 7, 3 and 1 are odd, so each check digit has the parity of its field's value sum, every field
   plus its check digit sums to an even number, and the composite check digit of a valid TD3 line 2 is always even. Cells 0-8: 631 letter and 269 digit truths at the clean step, **the same multiset as #685's**, cell by
   cell. The preflight's truths matched, so the brief's expectation held this time.
2. **The filler read as something else.** At the clean render **1,385 of 1,400 cells read as `<`** (98.9%, Wilson
   98.2-99.3); the 15 misses are `?` 11, `5` 2, `3` 1 and `8` 1, all in cells 28-33 (cell 28 has 6). Over the degraded
   steps 7,625 of 75,600 renders are wrong (10.1%). **Three glyphs take 80.6% of the wrong winners: `E` 3,596, `C` 1,734
   and lower-case `c` 814**; then `?` 777, **`K` 196**, `5` 145, `G` 68, `g` 39, a space 39, **`0` 37**, `8` 36. The
   axes split them (the table below lists every step):
   - **blur** 4,362 of 12,600 wrong (34.6%): the filler is lost at 4 sigma and above (4,198 of 4,200 renders wrong,
     `E` first, then `C` and `c`) and is intact to 3 sigma in cells 30-41 (cell 28: lost at 0.5 sigma, 78 of 100 read);
   - **resolution** 1,916 of 15,400 (12.4%): lost at 5 px per cell (606 of 1,400, `C`) and 4 px (1,268, `C` and `c`);
   - **rotation** 539 of 11,200 (4.8%): `E` 309 and `K` 174, from 2 degrees up;
   - **contrast** 563 of 11,200 (5.0%), mostly `?` (411); **noise** 206 of 9,800 (2.1%), mostly `5` (87) and `?`;
     **jpeg** 39 of 15,400 (0.3%), nearly all `?`.

   **Floors** (table below): resolution to 6 px per cell in cells 29-37 and to 5 px in cells 38-41 (cell 28: to 15);
   jpeg holds at every step in all 14 cells and noise in 12 of the 14 (cells 28 and 29 do not); blur to 3 sigma in cells 30-41 (cell 29: 2.5, cell
   28: none below the clean render); rotation from 1.5 degrees to every step, cell by cell. Cell 28 is the weakest on
   five of the six axes (answer 4).
3. **The check-blind substitutions.** **233 of 75,600 degraded renders (0.31%, Wilson 0.27-0.35) read a `<` as `0` or
   `K`**, against 7,392 other wrong winners: **3.1% of the wrong winners are invisible to cell 42 and the composite,
   96.9% are visible**. `K` 196, `0` 37, **`A` and `U` never**. At the clean render: 0 of 1,400 (all 15 clean misses
   are visible: `?`, `5`, `3`, `8`). Per axis:

   | Axis | Renders (cells 28-41) | Wrong | Blind (share of renders, Wilson 95%) | Which |
   |---|---:|---:|---|---|
   | resolution | 15,400 | 1,916 | 21 (0.14%, 0.09-0.21) | `0` 21, all at 4 px |
   | jpeg | 15,400 | 39 | 1 (0.01%, 0.00-0.04) | `K` 1 |
   | **rotation** | 11,200 | 539 | **176 (1.57%, 1.36-1.82)** | `K` 174, `0` 2 |
   | blur | 12,600 | 4,362 | 15 (0.12%, 0.07-0.20) | `0` 14, `K` 1 |
   | noise | 9,800 | 206 | 3 (0.03%, 0.01-0.09) | `K` 3 |
   | contrast | 11,200 | 563 | 17 (0.15%, 0.09-0.24) | `K` 17, from 0.4 to 0.1 |
   | **All** | **75,600** | **7,625** | **233 (0.31%, 0.27-0.35)** | `K` 196, `0` 37 |

   **Where they occur:** 43 (cell, axis, step) triples hold one. **Rotation dominates: 129 of the 233 are at 5 degrees
   (129 of 1,400 renders, 9.2%, all `K`), 29 at 2 degrees, 14 at 4**, none below 1.5. The `K`s at rotation sit in cells
   28-32 (138 of 174: cell 29 alone 60) and in cells 37, 40 and 41 (30); the `0`s sit in cell 28 (resolution 4 px: 20 of
   the 21; blur 2 to 4 sigma: 12; **rotation at 2 degrees: the 2 on that axis**) with 3 more in cells 29 (blur, 2) and 40
   (resolution 4 px, 1). By cell: 28: 67, 29: 62, 30: 37, 32: 20, 37: 13, 41:
   11, 31: 10, 40: 7, 33: 6; **cells 34-36, 38 and 39: none**. Cell 28's share of its own wrong winners is 5.7% blind
   (67 of 1,173), cells 29-41's is 2.6% (166 of 6,452). The reverse direction exists too: **cell 42's `0` read as `<` 157
   times** (answer 5), also blind, since `<` and `0` are both 0.
4. **The first filler against the run.** **Cell 28, right after the expiry check digit in cell 27, is the weakest filler
   cell in every comparison, as the first `<` after a letter was on line 1 ([#688](glyph-atlas-td3-line1-names-2026-10-03.md)).**
   Clean: 94 of 100 (94.0%, 87.5-97.2) against 1,291 of 1,300 (99.3%, 98.7-99.6) for cells 29-41 (z -5.0). Degraded:
   4,227 of 5,400 (78.3%, 77.2-79.4) against 63,748 of 70,200 (90.8%, 90.6-91.0). **It is lower in 31 of the 54
   degraded steps at |z| of 2 or more (about 3 would reach it by chance), and higher in none**: blur 3 sigma (41 of 100
   read against 1,279 of 1,300), rotation 5 degrees (39 against 1,076 of 1,300), resolution 5 px (12 against 782), every
   contrast step from 0.8 down (55 to 75 of 100 against 96% or more). Its wrong winners differ too: `?` 378 of 1,173
   (32%; 6% in cells 29-41), `E` 238, `C` 184, `5` 95. Over cells 28 to 41 the degraded rate climbs from 78.3% to 93.6%
   with one reversal (cell 35, 90.6% after cell 34's 91.1%); at the clean render cells 34-41 are 100 of 100 (Observed).
   What makes the first filler hard on line 2 is not tested; Hypothesized, that the model reads the boundary between
   the digit run and the filler run least well.
5. **Cells 42 and 43.** Both read as themselves at **100 of 100** at the clean render. Degraded: **cell 42 4,816 of
   5,400 (89.2%, 88.3-90.0), cell 43 4,871 of 5,400 (90.2%, 89.4-91.0)**; the wrong renders are on two axes (cell 42:
   rotation 294 of 800, blur 288 of 900; cell 43: rotation 247, blur 265; resolution gives 2 and 15, jpeg, noise and
   contrast at most 1). **Cell 42's wrong winners on rotation are `<` 157, `?` 88, `R` 20, `G` 9; on blur `D` 179, `N`
   53, `S` 37, `R` 11.** The 157 `<` are a `0` read as the filler, blind to the check digit it is the check digit of;
   all 157 are at rotation 3 to 5 degrees, where cell 42 is wrong in 83, 100 and 100 of 100 renders. Cell 43's wrong
   winners are letters and punctuation mostly (`S` 118, `E` 107, `N` 78, `s` 28, `,` 22), only 52 of 529 a digit. Floors:
   cell 42 holds to 2 degrees and 3 sigma, cell 43 to 1.5 degrees and 3 sigma; both hold at every step of the other
   four axes.
6. **The paired control against #685** (same seeds 0-99, same machine; cells 0-8 hold the same truths): **the reads
   are not identical, and the difference is small in total.** Clean: 850 of 900 (94.4%, 92.8-95.8) against 855 of 900
   (95.0%, 93.4-96.2). Degraded: 42,922 of 48,600 (88.31%) against 42,934 (88.34%), **12 renders apart**. 380 of the 531
   (cell, axis, step) triples differ, by at most 9 renders, in both directions (a cell's net over all steps runs from
   -45 to +36). Cell 0 lost the most: its net over all steps is -45 renders, the largest drop of the nine cells (95
   against 97 at the clean render, 4,743 against 4,778 of 5,400 degraded (87.8% against 88.5%)); cells 1-8: 38,179 against 38,156
   of 43,200. **A repeat of this very command on the same machine is identical:** the partial first run's 127,494
   records equal the finished run's, all of them, timing aside, so run-to-run noise is not what moves the reads
   (Observed). The only input that differs from #685 is the rest of the line, so it is a **line-context effect**
   (Observed: the difference exists; Hypothesized, untested: the CTC distribution over a line whose tail is fillers
   gives the cells before it slightly different scores). Two consequences:
   - **The cell 0 pattern of #685 comes back.** Cell 0 against cells 1-8, over the 54 degraded steps: lower at 7 steps
     (resolution 8, 5, 4 px; rotation 1.5, 3, 4, 5 degrees; z -2.2 to -4.2) and higher at 2 (contrast 0.2, 0.15). #685's
     counted over the same steps: lower at 8 (the same seven and rotation 2), higher at 3. No correction for the
     multiple steps.
   - **Floors move with the line.** 17 of the 54 (cell, axis) floors of cells 0-8 differ between the two runs, some by
     many steps (cell 4's resolution floor 22 here, 7 in #685; cell 3's rotation floor 3 here, 0.5 there), because a
     floor needs 90 of 100 at every step down from the clean render and one dip of two or three renders ends it. A floor
     of this tool is not stable to the choice of the other cells on the line (Observed), so none of the floors in this
     note is a limit.
   - **The filler as a winner on the document number:** on letter truths in cells 0-8, `<` wins **3 of 34,074 degraded
     renders** here (all `K`s, in cell 0: resolution 5 px, blur 5 and 6 sigma), against 1 of 34,074 in #685. The count
     is small in both; the direction of #537 (a `K` read as the filler) is real in both runs and rare in both.
7. **The class split.** Cells 28-41 have the class `Any`, so a wrong winner leaves the class only when it is outside the
   MRZ set: **1,715 of the 7,625 wrong winners (22.5%)**, `c` 814, `?` 777, `g` 39, a space 39, `e` 33, `(` 6, `s` 3,
   `*` 3, `x` 1; the other 5,910 are MRZ characters, which the class cannot reject (5,637 letters, 273 digits). Cells 0-8:
   1,366 of 5,678 wrong (24.1%) outside the set. Cell 42 is `DigitOrFiller`: 159 of its 584 wrong winners (27.2%) are in
   class (the 157 `<` and two digits), so the class rejects the rest; cell 43 is `Digit`: 52 of 529 (9.8%) in class.
   The class therefore catches most wrong reads of the check digits, and none of the blind ones in cells 28-41 (a `0` or `K` is an MRZ character,
   and is class-valid there). That `0` read as `<` in cell 42 is in class is Observed.

## Cell by cell: the clean render

| Cell | Field | Clean (Wilson 95%) | Wrong winners |
|---|---|---|---|
| 0 | document number 1 | 95 / 100 [0.888, 0.978] | `O` 2, `2` 2, `?` 1 |
| 1 | document number 2 | 95 / 100 [0.888, 0.978] | `0` 2, `O` 1, `?` 1, `v` 1 |
| 2 | document number 3 | 92 / 100 [0.850, 0.959] | `?` 3, `0` 2, `O` 1, `@` 1, `2` 1 |
| 3 | document number 4 | 94 / 100 [0.875, 0.972] | `?` 2, `O` 1, space 1, `0` 1, `v` 1 |
| 4 | document number 5 | 96 / 100 [0.902, 0.984] | `0` 2, `O` 1, `?` 1 |
| 5 | document number 6 | 94 / 100 [0.875, 0.972] | space 2, `?` 2, `0` 2 |
| 6 | document number 7 | 95 / 100 [0.888, 0.978] | `O` 2, `?` 1, `0` 1, `2` 1 |
| 7 | document number 8 | 94 / 100 [0.875, 0.972] | `0` 2, `?` 2, `O` 1, `2` 1 |
| 8 | document number 9 | 95 / 100 [0.888, 0.978] | `0` 2, `O` 1, `$` 1, `?` 1 |
| 28 | personal number 1 (`<`) | 94 / 100 [0.875, 0.972] | `5` 2, `?` 2, `3` 1, `8` 1 |
| 29 | personal number 2 (`<`) | 98 / 100 [0.930, 0.994] | `?` 2 |
| 30 | personal number 3 (`<`) | 98 / 100 [0.930, 0.994] | `?` 2 |
| 31 | personal number 4 (`<`) | 98 / 100 [0.930, 0.994] | `?` 2 |
| 32 | personal number 5 (`<`) | 98 / 100 [0.930, 0.994] | `?` 2 |
| 33 | personal number 6 (`<`) | 99 / 100 [0.946, 0.998] | `?` 1 |
| 34-41 | personal number 7-14 (`<`) | 100 / 100 each [0.963, 1.000] | none |
| 42 | personal number check digit (`0`) | 100 / 100 [0.963, 1.000] | none |
| 43 | composite check digit | 100 / 100 [0.963, 1.000] | none |

## Cells 28-41 per step: wrong renders of 1,400, and the two commonest winners

Each step pools 14 cells of 100 renders. A step with no entry has no wrong render.

- **resolution (px per cell):** 22: 15 (`?` 11, `5` 2) · 18: 5 · 15, 11, 10, 9, 8, 7: 0 · 13: 19 (`?` 14) · 6: 18
  (`?` 9, `C` 8) · **5: 606 (`C` 527, `?` 46) · 4: 1,268 (`C` 746, `c` 342)**
- **jpeg (quality):** 100: 13 · 90: 8 · 75: 3 · 60: 3 · 50: 2 · 40, 30, 15, 10: 0 · 20: 3 · 5: 7 (`?` 6, `K` 1); nearly all `?`
- **rotation (degrees):** 0: 15 · 0.25, 0.5, 1: 0 · 1.5: 10 · **2: 146 (`E` 102, `K` 27)** · 3: 7 · 4: 91 (`E` 67, `K` 14)
  · **5: 285 (`E` 138, `K` 129)**
- **blur (sigma, native px):** 0: 15 · 0.5: 53 (`?` 34, `g` 11) · 1: 19 · 1.5: 0 · 2: 1 · 2.5: 11 (`0` 6) · 3: 80 (`C` 41, `E` 26)
  · **4: 1,398 (`E` 1,018, `C` 378) · 5: 1,400 (`E` 1,127, `c` 230) · 6: 1,400 (`E` 938, `c` 215)**
- **noise (sigma, grey levels):** 0: 15 · 2: 26 · 4: 34 · 8: 48 (`5` 18, `?` 17) · 12: 41 (`5` 18, `E` 9) · 16: 21 · 24: 19 ·
  32: 17 (`5` 7, `4` 4)
- **contrast (ink scale):** 1: 15 · 0.8: 46 · 0.6: 55 · 0.5: 66 · 0.4: 72 · 0.3: 82 · 0.2: 87 · 0.15: 80 · 0.1: 75;
  `?` first at every step

## Floors, per cell and axis

`all steps` = none failed; `none` = the clean step already fails. Units: resolution px per cell (22 is native), JPEG
quality, rotation degrees, blur sigma in native px, noise sigma in grey levels, contrast ink scale. The jpeg axis has
no identity step: its first step is quality 100. **Read these with answer 6: a floor moves by many steps between two
runs that differ only in the rest of the line.**

| Cell | resolution | jpeg | rotation | blur | noise | contrast |
|---|---|---|---|---|---|---|
| 0 | to 9 (fails at 8) | all steps | to 0.5 (fails at 1) | to 2 (fails at 2.5) | all steps | all steps |
| 1 | to 6 (fails at 5) | all steps | to 2 (fails at 3) | to 2 (fails at 2.5) | all steps | all steps |
| 2 | to 7 (fails at 6) | all steps | to 1 (fails at 1.5) | to 2.5 (fails at 3) | all steps | all steps |
| 3 | to 15 (fails at 13) | to 10 (fails at 5) | to 3 (fails at 4) | to 2 (fails at 2.5) | all steps | all steps |
| 4 | to 22 (fails at 18) | all steps | to 1.5 (fails at 2) | to 2 (fails at 2.5) | all steps | all steps |
| 5 | to 9 (fails at 8) | all steps | to 1.5 (fails at 2) | to 2 (fails at 2.5) | all steps | all steps |
| 6 | to 6 (fails at 5) | all steps | to 1.5 (fails at 2) | to 2 (fails at 2.5) | all steps | all steps |
| 7 | to 22 (fails at 18) | to 10 (fails at 5) | to 3 (fails at 4) | to 3 (fails at 4) | all steps | to 0.4 (fails at 0.3) |
| 8 | to 6 (fails at 5) | all steps | to 0 (fails at 0.25) | to 2.5 (fails at 3) | all steps | all steps |
| 28 | to 15 (fails at 13) | all steps | to 1.5 (fails at 2) | to 0 (fails at 0.5) | to 0 (fails at 2) | to 1 (fails at 0.8) |
| 29 | to 6 (fails at 5) | all steps | to 3 (fails at 4) | to 2.5 (fails at 3) | to 8 (fails at 12) | to 0.6 (fails at 0.5) |
| 30 | to 6 (fails at 5) | all steps | to 1.5 (fails at 2) | to 3 (fails at 4) | all steps | to 0.2 (fails at 0.15) |
| 31 | to 6 (fails at 5) | all steps | to 1.5 (fails at 2) | to 3 (fails at 4) | all steps | all steps |
| 32 | to 6 (fails at 5) | all steps | to 3 (fails at 4) | to 3 (fails at 4) | all steps | all steps |
| 33 | to 6 (fails at 5) | all steps | all steps | to 3 (fails at 4) | all steps | all steps |
| 34 | to 6 (fails at 5) | all steps | to 3 (fails at 4) | to 3 (fails at 4) | all steps | all steps |
| 35 | to 6 (fails at 5) | all steps | to 4 (fails at 5) | to 3 (fails at 4) | all steps | all steps |
| 36 | to 6 (fails at 5) | all steps | all steps | to 3 (fails at 4) | all steps | all steps |
| 37 | to 6 (fails at 5) | all steps | to 4 (fails at 5) | to 3 (fails at 4) | all steps | all steps |
| 38 | to 5 (fails at 4) | all steps | all steps | to 3 (fails at 4) | all steps | all steps |
| 39 | to 5 (fails at 4) | all steps | all steps | to 3 (fails at 4) | all steps | all steps |
| 40 | to 5 (fails at 4) | all steps | all steps | to 3 (fails at 4) | all steps | all steps |
| 41 | to 5 (fails at 4) | all steps | to 1.5 (fails at 2) | to 3 (fails at 4) | all steps | all steps |
| 42 | all steps | all steps | to 2 (fails at 3) | to 3 (fails at 4) | all steps | all steps |
| 43 | all steps | all steps | to 1.5 (fails at 2) | to 3 (fails at 4) | all steps | all steps |

## Wrong winners on degraded steps

Every cell is counted over the 54 degraded steps (5,400 renders). The last column counts the wrong winners that are
`0`, `A`, `K` or `U` where the truth is `<` (cells 28-41).

| Cell | Field | Wrong | Winner is an MRZ character | Outside the MRZ set | Blind (`<` read as `0` or `K`) |
|---|---|---:|---:|---:|---:|
| 0 | document number 1 | 657 | 522 (79.5%) | 135 (20.5%) | |
| 1 | document number 2 | 594 | 475 (80.0%) | 119 (20.0%) | |
| 2 | document number 3 | 651 | 503 (77.3%) | 148 (22.7%) | |
| 3 | document number 4 | 597 | 438 (73.4%) | 159 (26.6%) | |
| 4 | document number 5 | 625 | 484 (77.4%) | 141 (22.6%) | |
| 5 | document number 6 | 589 | 402 (68.3%) | 187 (31.7%) | |
| 6 | document number 7 | 671 | 545 (81.2%) | 126 (18.8%) | |
| 7 | document number 8 | 628 | 436 (69.4%) | 192 (30.6%) | |
| 8 | document number 9 | 666 | 507 (76.1%) | 159 (23.9%) | |
| 28 | personal number 1 | 1,173 | 664 (56.6%) | 509 (43.4%) | 67 |
| 29 | personal number 2 | 732 | 522 (71.3%) | 210 (28.7%) | 62 |
| 30 | personal number 3 | 707 | 554 (78.4%) | 153 (21.6%) | 37 |
| 31 | personal number 4 | 581 | 451 (77.6%) | 130 (22.4%) | 10 |
| 32 | personal number 5 | 548 | 418 (76.3%) | 130 (23.7%) | 20 |
| 33 | personal number 6 | 519 | 416 (80.2%) | 103 (19.8%) | 6 |
| 34 | personal number 7 | 482 | 388 (80.5%) | 94 (19.5%) | 0 |
| 35 | personal number 8 | 510 | 421 (82.5%) | 89 (17.5%) | 0 |
| 36 | personal number 9 | 440 | 370 (84.1%) | 70 (15.9%) | 0 |
| 37 | personal number 10 | 416 | 360 (86.5%) | 56 (13.5%) | 13 |
| 38 | personal number 11 | 402 | 341 (84.8%) | 61 (15.2%) | 0 |
| 39 | personal number 12 | 396 | 342 (86.4%) | 54 (13.6%) | 0 |
| 40 | personal number 13 | 376 | 344 (91.5%) | 32 (8.5%) | 7 |
| 41 | personal number 14 | 343 | 319 (93.0%) | 24 (7.0%) | 11 |
| 42 | personal number check digit | 584 | 489 (83.7%) | 95 (16.3%) | |
| 43 | composite check digit | 529 | 438 (82.8%) | 91 (17.2%) | |

## What this does not show

- Nothing about real documents or the shipped pipeline; the atlas reads an oracle crop of a synthetic line with a
  vendored font. Nothing about the real Cyprus pass-02 read of #537 or its retry passes: this measures one recognition
  pass on a clean crop of a degraded render, not a retry that varies the crop. A blind read here is a `<` that the model
  scored as `0` or `K`; whether such a read reaches a decoded string, and whether a later stage keeps it, is not
  measured.
- **An empty field only.** The 14 cells are all `<`, a run the generator draws as a block. A short number followed by
  padding, with a letter or digit immediately before the filler, is not measured; cell 28 (a filler after a digit, the
  expiry check digit) is the nearest, and it is the weak one (answer 4). The 233 are a count for this block, not a
  rate for fillers in general.
- The blind set `0`, `A`, `K`, `U` is derived from `char_value` and the weights, not measured: whether each blind
  winner would in fact leave cell 42 valid also depends on cell 42 itself being read right in the same pass, which this
  atlas, reading every cell separately, does not combine.
- The pooled rates rest on 100 renders per cell and step. A single cell's rate (cell 28's 67 blind reads) rests on 5,400
  renders; the 129 `K`s at 5 degrees rest on a single step of a single axis and are indicative of where blind reads
  concentrate, not of how often rotation of that size occurs.
- The cell 0 against cells 1-8 comparison runs 54 steps with no multiple-comparison correction. The floors are not
  stable to the rest of the line (answer 6).
- One run, one machine, one thread setting. A repeat of the first 84 seeds was identical, so the spread over seeds is
  the only spread measured. The desktop's atlas runs are not set against these counts (the tool's module doc forbids
  comparing atlas runs across machines).
