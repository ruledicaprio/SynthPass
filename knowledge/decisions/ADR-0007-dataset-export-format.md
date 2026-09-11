# ADR-0007 — Dataset export format: adopt the DeepSeek-OCR 0–1000 convention, JSONL first

**Status:** Accepted
**Date:** 2026-09-07

## Context

M6 commits to *"dataset exports (COCO / YOLO / JSONL / Hugging Face), consumed by at least one
external trainer end-to-end"* ([`ROADMAP.md`](../ROADMAP.md#m6--expansion--enterprise-readiness),
"Then — expansion and enterprise readiness"). With the deterministic core done — MRZ sequence
completeness and TD1/TD2/MRVA/MRVB reading through registered `synthpass-die` providers
([`ADR-0006`](ADR-0006-m6-accuracy-first.md)) — this is the next M6 track.

Four facts shape the decision, and the decision has to be made now rather than during
implementation.

**The roadmap fixes the format list but not the format.** "COCO / YOLO / JSONL / Hugging Face"
names four targets; COCO and YOLO are defined external standards, but the JSONL and Hugging
Face halves have no agreed schema. [`research/long-horizon-parsing.md`](../research/long-horizon-parsing.md)
§1 ("the one to act on ⭐") argues this is *"a choice about what the exporter writes, made
before the exporter is written, which is the only cheap moment to make it"* — pick the wrong
shape and every consumer integration pays for it; pick it after the code exists and changing it
is a breaking change.

**A well-travelled external recipe already exists.** The Baidu Unlimited-OCR / DeepSeek-OCR
data engine ([`papers/Unlimited_OCR_Works.md`](../papers/Unlimited_OCR_Works.md) §4.1) builds
its training corpus by concatenating each block's coordinates and content into one end-to-end
ground-truth string, normalising coordinates to **0–1000**, joining multi-page samples with a
`<page>` separator, and packing to ~32K tokens. That recipe is currently a well-travelled
training path. A corpus written in its shape is drop-in for anyone following it — which is
exactly how the M6 Definition of Done ("consumed by at least one external trainer end-to-end")
gets satisfied without going to find a trainer first.

**SynthPass is strictly ahead on the axis that limited that recipe.** Baidu pseudo-labelled
with an OCR engine because they had no generator, and inherited that engine's errors as ground
truth. `synthpass-gen` produces labels that are 100% accurate by construction — that is M2's
Definition of Done. `synthpass_gen::Labels` (`crates/synthpass-gen/src/labels.rs`) carries a
pixel bounding box for every VIZ field plus the whole MRZ band; the CLI already serialises them
to a per-image JSON sidecar (`crates/synthpass-cli/src/generate.rs`, `LabelsJson`).

**But the geometry is only rich on the generation side, and only for `--profile clean`.**
`ExtractionV2` (`crates/synthpass-core/src/v2.rs`) has no per-field bounding box — only
unpopulated `portrait` and `barcodes` slots — so COCO/YOLO can only ever describe *synthetic
generated* corpora, never a real-document extraction run. The generation labels themselves are
layout *slot* rectangles (not glyph-ink-tight), the portrait box and the per-MRZ-line boxes are
computed by `PageLayout` but never surfaced into `Labels`, and any geometry-changing degrade
profile (`border-kiosk` rotates 2.5°, `damaged` punches an occlusion disc) desyncs the
axis-aligned boxes because `degrade` applies no transform to them and returns none to invert.
See [`VIZ_TIER2_DESIGN.md §2.2`](../VIZ_TIER2_DESIGN.md#22-label--the-geometry-exists-and-is-thrown-away)
for the same "the geometry exists and is thrown away" observation from the Tier-2 side.

## Decision

**1. Record-based exports (JSONL, Hugging Face) adopt the DeepSeek-OCR / Unlimited-OCR
convention.** Coordinates normalised to integer **0–1000** relative to image width and height;
each block's normalised box and its content concatenated into one end-to-end ground-truth
string; `<page>` as the separator between concatenated documents; ~32K tokens as the packing
target for multi-document records. The exact row schema, the rounding rule, and the block
ordering are specified in [`EXPORTS.md`](../EXPORTS.md).

**2. JSONL ships first; COCO and YOLO follow in a later PR.** JSONL satisfies the M6 DoD on its
own, costs zero new dependencies, and needs no geometry work. COCO and YOLO each need work that
JSONL does not: surfacing `PageLayout::portrait` and the per-line `mrz_lines: Vec<Rect>` into
`Labels`, a decision on ink-tight versus slot boxes, an invented integer class taxonomy
(`CoreField::ALL` plus MRZ line/band classes, and the five formats have different geometry),
per-format normalisation (COCO absolute `[x, y, w, h]`; YOLO normalised `cx, cy, w, h`), and a
degradation transform threaded back out of `synthpass_gen::degrade`. [`EXPORTS.md`](../EXPORTS.md)
names each gap.

**3. "Hugging Face" export is a local on-disk layout, never a Hub push.** A directory of JSONL
shard(s) plus a `dataset_infos.json` / loading script that `datasets.load_dataset` reads from
disk. No network call, no upload — [`VISION.md §2`](../VISION.md#2-long-term-vision): *"it does
not do cloud anything"*, *"air-gapped or it does not ship"*.

**4. The exporter lives in a new `crates/synthpass-export` crate.** It depends on
`synthpass-gen` (`Labels`, `generate_from_seed`), `synthpass-core` (`CoreField`), `serde` /
`serde_json`, and `image`. `synthpass-cli` gets a hand-rolled `export` subcommand (no clap,
matching `generate.rs`) that calls into it. This matches the workspace's one-crate-per-concern
layout (`mrz`, `synthpass-gen`, `synthpass-bench`, `synthpass-license` are all separate crates),
keeps `synthpass-cli`'s dependency tree lean, and lets `synthpass-bench` reuse the exporter.

**5. Export is gated behind a new `FEATURE_EXPORT` license feature**, added to the Pro and
Enterprise tier presets in `crates/synthpass-license/src/lib.rs`, enforced the same way
`FEATURE_BATCH` gates the `batch` subcommand. Bulk dataset production is a *"higher-capacity
generation"* surface under [`BRANDING.md §5`](../BRANDING.md#5-commercial-strategy), which draws
the paid boundary at *"capacity, support, and enterprise-integration surfaces"*. A single
`generate` call stays free and ungated — it is the core; a corpus builder is a capacity knob.

**6. v1 exports `--profile clean` only.** It is the only profile whose axis-aligned boxes stay
pixel-accurate. Degraded-profile export waits on the `degrade` transform work in decision 2.

## Alternatives rejected

**COCO and YOLO in the first PR.** Roughly two to three times the work — the geometry surfacing,
class taxonomy, per-format normalisation, and degrade-transform items above — for annotation
formats that can only ever describe synthetic corpora, when JSONL already clears the DoD.
Deferred, not dropped: the format list stays four.

**Absolute-pixel coordinates for the JSONL rows.** Loses the drop-in property with the
DeepSeek-OCR training recipe, which is the entire reason `long-horizon-parsing.md` says to fix
the convention now. A consumer following that recipe would have to rescale every box, and a
format nobody else reads is the status quo this ADR exists to avoid.

**A Parquet / Arrow Hugging Face export.** A heavy new dependency category against the
workspace rule (`Cargo.toml`: *"new dependencies must be pure Rust or justify themselves in
writing"*), and there is no Parquet or Arrow crate in the tree today. `datasets` loads JSONL
from disk natively, so columnar buys nothing for the DoD. Revisit only with its own ADR if a
real consumer needs it.

**Export logic as a module in `synthpass-gen` or `synthpass-cli`.** `synthpass-gen` has no
`serde` dependency today — the CLI keeps a hand-mirrored copy of `Labels` precisely to avoid
adding one to the generator — so a module there pulls `serde` into the generator. A module in
`synthpass-cli` strands the logic in a thin binary that `synthpass-bench` cannot reuse.

**Leave export ungated, like `generate`.** The synthetic-data / no-PII argument is real and is
why `generate` itself is ungated. But bulk dataset production is a **capacity surface** rather
than a product feature (`BRANDING.md §5`), and a batch corpus builder is exactly that. Gating the
corpus builder while leaving single-document `generate` free is the same line the licensing
model already draws for `batch` versus single-document `extract`.

*(Citation corrected 2026-09-11: this paragraph previously quoted `BRANDING.md §5` as putting
"higher-capacity generation … knobs" in "the paid tier". §5 has since rejected a feature-gated
paid tier and retained `synthpass-license` as capacity metering — the framing
`crates/synthpass-cli/src/export.rs` already uses in its own module docs. The decision recorded
here is unchanged; only the quotation, which no longer matched its source.)*

**Push finished datasets to the Hugging Face Hub.** Violates the `VISION.md` air-gap non-goal
outright — a network call in a tool whose musl single-file air-gapped build is a shipped
feature. "Hugging Face export" means the on-disk `datasets`-loadable layout and nothing more.

## Consequences

**Positive**

- The JSONL / Hugging Face schema is pinned before any code is written — the one cheap moment.
- The "consumed by an external trainer" DoD has a concrete answer (the DeepSeek-OCR recipe)
  instead of a search for a willing trainer.
- Zero new dependencies for the v1 (JSONL + Hugging Face) scope.
- Gating reuses the existing offline-Ed25519 feature mechanism and matches how `batch` is
  already treated — no new licensing concept.

**Negative**

- The 0–1000 convention is a 2026 recipe, not a standard; it could be superseded before M6
  lands. The risk is bounded — it costs one serialisation function either way, and a format
  nobody else reads is the alternative.
- COCO / YOLO fidelity work is real and deferred: ink-tight boxes, the degradation transform,
  and surfacing the portrait and per-line MRZ geometry into `Labels`.
- `crates/synthpass-export` is another workspace member to build and maintain.
- v1 exports only `--profile clean`, so the first datasets carry no capture degradation.

**Explicitly not licensed by this decision:** exporting real-document extraction results as
COCO/YOLO (the schema has no per-field geometry and this ADR does not add it), pushing to any
remote dataset host, and any new heavy dependency for a columnar format.
