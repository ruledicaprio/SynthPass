# Phase D: the two stacks tie on count, split on documents — and the browser reads names

**Date:** 2026-09-18 · **MAIN:** `b2a0afd19fdc826f21bd822c66009dcbc49d6484` · **DATA:** `469a4ee7723148917cf023f1a7ab2af80a0ef7d3` · **Evidence:** Observed · **Status:** current

**2026-09-18.** The Phase D comparison prepared in
[`phase-d-measurement-2026-09-17.md`](phase-d-measurement-2026-09-17.md) **ran**, successfully, on
2026-09-17 — `web-ocr.yml` run **35169813105**, dispatched 01:14:59Z, browser sweep finished
02:35:06Z. That setup document's "comparison measurement not run" line was true when written and
false by the time it was read; it is amended, not rewritten. This file is the result.

Every figure below is **Observed** from the run's three artifacts unless marked otherwise. No
number here is a live headline: the live browser-vs-native row lives in
[`README.md`](README.md#current-headline-numbers) and points at this file.

## Provenance

| Pin | Value |
| :-- | :-- |
| Workflow / run | `web-ocr.yml`, run `35169813105` (schedule-equivalent `workflow_dispatch`) |
| Reference tag | `v1.5.0` = `a3951b33c94f551cb3fe39696b3ee3ee0cec00ed` (asserted an ancestor of MAIN) |
| MAIN | `b2a0afd19fdc826f21bd822c66009dcbc49d6484` (#315, 2026-09-17 03:09 +0200) |
| DATA (`samples-data`) | `469a4ee7723148917cf023f1a7ab2af80a0ef7d3`, resolved from the committed baseline pin |
| Baseline consumed | `real-specimen-mrz-baseline.json`, sha256 `29abe8783e0ca1b925ed23175d9a1148642c54c7376a55cf3ce53b6eff92fe64`, `measured_on_ci_sha` `2f14e00b357a51ea3130754b283bc95984da2bf4` |
| Manifest | `samples/corpus.jsonl` at MAIN, sha256 `d108d452008afaf1a9bb95cfdce89e860e8038a8e7281d0588ca6f2f64b6c0e2` |
| Native arm | `provider-bench --real-specimens --mrz-only --progress`, one `mrz` provider, `deterministic: true` |
| Native OCR arms | `SYNTHPASS_OCR_MAX_PASSES=14`, `MAX_SECONDS=52`, `ORDER=default`, `TEXTURE=on`, `SKEW=default`, `ROTATE=default` |
| Browser arm | `tests/web/run-corpus.mjs` — tesseract.js in headless Chromium (Playwright), `rotate=0`, no `--limit`, no `--floor` |

**The baseline this run pinned has since been superseded twice** — `#324` (San Marino relabel, DATA
`5988c7bd878e58cf0279bbe078b0e6a679ac46af`) and `#332` (strict-name and refusal bless,
`measured_on_ci_sha` `649cccc4a12c57e50b6feafa6d414f78c64672b1`), both landing after `b2a0afd`.
The native arm nonetheless reconciles with today's committed baseline **document for document**,
with exactly one difference, and it is that relabel (§ "Reconciling with the current baseline").

## 1. The populations reconcile

`tools/audit_benchmark_identity.py --check` ran before the corpus was materialized, against the
pinned DATA commit. It is clean:

| Audit field | Value |
| :-- | --: |
| `candidate_assets` / `manifest_assets` / `baseline_reported_documents` | 261 / 261 / 261 |
| `missing_assets`, `unlisted_assets`, `hash_mismatches` | 0, 0, 0 |
| `same_path_byte_conflicts`, `duplicate_sha256` groups | 0, 0 |
| `duplicate_stems` | 2 (each one stem, two encodings, distinct bytes) |
| Asset sources | 255 `samples-data` only, 6 `samples-data` + MAIN |
| Assets with ground truth | 59 |

**Did the two stacks run the same population? Yes, on every asset either of them touched.** The
native arm scored all 261. The browser arm scans the manifest rows where `mrz.present` is true —
212 rows — and every one of those 212 `asset_id`s is present in the native report (`web ∖ native =
0`). The 49 assets the browser did not scan are, without exception, native `no_mrz_expected`: the
refusal population. The manifest's other 34 rows (`covers/`) are outside the benchmark walk
entirely, which is why `web.corpus.manifest_rows` reads 295 against the audit's 261.

**But 212 is not a denominator either stack publishes.** It contains 58 documents native scores out
because no pipeline can hit them — 39 `redacted_mrz` and 19 `checksum_failed_specimen`. The
browser's `web.rate` of 144/212 = 67.9% and the report's `native_reference.rate` of 142/212 = 66.9%
are therefore over a mixed population, and neither is comparable to a published rate. The rest of
this file uses the scored population (154 at this DATA pin) and says so each time.

## 2. Same count, different documents

Both denominators, on this run's corpus pin (**Observed**):

| Population | n | Native | Browser |
| :-- | --: | --: | --: |
| Scored (can yield a hit) | 154 | **140** Tier-1 hits | **140** checksum-valid |
| Whole specimen corpus | 261 | 140 | not scanned beyond 212 |
| MRZ-bearing, non-redacted | 173 | 141 checksum-valid | 141 checksum-valid |
| MRZ-bearing incl. redacted (the report's own axis) | 212 | 142 checksum-valid | 144 checksum-valid |

The two arms are **not measuring the same predicate**: native's 140 is a Tier-1 hit (checksum-valid
**and** document number matches ground truth); the browser's 140 is "checksum-valid", with no
ground-truth check. `native_checksum_valid` in the joined table is `mrz_checksums_valid`, which is
why native reads 142 on the 212 axis and 140 as hits — the two extra are documents scored out for
other reasons (§4).

A tie on count is not a tie on documents. **Sixteen documents disagree**, eight each way:

| Class | n | Assets |
| :-- | --: | :-- |
| Native hit, browser failed | 8 | `id_cards/Russian_Federation_ID_Specimen_2013_back_mrz.jpeg`, `id_cards/Switzerland_ID_Specimen_2003_back_mrz.jpg`, `passports/Angola_Passport_Specimen_PN_AGO_2022_mrz.webp`, `passports/China_Passport_Specimen_2012_mrz.webp`, `passports/Dominican_Republic_Passport_Specimen_P0_DOM_2020_mrz.png`, `passports/Kazakhstan_Passport_Specimen_P0_KAZ_2004_mrz.png`, `passports/Netherlands_Passport_Specimen_P0_NLD_2014_mrz_blur.jpg`, `passports/North_Macedonia_Passport_Specimen_P0_MKD_2020_mrz_rotated.jpg` |
| Browser valid, native scored miss | 8 | `id_cards/Belgium_ID_Specimen_2021_back_mrz.png`, `id_cards/Croatia_ID_Specimen_2021_back_mrz.jpg`, `id_cards/France_ID_Specimen_2020_back_mrz.png`, `id_cards/Sweden_ID_Specimen_2022_back_mrz.jpg`, `passports/Afghanistan_Passport_Specimen_P0_AFG_2016_mrz.webp`, `passports/Czechia_Passport_Specimen_P0_CZE_2005_mrz.jpg`, `passports/Germany_Passport_Specimen_P0_D00_2024_mrz.jpg`, `passports/Romania_Passport_Specimen_PE_ROU_2024_mrz.jpg` |
| Both miss | 6 | `id_cards/Italy_ID_Specimen_2022_back_mrz.jpg`, `id_cards/San_Marino_ID_Specimen_2017_back_mrz.jpg`, `passports/Hong_Kong_Passport_Specimen_P0_HKG_2007_mrz.png`, `passports/Hong_Kong_Passport_Specimen_P0_HKG_2019_mrz.png`, `passports/Moldova_Passport_Specimen_PA_MDA_2014_mrz.jpeg`, `passports/Russian_Federation_Passport_Specimen_P0_RUS_2019_mrz.jpg` |

Seven of the eight native-only wins are browser **near misses** (`near_miss` — MRZ-shaped text
parsed, check digits failed), one returned nothing; all eight burned 13–17 browser passes.

Six of the eight browser-only wins are **verified against hand transcription**, not merely
self-consistent: five match the reviewed fixture's MRZ lines byte for byte
(`mrz_line_matches_fixture: true` — Croatia, Sweden, Afghanistan, Czechia, Romania) and Belgium
matches all nine reviewed fields while differing from the stored `mrz_line` string. The remaining
two (France, Germany) carry no fixture and are **unverified** beyond their own check digits.

## 3. What native was doing on its misses

`retry_stop` over all 261 documents (**Observed**): `exhausted` 123, `variant_valid` 73,
`general_valid` 63, `budget` 2. On the 14 scored misses:

| Miss | n | `retry_stop` | Median `ocr_ms` |
| :-- | --: | :-- | --: |
| `checksum_failed` | 10 | `exhausted` ×10 | — |
| `no_mrz_found` | 4 | `exhausted` ×3, `budget` ×1 | — |
| All scored misses | 14 | — | **17 004** |
| Scored hits | 140 | `variant_valid` 71, `general_valid` 63, `exhausted` 6 | **4 107** |

**Thirteen of the fourteen scored misses ran the pass list out without hitting the clock** — they
failed on recognition, not on time, so raising `MAX_SECONDS` buys nothing on them. The single
budget-stopped document is `id_cards/France_ID_Specimen_2020_back_mrz.png`: 72 227 ms of OCR
against a 52 s budget, `retry_budget_hit: true`, and it found no MRZ at all. The browser read a
checksum-valid MRZ on that same file on its **third** pass ("Retrying with contrast stretch") in
1 305 ms. That is the sharpest localization in the run: 55× the time, opposite outcome, same bytes.

The failure-heavy cost shape ADR-0010 predicts holds: scored misses cost 4.1× the median hit's OCR
time. Native OCR over the 212 shared assets totalled 2 118 246 ms (median 6 646); the browser sweep
totalled 1 542 031 ms (median 1 860). Both are CI-runner wall times on a shared machine and are
**not** a like-for-like engine benchmark — different language, different model, different
harness — so treat the ratio as indicative only (**Derived**). The native report's `speed` block
(`mean_ms: 3`) times the provider call, not OCR, and must not be quoted as a speed result.

## 4. Checksum-valid reads outside the scored population

Native produced a checksum-valid MRZ on two off-denominator documents, both already understood:
`passports/Argentina_Passport_Specimen_P0_ARG_2026_mrz.png` (`checksum_failed_specimen`, read as
MRV-A on retry `pass-09` — the fake printed zone from
[`denominator-bucket-a-2026-09-10.md`](denominator-bucket-a-2026-09-10.md)) and
`passports/Malaysia_Passport_Specimen_P0_MYS_2019_redacted_mrz_blur.png` (`redacted_mrz`, the known
partial redaction whose line 2 is intact). Native's `false_positive_mrz` bucket — a checksum-valid
MRZ on a document carrying none — is **0**, as in the baseline.

The browser produced four, and they are not the same shape:

| Asset | Native bucket | Verifiable? |
| :-- | :-- | :-- |
| `passports/India_Passport_Specimen_P0_IND_2013_mrz.jpg` | `checksum_failed_specimen` | **Yes — and wrong.** Reviewed fixture: `issuing_country`, `document_number` and `nationality` all disagree; MRZ lines do not match |
| `passports/Australia_Passport_Specimen_P0_AUS_2015_redacted_mrz.jpg` | `redacted_mrz` | No fixture. Won on pass 14 |
| `passports/Belarus_Passport_Specimen_P0_BLR_2006_redacted_mrz.jpg` | `redacted_mrz` | No fixture. Won on pass 3 |
| `passports/Iran_Passport_Specimen_P0_IRN_2017_redacted_mrz.jpg` | `redacted_mrz` | No fixture. Won on pass 13 |

The India case is a **demonstrated false accept**: a checksum-valid MRZ whose document number
disagrees with hand transcription, which under the native classifier is `document_number_mismatch`,
a scored miss. The other three have no fixture; visual inspection of the pinned bytes (sha256
verified against the audit) shows all three zones masked in whole or in part — the Iran specimen's
printed zone is essentially all filler, and the Australia and Belarus zones are white-boxed across
fields that carry check digits — so a checksum-valid read there cannot be corroborated and is
**Hypothesized** to be fabricated rather than recovered. Resolving it needs the raw read, which
neither artifact persists.

Two consequences. First, four of the browser's twelve `head_to_head.web_only` wins are on documents
where a checksum-valid answer is unverifiable or provably wrong, so `web_only: 12` over-credits the
browser by a third. Second, **the browser's refusal behaviour is unmeasured**: it never scans the
49 `no_mrz_expected` documents, so there is no browser counterpart to the baseline's 0 false
accepts over 108 refusal-population documents.

## 5. Names: the gap is in the native recognizer

The strongest result in the run, and the one the count-level tie hides. Restrict to documents that
**both** stacks read checksum-valid **and** that carry a reviewed fixture with both name fields —
31 documents, same bytes, same truth strings, no detection difference left in the comparison:

| Arm | Both name fields exact | Of 31 |
| :-- | --: | --: |
| Browser (tesseract.js) | **28** | 90.3% |
| Native (`ocrs`/`rten`) | **11** | 35.5% |

Seventeen documents are browser-right / native-wrong. **Zero are native-right / browser-wrong.**
Three are wrong in both. Native's `name_error` on its 20 misses there: `other` 17, `split_shifted`
2, `filler_read_as_letters` 1. The 17 span TD1 and TD3 and eleven issuing states.

Native's whole-run strict-name numbers match the committed baseline exactly —
`strict_hits` 12 / `name_scorable_documents` 40 / `name_scorable_hits` 33,
`names_exact_among_hits` 0.3636 — so this is not a divergent arm: it is the same 30.0% seen from
the browser's side.

**Browser-side name scoring exists, with limits worth stating.** `run-corpus.mjs` scores fields
only on a checksum-valid read, and only against a fixture: all nine fields for a `reviewed`
fixture, the three ICAO-checksummed fields for a `derived` one. There is therefore **no browser
strict-name rate over a scored denominator** and none is invented here — the 28/31 above is the
browser's analogue of `names_exact_among_hits`, not of `strict_tier1_hit_rate`. Browser field
tallies across its 38 reviewed valid reads: `surname` 37/38, `given_names` 35/38, `document_number`
37/38, `sex` 37/37, `date_of_expiry` 37/37, `date_of_birth` 34/36.

## Reconciling with the current baseline

| Bucket | This run (MAIN `b2a0afd`, DATA `469a4ee`) | Committed baseline (CI `649cccc`, DATA `5988c7b`) |
| :-- | --: | --: |
| `documents` | 261 | 261 |
| `scored` | 154 | 153 |
| Tier-1 hits | 140 | 140 |
| `checksum_failed` | 10 | 10 |
| `no_mrz_found` | 4 | 3 |
| `no_mrz_expected` | 49 | 50 |
| `redacted_mrz` | 39 | 39 |
| `checksum_failed_specimen` | 19 | 19 |
| `false_positive_mrz` | 0 | 0 |
| `strict_hits` / `name_scorable_documents` | 12 / 40 | 12 / 40 |

One document moved, in one direction, for a reason already recorded: `#324` relabelled
`id_cards/San_Marino_ID_Specimen_2017_back_mrz.jpg` as a blank filler template, taking it out of
the scored denominator. It is one of the six documents both stacks miss above. Restated on today's
population the scored figures are 140/153 for native and 140/153 for the browser, and the
browser's MRZ-bearing population is 211, not 212 (**Derived** — arithmetic on the observed run, not
a re-measurement).

## What this does not claim

- **Not that the two stacks are equally good.** They tie on a count over one population and split
  on sixteen documents; a tie of differently-composed sets is a result about *where* each fails.
- **Not a browser hit rate.** The browser arm never checks a document number against ground truth
  except through a fixture, and only on valid reads. Its 140 and native's 140 are different
  predicates that happen to coincide.
- **Not that the browser's extra reads are wins.** Four of them are on documents no scored
  population contains, one of those is provably wrong, and three are unverifiable.
- **Not a diagnosis of the 17 native name misses.** `name_error: other` covers a glyph misread and
  a field-normalization difference alike; neither artifact persists the read values, so which of
  the two dominates is unmeasured. It is the next thing to measure, not something to assume.
- **Not a speed comparison.** The two wall-clock totals come from different engines in different
  runtimes on a shared CI runner, with no clean-machine control.
- **Not current-corpus.** The run predates `#324` and `#332`; §"Reconciling" gives the one-document
  restatement rather than pretending the pins match.
- **Not evidence that a native change would recover the eight browser-only documents.** That needs
  a same-binary A/B on the native side, which this run is not.

## Candidates rejected on the way

- **Quoting `web.rate` (144/212 = 67.9%) against `native_reference.rate` (142/212 = 66.9%) as the
  Phase D headline.** Rejected: the 212 population mixes 58 documents that no pipeline can hit into
  a denominator, and the two numerators are different predicates. The report computes both
  honestly; they are just not the comparison anyone wants.
- **Reading `head_to_head` as the disagreement set.** Rejected: `web_only: 12` includes the four
  off-denominator reads of §4, and `native_only: 10` includes the two off-denominator native reads.
  The scored-population split is 8 / 8.
- **Treating the 34-row manifest gap (295 vs 261) as a population defect.** Rejected after checking:
  the extra rows are `covers/`, outside the benchmark walk by construction, and the identity audit
  reconciles at 261.
- **Attributing the browser's redacted-document reads to partial redaction, by analogy with the
  Malaysia specimen.** Rejected as unsupported: Malaysia has an intact line 2 and native reads it;
  these three are masked across check-digit-bearing fields and *only* the browser reads them.
- **Comparing this run's browser figure to the 2026-09-09 "80.0% vs 74.4% over 160" row.** Rejected:
  different corpus pin, different population size (173 non-redacted MRZ-bearing here), and the
  native arm moved twice in between. The row is superseded by this file, not differenced against it.
