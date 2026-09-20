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

> **Denominators, as of the 2026-09-19 re-bless.** The figures above are what CI run
> `35232307784` measured and are left as it measured them. The committed baseline now reads
> **140 of 152** scored and **12 of 45** name-scorable, because that re-bless moved two
> denominators and no reading — see
> [`denominator-rebless-2026-09-19.md`](../benchmarks/denominator-rebless-2026-09-19.md). The
> argument below is unaffected: it turns on the residue being character-level, which neither
> denominator change touches.

Two further facts bound what any change here can assume:

- **`ocrs` exposes no per-character score.** Its `TextChar` carries a rect and a character, no
  confidence; the repository's own `confidence` is a character-plausibility proxy computed after
  the fact (recorded as High debt in [`technical_debt.md`](../technical_debt.md), "OCR confidence
  is a character-plausibility proxy, not a model score"). Nothing downstream can currently rank
  two candidate readings of the same cell.
- **The CTC recognizer represents an isolated `<`, but its context suppresses it.** **Observed:**
  the 2026-09-16 probes (stretching the crop, re-spacing cells, and re-recognizing) saw the real
  suppression effect but named its cause incorrectly. The 2026-09-18 raw-matrix probe run over
  `samples/ocr_fixtures` read raw CTC label-29 (`<`) mass over 44 scored documents / 3,123
  cells from 62 fixtures / 255 images; 18 documents were skipped (7 `band_line_count_mismatch`, 5
  `no_line_fit_at_all`, 6 `no_mrz_band`). It separates 59 isolated truth-fillers from 980
  truth-fillers inside a run:

  | context window | isolated (n=59) mean / median | in-run (n=980) mean / median |
  | --- | --- | --- |
  | `cell_n0` (the cell alone) | **0.0573** / 0.0667 | 0.0636 / 0.0768 |
  | `cell_n1` | 0.0498 / 0.0270 | 0.0731 / 0.0845 |
  | `cell_n2` | 0.0486 / 0.0186 | 0.0686 / 0.0814 |
  | `cell_n4` | 0.0443 / 0.0099 | 0.0665 / 0.0775 |
  | full line | **0.0513** / 0.0382 | 0.0818 / 0.0824 |

  At `cell_n0`, isolated fillers carry 90% of the in-run `<` mass: the representation is
  nearly context-free. Any context drops the isolated-to-in-run ratio to 0.63–0.71 on means and
  0.13–0.46 on medians. The separation is not strictly monotone in window width: the mean ratio
  rises at `cell_n2` (0.682 → 0.709), and the median ratio recovers at full line (0.128 → 0.463).
  This zero-to-context step suggests a narrow-window read may capture most of the benefit. `<` is
  not the argmax in either population — absolute mass remains about 5–8% — so this
  establishes parity of representation at zero context, not that the beam should have emitted it.
  A per-cell or narrow-window read and fill-only geometric reconstruction are therefore distinct
  remedies.

  **Observed prototype floor:** over the same 3,123 cells, the `ocrs` beam reads 81.81% (2,555)
  and the best raw matrix mode, `cell_n2`, reads 80.85% (2,525). This is parity from a raw read
  with none of the masks, ink priors, or checksum search proposed below; it is a floor for the
  prototype, not a result for it.

Meanwhile 10 of the 13 still-open frozen M6 misses are `checksum_failed`
([ADR-0011](ADR-0011-split-m6-packaging-into-m8.md) amendment) — documents where an MRZ was
found and read, but at least one check digit disagrees. Those are recognition failures too.

### Threshold 1 is already below parity

The committed matrix probe measured 44 documents and 3,123 aligned cells. On that same population,
the ordinary `ocrs` beam read **2,555/3,123 (81.81%)** correctly. The best raw per-cell matrix mode,
`cell_n2`, read **2,525/3,123 (80.85%)**. The proposed raw cell read therefore misses threshold 1
(`per-cell accuracy ≥ ocrs`) by 30 cells, or 0.96 percentage points.

This is a measured result for the raw read, not a rejection of the constrained prototype. It does
mean that a per-cell read by itself buys nothing: any value in `mrz-cell` must come from the
constraints around it, and those constraints remain unmeasured.

The remaining work has three separately testable obligations:

- **Masks and position classes** must raise the same-cell accuracy to at least the `ocrs` reference
  (2,555/3,123) without silently dropping cells; this is threshold 1 on the fixed real-fixture
  population.
- **Ink priors and filler handling** must improve the strict name rate by at least 10 percentage
  points over the best `chargrid` arm on the 45 name-scorable real documents (the
  current baseline's count), while preserving the
  document-for-document Tier-1 count; these are thresholds 2 and 3.
- **Checksum-guided search** must recover only readings that satisfy the relevant checks, stay within
  the 200 ms p50 latency budget on the recorded machine, and produce byte-identical repeated reports;
  these are thresholds 3–5, with the latency and determinism measurements required on both the real
  and fixed-seed synthetic populations.

### Options after the parity result

- **A — Reject now.** Treat the raw read's 80.85% as the measured ceiling, keep the matrix numbers,
  and stop the prototype. This costs no implementation time and closes the question, but leaves the
  unmeasured constraints unexplored.
- **B — Narrow the proposal to a constraints-only prototype.** Keep this ADR Proposed, explicitly
  require masks, ink priors and checksum search to clear the five existing thresholds, and measure
  those pieces before funding a full classifier. This costs one controlled prototype and its A/B
  runs, but preserves a falsifiable route to the gains the raw read cannot provide.
- **C — Continue the full design as written.** Build every component and defer the parity failure
  until the end. This spends the most engineering time while leaving the first gate already known
  to fail, and risks treating an unmeasured constraint as an assumed improvement.

**Recommendation: B.** The raw result rules out the simplest interpretation of the ADR, while the
closed alphabet, grid geometry and checksum oracle still make a constrained experiment testable.
Narrowing the proposal records that distinction and makes the next spend answer the thresholds
directly. The maintainer may instead choose A and reject the ADR; this amendment does not change its
`Proposed` status.

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
