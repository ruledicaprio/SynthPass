# samples/

Identity-document specimen images used as test/example fixtures for the OCR,
LLM, and pipeline crates. Files are organized by document kind into
subdirectories and follow a single filename convention (see "Naming
convention" below).

**Only `ocr_fixtures/` is tracked in git.** It's the hand-verified,
ground-truth-labelled subset, and required CI (`native_ocr_e2e`,
`rust_ocr_smoke`) depends on specific files in it. Everything else —
`passports/`, `id_cards/`, `driving_licenses/`, `misc/` — is a gitignored
local corpus: git keeps every committed byte forever, and a corpus this size
mirrored by hand across machines doesn't need git for that. Populate it by
running `tools/fetch_commons_mrz_specimens_v2.py` or by copying an existing
`samples/` tree from another machine.

**Images are not tracked on `main`.** They live on the orphan `samples-data`
branch — run `./scripts/sync-samples.ps1` after a fresh clone (or any time
you want the latest corpus) to populate this directory locally; it's
`.gitignore`d otherwise. `samples/README.md` and `samples/ocr_fixtures/*.json`
/ `*.md` (the hand-verified OCR ground truth) are the only things under
`samples/` still tracked here. See CONTRIBUTING.md's "Adding a corpus
specimen" section and `knowledge/benchmarks/README.md`'s "local bench loop"
for how the corpus grows now.

## Layout

```
samples/
  passports/          passport specimen images (TD3 MRZ format) — gitignored, local mirror
  id_cards/            national identity card images (TD1 / TD2 MRZ format) — gitignored, local mirror
  driving_licenses/     driving licence specimen images (no MRZ) — gitignored, local mirror
  ocr_fixtures/         docling OCR test fixtures, each an image + .md + .json triple — tracked, required by CI
  misc/                 unclassifiable specimens (e.g. border-pass documents, wiki reference image) — gitignored, local mirror
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
| `ocr_fixtures/`       | Mixed (passport/ID pages used for docling OCR ground-truth) | Varies; see each fixture's `.md`/`.json` |

## Naming convention

Filenames are lookup keys: the `find_sample` / `walk_samples` test helpers
walk `samples/` recursively and match by **basename**, and
`crates/synthpass-ocr/examples/mrz_corpus.rs`'s `CORPUS`/`NEGATIVE` lists
name specimens by basename too. Every file — tracked or gitignored — follows:

```
Country_DocType_Specimen[_YYYY][_N][_side]_mrz.ext
```

- `Country` — issuing country/territory, `Snake_Case` or `Title_Case`
  (e.g. `Croatia`, `North_Macedonia`). Adjective forms survive where already
  established (`Slovenian_ID_…`).
- `DocType` — `Passport` or `ID`, matching the subdirectory.
- `Specimen` — always present; these are specimen/void documents, never real.
- `YYYY` (optional) — issue year, when known from the document.
- `N` (optional) — disambiguator when the same country/year has more than one
  specimen (`_2`, `_3`, …).
- `side` (optional) — `front`, `back`, or `inner_page` for a multi-page/side
  document; omit for a single-side passport datapage.
- `_mrz` / `_no_mrz` — tags whether this image carries the MRZ. `_redacted_mrz`
  when the MRZ is physically blacked out on the source. This tag keys the
  `integrity_survey.rs --mrz-only` filter (`filename.contains("_mrz")`), so it
  is not optional.
- `ext` — prefer `jpg`/`png`; `webp` only when that is the source format.

Basenames must stay **unique across all of `samples/`**, regardless of
subdirectory, since lookups are basename-based and do not disambiguate by
directory.

Renaming a tracked `ocr_fixtures/` file means updating its `CORPUS`/`NEGATIVE`
entry and any `find_sample("…")` literal in the same change — `grep -r` the
old basename first.

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
