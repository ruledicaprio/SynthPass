# Glyph atlas on TD3 line 1's name field (cells 5-43), 100 seeds on the desktop: the first `<` of the `<<` separator is read as `S` in 94 of 100 clean renders, a letter is never read as the filler, and the generator draws no single `<` between letters

**Date:** 2026-10-03 · **MAIN:** `0085cc8` (`origin/main` when the run started; the tool is [#673](https://github.com/ruledicaprio/SynthPass/pull/673)'s `glyph_atlas`, unchanged; the summary header says `dirty: false`) · **DATA:** none (generated renders, seeds 0-99; the vendored font `crates/synthpass-gen/fonts/ocr-b.ttf`, sha256 `367d876cca94`; models `f15cfb56bd02` (detection) and `e484866d4cce` (recognition), both pinned by `synthpass_ocr::verify`) · **Evidence:** Observed on synthetic renders, vendored font, oracle crop · **Status:** current

**2026-10-03.** This is the measurement half of [#575](https://github.com/ruledicaprio/SynthPass/issues/575): how the
recogniser itself reads the name field of a TD3 line 1, cell by cell, under one controlled degradation at a time.
#575's premise is that the main real-name error is a printed filler read as a letter: on 21 wrong-name Tier-1 hits, a
single `<` was read as a letter in 10 documents (13 cells: `S` x7, `C` x3, `Z` x2, `G` x1) and the filler tail was read
as letters, mostly `C`, in 9 documents, and no check digit sees either. Line 1 had only been swept at cells 0 and 1
(`P<`: [#624](https://github.com/ruledicaprio/SynthPass/pull/624), [#660](https://github.com/ruledicaprio/SynthPass/pull/660),
and the control of [#676](https://github.com/ruledicaprio/SynthPass/pull/676) on this machine). This sweep covers
**cells 5-43, the name field**, whose truth varies per seed, so both directions are measurable: the filler read as a
letter, and a letter read as the filler. The settings are the desktop atlas runs' (`RTEN_NUM_THREADS=4`), so it sits
beside #676 and its control. The atlas reads the oracle line crop, bypasses imageprep, the retry loop and the grid fit,
and records no decoded text. It says nothing about real documents, chargrid or the dropped filler-or-letter classifier
([#650](https://github.com/ruledicaprio/SynthPass/pull/650)). No code, ADR, baseline or default changes with this note.

**The run.**

| | |
|---|---|
| Machine | the desktop, Windows; `RTEN_NUM_THREADS=4` (recorded in the summary header); one run at a time under `slot.ps1` |
| Tool | `glyph_atlas` release build, `schema` 3, ocrs 0.13.1, rten 0.26.0; `Cargo.lock` of `0085cc8` |
| Preflight | `--axes blur --seeds 2 --seed-start 0`, the same cells: exit 0, 20 computed renders in 14.6 s. Its summary header listed letters and `<` in cells 11, 12, 14 and 15, letters only in the other cells up to 21 (two seeds), and `<` alone in cells 22-43, as the brief expected |
| Command | `glyph_atlas --format td3 --line 1 --cells 5,6,7,...,43 --axes resolution,jpeg,rotation,blur,noise,contrast --seeds 100 --seed-start 0` |
| Time | 08:48:54Z to 09:56:55Z, exit 0; 5,500 computed renders (5,900 rendered steps) in 4,079.4 s |
| Output | 16,756 summary rows, one per (cell, truth glyph, axis, step); `records.jsonl` is per-seed and stays local |

**Truth composition.** Cells 5-10 hold a letter in every one of the 100 seeds. From cell 11 on the filler appears, and
**from cell 26 on every cell is `<` in all 100 seeds**. Nothing but letters and `<` occurs in any cell (no digit or other
character in any cell of the 100 seeds). The 100 identities have exactly one surname and one given name each
(`crates/synthpass-gen/src/data.rs` lines 211-223 draw one name from each pool), so the name field is *surname, `<<`,
given name, padding*. **There is no single `<` between two letters in any of the 100 renders.** The one separator is
always the pair `<<`, and the padding tail always follows the given name. #575's real single `<` therefore cannot be drawn here; item 4 below uses the
four positions the generator does produce. The `<` of `P<` in cell 1 is the only single filler between two letters in
the tool, and it has been swept (#660, #676).

**Pooling.** A cell's 100 renders are split between letter and filler truths, and its letters are split among their
glyphs, so one (cell, glyph) pair has a handful. The tables **pool a cell's letters and a cell's fillers** and say the n
of every pooled rate; Wilson 95% intervals are written after a rate. **Read as itself** is the first sweep's definition:
the winner of the CTC distribution averaged over the cell's timesteps, blank excluded, equals the truth glyph, over every
label the model has. All cells here have the class `LetterOrFiller` in
[`mrz::position_class`](../../crates/mrz/src/position.rs). **Letter truths and filler truths are reported apart
throughout.**

## The answer

1. **Truth composition (clean render, 100 renders per cell).** In all, 1,372 letter truths and 2,528 filler truths
   over the 39 cells. Cells 5-10: 100 letters and no filler. Cells 11-25 mix the two (cell 11: 80 letters and 20 fillers;
   cell 14: 47 and 53; cell 21: 33 and 67). Cells 26-43: 100 fillers each, no letter. **Fewer than 20 of a class:**
   fillers in cells 5-10 (0), 16 (11), 17 (4) and 18 (12); letters in cells 22-25 (12, 6, 3, 2) and 26-43 (0). Those cells
   are rated with their n shown, and the floors tables leave out a class with fewer than 20 (Observed).

2. **The filler read as a letter.**
   - **At the clean render 2,288 / 2,528 (90.5%, 89.3-91.6) of the filler truths are read as `<`**; 207 / 2,528 (8.2%, 7.2-9.3) are read as a
     letter and 33 / 2,528 (1.3%, 0.9-1.8) as a character outside the MRZ set (`?`, `s`, `-`). The clean misses gather in three
     places, not everywhere (table in answer 4): **the first `<` of the `<<` separator is read as `S` in 94 of 100
     renders** (5 / 100 as itself), the second `<` as itself in 55 of 100 (a letter in 44, `S` in 43), and the first `<`
     of the padding tail as itself in 47 of 100 (a letter in 34, `E` in 26, `?` in 19). **The later padding cells read as
     `<` in 97.9% of 2,228 renders.**
   - The letters that win a clean filler are `S` 165, `E` 41 and `C` 1 (of the 207 letter winners), and the rest `?` 21,
     `s` 9 and `-` 3. **Over the degraded steps 136,512 filler renders: 91,726 / 136,512 (67.2%, 66.9-67.4) read as `<`, 41,911 / 136,512 (30.7%, 30.5-30.9)
     as a letter, 2,870 / 136,512 (2.1%, 2.0-2.2) outside the MRZ set.** The winning letters over the degraded steps (41,911 letter
     winners of 44,786 wrong): `E` 24,076, `S` 8,673, `C` 6,518, then `K`, `R`, `T`, `I`, `O`, `N`, `L`,
     with `Z` 53 and `G` 48. #575's real `Z` and `G` are the rare ones here; its `S` and `C` are common. The
     most frequent wrong winner is `E`, which #575 did not see in the real hits (Observed). Hypothesized, untested: the
     tool's own convention folds ocrs's mangled class (labels 44 and 49) into `E`, and a blurred `<` may be landing on
     it; the atlas does not break the sum apart.
   - **By axis** (steps and rates in the pooled per-step table below, all filler truths): **rotation is the axis with the
     lowest rate of `<` in all four positions** (47.8% in the later tail down to 1.1% for the first `<` of the separator,
     over the degraded steps; table in answer 4). **Floors** (a floor is the most severe step still read as itself in at least 90 of a cell's renders): of the
     30 cells with 20 or more fillers, **cells 11-15, 19-22 and 32 have no floor on any axis, the clean render already
     misses**; cell 23 has one, on jpeg only (to 90, fails at 75); cells 24-31 and 33-43 hold to between 18 and 5 px per
     cell, to between 1.5 and 0.5 degrees of rotation and to between 0 and 2.5 sigma of blur; on contrast cells 24-28 and
     30 hold to 1, cells 29 and 31 to 0.8, cell 33 to 0.5, and cells 34-43 through every step (Observed; the table is
     below).

3. **A letter read as the filler (the reverse).** **Never: 0 of 1,372 clean letter renders and 0 of 74,088
   degraded letter renders have `<` as the winner**, on any axis and in any cell. Letters are read as themselves in
   1,368 / 1,372 (99.7%, 99.3-99.9) at the clean render (the four misses are `Q` read as `O` twice and as `R` once, and one `Z` read as
   `?`) and in 70,134 / 74,088 (94.7%, 94.5-94.8) over the degraded steps. The swaps that do occur over the degraded steps are between
   letters: `I` read as `T` 254, `N` read as `S` 183, `I` read as `E` 148, `A` read as `S` 136, `R` read as `S` 132, `K` read as `E` 129, `M` read as `R` 106, `O` read as `S` 95. So on line 1's name field the swap #537 found on line 2 (a letter read as the filler) is not
   present in these 100 seeds, and a degraded letter is never *absorbed* into a filler run. Hypothesized, untested: that
   follows from the model giving `<` a distinct, strong shape; the atlas shows the outcome, not the cause.

4. **A single filler against a filler in a run.** The split #575 drew on real documents (a single `<` against the
   tail) **cannot be drawn: isolated n = 0**, for the reason under *Truth composition*. What the generator does produce
   splits by the filler's neighbours into four positions, and they read very differently at the clean render:

| Position of the filler | Clean n | Read as `<` | Read as a letter | Outside the MRZ set | Winners (clean) |
|---|---:|---|---|---|---|
| first `<` of the `<<` separator (letter on the left, `<` on the right) | 100 | 5 / 100 (5.0%, 2.2-11.2) | 94 / 100 (94.0%, 87.5-97.2) | 1 / 100 (1.0%, 0.2-5.4) | `S` 94, `?` 1 |
| second `<` of the separator (`<` on the left, letter on the right) | 100 | 55 / 100 (55.0%, 45.2-64.4) | 44 / 100 (44.0%, 34.7-53.8) | 1 / 100 (1.0%, 0.2-5.4) | `S` 43, `?` 1, `C` 1 |
| first `<` of the padding tail (letter on the left) | 100 | 47 / 100 (47.0%, 37.5-56.7) | 34 / 100 (34.0%, 25.5-43.7) | 19 / 100 (19.0%, 12.5-27.8) | `E` 26, `?` 19, `S` 8 |
| later `<` of the padding tail (`<` on the left) | 2,228 | 2,181 / 2,228 (97.9%, 97.2-98.4) | 35 / 2,228 (1.6%, 1.1-2.2) | 12 / 2,228 (0.5%, 0.3-0.9) | `S` 20, `E` 15, `s` 9, `-` 3 |

   Over the degraded steps (the 54 non-identity steps; letters are `A`-`Z`, outside is anything not in the MRZ set):

| Position of the filler | Degraded renders | Read as `<` | Read as a letter | Outside the MRZ set | Commonest winners |
|---|---:|---|---|---|---|
| first `<` of the `<<` separator (letter on the left, `<` on the right) | 5,400 | 26.7% | 62.5% | 10.8% | `S` 2357, `C` 568, `?` 322, `E` 311, space 155 |
| second `<` of the separator (`<` on the left, letter on the right) | 5,400 | 51.4% | 45.7% | 2.9% | `S` 1401, `C` 787, `E` 98, `O` 80, `?` 63 |
| first `<` of the padding tail (letter on the left) | 5,400 | 33.9% | 56.6% | 9.5% | `S` 1124, `E` 988, `C` 646, `?` 416, `R` 151 |
| later `<` of the padding tail (`<` on the left) | 120,312 | 71.2% | 27.4% | 1.3% | `E` 22679, `C` 4517, `S` 3791, `K` 1049, `c` 834 |

   Rate read as `<` by axis, degraded steps pooled:

| Position | resolution | jpeg | rotation | blur | noise | contrast |
|---|---:|---:|---:|---:|---:|---:|
| first `<` of the `<<` separator (letter on the left, `<` on the right) | 28.6% | 38.8% | 1.1% | 4.6% | 27.0% | 57.8% |
| second `<` of the separator (`<` on the left, letter on the right) | 56.6% | 70.6% | 13.2% | 22.3% | 56.7% | 84.0% |
| first `<` of the padding tail (letter on the left) | 53.2% | 60.9% | 13.8% | 24.4% | 16.4% | 16.0% |
| later `<` of the padding tail (`<` on the left) | 83.1% | 95.2% | 47.8% | 50.6% | 66.4% | 72.7% |

   Floors by position (the first three fail at the clean render; the later tail cells hold):

| Position | resolution | jpeg | rotation | blur | noise | contrast |
|---|---|---|---|---|---|---|
| first `<` of the `<<` separator (letter on the left, `<` on the right) | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| second `<` of the separator (`<` on the left, letter on the right) | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| first `<` of the padding tail (letter on the left) | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| later `<` of the padding tail (`<` on the left) | to 7 (fails at 6) | to 30 (fails at 20) | to 0.5 (fails at 1) | to 2 (fails at 2.5) | to 2 (fails at 4) | to 0.8 (fails at 0.6) |

   Observed: **a letter on the left is not what decides it**: the first `<` of the pair and the head of the tail both
   have a letter on the left and read as themselves 5% and 47% of the time; the later tail cells, with a `<` on the left,
   read as themselves 97.9%. The second `<` of the pair (a `<` on the left, a letter on the right) reads 55%. `S` is the
   winner of the separator (2,357 of 3,957 wrong renders of its first `<` and 1,401 of 2,624 of its second
   over the degraded steps), and `E` the commonest winner of the later tail (22,679 of its 34,633 wrong renders, against
   `S` 3,791 of 34,633).
   The tail head is the cell where `?`, a character outside the MRZ set, is most common at the clean render (19 of 100).

5. **The class split.** Every cell is `LetterOrFiller`, so a winner is *in class* when it is a letter or `<`. Over the
   degraded steps:

| Truth | Degraded renders | Wrong | Winner in class (a letter or `<`) | An MRZ digit | Outside the MRZ set |
|---|---:|---:|---:|---:|---:|
| letter | 74,088 | 3,954 (5.3%) | 3,579 (90.5%) | 9 (0.2%) | 366 (9.3%) |
| filler `<` | 136,512 | 44,786 (32.8%) | 41,911 (93.6%) | 5 (0.0%) | 2,870 (6.4%) |

   **93.6% of the wrong winners of a filler are a letter (in class)**: the class the position allows (a letter or
   `<`) cannot see them. Only 5 of the 44,786 are an MRZ digit. The wrong letters are mostly in class too (90.5%),
   and 366 (9.3%) are outside the MRZ set (space, `$`, `?`, `@`, lower case). The commonest wrong winners of a filler,
   clean: `S` 165, `E` 41, `?` 21, `s` 9, `-` 3, `C` 1; degraded: `E` 24076, `S` 8673, `C` 6518, `?` 1449, `K` 1147, `c` 1062, `R` 860, space 182, `T` 144, `I` 112. The commonest letter swaps, clean: `Q` as `O` 2, `Q` as `R` 1, `Z` as `?` 1; degraded: `I` read as `T` 254, `N` read as `S` 183, `I` read as `E` 148, `A` read as `S` 136, `R` read as `S` 132, `K` read as `E` 129, `M` read as `R` 106, `O` read as `S` 95.

6. **Beside this machine's own numbers only: the `P<` of the #676 control** (same machine, 100 seeds, same settings;
   the control's commit is `3a9b0c2`, the tool unchanged). At the clean render `P` in cell 0 is read as `P` in
   100 / 100 renders, and **the `<` of cell 1 is read as `<` in 4 / 100, the winner `S` in 96** (`S` 96).
   Over the degraded steps that `<` is read as itself 793 / 5,400 (14.7%, 13.8-15.7) with `S` the winner in 3,398 of 4,607 wrong renders
   (`S` 3398, `C` 288, `E` 257, `R` 124, `K` 119). It is the only single filler between two letters the tool has swept. Beside it, the first `<` of the
   `<<` separator in the name field reads as itself in 5 / 100 clean renders and 2,357 of 3,957 wrong renders are
   `S`: the same `S`, and at the clean render nearly the same rate, with a letter on the left in both. Over the degraded steps the two differ (the control's `<` is read as itself 14.7%, the separator's first `<` 26.7%). Observed. No other machine's numbers
   (#624, #660, #683, #685) are set beside these: the tool's module doc forbids comparing atlas runs across machines.

7. **What this does not show** is in the last section. Every derived statement above is labelled Observed or
   Hypothesized.

## Cell by cell: the clean render

Letter and filler truths are pooled within a cell; rates are `k / n (rate, Wilson 95%)`.

| Cell | Letter truths | Filler truths | Letter read as itself | Filler read as itself | Wrong winners of the filler |
|---|---:|---:|---|---|---|
| 5 | 100 | 0 | 100 / 100 (100.0%, 96.3-100.0) | - | - |
| 6 | 100 | 0 | 100 / 100 (100.0%, 96.3-100.0) | - | - |
| 7 | 100 | 0 | 100 / 100 (100.0%, 96.3-100.0) | - | - |
| 8 | 100 | 0 | 100 / 100 (100.0%, 96.3-100.0) | - | - |
| 9 | 100 | 0 | 100 / 100 (100.0%, 96.3-100.0) | - | - |
| 10 | 100 | 0 | 100 / 100 (100.0%, 96.3-100.0) | - | - |
| 11 | 80 | 20 | 80 / 80 (100.0%, 95.4-100.0) | 0 / 20 (0.0%, 0.0-16.1) | `S` 19, `?` 1 |
| 12 | 64 | 36 | 64 / 64 (100.0%, 94.3-100.0) | 7 / 36 (19.4%, 9.8-35.0) | `S` 29 |
| 13 | 74 | 26 | 74 / 74 (100.0%, 95.1-100.0) | 7 / 26 (26.9%, 13.7-46.1) | `S` 18, `?` 1 |
| 14 | 47 | 53 | 46 / 47 (97.9%, 88.9-99.6) | 10 / 53 (18.9%, 10.6-31.4) | `S` 42, `C` 1 |
| 15 | 46 | 54 | 46 / 46 (100.0%, 92.3-100.0) | 27 / 54 (50.0%, 37.1-62.9) | `S` 27 |
| 16 | 89 | 11 | 89 / 89 (100.0%, 95.9-100.0) | 9 / 11 (81.8%, 52.3-94.9) | `S` 2 |
| 17 | 96 | 4 | 95 / 96 (99.0%, 94.3-99.8) | 3 / 4 (75.0%, 30.1-95.4) | `?` 1 |
| 18 | 88 | 12 | 88 / 88 (100.0%, 95.8-100.0) | 11 / 12 (91.7%, 64.6-98.5) | `S` 1 |
| 19 | 71 | 29 | 71 / 71 (100.0%, 94.9-100.0) | 21 / 29 (72.4%, 54.3-85.3) | `?` 3, `E` 3, `S` 2 |
| 20 | 61 | 39 | 60 / 61 (98.4%, 91.3-99.7) | 31 / 39 (79.5%, 64.5-89.2) | `E` 6, `?` 1, `S` 1 |
| 21 | 33 | 67 | 32 / 33 (97.0%, 84.7-99.5) | 48 / 67 (71.6%, 59.9-81.0) | `?` 10, `E` 6, `S` 3 |
| 22 | 12 | 88 | 12 / 12 (100.0%, 75.8-100.0) | 70 / 88 (79.5%, 70.0-86.7) | `E` 10, `?` 4, `S` 3, `-` 1 |
| 23 | 6 | 94 | 6 / 6 (100.0%, 61.0-100.0) | 82 / 94 (87.2%, 79.0-92.5) | `E` 12 |
| 24 | 3 | 97 | 3 / 3 (100.0%, 43.9-100.0) | 95 / 97 (97.9%, 92.8-99.4) | `E` 2 |
| 25 | 2 | 98 | 2 / 2 (100.0%, 34.2-100.0) | 96 / 98 (98.0%, 92.9-99.4) | `-` 1, `E` 1 |
| 26 | 0 | 100 | - | 98 / 100 (98.0%, 93.0-99.4) | `E` 1, `S` 1 |
| 27 | 0 | 100 | - | 99 / 100 (99.0%, 94.6-99.8) | `-` 1 |
| 28 | 0 | 100 | - | 97 / 100 (97.0%, 91.5-99.0) | `s` 2, `S` 1 |
| 29 | 0 | 100 | - | 99 / 100 (99.0%, 94.6-99.8) | `S` 1 |
| 30 | 0 | 100 | - | 96 / 100 (96.0%, 90.2-98.4) | `s` 3, `S` 1 |
| 31 | 0 | 100 | - | 99 / 100 (99.0%, 94.6-99.8) | `S` 1 |
| 32 | 0 | 100 | - | 88 / 100 (88.0%, 80.2-93.0) | `S` 8, `s` 4 |
| 33 | 0 | 100 | - | 99 / 100 (99.0%, 94.6-99.8) | `S` 1 |
| 34 | 0 | 100 | - | 98 / 100 (98.0%, 93.0-99.4) | `S` 2 |
| 35 | 0 | 100 | - | 99 / 100 (99.0%, 94.6-99.8) | `S` 1 |
| 36 | 0 | 100 | - | 99 / 100 (99.0%, 94.6-99.8) | `S` 1 |
| 37 | 0 | 100 | - | 100 / 100 (100.0%, 96.3-100.0) | - |
| 38 | 0 | 100 | - | 100 / 100 (100.0%, 96.3-100.0) | - |
| 39 | 0 | 100 | - | 100 / 100 (100.0%, 96.3-100.0) | - |
| 40 | 0 | 100 | - | 100 / 100 (100.0%, 96.3-100.0) | - |
| 41 | 0 | 100 | - | 100 / 100 (100.0%, 96.3-100.0) | - |
| 42 | 0 | 100 | - | 100 / 100 (100.0%, 96.3-100.0) | - |
| 43 | 0 | 100 | - | 100 / 100 (100.0%, 96.3-100.0) | - |

## Over the degraded steps, cell by cell

The 54 degraded steps are the six axes' steps without their identity steps (the jpeg axis has none; its first step is
quality 100 and counts as degraded). Each cell's letters and fillers are rated apart.

| Cell | Letter renders | Letter wrong | Filler renders | Filler read as `<` | Filler read as a letter | Filler outside the MRZ set |
|---|---:|---:|---:|---:|---:|---:|
| 5 | 5,400 | 6.7% | 0 | n/a | n/a | n/a |
| 6 | 5,400 | 5.7% | 0 | n/a | n/a | n/a |
| 7 | 5,400 | 5.2% | 0 | n/a | n/a | n/a |
| 8 | 5,400 | 4.1% | 0 | n/a | n/a | n/a |
| 9 | 5,400 | 5.1% | 0 | n/a | n/a | n/a |
| 10 | 5,400 | 5.7% | 0 | n/a | n/a | n/a |
| 11 | 4,320 | 4.7% | 1,080 | 17.9% | 70.2% | 11.9% |
| 12 | 3,456 | 4.2% | 1,944 | 36.8% | 55.1% | 8.1% |
| 13 | 3,996 | 5.4% | 1,404 | 39.1% | 50.4% | 10.5% |
| 14 | 2,538 | 7.8% | 2,862 | 34.5% | 58.7% | 6.8% |
| 15 | 2,484 | 5.2% | 2,916 | 48.3% | 48.2% | 3.5% |
| 16 | 4,806 | 5.8% | 594 | 61.4% | 37.4% | 1.2% |
| 17 | 5,184 | 5.3% | 216 | 44.9% | 45.4% | 9.7% |
| 18 | 4,752 | 3.8% | 648 | 45.1% | 47.2% | 7.7% |
| 19 | 3,834 | 5.4% | 1,566 | 43.7% | 48.3% | 8.0% |
| 20 | 3,294 | 6.3% | 2,106 | 49.8% | 46.0% | 4.2% |
| 21 | 1,782 | 5.6% | 3,618 | 43.5% | 48.5% | 7.9% |
| 22 | 648 | 5.1% | 4,752 | 51.3% | 45.5% | 3.2% |
| 23 | 324 | 5.2% | 5,076 | 53.9% | 44.3% | 1.8% |
| 24 | 162 | 4.9% | 5,238 | 56.6% | 41.6% | 1.8% |
| 25 | 108 | 1.9% | 5,292 | 56.5% | 42.0% | 1.5% |
| 26 | 0 | n/a | 5,400 | 59.9% | 38.4% | 1.7% |
| 27 | 0 | n/a | 5,400 | 58.6% | 39.2% | 2.1% |
| 28 | 0 | n/a | 5,400 | 62.7% | 35.3% | 1.9% |
| 29 | 0 | n/a | 5,400 | 66.4% | 31.7% | 1.9% |
| 30 | 0 | n/a | 5,400 | 69.4% | 28.8% | 1.8% |
| 31 | 0 | n/a | 5,400 | 69.9% | 28.0% | 2.1% |
| 32 | 0 | n/a | 5,400 | 72.0% | 26.1% | 1.8% |
| 33 | 0 | n/a | 5,400 | 73.5% | 25.0% | 1.5% |
| 34 | 0 | n/a | 5,400 | 76.4% | 22.2% | 1.4% |
| 35 | 0 | n/a | 5,400 | 77.7% | 21.0% | 1.3% |
| 36 | 0 | n/a | 5,400 | 80.1% | 19.0% | 0.9% |
| 37 | 0 | n/a | 5,400 | 81.3% | 18.0% | 0.7% |
| 38 | 0 | n/a | 5,400 | 81.1% | 18.3% | 0.6% |
| 39 | 0 | n/a | 5,400 | 83.3% | 16.2% | 0.5% |
| 40 | 0 | n/a | 5,400 | 82.7% | 16.9% | 0.4% |
| 41 | 0 | n/a | 5,400 | 85.2% | 14.6% | 0.2% |
| 42 | 0 | n/a | 5,400 | 83.9% | 15.9% | 0.1% |
| 43 | 0 | n/a | 5,400 | 81.8% | 18.2% | 0.0% |

## Pooled per axis and step

All filler truths and all letter truths of the run, at every step. Units: resolution px per cell (22 is native), JPEG
quality, rotation degrees, blur sigma in native px, noise sigma in grey levels, contrast ink scale. "Outside" is every
filler winner that is not `<`, not a letter.

| Axis | Step | Filler renders | Read as `<` | Read as a letter | Outside the MRZ set | Letter renders | Letter wrong | Letter read as `<` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| resolution | 22 | 2,528 | 90.5% | 8.2% | 1.3% | 1,372 | 0.3% | 0 |
| resolution | 18 | 2,528 | 94.4% | 5.2% | 0.4% | 1,372 | 0.2% | 0 |
| resolution | 15 | 2,528 | 90.1% | 9.7% | 0.2% | 1,372 | 0.1% | 0 |
| resolution | 13 | 2,528 | 93.1% | 6.5% | 0.4% | 1,372 | 0.1% | 0 |
| resolution | 11 | 2,528 | 89.5% | 9.6% | 0.9% | 1,372 | 0.0% | 0 |
| resolution | 10 | 2,528 | 94.1% | 5.0% | 0.9% | 1,372 | 0.0% | 0 |
| resolution | 9 | 2,528 | 98.1% | 1.5% | 0.4% | 1,372 | 0.0% | 0 |
| resolution | 8 | 2,528 | 96.5% | 2.7% | 0.8% | 1,372 | 0.1% | 0 |
| resolution | 7 | 2,528 | 97.4% | 1.5% | 1.0% | 1,372 | 0.1% | 0 |
| resolution | 6 | 2,528 | 70.1% | 29.7% | 0.2% | 1,372 | 0.8% | 0 |
| resolution | 5 | 2,528 | 42.0% | 57.7% | 0.4% | 1,372 | 1.6% | 0 |
| resolution | 4 | 2,528 | 0.0% | 100.0% | 0.0% | 1,372 | 17.4% | 0 |
| jpeg | 100 | 2,528 | 91.1% | 7.7% | 1.1% | 1,372 | 0.3% | 0 |
| jpeg | 90 | 2,528 | 93.2% | 5.9% | 0.9% | 1,372 | 0.2% | 0 |
| jpeg | 75 | 2,528 | 92.2% | 6.2% | 1.6% | 1,372 | 0.1% | 0 |
| jpeg | 60 | 2,528 | 90.7% | 8.1% | 1.3% | 1,372 | 0.1% | 0 |
| jpeg | 50 | 2,528 | 94.9% | 4.7% | 0.4% | 1,372 | 0.0% | 0 |
| jpeg | 40 | 2,528 | 91.9% | 6.9% | 1.2% | 1,372 | 0.0% | 0 |
| jpeg | 30 | 2,528 | 91.9% | 6.8% | 1.3% | 1,372 | 0.1% | 0 |
| jpeg | 20 | 2,528 | 71.2% | 27.3% | 1.5% | 1,372 | 0.4% | 0 |
| jpeg | 15 | 2,528 | 91.9% | 7.1% | 1.0% | 1,372 | 0.1% | 0 |
| jpeg | 10 | 2,528 | 97.8% | 1.7% | 0.5% | 1,372 | 0.3% | 0 |
| jpeg | 5 | 2,528 | 90.4% | 8.0% | 1.6% | 1,372 | 0.9% | 0 |
| rotation | 0 | 2,528 | 90.5% | 8.2% | 1.3% | 1,372 | 0.3% | 0 |
| rotation | 0.25 | 2,528 | 89.0% | 11.0% | 0.0% | 1,372 | 0.1% | 0 |
| rotation | 0.5 | 2,528 | 90.2% | 9.6% | 0.2% | 1,372 | 0.1% | 0 |
| rotation | 1 | 2,528 | 71.7% | 28.2% | 0.1% | 1,372 | 0.0% | 0 |
| rotation | 1.5 | 2,528 | 50.9% | 48.8% | 0.3% | 1,372 | 0.1% | 0 |
| rotation | 2 | 2,528 | 15.3% | 84.0% | 0.8% | 1,372 | 1.3% | 0 |
| rotation | 3 | 2,528 | 14.3% | 85.4% | 0.3% | 1,372 | 16.2% | 0 |
| rotation | 4 | 2,528 | 5.5% | 93.3% | 1.2% | 1,372 | 32.1% | 0 |
| rotation | 5 | 2,528 | 9.1% | 90.5% | 0.4% | 1,372 | 49.7% | 0 |
| blur | 0 | 2,528 | 90.5% | 8.2% | 1.3% | 1,372 | 0.3% | 0 |
| blur | 0.5 | 2,528 | 83.7% | 14.9% | 1.3% | 1,372 | 0.1% | 0 |
| blur | 1 | 2,528 | 83.7% | 15.5% | 0.7% | 1,372 | 0.0% | 0 |
| blur | 1.5 | 2,528 | 93.0% | 5.7% | 1.2% | 1,372 | 0.0% | 0 |
| blur | 2 | 2,528 | 92.8% | 6.7% | 0.5% | 1,372 | 0.0% | 0 |
| blur | 2.5 | 2,528 | 65.7% | 34.3% | 0.0% | 1,372 | 0.4% | 0 |
| blur | 3 | 2,528 | 1.0% | 99.0% | 0.0% | 1,372 | 1.7% | 0 |
| blur | 4 | 2,528 | 0.0% | 99.9% | 0.1% | 1,372 | 16.7% | 0 |
| blur | 5 | 2,528 | 0.0% | 67.9% | 32.1% | 1,372 | 67.2% | 0 |
| blur | 6 | 2,528 | 0.0% | 63.3% | 36.7% | 1,372 | 73.3% | 0 |
| noise | 0 | 2,528 | 90.5% | 8.2% | 1.3% | 1,372 | 0.3% | 0 |
| noise | 2 | 2,528 | 88.6% | 9.9% | 1.5% | 1,372 | 0.1% | 0 |
| noise | 4 | 2,528 | 74.8% | 22.9% | 2.2% | 1,372 | 0.2% | 0 |
| noise | 8 | 2,528 | 60.5% | 38.4% | 1.0% | 1,372 | 0.1% | 0 |
| noise | 12 | 2,528 | 59.1% | 40.0% | 0.8% | 1,372 | 0.3% | 0 |
| noise | 16 | 2,528 | 58.8% | 40.7% | 0.5% | 1,372 | 0.1% | 0 |
| noise | 24 | 2,528 | 52.5% | 46.4% | 1.1% | 1,372 | 0.5% | 0 |
| noise | 32 | 2,528 | 43.0% | 55.9% | 1.1% | 1,372 | 0.7% | 0 |
| contrast | 1 | 2,528 | 90.5% | 8.2% | 1.3% | 1,372 | 0.3% | 0 |
| contrast | 0.8 | 2,528 | 86.7% | 11.8% | 1.5% | 1,372 | 0.4% | 0 |
| contrast | 0.6 | 2,528 | 80.3% | 18.2% | 1.5% | 1,372 | 0.5% | 0 |
| contrast | 0.5 | 2,528 | 72.5% | 25.8% | 1.8% | 1,372 | 0.4% | 0 |
| contrast | 0.4 | 2,528 | 64.0% | 34.2% | 1.9% | 1,372 | 0.6% | 0 |
| contrast | 0.3 | 2,528 | 62.5% | 35.8% | 1.7% | 1,372 | 0.5% | 0 |
| contrast | 0.2 | 2,528 | 62.9% | 35.6% | 1.5% | 1,372 | 0.5% | 0 |
| contrast | 0.15 | 2,528 | 68.0% | 31.0% | 1.0% | 1,372 | 0.5% | 0 |
| contrast | 0.1 | 2,528 | 65.5% | 33.8% | 0.6% | 1,372 | 0.3% | 0 |

## Floors per cell and axis

A floor is the most severe step still read as itself in at least 90 of the cell's renders of that class, going down from
the clean render without a failing step (`all steps` = none failed); `none` = the clean step already fails. Only cells
with 20 or more renders of the class are listed.

**Letter truths:**

| Cell | n | resolution | jpeg | rotation | blur | noise | contrast |
|---|---:|---|---|---|---|---|---|
| 5 | 100 | to 5 (fails at 4) | all steps | to 2 (fails at 3) | to 3 (fails at 4) | all steps | all steps |
| 6 | 100 | to 5 (fails at 4) | all steps | to 2 (fails at 3) | to 3 (fails at 4) | all steps | all steps |
| 7 | 100 | to 5 (fails at 4) | all steps | to 2 (fails at 3) | to 4 (fails at 5) | all steps | all steps |
| 8 | 100 | all steps | all steps | to 2 (fails at 3) | to 4 (fails at 5) | all steps | all steps |
| 9 | 100 | to 5 (fails at 4) | all steps | to 2 (fails at 3) | to 4 (fails at 5) | all steps | all steps |
| 10 | 100 | to 5 (fails at 4) | all steps | to 2 (fails at 3) | to 3 (fails at 4) | all steps | all steps |
| 11 | 80 | to 5 (fails at 4) | all steps | to 3 (fails at 4) | to 4 (fails at 5) | all steps | all steps |
| 12 | 64 | all steps | all steps | to 2 (fails at 3) | to 4 (fails at 5) | all steps | all steps |
| 13 | 74 | to 5 (fails at 4) | all steps | to 3 (fails at 4) | to 3 (fails at 4) | all steps | all steps |
| 14 | 47 | to 5 (fails at 4) | all steps | to 2 (fails at 3) | to 2 (fails at 2.5) | all steps | all steps |
| 15 | 46 | to 5 (fails at 4) | all steps | to 3 (fails at 4) | to 3 (fails at 4) | all steps | all steps |
| 16 | 89 | to 5 (fails at 4) | all steps | to 2 (fails at 3) | to 3 (fails at 4) | all steps | all steps |
| 17 | 96 | to 5 (fails at 4) | all steps | to 2 (fails at 3) | to 3 (fails at 4) | all steps | all steps |
| 18 | 88 | to 5 (fails at 4) | all steps | to 3 (fails at 4) | to 3 (fails at 4) | all steps | all steps |
| 19 | 71 | to 5 (fails at 4) | all steps | to 3 (fails at 4) | to 3 (fails at 4) | all steps | all steps |
| 20 | 61 | to 5 (fails at 4) | all steps | to 3 (fails at 4) | to 3 (fails at 4) | all steps | all steps |
| 21 | 33 | to 5 (fails at 4) | all steps | to 3 (fails at 4) | to 3 (fails at 4) | all steps | all steps |

**Filler truths:**

| Cell | n | resolution | jpeg | rotation | blur | noise | contrast |
|---|---:|---|---|---|---|---|---|
| 11 | 20 | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| 12 | 36 | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| 13 | 26 | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| 14 | 53 | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| 15 | 54 | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| 19 | 29 | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| 20 | 39 | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| 21 | 67 | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| 22 | 88 | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| 23 | 94 | none (fails at 22) | to 90 (fails at 75) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| 24 | 97 | to 18 (fails at 15) | to 75 (fails at 60) | to 1.5 (fails at 2) | to 0 (fails at 0.5) | to 0 (fails at 2) | to 1 (fails at 0.8) |
| 25 | 98 | to 7 (fails at 6) | to 50 (fails at 40) | to 1.5 (fails at 2) | to 0 (fails at 0.5) | to 2 (fails at 4) | to 1 (fails at 0.8) |
| 26 | 100 | to 7 (fails at 6) | to 30 (fails at 20) | to 1 (fails at 1.5) | to 0 (fails at 0.5) | to 2 (fails at 4) | to 1 (fails at 0.8) |
| 27 | 100 | to 7 (fails at 6) | to 50 (fails at 40) | to 1 (fails at 1.5) | to 0.5 (fails at 1) | to 2 (fails at 4) | to 1 (fails at 0.8) |
| 28 | 100 | to 7 (fails at 6) | to 30 (fails at 20) | to 1 (fails at 1.5) | to 0 (fails at 0.5) | to 2 (fails at 4) | to 1 (fails at 0.8) |
| 29 | 100 | to 7 (fails at 6) | to 30 (fails at 20) | to 1 (fails at 1.5) | to 0.5 (fails at 1) | to 2 (fails at 4) | to 0.8 (fails at 0.6) |
| 30 | 100 | to 6 (fails at 5) | all steps | to 1 (fails at 1.5) | to 0 (fails at 0.5) | to 2 (fails at 4) | to 1 (fails at 0.8) |
| 31 | 100 | to 6 (fails at 5) | to 90 (fails at 75) | to 1 (fails at 1.5) | to 2 (fails at 2.5) | to 2 (fails at 4) | to 0.8 (fails at 0.6) |
| 32 | 100 | none (fails at 22) | none (fails at 100) | none (fails at 0) | none (fails at 0) | none (fails at 0) | none (fails at 1) |
| 33 | 100 | to 7 (fails at 6) | all steps | to 0.5 (fails at 1) | to 2 (fails at 2.5) | to 4 (fails at 8) | to 0.5 (fails at 0.4) |
| 34 | 100 | to 7 (fails at 6) | all steps | to 1 (fails at 1.5) | to 0 (fails at 0.5) | to 8 (fails at 12) | all steps |
| 35 | 100 | to 7 (fails at 6) | to 30 (fails at 20) | to 1 (fails at 1.5) | to 2 (fails at 2.5) | to 16 (fails at 24) | all steps |
| 36 | 100 | to 5 (fails at 4) | all steps | to 0.5 (fails at 1) | to 2.5 (fails at 3) | to 16 (fails at 24) | all steps |
| 37 | 100 | to 5 (fails at 4) | all steps | to 0.5 (fails at 1) | to 2.5 (fails at 3) | to 16 (fails at 24) | all steps |
| 38 | 100 | to 6 (fails at 5) | all steps | to 0.5 (fails at 1) | to 2.5 (fails at 3) | to 16 (fails at 24) | all steps |
| 39 | 100 | to 6 (fails at 5) | all steps | to 1.5 (fails at 2) | to 2.5 (fails at 3) | to 16 (fails at 24) | all steps |
| 40 | 100 | to 5 (fails at 4) | all steps | to 1.5 (fails at 2) | to 2.5 (fails at 3) | to 24 (fails at 32) | all steps |
| 41 | 100 | to 5 (fails at 4) | all steps | to 1.5 (fails at 2) | to 2.5 (fails at 3) | to 24 (fails at 32) | all steps |
| 42 | 100 | to 5 (fails at 4) | all steps | to 1.5 (fails at 2) | to 2.5 (fails at 3) | to 24 (fails at 32) | all steps |
| 43 | 100 | to 6 (fails at 5) | all steps | to 1.5 (fails at 2) | to 2 (fails at 2.5) | to 24 (fails at 32) | all steps |

## Wrong winners on degraded steps

The class split over the degraded steps, pooled, is in answer 5. By cell, the share of letter and filler renders read as
a letter or outside the MRZ set is in the table "Over the degraded steps, cell by cell" above.

## What this does not show

- Nothing about real documents or the shipped pipeline; the atlas reads an oracle crop of a synthetic line with a
  vendored font. Nothing about real names, chargrid, or the dropped filler-or-letter classifier of #650; no code, ADR,
  baseline or default changes.
- **No single `<` between letters, and no name with more than one given name or surname**: the generator draws one of
  each, so the position #575 saw most (a single filler inside a name) is absent. The positions measured here are the
  `<<` separator and the padding tail, which are the generator's, and the tail's position relative to a name's end.
- The separator's two cells and the tail's head are 100 renders each, one per seed; their rates have wide intervals
  (the intervals are in the answer). The cells of the first tables mix positions: the same cell is a separator `<` in
  one seed and a tail `<` in another, so a per-cell rate in cells 11-25 is a blend; the position table is the clean view.
- The pooled rates rest on 100 renders per cell and step; a single glyph's rate is indicative at best. The floors pool a
  cell's letters or fillers, so a glyph that always fails can sit under a passing cell rate.
- The `E` winner on degraded fillers is a count, not an explanation (see answer 2).
- One run, one machine, one thread setting; its spread is not measured. Axes are applied one at a time, never composed.
