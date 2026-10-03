# Glyph atlas on TD3 line 2's document number and personal number, 100 seeds on the laptop: 95.0% and 90.8% of clean cells read as themselves, a letter is read as the filler once in 34,074 degraded renders of the document number, and the personal number's filler winners come from rotation at the end of the line

**Date:** 2026-10-03 · **MAIN:** `0e47197` (`origin/main` when the run started; the tool is [#673](https://github.com/ruledicaprio/SynthPass/pull/673)'s `glyph_atlas`, unchanged; the summary header says `dirty: true`, which is the tree's untracked files only, the tracked tree was clean) · **DATA:** none (generated renders, seeds 0-99; the vendored font `crates/synthpass-gen/fonts/ocr-b.ttf`, sha256 `367d876cca94`; models `f15cfb56bd02` (detection) and `e484866d4cce` (recognition), both pinned by `synthpass_ocr::verify`) · **Evidence:** Observed on synthetic renders, vendored font, oracle crop · **Status:** current

**2026-10-03.** This is the sweep that [#537](https://github.com/ruledicaprio/SynthPass/issues/537) asked for: the
glyph atlas on the cells of TD3's second MRZ line that the earlier sweeps left out, **cells 0-8 (the document number)
and 28-41 (the personal number)**. #537 is a real read in which one retry variant read an intact first
document-number cell, a `K`, as the filler; the check digits cannot see that swap, and the crop hypothesis was ruled
out on the real image. The atlas is the instrument for the recognition question. The sweep follows the line-2 notes
([#676](https://github.com/ruledicaprio/SynthPass/pull/676) at 100 seeds on the desktop,
[#683](https://github.com/ruledicaprio/SynthPass/pull/683) at 30 seeds on the laptop, cells 9-27, 42 and 43) and uses
the laptop's settings (`RTEN_NUM_THREADS=4`), so it sits beside
[#624](https://github.com/ruledicaprio/SynthPass/pull/624), [#660](https://github.com/ruledicaprio/SynthPass/pull/660)
and #683. The atlas reads the oracle line crop, bypasses imageprep, the retry loop and the grid fit, and records no
decoded text. It says nothing about real documents and cannot reproduce the real pass-02 event, only place it in
context. No code, ADR, baseline or default changes with this note.

**The run.**

| | |
|---|---|
| Machine | win11-i7-1255U, 12 logical CPUs, Windows, AC power |
| `RTEN_NUM_THREADS` | 4; one run at a time under `slot.ps1` |
| Tool | `glyph_atlas` release build, `schema` 3, ocrs 0.13.1, rten 0.26.0; `Cargo.lock` of `0e47197` |
| Preflight | `--axes blur --seeds 2`, the same cells: exit 0, 20 computed renders in 13.0 s |
| Command | `glyph_atlas --format td3 --line 2 --cells 0,1,2,3,4,5,6,7,8,28,29,30,31,32,33,34,35,36,37,38,39,40,41 --axes resolution,jpeg,rotation,blur,noise,contrast --seeds 100 --seed-start 0` |
| Time | 07:05:55Z to 07:57:49Z, exit 0; 5,500 computed renders (5,900 rendered steps) in 3,113.0 s |
| Output | 45,666 summary rows, one per (cell, truth glyph, axis, step); `records.jsonl` is per-seed and stays local |

**The truths are not what the brief for this run expected.** It expected the document number to take letters and
digits and the personal number digits and the filler `<`. The document number does take letters and digits. **The
personal number took letters and digits too, and no filler in any of the 100 seeds**: the generator fills all 14 cells,
so the filler padding of a short personal number is not in these renders. At the clean step the 900 renders of cells
0-8 are 631 letters and 269 digits, and the 1,400 renders of cells 28-41 are 1,018 letters and 382 digits, no filler.
Each cell's glyphs are listed in the summary header; every cell takes 31 to 36 of the 36 alphanumerics, and cell 0
took no `I`, `Q` or `U` in these 100 seeds (the generator draws every cell from the same 36 symbols, so about two symbols per cell are absent by chance, not excluded). Every cell has 100 renders per step, one per
seed, with no missing recognition.

**Pooling.** A cell's 100 renders are split among its glyphs, so one (cell, glyph) pair has a handful. The tables
**pool the glyphs of a cell** and say the n of every pooled rate. A rate of 100 / 100 has a Wilson 95% interval of
0.963-1.000. **Read as itself** is the first sweep's definition: the winner of the CTC distribution averaged over the
cell's timesteps, blank excluded, equals the truth glyph, over every label the model has. Both groups have the class
"any" in [`mrz::position_class`](../../crates/mrz/src/position.rs): the layout allows any MRZ character there.

## The answer

1. **Cells 0-8 at the clean render: 855 of 900 read as themselves** (95.00%, Wilson 95% 93.4-96.2). Letter truths
   601 / 631 (95.25%, 93.3-96.6), digit truths 254 / 269 (94.42%, 91.0-96.6): the two are not apart. The nine cells
   run from 92 to 97 of 100 (cell 2 lowest). Of the 45 misses, 24 are the pair `O` and `0`: `O` read as `0` in 14 of 30
   renders and `0` read as `O` in 10 of 20 (Observed).
2. **#537's confusion does not appear in the document number.** On letter truths in cells 0-8 the filler wins **1
   time in 34,074 degraded renders** (3,304 of them wrong, 9.7%), and 0 times among the 30 wrong clean renders. The one
   is a `K` in **cell 0** at 5 px per cell, in a row of 4 renders of that glyph at that step. **`K` alone:** 24 of 24
   at the clean render; over the degraded steps 1,296 renders, 64 wrong (4.9%, 45 of them on blur and 13 on resolution),
   1 with the filler as winner. The other 3,303 wrong letters are read as other characters. The reverse cannot be
   measured: there is **no filler truth** in any render of these cells (see above). What can be said of
   the filler as a winner is in answer 4 (Observed).

   | Axis | Letter renders (cells 0-8) | Wrong | Filler wins | `K` renders | `K` wrong | `K` filler wins |
   |---|---:|---:|---:|---:|---:|---:|
   | resolution | 6,941 | 733 | 1 | 264 | 13 | 1 |
   | jpeg | 6,941 | 332 | 0 | 264 | 1 | 0 |
   | rotation | 5,048 | 355 | 0 | 192 | 4 | 0 |
   | blur | 5,679 | 1,388 | 0 | 216 | 45 | 0 |
   | noise | 4,417 | 251 | 0 | 168 | 1 | 0 |
   | contrast | 5,048 | 245 | 0 | 192 | 0 | 0 |
   | **All** | **34,074** | **3,304** | **1** | **1,296** | **64** | **1** |

3. **Cell 0 is not weaker than cells 1-8 at the clean render or in aggregate, but it is weaker under rotation and at
   the lowest resolutions.** Clean: 97 / 100 against 758 / 800 (94.75%). Over the 54 degraded steps: 4,778 / 5,400
   (88.5%) against 38,156 / 43,200 (88.3%); the nine cells' own degraded rates are 87.7% to 89.1%. Step by step (59
   steps, a two-proportion z over 100 against 800 renders): cell 0 is lower at 8 steps, **every rotation step from 1.5
   to 5 degrees (7.5 to 15.1 points lower: 84 / 100 against 735 / 800 at 1.5, 68 / 100 against 665 / 800 at 4) and
   resolution 8, 5 and 4 px (7.0, 16.1 and 15.8 points lower)**, and higher at 3 (rotation 0.5, contrast 0.5 and 0.2).
   About 3 of 59 would reach |z| 2 by chance, and no correction was made, so the eight are a pattern to confirm, not 8
   findings. Cell 0's floors are not the lowest on blur (to 2 sigma, as cells 3, 6 and 8) but its rotation floor, 0.5
   degrees, is among the lowest of the nine (cell 8: 0.25, cell 3: 0.5, cell 6: 1.5, cells 1, 2 and 4: 2, cells 5 and 7: 3). Hypothesized, untested: rotation
   is about the line centre, so the two end cells move most, and the cell-to-timestep map under rotation is an
   approximation (its module doc says so) (Observed for the numbers).
4. **Cells 28-41: 1,271 of 1,400 clean renders read as themselves** (90.79%, 89.2-92.2), against 97.8% for #683's
   cells; the fourteen cells run from 85 to 95 of 100, the last two (cells 40 and 41) lowest at 85. Letter truths 933 /
   1,018 (91.65%, 89.8-93.2), digit truths 338 / 382 (88.48%, 84.9-91.3). Over the degraded steps 16.0% of the 75,600
   renders are wrong (8.5% in #683's cells, 11.7% in cells 0-8), digit truths worse than letter truths (20.0% of 20,628
   against 14.4% of 54,972; in cells 0-8, 16.3% against 9.7%). The clean misses gather in a few pairs: `O` read as `0`
   23 of 32, `0` as `O` 15 of 35, `1` read as a space 18 of 35, `S` read as `$`, `?` or `s` 15 of 54 (Observed).
   - **The filler as a winner:** 220 of the 75,600 renders (0.29%) have `<` as the wrong winner, 156 on letter truths
     and 64 on digit truths. **All 220 are on the rotation axis at 3, 4 and 5 degrees** (2, 26 and 192), none on any
     other axis, and **203 (92.3%) are in cells 38-41**, 98 of them in cell 41 (cell 40: 51, 39: 38, 38: 16; every
     cell below 38 has 6 or fewer). The reverse (filler truths read as something else) has n = 0.
   - **Floors** (below) are mostly tighter than #683's digit cells: rotation floors of 0.25 degrees in six cells and
     blur floors of 0.5 to 1 sigma in four, where #683's cells mostly held to 2 to 4 degrees and 2.5 to 3 sigma. Cells 35, 38,
     40 and 41 have no floor because their clean render is already below 90 of 100.
5. **The class split cannot constrain these cells**, because the class is "any": every MRZ character is in class.
   Wrong winners over the degraded steps: cells 0-8, 5,666 of 48,600 renders (11.7%), 4,135 (73.0%) an MRZ character
   and 1,531 (27.0%) outside the MRZ set; cells 28-41, 12,061 of 75,600 (16.0%), 7,233 (60.0%) an MRZ character and
   4,828 (40.0%) outside. The commonest swaps are the shape pairs, over the degraded steps: cells 0-8 `O` as `0` 700,
   `0` as `O` 529, `5` as `S` 354, `Z` as `2` 182, `3` as `S` 163; cells 28-41 `O` as `0` 1,078, `0` as `O` 799, `1`
   as a space 663, `J` as a space 406, `5` as `S` 374, `S` as `$` 326, `Q` as `@` 312 (Observed). The O and `0` pair is
   30% (cells 0-8) and 26% (cells 28-41) of the wrong winners that are MRZ characters, which a position class that
   allows both cannot separate.

## Cell by cell: the clean render

Letter and digit truths are pooled within a cell; the `All` column is the cell's 100 renders.

| Cell | Field | Letter truths | Digit truths | All | Wrong winners |
|---|---|---|---|---|---|
| 0 | document number 1 | 70 / 71 [0.924, 0.998] | 27 / 29 [0.780, 0.981] | 97 / 100 [0.915, 0.990] | `2` 1, `O` 1, `?` 1 |
| 1 | document number 2 | 72 / 76 [0.872, 0.979] | 21 / 24 [0.690, 0.957] | 93 / 100 [0.863, 0.966] | `0` 2, `O` 2, space 1, `v` 1, `?` 1 |
| 2 | document number 3 | 67 / 74 [0.817, 0.953] | 25 / 26 [0.811, 0.993] | 92 / 100 [0.850, 0.959] | `0` 2, `?` 2, `@` 1, `x` 1, `2` 1, `O` 1 |
| 3 | document number 4 | 69 / 71 [0.903, 0.992] | 27 / 29 [0.780, 0.981] | 96 / 100 [0.902, 0.984] | `0` 1, `v` 1, `O` 1, `?` 1 |
| 4 | document number 5 | 70 / 72 [0.904, 0.992] | 27 / 28 [0.823, 0.994] | 97 / 100 [0.915, 0.990] | `0` 2, `O` 1 |
| 5 | document number 6 | 69 / 72 [0.885, 0.986] | 27 / 28 [0.823, 0.994] | 96 / 100 [0.902, 0.984] | `0` 2, `?` 1, space 1 |
| 6 | document number 7 | 59 / 61 [0.888, 0.991] | 37 / 39 [0.831, 0.986] | 96 / 100 [0.902, 0.984] | `O` 2, `?` 1, `0` 1 |
| 7 | document number 8 | 67 / 72 [0.848, 0.970] | 26 / 28 [0.774, 0.980] | 93 / 100 [0.863, 0.966] | `?` 3, `0` 2, `2` 1, `O` 1 |
| 8 | document number 9 | 58 / 62 [0.846, 0.975] | 37 / 38 [0.865, 0.995] | 95 / 100 [0.888, 0.978] | `0` 2, `$` 1, `?` 1, `O` 1 |
| 28 | personal number 1 | 62 / 70 [0.790, 0.941] | 30 / 30 [0.886, 1.000] | 92 / 100 [0.850, 0.959] | `?` 5, `2` 2, `0` 1 |
| 29 | personal number 2 | 67 / 73 [0.832, 0.962] | 25 / 27 [0.766, 0.979] | 92 / 100 [0.850, 0.959] | `0` 2, `O` 2, `@` 1, `$` 1, `?` 1, space 1 |
| 30 | personal number 3 | 72 / 77 [0.857, 0.972] | 19 / 23 [0.629, 0.930] | 91 / 100 [0.838, 0.952] | `?` 4, `0` 2, `O` 2, space 1 |
| 31 | personal number 4 | 65 / 71 [0.828, 0.961] | 27 / 29 [0.780, 0.981] | 92 / 100 [0.850, 0.959] | space 3, `$` 2, `0` 1, `?` 1, `O` 1 |
| 32 | personal number 5 | 69 / 71 [0.903, 0.992] | 26 / 29 [0.736, 0.964] | 95 / 100 [0.888, 0.978] | space 3, `0` 2 |
| 33 | personal number 6 | 67 / 72 [0.848, 0.970] | 25 / 28 [0.728, 0.963] | 92 / 100 [0.850, 0.959] | `?` 3, space 2, `0` 2, `O` 1 |
| 34 | personal number 7 | 63 / 66 [0.875, 0.984] | 30 / 34 [0.734, 0.953] | 93 / 100 [0.863, 0.966] | space 4, `0` 1, `@` 1, `?` 1 |
| 35 | personal number 8 | 79 / 86 [0.841, 0.960] | 10 / 14 [0.454, 0.883] | 89 / 100 [0.814, 0.937] | space 4, `0` 2, `@` 2, `$` 1, `?` 1, `O` 1 |
| 36 | personal number 9 | 63 / 64 [0.917, 0.997] | 28 / 36 [0.619, 0.883] | 91 / 100 [0.838, 0.952] | space 4, `O` 2, `?` 2, `0` 1 |
| 37 | personal number 10 | 71 / 75 [0.871, 0.979] | 22 / 25 [0.700, 0.958] | 93 / 100 [0.863, 0.966] | space 3, `0` 1, `$` 1, `O` 1, `?` 1 |
| 38 | personal number 11 | 65 / 73 [0.798, 0.943] | 24 / 27 [0.719, 0.961] | 89 / 100 [0.814, 0.937] | `?` 4, space 4, `0` 2, `V` 1 |
| 39 | personal number 12 | 66 / 71 [0.846, 0.970] | 26 / 29 [0.736, 0.964] | 92 / 100 [0.850, 0.959] | space 3, `O` 2, `1` 1, `s` 1, `?` 1 |
| 40 | personal number 13 | 61 / 71 [0.760, 0.922] | 24 / 29 [0.655, 0.924] | 85 / 100 [0.767, 0.907] | `?` 6, `O` 3, space 2, `0` 2, `1` 1, `$` 1 |
| 41 | personal number 14 | 63 / 78 [0.707, 0.880] | 22 / 22 [0.851, 1.000] | 85 / 100 [0.767, 0.907] | `0` 4, `?` 3, space 3, `$` 2, `1` 1, `V` 1, `2` 1 |

## Floors, per cell and axis

A floor is the most severe step still read as itself in at least 90 of 100 renders, going down from the clean render
without a failing step (`all steps` = none failed); `none` = the clean step already fails. Units: resolution px per
cell (22 is native), JPEG quality, rotation degrees, blur sigma in native px, noise sigma in grey levels, contrast ink
scale. The jpeg axis has no identity step: its first step is quality 100. A cell whose clean rate sits near 90 has a
floor on the edge: cells 28 to 41 start between 85 and 95.

| Cell | resolution | jpeg | rotation | blur | noise | contrast |
|---|---|---|---|---|---|---|
| 0 | to 9 (fails at 8) | all steps | to 0.5 (fails at 1) | to 2 (fails at 2.5) | all steps | all steps |
| 1 | to 9 (fails at 8) | all steps | to 2 (fails at 3) | to 2.5 (fails at 3) | to 24 (fails at 32) | all steps |
| 2 | to 7 (fails at 6) | all steps | to 2 (fails at 3) | to 2.5 (fails at 3) | all steps | all steps |
| 3 | to 6 (fails at 5) | all steps | to 0.5 (fails at 1) | to 2 (fails at 2.5) | all steps | all steps |
| 4 | to 7 (fails at 6) | all steps | to 2 (fails at 3) | to 1.5 (fails at 2) | all steps | all steps |
| 5 | to 9 (fails at 8) | all steps | to 3 (fails at 4) | to 1 (fails at 1.5) | all steps | all steps |
| 6 | to 6 (fails at 5) | all steps | to 1.5 (fails at 2) | to 2 (fails at 2.5) | all steps | all steps |
| 7 | to 5 (fails at 4) | to 10 (fails at 5) | to 3 (fails at 4) | to 3 (fails at 4) | all steps | to 0.8 (fails at 0.6) |
| 8 | to 13 (fails at 11) | all steps | to 0.25 (fails at 0.5) | to 2 (fails at 2.5) | all steps | all steps |
| 28 | to 7 (fails at 6) | to 10 (fails at 5) | to 0.25 (fails at 0.5) | to 0.5 (fails at 1) | all steps | all steps |
| 29 | to 13 (fails at 11) | to 40 (fails at 30) | to 2 (fails at 3) | to 1 (fails at 1.5) | to 16 (fails at 24) | to 0.2 (fails at 0.15) |
| 30 | to 22 (fails at 18) | to 60 (fails at 50) | to 0.25 (fails at 0.5) | to 0.5 (fails at 1) | to 2 (fails at 4) | to 0.8 (fails at 0.6) |
| 31 | to 15 (fails at 13) | to 50 (fails at 40) | to 2 (fails at 3) | to 0.5 (fails at 1) | to 24 (fails at 32) | to 0.15 (fails at 0.1) |
| 32 | to 6 (fails at 5) | all steps | to 2 (fails at 3) | to 2.5 (fails at 3) | all steps | all steps |
| 33 | to 7 (fails at 6) | all steps | to 0.25 (fails at 0.5) | to 2 (fails at 2.5) | all steps | all steps |
| 34 | to 7 (fails at 6) | to 90 (fails at 75) | to 2 (fails at 3) | to 2.5 (fails at 3) | to 12 (fails at 16) | to 0.4 (fails at 0.3) |
| 35 | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| 36 | to 10 (fails at 9) | to 40 (fails at 30) | to 0.25 (fails at 0.5) | to 1.5 (fails at 2) | all steps | to 1 (fails at 0.8) |
| 37 | to 7 (fails at 6) | to 20 (fails at 15) | to 0.25 (fails at 0.5) | to 2 (fails at 2.5) | to 12 (fails at 16) | to 0.5 (fails at 0.4) |
| 38 | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| 39 | to 18 (fails at 15) | to 75 (fails at 60) | to 0.25 (fails at 0.5) | to 2 (fails at 2.5) | to 12 (fails at 16) | to 0.2 (fails at 0.15) |
| 40 | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| 41 | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |

## Wrong winners on degraded steps

The 54 degraded steps are the six axes' steps without their identity steps. Every cell has the class "any", so a winner
is in class when it is an MRZ character and outside otherwise (a space, `$`, `?`, `@` or a lower-case letter). The last
two columns count the wrong winners that are the filler `<`; no cell has a filler truth.

| Cell | Field | Renders | Wrong | Winner is an MRZ character | Outside the MRZ set | Letter read as `<` | Digit read as `<` |
|---|---|---:|---:|---:|---:|---:|---:|
| 0 | document number 1 | 5,400 | 622 | 488 (78.5%) | 134 (21.5%) | 1 | 0 |
| 1 | document number 2 | 5,400 | 620 | 470 (75.8%) | 150 (24.2%) | 0 | 0 |
| 2 | document number 3 | 5,400 | 634 | 482 (76.0%) | 152 (24.0%) | 0 | 0 |
| 3 | document number 4 | 5,400 | 586 | 424 (72.4%) | 162 (27.6%) | 0 | 0 |
| 4 | document number 5 | 5,400 | 634 | 458 (72.2%) | 176 (27.8%) | 0 | 0 |
| 5 | document number 6 | 5,400 | 619 | 404 (65.3%) | 215 (34.7%) | 0 | 0 |
| 6 | document number 7 | 5,400 | 657 | 517 (78.7%) | 140 (21.3%) | 0 | 0 |
| 7 | document number 8 | 5,400 | 629 | 410 (65.2%) | 219 (34.8%) | 0 | 0 |
| 8 | document number 9 | 5,400 | 665 | 482 (72.5%) | 183 (27.5%) | 0 | 0 |
| 28 | personal number 1 | 5,400 | 755 | 438 (58.0%) | 317 (42.0%) | 0 | 2 |
| 29 | personal number 2 | 5,400 | 788 | 502 (63.7%) | 286 (36.3%) | 0 | 1 |
| 30 | personal number 3 | 5,400 | 964 | 551 (57.2%) | 413 (42.8%) | 0 | 0 |
| 31 | personal number 4 | 5,400 | 833 | 441 (52.9%) | 392 (47.1%) | 0 | 0 |
| 32 | personal number 5 | 5,400 | 688 | 453 (65.8%) | 235 (34.2%) | 1 | 1 |
| 33 | personal number 6 | 5,400 | 716 | 494 (69.0%) | 222 (31.0%) | 0 | 0 |
| 34 | personal number 7 | 5,400 | 810 | 459 (56.7%) | 351 (43.3%) | 0 | 1 |
| 35 | personal number 8 | 5,400 | 757 | 435 (57.5%) | 322 (42.5%) | 1 | 0 |
| 36 | personal number 9 | 5,400 | 905 | 558 (61.7%) | 347 (38.3%) | 2 | 4 |
| 37 | personal number 10 | 5,400 | 871 | 491 (56.4%) | 380 (43.6%) | 4 | 0 |
| 38 | personal number 11 | 5,400 | 860 | 457 (53.1%) | 403 (46.9%) | 13 | 3 |
| 39 | personal number 12 | 5,400 | 929 | 530 (57.1%) | 399 (42.9%) | 28 | 10 |
| 40 | personal number 13 | 5,400 | 1,065 | 697 (65.4%) | 368 (34.6%) | 32 | 19 |
| 41 | personal number 14 | 5,400 | 1,120 | 727 (64.9%) | 393 (35.1%) | 75 | 23 |

## What this does not show

- Nothing about real documents or the shipped pipeline; the atlas reads an oracle crop of a synthetic line with a
  vendored font. Nothing about the real Cyprus pass-02 read of #537: the atlas finds one `K` read as the filler in 1,296
  renders of `K`, in one low-resolution step of cell 0, which is the same swap, but one render cannot say it is the
  same mechanism.
- No filler truths: the personal number was fully filled in all 100 seeds, so how a filler cell reads in cells 28-41,
  and how a short personal number's padding reads, is not measured here. The line-1 filler after `P` (#660) is.
- The pooled rates rest on 100 renders per cell and step spread over 31 to 36 glyphs; a single glyph's rate (`K` in
  cell 0 at one step: 4 renders) is indicative at best. The floors pool a cell's glyphs, so a glyph that always fails
  can sit under a passing cell rate.
- The cell 0 against cells 1-8 comparison runs 59 steps with no multiple-comparison correction.
- One run, one machine, one thread setting; its spread is not measured. The desktop's #676 sweep is not set against
  these counts (the tool's module doc forbids comparing atlas runs across machines).
