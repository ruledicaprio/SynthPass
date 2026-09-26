# `SYNTHPASS_OCR_STOP=clean` loses hits on both corpora, because the parser never sees the loop's decision

**Date:** 2026-09-26 · **MAIN:** `405b4f0` · **DATA:** `396b22f` (real arm); none for the synthetic arms (generated corpus) · **Evidence:** Observed (one local release build, six A/B runs switched by env var, plus single-document verbose traces on the same binary) plus Derived (per-document and per-seed comparison) plus Hypothesized (the exact parser branch, marked where used) · **Status:** current

**2026-09-26.** The same-binary A/B that
[#473](https://github.com/ruledicaprio/SynthPass/issues/473) asked for, on the arm PR
[#488](https://github.com/ruledicaprio/SynthPass/pull/488) added. The arms are
`SYNTHPASS_OCR_STOP=first-valid` (the default) and `clean`, with the confirm budget
`SYNTHPASS_OCR_CONFIRM_PASSES` at its default of 2. The real reports record both knobs in
`ocr_arms` (PR [#498](https://github.com/ruledicaprio/SynthPass/pull/498)). Under `clean`, a
checksum-valid reading that only `mrz`'s damaged-capture search produced (`damaged_recovery`) does
not end the OCR retry loop. The loop holds it and runs up to two more passes, looking for a clean
reading or one that agrees field for field.

## The answer

- **Observed: `clean` is worse on every population measured, and wins nothing.** Real specimens
  **139 → 137** Tier-1 hits. Synthetic TD3 ×100 **75 → 71**. The M4 gate set **39 → 37**. No
  document or seed moved the other way.
- **Observed: the arm fires rarely on real specimens.** It changed the retry path of 8 of 261
  documents. `repair_unconfirmed` fired 5 times and `variant_valid_confirmed` fired **0** times.
- **Observed and Derived: the mechanism is not the oracle.** The loop's decision is lost. The
  retry loop appends every pass's MRZ-shaped lines to one `text`. `mrz::find_and_parse` then
  re-decides over all of it. The confirm passes add readings that disagree, so the final parse
  falls back to the unrepaired, checksum-invalid OCR string. This is the hazard `has_valid_mrz`'s
  doc comment already records for the reverted stricter oracle ("appending more candidate lines
  … measurably hurt accuracy"). It is not
  [#482](https://github.com/ruledicaprio/SynthPass/issues/482).
- **Derived: the +7.5% wall time is not the arm's cost.** The documents the arm did not touch
  slowed by the same amount. The modelled cost of the arm is about **+18 s over 2 700 s of OCR
  (0.7%)**.

## Before / after

Real specimens, `provider-bench --real-specimens --mrz-only`, DATA `396b22f`:

| | `first-valid` | `clean` |
| :-- | --: | --: |
| Tier-1 hits, scored | **139 / 151 = 92.1%** | **137 / 151 = 90.7%** |
| Tier-1 hits, whole corpus | 139 / 261 = 53.3% | 137 / 261 = 52.5% |
| `checksum_failed` (scored) | 10 | 12 |
| `no_mrz_found` (scored) | 2 | 2 |
| `false_positive_mrz` | 0 | 0 |
| `no_mrz_expected` / `redacted_mrz` / `checksum_failed_specimen` | 51 / 39 / 20 | 51 / 39 / 20 |
| Strict names (both exact) / name-scorable | 12 / 45 = 26.7% | 11 / 45 = 24.4% |
| `retry_stop`: `general_valid` / `variant_valid` | 62 / 72 | 58 / 71 |
| `retry_stop`: `repair_unconfirmed` / `variant_valid_confirmed` | — / — | **5 / 0** |
| `retry_stop`: `exhausted` / `budget` | 125 / 2 | 125 / 2 |
| OCR time (sum of per-document `ocr_ms`) | 2 700.1 s | 2 904.3 s (+7.56%) |
| Wall time (run log) | 2 717 s | 2 920 s (+7.47%) |

The `first-valid` arm matches the committed CI ledger (`real-specimen-outcomes.jsonl`, CI
`eeacdf5`) on the outcome of **all 261 documents**. So the local control is the baseline, and
the two moves below are not `rten` noise. One of them (Azerbaijan 2013) reproduced in a second,
independent `clean` run.

Synthetic, `synthpass-bench --profile clean --seed 0`. "Correct" means a hit exact on all twelve
scored fields, per `tools/synth_ab_diff.py`:

| | TD3 ×100 `first-valid` | TD3 ×100 `clean` | M4 set ×50 `first-valid` | M4 set ×50 `clean` |
| :-- | --: | --: | --: | --: |
| Hits | 75 | 71 | 39 | 37 |
| Correct | 39 | 37 | 18 | 17 |
| Wrong accepts | 36 | 34 | 21 | 20 |
| Valid misses | 3 | 3 | 1 | 1 |
| `strict_hits` | 45 | 43 | 23 | 22 |

The M4 set is seeds 0–49 of the same TD3 corpus. Its per-seed results equal the TD3 ×100 run's
seeds 0–49 in both arms, field for field. Both arms clear `ci.yml`'s `--min-hit-rate 0.30` floor.
That is a floor, not a no-regression check.

## What moved

**Real specimens: two hits lost, one name lost, nothing gained.** These are the 8 documents
whose retry path changed:

| Document | `retry_stop` (variant) FV → clean | Outcome | `ocr_ms` |
| :-- | :-- | :-- | :-- |
| `passports/Azerbaijan_Passport_Specimen_PC_AZE_2013_mrz.webp` | `general_valid` → `repair_unconfirmed` (general) | **hit → `checksum_failed`** | 2 861 → 4 805 |
| `passports/Kuwait_Passport_Specimen_P0_KWT_2023_mrz.png` | `general_valid` → `repair_unconfirmed` (general) | **hit → `checksum_failed`** | 2 019 → 4 261 |
| `passports/Azerbaijan_Passport_Specimen_PC_AZE_2022_mrz_black_and_white.png` | `variant_valid` pass-00 → pass-01 | hit → hit, **names exact → wrong** | 4 770 → 6 560 |
| `passports/Canada_Passport_Specimen_PP_CAN_2023_mrz_highlight.webp` | `general_valid` → `variant_valid` pass-01 | hit → hit (names wrong in both) | 2 790 → 5 362 |
| `passports/Netherlands_Passport_Specimen_P0_NLD_2006_mrz.jpg` | `variant_valid` pass-00 → pass-02 | hit → hit | 3 148 → 5 686 |
| `passports/Hungary_Passport_Specimen_P0_HUN_2012_mrz.png` | `general_valid` → `repair_unconfirmed` (general) | hit → hit | 3 297 → 6 293 |
| `id_cards/Romania_ID_Specimen_2021_back_mrz.png` | `variant_valid` → `repair_unconfirmed` (pass-01) | hit → hit | 3 740 → 4 943 |
| `misc/Croatia_BorderPass_Specimen_CB_HRV_2025_back_mrz.png` | `variant_valid` → `repair_unconfirmed` (pass-06) | hit → hit | 10 314 → 15 455 |

In three of the eight, a clean reading superseded the held repair, which is the case the arm was
built for. Those three gained nothing. In Azerbaijan 2022, the clean reading was **worse on
names**: no check digit covers a name, so a reading being clean is no evidence the names are right.

**Synthetic: four seeds lost, none gained.** Seeds 22 and 66 went from correct to
`checksum_failed`. Seeds 43 and 72 went from a wrong accept to `checksum_failed`. The field that
now fails its digit:

| Seed | Field | Truth | `clean` returns | General pass read |
| --: | :-- | :-- | :-- | :-- |
| 22 | `document_number` | `6OQCGRMOU` | `60QCGRMOU` (O→0) | `60QCGRMOU5BRA9 1 10018 …` |
| 43 | `document_number` | `NZ13888OS` | `NZ138880S` (O→0) | `NZ 138880S6C AN9 …` |
| 66 | `personal_number` | `…LU7U0AF` | `…LU7UOAF` (**0→O**) | `…ILU7UOAF 12` |
| 72 | `personal_number` | `…X6ED0Z` | `…X6EDOZ` (**0→O**) | `…JS4IDSMEX6EDOZ52` |

Two of the four are 0→O, not O→0. The wrong glyph was not read by a later pass: the **general
pass** read it. Under `first-valid`, the damaged-capture search repaired it back to the truth, and
the loop stopped there. Under `clean`, the returned value is that unrepaired string, surfacing as
the checksum-invalid fallback.

## Mechanism

**Observed**, from `SYNTHPASS_OCR_VERBOSE=1` traces on the A/B binary (seed 22, `clean`):

```
general pass: valid MRZ needed damaged-capture repair; holding for confirmation
variant 0: valid but damaged-capture MRZ, holding for confirmation (1 pass(es) left)
variant 1: MRZ-shaped but checksum-invalid lines:
  PBRAMORAVEC<MAREN<<<<<<
  60QCGRMOU5BRA9110018F29010198CHFTF6E6KC73H78
confirm-pass budget (2) exhausted before variant 2; accepting the damaged-capture reading from Some("general") unconfirmed
seed 22 [clean]: MISS - checksum invalid: document_number, composite
```

The loop logs that it accepts the general pass's reading, and the document still scores
`checksum_failed`. The decision is not handed over. `OcrPage.text` is the general text plus the
candidate lines of every confirm pass, and `MrzReader` re-parses all of it. The loop's own
`same_document` check already showed that variant 0's repair disagreed with the held one. That
disagreement ends up in the one `damaged_pass` over the combined text.

- **Observed: the decision is lost even when the confirmation succeeds.** Re-run seed 22 with
  `SYNTHPASS_OCR_CONFIRM_PASSES=3` and variant 2's repair *agrees* with the held reading. The loop
  stops with `variant_valid_confirmed`, and the result is still `checksum_failed` on the same
  field.
- **Observed on seeds 66, 72 and Azerbaijan 2013:** neither confirm pass validates on its own.
  Adding their lines to the text is still enough to lose the general pass's recovered hit.
- **Hypothesized: the exact parser branch.** The most likely cause is `single()` (#440's unanimity
  gate in `damaged_pass`) refusing, because the larger line set produces more than one
  disagreeing `accept_damaged` reading. The alternative is the shared `MAX_DAMAGED_ATTEMPTS` budget
  (200 000), consumed by the extra lines before the general pair is reached. One replay would
  decide it: feed the dumped `text` through an instrumented `find_and_parse` and print the hit
  count and the remaining budget.

**Not #482.** [#482](https://github.com/ruledicaprio/SynthPass/issues/482) is the ordinary scan
*accepting* the first of two disagreeing valid line-1 readings. It was closed on 2026-09-26. Here
nothing validates in the ordinary scan, and the damaged pass *refuses*. The defect sits in the
handoff between `synthpass-ocr` and `mrz`, not in either parser rule.

## The #469 / #473 repro seeds were exercised

#473's repro (seeds 99 and 85) and #469's (seeds 26, 86, 66, 18) both state "synthetic TD3, clean,
seed 0". #473 gives the invocation in full, `--document-type td3 --profile clean --count 100
--seed 0`, which is exactly this run's TD3 ×100 arm. **The arm fired on both:**

- **Seed 99** (2 066 → 3 601 ms). The loop holds the general pass's repaired `73UTQNYVZ`. Both
  confirm passes read line 2 *exactly* (`Z3U1QNYVZ8MKD…`) but truncate line 1 to 27 and 26 of its
  44 cells, so neither validates. The parser pairs only adjacent lines, so the correct line 2
  never meets a usable line 1. The same wrong document number comes back at budgets 2, 3 and 5.
  #473's "pass 3 read the zone cleanly" predates #471. At `405b4f0`, no pass up to variant 4 does.
- **Seed 85** (4 481 → 5 006 ms). The loop holds pass-02's reading (`STRANDASTRID`, the separator
  lost), and no confirm pass yields a reading. The same wrong-name hit is accepted at budgets 2, 3
  and 5.

## Latency

- **Derived: the documents the arm did not touch** (253 of 261) slowed from 2 667.2 s to 2 851.0 s,
  **+6.9%**. 218 were slower, 32 faster and 3 equal. That is 184 of the 204 s total increase.
- **Derived: `exhausted` is a negative control.** For the 125 `exhausted` documents, the arm cannot
  change the pass sequence: with no valid reading, nothing is ever held. They still slowed by
  **+6.1%**.
- **Derived: the drift is not even across the run.** By run-order quartile, the untouched
  documents slowed +1.9%, +8.9%, +9.6% and +9.1%. The machine got slower about a quarter of the
  way into the `clean` arm and stayed slower.
- **Derived: the arm's own cost.** The 8 exercised documents went 32.9 s → 53.4 s (+20.4 s; +1.2 to
  +5.1 s each, one or two extra passes). Scaled by the untouched documents' ratio, that is about
  **+18.2 s**, or **0.67%** of the control's OCR time. This figure is modelled.
- **Synthetic, same picture.** On TD3 ×100 the total rose 412.7 → 441.5 s. The 16 seeds with a
  per-seed increase over 700 ms account for +21.3 s, and the other 84 rose ×1.021. The 700 ms
  proxy for "held" is noisy: the M4 run flags 18, 24 and 40, which the ×100 run does not.

The `speed` block in the real reports (mean 8 ms vs 9 ms) times the provider call, not OCR, so it
says nothing about this arm.

## Invocations

One release build at `405b4f0` in worktree `ocr-retry-stop-473`
(`cargo build --release -p synthpass-bench --bins`). The arm was set per run, and every other
`SYNTHPASS_OCR_*` variable was unset. Driver: `m4-ab/ab-473.sh`, log `m4-ab/473-ab.log`
(2026-09-26 16:06–18:09 +02:00, cargo slot held).

```
SYNTHPASS_OCR_STOP=<arm> synthpass-bench --count 100 --seed 0 --profile clean --document-type td3 --out 473-td3x100-<arm>.json
SYNTHPASS_OCR_STOP=<arm> synthpass-bench --count 50 --seed 0 --profile clean --out 473-m4-<arm>.json
SYNTHPASS_OCR_STOP=<arm> provider-bench --real-specimens --mrz-only --progress --out 473-real-<arm>.json
python tools/synth_ab_diff.py 473-td3x100-first-valid.json 473-td3x100-clean.json
```

Traces, on the same binary, with results in `artifacts/473-trace-*`:

```
SYNTHPASS_OCR_VERBOSE=1 SYNTHPASS_OCR_STOP=<arm> [SYNTHPASS_OCR_CONFIRM_PASSES=3|5] \
  synthpass-bench --count 1 --seed <22|43|66|72|85|99> --profile clean --document-type td3 --dump-ocr
SYNTHPASS_OCR_VERBOSE=1 SYNTHPASS_OCR_STOP=clean provider-bench --real-specimens --mrz-only \
  --format passport --limit 13 --dump-ocr --out artifacts/473-trace-real-passport13-clean.json
```

`--limit 13` is a stride over the 192 passports. It includes index 14, Azerbaijan 2013 `.webp`.

## What this does not claim

- **Not a CI measurement.** Local `rten` differs from CI by float rounding. The control matches the
  CI ledger on all 261 outcomes, but the clean arm's counts are local and are not a baseline.
- **TD3 only on the synthetic side.** #473 asked for all five formats. TD1, TD2, MRV-A and MRV-B
  were not run.
- **Not a verdict on the oracle idea.** The measured arm cannot express its decision, so this
  measures the handoff defect plus the oracle together. A version that hands the parser only the
  accepted reading has not been measured.
- **Budgets 3 and 5 were probed on three seeds only** (22, 85, 99), not swept over a corpus.
- **The synthetic `retry_stop` counts are not reported.** `synthpass-bench`'s JSON records neither
  `retry_stop` nor `ocr_arms`. The synthetic "held" counts come from traces and the timing proxy.

## Rejected on the way

- **`clean` as the default.** It would fail the real-specimen gate at `tolerance: 0`
  (`tier1_hits` 137 < 139, `checksum_failed` 12 > 10). It has no measured win on any population.
- **A larger confirm budget.** At `SYNTHPASS_OCR_CONFIRM_PASSES=3` and `=5`, seeds 22, 85 and 99
  keep their outcome, and each extra pass costs about 1 s. On real specimens,
  `variant_valid_confirmed` never fired at 2.
- **#482 as the cause.** #482 is the opposite rule (accept-first versus refuse), in a different
  stage (the ordinary scan versus the damaged pass). See above.
- **The +7.5% wall time as the arm's cost.** The negative control drifted +6.1%.
- **"The later pass read O as 0".** It was the general pass, and two of the four swaps are 0→O.
