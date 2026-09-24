# ADR-0021 — Fixed-grid MRZ strips: align the read to its format's template, anchored by check digits, and refuse when the anchors cannot decide

**Status:** Proposed
**Date:** 2026-09-24

> **What this ADR asks for first is a measurement, not code.** Its first stage (Phase 0 below) is
> run against the current tree and can end the proposal on its own numbers. Nothing enters the
> default path before the go/no-go is met, and a no-go sets this ADR to Rejected with the numbers
> kept.

## Context

### The idea, stated plainly

An MRZ is printed as two or three lines, but the format fixes every cell's position. A TD3 zone
is 88 cells: line 1 is cells 0–43, and cell 44 always starts line 2. TD2 and MRV-B are 72 cells
(36 + 36), MRV-A is 88 (44 + 44), and TD1 is 90 (30 + 30 + 30). The line breaks are **positions
the format fixes**, not boundaries a reader has to detect. Read that way, a zone is one
fixed-pitch strip of cells, and every format's template is a sequence of typed cells:

- **Check-digit cells** and the fields they cover.
- **Structurally constrained cells**: the document code, the issuing state from a closed registry,
  the sex vocabulary, calendar dates, the filler rules.
- **Unverifiable cells**, whose content any reading may fill: names and optional data.

Cells 0–4 mean the same thing in all five formats: two cells of document code, then three of
issuing state. So the formats share the strip's start and diverge only after it.

The metaphor comes from ECMA-21, *Character Positioning on OCR Journal Tape* (1969,
[`knowledge/ecma/`](../ecma/ECMA-21_Character_Positioning_on_OCR_Journal_Tape_1969.md)). A
journal tape is one continuous medium, read as a sequence of lines at a controlled spacing. To
be exact about the source: **ECMA-21 defines line boundaries, line spacing, margins and tape
dimensions. It does not define a cell index, a fixed characters-per-line count or a strip.** The
mapping from cell index to (line, column) comes from ICAO Doc 9303 Parts 4–7. ECMA-21 supplies
the reading model, and Doc 9303 supplies the template.

Two consequences follow, and they are what this ADR is about:

1. **Check digits can act as alignment anchors, not just as validators.** Given a noisy OCR
   stream, the question "which cells did the reader drop, add or split?" is an alignment of the
   stream against the template. An alignment that puts every check-digit cell where its field's
   arithmetic agrees is evidence for that alignment. How strong that evidence is gets bounded
   below; it is weaker than intuition suggests.
2. **The MRZ's own guarantees can seed detection.**
   - The first cell is always one of `P`, `V`, `A`, `C` or `I`.
   - The second is `<` or an issuer-chosen character within the limits of each Part's
     document-code note.
   - The pitch is fixed, and so is the cell count per line.
   - Filler runs are the zone's most repeated glyph.

### Where the parser does alignment today, one case at a time

`crates/mrz/src/parser.rs`'s `find_and_parse_with` scans free text and tries hand-written
structural candidates in a fixed order:

- **`unshift_line1_prefix`** undoes a dropped position-1 filler.
- **`shift_line1_right_at_country`** undoes an inserted character before the issuing state.
- **`shift_or_unshift_line1`** composes the two, because "each is a no-op passthrough of the
  other's trigger case".
- **`unshift_if_country_resolves`** gates the unshift on the country registry.
- **`repair_td1_line1_unshifted`** handles TD1.
- **The TD1 line-gap tolerance** (`take(3)` windows) and the **merged-token fast paths** split an
  84–92-character token at exactly 44.
- **`crates/mrz/src/checksum.rs`'s `fit_length`** is documented as "Candidate alignments of a
  wrong-length line". It inflates or deflates the longest filler run, or pads or trims the ends.

Every one of these is an alignment decision, made by a rule chosen for one observed shape. Two
properties of the current design matter here, and the code documents both itself:

- **Candidate order can decide the answer.** `consider()` returns the first reading whose
  check digits all validate. On TD3, TD2 and MRV, line 1 carries no check digit. So two readings
  of line 1 that differ only in alignment are equally "valid", and the first one tried wins.
  `unshift_line1_prefix`'s doc comment records this. When the transform was first applied
  unconditionally, it measured a Tier-1 regression on a synthetic TD3 corpus. The shipped version
  is "not a full fix … some corrupted readings still win over the unshifted candidate when both
  parse structurally, since nothing on line 1 discriminates between them".
- **Each new shape is a new rule.** Three open issues are alignment problems of the same kind:
  - [#429](https://github.com/ruledicaprio/SynthPass/issues/429): a right-shifted TD3 line 1 whose
    document code is `PP` is never repaired, because `shift_line1_right_at_country` acts only
    when position 1 is `<`.
  - [#409](https://github.com/ruledicaprio/SynthPass/issues/409): `find_and_parse` re-flows an
    intact Türkiye 2020 TD1 zone into TD2.
  - [#408](https://github.com/ruledicaprio/SynthPass/issues/408): one OCR space splits line 2
    into two tokens, and neither is usable.

The damaged-capture pass already contains the right safety rule, in one place.
`damaged_pass` and `class_sweep_pass` collect every reading that passes and then call `single()`.
That returns a reading only when all the readings agree on the extracted fields, and otherwise
refuses: "Several genuinely different readings means the MRZ cannot distinguish them, and the
honest answer is the ordinary checksum-failed fallback." The main scan has no such rule.

### Why position 1 of TD3 line 1 matters more every year

Doc 9303 Part 4 §4.4, verified against the PDF and recorded in
[`CONFORMANCE_BASIS.md`](../docs9303/CONFORMANCE_BASIS.md) ("Part 4 §4.4 — harmonized TD3
document codes"):

- From 1 January 2026, a passport issued with a secondary document code must use the table
  (`PP`, `PE`, `PD`, `PO`, `PR`, `PT`, `PS`, `PL`, `PM`).
- From 1 January 2028, every passport must carry one.
- Every passport without one expires before 1 January 2038.

So position 1 is a real letter on a growing share of documents, and any repair that assumes `<`
there degrades over time. The same entry notes that codes outside the table are not rejected:
until 2038 a valid passport may carry a pre-harmonization secondary code. So until then **the
table is a prior, not a constraint**.

What OCR does at that position is only partly known. `CONFORMANCE_BASIS.md` records that, in the
45 documents whose raw OCR line 1 survives in older dump logs, position 1 was never read as `K`.
[#420](https://github.com/ruledicaprio/SynthPass/issues/420) records that the question could be
answered for 89 of 261 documents, and that `<` was read as S, R, C or E. Raw OCR is not
persisted per run today; #420 proposes that it should be.

### What the measurements say — and the premise they correct

**Detection is no longer the bottleneck on the scored population.** When
[ADR-0008](ADR-0008-mrz-detection-track.md) was written, `no_mrz_found` outnumbered
`checksum_failed`. Its 2026-09-12 amendment records the premise "has run out" at 7 : 7. The
committed baseline
([`real-specimen-mrz-baseline.json`](../benchmarks/real-specimen-mrz-baseline.json), CI,
2026-09-19) records **`no_mrz_found` 2 and `checksum_failed` 10**.
[`twelve-scored-misses-2026-09-19.md`](../benchmarks/twelve-scored-misses-2026-09-19.md)
attributes the twelve by mechanism: **3 detection or segmentation, 9 recognition**. The three are
France and Italy, plus Russia, which is a TD3 read as 3 × 30. Current headline figures live only in
[`benchmarks/README.md`](../benchmarks/README.md).

**The scored checksum failures are mostly not shifts.** In the same record, **5 of the 9**
recognition misses carry one or two isolated wrong cells on their check-digit line. The Czechia
attribution in
[`checksum-failed-miss-mechanisms-2026-09-18.md`](../benchmarks/checksum-failed-miss-mechanisms-2026-09-18.md)
states "There is no shift". So an aligner is **not** expected to move the Tier-1 count much on
the current corpus. This ADR does not claim it will.

**But much of the observed mismatch is structural, not character-level.**
[`observed-ocrb-confusions-2026-09-21.md`](../benchmarks/observed-ocrb-confusions-2026-09-21.md)
aligned the 31 dumped misses against reviewed fixtures, 19 of them `checksum_failed_specimen`.
Three results matter here:

- **Wrong-line pairs:** 10 of 57 line pairs were a re-read of a *different* printed line.
- **Shifted lines:** 12 lines were ones where "an indel alignment beats positional".
- **Reclassified mass:** 86.5% of the naive per-position mismatch mass was reclassified from
  "confusion" to "unalignable".

That population is misses only, and mostly specimens outside the Tier-1 denominator. What is
**unmeasured** is how often a *hit* carries a misaligned line 1: a correct document number and
dates, but a shifted document code, issuing state or name split. That is where an aligner could
improve correctness without moving the Tier-1 count, and it is the first thing Phase 0 measures.

**Filler ink is distinguishable, but not by ink mass alone.**
[`ocrb-filler-geometry-2026-09-23.md`](../benchmarks/ocrb-filler-geometry-2026-09-23.md) §5,
measured on 16 TD3 specimens, found:

- A filler cell carries **0.67× a letter's cell ink** (range 0.60–0.72): "a real contrast, but
  not a gap".
- A column-run shape heuristic misclassified 3 of 28 letters on one specimen and was rejected.
- What separated fillers reliably was **shape self-similarity**: the medoid of the largest
  cluster of near-identical glyphs, "because the trailing run guarantees a dozen or more copies
  of one glyph".
- Character pitch is fixed to within a 3.2% grid residual.
- Line spacing is **not** one number: 1.65–2.99 cap on TD3, against a TD1 nominal of 1.72.

The maintainer's working phrase "differentiable filler ink" is used here in the sense of
*distinguishable*. Nothing in this ADR is trained or differentiated.

### What a check digit can and cannot anchor

Where the anchors are, per format (0-based cells in the strip):

| Format | Strip | Check-digit cells | Line with no check digit |
| --- | --- | --- | --- |
| TD3 | 44 + 44 | 53, 63, 71, 86 (fields), 87 (composite) | line 1 (cells 0–43) |
| TD2 | 36 + 36 | 45, 55, 63 (fields), 71 (composite) | line 1 (cells 0–35) |
| MRV-A | 44 + 44 | 53, 63, 71; no composite | line 1 |
| MRV-B | 36 + 36 | 45, 55, 63; no composite | line 1 |
| TD1 | 30 + 30 + 30 | 14; 36, 44 (fields); 59 (composite, covering line 1 cells 5–29 and line 2) | line 3 (cells 60–89) |

- **On TD3, TD2 and MRV, line 1 is anchored only structurally.** Those anchors are cell 0's
  format letter, cells 2–4 against the country registry, the `<<` name separator, and the filler
  tail. On TD3, 48 of 88 cells carry no check digit at all
  ([#421](https://github.com/ruledicaprio/SynthPass/issues/421)).
- **TD1's composite is the one genuine cross-line anchor.** `parse_td1` already verifies it
  after lines are chosen. What the line-by-line scan cannot do is use it to *choose* an alignment
  that crosses the line break. One example is a cell dropped at the end of line 1 when the zone
  arrives as one merged token.
- **A check digit is a weaker alignment anchor than a substitution anchor, and this can be
  proved.** [`blindspot.rs`](../../crates/mrz/src/blindspot.rs) states the law for one
  substitution: a check digit is blind to it exactly when the two values are congruent mod 10.
  The analogous law for a **one-cell shift** follows from the 7-3-1 weights:
  - Moving a character one cell changes its weight by −4, −2 or +6.
  - All three are even, so a shift can only move the sum to one of five residues, not ten.
  - The check digit is unchanged exactly when Σ vᵢ·Δwᵢ ≡ 0 (mod 5).
  - So **a block of characters whose values are all ≡ 0 (mod 5) — `0 5 A F K P U Z <` — can be
    shifted one cell without changing any check digit it lies under.**
  - More generally, under a uniform random model a single check digit catches a one-cell filler
    relocation inside its field with probability about **4/5, not 9/10**.

  This is arithmetic, not a corpus measurement. It is to be pinned by a property test next to
  `blindspot.rs`'s, and it is the reason the decision below insists on several anchors and a
  unanimity rule.

## The question

Should the MRZ scan stop enumerating hand-written structural candidates in a fixed order? The
alternative is to align each candidate window of OCR text against every format's template in one
deterministic search that:

- scores every alignment by the anchors it satisfies;
- accepts only a reading the anchors single out;
- reports **ambiguous** when they do not.

And should detection be seeded by the same template guarantees?

## Decision (proposed)

Three layers, each with its own crate boundary, arm and go/no-go. Layers 1 and 2 are this ADR's
substance. Layer 3 is stated here only to define its interface to Layer 2. Its method belongs to
[ADR-0015](ADR-0015-geometric-mrz-band-location.md), and this ADR recommends amending that one
rather than duplicating it.

### Layer 1 — the strip template (in `mrz`, crate-private at first)

One table per `Format` gives, for every cell of the strip:

- its line and column;
- its **role**: which field's check digit covers it, whether the composite also covers it, which
  structural rule applies, or that it is unverifiable;
- its **allowed class**: digit, letter, letter-or-filler, the closed MRZ alphabet, or the fixed
  format letter.

- **One table, three consumers.** The same data is what
  [#421](https://github.com/ruledicaprio/SynthPass/issues/421) asks for as a coverage map, and
  what [ADR-0014](ADR-0014-per-cell-ocrb-classification.md) asks for as `position_class`. Building
  it once means those two cannot drift apart. It is also derivable from the `CdFields` tables
  `parser.rs` already carries for the class sweep, which this table should replace rather than
  sit beside.
- **Tests are the contract.**
  - The table is total: every cell of every format has exactly one role.
  - It is honest: mutate each cell, and the parser reacts as the role says.
  - Its ranges are derived from the Doc 9303 PDFs, per the docs9303 audit, not from memory.
- **Crate-private** until #421 decides whether it becomes public API. No behaviour change in this
  layer.

### Layer 2 — check-anchor alignment with an explicit ambiguity outcome (in `mrz`)

**Input.** The same candidate windows `find_and_parse_with` already forms: one line, or up to
three consecutive candidate lines within today's `take(3)` gap tolerance. Whitespace inside a
window is dropped. The OCR's own line breaks become **soft evidence** (a cost), not hard
boundaries. This addresses #408's split token and merged zones with a dropped cell.

**Search.** For each format whose cell-0 letter is admissible, align the window's character stream
to the template. Allowed edits are insert a cell, delete a cell, and treat an OCR line break as a
position. They are bounded by **k edits per line** (k swept in Phase 0, not chosen here). Edits
are admissible only where something can arbitrate them:

- **(a) inside a region a check digit covers**, where the arithmetic arbitrates;
- **(b) inside a structurally anchored region**: cells 0–4, the sex cell, the date fields, the
  check-digit cells themselves;
- **(c) nowhere else, from text alone.**

In unverifiable regions — names, optional data without their own digit — the aligner does **not**
invent a shift. Those cells are taken as the existing per-line repairs produce them today,
`fit_length` included. Shifting a name from text alone is a guess, and the arithmetic cannot
refute it. Geometric evidence can (Layer 3's cell positions). That is the aligner's second version
and is out of scope here.

The search is exact and small. Its state is (cell, offset, running weighted residue mod 10 for the
open field, and for the composite where one exists): about 10⁴–10⁵ states per format and window.
It is not a heuristic beam, has no randomness, no floating point in any decision, and needs no
dependency. The implementation must stay within `mrz`'s MSRV (1.82), zero dependencies by
default, and `wasm32`-clean, because `mrz-wasm` ships it to the browser demo.

**The aligner decides *where* cells are; the existing repairs decide *what* they are.** Layer 2
does no substitution of its own. Each aligned candidate still passes through today's per-line
repairs (`repair_positions`' digitize and letterize, `fix_doc_code`, defillering) and, behind its
own flag, the class sweep. Keeping indels and substitutions in separate stages keeps the
candidate count bounded, and keeps each stage's measured record meaningful.

**Outcome.** Readings are compared as **distinct decoded strips**, not distinct paths: two edit
paths that produce the same strip are one reading. The filler-relocation law above makes many
paths collapse this way.

| Result within bound k | What `find_and_parse_with` returns |
| --- | --- |
| Exactly one distinct reading verifies every applicable check digit and structural anchor | That reading, as today |
| More than one does, and they disagree on any **checked** field (document number, dates, personal number or optional data under a digit) or on format | `MrzError::AmbiguousAlignment { format, readings }` — a new variant on the already `#[non_exhaustive]` enum. No reading is accepted. |
| They agree on every checked field, and disagree only on the line-1 structural prefix (document code, issuing state) | The checked fields are accepted. Line 1 is returned as the existing repairs produce it, not as either alignment chose, so `synthpass_core::fusion::check_line1_integrity`'s existing findings see what they see today. |
| None verifies | Today's fallback ranking (`fallback_rank`), unchanged |

A margin rule of the form "accept the fewest-edits reading if the next one costs Δ more" is a
**prior about how OCR fails, not arithmetic**. It is not adopted. If Phase 0 shows unanimity
refuses too much, it comes back as its own measured amendment, with its constant's sweep.

**Arms.** `ParseOptions::with_strip_align(bool)`, off by default, is the same pattern as
`with_class_sweep`: additive under ARCHITECTURE §13.4. `synthpass-die` gets
`SYNTHPASS_MRZ_STRIP_ALIGN=off|on|control`, mirroring `class_sweep_arm`. In `control` the
aligner runs and its result is discarded, so an A/B separates the treatment from its cost.

**Pipeline and routing.** An `AmbiguousAlignment` is a Tier-1 non-accept, and routes exactly as a
checksum failure does today. The competing readings are not handed to Tier 2 as a menu in this
version; that is an open question below. They carry holder data, so they stay inside the zeroized
document JSON and never reach logs or metric labels (principle 7's boundary).

**Benchmark taxonomy.** A new scored miss bucket, `ambiguous_alignment`, sits inside the existing
scored denominator. Refusing to choose between readings of a document that does carry an MRZ is
a miss, not an exclusion, so neither denominator moves. The first run that can produce the bucket
re-blesses the baseline in the same PR, per the maintenance contract in
[`benchmarks/README.md`](../benchmarks/README.md).

### Layer 3 — detection from the template's guarantees (interface here; method in ADR-0015)

The order is **coarse → anchor → fine**:

1. **Coarse candidate bands.** ADR-0015's recognition-free search over `ocrs`'s text-probability
   map.
2. **Anchor.** Confirm the candidate's top-left cell is a letter in {`P`, `V`, `A`, `C`, `I`}, and
   that a long run of the band's most repeated glyph (the filler) exists.
3. **Fine grid fit.** Pitch, baseline and skew, by robust fits: the Theil-Sen approach the
   2026-09-23 instrument used, with pitch bounded at 0.93–1.07 cap per
   [`knowledge/ocrb/line-and-pitch.md`](../ocrb/line-and-pitch.md).
4. **Crop, upscale and re-read the band.** Band crops already exist as retry variants
   (`plain_band`, `geometry_band_variants`). What is new is that the crop comes from geometry,
   not from recognized text.
5. **Hand Layer 2 positioned cells:** each glyph with a cell index, or with nothing, rather than a
   bare string.

Filler/character classification per cell is by **shape self-similarity within the band**, with
ink mass as a secondary feature. The measured filler/letter ink contrast (0.6–0.72×) is too weak
to classify on alone. Template correlation against the vendored OCR-B `<` is a second, independent
feature. Both are ADR-0014's territory. This two-class slice is the cheapest falsifiable piece of
ADR-0014's Option B.

Crate boundaries:

- **`synthpass-imageprep`** takes the geometry: band candidates, grid fit, per-cell features. It
  has no OCR types, keeps its one dependency, and must keep compiling for
  `wasm32-unknown-unknown`.
- **`synthpass-ocr`** obtains the probability map and the re-read.
- **`mrz`** receives only characters with optional cell indices: plain integers, no image type.
- **`synthpass-die` and `synthpass-core`** name no engine.

Two measured corrections bind the method, whichever ADR carries it
([`ocrb-filler-geometry-2026-09-23.md`](../benchmarks/ocrb-filler-geometry-2026-09-23.md) §5,
[#411](https://github.com/ruledicaprio/SynthPass/issues/411)):

- Fillers sit at **cell centres**, not on cell boundaries.
- Line spacing is a **band of values** per format family, not one prior. TD1 is much tighter than
  TD3.

## What an alignment may and may not claim

This is ADR-0017's and principle 1's discipline, carried over from values to positions.

**It may claim:** under this format's template, within k admissible edits per line, exactly one
distinct reading of this window satisfies every applicable check digit and structural anchor —
and here it is.

**It may not claim:**

- **That the reading is the printed zone.** Agreement with the printed check digits is
  consistency, as it is for substitutions.
- **Anything about line 1 of TD3, TD2 or MRV beyond structural consistency.** No check digit
  reaches it.
- **Anything about cells in unverifiable regions.** Layer 2 does not move them.
- **Correctness outside the bound.** An alignment needing more than k edits is not in the
  candidate set. A unique verifying reading inside the bound can still be wrong if the true
  reading lies outside it.
- **That uniqueness means correctness when the truth is not a candidate.** If a character in the
  true zone was also misread in a way no repair models, the true reading fails its anchors, and
  a *wrong* alignment can be the only one that passes. That is why Phase 0 measures the
  wrong-but-unique rate on perturbed ground truth, not only on clean ground truth.
- **That a refusal means the document is unreadable.** `AmbiguousAlignment` means only "these
  anchors cannot choose".

## What it subsumes, and what it does not replace

**Subsumes, once the go/no-go is met, each retired in its own PR:**

| Existing mechanism | Why the aligner covers it |
| --- | --- |
| `unshift_line1_prefix`, `repair_td1_line1_unshifted`, `repair_*_line1_unshifted` | A deletion at cell 1, arbitrated by the country registry (structural) or, on TD1, by the document-number digit and the composite |
| `shift_line1_right_at_country`, `shift_or_unshift_line1`, `unshift_if_country_resolves` | An insertion or deletion in cells 1–4, arbitrated by the registry in both directions. This covers the `PP` case in #429 without special-casing position 1's content |
| `fit_length`'s filler-run inflation and deflation on check-digit lines | Edits inside a covered field, arbitrated by its digit |
| The merged-token fast paths' fixed split at 44, 36 or 30 | The line break is a position, so a merged token with a dropped cell realigns across the break |
| #408's whitespace-split line 2 | Whitespace is not a cell |
| #409's TD1 re-flowed into TD2 | Format is part of the alignment. Reading an intact 90-cell zone as 72 costs 18 deletions, and the ambiguity rule refuses rather than guesses if both verify. *Unmeasured* whether this is #409's mechanism; Phase 0 replays it. |

Retirement conditions:

- Every pinned test of the retired function passes unchanged under the aligner:
  `line1_prefix_shift.rs`, `line1_right_shift.rs`, `td1_line_gap.rs`, `td2_line1_repair.rs`,
  `format_gate.rs`, `sequence_completeness.rs`.
- The replay and the real gate show zero per-document difference.
- A test is never deleted to make a retirement pass.

**Does not replace:**

- **Character repair.** `repair.rs`'s single-substitution search (`MAX_SUBSTITUTIONS = 1`), the
  class sweep, `CONFUSABLES`, and the digitize/letterize position repairs stay as they are. The
  aligner feeds them.
- **The damaged-capture pass.** It restores one destroyed glyph of unknown identity, with its
  calendar filter and `single()` gate. That is an insertion of an *unknown* character, which
  Layer 2 does not model. It may be folded in later if Phase 0 shows overlap.
- **`looks_like_non_mrz_text`** and the not-found gate.
- **Name repair.** No check digit covers a name. `chargrid`'s geometric filler recovery and
  ADR-0014 remain the only routes to a correct name. Layer 2 can only stop a *structural* shift
  from corrupting the fields before the name.
- **ADR-0017's `Checks`.** The aligner reports through them unchanged.
- **Anything for third-party OCR text beyond what `find_and_parse` already accepts.** The
  text-only aligner still matters to that audience, because `mrz` is published standalone and fed
  arbitrary OCR text by callers outside this pipeline.

## Relationship to other decisions

- **[ADR-0014](ADR-0014-per-cell-ocrb-classification.md) — extended, not superseded.** Layer 1 is
  its `position_class`, built once. Layer 3's two-class filler/character classifier is the
  cheapest measurable slice of its Option B. If ADR-0014 later classifies every cell in place on a
  fitted grid, indels mostly vanish at the source, and Layer 2 becomes the path for text-only
  input.
- **[ADR-0015](ADR-0015-geometric-mrz-band-location.md) — extended by amendment.** Layer 3's
  anchor confirmation and its cell-positioned output belong in ADR-0015, together with #411's
  corrections to its spacing prior and filler-position wording. This ADR adds no second detection
  track.
- **[ADR-0017](ADR-0017-checks-distinguish-absent-from-verified.md),
  [ADR-0013](ADR-0013-names-are-scored-against-mrz-form-truth.md),
  [ADR-0018](ADR-0018-optional-data-named-for-what-it-holds.md) to
  [ADR-0020](ADR-0020-mrz-value-wire-contract.md) — unchanged.**
- **[ADR-0008](ADR-0008-mrz-detection-track.md)** — this ADR relies on its 2026-09-12 amendment
  and does not reopen detection as M6's track.
- **ARCHITECTURE §13.3's fixed format order** stays in v1: the aligner runs inside each format's
  existing loop. Replacing the order with joint scoring across formats is an open question, and
  would be an amendment to §13.3.

## Measurement plan — before anything is built

### Phase 0: split the misses and the hits by mechanism (no product code)

- **P0.1 — one dump of everything.**
  - Run `provider-bench --real-specimens --mrz-only --dump-ocr --dump-ocr-hits`: release build,
    all `SYNTHPASS_OCR_*` and `SYNTHPASS_MRZ_*` arms cleared, no concurrent build.
  - Output goes to the gitignored `artifacts/`. Expected runtime is that of a local real-specimen
    arm.
  - This captures raw OCR for **hits too**, which no committed record has.
- **P0.2 — classify every MRZ-bearing document with a reviewed fixture**, hits and
  `checksum_failed_specimen` included. Use the three-gate alignment rule
  `observed-ocrb-confusions-2026-09-21.md` established. Each document falls into one of:
  - **(A) band visible, not found or not parsed** — detection;
  - **(B) found and read, aligned per cell, wrong characters** — recognition;
  - **(C) found and read, mis-aligned** — an indel alignment beats positional, or a line is
    assigned to the wrong printed line.

  Split (C) by line and by Layer 1 role (checked, structural, unverifiable). For hits, record
  whether (C) touches `document_type`, `issuing_country` or the name split.
- **P0.3 — the wrong-but-verifying atlas, on ground truth.** For every reviewed fixture's truth
  zone, and for every synthetic truth zone at a fixed seed, enumerate every alignment within k
  admissible edits per line (k = 1, 2, 3):
  - on the truth as printed;
  - on the truth with one confusable substitution, per `CONFUSABLES`.

  Count distinct readings that satisfy every anchor. This is pure arithmetic: no OCR, seconds per
  corpus. It measures, per format and per k:
  - how often unanimity would refuse a perfect read;
  - how often a wrong reading is the *unique* verifier.

  It also pins the one-cell shift law in a property test.
- **P0.4 — which existing path produced each hit.** Do this in the replay harness (an example
  binary, like `crates/synthpass-bench/examples/vocab_replay.rs`), not in product code. It sizes
  what retiring the old repairs would touch.

**Phase 0 output** is a dated note in `knowledge/benchmarks/`, describing documents by field and
position only, never by content.

### Kill criteria at the end of Phase 0 (proposed; the thresholds are the maintainer's call)

Stop, record the numbers and set this ADR to Rejected if **any** of these holds:

- **K1.** Across hits and misses, fewer than three documents are in class (C) at a position an
  admissible edit could reach, with a fixture-verified better field value. Nothing to fix.
- **K2.** At every k, unanimity refuses a perfect synthetic or fixture read more often than the
  aligner would recover a document. It costs more than it earns.
- **K3.** At the smallest k that reaches the (C) population, a wrong reading is the *unique*
  verifier on any fixture truth with at most one substitution. The anchors cannot carry the
  claim.

If (C) exists but never crosses a line break, choose the **line-based** variant (Alternatives,
C). It is the same aligner with the strip's cross-line edges removed, and it is cheaper.

### A/B design, once built

1. **Replay (seconds, deterministic).** Feed the P0.1 dump through one binary's
   `find_and_parse_with` with the option off, on and control.
   - The accept rule follows `vocab_replay.rs`: at least one miss→hit or one fixture-verified
     field correction on a hit, and **zero** hit→miss.
   - The one exception is a hit the fixture shows was already wrong, named individually.
   - A replay cannot see the retry loop. That loop stops on the first variant that validates, so
     a parser change can change which OCR text the parser is shown. The replay is how we learn
     whether the gate is worth running; it does not replace the gate.
2. **Synthetic, fixed seed, three arms, same binary.** Full ground truth, so it measures
   field-level correctness, names and issuer included, plus the wrong-but-verifying count.
   Synthetic results have not predicted real results for a repair class before (the chargrid
   A/B), so **this stage can veto but cannot justify**.
3. **The real-specimen gate on all 261 documents, three arms via
   `SYNTHPASS_MRZ_STRIP_ALIGN`.** Same release binary, same machine, no concurrent build.
   - Report **both denominators**: scored and corpus-wide.
   - Per-document crosstab keyed by `asset_id`, not by name. The class-sweep A/B found two
     name-sharing pairs that name-keying silently merges.
   - Strict names. Every `ambiguous_alignment` listed by document, with the fields its readings
     disagree on.

### Go / no-go for default-on (all must hold)

1. **No Tier-1 loss.** Zero hit→miss on the real gate, apart from individually named hits the
   fixtures show were wrong before.
2. **A real gain.** At least one miss→hit, or at least one fixture-verified field correction on a
   hit (`document_type`, `issuing_country`, or the name split). A null result is a no-go, as the
   class sweep's was (see
   [`mrz-class-sweep-ab-2026-09-20.md`](../benchmarks/mrz-class-sweep-ab-2026-09-20.md)).
3. **No invented reads.** `false_positive_mrz` and `document_number_mismatch` stay 0.
4. **Synthetic does not regress.** No loss in hits, and no rise in wrong-but-verifying readings.
5. **Latency.** p50 added ≤ 20 ms per document on the recorded machine. This budget is generous
   against OCR seconds, and a tighter one needs the control arm's number first.
6. **Determinism.** Two runs of the same commit give byte-identical reports.

A no-go sets this ADR to Rejected, keeps the numbers, and deletes the option in one PR.

## Alternatives considered

- **A. Keep the rule-per-shape repairs, and fix #429, #409 and #408 one at a time.** This is the
  cheapest route per issue, each fix measured on its own. It is not rejected outright: it is the
  fallback if Phase 0 kills this ADR. It loses on maintainability and correctness. Each new rule
  is order-dependent under first-valid-wins, and pairs of rules are no-ops of each other's
  trigger. Both are recorded in the functions' own doc comments. The cost grows with every issuer
  that adopts `PP`.
- **B. A trained detector or sequence model with a format grammar (CTC and the like).** Rejected:
  [`VISION.md`](../VISION.md)'s permanent non-goals rule out training OCR models. It would replace
  an inspectable rule with a weight file, and could not live in the zero-dependency `mrz` crate
  that ships to the browser.
- **C. Line-based alignment only: each line to its own template.** It fits the current structure
  and captures nearly all the anchors on TD3, TD2 and MRV, whose check digits sit on line 2 only.
  It loses TD1's composite as a joint anchor across lines 1 and 2, and it loses realignment across
  a mis-split merged token. **Kept as the fallback shape.** If Phase 0 finds no class (C) crossing
  a line break, it is the right choice.
- **D. Probability-weighted scoring:** sum log-likelihoods of OCR confusions and take the maximum.
  Rejected. `ocrs` exposes no per-character score (ADR-0014, Context), and principle 2 forbids a
  calibrated-looking number without a reliability diagram. Counting anchors is ordinal and honest;
  a likelihood would launder a guess.
- **E. Plain edit distance to the template, without check digits.** Rejected. It finds a shape but
  verifies nothing, and anchoring is the point.
- **F. Hard-code the §4.4 table, or assume `<` at position 1.** Rejected. The table is a prior
  until 2038, and assuming `<` grows more wrong every year
  ([`CONFORMANCE_BASIS.md`](../docs9303/CONFORMANCE_BASIS.md)).
- **G. Ask the Tier-2 model to choose the alignment.** Rejected. Where a check digit can decide,
  a model must not. Where none can, a model's pick is a plausible guess that nothing downstream
  could tell from a verified one.
- **H. Wait for ADR-0014 to make alignment unnecessary.** Partly right: on a well-fitted grid,
  classifying each cell in place removes most indels at the source. But `find_and_parse` is a
  public API fed plain text by callers who never touch our recognizer, and ADR-0014's own
  threshold 1 currently fails at parity. The text path needs its own answer.

## Consequences

**Positive**

- One search with one safety rule replaces an ordered list of shape-specific repairs, and the rule
  refuses instead of letting candidate order decide.
- #429 is handled without special-casing position 1's content. That matters more every year
  under §4.4.
- TD1's composite becomes usable as a cross-line anchor.
- One template table serves #421's coverage map, ADR-0014's position classes and the aligner, so
  the three cannot disagree.
- `mrz` gains a checkable statement of what alignment arithmetic can see — the shift law — as it
  already has for substitutions.
- A new, honest outcome: `AmbiguousAlignment` says "the anchors cannot choose" instead of a
  confident first pick.

**Negative**

- **Two scan paths coexist** until the old repairs are retired: more code, for a while, in the
  crate the product rests on.
- **A new public variant** on `MrzError`, and a new benchmark bucket. The bucket changes the
  baseline's shape and needs a re-bless.
- **Refusals can cost hits** that the current order happened to get right. The go/no-go counts
  each one.
- **The expected Tier-1 gain on the current corpus is small**, by the measurements above. The case
  rests on correctness of already-accepted documents and on maintainability, which is harder to
  show in one number.
- **The fix for names is not here.** No alignment can make a name correct; it can only stop a
  shift from corrupting the fields before it.

## Risks

- **Wrong-but-verifying alignments.**
  - A one-cell shift moves a check-digit sum among five residues, not ten.
  - Blocks of `0 5 A F K P U Z <` shift invisibly.
  - Filler-heavy fields are exactly where OCR drops cells.

  Mitigations: the bound k, admissible regions, unanimity over distinct readings, and P0.3's atlas
  measured before building.
- **Truth outside the candidate set.** A unique verifier can still be wrong when the true reading
  needs a substitution no repair models. Measured by P0.3's perturbed-truth arm.
- **Search growth.** Aligner candidates, crossed with per-line repair variants and the damaged
  pass, can multiply. Bound the total the way `MAX_DAMAGED_ATTEMPTS` does, and report when the
  bound is hit rather than returning a partial search as if it were complete.
- **MSRV, zero dependencies, wasm.** std only, within 1.82, and no new public types beyond the
  option and the error variant. CI's `msrv`, `semver` and wasm checks are the enforcement.
- **Registry gaps.** An incomplete `country_name` table
  ([#425](https://github.com/ruledicaprio/SynthPass/issues/425)) makes a correct issuing state
  fail its structural anchor. That turns a resolvable alignment into an ambiguous or unanchored
  one: it can cost a repair, but it cannot manufacture one.
- **Synthetic is not predictive** for repair classes on this project. Only the real gate can
  justify default-on.

## Staged rollout

Off by default at every stage until the go/no-go, like the chargrid and class-sweep arms.

0. **Phase 0.** Measurement and the dated note. No product code. Can end here.
1. **Layer 1 table** in `mrz`, crate-private, with the totality, honesty and shift-law tests.
   Zero behaviour change.
2. **Layer 2 aligner** behind `ParseOptions::with_strip_align(false)`, plus the
   `SYNTHPASS_MRZ_STRIP_ALIGN` arm and the replay A/B.
3. **Synthetic and real-gate A/B.** If go: default-on in an `mrz` minor release, CHANGELOG naming
   the behaviour change, and the baseline re-blessed in the same PR.
4. **Retire subsumed repairs one PR at a time**, each with a zero-difference replay and gate.
5. **Geometry-positioned cells from Layer 3**, under ADR-0015's amended go/no-go. This is the
   aligner's second version and needs its own measurement.

## Open questions for the maintainer

1. **Is the reframing acceptable?** The expected gain is correctness on already-accepted
   documents plus a simpler, safer scan, not Tier-1 count. If only a Tier-1 gain justifies the
   work, Phase 0's K1 should say so explicitly.
2. **Ambiguity semantics.** Whole-read refusal on checked-field ambiguity, and as-read line 1 on
   structural-prefix ambiguity: agreed? Should the bucket be named `ambiguous_alignment` and sit
   inside the scored denominator?
3. **Tier 2 and ambiguous readings.** Should Tier 2 ever receive the set of verifying readings as
   a constrained choice? Every candidate passes the validator, so no validator is overridden; the
   pick would carry Tier-2 provenance. Recommended: not in v1.
4. **Format order.** Keep §13.3's fixed order, or move to joint scoring across formats in a later
   amendment?
5. **The template table.** Stay crate-private, or become public through #421?
6. **Layer 3's home.** Fold it into ADR-0015 as an amendment (recommended), or keep it here?
7. **Numbering.** 0021 is not reserved in any tracked document. If it is being held for the VIZ
   layer-boundary ADR, renumber this one to 0022.

## Sequencing

- **Phase 0 can run now.** It needs no code, only one dump and an offline analysis, and changes
  no baseline.
- **Layers 1 and 2 are parser work inside `mrz`.** They add no dependency and no engine, so
  ROADMAP's M6 prohibition does not bar them. Anything that moves an outcome is still measured
  against the frozen list, per [ADR-0011](ADR-0011-split-m6-packaging-into-m8.md)'s amendment.
  Target release: `mrz` 0.9.0.
- **Layer 3 waits on ADR-0015's own sequencing.**
