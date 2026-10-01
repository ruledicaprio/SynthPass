# Glyph atlas, per document code: the filler after `P` and `V` is misread at the clean render; after `A`, it reads 22 / 30; after `I` and `C`, 30 / 30

**Date:** 2026-10-01 · **MAIN:** `66e22e2` (the head of [#643](https://github.com/ruledicaprio/SynthPass/pull/643), `origin/main` `048695e` plus the `--format` and `--code` flags; every summary's header says `dirty: false`) · **DATA:** none (generated renders, seeds 0-29 of each format; the vendored font `crates/synthpass-gen/fonts/ocr-b.ttf`, sha256 `367d876cca94`) · **Evidence:** Observed on synthetic renders, vendored font, oracle crop (37 `glyph_atlas` runs, one per code per stage, counts from their `summary.json`) plus Derived (the contiguous-step readings below, computed over those rows) · **Status:** current

**2026-10-01.** This is the per-code follow-up to [#433](https://github.com/ruledicaprio/SynthPass/issues/433)
and to [the first sweep](glyph-atlas-p-filler-2026-09-30.md), which measured `P<` of a TD3 line only. The
[`glyph_atlas`](../../crates/synthpass-ocr/examples/glyph_atlas.rs) example now takes `--format` and
`--code` ([#643](https://github.com/ruledicaprio/SynthPass/pull/643)), so this note covers the 31 document
codes that `samples/corpus.jsonl` holds or that sit one character from one. For each code the tool renders a
clean page of the code's format with the vendored OCR-B font, degrades it along one axis at a time, and for
cells 0 and 1 of MRZ line one (the two code characters) reads the raw CTC matrix of the *oracle* line crop.
**Evidence label: Observed on synthetic renders, vendored font, oracle crop.** The atlas bypasses imageprep,
the retry loop and the grid fit, records no decoded text, and says nothing about real documents. No code,
ADR, baseline or headline changes with this note.

**What "read as itself" means** is the first sweep's definition: the winner of the CTC distribution averaged
over the cell's timesteps, blank excluded, equals the truth glyph. The winner is taken over every label the
recognition model has.

**What was run.** Stage 1 is the resolution axis (12 steps, 22 px per cell is the clean render) for all 31 codes,
30 seeds from 0, cells 0 and 1. Stage 2 is all six axes (55 distinct steps) with the same seeds and cells, for the
six codes below. All 37 runs exited 0; seed 0's line one started with the code in every run. A default run of the
tool (TD3, `P<`) reproduced the 54 rotation rows of the cells sweep of 2026-09-30 in every field, on this machine
with `RTEN_NUM_THREADS=4`. **Cells 0 and 1 only:** cell 2 is the issuing state's first letter, and the generator
draws the state per seed, so the tool, which keys each cell on seed 0's glyph, cannot measure it; cells 0 and 1
are measured over that varying context.

## The answer

Every rate is over n = 30 seeds per step with no missing recognition. A rate of 30 / 30 has a Wilson 95%
interval of 0.886-1.000.

- **Cell 0 (the first code letter) is read as itself in 30 / 30 at the clean render for all 31 codes** (Observed).
- **Cell 1 is read as itself in 30 / 30 for all 26 lettered codes** (16 TD3, 9 TD1 and `VC`) (Observed).
- **The filler `<` in cell 1 depends on the letter before it** (Observed, at the clean render):

  | Code | Format | Read as itself, of 30 [Wilson 95%] | Wrong winners |
  | --- | --- | --- | --- |
  | `P<` | TD3 | 0 [0.000, 0.114] | `S` 30 |
  | `V<` | MRV-A | 4 [0.053, 0.297] | `S` 26 |
  | `A<` | TD1 | 22 [0.556, 0.858] | `L` 7, `C` 1 |
  | `I<` | TD1 | 30 [0.886, 1.000] | none |
  | `C<` | TD1 | 30 [0.886, 1.000] | none |

  The `P<` count is consistent with the first sweep's 4 / 100 (seeds 0-99 contain seeds 0-29). Stage 2
  repeated stage 1's clean-render counts for `V<`, `A<`, `I<` and `C<` exactly.
- **Where the filler is misread at the clean render, it is misread on every axis.** For `V<` and `A<` cell 1 is
  below 27 of 30 at the first step of all six axes, so no degradation level is reached (Observed).
- **Where the filler is read (`I<`, `C<`), it ends before cell 0 on rotation (0.5 against 1.5 degrees) and blur
  (sigma 2.0 against 3 to 4); on resolution cell 0 ends first (9 px against 7).** These are the last steps read
  in at least 27 of 30, going down without a failing step. `I<` cell 1 is the only cell read at the clean render
  that fails on JPEG (last good quality 10, fails at 5) and noise (last good sigma 16, fails at 24) (Derived).
- **JPEG, noise and contrast never ended a read of a cell that is read at the clean render**, except for `I<`
  cell 1 above (Derived).

## Stage 1: all 31 codes, resolution axis, 30 seeds

`min px (contig / any)` is the smallest px per cell still read as itself in at least 27 of 30 seeds, going down from
22 without a failing step (contig) and at any step (any); `None` means no step reached 27. Wilson 95% in brackets.

| format | code | cell 0 at 22 px (95%) | cell 0 wrong winners | cell 0 min px (contig / any) | cell 1 at 22 px (95%) | cell 1 wrong winners | cell 1 min px (contig / any) |
|---|---|---|---|---|---|---|---|
| MRV-A | `V<` | V 30/30 (0.89-1.00) | - | 7.0 / 5.0 | < 4/30 (0.05-0.30) | S 26 | None / None |
| MRV-A | `VC` | V 30/30 (0.89-1.00) | - | 7.0 / 4.0 | C 30/30 (0.89-1.00) | - | 5.0 / 5.0 |
| TD1 | `A<` | A 30/30 (0.89-1.00) | - | 9.0 / 4.0 | < 22/30 (0.56-0.86) | L 7, C 1 | None / 7.0 |
| TD1 | `AC` | A 30/30 (0.89-1.00) | - | 4.0 / 4.0 | C 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD1 | `AR` | A 30/30 (0.89-1.00) | - | 4.0 / 4.0 | R 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD1 | `C<` | C 30/30 (0.89-1.00) | - | 9.0 / 5.0 | < 30/30 (0.89-1.00) | - | 7.0 / 7.0 |
| TD1 | `CA` | C 30/30 (0.89-1.00) | - | 4.0 / 4.0 | A 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD1 | `CB` | C 30/30 (0.89-1.00) | - | 5.0 / 5.0 | B 30/30 (0.89-1.00) | - | 6.0 / 4.0 |
| TD1 | `CL` | C 30/30 (0.89-1.00) | - | 5.0 / 5.0 | L 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD1 | `I<` | I 30/30 (0.89-1.00) | - | 9.0 / 5.0 | < 30/30 (0.89-1.00) | - | 7.0 / 7.0 |
| TD1 | `IC` | I 30/30 (0.89-1.00) | - | 5.0 / 5.0 | C 30/30 (0.89-1.00) | - | 5.0 / 5.0 |
| TD1 | `ID` | I 30/30 (0.89-1.00) | - | 5.0 / 5.0 | D 30/30 (0.89-1.00) | - | 5.0 / 5.0 |
| TD1 | `IO` | I 30/30 (0.89-1.00) | - | 7.0 / 5.0 | O 30/30 (0.89-1.00) | - | 8.0 / 5.0 |
| TD1 | `IP` | I 30/30 (0.89-1.00) | - | 5.0 / 5.0 | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD3 | `P<` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | < 0/30 (0.00-0.11) | S 30 | None / None |
| TD3 | `PA` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | A 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD3 | `PB` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | B 30/30 (0.89-1.00) | - | 5.0 / 5.0 |
| TD3 | `PC` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | C 30/30 (0.89-1.00) | - | 5.0 / 5.0 |
| TD3 | `PD` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | D 30/30 (0.89-1.00) | - | 5.0 / 5.0 |
| TD3 | `PE` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | E 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD3 | `PL` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | L 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD3 | `PM` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | M 30/30 (0.89-1.00) | - | 6.0 / 6.0 |
| TD3 | `PN` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | N 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD3 | `PO` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | O 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD3 | `PP` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD3 | `PR` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | R 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD3 | `PS` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | S 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD3 | `PT` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | T 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD3 | `PU` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | U 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD3 | `PV` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | V 30/30 (0.89-1.00) | - | 4.0 / 4.0 |
| TD3 | `PX` | P 30/30 (0.89-1.00) | - | 4.0 / 4.0 | X 30/30 (0.89-1.00) | - | 5.0 / 5.0 |

## Stage 2: six codes, all six axes, 30 seeds

Clean render = the first step of the resolution axis: the truth glyph, then the seeds out of 30 where it won.
Each axis column is the most severe step still read as itself in at least 27 of 30 seeds going down from the clean
render without a failing step (`all steps` = none failed) and the first step that fails; `none` = the first step
already fails. Units: resolution px per cell, JPEG quality, rotation degrees, blur and noise sigma, contrast ink
scale. `P<` was not rerun: the first sweep covers it at 100 seeds.

| format | code | cell | clean render | wrong winners at clean | resolution (px/cell) | jpeg (quality) | rotation (deg) | blur (sigma) | noise (sigma) | contrast (scale) |
|---|---|---|---|---|---|---|---|---|---|---|
| MRV-A | `V<` | 0 | V 30/30 | - | to 7.0 (fails at 6.0) | all steps | to 1.5 (fails at 2.0) | to 3.0 (fails at 4.0) | all steps | all steps |
| MRV-A | `V<` | 1 | < 4/30 | S 26 | none (fails at 22.0) | none (fails at 100.0) | none (fails at 0.0) | none (fails at 0.0) | none (fails at 0.0) | none (fails at 1.0) |
| TD1 | `A<` | 0 | A 30/30 | - | to 9.0 (fails at 8.0) | all steps | to 3.0 (fails at 4.0) | to 4.0 (fails at 5.0) | all steps | all steps |
| TD1 | `A<` | 1 | < 22/30 | L 7, C 1 | none (fails at 22.0) | none (fails at 100.0) | none (fails at 0.0) | none (fails at 0.0) | none (fails at 0.0) | none (fails at 1.0) |
| TD1 | `C<` | 0 | C 30/30 | - | to 9.0 (fails at 8.0) | all steps | to 1.5 (fails at 2.0) | to 4.0 (fails at 5.0) | all steps | all steps |
| TD1 | `C<` | 1 | < 30/30 | - | to 7.0 (fails at 6.0) | all steps | to 0.5 (fails at 1.0) | to 2.0 (fails at 2.5) | all steps | all steps |
| TD1 | `I<` | 0 | I 30/30 | - | to 9.0 (fails at 8.0) | all steps | to 1.5 (fails at 2.0) | to 3.0 (fails at 4.0) | all steps | all steps |
| TD1 | `I<` | 1 | < 30/30 | - | to 7.0 (fails at 6.0) | to 10.0 (fails at 5.0) | to 0.5 (fails at 1.0) | to 2.0 (fails at 2.5) | to 16.0 (fails at 24.0) | all steps |
| TD1 | `ID` | 0 | I 30/30 | - | to 5.0 (fails at 4.0) | all steps | to 2.0 (fails at 3.0) | to 3.0 (fails at 4.0) | all steps | all steps |
| TD1 | `ID` | 1 | D 30/30 | - | to 5.0 (fails at 4.0) | all steps | to 2.0 (fails at 3.0) | to 3.0 (fails at 4.0) | all steps | all steps |
| TD3 | `PP` | 0 | P 30/30 | - | all steps | all steps | to 2.0 (fails at 3.0) | to 4.0 (fails at 5.0) | all steps | all steps |
| TD3 | `PP` | 1 | P 30/30 | - | all steps | all steps | to 2.0 (fails at 3.0) | to 5.0 (fails at 6.0) | all steps | all steps |

## What this does not show

- Nothing about how the pipeline reads a real `P<` or `V<` page: the atlas reads an oracle crop, not a detected
  one, and the generator's glyph context is synthetic.
- Nothing about cell 2 or later, or about which glyph follows the letter in a real document, beyond the
  generator's seeded issuing-state letters.
- Nothing about codes outside the 31 (a full cell-1 sweep after `P`, `I`, `A`, `C` and `V` was not run).
- n = 30 per step: a step at 27 / 30 has a Wilson 95% lower bound of 0.74.
