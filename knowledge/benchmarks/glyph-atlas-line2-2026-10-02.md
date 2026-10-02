# Glyph atlas on MRZ line 2: 98.0% (TD3) and 99.3% (TD1) of clean cells read as themselves, and on degraded renders 90% of the wrong winners lie outside the cell's layout class

**Date:** 2026-10-02 · **MAIN:** `3a9b0c2` (the head of [#673](https://github.com/ruledicaprio/SynthPass/pull/673): `origin/main` `a0df033` plus `glyph_atlas --line` and truths that vary per seed; every summary's header says `dirty: false`) · **DATA:** none (generated renders, seeds 0-99, TD3 with code `P<` and TD1 with code `I<`; the vendored font `crates/synthpass-gen/fonts/ocr-b.ttf`, sha256 `367d876cca94`) · **Evidence:** Observed on synthetic renders, vendored font, oracle crop (three `glyph_atlas` runs on one desktop PC; the counts below come from their three summary files) · **Status:** current

**2026-10-02.** This note covers step 3 of
[#433](https://github.com/ruledicaprio/SynthPass/issues/433): the cells of MRZ line 2 whose truth changes with the
document, namely the nationality, both dates and their check digits, the sex, and TD3's document-number check digit.
It reads the sweeps of the [`glyph_atlas`](../../crates/synthpass-ocr/examples/glyph_atlas.rs) example at #673's head.
That head adds `--line N` and lets a cell's truth glyph vary between seeds. The sweeps are:

- TD3 line 2, cells 9-27;
- TD1 line 2, cells 0-17;
- a control on line 1 that repeats the laptop's 2026-09-30 sweep.

Each sweep ran 100 seeds and all six degradation axes. **Evidence label: Observed on synthetic renders, vendored
font, oracle crop.**

The counts follow the same conventions as [the first atlas note](glyph-atlas-p-filler-2026-09-30.md):

- A render is correct when the winner equals the truth glyph. The winner is taken from the recognition model's
  distribution averaged over the cell's timestep range, with blank excluded.
- The winner is taken over every label the model has, not only the 37 MRZ characters.
- The atlas reads the oracle line crop. It bypasses imageprep, the retry loop and the grid fit, and it records no
  decoded text.

Nothing here is about the pipeline's decode or about real documents. No code, ADR, baseline or headline changes with
this note.

**The class of a cell** is [`mrz::position_class`](../../crates/mrz/src/position.rs), the character class the
format's layout allows at that cell:

- **digit:** the check digits;
- **digit or filler:** the date cells;
- **letter or filler:** the nationality and the sex.

Each wrong render falls into exactly one of three groups by its winner:

- **in class:** the cell's class allows the winner, as when one digit is read as another;
- **other class:** the winner is one of the 37 MRZ characters, but the class excludes it, as with a letter in a digit
  cell or a digit in a letter cell;
- **outside the MRZ set:** every other label of the model, such as `?`, `$`, a space or lower case.

**Steps.** Each sweep has 59 steps over its six axes. Five of them (resolution 22, rotation 0, blur 0, noise 0 and
contrast 1) are the same clean render, so there are 55 distinct steps: the clean render and 54 degraded steps. Every
count below takes each distinct step once.

## The answer

- **The control reproduces the laptop's sweep exactly.** Line 1 cells 0 and 1 (TD3, `P<`) on the desktop PC match
  the laptop's 2026-09-30 cells sweep (`bc36b24`) in all 118 rows. The rows were joined on (cell, axis, step, truth),
  every field is identical, and the largest difference is 0. On these two machines, with the same `Cargo.lock`,
  models and `RTEN_NUM_THREADS=4`, the atlas gives bit-identical numbers (Observed). The filler after `P` again reads
  correctly in 4 of 100 clean renders, with `S` the winner in all 96 misses.
- **At the clean render, line 2's cells read as themselves almost always:**
  - TD3: 1,862 of 1,900 cell renders (98.00%, Wilson 95% 97.27-98.54);
  - TD1: 1,787 of 1,800 (99.28%, 98.77-99.58).

  None of the 51 wrong clean renders has an in-class winner. 21 are another class's MRZ character and 30 lie outside
  the MRZ set. The clean misses gather in a few (cell, glyph) pairs, and the three largest are all in TD3 (Observed):
  - the document-number check digit `0` reads as `O` in 6 of 7 renders;
  - the third nationality letter `O` reads as `0` in 10 of 12;
  - the same cell's `S` reads as `$` in 3 of 9.
- **Over the 54 degraded steps, about nine in ten wrong winners lie outside the cell's class:**

  | Format | Renders | Wrong | In class | Other class | Outside the MRZ set |
  |---|---:|---:|---:|---:|---:|
  | TD3 | 102,600 | 8,212 (8.0%) | 835 (10.2%) | 5,254 (64.0%) | 2,123 (25.9%) |
  | TD1 | 97,200 | 7,990 (8.2%) | 857 (10.7%) | 3,907 (48.9%) | 3,226 (40.4%) |

  So 89.8% (TD3) and 89.3% (TD1) of the wrong winners lie outside the class (Observed).
- **The in-class errors sit in the letter cells, not the digit cells:**

  | Format | Cells | Wrong | In class |
  |---|---|---:|---:|
  | TD3 | digit | 6,282 | 189 (3.0%) |
  | TD3 | letter | 1,930 | 646 (33.5%) |
  | TD1 | digit | 6,020 | 291 (4.8%) |
  | TD1 | letter | 1,970 | 566 (28.7%) |

  When a date or check-digit cell is wrong, its winner is almost never another digit. TD3's document-number check
  digit has 1 in-class winner in 1,135 wrong renders. When a nationality or sex cell is wrong, about a third of its
  winners are letters the class allows (Observed).
- **The commonest other-class swaps are the familiar shape pairs:**
  - TD3, over the degraded steps: `0` as `O` 1,153, `3` as `S` 492, `5` as `S` 477, `O` as `0` 431, `7` as `T` 395
    and `2` as `Z` 227;
  - TD1: `0` as `O` 669, `5` as `S` 296, `3` as `S` 234, `2` as `Z` 176, `7` as `T` 169 and `8` as `B` 146
    (Observed).
- **The same field reads differently at the two formats' offsets.** Both sweeps drew the same 100 identities, so a
  TD1 field and its TD3 twin have the same truths and the same render counts. The twins still differ by up to 41
  points (table below), in both directions (Observed).

What the class split means for the pipeline is an inference, not a measurement. In the digit cells,
`mrz::find_and_parse`'s repair already turns a misread letter into a digit. `position_class` documents that its digit
classes are exactly those cells, and a test keeps the two in step. The MRZ charset already excludes the "outside" group
from a decode. Neither constraint can remove an in-class winner. On these renders, in-class winners are a few percent
of the wrong ones in the digit cells and about a third in the nationality and sex cells. A per-cell rule for the letter
cells therefore needs information beyond the class. This note does not measure what that information would be.

## Line 2, cell by cell

Over the 54 degraded steps, 100 seeds each. *Observed on synthetic renders, vendored font, oracle crop.*

### TD3 line 2

| Cell | Field | Class | Renders | Wrong | In class | Other class | Outside the MRZ set | Commonest other-class swap |
|---:|---|---|---:|---:|---:|---:|---:|---|
| 9 | document number check digit | digit | 5,400 | 1,135 | 1 (0.1%) | 818 (72.1%) | 316 (27.8%) | `0` as `O` (300) |
| 10 | nationality 1 | letter or filler | 5,400 | 381 | 191 (50.1%) | 55 (14.4%) | 135 (35.4%) | `B` as `8` (46) |
| 11 | nationality 2 | letter or filler | 5,400 | 234 | 146 (62.4%) | 18 (7.7%) | 70 (29.9%) | `B` as `8` (5) |
| 12 | nationality 3 | letter or filler | 5,400 | 751 | 85 (11.3%) | 446 (59.4%) | 220 (29.3%) | `O` as `0` (431) |
| 13 | birth date 1 | digit or filler | 5,400 | 617 | 5 (0.8%) | 488 (79.1%) | 124 (20.1%) | `0` as `O` (166) |
| 14 | birth date 2 | digit or filler | 5,400 | 384 | 5 (1.3%) | 292 (76.0%) | 87 (22.7%) | `5` as `S` (66) |
| 15 | birth date 3 | digit or filler | 5,400 | 372 | 1 (0.3%) | 301 (80.9%) | 70 (18.8%) | `0` as `O` (212) |
| 16 | birth date 4 | digit or filler | 5,400 | 298 | 10 (3.4%) | 248 (83.2%) | 40 (13.4%) | `5` as `S` (39) |
| 17 | birth date 5 | digit or filler | 5,400 | 264 | 3 (1.1%) | 214 (81.1%) | 47 (17.8%) | `0` as `O` (70) |
| 18 | birth date 6 | digit or filler | 5,400 | 269 | 15 (5.6%) | 221 (82.2%) | 33 (12.3%) | `5` as `S` (41) |
| 19 | birth date check digit | digit | 5,400 | 353 | 32 (9.1%) | 246 (69.7%) | 75 (21.2%) | `3` as `S` (40) |
| 20 | sex | letter or filler | 5,400 | 564 | 224 (39.7%) | 43 (7.6%) | 297 (52.7%) | `F` as `1` (24) |
| 21 | expiry date 1 | digit or filler | 5,400 | 365 | 29 (7.9%) | 270 (74.0%) | 66 (18.1%) | `3` as `S` (168) |
| 22 | expiry date 2 | digit or filler | 5,400 | 295 | 27 (9.2%) | 205 (69.5%) | 63 (21.4%) | `7` as `T` (37) |
| 23 | expiry date 3 | digit or filler | 5,400 | 370 | 3 (0.8%) | 241 (65.1%) | 126 (34.1%) | `0` as `O` (134) |
| 24 | expiry date 4 | digit or filler | 5,400 | 325 | 15 (4.6%) | 257 (79.1%) | 53 (16.3%) | `7` as `T` (36) |
| 25 | expiry date 5 | digit or filler | 5,400 | 327 | 4 (1.2%) | 247 (75.5%) | 76 (23.2%) | `0` as `O` (78) |
| 26 | expiry date 6 | digit or filler | 5,400 | 393 | 27 (6.9%) | 302 (76.8%) | 64 (16.3%) | `3` as `S` (70) |
| 27 | expiry date check digit | digit | 5,400 | 515 | 12 (2.3%) | 342 (66.4%) | 161 (31.3%) | `7` as `T` (59) |

### TD1 line 2

| Cell | Field | Class | Renders | Wrong | In class | Other class | Outside the MRZ set | Commonest other-class swap |
|---:|---|---|---:|---:|---:|---:|---:|---|
| 0 | birth date 1 | digit or filler | 5,400 | 442 | 16 (3.6%) | 256 (57.9%) | 170 (38.5%) | `5` as `S` (38) |
| 1 | birth date 2 | digit or filler | 5,400 | 406 | 27 (6.7%) | 225 (55.4%) | 154 (37.9%) | `0` as `O` (25) |
| 2 | birth date 3 | digit or filler | 5,400 | 397 | 12 (3.0%) | 293 (73.8%) | 92 (23.2%) | `0` as `O` (101) |
| 3 | birth date 4 | digit or filler | 5,400 | 408 | 37 (9.1%) | 246 (60.3%) | 125 (30.6%) | `5` as `S` (22) |
| 4 | birth date 5 | digit or filler | 5,400 | 378 | 17 (4.5%) | 241 (63.8%) | 120 (31.7%) | `0` as `O` (56) |
| 5 | birth date 6 | digit or filler | 5,400 | 388 | 20 (5.2%) | 233 (60.1%) | 135 (34.8%) | `5` as `S` (33) |
| 6 | birth date check digit | digit | 5,400 | 456 | 21 (4.6%) | 272 (59.6%) | 163 (35.7%) | `0` as `O` (35) |
| 7 | sex | letter or filler | 5,400 | 538 | 249 (46.3%) | 30 (5.6%) | 259 (48.1%) | `F` as `0` (7) |
| 8 | expiry date 1 | digit or filler | 5,400 | 423 | 16 (3.8%) | 214 (50.6%) | 193 (45.6%) | `3` as `S` (78) |
| 9 | expiry date 2 | digit or filler | 5,400 | 405 | 30 (7.4%) | 239 (59.0%) | 136 (33.6%) | `5` as `S` (39) |
| 10 | expiry date 3 | digit or filler | 5,400 | 374 | 2 (0.5%) | 270 (72.2%) | 102 (27.3%) | `0` as `O` (195) |
| 11 | expiry date 4 | digit or filler | 5,400 | 444 | 35 (7.9%) | 256 (57.7%) | 153 (34.5%) | `5` as `S` (35) |
| 12 | expiry date 5 | digit or filler | 5,400 | 403 | 11 (2.7%) | 267 (66.3%) | 125 (31.0%) | `0` as `O` (95) |
| 13 | expiry date 6 | digit or filler | 5,400 | 489 | 28 (5.7%) | 282 (57.7%) | 179 (36.6%) | `3` as `S` (46) |
| 14 | expiry date check digit | digit | 5,400 | 607 | 19 (3.1%) | 341 (56.2%) | 247 (40.7%) | `0` as `O` (58) |
| 15 | nationality 1 | letter or filler | 5,400 | 709 | 147 (20.7%) | 63 (8.9%) | 499 (70.4%) | `B` as `8` (38) |
| 16 | nationality 2 | letter or filler | 5,400 | 327 | 125 (38.2%) | 25 (7.6%) | 177 (54.1%) | `G` as `6` (10) |
| 17 | nationality 3 | letter or filler | 5,400 | 396 | 45 (11.4%) | 154 (38.9%) | 197 (49.7%) | `O` as `0` (145) |

## Wrong at the clean render

Every wrong clean render, with its winners. *Observed on synthetic renders, vendored font, oracle crop.* "n" is how
many of the 100 seeds drew that glyph at that cell.

| Format | Cell | Field | Truth | Wrong / n | Winners |
|---|---:|---|---|---:|---|
| TD3 | 9 | document number check digit | `0` | 6 / 7 | `O` 6 |
| TD3 | 9 | document number check digit | `1` | 2 / 10 | space 2 |
| TD3 | 9 | document number check digit | `2`, `3`, `6` | 1 / 12, 1 / 9, 1 / 7 | `?` 1 each |
| TD3 | 12 | nationality 3 | `O` | 10 / 12 | `0` 10 |
| TD3 | 12 | nationality 3 | `S` | 3 / 9 | `$` 3 |
| TD3 | 12 | nationality 3 | `D` | 1 / 5 | space 1 |
| TD3 | 13 | birth date 1 | `0` | 2 / 12 | `O` 2 |
| TD3 | 13 | birth date 1 | `7` | 2 / 24 | `?` 2 |
| TD3 | 14, 15, 17, 18, 23 | birth and expiry date digits | `1` | 1 / 14, 1 / 23, 1 / 31, 1 / 14, 1 / 23 | space 1 each |
| TD3 | 14 | birth date 2 | `7` | 1 / 10 | `?` 1 |
| TD3 | 20 | sex | `F` | 1 / 53 | space 1 |
| TD3 | 21 | expiry date 1 | `3` | 1 / 64 | `?` 1 |
| TD3 | 27 | expiry date check digit | `0` | 1 / 13 | `O` 1 |
| TD1 | 7 | sex | `F` | 1 / 53 | space 1 |
| TD1 | 14 | expiry date check digit | `2` | 1 / 9 | space 1 |
| TD1 | 15 | nationality 1 | `C`, `J`, `S`, `U` | 1 / 4, 2 / 5, 2 / 12, 2 / 25 | space 1, space 2, `?` 2, `?` 1 and `u` 1 |
| TD1 | 16 | nationality 2 | `S`, `T` | 1 / 5, 1 / 12 | `?` 1 each |
| TD1 | 17 | nationality 3 | `O` | 2 / 12 | `0` 2 |

## The same field at two offsets

A TD1 cell *c* holds the TD3 field at cell *c* + 13 for the dates, check digits and sex, and at cell *c* - 5 for the
nationality. Over the 55 distinct steps, these are the (field, truth) pairs whose twins differ by 15 points or more.
*Observed on synthetic renders, vendored font, oracle crop.*

| Field | Truth | TD1 cell | Correct | TD3 cell | Correct | TD1 minus TD3 (points) |
|---|---|---:|---:|---:|---:|---:|
| nationality 3 | `O` | 17 | 481 / 660 | 12 | 208 / 660 | +41.4 |
| nationality 3 | `S` | 17 | 468 / 495 | 12 | 325 / 495 | +28.9 |
| birth date 1 | `0` | 0 | 608 / 660 | 13 | 465 / 660 | +21.7 |
| nationality 1 | `C` | 15 | 133 / 220 | 10 | 215 / 220 | -37.3 |
| nationality 1 | `J` | 15 | 164 / 275 | 10 | 218 / 275 | -19.6 |
| nationality 1 | `Z` | 15 | 125 / 165 | 10 | 157 / 165 | -19.4 |

The sweeps do not separate the causes. A different line length means a different crop width and timestep alignment,
and the neighbouring glyphs differ too. What they show is that the glyph the model gets wrong at one offset is not
always the one it gets wrong at the same field's other offset.

## What this does not show

- Nothing about how the pipeline reads line 2. The atlas reads an oracle crop and takes its winner over every label of
  the model. The shipped decode restricts itself to the MRZ charset, so an "outside the MRZ set" winner there would
  become some MRZ character, which this note does not record.
- Nothing about real documents, other codes, or cells outside the ones swept (TD3 cells 0-8 and 28-43, TD1 cells
  18-29).
- How often each truth occurs depends on the generator's identities. *n* per (cell, glyph) runs from 2 to 77 seeds,
  so a rate on a rare glyph rests on few renders. The summaries carry a Wilson interval for every step.
- Each axis's severe steps weigh as much as its mild ones in the totals above. A threshold argument needs the per-step
  rows.

## Evidence and limits

- **Runs**, from the run headers and the desktop's report:
  - three `glyph_atlas` runs at `3a9b0c2df555cabf8cb05caeb77c6546ead28201`, schema 3, release build;
  - on the desktop PC: Windows x86_64, 4 logical CPUs, `RTEN_NUM_THREADS=4`; the header records `cpu_model: null`;
  - one measurement process at a time through the CPU slot;
  - ocrs 0.13.1, rten 0.26.0, beam width 24, crop `oracle_line`, detection threshold 0.2;
  - the pinned models: detection sha256 `f15cfb56bd02…` and recognition `e484866d4cce…`;
  - seeds 0-99 and all six axes with the module doc's steps.

  The commands, each run with `RTEN_NUM_THREADS=4 cargo run -p synthpass-ocr --release --example glyph_atlas --`:
  - `--seeds 100` (the control: TD3, line 1, cells 0 and 1);
  - `--line 2 --cells 9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27 --seeds 100`;
  - `--format td1 --line 2 --cells 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17 --seeds 100`.

  Wall times were 3,932 s, 3,848 s and 3,681 s, each for 5,500 computed renders (55 distinct steps × 100 seeds).
- **Output:** each run's summary holds aggregates only. The control has 118 rows (sha256 `cd23b44e86b3…`), TD3 line 2
  has 8,496 (`4e4250cd4ba6…`) and TD1 line 2 has 7,906 (`daf81dc25844…`). Neither the summaries nor the per-seed
  records are committed. The records stayed on the desktop PC, and this note uses nothing from them.
- **Determinism:** the degradations are pure functions of the clean render and (seed, axis, step), and inference is
  floating-point. The control's bit-identical match across two machines is one observation in one configuration, so
  the tool's advice to compare runs on one machine stands.
