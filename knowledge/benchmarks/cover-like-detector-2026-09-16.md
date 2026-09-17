# No image in the default real-specimen walk is cover-like

**Date:** 2026-09-16 · **MAIN:** `0b879ee` · **DATA:** `33001da4bbf7942d594695b3167fd47f41b0fdf0` · **Evidence:** unlabelled (pre-standard) · **Status:** superseded by [README.md#current-headline-numbers](README.md#current-headline-numbers)

**Superseded by:** the live headline in
[`README.md#current-headline-numbers`](README.md#current-headline-numbers).

**2026-09-16.** Task T15 of the specimen-acquisition loop, asked before any file is moved:
which images in the default `provider-bench --real-specimens` walk (`samples/passports`,
`id_cards`, `driving_licenses`, `misc` — not `samples/covers`, `local` or `private`) are
*covers*, and should therefore be relocated to the labelled cover class that
[ADR-0012](../decisions/ADR-0012-cover-only-specimens-are-a-labelled-class.md)'s 2026-09-15
amendment put outside that walk. Read-only: no code, no corpus file, no baseline was touched,
and no benchmark was run against the baseline.

**The answer is none.** The plan's proposed rule — *"any `no_mrz_expected` image with almost no
OCR text across three baselines is listed for relocation"* — was applied to all **51**
`no_mrz_expected` documents and produced **22 candidates** below the control's text volume and
**zero covers**. Every candidate it selects is a data face: an ID-card front, a card back, or a
passport bio-data page. The rule selects images `ocrs` reads *badly*, not images that carry
little to read, and the two populations barely overlap in this corpus.

## What the sources actually record

| Source | Carries | Answers "how much text"? |
| :-- | :-- | :-- |
| [`real-specimen-mrz-baseline.json`](real-specimen-mrz-baseline.json) | bucket counts, `documents`, `scored` | no |
| CI gate report `documents_detail` | `name`, `mrz_found`, `read_ok`, `mrz_checksums_valid`, `miss_reason`, assertion counts, `ocr_ms` | **no** |
| `samples/corpus.jsonl` | `mrz.observed.read_by_ocr` (a boolean), `mrz.observed.*` | no, and it is a dated read |
| `provider-bench --dump-ocr` | full OCR text — but only for `checksum_failed` and `no_mrz_found` | not for this population |

**No artifact in the repo records OCR text length for a `no_mrz_expected` document.**
`--dump-ocr`'s two miss kinds are both scored kinds; `no_mrz_expected` is off the denominator and
never reaches that branch (`crates/synthpass-bench/src/provider_bench.rs`, the `dump_miss_kind`
match). `ocr_ms` is the only text-adjacent field the reports carry, and it is dominated by image
size and by how many retry passes a document costs, not by how much text is on it — France
`ID_2020_front_no_mrz` is the slowest document in the corpus at 75.1 s and is also one of the
*most* text-dense. So the measurement was taken locally, with the one instrument that reports it.

## Method

1. **Population.** The 51 documents CI classifies `no_mrz_expected`, taken from the per-document
   `documents_detail` of two CI `write-baseline` runs (c13 `35002739414`, c14 `35049943310`).
2. **Stability across runs.** Both runs classify **266 of 266** documents identically — every
   `miss_reason`, not just the totals. A third run, the `cohort-c14` PR assert
   (`35053007483`, green), reproduces the same histogram at bucket level against the committed
   baseline. `no_mrz_expected = 51` in all three. The plan's "across three baselines" condition is
   therefore satisfied for every candidate below, and it discriminates nothing: this bucket is
   decided by the filename's `no_mrz` token before OCR runs, so it cannot flap between runs.
3. **Text volume, measured locally.** `dump_variants` prints `page.text.len()` — the character
   count of the OCR text the pipeline actually consumed, after `recognize_detailed`'s rotation,
   deskew and retry chain. Run on all 51, one process, synchronously.
4. **Byte-identical inputs.** The local `samples/` checkout is stale (5 of the 51 files are absent
   from it), so every image was extracted from `origin/samples-data` at
   `33001da4bbf7942d594695b3167fd47f41b0fdf0` — the exact `samples_data_sha` the committed
   baseline names — and each file's SHA-256 was checked against `samples/corpus.jsonl`. **53/53
   match** (53, not 51: see *Also found*).
5. **Adjudication from the image.** Every candidate under the threshold was opened and classified
   by what the image shows, not by what OCR returned. This is the rung order
   [the 2026-09-09 denominator correction](denominator-correction-2026-09-09.md) established:
   ask what the document is before asking what the run achieved.

### The threshold, and why it is anchored rather than chosen

The distribution has **no gap to cut at**: 23 → 2216 characters, median 537, continuous
throughout. There is no bimodality, because there is no cover class hiding in it. Any absolute
cut would be arbitrary, so the screen is anchored on the control instead:

> **Screen: page OCR text < 501 characters** — strictly below
> `Kenya_Passport_Specimen_P0_KEN_XXXX_no_mrz.jpg`, the document the task names as the case a
> cover detector must *not* select. **22 of 51 files** fall below it.

A stricter reading of "almost no OCR text" (< 360 characters, the last point before the run of
382s) selects **13 files**. The verdict is identical either way — 0 covers — so the finding does
not rest on where the line is drawn, which is the only reason a threshold with no natural break
behind it is worth publishing at all.

## The 51, by measured OCR text volume

`miss_reason` is `no_mrz_expected` for every row in all three runs; the column is omitted rather
than repeated 51 times. "Band" is `mrz_band_score`, `—` when no band was scored.

| File | Dir | OCR chars | Band |
| :-- | :-- | --: | --: |
| `Bosnia_Herzegovina_Driving_License_Specimen_front.gif` | driving_licenses | 23 | — |
| `Switzerland_ID_Specimen_2003_front_no_mrz.jpg` | id_cards | 49 | — |
| `Bosnia_Herzegovina_Driving_License_Specimen_face.gif` | driving_licenses | 120 | — |
| `United_Arab_Emirates_ID_Specimen_XXXX_front_no_mrz.jpg` | id_cards | 191 | — |
| `Monaco_ID_Specimen_XXXX_front_no_mrz.png` | id_cards | 209 | 0.18 |
| `South_Africa_ID_Specimen_2013_back_no_mrz.png` | id_cards | 228 | — |
| `South_Africa_ID_Specimen_2013_front_no_mrz.png` | id_cards | 262 | — |
| `United_Arab_Emirates_Passport_Specimen_P0_ARE_2018_no_mrz.png` | passports | 288 | — |
| `Croatia_ID_Specimen_2002_front_no_mrz.jpg` | id_cards | 310 | 0.44 |
| `Croatia_ID_Specimen_2021_2_front_no_mrz.jpg` | id_cards | 312 | — |
| `Cambodia_Passport_Specimen_PN_KHM_XXXX_no_mrz.jpg` | passports | 323 | — |
| `Serbia_ID_Specimen_2008_face_no_mrz.png` | id_cards | 327 | — |
| `Russian_Federation_ID_Specimen_2013_front_no_mrz.jpeg` | id_cards | 350 | — |
| `Czechia_Passport_Specimen_P0_CZE_2005_no_mrz.jpg` | passports | 358 | 0.26 |
| `Bosnia_and_Herzegovina_ID_Specimen_2013_front_no_mrz.jpg` | id_cards | 382 | 0.62 |
| `Philippines_ID_Specimen_XXXX_front_no_mrz.png` | id_cards | 382 | — |
| `Turkiye_ID_Specimen_2026_residence_permit_front_no_mrz.webp` | id_cards | 414 | — |
| `Dominica_ID_Specimen_2024_front_no_mrz.png` | id_cards | 432 | 0.32 |
| `Croatia_ID_Specimen_2021_front_no_mrz.jpg` | id_cards | 450 | 0.20 |
| `Liechtenstein_ID_Specimen_front_no_mrz.jpg` | id_cards | 455 | — |
| `Slovenia_ID_Specimen_2022_front_no_mrz.jpg` | id_cards | 484 | — |
| `Luxembourg_ID_Specimen_front_no_mrz.jpg` | id_cards | 485 | — |
| `Kenya_Passport_Specimen_P0_KEN_XXXX_no_mrz.jpg` | passports | 501 | 0.45 |
| `Belgium_ID_Specimen_2021_front_no_mrz.png` | id_cards | 508 | — |
| `Sweden_ID_Specimen_2027_front_no_mrz.png` | id_cards | 510 | — |
| `Turkiye_ID_Specimen_2023_front_no_mrz.jpg` | id_cards | 537 | 0.61 |
| `Turkiye_ID_Specimen_2020_front_no_mrz.jpg` | id_cards | 539 | 0.62 |
| `Austria_ID_Specimen_2021_front_no_mrz.png` | id_cards | 547 | — |
| `Croatia_BorderPass_Specimen_CB_HRV_2025_front_no_mrz.png` | misc | 550 | 0.40 |
| `Turkiye_ID_Specimen_2025_front_no_mrz.png` | id_cards | 555 | 0.65 |
| `Belgium_ID_Specimen_front_no_mrz.jpg` | id_cards | 560 | — |
| `Poland_ID_Specimen_2021_front_no_mrz.jpg` | id_cards | 571 | 0.16 |
| `Netherlands_ID_Specimen_2014_front_no_mrz.jpg` | id_cards | 582 | 0.33 |
| `Bulgaria_ID_Specimen_2024_front_no_mrz.png` | id_cards | 595 | 0.31 |
| `Vietnam_Passport_Specimen_P0_VNM_2022_no_mrz.jpg` | passports | 596 | 0.29 |
| `Germany_ID_Specimen_2021_front_no_mrz.jpg` | id_cards | 621 | — |
| `Poland_ID_Specimen_2015_front_no_mrz.jpg` | id_cards | 733 | 0.31 |
| `Germany_ID_Specimen_2024_front_no_mrz.jpg` | id_cards | 751 | 0.34 |
| `Norway_ID_Specimen_2021_front_no_mrz.png` | id_cards | 754 | 0.23 |
| `Latvia_ID_Specimen_2021_front_no_mrz.png` | id_cards | 770 | 0.25 |
| `Italy_ID_Specimen_2022_front_no_mrz.jpg` | id_cards | 912 | 0.32 |
| `Algeria_Passport_Specimen_XX_XXX_XXXX_no_mrz.jpg` | passports | 992 | 0.43 |
| `Slovakia_Passport_Specimen_no_mrz.webp` | passports | 1030 | 0.50 |
| `Bangladesh_Passport_Specimen_P0_BGD_XXXX_no_mrz.jpg` | passports | 1163 | 0.34 |
| `Bosnia_and_Herzegovina_ID_Specimen_front_no_mrz.jpg` | id_cards | 1261 | 0.38 |
| `Turkiye_ID_Specimen_2020_front_no_mrz.webp` | id_cards | 1267 | 0.80 |
| `Switzerland_ID_Specimen_2023_front_no_mrz.jpg` | id_cards | 1278 | 0.73 |
| `Ireland_ID_Specimen_2015_front_no_mrz.jpg` | id_cards | 1907 | 0.72 |
| `Netherlands_Driving_License_Specimen_no_mrz.jpg` | driving_licenses | 2141 | 0.77 |
| `France_ID_Specimen_2020_front_no_mrz.png` | id_cards | 2151 | 0.43 |
| `Portugal_ID_Specimen_2024_front_no_mrz.jpeg` | id_cards | 2216 | 0.23 |

The first 22 rows are the screen's candidates; the Kenya control is row 23, and everything below
it is out of scope by construction.

## Every candidate, adjudicated

All 22 were opened. Not one is a cover, and the reason each is not is a property of the image.

| # | Candidate | OCR chars | What the image is | Call |
| --: | :-- | --: | :-- | :-- |
| 1 | `Bosnia_Herzegovina_Driving_License_Specimen_front.gif` | 23 | the licence **back**: the 12-column category table, serial `1234567890123`, ~40 short strings, at 285 × 170 px | not a cover |
| 2 | `Switzerland_ID_Specimen_2003_front_no_mrz.jpg` | 49 | card face: portrait, five-language header, `30002568`, name, signature, at ~230 × 145 px | not a cover |
| 3 | `Bosnia_Herzegovina_Driving_License_Specimen_face.gif` | 120 | licence face: portrait, `KOVAČEVIĆ`, `11.02.1985`, `TT8300155`, SPECIMEN overprint | not a cover |
| 4 | `United_Arab_Emirates_ID_Specimen_XXXX_front_no_mrz.jpg` | 191 | ID-card face: chip, `784-1979-1234567-1`, name in Arabic and Latin | not a cover |
| 5 | `Monaco_ID_Specimen_XXXX_front_no_mrz.png` | 209 | card face: `029067`, `01.01.2999`, three `SPECIMEN` lines | not a cover |
| 6 | `South_Africa_ID_Specimen_2013_back_no_mrz.png` | 228 | card back: conditions paragraph, `123456789`, 1-D and 2-D barcodes | not a cover |
| 7 | `South_Africa_ID_Specimen_2013_front_no_mrz.png` | 262 | card face: `ABCDEFGHIJKLMNOPQRSTUVXZY`, `0123456789012`, `16 JUL 1969` | not a cover |
| 8 | `United_Arab_Emirates_Passport_Specimen_P0_ARE_2018_no_mrz.png` | 288 | **passport bio-data spread, every field under a black bar, including a full-width bar across the MRZ strip** | not a cover — **mislabelled**, see below |
| 9 | `Croatia_ID_Specimen_2002_front_no_mrz.jpg` | 310 | card face: `SPECIMEN` ×2, `HRV`, `01.01.1977`, `12.12.2002` | not a cover |
| 10 | `Croatia_ID_Specimen_2021_2_front_no_mrz.jpg` | 312 | card face: `FERATOVIĆ ALEN`, `115618331`, `26.02.1993`, photo, no SPECIMEN marking | not a cover — see *Also found* |
| 11 | `Cambodia_Passport_Specimen_PN_KHM_XXXX_no_mrz.jpg` | 323 | bio page: `PN` / `KHM` / `CAMBODIAN` legible, every personal field and the MRZ band blurred | not a cover |
| 12 | `Serbia_ID_Specimen_2008_face_no_mrz.png` | 327 | card face: `ТЕСТ` / `МИЛИЦА`, `955555546`, `SPECIMEN` | not a cover |
| 13 | `Russian_Federation_ID_Specimen_2013_front_no_mrz.jpeg` | 350 | card face: `АЛЕКСАНДРОВА`, `02 41 456789`, place of birth | not a cover |
| 14 | `Czechia_Passport_Specimen_P0_CZE_2005_no_mrz.jpg` | 358 | bio page: `SPECIMEN` / `VZOR`, `0000000`, cropped above the MRZ strip | not a cover |
| 15 | `Bosnia_and_Herzegovina_ID_Specimen_2013_front_no_mrz.jpg` | 382 | card face: `KOVAČEVIĆ`, `TT1300005`, `SPECIMEN` | not a cover |
| 16 | `Philippines_ID_Specimen_XXXX_front_no_mrz.png` | 382 | card face: `DELA CRUZ`, `JUAN MIGUEL FERNANDO`, `JANUARY 01, 1980` | not a cover |
| 17 | `Turkiye_ID_Specimen_2026_residence_permit_front_no_mrz.webp` | 414 | residence-permit face: `ÖRNEK` / `ÖRNEKOĞLU`, `00000000000`, `A00A00000` | not a cover |
| 18 | `Dominica_ID_Specimen_2024_front_no_mrz.png` | 432 | *Dominican Republic* cédula face: `SANTO DOMINGO, R.D.`, fields partly blurred | not a cover |
| 19 | `Croatia_ID_Specimen_2021_front_no_mrz.jpg` | 450 | card face: `SPECIMEN` ×2, `123456789`, `234606` | not a cover |
| 20 | `Liechtenstein_ID_Specimen_front_no_mrz.jpg` | 455 | card face: `OSPELT-BECK`, `ID98754015`, `12.05.1982`, `SPECIMEN` | not a cover |
| 21 | `Slovenia_ID_Specimen_2022_front_no_mrz.jpg` | 484 | card face: `VZOREC JANA`, `IE9876543`, `2806985505145` | not a cover |
| 22 | `Luxembourg_ID_Specimen_front_no_mrz.jpg` | 485 | photograph of a *carte de légitimation*: name, expiry and position redacted, eyes barred | not a cover |

The eight `passports/` rows are the population an intuitive reading would have relocated first —
ADR-0012's Context notes that "seven `passports/` rows today are `no_mrz` images". All eight were
opened: Algeria, Bangladesh, Cambodia, Czechia, Kenya, Slovakia, UAE 2018 and Vietnam 2022 are
**bio-data pages** — blank, cropped, redacted or blurred — and not one is a cover of a passport.
Relocating them would move eight data pages into a class defined as "no data page and no MRZ".

## The Kenya control

`Kenya_Passport_Specimen_P0_KEN_XXXX_no_mrz.jpg` measures **501 OCR characters**, rank 23 of 51 —
the middle of the distribution, above every candidate the screen selects. The image is a two-page
spread: a *DESCRIPTION / MAELEZO* page (`NAIROBI WEST`, height `1.83 m`, `BROWN`, `NIL`) above a
bio-data page (`REPUBLIC OF KENYA`, `P`, `KEN`, `KENYAN`, `EMBAKASI`) whose personal fields have
been erased and whose MRZ strip is not on the image. It is a **redacted bio spread, not a cover**,
and the method does not list it, for three independent reasons:

1. **Text volume.** Erasing the *values* leaves the labels, the headings and the boilerplate. A
   redacted data page still prints a data page's worth of text — which is exactly why redaction
   does not look like a cover to this instrument.
2. **The image.** It shows named fields, a field grid and a passport header; a cover shows a coat
   of arms, a country name and a document type.
3. **The role.** Its `dir` is `passports/` and its filename carries no `cover` token — per
   ADR-0012 a cover's label is the token, and nothing else.

The failure mode the control guards against is real, and this corpus contains it: a *fully* black
redaction bar produces almost no OCR text, which is the 2026-09-09 finding that **the cleaner the
redaction, the worse the document scores**. Candidate 8, the UAE 2018 spread, is precisely that
document — 288 characters, every field barred — and a text-volume rule relocates it into the cover
class, where it would sit permanently outside the corpus, unlabelled as redacted, and invisible to
the denominator work that owns it.

## Why the proposed rule does not work

The two lowest-text images in the whole population are a driving-licence *category table* (23
characters) and a Swiss ID *card face* (49) — two of the most information-dense small images in
the set. Both are about 250 px wide. What the screen ranks is **how legible the scan is**, a
property of the file; the cover question asks **what the document is**, a property of the
document. That is the same class of bug the 2026-09-09 correction reversed for redaction and
non-conformance, and the same one the
[2026-09-13 manifest review](manifest-review-no-mrz-found-2026-09-13.md) found in the Netherlands
licence's `mrz.present`: *a property of the document decided by what the run returned*. A cover
detector built on OCR volume would have relocated 22 data faces, and would have missed a cover
that happened to arrive as a clean high-resolution scan with a country name printed across it.

**The instrument that answers this question is the image, and the record of the answer is the
filename token.** Both already exist: ADR-0012 puts the `cover` token next to `no_mrz`, and the
scout worker reports `side: cover` at acquisition time. Nothing needs to be detected after the
fact for material arriving through the loop; this report concerns the 51 documents that predate
it.

## What relocation would do to both denominators

Covers leave `documents` and never touch `scored`, so the arithmetic is one-sided. Against the
committed baseline (CI sha `0b879ee`, measured 2026-09-16, `samples_data_sha` `33001da`):

| | now (measured) | relocate the 0 covers found | relocate all 8 `passports/` `no_mrz` (modelled, **wrong**) |
| :-- | --: | --: | --: |
| `documents` | 266 | 266 | 258 |
| `scored` | 159 | 159 | 159 |
| Tier-1 HIT | 145 | 145 | 145 |
| hit rate, scored | **91.2%** | 91.2% | 91.2% |
| hit rate, whole corpus | **54.5%** | 54.5% | **56.2%** |
| `no_mrz_expected` | 51 | 51 | 43 |

The third column is the one to keep in view: relocating documents out of the walk raises the
corpus-wide rate by 1.7 pp without reading a single extra document. That rate exists to answer
"what happens to my drawer of documents", and a drawer containing a redacted UAE bio page still
yields nothing from it. Moving a *cover* out is honest, because a cover is not a document anyone
points a reader at; moving a data page out is the metric flattering itself. Both denominators are
quoted here for the reason [`README.md`](README.md#current-headline-numbers) publishes both.

CI cost is the other side of this, and it is what ADR-0012's amendment was about. These 51
documents cost the gate **11 min 16 s** of OCR per run (sum of `ocr_ms`, run `35049943310`,
against 44 min 50 s for all 266), **25.1%** of the `mrz` provider's total OCR time — spent on
documents that can never move the hit rate. None of it is recoverable by relocation, because none
of these documents is a cover: they are legitimate negative controls, and four of them (France,
Italy, Sweden 2027, the Netherlands licence) are documents the denominator work specifically
relies on.

## Also found

Three things the question did not ask, all off the same two runs and the same images.

**1. Two `no_mrz` passport pages do have an MRZ, and one of them is a live gate hazard.**
`United_Arab_Emirates_Passport_Specimen_P0_ARE_2018_no_mrz.png` carries a full-width black
redaction bar where the zone is, and `Vietnam_Passport_Specimen_P0_VNM_2022_no_mrz.jpg` shows
**two visibly printed MRZ lines** — the `P<` prefix, long `<<<<` filler runs and a trailing `06`
are all legible, and only the data segments are blurred. Both are `redacted_mrz` documents wearing
a `_no_mrz` name. Neither moves `scored` (both buckets are off the denominator), but the rung they
land on differs in a way that matters:

```rust
// crates/synthpass-bench/src/provider_bench.rs
let miss_reason = if !bench_page.mrz_expected {
    if bench_page.mrz_found && reading.evidence.mrz_checksums_valid {
        Some(MissReason::FalsePositiveMrz)
```

`!mrz_expected` is the **first** rung, and a checksum-valid read there is `false_positive_mrz`,
which **fails the build**. The `redacted` rung is second and absorbs the same read harmlessly. So
the day OCR improves enough to read through Vietnam's blur, the gate goes red on a correct read of
a correctly-redacted specimen. Renaming the two files to `_redacted_mrz` (Vietnam plausibly
`_redacted_mrz_blur`, the form Malaysia 2019 already uses) moves `no_mrz_expected` 51 → 49 and
`redacted_mrz` 37 → 39 — both warn-only buckets, so it owes a re-bless the gate will not ask for.
Cambodia `PN_KHM_XXXX` is the same shape but was reviewed deliberately: its `notes` field records
the H5 admission and states the MRZ band itself is blurred out. That makes it a convention
question rather than a defect — this corpus currently spells "redacted to the point of having no
zone" two different ways.

**2. Three specimens are in the walk twice, in two image formats.** `corpus.jsonl` holds
`Azerbaijan_Passport_Specimen_PC_AZE_2013_mrz` as `.jpg` **and** `.webp`,
`China_Passport_Specimen_P0_CHN_2012_mrz` as `.png` **and** `.webp`, and
`Turkiye_ID_Specimen_2020_front_no_mrz` as `.jpg` **and** `.webp` — distinct SHA-256s, distinct
`ocr_ms` (Azerbaijan: 7007 ms against 2141 ms), the same document. The Azerbaijan and China pairs
are **HIT in both runs, twice each**, so `tier1_hits` 145 and `scored` 159 each contain two
duplicates: de-duplicated, the scored rate is 143 / 157 = **91.1%** against the published
145 / 159 = 91.2%. Inside anybody's rounding, and worth knowing before someone re-derives it. It
also means the gate report's `documents_detail` **cannot be keyed by `name`**: three stems
collide, and a consumer that builds a dictionary from it silently drops three documents
(266 → 263). Join on index, or on name plus extension.

**3. Provenance observations, recorded and not acted on.**
`Croatia_ID_Specimen_2021_2_front_no_mrz.jpg` shows a named holder (`FERATOVIĆ ALEN`), a card
number and an unobscured photograph, with no SPECIMEN marking anywhere on the card — it reads as a
real person's document rather than a template, and its `origin` is `unrecorded`.
`Luxembourg_ID_Specimen_front_no_mrz.jpg` is a photograph of a real *carte de légitimation* with
the name, expiry and position redacted and the eyes barred. Bangladesh `P0_BGD_XXXX` and Algeria
`XX_XXX_XXXX` both carry unblurred portrait photographs over blank data fields. This is *inferred
from the images alone*; the 2026-09-02 provenance audit is settled and is not re-opened here, and
the H5 standard was revised on 2026-09-14, after these files were ingested. Flagged once, for
whoever owns corpus hygiene.

## What this does not claim

**It does not claim there are no covers in `samples/`.** It claims there are none among the 51
`no_mrz_expected` documents of the default walk. The 37 `redacted_mrz` and 19
`checksum_failed_specimen` documents were not screened — a cover mislabelled `_mrz` or
`_redacted_mrz` would not appear in this population at all. The 34 files already in
`samples/covers/` were not examined either; they are outside the walk by design and outside this
question.

**It is not a measurement of the pipeline.** No code, threshold or model moved, no A/B was run,
and `tier1_hits` is quoted from CI rather than re-measured.

**The OCR character counts are local, and are not comparable to a CI number.** They were produced
by `dump_variants` on this machine while another session's `cargo test --workspace` was running,
so they carry local `rten` float variance and no timing claim is made from them. What they support
is a *ranking* over a hundred-fold range, which that variance cannot invert. The inputs are
byte-identical to CI's (SHA-256 verified against `corpus.jsonl` at `samples_data_sha` `33001da`),
so the ranking is over the same corpus the baseline measured.

**"Three baselines" was satisfied two ways, not three.** Per-document `miss_reason` was compared
across two CI `write-baseline` runs (`35002739414`, `35049943310`), which agree on 266 of 266. The
third run (`35053007483`, the `cohort-c14` PR assert) is evidence at **bucket level only** — its
artifact could not be retrieved from this session, so its agreement is inferred from a green
`--assert-baseline` against a `tolerance: 0` baseline rather than read per document.

**The adjudications are visual calls on single images.** Each names what is visible in the image;
none rests on a fixture, and none of these documents has one.

## Candidates considered and rejected

- **`ocr_ms` from the CI reports as the text-volume proxy.** The only text-adjacent field the gate
  publishes, and it needs no local run. Rejected on the data: it ranks by pixels and retry passes,
  not by text. The most text-dense document in the population (France ID 2020 front, 2151
  characters) is also the slowest at 75.1 s, while the *least* text-dense reading (23 characters)
  costs 9.9 s. A proxy that inverts on its own extremes is not a proxy.
- **`samples/corpus.jsonl`'s `mrz.observed.read_by_ocr`.** Free, already committed, one boolean
  per file. Rejected twice over: it is binary, so it cannot rank; and it is a **dated read**,
  frozen at whatever pipeline last regenerated the manifest (unchanged for pre-existing rows since
  2026-09-03), so it would have answered for a pipeline two orientation fixes old.
- **`integrity_survey --dir <sub> --mrz-only`.** The obvious corpus-walking harness, and it
  records `ocr_ms`, dimensions and `mrz_found` per file. Rejected on two counts: `--mrz-only`
  skips every `no_mrz` filename, which is this entire population, and its JSONL carries no
  text-volume field at all.
- **`visual_zone_survey --file`, whose `total_lines` is a genuine text-volume signal.** The best
  alternative instrument, and it reports noise-line fractions this report could have used.
  Rejected on staleness: the built binary predates its own source by three days and does not
  contain the `--file` flag, so running it would have walked all of `samples/` — 300 images,
  `covers/` included — and written a corpus JSONL. `dump_variants` needed the same rebuild and
  reports characters directly.
- **Relocating the eight `passports/*_no_mrz` files on their directory and token alone.** The
  cheap answer, and the one the framing invites. Rejected on the images: all eight are bio-data
  pages. It would also have raised the published corpus-wide rate by 1.7 pp for free.
- **Proposing a `cover` filename token for any of the 22.** Rejected for the same reason: the
  token asserts what the image *is*, and none of them is a cover. ADR-0012 is explicit that the
  name is the label, which makes a wrong token worse than no token.

## Recommended next step

**Relocate nothing.** There is no cover in the default walk to relocate, and no T14-style corpus
PR is owed for covers.

The two renames in *Also found* item 1 are a different, smaller PR, and worth doing because one of
them disarms a future build failure:

```text
samples/passports/United_Arab_Emirates_Passport_Specimen_P0_ARE_2018_no_mrz.png
  -> United_Arab_Emirates_Passport_Specimen_P0_ARE_2018_redacted_mrz.png
samples/passports/Vietnam_Passport_Specimen_P0_VNM_2022_no_mrz.jpg
  -> Vietnam_Passport_Specimen_P0_VNM_2022_redacted_mrz_blur.jpg
```

Both are `samples-data` renames plus a `corpus.jsonl` regeneration — never a hand-edit of
`mrz.present`, which is re-derived from the filename on every manifest run (the 2026-09-13 review
records why). Modelled effect: `no_mrz_expected` 51 → 49, `redacted_mrz` 37 → 39, `scored` 159 and
`tier1_hits` 145 unchanged, both rates unchanged. **The gate will not ask for the re-bless** —
neither bucket is a `REGRESSION_BUCKETS` member and `documents` does not move — so it has to be
done by the author in the same PR, or a `tolerance: 0` baseline stands over a corpus it no longer
describes.

What would change the conclusion: an image. If anyone opens one of the 22 and sees a cover, that
file's call changes, and the method should be told why it missed it.

## Exact invocations

```bash
# 1. the population, and its per-document stability across the two CI write-baseline
#    artifacts already on disk (the run-artifact download subcommand is not available
#    to this role, so the third run is bucket-level evidence only)
python -c "... documents_detail of %TEMP%/rebless-c13 and %TEMP%/rebless-r5k7etof ..."
#    -> 266/266 identical miss_reason; no_mrz_expected = 51 in both

# 2. byte-identical inputs, read out of the branch the baseline names
#    (read-only; MSYS_NO_PATHCONV=1, via python subprocess)
git show origin/samples-data:samples/<dir>/<file>
#    -> artifacts/cover-like-images/, 53 files, SHA-256 checked against samples/corpus.jsonl

# 3. the instrument (rebuilt first: the installed binary was five days stale)
cargo build -p synthpass-ocr --release --example dump_variants
SYNTHPASS_OCR_DUMP_VARIANTS=artifacts/cover-like-crops \
  ./target/release/examples/dump_variants.exe <paths...>     # 6 batches, 51 files
#    -> artifacts/cover-like-ocrchars.log, artifacts/cover-like-ocrchars.json

# 4. adjudication: each of the 22 candidate images opened and read
```

Reports read, not run: `real-specimen-gate-report.json` from runs `35002739414` (c13
write-baseline, 2026-09-15) and `35049943310` (c14 write-baseline, 2026-09-16 — the run behind the
committed baseline; its artifact's counts are byte-identical to
`real-specimen-mrz-baseline.json`), plus `gh run view 35053007483` for the third, assert-only run.
Source read, not run: `crates/synthpass-bench/src/provider_bench.rs` (the rung order and the
`--dump-ocr` miss-kind filter), `crates/synthpass-ocr/examples/dump_variants.rs`,
`crates/synthpass-ocr/examples/visual_zone_survey.rs`,
`crates/synthpass-ocr/examples/integrity_survey.rs`.