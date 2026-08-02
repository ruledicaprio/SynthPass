# samples/

Identity-document specimen images used as test/example fixtures for the OCR,
LLM, and pipeline crates. Files are organized by document kind into
subdirectories; **no file has been renamed** when moved between them (see
"Naming convention" below for why).

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

## Naming convention for FUTURE additions

Existing filenames are used as lookup keys by several tests (`find_sample`
/ `walk_samples` helpers walk `samples/` recursively and match by
**basename**), so **existing files are intentionally left un-renamed** when
moved into subdirectories — renaming any of them would break those test
references. For the gitignored portion of the corpus this is no longer
something CI can check (nothing there is tracked), so treat it as an
honor-system convention enforced by the fetch script's own naming, plus
whatever discipline you bring to a hand-maintained local mirror.

New files added going forward should follow:

```
Country_DocType_Detail.ext
```

- `Country` — the issuing country/territory, `Snake_Case` or `Title_Case`
  matching existing entries (e.g. `Croatia`, `North_Macedonia`).
- `DocType` — one of `Passport`, `ID_Card`, `Driving_License`, matching the
  subdirectory the file will be placed in.
- `Detail` (optional) — page/variant qualifier, e.g. `Specimen`,
  `Specimen_2`, `Biodata`, `Front`, `Back`, `Cover`.
- `ext` — prefer `jpg`/`png`; avoid `webp`/`gif` unless that is the only
  source format available.

Basenames must stay **unique across all of `samples/`**, regardless of
which subdirectory they end up in, since lookups are basename-based and do
not disambiguate by directory.

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
