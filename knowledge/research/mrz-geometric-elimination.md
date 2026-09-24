# Reading the MRZ by elimination: geometric constraints on a closed alphabet

**Date:** 2026-09-18 · **MAIN:** `daab97e` · **DATA:** `5988c7b` · **Evidence:** font geometry **Observed**, recognition impact **Hypothesized** · **Status:** design note, no measurement yet

Staged for `knowledge/research/`. Nothing here has been measured against a document. The font
measurements are exact and reproducible; every claim about what they would do to accuracy is a
hypothesis, and the note is written so that a later measurement can falsify it cleanly.

## Why this note exists

[ADR-0013](../decisions/ADR-0013-names-are-scored-against-mrz-form-truth.md) established that a Tier-1
hit proves the document number and the dates and proves nothing about the name, because no ICAO 9303
check digit covers one. [ADR-0014](../decisions/ADR-0014-per-cell-ocrb-classification.md) proposed
`mrz-cell` in response, and justified rendered-template matching partly on the grounds that `ocrs`
exposes no per-character score and that its recogniser "has no representation to emit" an isolated `<`.

Work on 2026-09-17/18 changed the cost of testing both premises, and produced a third option that
neither document anticipated. This note records it before it is built, so that the eventual amendment
argues from something written down rather than from memory.

## The reframe

The working assumption in every approach so far — `ocrs`, the fixed-grid repair, and the template
classifier ADR-0014 proposes — has been **recognition**: look at a cell and decide what it is.

The alternative is **elimination**: look at a cell and rule out what it cannot be. On an MRZ band this
is not a rhetorical difference. The alphabet is closed at 37 glyphs, the typeface is specified, the
layout is a fixed grid, most positions are constrained by the standard before any pixel is read, and
four fields plus a composite carry check digits. A recogniser that is mostly right is a liability on a
product sold on determinism. An eliminator that provably rules out thirty-six candidates leaves one
answer with a citable reason, and where it leaves two it has *localised* the ambiguity rather than
guessed at it — which is the honest outcome the refusal-class metric already rewards.

## Four signals, and what each one is blind to

Measured from the vendored OCR-B at `crates/synthpass-gen/fonts/ocr-b.ttf` (1024 units per em)
directly from the glyph outlines. No rendering, no model, no OCR. First measured with `fontTools`;
[`tools/ocrb_metrics.py`](../../tools/ocrb_metrics.py) now reproduces every figure below with a
standard-library reader cross-checked against `fontTools`, and adds per-glyph ink runs, the
confusable pairs ranked by what separates them, and the chargrid ink-floor comparison.

| Signal | Source | Eliminates | Blind to |
| --- | --- | --- | --- |
| Position class | the standard | digits where letters are required, and the reverse | anything within a class |
| Top edge (`yMax`) | geometry | digit vs letter | `<` (sits inside the letter band) |
| Ink mass and per-column profile | geometry | filler vs letter; `<` vs `K` | same-shape pairs |
| Check digit | arithmetic | readings that cannot balance | same-residue swaps, and `<`/`K` **provably** |

**Digits and capitals do not overlap in height.** Letters occupy `yMax` 876–912, digits 962–982 — a
gap of roughly 50 units on a 1024 em, with no overlap at all. The consequence is that a cell's top edge
classifies digit-versus-letter without a model, and the standard already says which positions must be
which.

**"Top edge" must be defined as a threshold, not an extreme.** `yMax` is an outline extreme, and round
tops (`0`, `8`, `O`, `Q`) erode under blur and binarisation faster than flat ones (`5`, `7`, `T`). A
measurement that takes the first inked row therefore mixes shape with height — and it biases exactly
the round-topped digits *toward* the letter band, which is the `0`/`O` pair, the one that matters most.
Define the top edge as the row where cumulative ink from the top of the cell crosses a fixed fraction
of that cell's total, and report the fraction alongside the separation. The 50-unit outline gap is an
upper bound on what survives rendering; measurement item 1 below exists to find what actually does.

That one measurement covers most of `mrz::CONFUSABLES`. Differences in top edge: `0`/`O` 70, `0`/`D`
105, `0`/`Q` 84, `8`/`B` 106, `2`/`Z` 96, `1`/`I` 81, `5`/`S` 58. This corrects an assumption made
earlier the same evening, that `0` misread as `O` or `D` had to be left to the check digit because the
two glyphs carry near-identical ink mass. Their masses are indeed close; their *extents* are not.

**The filler is the exception, and not by height.** `<` measures an ink height of 881 with a `yMax` of
890 — squarely inside the letter band. What separates it from a letter is mass: two thin diagonals
against a dense glyph.

**`<` versus `K` is the case where every other signal fails.** Their bounding boxes differ by 4 units of
height and 5 of top edge. The check digit cannot see the difference either, and the repository already
knew this — `crates/mrz/src/parser.rs`, in `repair_td3_line2`:

> `K` ≡ `<` (both value 20 ≡ 0 mod 10) under every 7-3-1 weight, so check digits are provably blind to
> this misread — heuristics must do it.

The per-column ink profile is the only discriminator left: `K` carries a full-height vertical stem at
its left edge where `<` carries only a vertex. This matters out of proportion to its subtlety, because
fillers misread as letters are the largest single error class the corpus has recorded.

**`M` versus `N` is the genuinely hard pair** — identical height, identical box, 38 units of width
between them. It crosses residue classes, so the check digit can reject the wrong reading, which is
why `CONFUSABLES` carries it as a measured entry.

## The format supplies a template, not just a constraint

`repair_td3_line2` already holds a per-position class table: `(9..10 digitize)`, `(10..13 letterize)`,
`(13..20 digitize)`, `(21..28 digitize)`, `(42..44 digitize)`. Position 20 — the sex field — is the gap
between the two digitize ranges, a letter island in a run of digits.

Read as heights, TD3 and TD2 line 2 columns 10–27 are therefore fixed in advance:

```
S S S   T T T T T T T   S   T T T T T T T
 nat        DOB + cd   sex     DOE + cd
```

**A blind spot in the motif:** ICAO 9303 permits `<` in unknown date digits, so a cell the template
marks `T` may legitimately hold a filler and measure as `S`. The template must therefore be scored as a
*correlation* rather than an exact match, and a document with a largely unknown date of birth has a
correspondingly weaker anchor. (TD3 column 9, the document-number check digit, is always a digit, so
the template could be extended to nineteen cells with a leading `T`. Not adopted here — eighteen is the
contiguous run, and a longer template buys little once scoring is a correlation.)

An eighteen-cell binary pattern, known before a pixel is read. Correlating measured top edges against
it does three things at once: it locks the grid offset (a grid off by one column scores badly), it
discriminates the format (TD1 carries the same motif at a different offset with nationality on the
other side), and it detects a 180° rotation. Two cells are enough for the weakest version of the last
one — line 2 always ends in the composite check digit, which is a digit; line 1 almost always ends in
filler.

Two consequences worth stating plainly. First, ADR-0014's proposed `position_class(format, line, col)`
is a **formalisation of data already in the crate**, not new analysis — which is a day saved and, more
importantly, a guarantee that the repair path and the classifier cannot drift apart. Second, this gives
the name field its **first verification signal**. Geometry cannot tell `E` from `F`, but a name
position can never legitimately hold a digit-height glyph, and a filler run can be confirmed by mass.
ADR-0013's premise — that names are unverifiable — is true of check digits and not quite true of the
document.

## Addendum 2026-09-23: what the standard says, and where the filler sits across fonts

**ECMA-11 (3rd edition, March 1976) figures.** §4.1 gives the size I letterpress heights: digits
2.60 mm, capitals 2.46 mm, small letters 1.83 mm, descender 0.60 mm. That makes digits 5.7%
taller than capitals, which is the height gap §"Four signals" relies on. §3.6 says the tallest
character is digit `8` and §3.7 that the widest is digit `0`. §3.8 sets the minimum size I pitch
at 2.54 mm, and §11.4-11.5 the nominal strokewidth at 0.35 mm (0.31 mm for small letters and
`#`, `%`, `@`). ECMA-11 gives **no numeric position for `<`**. The index table (ref. 79, page 19)
refers to ECMA-30, which has no geometry either. Drawing 79 is not among the reproduced reference
drawings (`1`, `E`, `§`, `¥`), so the only in-document evidence is the §13 4:1 illustration.

*Amended 2026-09-24:* the 5.7% gap above is ECMA-11's **letterpress** table. ICAO requires the constant-strokewidth style, whose ISO 1073-2 Table 1 digit height is 2.66 mm, **8.1%** above capitals; see [`../ocrb/glyph-dimensions.md`](../ocrb/glyph-dimensions.md).

**Cross-font table** (cap units: baseline 0, cap line 1, cap = median flat capital; the
Barcodesoft files are local, commercial and not vendored):

| Source | Digit / cap | `<` bottom | `<` top | `<` height | `<` centre |
| --- | ---: | ---: | ---: | ---: | ---: |
| ECMA-11 §4.1 (text) | 1.057 | — | — | — | — |
| ECMA-11 §13 illustration, 4:1 (measured) | 1.063 | +0.050 | 1.046 | 0.996 | 0.548 |
| Vendored `ocr-b.ttf` (Raisty 2019, OFL) | 1.087 (`1`) | +0.010 | 1.006 | 0.996 | 0.508 |
| Barcodesoft `ocrbI.ttf`, `ocrbIV.ttf` | 1.077 (`1`) | +0.074 | 0.995 | 0.921 | 0.535 |
| Barcodesoft `OCRB.TTF` | 1.079 (`1`) | +0.162 | 1.037 | 0.875 | 0.600 |
| 16 real TD3 specimens, median (measured) | — | +0.061 | 0.997 | 0.924 | 0.528 |

The vendored filler matches ECMA-11's height but sits about 0.04 cap lower than the illustration
draws it, and about 0.03 cap lower than 14 of 16 real specimens print it (≈0.6 px at a 20 px cap).
Real print spans at least two filler families; United Arab Emirates 2011 prints a short raised
filler of the `OCRB.TTF` kind. The per-specimen table, the positive control that calibrates the
instrument, and the ISO 1831 tolerances that bound all of this are in
[`ocrb-filler-geometry-2026-09-23.md`](../benchmarks/ocrb-filler-geometry-2026-09-23.md).

One consequence for this note: a threshold that separates `<` from a letter by vertical position
must be measured on real bands. A synthetic `<` rendered from the vendored font sits lower than
most printed ones.

## What this does not touch

**Band detection is upstream, and it is not solved by any of this.** Every signal above assumes a band
that has been found, oriented and cropped. The three detection misses frozen in
[ADR-0011](../decisions/ADR-0011-split-m6-packaging-into-m8.md)'s 2026-09-17 amendment — France ID 2020
back, Italy CIE 2022 back, Moldova `PA_MDA_2014` — fail before any of it applies, and would fail
identically with a perfect classifier. Improving recognition cannot close a detection miss. The
separate locator idea recorded in the current plan (an ICAO geometry prior plus a matched filter for
the periodic `<<<<` run in the corner strips) addresses that half, and the filler-run periodicity it
looks for is the same signature this note relies on for grid validation — the two share a mechanism
and should be built to share code.

## Risks, stated before they are discovered

1. **The digit/letter gap is 5% of character height.** On a real crop with 30 px characters that is
   about 1.5 px. It is measurable, marginal at low resolution, and comfortable only as resolution
   rises. Thresholds must therefore be normalised **within the band**, against that line's own measured
   baseline and cap line, never as absolute pixels. The resolution floor is a number to be measured,
   and once measured it is a specification worth publishing.
2. **Every figure here comes from one font file, which has a defect in its advance table — and the
   defect does not reach the rendered glyph.** In the vendored `ocr-b.ttf` the digit `5` carries an
   advance of 782 where every other MRZ glyph is 884 or 886, 11.7% narrow. OCR-B is monospaced by
   design (ISO 1073-2), so this is almost certainly a digitisation fault rather than the typeface. But
   the fault is in `hmtx` only: `5`'s outline runs xMin 120 to xMax 662 and is centred within its own
   782 advance to **0.0 units**, and across all 37 MRZ glyphs the largest ink-versus-advance offset is
   `G` at 20.5 units (about 2%). Because `draw_mrz_glyphs`
   (`crates/synthpass-gen/src/render.rs`) centres each glyph by its advance within a fixed cell, every
   glyph's ink lands dead-centre in its cell whatever its advance. A rendered `5` is therefore neither
   inset nor narrow — its ink width of 542 is ordinary beside `S` at 555 and `2` at 580. The defect
   bites only under **flow layout**, where it shifts everything after a `5` by 104 units, which is
   precisely the drift `draw_mrz_glyphs` was written to avoid. Cross-check the advance table against
   another OCR-B source; templates are unaffected.
   *(Corrected 2026-09-18 after an independent re-measurement. The original text claimed a rendered
   `5` template would be systematically narrow and that a synthetic `5` sits further from its cell
   edges than a printed one. Neither follows from the outlines.)*
3. **Perspective breaks the fixed-pitch assumption, and nothing else here does.** The generator's
   `Rotate` is in-plane only; a photograph taken at an angle makes pitch vary across the line. Every
   other degradation makes the signals noisier, while keystone makes the grid *wrong* — and a
   confidently misfitted grid is worse than a refusal.
4. **Synthetic degradation is a model of degradation.** `crates/synthpass-gen/src/degrade.rs` says so
   itself: it does not model any real device's noise transfer function, lens distortion or paper-ink
   interaction. Sweeping it finds where a threshold breaks relative to a controlled axis. Only real
   specimens say whether the threshold works, and a synthetic envelope must never be quoted as a real
   one.
5. **A raw model score is uncalibrated.** If the CTC matrix is read directly (see below), the
   per-character probability it yields is better than the current plausibility proxy but is not a
   calibrated confidence. `project_principles.md` §2 permits an uncalibrated score for *routing* and
   forbids publishing one without a reliability diagram. Obtaining a model score is therefore necessary
   but not sufficient to retire the High-severity debt entry, and neither that entry nor ADR-0014 §5
   currently says so.

## Relationship to the recogniser question

Separately on 2026-09-17 it was established that `ocrs::OcrEngine::prepare_recognition_input` is public
and returns the exact preprocessed line tensor the recognition model consumes, so the CTC
log-probability matrix can be obtained by running the already-pinned `.rten` model through `rten`
directly — no fork, no vendored patch, no new dependency. That path is recorded in
[`technical_debt.md`](../technical_debt.md) as the prescribed fix and is cheaper than the version
written there.

The geometric signals in this note are **complementary to that, not an alternative**. The recognition
model is a CRNN whose two bidirectional GRU layers carry context across the whole line, which is the
most plausible explanation for its refusal to emit an isolated `<`: fillers occur in runs in training
data and essentially never singly. Ink mass and glyph extent are measured from pixels and cannot be
influenced by that prior. They are the evidence the model's context cannot reach — which is precisely
what is needed at the positions where the model is known to be wrong.

A rendered-template classifier (ADR-0014 as written), a masked read-out of the model's own matrix, and
these geometric eliminations all consume the same segmentation from `synthpass_ocr::chargrid`. They are
arms of one bake-off, not competing designs, and a disagreement between them localises a failure to
segmentation rather than classification — which ADR-0014 already names as the useful outcome of a
no-go.

## What would have to be measured

Before any of this earns a decision record:

1. Digit/letter separation as a **distribution over real fixtures at their real resolutions**, not as a
   font constant — yielding the resolution floor.
2. Ink-profile separation of `<` from `K` on real specimens, against the fixture truth.
3. Grid-offset recovery by correlation against the eighteen-cell height template, scored on how sharply
   it peaks at the correct offset.
4. An operating envelope: sweep one degradation axis at a time and report where each signal fails —
   character height in pixels, ink-spread radius, keystone angle. The envelope is the deliverable.
5. All of it conditioned on the grid-fit self-check passing, since a misfitted grid invalidates every
   measurement above equally.

A no-go keeps the numbers. Measuring that geometry cannot carry this would be a real result: it would
say the residue is in segmentation or in genuinely ambiguous ink, and it would say so for the cost of a
probe rather than the cost of a prototype.

## Sequencing

After the `chargrid` A/B has a number, so that ADR-0014's "+10 pp over the best arm" threshold has a
value, and after M6's Definition of Done is evaluated against its frozen list — `ROADMAP.md`'s M6 rule
and ADR-0011's amendment bar an OCR engine replacement, a vision provider or a new dependency from
landing under the milestone. Nothing in this note is any of those; all of it is post-M6 regardless,
because a recognition change is judged against the frozen snapshot.
