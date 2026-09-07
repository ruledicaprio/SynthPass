# EXPORTS.md — `synthpass export` dataset formats

**Status:** JSONL and Hugging Face formats shipped (`crates/synthpass-export`, the
`synthpass export` subcommand); COCO / YOLO deferred (see "Deferred" below). The formats and
coordinate convention are fixed by
[`decisions/ADR-0007-dataset-export-format.md`](decisions/ADR-0007-dataset-export-format.md);
this is an M6 "Then — expansion and enterprise readiness" item
([`ROADMAP.md`](ROADMAP.md#m6--expansion--enterprise-readiness)).

`synthpass export` turns a deterministic `synthpass-gen` corpus into a training dataset in a
standard on-disk shape. It is the sibling of [`SYNTHPASS.md`](SYNTHPASS.md)'s `synthpass-bench`
(which *measures* the pipeline) and [`ADVERSARIAL.md`](ADVERSARIAL.md)'s degraded profiles
(which stress it): same seeded generator, different output.

## Why the labels are worth exporting

`synthpass-gen` labels are **100% accurate by construction** — that is M2's Definition of Done,
not an aspiration. Every generated document already carries, in `synthpass_gen::Labels`
(`crates/synthpass-gen/src/labels.rs`): a `value` + pixel `Rect` for each of the nine VIZ
fields (plus optional `personal_number`), the MRZ lines as text, and one `Rect` for the whole
MRZ band. A corpus exported from these is cleaner ground truth than the OCR-pseudo-labelled
corpora most training recipes were built on (see
[`research/long-horizon-parsing.md`](research/long-horizon-parsing.md#1-the-data-engine--the-one-to-act-on-)).

## CLI surface

```text
synthpass export --format FMT [--count N] [--seed N] [--document-type TYPE]
                 [--profile clean] [--pack-pages N] --out-dir DIR
  --format FMT          jsonl | hf        (coco / yolo error as "not implemented yet")
  --count N             number of documents to generate and export (default: 100)
  --seed N              base seed; document i uses seed N + i (default: 0)
  --document-type TYPE  td1 | td2 | td3 | mrva | mrvb | all   (default: td3)
                        "all" round-robins the five formats across the corpus by
                        document index, matching synthpass-bench's --profile all
  --profile clean       fixed at "clean" in v1 (see "Capture profiles" below)
  --pack-pages N        concatenate N documents per JSONL row with <page> separators
                        (default: 1). Applies to hf too — its rows are the same JSONL rows.
  --out-dir DIR         output directory, required (created if absent)
```

Hand-rolled flag parsing, no clap — consistent with `crates/synthpass-cli/src/generate.rs`.
The subcommand is **gated on `FEATURE_EXPORT`** (see "Licensing" below); it calls into the
`crates/synthpass-export` library (`synthpass_export::run(&ExportConfig)`), which is where
every consumer (`synthpass-cli`, `synthpass-bench`, tests) reaches it.

Per-document generation is exactly `generate.rs`'s loop:
`synthpass_gen::generate_from_seed` on `GeneratorConfig::with_document_type(base_seed + i,
doc_type)` → `(image, labels, _passport)`, `include_personal_number` always `true`. v1 always
writes the rendered PNGs alongside the JSONL (`images/<doctype>-<seed:06>.png`).

## The 0–1000 coordinate transform

Every box in a record-based export is normalised to integers in `[0, 1000]`, relative to the
rendered image dimensions (`image.width()` / `image.height()`), matching the DeepSeek-OCR /
Unlimited-OCR data-engine convention (ADR-0007):

```text
x0' = clamp(round(rect.x                * 1000 / image_width ), 0, 1000)
y0' = clamp(round(rect.y                * 1000 / image_height), 0, 1000)
x1' = clamp(round((rect.x + rect.width ) * 1000 / image_width ), 0, 1000)
y1' = clamp(round((rect.y + rect.height) * 1000 / image_height), 0, 1000)
```

Round half to even (Rust `f64::round` is half-away-from-zero; either is fine, but the choice is
pinned so exports are byte-reproducible). A box is stored as `[x0', y0', x1', y1']` — top-left
and bottom-right, not width/height — because the concatenated ground-truth string (below) reads
better with explicit corners and downstream recipes expect them.

## JSONL row schema

One JSON object per line. One line per row; a row is one document when `--pack-pages 1`
(default), or `N` documents when `--pack-pages N`.

```jsonc
{
  "id": "td3-000042",                 // "<doctype>-<seed:06>"; for a packed row,
                                       // "<doctype>-<firstSeed>+<n>" (doctype = the first doc's)
  "documents": [                       // one entry per packed document (length 1 by default)
    {
      "seed": 42,
      "document_type": "td3",          // td1 | td2 | td3 | mrva | mrvb
      "mrz_format": "TD3",             // synthpass_gen mrz_format.as_str()
      "image": "images/td3-000042.png",// path relative to the JSONL file; always written in v1
      "width": 1200,                   // rendered image pixels (informational; boxes are 0–1000)
      "height": 840,
      "blocks": [                      // reading order: VIZ fields top-to-bottom, then MRZ
        { "field": "document_type",   "value": "P",           "box": [17, 60, 45, 90] },
        { "field": "issuing_country", "value": "BRA",          "box": [ ... ] },
        { "field": "surname",         "value": "ESKANDARI",    "box": [ ... ] },
        { "field": "given_names",     "value": "MAREN",        "box": [ ... ] },
        { "field": "document_number", "value": "FLLF2W13I",    "box": [ ... ] },
        { "field": "nationality",     "value": "BRA",          "box": [ ... ] },
        { "field": "date_of_birth",   "value": "1975-05-18",   "box": [ ... ] },
        { "field": "sex",             "value": "F",            "box": [ ... ] },
        { "field": "date_of_expiry",  "value": "2034-04-18",   "box": [ ... ] },
        { "field": "personal_number", "value": "ZE184226",     "box": [ ... ] },  // omitted if None
        { "field": "mrz_line",        "value": "P<BRAESKANDARI<<MAREN<<<<<<<<<<<<<<<<<<<<<<<<", "box": [ ... ], "line": 0 },
        { "field": "mrz_line",        "value": "FLLF2W13I8BRA7505184F3404187ZE184226<<<<<<02",  "box": [ ... ], "line": 1 }
        // TD1 has three "mrz_line" blocks
      ],
      "mrz": {
        "lines": [ "P<BRA...", "FLLF2W13I8BRA..." ],   // synthpass_gen Labels.mrz_lines verbatim
        "format": "TD3",
        "band_box": [ ... ]                            // the whole-MRZ-band Rect, normalised
      },
      "ground_truth": "<block>...<block>"              // see "Ground-truth string" below
    }
  ],
  "ground_truth": "…<page>…"            // documents[*].ground_truth joined with "<page>"
}
```

**Field vocabulary.** The nine VIZ `field` names are exactly `synthpass_core::v2::CoreField`'s
`serde` names (`document_type`, `issuing_country`, `document_number`, `surname`, `given_names`,
`nationality`, `date_of_birth`, `sex`, `date_of_expiry`, `personal_number`). MRZ lines use the
synthetic name `mrz_line` with a 0-based `line` index. `synthpass-export` re-exports the
`CoreField` list rather than hard-coding a fourth parallel copy of the ICAO field names (the
repo already has three — `ROADMAP.md` "Open backlog", `knowledge/technical_debt.md`).

**Value forms.** `date_of_birth` / `date_of_expiry` are ISO `YYYY-MM-DD` (as in `Labels`); the
MRZ strings carry the `YYMMDD` forms. `sex` is one character. Values are the generator's ground
truth verbatim — never re-derived from the image.

**Blocks with no geometry.** Every VIZ field and every MRZ line has a real `Rect` in `Labels`
today, so `box` is always present in v1. Do not emit a block for a field the document does not
have (`personal_number` on a document generated without one).

## Ground-truth string

Per document, concatenate each block as:

```text
<|ref|><field><|/ref|><|box|>x0,y0,x1,y1<|box|><value>
```

in the same reading order as the `blocks` array, no separator between blocks. This is the
"coordinates and content concatenated into end-to-end ground truth" shape from the data-engine
recipe; the exact sentinel tokens are pinned here so every export is identical. For a packed
row (`--pack-pages N`), the top-level `ground_truth` joins the per-document strings with the
literal token `<page>`. SynthPass documents are single-page, so `<page>` only ever appears
between packed samples, never inside one.

Packing targets ~32K tokens per row as a soft cap: stop adding documents to a row once the
running character count crosses `32000 * 4` (a coarse chars-per-token proxy — the exporter does
not depend on a tokenizer). A single document always gets its own row even if it exceeds the
cap.

## Manifest

One `manifest.json` at `--out-dir` root, generated (never hand-edited), mirroring
`SYNTHPASS.md`'s stance on bench reports:

```jsonc
{
  "synthpass_version": "1.4.0",           // crate version of synthpass-export
  "generator_version": "1.4.0",           // synthpass-gen version — the labels' provenance
  "format": "jsonl",
  "created_unix": 1750000000,
  "seed_base": 0,
  "count": 100,
  "document_type": "td3",                 // or "all"
  "profile": "clean",
  "pack_pages": 1,
  "command": "synthpass export --format jsonl --count 100 --seed 0 --document-type td3 --out-dir ./ds",
  "files": [
    { "path": "data.jsonl", "sha256": "…", "rows": 100 }
  ]
}
```

Reproducing an export needs only `seed_base`, `document_type`, `profile`, `pack_pages`, and the
`seed_base + i` rule (`include_personal_number` is always `true` from this path). Determinism is
guaranteed by `crates/synthpass-gen/tests/determinism.rs` — byte-identical pixels and equal
labels for a given seed across all five formats.

## Hugging Face layout

`--format hf` writes a directory `datasets` can load from disk with no network:

```text
<out-dir>/
  data/
    train.jsonl              # the JSONL rows above (sharded as data/train-00000-of-000NN.jsonl
                             # once a shard would exceed ~256 MB)
  dataset_infos.json         # features schema + split sizes, so load_dataset("<out-dir>") works
  manifest.json              # the same manifest as above
  README.md                  # dataset card: generator version, seed range, license, the
                             # "synthetic, watermarked, non-country template" statement
```

Loadable as `datasets.load_dataset("<out-dir>")` or
`datasets.load_dataset("json", data_files="<out-dir>/data/train.jsonl")`. **No `push_to_hub`,
no Hub API, no upload** — ADR-0007 decision 3, `VISION.md §2`. If a user wants it on the Hub
they run `huggingface-cli upload` themselves against the local directory; SynthPass never makes
that call.

## Licensing

`synthpass export` is gated on a new `FEATURE_EXPORT` constant in
`crates/synthpass-license/src/lib.rs`, added to `Tier::Pro` and `Tier::Enterprise`
`default_features()`. Enforced in `synthpass-cli` with `check_license_feature(FEATURE_EXPORT)`
before the subcommand runs — the same shape as `FEATURE_BATCH` guarding `batch`
(`crates/synthpass-cli/src/main.rs`). Rationale in ADR-0007 decision 5 and
[`BRANDING.md §5`](BRANDING.md#5-commercial-strategy): bulk dataset production is a capacity
surface; single-document `generate` stays free.

`SYNTHPASS_LICENSE_SKIP=1` bypasses the gate for local development, consistent with the rest of
the CLI.

## Capture profiles

v1 exports `--profile clean` only. The `Labels` rects are computed on the pristine render
(`labels.rs` `build_labels`); `synthpass_gen::degrade` runs *after* labelling, applies no
transform to the rects, and returns none to invert. `border-kiosk` (2.5° rotation) and
`damaged` (occlusion disc) therefore desync the axis-aligned boxes from the pixels; `mobile` /
`scanner` / `worn` are photometric-only and *would* be safe, but are held back until the
degrade API can report its geometric transform so every profile is handled the same way.

## Deferred: COCO / YOLO

Both are bounding-box detection formats. They can only ever describe *synthetic generated*
corpora — `ExtractionV2` has no per-field geometry (`crates/synthpass-core/src/v2.rs`, only the
unpopulated `portrait` / `barcodes` slots), so there is no real-document extraction run to
annotate. Shipping them needs, in order:

1. **Surface the geometry `Labels` hides.** `PageLayout` (`crates/synthpass-gen/src/layout.rs`)
   already computes the `portrait` rect, the `watermark` rect, and `mrz_lines: Vec<Rect>` (per
   line, not just the band); `mrz_char_rect_for_line()` gives per-character cells. `Labels`
   surfaces none of it. COCO/YOLO want at least the portrait box and the per-line MRZ boxes.
2. **Decide ink-tight vs slot boxes.** `Labels` rects are fixed layout column slots (`SURNAME`
   is always 700 px wide regardless of the value). With the default build (`embedded-fonts`
   off) the placeholder bar fills the slot, so the box is tight; with real fonts on, VIZ text
   is left-aligned and narrower. Detection training usually wants tight boxes — that means a
   glyph-ink bounding box the codebase does not compute anywhere today.
3. **Invent the class taxonomy.** COCO/YOLO need an integer category set. `CoreField::ALL` is
   a natural 10-class basis, but `mrz_line` (2–3 boxes), the MRZ band, and `personal_number`
   (optional, TD3-only) complicate a fixed list, and the five formats have different layouts.
4. **Thread the degrade transform out** so degraded profiles can be exported with boxes that
   match the pixels (also unblocks non-`clean` JSONL export).
5. **Per-format writers.** COCO: one `images[]` + `annotations[]` + `categories[]` JSON.
   YOLO: one `.txt` per image, `class cx cy w h` with `cx,cy,w,h` normalised to `[0,1]` (note:
   `[0,1]` floats, *not* the 0–1000 integers the JSONL side uses — YOLO's own convention wins
   for a YOLO export), plus a `data.yaml`.

See [`VIZ_TIER2_DESIGN.md §2.2`](VIZ_TIER2_DESIGN.md#22-label--the-geometry-exists-and-is-thrown-away)
for the same "geometry exists and is thrown away" gap from the extraction side.

## Anticipated schema growth

`place_of_birth`, `issuing_authority`, and `date_of_issue` are planned canonical fields
([`VIZ_TIER2_DESIGN.md §2.4`](VIZ_TIER2_DESIGN.md#24-add-the-fields-only-the-viz-has),
`ROADMAP.md` "Open backlog"). The `blocks` array is additive — a new `field` value slots in
without a schema version bump — so an exporter written now does not block them.
