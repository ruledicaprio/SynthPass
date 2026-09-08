# Real-specimen `checksum_failed` misses, categorised — 2026-09-08

`checksum_failed` is the largest real-specimen Tier-1 miss category, and the
[2026-08-16 weak-spot finding](README.md#2026-08-16--first-genuine-real-specimen-tier-1-numbers)
framed it as the clean signal: *"OCR found MRZ-shaped text and got at least one
character wrong inside it."* This note pulls the raw OCR text for every one of
those misses — the follow-up that finding asked for — and the framing does not
hold. **`checksum_failed` on the current corpus is not a usable OCR-accuracy
signal.** Most of it is two other things.

## Method

- `provider-bench --real-specimens --dump-ocr` (the full pre-parse OCR dump added
  in [#236](https://github.com/ruledicaprio/SynthPass/pull/236)), `mrz` provider,
  whole corpus, no `--limit`.
- Repo `320ef00`; `samples/corpus.jsonl` sha256 `e86a8646…`, 236 rows.
- The `llm` provider pass was not run to completion (its CPU inference dominates
  runtime and adds nothing here — `checksum_failed` is the deterministic provider's
  verdict). The 71 `mrz`-provider dump blocks are in
  `artifacts/checksum-dump/mrz-pass.log`.
- Each of the 71 blocks was read by hand; a handful were re-checked by feeding the
  OCR's own cleanest MRZ lines back through `mrz::find_and_parse`.

## Run totals

| | count | share of 236 |
|---|---|---|
| Tier-1 HIT | 118 | 50.0% |
| MISS `checksum_failed` | 71 | 30.1% |
| MISS `no_mrz_found` | 47 | 19.9% |

## The 71 `checksum_failed` misses break into three populations

| population | count | what it actually is |
|---|---:|---|
| **A — no MRZ exists** (`*_no_mrz` specimens) | 19 | The document has no machine-readable zone. OCR read VIZ text / card labels / the guilloché as two `[A-Z0-9<]` lines ≥ 28 chars, and `mrz::find_and_parse` accepted them. |
| **B — MRZ deliberately redacted** (`*_redacted_mrz`) | 25 | The MRZ is blacked out, X-ed out, or scrambled in the specimen itself. The information is not in the image. |
| **C — MRZ present, no ground truth** (`*_mrz`) | 27 | A real MRZ is on the page. But **no specimen in this set has verified ground truth**, and spot-checks show several carry deliberately non-conforming MRZs (template / "SPECIMEN" / "ÖRNEK" / "VZOR" data with wrong check digits by design). |

Only population C could contain a genuine OCR misread, and it cannot currently be
told apart from a non-conforming specimen.

### A — `find_and_parse` accepts VIZ text as an MRZ (19 of 71)

The "recovered MRZ zone" for these is card boilerplate:

```
Czechia_Passport_..._no_mrz     ->  Td2   "CESKAREPUBLIKACZECHREPUBLIC<<<" / "CE5KAREPU811KACZECHREPUBLIC<<<"
Algeria_Passport_..._no_mrz     ->  Td2   "CEPASSERORTCONTIENT28PAGES..."  / "THISPASSP0RTC0NTA1N528PA6E5..."
Croatia_BorderPass_..._no_mrz   ->  MrvB  "VRSTADOZVOLEITYPEOFPERMIT..."   / "POGRANICNAPROPU5N1CA323002100..."
Liechtenstein_ID_..._no_mrz     ->  MrvB  "VORNAMEINIFIRSTNAMEISI..."      / "GEBURTSDATUMDATE0F81RTH..."
Luxembourg_ID_..._no_mrz        ->  Td2   "CHIEFOFPROTOCO1SIGNATURE..."    / "M1N15TERE0E5AFFAIRES..."
```

`mrz::find_and_parse` requires two lines of the MRZ character set at a plausible
width and nothing more — no gate on line 1 carrying a valid document code, on the
issuing-country field being a code in the table, or on the date fields being
digits. So a "document number" that is all letters and a "date of birth" that is
`REPUBL` parse fine and then fail their check digits. Every one of these 19
documents *should* be `no_mrz_found`.

This is also the one item here with a **correctness** stake, not just a metric one:
a hallucinated MRZ that happened to satisfy its check digits would be a silent
wrong extraction, which is worse than any miss.

### B — redacted MRZ (25 of 71)

`Armenia_..._redacted_mrz` OCRs as `XXXXXXXXXXXXXXXXXX`. `Albania_..._redacted_mrz`
reads the redaction bar as `AASFLKJAFLAKGSDF…`. A few (`Australia`, `Nepal …
blur_rotated`, `Russia 2016`) keep a readable name line but the number/date zone is
gone. Nothing to recover — the pixels don't carry it. These should be excluded from
the accuracy denominator or tracked as their own outcome, not scored as OCR misses.

### C — real MRZ, unverifiable (27 of 71)

All 27 have a real MRZ and ≥ 2 MRZ-shaped OCR lines. But feeding the OCR's own
cleanest lines back through `mrz::find_and_parse` still fails for the ones checked
(UK, Poland, Korea ×1, Türkiye ×1, Mauritania) — e.g. the UK specimen's
consistently-read `8248122645GBR8912126M3107054<<<<<<<<<<<<<<00` has a
document-number check digit of `5` where the 7-3-1 rule wants `3`. These are
`…TEMPLATE<<FAKE…` / `…ORNEK…` documents whose printed MRZ is not conformant.

Genuine OCR/parser problems *are* visible in this group, they just can't be counted
without labels:

- **character confusion** — `0↔O` (`Ghana`: `H00000014` vs `HOODOD14`), `S↔5`
  (`India 2013`: `L5733700` read `LS733700`), `2↔7` (`India boxed`: leading `2`
  read `7`), `M↔N` (`Ghana` sex `M`→`N`).
- **line-2 left-truncation** — `Afghanistan` drops the leading `44` of the document
  number, shifting every field left.
- **candidate selection picks a garbled duplicate line-1 for line 2** — `UK`,
  `Poland`, `Korea 2022`, `Türkiye 2025`, `Russia 2019`, `Mauritania`: the recovered
  zone is two line-1 reads, or a mangled line-1 in the line-2 slot, while a cleaner
  line-2 sits elsewhere in the same OCR output. Matches the ROADMAP note that
  line-1 prefix-shift repair is *"hit-rate-safe but not complete."*
- **personal-number-only near-miss** — `Germany D00 2018` fails **only** the
  optional-data check digit; `Colombia 2026`, `Czechia 2005`, `Romania 2024` fail
  `personal_number` + `composite` and nothing else. One digit wrong in the 14-char
  optional field.

## Result — step 1 shipped (`mrz` 0.7.0, PR #238)

`find_and_parse` now returns `NotFound` when the best-scoring non-validating reading
has an unrecognized issuing state **and** an unrecognized nationality **and** a
non-numeric date of birth (`parser::looks_like_non_mrz_text`). Re-run of the full
corpus with `mrz` 0.7.0, same 236 specimens:

| outcome | before (0.6.6) | after (0.7.0) |
|---|---:|---:|
| Tier-1 HIT | 118 | **118** |
| `checksum_failed` | 71 | **33** |
| `no_mrz_found` | 47 | **85** |

**Zero HIT regression** — the gate is `!valid()`-only and the 118 hits all carry a
resolving issuing state or nationality. **38 documents** moved `checksum_failed →
no_mrz_found`: 17 of the 19 population-A hallucinations (the two that stay,
`Turkiye_ID` and `Portugal_ID`, have a real 3-letter code — `TUR`, a `PRT` fragment —
sitting in the boilerplate), 17 population-B redacted specimens (a blackout bar
reads as junk in all three fields too — correctly not-found), and 3 population-C
documents whose OCR was total mush (`France_ID_..._back`, `Italy_ID_..._back`,
`Moldova_Passport_2014` — `no_mrz_found` is the honest label; the OCR never
produced a readable zone). Nothing entered `checksum_failed`.

The `checksum_failed` bucket is now **33**: 23 genuine `*_mrz`, 8 partially-readable
redacted, 2 stubborn no-MRZ.

## Result — step 3 shipped: the 23 are labelled (PR 1)

Every one of the 23 residual genuine `*_mrz` specimens now has a hand-transcribed
`samples/ocr_fixtures/<stem>.json` recording the **true printed MRZ zone**, read from the
specimen scan and checked against `mrz::find_and_parse`. `samples/corpus.jsonl` carries
`ground_truth_stem` for all 23 and `expected_document_number` for the 7 whose printed zone
is checksum-valid (`corpus_manifest` now withholds the expected number from a specimen
labelled `mrz_checksums_valid: false` — a conformant Tier-1 read of it produces nothing to
expect).

The transcriptions settle the question the residual bucket left open — is a
`checksum_failed` here an OCR misread, or a specimen whose printed check digits are wrong by
design:

| | count | what a `checksum_failed` on it means |
|---|---:|---|
| **Printed MRZ is checksum-valid** | 7 | The zone passes every ICAO check digit. Any `checksum_failed` is **100 % an OCR error** — a step-4 anchor. |
| **Printed MRZ is non-conforming** | 16 | The zone itself fails ≥ 1 check digit, or is structurally malformed. Not an OCR-accuracy signal; belongs outside the denominator. |

**The 7 checksum-valid** (step-4 targets): Afghanistan `P0_AFG_2016` (OCR reads the leading
letter `O` of the document number as `0`), Belgium ID 2021 back, Croatia ID 2021 back,
Czechia `P0_CZE_2005`, Romania `PE_ROU_2024`, Russia `P0_RUS_2019`, Sweden ID 2022 back.
Croatia's zone is all-zeros and Czechia's date-of-birth field is `110229` (Feb 29 of a
non-leap year) — the *check digits* are valid, the *dates* are placeholders; each fixture
records that (`mrz_checksums_valid: true`, malformed date fields left null).

**The 16 non-conforming**, by the field that fails: Germany `P0_D00_2018` (optional-data
check only — a one-digit near-miss), Ghana `P0_GHA_2019` (composite only), Colombia
`PP_COL_2026` (personal-number + composite), India `P0_IND_2013` (`L5733700`, check digit
`0` where the rule wants `6`), India `P0_IND_2013` boxed (line 1 malformed; expiry field
`230000`), Indonesia `P0_IDN_2011` (sex field printed as the letter `S`), Korea `PM_KOR_2020`
/ `PM_KOR_2022` (birth + expiry check digits printed `0`), Mauritania `P0_MRT_2010` (line 2
is 45 characters — an extra `<` at position 10), Poland `P0_POL_2023` (blank document-number
check digit), Switzerland ID 2023 back, and the five `TEMPLATE` / `FAKE` / `ÖRNEK` / `TEST`
passports (Türkiye `P0_TUR_2010` / `2024` / `2025`, Türkiye ID 2020 back, UK `P0_GBR_2021`).

Method: the MRZ band was cropped and upscaled from each scan, read by eye, and every
`mrz_line` fed back through `mrz::find_and_parse` — its `valid()` verdict is what sets each
fixture's `mrz_checksums_valid`. `extraction_method` is `"hand-transcribed"`. The `.md`
companion each fixture needs for parity is the live OCR text, which for these specimens is
visibly garbled (`H0000014` → `HOODOD14`, sex `M` → `N`, …) — the step-4 material.

## Result — step 4 tooling shipped: the bench splits the bucket (PR 2)

`provider-bench --real-specimens` now sub-classifies a `checksum_failed` miss. When the
specimen carries a hand-transcribed `mrz_line` **and** the run's `find_and_parse` recovered
that exact zone character-for-character (`mrz_zone_mismatch == 0`), the miss is reported as
`checksum_failed_specimen` — the printed document's own check digits are non-conforming, not
an OCR error. `--dump-ocr` rows (and, since this PR, the stdout dump block) carry
`ground_truth_mrz` and `zone_mismatch` — the per-line character distance between the parser's
recovered zone and the transcription. Unlabelled specimens and the entire synthetic corpus
are unaffected (`specimen_nonconforming: false` always).

Full-corpus re-run, `mrz` provider, repo `b3965f7`, 238 specimens (the corpus grew by 2
after step 1's run):

| outcome | step 1 (0.7.0, 236 docs) | with the split (238 docs) |
|---|---:|---:|
| Tier-1 HIT | 118 | 120 |
| `checksum_failed` | 33 | **32** |
| `checksum_failed_specimen` | — | **1** |
| `no_mrz_found` | 85 | 85 |

PR 2 relabels only — it changes no verdict. Exactly **1** document moves
`checksum_failed → checksum_failed_specimen`: **Colombia `PP_COL_2026`**, whose non-conforming
printed zone (personal-number + composite check digits wrong) native OCR happens to recover
exactly. The HIT `+2` / bucket `−1` against step 1 is the two new specimens, not this change.

The other **22** of the 23 labelled specimens stay in `checksum_failed`: native OCR does not
reproduce their printed zone, so the classifier cannot yet attribute the failure to the
document. That gap *is* the step-4 signal — `zone_mismatch` now quantifies it per specimen:

- **7 checksum-valid anchors**, native-OCR zone error (characters): Afghanistan `1`,
  Czechia `3`, Belgium ID `17`, Romania `23`, Sweden ID `27`, Croatia ID `30`, Russia `112`
  (parser latched a wrong 29-char candidate). Afghanistan is **one character** (`O`→`0` in
  the document number) from a clean Tier-1 HIT — the tightest step-4 target.
- **16 non-conforming**, zone error: Colombia `0` (→ `checksum_failed_specimen`), India boxed
  `1`, Türkiye `P0_TUR_2010` `8`, India `17`, Germany `18`, Switzerland `22`, Ghana `32`,
  Türkiye `2024` `34` / `2025` `35`, UK `36`, Mauritania `40`, Korea `2022` `43` / `2020`
  `44`, Poland `55`, Türkiye ID 2020 `56`, Indonesia `109`.

The split mechanism is proven (unit tests + the Colombia live case); native OCR accuracy on
these low-resolution guilloché scans is now the sole thing between the bucket and a full
attribution, which is precisely what step 4 works on.

## Ranked next steps

1. ~~**Gate MRZ acceptance on line-1 structure.**~~ **Done** — `mrz` 0.7.0 above.
2. **Give redacted specimens their own outcome in the bench.** 8 `*_redacted_mrz`
   still land in `checksum_failed` (the 17 whose redaction bar OCR'd as junk in all
   three fields already moved to `no_mrz_found` via step 1). `samples/corpus.jsonl`
   already carries `mrz.redacted: true`; the bench does not read it. Excluding them
   from the miss denominator (or a `redacted` `MissReason`) removes the last of
   population B. (`MissReason::ChecksumFailed` now also carries
   `specimen_nonconforming` — see the step-4-tooling result above.)
3. ~~**Grow ground truth for the 23 remaining `*_mrz`.**~~ **Done** — the "Result — step 3"
   section above. All 23 have a `samples/ocr_fixtures/<stem>.json`; 7 carry a checksum-valid
   printed zone, 16 are non-conforming by design. An OCR misread and a non-conforming
   specimen can now be told apart.
4. **Then** the character-confusion / line-selection `mrz` fixes (wider
   `CONFUSABLES` / wiring `solve_substitution` into the checksum-invalid path;
   line-2 left-anchor repair; candidate ranking that prefers a valid line-1+line-2
   pair over two line-1s). Each pinned by a regression test built from the specimen
   that motivated it — (3) and the step-4 tooling above now make "motivated"
   mean something: start with Afghanistan `P0_AFG_2016` (`zone_mismatch` 1,
   `O`→`0`) and Czechia `P0_CZE_2005` (`zone_mismatch` 3), the two checksum-valid
   anchors closest to a HIT.

`blindspot_seq` (sequence-level check-digit blind spot,
[2026-08-05 note](checksum-blindspots-measured-2026-08-05.md)) is unrelated and
stays deferred — it is about mismatches that *pass* the checksum, the opposite end.
