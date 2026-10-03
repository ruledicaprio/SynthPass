# Glyph atlas on TD3 line 2, 30 seeds on the laptop: 587 of 600 clean cells read as themselves, nationality 3 is the weak cell (24 / 30), and 91.4% of wrong winners lie outside the cell's class

**Date:** 2026-10-02 · **MAIN:** `dcf4592` (`origin/main` when the run started; the tool is [#673](https://github.com/ruledicaprio/SynthPass/pull/673)'s `glyph_atlas`, unchanged; the summary header says `dirty: true`, which is the tree's untracked files only, the tracked tree was clean) · **DATA:** none (generated renders, seeds 0-29; the vendored font `crates/synthpass-gen/fonts/ocr-b.ttf`, sha256 `367d876cca94`; models `f15cfb56bd02` (detection) and `e484866d4cce` (recognition), both pinned by `synthpass_ocr::verify`) · **Evidence:** Observed on synthetic renders, vendored font, oracle crop · **Status:** current

**2026-10-02.** This is the last step of [#433](https://github.com/ruledicaprio/SynthPass/issues/433)'s line-2
work: the cells of TD3's second MRZ line whose truth glyph changes with the seed, measured by the laptop with the
settings of the [per-document-code sweep](glyph-atlas-document-codes-2026-10-01.md) (all six axes, 30 seeds from 0).
The desktop's sweep of cells 9-27 at 100 seeds and its finding ([#676](https://github.com/ruledicaprio/SynthPass/pull/676))
are already on `main`. This note adds two things: **cells 42 and 43**, which that sweep did not cover (the
personal-number check digit and the composite check digit), and the same cells at the 10-01 seed count **on the
laptop**, the machine of [#624](https://github.com/ruledicaprio/SynthPass/pull/624) and
[#660](https://github.com/ruledicaprio/SynthPass/pull/660). The tool's own module doc forbids comparing atlas runs
across machines, so #676's counts are not set against these row by row. The atlas reads the oracle line crop, bypasses
imageprep, the retry loop and the grid fit, and records no decoded text. It says nothing about real documents. No
code, ADR, baseline or default changes with this note.

**The run.**

| | |
|---|---|
| Machine | win11-i7-1255U, 12 logical CPUs, Windows, AC power |
| `RTEN_NUM_THREADS` | 4 (the value of the 10-01 laptop sweeps); one run at a time under `slot.ps1` |
| Tool | `glyph_atlas` release build, `schema` 3, ocrs 0.13.1, rten 0.26.0; `Cargo.lock` of `dcf4592` |
| Command | `glyph_atlas --format td3 --line 2 --cells 9,10,11,12,13,14,15,16,17,18,19,21,22,23,24,25,26,27,42,43 --axes resolution,jpeg,rotation,blur,noise,contrast --seeds 30 --seed-start 0` |
| Time | 20:41:16Z to 21:01:26Z, exit 0; 1,650 computed renders (1,770 rendered steps) in 1,208.8 s |
| Output | 8,673 summary rows, one per (cell, truth glyph, axis, step); `records.jsonl` is per-seed and stays local |

Every cell has 30 renders per step, one per seed, with no missing recognition. A cell whose truth varies has its 30
renders split among its glyphs, so a single (cell, glyph) pair has few renders (cell 15 takes only `0` and `1`; cell
12 takes nine letters). The tables therefore **pool the glyphs of a cell**, as the line-2 note does, and read a rate
over n = 30. **Read as itself** is the first sweep's definition: the winner of the CTC distribution averaged over the
cell's timesteps, blank excluded, equals the truth glyph, taken over every label the model has. A rate of 30 / 30 has
a Wilson 95% interval of 0.886-1.000.

**The truth of each cell** (the glyphs it took over seeds 0-29, from the summary header) is in the `Truths` column
below. The cell classes are those of [`mrz::position_class`](../../crates/mrz/src/position.rs): the check digits
(9, 19, 27, 43) are digits; the date cells (13-18, 21-26) and, in TD3, cell 42 are digits or fillers; the nationality
cells (10-12) are letters or fillers.

## The answer

- **At the clean render 587 of 600 cell renders read as themselves** (97.83%, Wilson 95% 96.33-98.73), 531 of 540
  (98.33%) in cells 9-27 and 56 of 60 in cells 42 and 43 (Observed). None of the 13 wrong clean renders has an
  in-class winner: nine are another class's MRZ character and four lie outside the MRZ set.
- **Cell 12 (the third nationality letter) is the weak cell: 24 of 30** [0.627, 0.905]. Every other cell of 9-27
  reads 29 or 30 of 30. The six misses are two glyphs, `O` read as `0` in all four of its renders and `S` read as `$`
  in both of its (Observed; six renders, so the glyph rates are indicative only).
- **The two new cells read at the clean render in 27 of 30 (cell 42) and 29 of 30 (cell 43).** Cell 42's three misses
  are `0` read as `O` twice (of six) and `7` read as `?` once; cell 43's one miss is a `0` read as `O`. Cell 42 sits
  exactly on the 27 / 30 line the floors below use, so its floors are on the edge: its rate is 24 to 27 of 30
  across rotation 0 to 2 degrees and 27 to 29 across blur 0 to 2.5, so the one step that "fails" at 0.25 degrees (24 of 30, back
  to 27 at 0.5 and 1) is not a floor (Observed).
- **On degraded renders 91.4% of wrong winners lie outside the cell's class.** Over the 54 degraded steps, 32,400
  cell renders, 2,744 are wrong (8.5%): 235 in class (8.6%), 1,984 another class's MRZ character (72.3%), 525 outside
  the MRZ set (19.1%). In the 18 cells shared with #676 it is 2,247 wrong, 184 in class (8.2%); in cells 42 and 43
  497 wrong, 51 in class (10.3%) (Observed).
- **The in-class errors sit in the letter cells.** In the digit and digit-or-filler cells 103 of 2,327 wrong winners
  (4.4%) are in class; in the three nationality cells 132 of 417 (31.7%) are (Observed). The commonest swaps over the
  degraded steps are `0` as `O` 460, `5` as `S` 189, `3` as `S` 154, `O` as `0` 146, `7` as `T` 130, `8` as `S` 92,
  `1` as a space 85 and `S` as `$` 65 (Observed).
- **Rotation and blur end a read in most cells; JPEG, noise and contrast almost never do** (floors below). The
  document-number check digit (cell 9) is the most rotation-sensitive: 29 of 30 at 0 degrees, 28 at 0.25, 21 at 0.5.

## Cell by cell: the clean render

| Cell | Field | Class | Truths | Read as itself, of 30 [Wilson 95%] | Wrong winners |
|---|---|---|---|---|---|
| 9 | document number check digit | digit | `0123456789` | 29 [0.833, 0.994] | `O` 1 |
| 10 | nationality 1 | letter or filler | `BDFGMRSUZ` | 30 [0.886, 1.000] | - |
| 11 | nationality 2 | letter or filler | `ABEGKLRSTUW` | 30 [0.886, 1.000] | - |
| 12 | nationality 3 | letter or filler | `ABDEFORSU` | 24 [0.627, 0.905] | `0` 4, `$` 2 |
| 13 | birth date 1 | digit or filler | `056789` | 29 [0.833, 0.994] | `O` 1 |
| 14 | birth date 2 | digit or filler | `013456789` | 29 [0.833, 0.994] | space 1 |
| 15 | birth date 3 | digit or filler | `01` | 30 [0.886, 1.000] | - |
| 16 | birth date 4 | digit or filler | `0123456789` | 30 [0.886, 1.000] | - |
| 17 | birth date 5 | digit or filler | `012` | 30 [0.886, 1.000] | - |
| 18 | birth date 6 | digit or filler | `12345678` | 30 [0.886, 1.000] | - |
| 19 | birth date check digit | digit | `0123456789` | 30 [0.886, 1.000] | - |
| 21 | expiry date 1 | digit or filler | `23` | 30 [0.886, 1.000] | - |
| 22 | expiry date 2 | digit or filler | `0123456789` | 30 [0.886, 1.000] | - |
| 23 | expiry date 3 | digit or filler | `01` | 30 [0.886, 1.000] | - |
| 24 | expiry date 4 | digit or filler | `0123456789` | 30 [0.886, 1.000] | - |
| 25 | expiry date 5 | digit or filler | `012` | 30 [0.886, 1.000] | - |
| 26 | expiry date 6 | digit or filler | `0123456789` | 30 [0.886, 1.000] | - |
| 27 | expiry date check digit | digit | `02456789` | 30 [0.886, 1.000] | - |
| 42 | personal number check digit | digit or filler | `0123456789` | 27 [0.744, 0.965] | `O` 2, `?` 1 |
| 43 | composite check digit | digit | `02468` | 29 [0.833, 0.994] | `O` 1 |

## Floors, per cell and axis

A floor is the most severe step still read as itself in at least 27 of 30 renders, going down from the clean render
without a failing step (`all steps` = none failed); `none` = the clean step already fails (cell 12). Units:
resolution px per cell (22 is native), JPEG quality, rotation degrees, blur sigma in native px, noise sigma in grey
levels, contrast ink scale. The jpeg axis has no identity step: its first step is quality 100.

| Cell | resolution | jpeg | rotation | blur | noise | contrast |
|---|---|---|---|---|---|---|
| 9 | to 15 (fails at 13) | to 10 (fails at 5) | to 0.25 (fails at 0.5) | to 1.5 (fails at 2) | all steps | all steps |
| 10 | to 5 (fails at 4) | all steps | all steps | to 3 (fails at 4) | all steps | all steps |
| 11 | to 5 (fails at 4) | all steps | all steps | to 3 (fails at 4) | all steps | all steps |
| 12 | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| 13 | to 6 (fails at 5) | all steps | to 1.5 (fails at 2) | to 2.5 (fails at 3) | all steps | all steps |
| 14 | to 5 (fails at 4) | all steps | to 1 (fails at 1.5) | to 2.5 (fails at 3) | all steps | all steps |
| 15 | to 5 (fails at 4) | all steps | to 2 (fails at 3) | to 2.5 (fails at 3) | all steps | all steps |
| 16 | all steps | all steps | to 4 (fails at 5) | to 3 (fails at 4) | all steps | all steps |
| 17 | all steps | all steps | all steps | to 2.5 (fails at 3) | all steps | all steps |
| 18 | all steps | all steps | to 4 (fails at 5) | to 3 (fails at 4) | all steps | all steps |
| 19 | all steps | all steps | to 3 (fails at 4) | to 2.5 (fails at 3) | all steps | all steps |
| 21 | all steps | all steps | to 3 (fails at 4) | to 3 (fails at 4) | all steps | all steps |
| 22 | all steps | all steps | to 4 (fails at 5) | to 3 (fails at 4) | all steps | all steps |
| 23 | all steps | all steps | to 4 (fails at 5) | to 3 (fails at 4) | all steps | all steps |
| 24 | all steps | all steps | to 4 (fails at 5) | to 3 (fails at 4) | all steps | all steps |
| 25 | all steps | all steps | to 4 (fails at 5) | to 3 (fails at 4) | all steps | all steps |
| 26 | all steps | all steps | to 2 (fails at 3) | to 3 (fails at 4) | all steps | all steps |
| 27 | all steps | all steps | to 3 (fails at 4) | to 3 (fails at 4) | all steps | all steps |
| 42 | to 22 (fails at 18) | all steps | to 0 (fails at 0.25) | to 2.5 (fails at 3) | all steps | to 0.3 (fails at 0.2) |
| 43 | to 5 (fails at 4) | all steps | to 2 (fails at 3) | to 2.5 (fails at 3) | all steps | all steps |

## Wrong winners, degraded steps

The 54 degraded steps are the six axes' steps with their identity steps (resolution 22, rotation 0, blur 0, noise 0,
contrast 1) left out. A winner is **in class** when the cell's class allows it, **other class** when it is one of the
37 MRZ characters the class excludes, and **outside** otherwise (a space, `$`, `?` or another label).

| Cell | Field | Class | Renders | Wrong | In class | Other class | Outside the MRZ set |
|---|---|---|---:|---:|---:|---:|---:|
| 9 | document number check digit | digit | 1,620 | 328 | 0 (0.0%) | 232 (70.7%) | 96 (29.3%) |
| 10 | nationality 1 | letter or filler | 1,620 | 94 | 65 (69.1%) | 8 (8.5%) | 21 (22.3%) |
| 11 | nationality 2 | letter or filler | 1,620 | 63 | 39 (61.9%) | 4 (6.3%) | 20 (31.7%) |
| 12 | nationality 3 | letter or filler | 1,620 | 260 | 28 (10.8%) | 148 (56.9%) | 84 (32.3%) |
| 13 | birth date 1 | digit or filler | 1,620 | 185 | 1 (0.5%) | 160 (86.5%) | 24 (13.0%) |
| 14 | birth date 2 | digit or filler | 1,620 | 127 | 1 (0.8%) | 95 (74.8%) | 31 (24.4%) |
| 15 | birth date 3 | digit or filler | 1,620 | 118 | 0 (0.0%) | 91 (77.1%) | 27 (22.9%) |
| 16 | birth date 4 | digit or filler | 1,620 | 75 | 5 (6.7%) | 65 (86.7%) | 5 (6.7%) |
| 17 | birth date 5 | digit or filler | 1,620 | 86 | 1 (1.2%) | 77 (89.5%) | 8 (9.3%) |
| 18 | birth date 6 | digit or filler | 1,620 | 83 | 1 (1.2%) | 75 (90.4%) | 7 (8.4%) |
| 19 | birth date check digit | digit | 1,620 | 103 | 10 (9.7%) | 83 (80.6%) | 10 (9.7%) |
| 21 | expiry date 1 | digit or filler | 1,620 | 102 | 7 (6.9%) | 85 (83.3%) | 10 (9.8%) |
| 22 | expiry date 2 | digit or filler | 1,620 | 91 | 8 (8.8%) | 70 (76.9%) | 13 (14.3%) |
| 23 | expiry date 3 | digit or filler | 1,620 | 104 | 1 (1.0%) | 75 (72.1%) | 28 (26.9%) |
| 24 | expiry date 4 | digit or filler | 1,620 | 96 | 2 (2.1%) | 84 (87.5%) | 10 (10.4%) |
| 25 | expiry date 5 | digit or filler | 1,620 | 96 | 3 (3.1%) | 77 (80.2%) | 16 (16.7%) |
| 26 | expiry date 6 | digit or filler | 1,620 | 120 | 9 (7.5%) | 92 (76.7%) | 19 (15.8%) |
| 27 | expiry date check digit | digit | 1,620 | 116 | 3 (2.6%) | 88 (75.9%) | 25 (21.6%) |
| 42 | personal number check digit | digit or filler | 1,620 | 298 | 44 (14.8%) | 204 (68.5%) | 50 (16.8%) |
| 43 | composite check digit | digit | 1,620 | 199 | 7 (3.5%) | 171 (85.9%) | 21 (10.6%) |

## What this does not show

- Nothing about real documents or the shipped pipeline; the atlas reads an oracle crop of a synthetic line.
- n = 30 per cell and step, split over the cell's glyphs: a step at 27 / 30 has a Wilson 95% lower bound of 0.74, and a
  glyph's own rate (cell 12's `O`, `S`) rests on a handful of renders.
- The floors pool a cell's glyphs, so a glyph that always fails can sit under a passing cell rate.
- One run, one machine: the run-to-run spread is not measured here. The line-1 control of #676 showed the two PCs
  identical at `RTEN_NUM_THREADS=4`, which says a run is reproducible on one machine; it does not make this run
  comparable to the desktop's line-2 sweep, which used 100 seeds and so a different set of renders per step.
- Cells 28-41 (the personal number) and cell 20 (the sex) are not in this run; cell 20 is in #676.
