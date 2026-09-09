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

## Result — step 2 shipped: redacted specimens leave the bucket (PR 3)

`provider-bench --real-specimens` classifies a `*_redacted_mrz` specimen as its own miss
kind, **`redacted_mrz`**, ahead of the checksum gate, and drops it from the Tier-1 hit-rate
denominator — the redaction bar carries no recoverable zone, so scoring it as a `checksum_failed`
OCR miss (or letting it cap the achievable rate) was never right. The signal is the filename
token: `provider-bench` walks the image directory, never `samples/corpus.jsonl`, so it derives
`redacted` the same way `corpus_manifest` records `mrz.redacted`. A `corpus_manifest` test
locks the two derivations together.

Deterministic relabel off the step-4 run's per-specimen verdicts (repo `b3965f7`, `mrz`
provider, 238 docs) — the classification is filename-only, so no re-measurement is needed:

| outcome | step-4 tooling | with redacted split |
|---|---:|---:|
| Tier-1 HIT | 120 | **119** |
| `checksum_failed` | 32 | **24** |
| `checksum_failed_specimen` | 1 | 1 |
| `redacted_mrz` | — | **9** |
| `no_mrz_found` | 85 | 85 |
| **denominator** | 238 | **229** |
| **Tier-1 hit rate** | 50.4 % | **52.0 %** |

The 9: the **8** redacted specimens that were in `checksum_failed` (Australia `P0_AUS_2015`,
Colombia `P0_COL_2021`, France, Iran `P0_IRN_2017`, Japan `P0_JPN_2009`, Nepal `P0_NPL_2011`
×2, Russia `P0_RUS_2016`) plus **Malaysia `P0_MYS_2019_redacted_mrz_blur`, which was scoring a
Tier-1 HIT** — `find_and_parse` validated over whatever survived the redaction. Moving a
checksum-"valid" read of a redacted zone out of the HIT column is the honest call, not a
regression; the rate still rises because the denominator loses 9 and the numerator only 1.
The other 27 redacted specimens were already `no_mrz_found` (their bar OCR'd as junk in all
three structural fields, caught by `mrz` 0.7.0's gate) and stay there — the new rung sits
after the `mrz_found` gate on purpose.

## Result — step 4: `CONFUSABLES` widened; guard investigated; the 7 anchors are OCR-bound

**Shipped (`mrz` 0.7.1).** The damaged-read repair table (`find_and_parse` only, after
nothing validated) gained `M`↔`N` and `2`↔`7` — both stroke-shape confusions named in the
step-3 findings (`Ghana_..._P0_GHA_2019` reads the sex `M` as `N`; `India_..._P0_IND_2013`
reads a printed `2` as `7`) that the existing round-letter / vertical-stroke rows did not
cover. Both cross residue classes, so the field check digit rejects the misread and a repair
is only taken when the digit proves it and nothing else also verifies. Two end-to-end
regression tests through `find_and_parse` (`crates/mrz/tests/repair.rs`); `cargo-semver-checks`
reports no API change.

**No measured corpus effect.** Replaying all 25 real-specimen `checksum_failed` OCR texts
(the step-2 run's `--dump-ocr` output) through the patched parser: every one of the 7
checksum-valid anchors is unchanged (`zone_mismatch` delta 0), and the non-conforming
specimens too. The two pairs are real, but on this corpus the blocker is never one `M`/`N`
or `2`/`7` in isolation.

**Investigated, not shipped — candidate-selection guard.** A predicate that skips a line-2
candidate which is structurally a *line 1* (`P`/`V` prefix + issuing-state shape, no line-2
birth date) was prototyped against the split-line search and replayed on the same 25 texts:
zero anchor movement; all effect confined to the 16 non-conforming template specimens, which
miss regardless — Mauritania `40→4`, Türkiye-2025 `35→5`, Korea-2020 `44→27`, Poland `55→48`
(cleaner recovered zone, still a miss), Korea-2022 `43→47` (small regression), UK-2021 and
Türkiye-2024 `checksum_failed → no_mrz_found` (on noisy multi-pass OCR the parser was already
mis-pairing; the guard trades one wrong bucket for another). No accuracy change, unpredictable
on noisy OCR — not shipped. The case it targets (a repeated name line sliced into a garbage
record) yields `checksum_failed`, never a silent valid read.

**The 7 checksum-valid anchors are OCR-quality-bound — track closed.** Each residual miss is
now attributable to native OCR on a low-resolution guilloché scan, not to `mrz`:

| anchor | `zone_mismatch` | why it stays a miss |
|---|---:|---|
| Afghanistan `P0_AFG_2016` | 1 | Line 2's `O`→`0` is already covered by `substituted()`; the blocker is line 1, whose `<<`/`<` filler runs OCR collapses. TD3 line 1 has no check digit — nothing can place the name separators. |
| Czechia `P0_CZE_2005` | 3 | Needs `9`→`2` in `personal_number` (a weak same-parity pair) **and** a second fix on the same line; `damaged_pass` applies one repair kind per line, `MAX_SUBSTITUTIONS = 1`. |
| Belgium `2021_back` 17 · Romania `PE_ROU_2024` 23 · Sweden `2022_back` 27 · Croatia `2021_back` 30 | 17–30 | Broad, diffuse multi-character degradation; no single confusion or line-selection fix applies. Croatia's printed zone is all-zeros with placeholder dates. |
| Russia `P0_RUS_2019` | 112 | Line 2 is physically unreadable in this scan (longest MRZ-shaped candidate ~24 chars vs 44). |

Out of scope unless a future measurement shows an anchor one clean confusion from valid:
over-width-by-one repair, per-field `solve_substitution` wired into `damaged_pass`,
`MAX_SUBSTITUTIONS > 1`, weak same-parity pairs (`9`↔`2`).

## Ranked next steps

1. ~~**Gate MRZ acceptance on line-1 structure.**~~ **Done** — `mrz` 0.7.0 above.
2. ~~**Give redacted specimens their own outcome in the bench.**~~ **Done** — the
   "Result — step 2" section above. `MissReason::Redacted` → `redacted_mrz`, ahead of the
   checksum gate and off the denominator; `checksum_failed` 32 → 24, rate 50.4 % → 52.0 %.
   The `MissReason::ChecksumFailed { specimen_nonconforming }` sub-split from the step-4
   tooling is unaffected.
3. ~~**Grow ground truth for the 23 remaining `*_mrz`.**~~ **Done** — the "Result — step 3"
   section above. All 23 have a `samples/ocr_fixtures/<stem>.json`; 7 carry a checksum-valid
   printed zone, 16 are non-conforming by design. An OCR misread and a non-conforming
   specimen can now be told apart.
4. ~~**Then** the character-confusion / line-selection `mrz` fixes.~~ **Done** — the
   "Result — step 4" section above. `CONFUSABLES` widened (`mrz` 0.7.1, no measured corpus
   effect); the candidate-selection guard was measured and rejected; the 7 checksum-valid
   anchors are each now attributable to native OCR, not `mrz`. **This closes the
   `checksum_failed` root-cause track** — any further gain is an OCR-quality problem.

`blindspot_seq` (sequence-level check-digit blind spot,
[2026-08-05 note](checksum-blindspots-measured-2026-08-05.md)) is unrelated and
stays deferred — it is about mismatches that *pass* the checksum, the opposite end.
