# samples/

Identity-document specimen images used as test/example fixtures for the OCR,
LLM, and pipeline crates. Files are organized by document kind into
subdirectories and follow a filename convention (see "Naming convention"
below), with the per-file metadata recorded in `corpus.jsonl`.

**Images are not tracked on `main`.** They live on the orphan `samples-data`
branch — run `./scripts/sync-samples.ps1` after a fresh clone (or any time
you want the latest corpus) to populate this directory locally; it's
`.gitignore`d otherwise. A small set of fixture-derived passport images under `passports/` is force-added because required CI (`native_ocr_e2e`, `rust_ocr_smoke`) does not sync the corpus first.

Tracked under `samples/` on `main`: this README, `corpus.jsonl` (the manifest),
`ocr_fixtures/*.json` + `*.md` (the hand-verified ground truth),
`ocr_fixtures/derived/*.json` + `*.md` (generated parity candidates), and the required fixture-derived passport images. `scripts/sync-samples.ps1` excludes `*.json`/`*.md` from **every** mirrored directory, so ground truth cannot drift onto `samples-data` even when it sits next to its image. See CONTRIBUTING.md's "Adding a corpus specimen"
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

## Validated ground-truth review

The Rust `ground-truth` tool shares the benchmark's MRZ parser and fixture schema. Batch A is
ADR-0011's 13 remaining frozen misses; Batch B is every JSON candidate in `ocr_fixtures/derived/`.
The default `all` combines them, with one card per stem. Images can live in another checkout:

```powershell
cargo run --release -p synthpass-bench --bin ground-truth -- review --batch all --samples-root D:/Projects/SynthPass/samples --out artifacts/ground-truth-review.html
```

Open the HTML locally, compare each field and each printed line with the image, then select
`verified`, `non_conforming` (faithfully transcribed, but printed check digits fail), or `skip`.
Names must use the MRZ form, with filler read as spaces, rather than the visual-zone spelling.
Every card starts at `skip`; **OCR read — unverified** and candidate fields are suggestions.
Batch A pre-fills only an exact structural OCR parse, otherwise its fields stay empty alongside
the OCR lines. `--no-ocr` leaves Batch A empty and omits crops. Missing local OCR models also
fall back to full images; model lookup checks `SYNTHPASS_OCR_MODEL_DIR`, the current directory,
and the parent of `--samples-root`. Nothing downloads models.

Click **Export** and save `ground-truth-verified.json` under `artifacts/`, then validate:

```powershell
cargo run --release -p synthpass-bench --bin ground-truth -- apply artifacts/ground-truth-verified.json
cargo run --release -p synthpass-bench --bin ground-truth -- apply artifacts/ground-truth-verified.json --write
cargo run --release -p synthpass-ocr --example corpus_manifest
cargo test -p synthpass-bench --test ocr_fixtures
```

`apply` defaults to dry run and exits nonzero if any entry is rejected. With `--write`, accepted
entries are promoted individually; rejected entries remain untouched. Every verified field must
match the exact printed MRZ parse; repairs, bad widths and name mismatches are rejected without
correction. A non-conforming zone must fail validation, and structurally parseable names must
still match its printed name field. If structural parsing fails, the tool reports that the names
remain a human assertion. It reports the remaining classification requirements: matching fixture
stem, `mrz_checksums_valid: false`, manifest `mrz.present: true` and `mrz.redacted: false`, followed
by manifest regeneration, attribution review and a separate gate re-bless.

New JSON uses the reviewed alphabetical key order and final newline. Existing reviewed content
must be identical to be accepted. The `.md` preserves existing OCR input from the reviewed or
candidate sidecar; a new Batch A sidecar contains only the transcribed MRZ, without invented VIZ
text. Successful promotion removes the candidate pair from `derived/`.

`--fixtures-dir` defaults to `samples/ocr_fixtures`. Review uses the manifest beside this
fixture directory when present, otherwise the image root's manifest. `corpus_manifest` reads its
own checkout's `samples/` and fixtures: when images and labels are in different worktrees, bring
the reviewed fixture changes into the image checkout before running that command there. Apply
never edits the manifest. For a write rehearsal, copy fixtures into a temporary directory and
pass that directory with `--fixtures-dir`.

Specimen images are PII-adjacent. Review HTML is allowed only under this worktree's gitignored
`artifacts/`, including when `--out` is supplied. Keep exports there too; neither belongs in Git.
The page embeds its images, styles and JavaScript, and makes no network requests.

## Layout

```
samples/
  passports/          passport specimen images (TD3 MRZ format) — gitignored, local mirror
  id_cards/            national identity card images (TD1 / TD2 MRZ format) — gitignored, local mirror
  driving_licenses/     driving licence specimen images (no MRZ) — gitignored, local mirror
  ocr_fixtures/         hand-verified OCR ground truth (.md + .json) — tracked; no benchmark images
    derived/            generated, unreviewed parity candidates (.md + .json) — tracked
  misc/                 unclassifiable specimens (e.g. border-pass documents, wiki reference image) — gitignored, local mirror
  covers/                cover-only images (no data page), any document type — gitignored, local mirror; outside the default benchmark walk, opt-in via --include-covers
  local/                specimens usable locally whose source does not allow redistribution — gitignored, never pushed
  corpus.jsonl          one row per image: MRZ document code, issuing state, provenance, origin, ground-truth link — tracked
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
| `covers/`             | Mixed (cover-only images, any document type)       | No MRZ — always tagged `no_mrz` |

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
  off the directory, never the filename. `covers/` is the one exception: it
  holds cover-only images of any `DocType`, so the directory itself is the
  class (`Cover`), and the `DocType` token in the name still says what the
  cover shows.
- `Specimen` / `Private` — provenance.
- `Code` — MRZ line 1 positions 1–2, the ICAO document code, with `0` standing
  in for the filler `<` (a path cannot hold `<`). So `P0` is `P<`, `PP` is
  `PP`. **`PO` with a letter O is a different code** and genuinely occurs.
- `State` — MRZ line 1 positions 3–5, the issuing state, same `0`→`<`
  substitution. Germany's legacy `D<<` is written `D00`.
- `YYYY` — the year. Which year it means is recorded per file in the manifest's
  `year.kind`, because it is the issue year in most names and the expiry year
  in at least one.
- `_redacted` — the publisher removed holder data on the source. Two forms: `_redacted_mrz` when a
  zone is still there but blacked out or blurred (the reader must refuse it), and `_redacted_no_mrz`
  when the redaction removed the zone entirely, so no MRZ is present at all (decided 2026-09-16;
  a document with no zone by design is plain `_no_mrz`).
- `_mrz` / `_no_mrz` — whether this image carries an MRZ at all. Not optional:
  it keys `integrity_survey.rs --mrz-only`, which skips names containing
  `no_mrz` (a *negative* filter — an image with no `_mrz` tag at all is kept).
- `variant…` — free-form tags for how the image is degraded or framed:
  `blur`, `rotated`, `highlight`, `contrast`, `pixelated`, `wide`, `child`,
  `emergency`, `inner_page`, and `cover` for a cover-only image (always with
  `no_mrz`; filed under `covers/` regardless of document type, and a class
  that never counts toward coverage, see
  [`ADR-0012`](../knowledge/decisions/ADR-0012-cover-only-specimens-are-a-labelled-class.md)
  and its amendment).
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

### Where each image came from: `origin`

Every row carries an `origin` object:

| Field | Holds |
|---|---|
| `url` | the http(s) URL the image was fetched from |
| `page` | the page that published it, when that is a different URL |
| `licence` | the class the publishing page states: `public-domain`, `cc-by`, `cc-by-sa`, `gov-published`, or `none-stated` when it says nothing |
| `fetched` | the fetch date, `YYYY-MM-DD` |
| `found_by` | `human`, `commons-fetcher`, `dsh` or `firecrawl` |

It is recorded when the image is fetched, because afterwards it cannot be. Rows
that predate the field read `unrecorded` in `found_by` and `licence` with the
other three `null`, and are never back-filled from memory — a reconstructed URL
is a guess. The generator carries `origin` across regeneration the way it
carries `notes`, and `crates/synthpass-bench/tests/corpus_manifest.rs` rejects
an origin recorded only in part.

The same test refuses an image recorded twice under two names (identical
`sha256`): every walk of `samples/` would read it twice and count it twice.

## Provenance

Images are public specimen / illustrative document images collected from
Wikipedia's "Passports by country" category and related identity-document
categories (national ID cards, driving licences), and — for newer additions —
from issuing authorities' own publications; the allowed sources are listed in
[`knowledge/SPECIMEN_SOURCES.md`](../knowledge/SPECIMEN_SOURCES.md). They depict specimen or
void documents published for public reference, not real issued documents
belonging to private individuals, unless explicitly marked otherwise (e.g.
filenames containing `_private`).

## Local-only track

`samples/local/` holds specimens that are fine to use on this machine but whose
source does not allow redistribution. It is gitignored; `scripts/sync-samples.ps1`
and the manifest generator both work from a fixed directory list that leaves it
out; and `provider-bench --real-specimens` skips it unless `--include-local` is
given, a flag it refuses to combine with `--write-baseline` or
`--assert-baseline`. So the committed baseline and every CI run measure the
public corpus only, and a local-track report never becomes a CI artifact.

## Covers track

`samples/covers/` holds cover-only images — a rendering of a passport's cover
or a card's face, never a data page — of any document type, always tagged
`no_mrz` plus the `cover` variant token
([`ADR-0012`](../knowledge/decisions/ADR-0012-cover-only-specimens-are-a-labelled-class.md)
and its amendment). Unlike `samples/local/`, it is mirrored to `samples-data`
like the rest of the public corpus: a cover carries the same provenance and
licence review as any other specimen.

It is **outside the default real-specimen walk**, opt-in via
`provider-bench --real-specimens --include-covers` (a flag it refuses to
combine with `--write-baseline` or `--assert-baseline`, the same as
`--include-local`). The reason is runtime, not licensing: the real-specimen
gate OCRs every image it walks (~10 seconds each), a cover never carries an
MRZ and so never enters the scored denominator, and a hundred covers would
add real minutes to every PR's gate for no accuracy information. A cover's
only benchmark value is the hallucination check — a checksum-valid MRZ read
off one is `false_positive_mrz` — which does not need to run on every PR.
Longer term the directory is a verified validation set for issuer/document-type
recognition (the VIZ facet). **Never a coverage claim**: a cover never changes
a code's status in [`CORPUS_COVERAGE.md`](../knowledge/CORPUS_COVERAGE.md).

## Licensing / usage

These images are used here strictly for specimen/illustrative purposes —
as test fixtures for MRZ/OCR parsing and layout detection. Refer to the
original Wikipedia/Wikimedia Commons file pages — or, for a row with a
recorded `origin`, its `page` and `licence` — for the specific licence of
each image if redistribution outside this repository's test suite is ever
needed.

## Format / count summary

The corpus now grows continuously (see CONTRIBUTING.md's "Adding a corpus
specimen"), so a hardcoded count here would go stale immediately. Check
the live count instead:

```powershell
Get-ChildItem samples -Recurse -File -Include *.jpg,*.jpeg,*.png,*.webp,*.gif |
    Group-Object { Split-Path (Split-Path $_.FullName -Parent) -Leaf } |
    Select-Object Name, Count
```
