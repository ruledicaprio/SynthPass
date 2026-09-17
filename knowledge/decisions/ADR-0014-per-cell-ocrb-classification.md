# ADR-0014 — `mrz-cell`: per-cell OCR-B classification for the MRZ band, as a benchmark-first prototype

**Status:** Proposed
**Date:** 2026-09-17

## Context

The deterministic Tier-1 path reads **140 of 153** scored real specimens
([`benchmarks/README.md`](../benchmarks/README.md), CI run `35232307784`, MAIN `649cccc`,
`samples-data` `5988c7b`). The same run gives the first CI-measured name figures
([ADR-0013](ADR-0013-names-are-scored-against-mrz-form-truth.md)): **12 of 40** name-scorable
scored documents are a hit whose surname *and* given names are exact — **21 of the 33
name-scorable hits carry a wrong name**. A hit proves the document number and the dates. It
proves nothing about the name, because no ICAO 9303 check digit covers one.

The committed per-document ledger
([`real-specimen-outcomes.jsonl`](../benchmarks/real-specimen-outcomes.jsonl)) classifies those
21 (`name_error`, the taxonomy in `crates/synthpass-bench/src/lib.rs`'s `NameError`):

| `name_error` | Count | What it means |
| --- | --- | --- |
| `other` | 18 | Diverged some way none of the specific rules names — an ordinary character misread, or a shifted split compounded with one |
| `split_shifted` | 2 | Both names present and in order, but the surname/given-names boundary moved |
| `filler_read_as_letters` | 1 | A trailing `<` run came back as `C`/`K` letters |

**The fixed-grid repair now in the tree cannot close most of that.**
`synthpass_ocr::chargrid` ([#331](https://github.com/ruledicaprio/SynthPass/pull/331)) restores
`<` fillers the recognizer dropped, using the character positions it did return. A 2026-09-17
review of all 21 against their fixtures put its ceiling at **at most 5**: the 2 measured
`split_shifted` cases plus 3 `other` cases that dropped fillers alone can explain. The
remaining 16 need a character to be read *correctly*, which filling cannot do. So even a
perfect fill-only repair moves the strict rate from 12/40 to about 17/40, and the residue is
character-level.

Two further facts bound what any change here can assume:

- **`ocrs` exposes no per-character score.** Its `TextChar` carries a rect and a character, no
  confidence; the repository's own `confidence` is a character-plausibility proxy computed after
  the fact (recorded as High debt in [`technical_debt.md`](../technical_debt.md), "OCR confidence
  is a character-plausibility proxy, not a model score"). Nothing downstream can currently rank
  two candidate readings of the same cell.
- **The CTC recognizer never emits an isolated `<`.** Three probes on 2026-09-16 (stretching the
  crop, re-spacing the cells, re-recognizing) failed to make it produce one; the model has no
  representation to emit. This is why `chargrid` reconstructs filler *positions* geometrically
  instead of asking for a better read.

Meanwhile 10 of the 13 still-open frozen M6 misses are `checksum_failed`
([ADR-0011](ADR-0011-split-m6-packaging-into-m8.md) amendment) — documents where an MRZ was
found and read, but at least one check digit disagrees. Those are recognition failures too.

### Why the MRZ band is unusually favourable to a non-learned classifier

A generic scene-text recognizer solves a much harder problem than this one. On an MRZ band:

- the alphabet is closed: `A`–`Z`, `0`–`9`, `<` — 37 glyphs, no case, no punctuation, no script
  variation;
- the typeface is specified: OCR-B, monospaced, designed for machine reading;
- the layout is a fixed grid: 2 × 44 (TD3, MRV-A), 2 × 36 (TD2, MRV-B), 3 × 30 (TD1);
- most positions are constrained before any pixel is read: a date field is six digits, a check
  digit is one digit, the issuing state is three letters-or-`<`;
- and four fields plus a composite carry **check digits**, so a candidate reading can be tested
  rather than trusted.

The repository already holds the pieces: the grid fitting, ink measurement and cell segmentation
in `synthpass_ocr::chargrid`; a vendored OFL-licensed `ocr-b.ttf` in
`crates/synthpass-gen/fonts/` with `ab_glyph` already a dependency for rasterizing it; and the
checksum oracle in `mrz`.

## Decision (proposed)

Prototype **`mrz-cell`**, a per-cell OCR-B classifier for the MRZ band, behind a cargo feature and a
three-arm environment variable, and decide it by measurement. Nothing enters the default path
before the go/no-go below is met.

1. **Scope.** The MRZ band only, after the existing detection and orientation steps have chosen a
   crop. Not a page recognizer, not a replacement for `ocrs` anywhere else.
2. **No training, no model.** Templates are rendered from the vendored OCR-B font at the pitch and
   height the grid fit reports. This keeps
   [`VISION.md`](../VISION.md) §2's non-goal (no custom-trained models) intact and keeps the path
   deterministic and explainable: every decision is a score against a rendered glyph, inspectable
   by a human.
3. **Method.**
   - Segment with `chargrid`: `fit_grid` / `grid_origin_from_ink` / `cell_ink` give the cell
     boundaries already used for filler repair.
   - Render the 37 glyphs at the fitted geometry; score each binarized cell by normalized
     cross-correlation against each template.
   - Constrain per position with a new **pure** `mrz` function, `position_class(format, line, col)`,
     returning the allowed set (digits, letters-or-filler, the closed alphabet). Additive API,
     so it ships with an `mrz` minor release.
   - Where a check digit covers a field, search the top-k cell candidates for a reading that
     satisfies the digit; accept the best scoring valid one.
   - Names carry no check digit, so they take the per-cell argmax, with an ink-mass prior for `<`
     (a filler has a small but real footprint, the same signal `cell_ink` already measures).
4. **Arms.** Cargo feature `mrz-cell` (off by default, so no cost to anyone not measuring) and
   `SYNTHPASS_OCR_MRZ_CELL=off|on|control`, mirroring the existing arm pattern. `control` runs the
   same work and discards the treatment, so the A/B measures the treatment and not the cost.
5. **Byproduct, deliberately sought:** a genuine per-character score. If the prototype ships, the
   `confidence` proxy debt above becomes fixable for the MRZ path.

## Go / no-go

Measured on the real fixtures (the 40 name-scorable scored documents and the 10 `checksum_failed`
misses) and on a fixed-seed synthetic set, as a same-binary A/B against the best `chargrid` arm.
All five must hold:

1. **Per-cell accuracy ≥ `ocrs`** on the same cells, counted against fixture truth.
2. **Strict name rate ≥ +10 pp** over the best `chargrid` arm on the real fixtures.
3. **No Tier-1 loss:** `tier1_hits` ≥ the committed baseline, document for document.
4. **Latency:** p50 added ≤ 200 ms per document, measured on a machine with a recorded descriptor
   and no concurrent build.
5. **Determinism:** two runs of the same commit over the same corpus produce byte-identical
   reports.

**A no-go sets this ADR to Rejected and keeps the numbers.** A rejected prototype that measured
its own ceiling is a result, not a loss: it tells the next attempt where the residue really is.

## Alternatives considered

- **Fill-only repair alone (`chargrid`, already merged).** Cheap, already built, and bounded at
  about 5 of 21 by the review above. It stays regardless; this ADR addresses the residue it
  cannot reach.
- **PP-OCRv5 through `rten`.** `rten` is already a dependency and `rten-convert` imports ONNX, so
  adopting it needs no new C dependency
  ([`long-horizon-parsing.md`](../research/long-horizon-parsing.md)). But its recognizer is
  Chinese-centric multilingual and **its OCR-B accuracy is unverified**; it is a second bake-off
  candidate, not a decision.
- **Fine-tuning `ocrs` on MRZ crops.** Rejected: [`VISION.md`](../VISION.md) §2 rules out
  custom-trained models, and it would replace an inspectable rule with a weight file.
- **Tesseract with an OCR-B model.** Rejected: a C dependency, against the project's
  single-binary, source-buildable posture.
- **Tier-2 LLM name repair.** Rejected for this purpose: a model rewriting a name it cannot verify
  produces a plausible name, not a correct one, and nothing downstream could tell the difference.

## Consequences

- One feature-gated module and one additive `mrz` function; no default-path change until the
  go/no-go is met, so the committed baseline cannot move while this is being measured.
- Template rendering needs the OCR-B font across the `synthpass-gen` → `synthpass-ocr` boundary.
  How to share it (a relative `include_bytes!` versus a workspace-level `fonts/`) is an open
  question, not settled here.
- If it goes, the MRZ path stops depending on a general recognizer for the one thing that
  recognizer is worst at, and gains a real per-character score.
- If it does not, the measurement still tells us whether the residue is segmentation, geometry or
  genuinely ambiguous ink.

## Open questions

- Where the OCR-B font lives once two crates need it.
- Whether `position_class` belongs in `mrz` (proposed) or in `synthpass-ocr`.
- Whether TD1's synthetic watermark line can be excluded by geometry alone before classification.

## Sequencing

After the `chargrid` A/B has a number (so "+10 pp over the best arm" has a value), and after M6's
Definition of Done is evaluated against its frozen list — this is a recognition change, and
[ADR-0011](ADR-0011-split-m6-packaging-into-m8.md)'s amendment keeps M6 scored on the frozen
snapshot.
