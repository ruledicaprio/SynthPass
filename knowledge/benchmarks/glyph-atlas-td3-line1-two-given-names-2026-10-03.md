# Glyph atlas on TD3 line 1's name field with a second given name, 100 seeds on the desktop: the single `<` between two given names is read as itself in 28 of 100 clean renders and as `S` in 58, and a letter is still never read as the filler

**Date:** 2026-10-03 · **MAIN:** `8cdbffc` (PR 1's squash commit on `main`); the run was built from `3e061f7` (PR 1's head), and the diff `git diff 3e061f7 8cdbffc -- crates Cargo.lock` is not empty, and the run was not repeated on MAIN: it holds (a) the example's `--no-personal-number` flag ([#690](https://github.com/ruledicaprio/SynthPass/pull/690), merged to `main` first and then into PR 1's branch as `79ffea3`, off by default), and (b) golden render hashes and watermark tests under `crates/synthpass-gen/tests` with a `sha2` dev-dependency ([#693](https://github.com/ruledicaprio/SynthPass/pull/693), test-only, one `Cargo.lock` line); `git diff --stat 3e061f7 8cdbffc -- 'crates/*/src'` is empty, and the two-given-names code path is the one PR 1 held · **DATA:** none (generated renders, seeds 0-99; the vendored font `crates/synthpass-gen/fonts/ocr-b.ttf`, sha256 `367d876cca94`; models `f15cfb56bd02` (detection) and `e484866d4cce` (recognition), both pinned by `synthpass_ocr::verify`) · **Evidence:** Observed on synthetic renders, vendored font, oracle crop · **Status:** current

**2026-10-03.** [#688](https://github.com/ruledicaprio/SynthPass/pull/688) measured the name field of a TD3 line 1 but
could not measure the position [#575](https://github.com/ruledicaprio/SynthPass/issues/575) lists first, **a single `<`
between two name parts**, because the generator draws one given name and one surname (isolated n = 0 in 100 seeds). This
note measures it. `glyph_atlas --two-given-names` ([#691](https://github.com/ruledicaprio/SynthPass/pull/691), off by default) gives each seed's identity a second given name,
the given name of the identity drawn for `seed + 2^32`, so the name field reads *surname, `<<`, first given name, one
`<`, second given name, padding*; nothing else of the seed's identity changes. The settings are #688's
(`RTEN_NUM_THREADS=4`, cells 5-43, 100 seeds, the same six axes), so the two runs are a paired comparison on the same
seeds and machine. The atlas reads the oracle line crop, bypasses imageprep, the retry loop and the grid fit, and
records no decoded text. It says nothing about real documents, chargrid or the dropped filler-or-letter classifier. No
ADR, baseline or default changes with this note.

**The run.**

| | |
|---|---|
| Machine | the desktop, Windows; `RTEN_NUM_THREADS=4` (recorded in the summary header); one run at a time under `slot.ps1` |
| Tool | `glyph_atlas` release build of `3e061f7`, `schema` 3, ocrs 0.13.1, rten 0.26.0 |
| Default unchanged | a short default run (`--format td3 --line 1 --cells 5,...,43 --axes blur --seeds 2`) from a build of `origin/main` (`957d9d7`) and from `3e061f7`: **800 record rows and 540 summary rows identical**, the `ms` timing field aside; the header differs only in `args` (the two `--out` paths) |
| Preflight | the same run with `--two-given-names`: exit 0, 20 computed renders in 14.8 s; the truth line of each of the two seeds reads surname, `<<`, first given name, one `<`, second given name, then the tail, and the single `<` falls in cell 22 in both |
| Command | `glyph_atlas --format td3 --line 1 --two-given-names --cells 5,6,7,...,43 --axes resolution,jpeg,rotation,blur,noise,contrast --seeds 100 --seed-start 0` |
| Time | 12:07:13Z to 13:13:09Z, exit 0; 5,500 computed renders (5,900 rendered steps) in 3,953.2 s |
| Output | 23,541 summary rows, one per (cell, truth glyph, axis, step); `records.jsonl` is per-seed and stays local |

**Pooling.** A cell's 100 renders are split between letter and filler truths, so the tables rate a **position** (a
filler's neighbours, as in #688) or a cell's letters and fillers pooled, and say the n of every rate; Wilson 95%
intervals follow a rate. **Read as itself** is the first sweep's definition: the winner of the CTC distribution averaged
over the cell's timesteps, blank excluded, equals the truth glyph, over every label the model has. All cells have the class
`LetterOrFiller` in [`mrz::position_class`](../../crates/mrz/src/position.rs). **Letter truths and filler truths are
reported apart throughout.** The five filler positions: the single `<` (letters on both sides), the first and second `<`
of `<<`, the first `<` of the padding tail (a letter on its left) and the later tail cells (a `<` on the left).

## The answer

1. **Truth composition.** Over the 39 cells, 1,909 letter truths and 1,991 filler truths at the clean render. Cells
   5-10 hold a letter in all 100 seeds; cells 11-32 mix the two; cells 33-43 are `<` in all 100 seeds. **All 100
   seeds kept exactly one single `<`**: the emitter truncated none of the 100 combined names, and the single
   `<` falls in cells 17-26 (the table below gives the count per cell). **Fewer than 20 of a class:** fillers in
   cells 5-10, 16-20, 23-24 (cells 5-10 have none); letters in cells 29-43 (cells 33-43 have none). Those cells are rated with
   their n shown, and the position tables do not depend on them (Observed).

2. **The single `<` between the two given names.**
   - Clean render: **28 / 100 (28.0%, 20.1-37.5) read as `<`**; 59 / 100 (59.0%, 49.2-68.1) as a letter, **`S` in 58 of the 59** (`C` in 1); 13 / 100 (13.0%, 7.8-21.0)
     as a character outside the MRZ set (`?`, 13). #575's real hits are `S` x7, `C` x3, `Z` x2, `G` x1 in 13 cells.
   - Degraded steps (5,400 renders, 54 steps x 100 seeds): 1,558 / 5,400 (28.9%, 27.7-30.1) as `<`, 3,394 / 5,400 (62.9%, 61.6-64.1) as a letter,
     447 / 5,400 (8.3%, 7.6-9.0) outside the MRZ set. Of the 3,842 wrong renders the winners are `S` 2,366, `C` 803,
     `?` 369, `O` 93, space 53, `E` 50; **`Z` 7 and `G` 9** (Observed). `S` and `C` are #575's two
     commonest and are the two commonest here; `Z` and `G` are rare.
   - **Floors: none, on any axis**: the clean render already misses (28 of 100 against the 90 a floor needs).
   - Per axis, degraded steps pooled, the single `<` is read as `<` on resolution 25.8%, jpeg 47.1%, rotation 11.2%, blur
     7.7%, noise 31.9% and contrast 46.8% (table below); per axis and step with its winners, in the section on the single
     `<`. Per cell the n is small (1 to 28 renders of the single `<`); in the three cells with 17 or more (19, 21 and 22) it
     is read as `<` in 23.5%, 32.1% and 28.6% (Observed).

3. **Beside it, in the same run** (the positions split as #688 split them; the last column is #688's rate on the same
   seeds):

| Position of the filler | n | Read as `<` | Read as a letter | Outside the MRZ set | Winners | #688 (one given name): read as `<` |
|---|---:|---|---|---|---|---|
| the single `<` between the two given names (letters on both sides) | 100 | 28 / 100 (28.0%, 20.1-37.5) | 59 / 100 (59.0%, 49.2-68.1) | 13 / 100 (13.0%, 7.8-21.0) | `S` 58, `?` 13, `C` 1 | not drawn |
| first `<` of the `<<` separator | 100 | 12 / 100 (12.0%, 7.0-19.8) | 88 / 100 (88.0%, 80.2-93.0) | 0 / 100 (0.0%, 0.0-3.7) | `S` 88 | 5 / 100 (5.0%, 2.2-11.2) |
| second `<` of the separator | 100 | 54 / 100 (54.0%, 44.3-63.4) | 46 / 100 (46.0%, 36.6-55.7) | 0 / 100 (0.0%, 0.0-3.7) | `S` 46 | 55 / 100 (55.0%, 45.2-64.4) |
| first `<` of the padding tail | 100 | 59 / 100 (59.0%, 49.2-68.1) | 25 / 100 (25.0%, 17.5-34.3) | 16 / 100 (16.0%, 10.1-24.4) | `E` 20, `?` 16, `S` 4, `R` 1 | 47 / 100 (47.0%, 37.5-56.7) |
| later `<` of the padding tail | 1,591 | 1,477 / 1,591 (92.8%, 91.5-94.0) | 99 / 1,591 (6.2%, 5.1-7.5) | 15 / 1,591 (0.9%, 0.6-1.5) | `S` 67, `E` 30, `-` 13, `C` 2 | 2,181 / 2,228 (97.9%, 97.2-98.4) |

   Over the degraded steps:

| Position of the filler | Degraded renders | Read as `<` | Read as a letter | Outside the MRZ set | Commonest winners |
|---|---:|---|---|---|---|
| the single `<` between the two given names (letters on both sides) | 5,400 | 28.9% | 62.9% | 8.3% | `S` 2,366, `C` 803, `?` 369, `O` 93, space 53 |
| first `<` of the `<<` separator | 5,400 | 31.4% | 60.0% | 8.6% | `S` 2,261, `C` 569, `E` 273, `?` 231, space 157 |
| second `<` of the separator | 5,400 | 51.6% | 45.8% | 2.5% | `S` 1,469, `C` 745, `O` 91, `E` 66, `K` 52 |
| first `<` of the padding tail | 5,400 | 47.5% | 46.0% | 6.4% | `E` 830, `S` 715, `C` 587, `?` 308, `R` 174 |
| later `<` of the padding tail | 85,914 | 76.1% | 23.2% | 0.7% | `E` 13,149, `C` 3,433, `S` 1,791, `K` 987, `R` 387 |

   Rate read as `<` by axis, degraded steps pooled:

| Position | resolution | jpeg | rotation | blur | noise | contrast |
|---|---:|---:|---:|---:|---:|---:|
| the single `<` between the two given names (letters on both sides) | 25.8% | 47.1% | 11.2% | 7.7% | 31.9% | 46.8% |
| first `<` of the `<<` separator | 29.5% | 42.8% | 1.1% | 6.6% | 36.0% | 72.6% |
| second `<` of the separator | 58.1% | 67.6% | 13.0% | 23.7% | 58.1% | 85.1% |
| first `<` of the padding tail | 63.5% | 70.5% | 46.9% | 28.1% | 25.9% | 35.2% |
| later `<` of the padding tail | 86.5% | 94.7% | 56.0% | 50.5% | 77.5% | 83.7% |

   Floors by position:

| Position | resolution | jpeg | rotation | blur | noise | contrast |
|---|---|---|---|---|---|---|
| the single `<` between the two given names (letters on both sides) | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| first `<` of the `<<` separator | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| second `<` of the separator | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| first `<` of the padding tail | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| later `<` of the padding tail | to 6 (fails at 5) | to 30 (fails at 20) | to 0.5 (fails at 1) | to 0 (fails at 0.5) | to 0 (fails at 2) | to 1 (fails at 0.8) |

   Observed: the single `<` reads as itself less often than every position but the first `<` of the separator (28%
   against 12%, 54%, 59% and 93%), and its wrong winner is `S` as for the separator. The two weakest positions at the
   clean render, the first `<` of the separator (12%) and the single `<` (28%), both have a letter on the left; the tail
   head, also with a letter on its left, reads 59%, and the second `<` of the separator, with a letter on its right only,
   54%. Which neighbour matters is not separable from these data (the positions also differ in where they sit on the
   line), and only the later tail, with a `<` on both sides, holds above 90% (Hypothesized, untested).

4. **The paired control against #688** (same machine, same seeds 0-99, same cells; the surname, the `<<` and the first
   given name hold the same truth in the same cells in both runs, and only the text after the first given name differs).
   The comparison is per render: the same seed, cell, axis and step in the two runs.

| Part of the line (same seeds in both runs) | Renders compared | Same winner | Read as itself, #688 | Read as itself, this run |
|---|---:|---:|---|---|
| clean: surname cells | 809 | 809 / 809 (100.0%, 99.5-100.0) | 100.0% (809) | 100.0% (809) |
| clean: first `<` of `<<` | 100 | 91 / 100 (91.0%, 83.8-95.2) | 5.0% (5) | 12.0% (12) |
| clean: second `<` of `<<` | 100 | 83 / 100 (83.0%, 74.5-89.1) | 55.0% (55) | 54.0% (54) |
| clean: first given name | 563 | 559 / 563 (99.3%, 98.2-99.7) | 99.3% (559) | 100.0% (563) |
| clean: the filler that ends the first name (a tail head in #688, the single `<` here) | 100 | 26 / 100 (26.0%, 18.4-35.4) | 47.0% (47) | 28.0% (28) |
| degraded: surname cells | 43,686 | 43,079 / 43,686 (98.6%, 98.5-98.7) | 94.7% (41,368) | 95.0% (41,491) |
| degraded: first `<` of `<<` | 5,400 | 4,349 / 5,400 (80.5%, 79.5-81.6) | 26.7% (1,443) | 31.4% (1,696) |
| degraded: second `<` of `<<` | 5,400 | 4,551 / 5,400 (84.3%, 83.3-85.2) | 51.4% (2,776) | 51.6% (2,788) |
| degraded: first given name | 30,402 | 29,661 / 30,402 (97.6%, 97.4-97.7) | 94.6% (28,766) | 95.3% (28,973) |
| degraded: the filler that ends the first name (a tail head in #688, the single `<` here) | 5,400 | 2,402 / 5,400 (44.5%, 43.2-45.8) | 33.9% (1,828) | 28.9% (1,558) |

   Observed: the **surname cells read exactly the same in all 809 clean renders** (809 / 809 (100.0%, 99.5-100.0)) and 43,079 / 43,686 (98.6%, 98.5-98.7) of the degraded
   renders; the first given name's cells agree in 559 / 563 (99.3%, 98.2-99.7) clean and 29,661 / 30,402 (97.6%, 97.4-97.7) degraded renders. The `<<` is where a
   line-context effect appears: the winner of its first `<` differs between the runs in 91 / 100 (91.0%, 83.8-95.2) of clean renders
   (`S` -> `<` in 7 of the 100, `<` -> `S` in 1; **#688 read it as itself in 5 of 100, this run in 12 / 100 (12.0%, 7.0-19.8)**, an
   interval overlap, not a clean separation) and of its second in 83 / 100 (83.0%, 74.5-89.1); degraded, the agreement is 80.5% and
   84.3%. The whole first given name sits between these cells and the first changed cell, so the recogniser reads them
   differently only because of what comes later on the line. The filler that ends the first name is a different
   thing in the two runs (the tail head in #688, with a `<` on its right; the single `<` here, with a letter on its right):
   its winner agrees in 26 of 100 clean renders, and it reads as `<` in 47 of 100 in #688 and 28 of 100 here.
   Hypothesized, untested: the model reads a whole line, so the text after a cell changes how it reads the cell.

5. **A letter read as the filler.** **Still never: 0 of 1,909 clean and 0 of 103,086
   degraded letter renders have `<` as the winner** (#688: 0 of 1,372 and 0 of 74,088), in every cell and on every axis.
   Letters are read as themselves in 1,909 / 1,909 (100.0%, 99.8-100.0) at the clean render and 98,351 / 103,086 (95.4%, 95.3-95.5) over the degraded steps; the swaps
   are between letters: `I` read as `T` 308, `I` read as `E` 254, `N` read as `S` 237, `M` read as `R` 185, `A` read as `S` 181, `K` read as `E` 180. Letters here are the surname and the two given names, not the same set of cells as
   #688's, so the totals differ.

6. **The class split.** Every cell is `LetterOrFiller`, so a winner is in class when it is a letter or `<`. Degraded steps:

| Truth | Degraded renders | Wrong | Winner in class (a letter or `<`) | An MRZ digit | Outside the MRZ set |
|---|---:|---:|---:|---:|---:|
| letter | 103,086 | 4,735 (4.6%) | 4,348 (91.8%) | 18 (0.4%) | 369 (7.8%) |
| filler `<` | 107,514 | 33,565 (31.2%) | 31,534 (93.9%) | 27 (0.1%) | 2,004 (6.0%) |

   The wrong winners of a filler are a letter in 93.9% of cases (in class), an MRZ digit in 27 of 33,565, and outside the
   MRZ set in 6.0%; the commonest, clean: `S` 263, `E` 50, `?` 30, `-` 13, `C` 3, `R` 1; degraded: `E` 14,368, `S` 8,602, `C` 6,137, `?` 1,321, `K` 1,118, `R` 619, space 262, `O` 216. The commonest letter swaps, degraded:
   `I` read as `T` 308, `I` read as `E` 254, `N` read as `S` 237, `M` read as `R` 185, `A` read as `S` 181, `K` read as `E` 180; clean: none (Observed).

7. **What this does not show** is in the last section. Every derived statement is labelled Observed or Hypothesized.
   These numbers are not set beside any other machine's atlas runs; the tool's module doc forbids it.

## The single `<` per cell, and per axis and step

Cells with a single `<` at the clean render (each seed has one):

| Cell | Single `<` renders | Read as `<` | Winners |
|---|---:|---|---|
| 17 | 4 | 1 / 4 (25.0%, 4.6-69.9) | S 3 |
| 18 | 8 | 0 / 8 (0.0%, 0.0-32.4) | S 5, outside 3 |
| 19 | 17 | 4 / 17 (23.5%, 9.6-47.3) | S 12, outside 1 |
| 20 | 10 | 4 / 10 (40.0%, 16.8-68.7) | S 5, outside 1 |
| 21 | 28 | 9 / 28 (32.1%, 17.9-50.7) | S 12, outside 7 |
| 22 | 21 | 6 / 21 (28.6%, 13.8-50.0) | S 14, outside 1 |
| 23 | 6 | 2 / 6 (33.3%, 9.7-70.0) | S 4 |
| 24 | 3 | 1 / 3 (33.3%, 6.1-79.2) | S 2 |
| 25 | 1 | 0 / 1 (0.0%, 0.0-79.3) | S 1 |
| 26 | 2 | 1 / 2 (50.0%, 9.5-90.5) | S 1 |

Per axis and step, the single `<` over its 100 renders (the identity steps of each axis repeat the clean render; "winners" are the commonest three):

| Axis | Step | n | Read as `<` | Read as a letter | Outside the MRZ set | Winners |
|---|---:|---:|---:|---:|---:|---|
| resolution | 22 | 100 | 28.0% | 59.0% | 13.0% | `S` 58, `?` 13, `C` 1 |
| resolution | 18 | 100 | 46.0% | 53.0% | 1.0% | `S` 53, `?` 1 |
| resolution | 15 | 100 | 26.0% | 72.0% | 2.0% | `S` 72, `?` 2 |
| resolution | 13 | 100 | 35.0% | 58.0% | 7.0% | `S` 56, `?` 5, `C` 2 |
| resolution | 11 | 100 | 8.0% | 76.0% | 16.0% | `S` 74, `?` 16, `C` 2 |
| resolution | 10 | 100 | 16.0% | 81.0% | 3.0% | `S` 76, `C` 5, `?` 3 |
| resolution | 9 | 100 | 51.0% | 46.0% | 3.0% | `S` 37, `C` 9, `?` 3 |
| resolution | 8 | 100 | 54.0% | 45.0% | 1.0% | `C` 27, `S` 15, `R` 2 |
| resolution | 7 | 100 | 45.0% | 53.0% | 2.0% | `C` 32, `S` 21, `?` 2 |
| resolution | 6 | 100 | 3.0% | 97.0% | 0.0% | `C` 74, `S` 23 |
| resolution | 5 | 100 | 0.0% | 100.0% | 0.0% | `C` 86, `S` 13, `E` 1 |
| resolution | 4 | 100 | 0.0% | 99.0% | 1.0% | `C` 99, space 1 |
| jpeg | 100 | 100 | 30.0% | 57.0% | 13.0% | `S` 56, `?` 13, `C` 1 |
| jpeg | 90 | 100 | 39.0% | 50.0% | 11.0% | `S` 49, `?` 11, `C` 1 |
| jpeg | 75 | 100 | 43.0% | 48.0% | 9.0% | `S` 47, `?` 9, `C` 1 |
| jpeg | 60 | 100 | 49.0% | 45.0% | 6.0% | `S` 44, `?` 6, `C` 1 |
| jpeg | 50 | 100 | 52.0% | 38.0% | 10.0% | `S` 37, `?` 10, `C` 1 |
| jpeg | 40 | 100 | 52.0% | 40.0% | 8.0% | `S` 37, `?` 8, `C` 3 |
| jpeg | 30 | 100 | 50.0% | 45.0% | 5.0% | `S` 41, `?` 5, `C` 3 |
| jpeg | 20 | 100 | 58.0% | 37.0% | 5.0% | `S` 25, `C` 10, `?` 5 |
| jpeg | 15 | 100 | 65.0% | 30.0% | 5.0% | `S` 19, `C` 10, `?` 4 |
| jpeg | 10 | 100 | 61.0% | 26.0% | 13.0% | `S` 19, `?` 7, `C` 7 |
| jpeg | 5 | 100 | 19.0% | 70.0% | 11.0% | `S` 66, `?` 10, `C` 4 |
| rotation | 0 | 100 | 28.0% | 59.0% | 13.0% | `S` 58, `?` 13, `C` 1 |
| rotation | 0.25 | 100 | 21.0% | 76.0% | 3.0% | `S` 76, `?` 3 |
| rotation | 0.5 | 100 | 27.0% | 70.0% | 3.0% | `S` 70, space 2, `?` 1 |
| rotation | 1 | 100 | 15.0% | 82.0% | 3.0% | `S` 81, `?` 2, space 1 |
| rotation | 1.5 | 100 | 18.0% | 81.0% | 1.0% | `S` 81, `?` 1 |
| rotation | 2 | 100 | 6.0% | 92.0% | 2.0% | `S` 84, `C` 3, `K` 3 |
| rotation | 3 | 100 | 1.0% | 98.0% | 1.0% | `S` 61, `C` 24, `K` 7 |
| rotation | 4 | 100 | 2.0% | 98.0% | 0.0% | `S` 66, `K` 8, `R` 7 |
| rotation | 5 | 100 | 0.0% | 98.0% | 2.0% | `S` 58, `E` 19, `T` 6 |
| blur | 0 | 100 | 28.0% | 59.0% | 13.0% | `S` 58, `?` 13, `C` 1 |
| blur | 0.5 | 100 | 32.0% | 56.0% | 12.0% | `S` 56, `?` 12 |
| blur | 1 | 100 | 22.0% | 57.0% | 21.0% | `S` 54, `?` 21, `C` 3 |
| blur | 1.5 | 100 | 11.0% | 74.0% | 15.0% | `S` 67, `?` 15, `C` 7 |
| blur | 2 | 100 | 4.0% | 92.0% | 4.0% | `S` 67, `C` 25, `?` 4 |
| blur | 2.5 | 100 | 0.0% | 100.0% | 0.0% | `C` 75, `S` 25 |
| blur | 3 | 100 | 0.0% | 100.0% | 0.0% | `C` 98, `S` 2 |
| blur | 4 | 100 | 0.0% | 100.0% | 0.0% | `C` 96, `O` 4 |
| blur | 5 | 100 | 0.0% | 95.0% | 5.0% | `O` 46, `C` 28, `E` 11 |
| blur | 6 | 100 | 0.0% | 61.0% | 39.0% | `O` 43, `?` 27, `c` 11 |
| noise | 0 | 100 | 28.0% | 59.0% | 13.0% | `S` 58, `?` 13, `C` 1 |
| noise | 2 | 100 | 29.0% | 61.0% | 10.0% | `S` 61, `?` 10 |
| noise | 4 | 100 | 32.0% | 62.0% | 6.0% | `S` 62, `?` 6 |
| noise | 8 | 100 | 37.0% | 56.0% | 7.0% | `S` 48, `?` 7, `C` 7 |
| noise | 12 | 100 | 34.0% | 56.0% | 10.0% | `S` 45, `C` 11, `?` 8 |
| noise | 16 | 100 | 36.0% | 57.0% | 7.0% | `S` 44, `C` 11, `?` 5 |
| noise | 24 | 100 | 25.0% | 57.0% | 17.0% | `S` 46, space 11, `C` 9 |
| noise | 32 | 100 | 30.0% | 56.0% | 14.0% | `S` 37, `C` 15, space 9 |
| contrast | 1 | 100 | 28.0% | 59.0% | 13.0% | `S` 58, `?` 13, `C` 1 |
| contrast | 0.8 | 100 | 51.0% | 34.0% | 15.0% | `S` 33, `?` 15, `C` 1 |
| contrast | 0.6 | 100 | 57.0% | 25.0% | 18.0% | `S` 25, `?` 15, space 3 |
| contrast | 0.5 | 100 | 60.0% | 24.0% | 16.0% | `S` 24, `?` 15, space 1 |
| contrast | 0.4 | 100 | 53.0% | 28.0% | 19.0% | `S` 28, `?` 15, space 3 |
| contrast | 0.3 | 100 | 44.0% | 38.0% | 18.0% | `S` 38, `?` 17, `-` 1 |
| contrast | 0.2 | 100 | 38.0% | 40.0% | 22.0% | `S` 40, `?` 20, space 2 |
| contrast | 0.15 | 100 | 37.0% | 44.0% | 19.0% | `S` 43, `?` 16, space 3 |
| contrast | 0.1 | 100 | 34.0% | 60.0% | 6.0% | `S` 60, `?` 5, space 1 |

## Truth composition per cell

| Cell | Letter truths | Filler truths | of which the single `<` |
|---|---:|---:|---:|
| 5 | 100 | 0 | - |
| 6 | 100 | 0 | - |
| 7 | 100 | 0 | - |
| 8 | 100 | 0 | - |
| 9 | 100 | 0 | - |
| 10 | 100 | 0 | - |
| 11 | 80 | 20 | - |
| 12 | 64 | 36 | - |
| 13 | 74 | 26 | - |
| 14 | 47 | 53 | - |
| 15 | 46 | 54 | - |
| 16 | 89 | 11 | - |
| 17 | 96 | 4 | 4 |
| 18 | 92 | 8 | 8 |
| 19 | 83 | 17 | 17 |
| 20 | 90 | 10 | 10 |
| 21 | 72 | 28 | 28 |
| 22 | 79 | 21 | 21 |
| 23 | 90 | 10 | 6 |
| 24 | 82 | 18 | 3 |
| 25 | 68 | 32 | 1 |
| 26 | 56 | 44 | 2 |
| 27 | 39 | 61 | - |
| 28 | 28 | 72 | - |
| 29 | 16 | 84 | - |
| 30 | 10 | 90 | - |
| 31 | 6 | 94 | - |
| 32 | 2 | 98 | - |
| 33 | 0 | 100 | - |
| 34 | 0 | 100 | - |
| 35 | 0 | 100 | - |
| 36 | 0 | 100 | - |
| 37 | 0 | 100 | - |
| 38 | 0 | 100 | - |
| 39 | 0 | 100 | - |
| 40 | 0 | 100 | - |
| 41 | 0 | 100 | - |
| 42 | 0 | 100 | - |
| 43 | 0 | 100 | - |

## What this does not show

- Nothing about real documents or the shipped pipeline; the atlas reads an oracle crop of a synthetic line with a
  vendored font. Nothing about real names, chargrid, or the dropped filler-or-letter classifier of #650. No code, ADR,
  baseline or default changes.
- **One extra given name drawn from the same pools.** It is another synthetic first name, not a real double name, an
  apostrophe or a hyphen; #575's real single `<` can come from any of those. The single `<` here always has two letters
  of the generator's own names on each side.
- The positions have 100 renders each (the later tail 1,591), so the intervals are wide for the single `<` (20.1-37.5%
  for 28 of 100). The single `<` is spread over cells 17-26, so a per-cell rate has 1 to 28 renders.
- The line-context effect in answer 4 is measured for these cells and this change only (a second name added after the
  first); it is not a model of how the recogniser's reading depends on the line.
- One run, one machine, one thread setting; its spread is not measured. Axes are applied one at a time, never composed.
