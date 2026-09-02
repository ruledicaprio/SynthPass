# samples/

Identity-document specimen images used as test/example fixtures for the OCR,
LLM, and pipeline crates. Files are organized by document kind into
subdirectories and follow a filename convention (see "Naming convention"
below), with the per-file metadata recorded in `corpus.jsonl`.

**Images are not tracked on `main`.** They live on the orphan `samples-data`
branch — run `./scripts/sync-samples.ps1` after a fresh clone (or any time
you want the latest corpus) to populate this directory locally; it's
`.gitignore`d otherwise. Four `ocr_fixtures/` images are the exception: they
are force-added because required CI (`native_ocr_e2e`, `rust_ocr_smoke`)
panics without them and does not sync the corpus first.

Tracked under `samples/` on `main`: this README, `corpus.jsonl` (the manifest),
`ocr_fixtures/*.json` + `*.md` (the hand-verified ground truth),
`ocr_fixtures/derived/*.json` + `*.md` (generated parity candidates), and those
four images. `scripts/sync-samples.ps1` excludes `*.json`/`*.md` from **every**
mirrored directory, so ground truth cannot drift onto `samples-data` even when
it sits next to its image. See CONTRIBUTING.md's "Adding a corpus specimen"
section and `knowledge/benchmarks/README.md`'s "local bench loop" for how the
corpus grows now.

## Ground truth: reviewed vs. derived

`ocr_fixtures/` holds pairs — an `.md` (the OCR text a Tier-2 prompt receives)
and a `.json` (a `synthpass_core::Extraction` to score against). Both
directories hold the same file format; the difference is **who vouched for it**.

| Directory | Origin | What `parity.rs` scores |
| :-- | :-- | :-- |
| `ocr_fixtures/` | a person compared it against the image | all nine prompt fields |
| `ocr_fixtures/derived/` | generated from a checksum-valid MRZ | `document_number`, `date_of_birth`, `date_of_expiry` |

The split exists because an ICAO check digit covers only four fields. A derived
fixture's *name* is one OCR pass's reading of characters nothing arbitrates, so
scoring a model against it would penalise a model that read the printed name
correctly. Regenerate both with:

```powershell
cargo run -p synthpass-bench --release --example ground_truth_candidates
```

To promote a candidate, check its unproven fields against the image, then
`git mv samples/ocr_fixtures/derived/<stem>.json samples/ocr_fixtures/` (and its
`.md` with it). Nothing in the generator ever writes a `.json` into the reviewed
directory.

## Layout

```
samples/
  passports/          passport specimen images (TD3 MRZ format) — gitignored, local mirror
  id_cards/            national identity card images (TD1 / TD2 MRZ format) — gitignored, local mirror
  driving_licenses/     driving licence specimen images (no MRZ) — gitignored, local mirror
  ocr_fixtures/         hand-verified OCR ground truth (.md + .json) — tracked; 4 images force-added for CI
    derived/            generated, unreviewed parity candidates (.md + .json) — tracked
  misc/                 unclassifiable specimens (e.g. border-pass documents, wiki reference image) — gitignored, local mirror
  corpus.jsonl          one row per image: MRZ document code, issuing state, provenance, ground-truth link — tracked
  README.md
```

`tools/fetch_commons_mrz_specimens_v2.py` (repo root `tools/`) is the
scraper used to collect candidate specimen images from Wikimedia Commons; it
stages them in a gitignored directory for `scripts/ingest-fetched-samples.ps1`
to sift into `samples/` (see CONTRIBUTING.md). It lives outside `samples/`
since it is not itself a test fixture.

## Document kind -> MRZ format

| Directory            | Document kind              | MRZ format                                   |
|-----------------------|-----------------------------|-----------------------------------------------|
| `passports/`          | Passport                    | TD3 (2 lines x 44 chars, on the biodata page) |
| `id_cards/`           | National identity card      | TD1 (3 x 30) or TD2 (2 x 36); MRZ is often on the *back/rear* image of a front+back pair |
| `driving_licenses/`   | Driving licence              | No MRZ — not a machine-readable travel document |
| `ocr_fixtures/`       | Mixed (passport/ID pages used as OCR ground truth) | Varies; see each fixture's `.md`/`.json` |

## Naming convention

Filenames are lookup keys: the `find_sample` / `walk_samples` test helpers walk
`samples/` recursively and match by **basename**. Passports follow:

```
Country_DocType_Specimen_Code_State_YYYY[_redacted]_(mrz|no_mrz)[_variant…].ext
```

- `Country` — issuing country/territory, `Snake_Case` or `Title_Case`
  (e.g. `Croatia`, `North_Macedonia`).
- `DocType` — `Passport` or `ID`, matching the subdirectory. That agreement is
  load-bearing: `synthpass_bench::classify_specimen` reads the document class
  off the directory, never the filename.
- `Specimen` / `Private` — provenance.
- `Code` — MRZ line 1 positions 1–2, the ICAO document code, with `0` standing
  in for the filler `<` (a path cannot hold `<`). So `P0` is `P<`, `PP` is
  `PP`. **`PO` with a letter O is a different code** and genuinely occurs.
- `State` — MRZ line 1 positions 3–5, the issuing state, same `0`→`<`
  substitution. Germany's legacy `D<<` is written `D00`.
- `YYYY` — the year. Which year it means is recorded per file in the manifest's
  `year.kind`, because it is the issue year in most names and the expiry year
  in at least one.
- `_redacted` — the MRZ is physically blacked out on the source.
- `_mrz` / `_no_mrz` — whether this image carries an MRZ at all. Not optional:
  it keys `integrity_survey.rs --mrz-only`, which skips names containing
  `no_mrz` (a *negative* filter — an image with no `_mrz` tag at all is kept).
- `variant…` — free-form tags for how the image is degraded or framed:
  `blur`, `rotated`, `highlight`, `contrast`, `pixelated`, `wide`, `child`,
  `emergency`, `inner_page`.
- `ext` — prefer `jpg`/`png`; `webp` only when that is the source format.

`id_cards/` and `driving_licenses/` predate the `Code`/`State` slots and keep
the older `Country_DocType_Specimen[_YYYY][_side]_mrz.ext` form.

Basenames must stay **unique across all of `samples/`**, regardless of
subdirectory, since lookups are basename-based and do not disambiguate by
directory.

### The manifest, not the filename, is the source of truth

`samples/corpus.jsonl` (tracked on `main`) carries one row per image, and it —
not the name — is what code reads. A filename cannot be validated; the manifest
is checked on every `cargo test --workspace` by
`crates/synthpass-bench/tests/corpus_manifest.rs`, which reads only the
manifest and so runs even in a clone that has no images.

Regenerate it after adding or renaming a specimen:

```powershell
cargo run -p synthpass-ocr --release --example corpus_manifest
cargo run -p synthpass-ocr --release --example corpus_manifest -- --check   # report only
```

Rows are keyed by content hash, so an unchanged image is never re-read. Note
that `mrz.document_code` / `mrz.issuing_state` come from the **filename**, and
`mrz.observed` from OCR: line 1 positions 1–5 carry no check digit in any ICAO
format, and the recogniser misreads them on most passports (`PS` for `P<`,
`OOO` for `SVK`). Where the two disagree the human who named the file wins, and
the generator prints the disagreement for review.

`mrz_corpus.rs`'s positive and negative sets and `parity.rs`'s fixture list are
both derived from the manifest, so a rename no longer strands them — which it
previously did, silently, for 16 of 21 corpus entries and all 6 parity fixtures.

## Provenance

Images are public specimen / illustrative document images collected from
Wikipedia's "Passports by country" category and related identity-document
categories (national ID cards, driving licences). They depict specimen or
void documents published for public reference, not real issued documents
belonging to private individuals, unless explicitly marked otherwise (e.g.
filenames containing `_private`).

## Licensing / usage

These images are used here strictly for specimen/illustrative purposes —
as test fixtures for MRZ/OCR parsing and layout detection. Refer to the
original Wikipedia/Wikimedia Commons file pages for the specific license
of each image if redistribution outside this repository's test suite is
ever needed.

## Format / count summary

The corpus now grows continuously (see CONTRIBUTING.md's "Adding a corpus
specimen"), so a hardcoded count here would go stale immediately. Check
the live count instead:

```powershell
Get-ChildItem samples -Recurse -File -Include *.jpg,*.jpeg,*.png,*.webp,*.gif |
    Group-Object { Split-Path (Split-Path $_.FullName -Parent) -Leaf } |
    Select-Object Name, Count
```
