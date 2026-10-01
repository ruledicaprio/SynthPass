# Chargrid placement: the filler lands one cell right because the next letter's `ocrs` anchor sits on the filler, not because of the grid

**Date:** 2026-10-01 · **MAIN:** `048695e` (the run's commit; its run manifest says `working_tree_dirty: false`) · **DATA:** the public corpus, `samples-data` `396b22f` (the baseline's pinned `samples_data_sha`), 261 documents, `samples/corpus.jsonl` sha256 `25b31530…` · **Evidence:** Observed (one local release `provider-bench --real-specimens --mrz-only --dump-ocr-passes` run with `SYNTHPASS_OCR_CHARGRID=on`, and its chargrid capture) plus Observed by eye (the 42 repaired name lines, hand-marked on their line crops) plus Derived (the fit replayed from the capture, one component varied per trial) plus Hypothesized (the readings marked so) · **Status:** current

**2026-10-01.** This is the dated note that
[#575](https://github.com/ruledicaprio/SynthPass/issues/575) section E asks for. Some name fillers
that chargrid inserts land one cell right of where they are printed; the issue named four candidate
causes and proved none. The capture is the one
[#598](https://github.com/ruledicaprio/SynthPass/pull/598) added: per repaired line, the glyph
edges, the line box, the fitted origin and pitch, the assigned cells and the column ink. Real
specimens are named by country, year and side, and cells by index (0-based along the name line).
No OCR zone text appears here. Chargrid stays off by default, and no code, ADR, baseline or default
changes with this note.

## The answer

- **The cause is a per-glyph anchor error, not a grid error** (Observed, Derived). In every
  right-shifted line, `ocrs` dropped the printed filler, and the letter after it came back with a
  left edge on the filler's cell: 0.75 to 1.06 pitch left of its own printed cell, and 1.83 pitch
  for a dropped pair. The DP places that letter where its anchor is, on the filler's cell, and the
  empty cell it leaves behind is the one after. All 55 glyphs that follow a dropped printed filler
  fall into two groups. 8 are displaced by −0.75 to −1.83 pitch, and those are exactly the 8
  right-shifted lines. The other 47 sit within −0.11 to +0.26, and every one of those lines is
  placed correctly. For comparison, the 893 other name-region glyphs (each line's first glyph excluded) sit within
  −0.18 to +0.19 (p5 to p95).
- **None of the four candidate components moves a right-shifted line to its printed cell, in any
  trial** (Derived). The ink origin (`grid_origin_from_ink`), a held or ink-measured pitch, the
  packing penalty at 0 or 0.2, and 0 or 1 refit rounds each fix 0 of 8. The packing penalty cannot
  matter in principle: either placement has exactly one skipped cell around the letter, so both pay
  `PACK` once.
- **Moving the anchor itself, to the glyph's right edge or its centre, fixes 4 of 8 and moves 31
  of 33 good lines** (Derived). In `ocrs` 0.13.1 a glyph's right edge is the next glyph's left edge
  (`text_lines_from_recognition_results`). So at a dropped filler, the glyph before the gap
  inherits the same ambiguity, and the error moves to the other side of the gap.
- **The counts on `048695e`:** 42 repaired lines. 8 place an interior filler one cell right of print
  (never left), 28 place every interior filler on its printed cell, 5 insert fillers only inside
  printed filler runs, and 1 (China 2012) misplaces a filler by 28 cells for a different reason
  (below). The issue's figures on `86987fe` were 39 repaired, 7 right-shifted and about 20 correct.
  This run did not re-run that commit, so the difference is not attributed.

## The right-shifted lines

"Anchor offset" is the displaced letter's left edge minus its **printed** cell's left edge, in
pitch units, on the captured grid. The trial codes are defined under Method. A1, P1, P2, K0, K2, R0
and R1 fix none of these lines, so they are not repeated per row.

| Document | Format, name line | Printed filler cell(s) | Inserted cell(s) | Anchor offset (pitch) | Trial that fixes it |
| --- | --- | --- | --- | ---: | --- |
| Portugal 2024, ID card, back | TD1, line 3 | 19 | 20 | −1.02 | none |
| Romania 2021, ID card, back | TD1, line 3 | 6, 7 | 7, 8 | −1.83 | none (A3 fails the ink gate) |
| Canada 2023, passport data page | TD3, line 1 | 12 | 13 | −0.88 | A2, A3 |
| Denmark 2007, passport data page | TD3, line 1 | 23 | 24 | −0.81 | none |
| Egypt 2022, passport data page | TD3, line 1 | 19 | 20 | −0.93 | A3 |
| Finland 2023, passport data page | TD3, line 1 | 25 | 26 | −1.06 | A2 |
| Singapore 2017, passport data page | TD3, line 1 | 15 | 16 | −0.75 | A2, A3 |
| United Kingdom 2015, passport data page | TD3, line 1 | 7 | 8 | −0.83 | A2, A3 |

Seven of the eight are a single printed filler between two name words. Romania is a printed pair,
of which `ocrs` read neither. The next letter's anchor sits at the start of the pair, and both
inserted fillers land one cell right. The displacement is relative to the letter's own predecessor
too: its anchor-to-anchor step is 0.76 to 1.14 pitch short (1.94 for Romania). So the grid is not
drifting under it.

**The other misplacement, China 2012 (passport data page, the `.png`; TD3, line 1).** A printed
filler at cell 15 was dropped, and chargrid inserted the missing filler at cell 43, so the eight
letters at printed cells 16 to 23 are assigned one cell left. This is not the anchor mechanism, since
those letters' anchors sit within 0.2 pitch of their printed cells. The fitted pitch is 16.17 px
against an ink period of 16.0 px (1.0% long, about 0.45 pitch over the line). The filler tail also
carries two glyph pairs 3 px and 5 px apart (0.19 and 0.31 pitch) that both read as `<`, which keeps
the glyph count at 43 of 44 and gives the DP no reason to open the gap at 15. Trial P2 (pitch held
at the ink period) is the only trial that puts the gap at cell 15. All other trials leave it at 43
or 42, or fail the ink gate (A1).

## Trials, one component at a time

Each trial changes one component of `fit_grid` and `align` and replays all 42 repaired lines from
the capture. "Fixed" means that every name-region letter lands on its printed cell and that the
repair still passes the ink gate and the prefix gate. "Moved away" counts the 28 correct and 5
tail-only lines on which any name-region letter leaves its printed cell. Gate rejections are
counted over all 42 lines, in their own column. China 2012 is scored separately, above.

| Trial | Component | What changed | Right-shifted fixed, of 8 | Good lines moved away, of 33 | Gate rejections |
| --- | --- | --- | ---: | ---: | --- |
| T0 | — | none: the production fit, replayed | 0 | 0 | none; replay equals the capture on all 43 lines, cells and grid |
| A1 | anchor | the final origin replaced by `grid_origin_from_ink` at the fitted pitch, over the line box ± one pitch; one `align` | 0 | 3 | 3 × `prefix_changed` |
| A2 | anchor | each glyph's right edge minus the seed pitch, in place of its left edge | 4 | 31 | 2 × `prefix_changed` |
| A3 | anchor | each glyph's centre minus half the seed pitch | 4 | 31 | 1 × `ink_mismatch`, 2 × `prefix_changed` |
| P1 | pitch | pitch held at the seed (line width / cells); the refit moves the origin only | 0 | 0 | none |
| P2 | pitch | pitch held at the ink period of the line box's column ink; the refit moves the origin only | 0 | 4 | 4 × `ink_mismatch` |
| K0 | packing penalty | `PACK = 0` | 0 | 0 | none |
| K2 | packing penalty | `PACK = 0.2` | 0 | 0 | none |
| R0 | refit feedback | 0 refit rounds (the seed grid's `align` is final) | 0 | 0 | none |
| R1 | refit feedback | 1 refit round instead of 3 | 0 | 0 | none |

- **A1 moves the origin by a median of −0.29 pitch** (range −1.45 to −0.09), so it reaches the
  grid's phase. It cannot reach a single letter whose anchor is one pitch off its neighbours, and it
  breaks three lines' protected prefix.
- **P1, K0, K2, R0 and R1 change no assignment on any of the 42 lines.** The fitted pitch is within
  a median 0.46% of the ink period, and within 2.5% on the 8 right-shifted lines (at most 0.55 pitch
  of drift over half a line). That is too little to move a letter a whole cell where its anchor is
  right.

## Method

- **The run.** It used a release `provider-bench` built from `048695e` with the pinned models
  (detection sha256 `f15cfb56bd02…`, recognition `e484866d4cce…`), and
  `SYNTHPASS_OCR_CHARGRID=on`, every other `SYNTHPASS_OCR_*` arm at its default, and
  `SYNTHPASS_OCR_DUMP_VARIANTS` pointed at a gitignored directory to keep each pass's image. It ran
  on one Linux container with 4 cores (Intel Xeon @ 2.10 GHz), from 2026-09-30 23:48 to
  2026-10-01 00:14 UTC (1,591 s), with exit 0.

  ```
  SYNTHPASS_OCR_CHARGRID=on SYNTHPASS_OCR_DUMP_VARIANTS=<ignored dir> \
    provider-bench --real-specimens --mrz-only --dump-ocr-passes --progress --out <ignored dir>/report.json
  ```

  It scored 137 / 149 Tier-1 hits with no budget stops. Chargrid verdicts: `skipped:no_valid_mrz`
  121, `repaired` 42, `rejected:no_deficit` 26, `skipped:no_line_match` 19, `unchanged` 12,
  `rejected:ink_mismatch` 11, `skipped:line_match_ambiguous` 10, `rejected:prefix_changed` 7,
  `skipped:grid_fit_failed` 7, `skipped:no_source_image` 6. 134 documents carry a capture, and 105
  of those have a matched line.
- **The replay.** A throwaway Python port of `fit_grid`, `align`, `cell_ink` and
  `grid_origin_from_ink` (f32 where the Rust is f32), not committed, reproduces every captured grid
  and cell assignment exactly. On the 103 captures that were not downscaled, the column ink recomputed
  from the dumped pass image (the `image` crate's integer luma and the `< 110` ink rule) equals the
  captured `column_ink`. That pins each crop to the pixels chargrid measured.
- **Hand-marking.** Each repaired line was rendered from its pass image with the fitted grid, the
  cell indices, every glyph's left edge and the inserted cells drawn on it. Every interior insertion,
  and every name letter whose anchor sat more than half a pitch from its assigned cell, was read by
  eye against the printed characters. Two lines (Portugal 2024 and the Netherlands 2014) were
  downscaled by the width cap. Their crops were re-downscaled with a different Lanczos implementation
  for viewing only, and the replay used the captured values.
- **"Name region"** is every cell before the first run of three fillers in the repaired line. Garbage
  glyphs that `ocrs` read inside a printed filler run (letters on fillers, on 6 of the lines) are not
  name letters, and their cells are not scored.
- **The ink period (P2)** is the autocorrelation peak of the mean-removed column ink over the line
  box, searched in 0.1 px steps within ±20% of the fitted pitch. It is this note's instrument and is
  not in the codebase.

## Readings (Hypothesized)

- **Why the anchor lands on the filler.** `ocrs` reports a character's left edge as the CTC
  timestep where its label is emitted, scaled back to the image. A filler that decodes as blank
  leaves timesteps with no label. One reading is that the recognizer emits the next letter's label
  as early as the filler's first timestep on some lines and at the letter's own ink on others. This
  run shows where the edge lands, not why the model emits there.
- **The 47 undisplaced followers say the displacement is not systematic in the geometry.** They
  include single fillers, pairs, every format, and pitches from about 11 to 54 px. What separates the
  8 from the 47 is not visible in the capture, which records edges and ink, not CTC scores.

## What this does not show

- **Nothing about names scored against truth.** 5 of the 42 repaired lines have name truth, and
  none of the 8 right-shifted lines does. "Printed cell" is read off the print, not off a scored
  field.
- **No fix.** No trial is a candidate change. A2 and A3 show the anchor is the component, and also
  that a box edge alone cannot carry the fix. Nothing here measures an ink-based per-glyph anchor,
  and no default, constant or gate changes.
- **Nothing about `86987fe`.** The issue's 39 / 7 / about 20 were not re-run here. The repaired set
  and the verdict counts differ from #598's R5 run (40 repaired) in ways this note does not explain.
- **Nothing about the rejected lines.** The 11 `ink_mismatch` and 7 `prefix_changed` lines were not
  hand-marked.
- **Nothing synthetic.** The 2026-09-24
  [reconciliation](chargrid-ab-reconciliation-2026-09-24.md) found that synthetic and real results
  did not transfer; this run is real only.
- **One run, one machine.** OCR is floating point; the capture is deterministic for this build,
  these models and this container, and was not repeated.

## Rejected on the way

- **"A whole-line offset."** 28 lines place every interior filler correctly in the same run, and
  the displacement is per glyph.
- **The four candidates as named in #575.** Origin from ink, pitch, `PACK` and refit each fix 0 of 8.
  Only the pitch trial changes the separate China 2012 misplacement.
