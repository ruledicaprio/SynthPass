# #508 A/B at `first-valid`: only `variant_valid` documents move, and the accepted pass is a worse witness than the concatenation

**Date:** 2026-09-27 · **MAIN:** `9a639c6` (arm A, unpatched) against `0a8f7ff` (arm B, #508; rebased onto `4732d9f` as `65c7944` during the runs) · **DATA:** the corpus in worktree `m6-dump` (sha not recorded; arm A matches the CI ledger measured at DATA `396b22f` on all 261 outcomes); none for the synthetic arms (generated corpus) · **Evidence:** Observed (two local release builds, six full-population runs, ten single-seed traces; CI `real-specimen-gate.yml` run 36331797441 on `65c7944`) plus Derived (per-document and per-seed comparison) plus Hypothesized (the #536 outcome, marked where used); the private specimen is Observed locally, class only · **Status:** current

**2026-09-27.** This is the measurement of the #508 fix that
[`retry-handoff-replay-2026-09-27.md`](retry-handoff-replay-2026-09-27.md) says it is not. The
replay showed that the accepted pass's text alone reproduces the loop's decision on six
documents under `SYNTHPASS_OCR_STOP=clean`. This A/B runs the patched code at the default stop
mode, `first-valid`, on the whole real corpus and on two synthetic populations. The builds are
two, not one: B changes which text Tier 1 parses, and no env var switches that. The OCR itself
is identical between the arms, which is what makes the two-build comparison valid here (see
[Control](#control)).

## The answer

- **Observed: every mover is a `variant_valid` document, and its retry path is identical in both
  arms.** No `general_valid` or `exhausted` document moved on any population. This confirms the
  architect's expectation.
- **Observed: real specimens 139 → 138 Tier-1 hits.** Cyprus `P0_CYP_2010` goes from hit to
  `document_number_mismatch`: a checksum-valid reading of the wrong document number. Strict
  names go **12 → 14** of 45. CI reproduced both on `65c7944`.
- **Observed: on synthetic TD3, B gains two hits and loses one correct read.** TD3 ×100 goes
  75 → 77 hits, but correct reads go 39 → 38, wrong accepts 36 → 39, and checksum-valid wrong
  document numbers 3 → 5. None of the five seeds that moved became correct.
- **Derived: B makes Tier 1 agree with the loop, and on these documents the loop was wrong.**
  Under A, 4 of TD3 ×100's 37 `variant_valid` seeds scored `checksum_failed`. Tier 1's parse
  over the concatenation refused a reading the loop had accepted. Under B that count is 0, and
  every one of the four comes back wrong: two wrong accepts and two valid misses. The
  concatenation did two jobs that nobody designed. It let the ordinary scan pair a line from one
  pass with a line from another, and it let `single()` refuse when passes disagreed. On a variant
  stop, B removes both.

## Before / after

Real specimens, `provider-bench --real-specimens --mrz-only`, 261 documents:

| | A `9a639c6` | B `0a8f7ff` |
| :-- | --: | --: |
| Tier-1 hits, scored | **139 / 151 = 92.1%** | **138 / 151 = 91.4%** |
| Tier-1 hits, whole corpus | 139 / 261 = 53.3% | 138 / 261 = 52.9% |
| `checksum_failed` / `no_mrz_found` (scored) | 10 / 2 | 10 / 2 |
| `document_number_mismatch` (scored) | 0 | **1** |
| `false_positive_mrz` | 0 | 0 |
| Off-denominator: `no_mrz_expected` / `redacted_mrz` / `checksum_failed_specimen` | 51 / 39 / 20 = 110 | 51 / 39 / 20 = 110 |
| Strict names (both exact) / name-scorable | 12 / 45 = 26.7% | 14 / 45 = 31.1% |
| Names exact among name-scorable hits | 12 / 33 | 14 / 32 |
| `checksum_valid_on_failed_specimen` | 0 | 0 |
| `retry_stop`: `general_valid` / `variant_valid` / `exhausted` | 62 / 72 / 127 | 62 / 72 / 127 |
| OCR time (sum of per-document `ocr_ms`) | 1 530.7 s | 1 531.6 s |
| Wall time | 1 540.9 s | 1 542.3 s |

Synthetic, `synthpass-bench --profile clean --seed 0`. "Correct" means a hit exact on all twelve
scored fields. A "valid miss" is a checksum-valid reading of the wrong document number
(`document_number_mismatch`), per `tools/synth_ab_diff.py`:

| | TD3 ×100 A | TD3 ×100 B | M4 ×50 A | M4 ×50 B |
| :-- | --: | --: | --: | --: |
| Hits | 75 | **77** | 39 | 39 |
| Correct | 39 | **38** | 18 | **17** |
| Wrong accepts | 36 | **39** | 21 | **22** |
| Prefix wrong accepts | 1 | 1 | 0 | 0 |
| Valid misses | 3 | **5** | 1 | **2** |
| `checksum_failed` / `no_mrz_found` | 21 / 1 | 17 / 1 | 9 / 1 | 8 / 1 |
| `strict_hits` | 45 | 44 | 23 | 22 |
| `variant_valid` scored `checksum_failed` | 4 | 0 | 1 | 0 |
| Wall time | 246.1 s | 242.3 s | 119.5 s | 119.3 s |

Arm A equals the 2026-09-26 `first-valid` arm on every synthetic count in
[`retry-stop-ab-2026-09-26.md`](retry-stop-ab-2026-09-26.md): 75 / 39 / 36 / 3 and 39 / 18 / 21 /
1. The M4 set is seeds 0–49 of the TD3 corpus. Its movers are the TD3 ×100 movers below 50.

## What moved

**Real specimens: six documents, all `variant_valid`, on the same pass in both arms.** The raw OCR
text is byte-identical between the arms on all 170 documents both dumps carry.

| Document | Pass | A → B | Why |
| :-- | :-- | :-- | :-- |
| `passports/Cyprus_Passport_Specimen_P0_CYP_2010_mrz.png` | pass-02 | **hit → `document_number_mismatch`** | pass-02's line 2 reads `<000002200CYP…`. Alone, that parses to document number `<00000220`. Its check digit and the composite still validate, because `K` (value 20) and `<` (value 0) are congruent mod 10. A's concatenation paired an earlier pass's `K000002200CYP…` line with a garbled line 1. That was a hit, with the names wrong in both arms |
| `passports/Cyprus_Passport_Specimen_P0_CYP_2020_mrz.jpg` | pass-03 | hit → hit, **names exact → wrong, format TD3 → TD2** | pass-03's candidates are 23 and 40 cells wide (`CYPPOLITISZINONASKKKK<<`). Alone, they parse as a TD2 zone. A's concatenation found a clean 44-cell TD3 pair from an earlier pass |
| `passports/India_Passport_Specimen_P0_IND_2022_mrz.png` | pass-05 | names wrong → exact | A paired `PSINDHASSAN<SHOHAMMAD…`. B reads `P<INDHASSAN<MOHAMMAD<ANZARUL`, which the harness scores exact even though the separator is still a single `<` |
| `passports/Slovakia_Passport_Specimen_P0_SVK_2005_mrz.png` | pass-01 | names wrong → exact | A's line 1 was a second copy of line 2 (`P0OOO0005SVK…`). B reads `PSSVKSPECIMEN<VZOR` |
| `passports/Somalia_Passport_Specimen_P0_SOM_2023_mrz.png` | pass-09 | names wrong → exact | A's line 1 carried a stray `Z` in the filler |
| `passports/Spain_Passport_Specimen_P0_ESP_2013_mrz.jpg` | pass-01 | names wrong in both; `other` → `split_shifted` | B reads `ESPANOLESPANOL<<JUAN` |

Every scored count is unchanged except the one hit that moved to `document_number_mismatch`.
`scored` stays 151 and `documents` stays 261.

**Synthetic: five seeds, all `variant_valid` in both arms.** A single-seed re-run of each seed
in each arm (`--count 1 --dump-ocr`) reproduced all five moves exactly.

| Seed | A → B | Field, truth → B |
| --: | :-- | :-- |
| 20 (M4 too) | correct → **wrong accept** | `given_names` `IULIIA` → `TULITA`. The accepted pass read `PETROV<TULITA`, while an earlier pass had read `PETROV<<IULIIA` |
| 40 (M4 too) | `checksum_failed` → **valid miss** | `document_number` `R0K68D54E` → `ROK68054F`, checksum-valid |
| 50 | `checksum_failed` → **wrong accept** | `surname` `KIRSCHNER` → `KIRSCHNEROMAR`, `given_names` → `C`, `personal_number` → `MH35XFOBRLN7LT` |
| 88 | `checksum_failed` → **wrong accept** | `surname` `VANTERPOOL` → `VANTERPOOLDMITRI`, `given_names` → empty |
| 98 | `checksum_failed` → **valid miss** | `document_number` `29OOGK7UO` → `290QGK7UO`, checksum-valid |

**No mover is outside `variant_valid`**, on any population. `general_valid` cannot move, because
its accepted text equals the full text. `exhausted` cannot move either, because its accepted
text is `None` and it falls back to the full text. Under `first-valid` nothing is ever held, so
`repair_unconfirmed`, `pass_cap` and `budget` with a held reading do not occur.

## Control

- **Observed: arm A equals the committed CI ledger** (`real-specimen-outcomes.jsonl`, CI
  `eeacdf5`, DATA `396b22f`) on the outcome of **all 261 documents**. It also equals the ledger
  on `names_exact` for all 261. Four documents took a different retry path locally, and none of
  them changed its outcome:
  - France ID 2020 back and front: `budget` in CI, `exhausted` locally.
  - Argentina `P0_ARG_2026`: `variant_valid` pass-09 in CI, `exhausted` locally.
  - Kuwait `P0_KWT_2023`: `variant_valid` pass-03 in CI, `general_valid` locally.

  This is the local-versus-CI `rten` float rounding and runner speed the README describes.
- **Observed: the arms differ only in the parse.** `retry_stop` and `retry_variant_id` are equal
  on all 261 real documents and all 150 synthetic ones. The dumped OCR text is byte-identical on
  170 of 170 real documents. `ocr_arms` is the default in both (`stop: first-valid`,
  `confirm_passes: 2`, `chargrid: off`). So the six real movers and five synthetic movers are
  not `rten` noise.
- **Observed: CI agrees.** The #535 `real-specimen-gate.yml` run
  [36331797441](https://github.com/ruledicaprio/SynthPass/actions/runs/36331797441) ran on head
  `65c7944` with DATA `396b22f`. It failed with `tier1_hits` 139 → 138,
  `document_number_mismatch` 0 → 1, `strict_hits` 12 → 14 (report-only) and
  `name_scorable_hits` 33 → 32. Its ledger diff is exactly one document: Cyprus `P0_CYP_2010`, hit →
  `document_number_mismatch`, read `<00000220` on pass-02 `variant_valid`. That is the same
  document, reading and aggregate counts as local arm B. CI's diff reports outcomes only, so
  the five local name movers can be reconciled only through the strict count. Local B gives
  12 + 3 − 1 = 14, which matches CI.

## Gate

B fails `real-specimen-gate.yml` against the committed baseline at `tolerance: 0` on two counts:

- `tier1_hits` 138 < 139;
- `document_number_mismatch` 1 > 0.

`checksum_failed`, `no_mrz_found`, `ocr_error` and `false_positive_mrz` are unchanged.
`documents`, `scored` and the three off-denominator buckets are unchanged, so the gate raises no
corpus warning. CI observed exactly this.

The owner reverted #535 instead of re-blessing. `main` returns to arm A, whose 261 outcomes match
the committed ledger, so the gate passes with no re-bless. See [What next](#what-next).

## What next

- **#535 is reverted.** `main` returns to arm A's Tier-1 behaviour. The handoff
  [#508](https://github.com/ruledicaprio/SynthPass/issues/508) asked for comes back only together
  with a stronger acceptance for a variant stop, such as a second agreeing pass or the
  concatenation's parse kept as a veto, and only once the #473 re-run below measures it.
- **[#536](https://github.com/ruledicaprio/SynthPass/issues/536): `mrz` refuses a document number
  that starts with `<`.** It still lands. The loop accepted Cyprus 2010's pass-02 reading before
  #535 as well: the committed ledger records the same pass-02 `variant_valid` stop. ICAO Doc 9303
  says a document number begins in the left-hand character position (Part 3, PDF p.28; Part 4,
  PDF p.25), so the reading is refused in any mode.
  - **Hypothesized:** with the revert, Tier 1 again parses the concatenation, where the `K` line
    validates, so Cyprus 2010 stays a hit while the loop itself runs on past pass-02. The gate on
    #536's pull request measures it.
- **[#537](https://github.com/ruledicaprio/SynthPass/issues/537): `K` recognized as `<` on the
  variant pass.** The hypothesis is that the line box clips the stem of the `K`. This is the
  recognition-side cause of the same document.
- **The synthetic losses return to arm A with the revert:** four refusals rather than two wrong
  accepts and two valid misses, and seed 20 correct again. What the five share is a variant stop
  accepted on one pass's lines, a single witness with no cross-pass check.
  - **The next lever is the #473 A/B (`SYNTHPASS_OCR_STOP=clean`), re-run on a build that carries
    both the handoff and the stronger acceptance.**
    [`retry-stop-ab-2026-09-26.md`](retry-stop-ab-2026-09-26.md) lists "a version that hands the
    parser only the accepted reading" as unmeasured; this A/B measured it at `first-valid` only.
  - That A/B needs the reports to record `damaged_recovery` per document. Neither harness
    records it today, so it is unknown which of the five were damaged-capture accepts.

**Amended 2026-09-27: the #473 re-run ran.** It ran on `55f4c0c`, which carries #548's
per-document damaged flags and ADR-0025's narrowed handoff. The list above is kept as written
before the run. Seeds 40, 50, 88 and 98 are damaged-capture accepts, and seed 20 is not.
Cyprus 2010 stays a hit with #536: the loop runs on to pass-04. Under `clean`, no hit is lost,
one synthetic read is gained and one real name is lost. See
[`retry-stop-rerun-2026-09-27.md`](retry-stop-rerun-2026-09-27.md).

## A private specimen (class only)

One more real case points the same way. It is a private TD1 identity card, back side, with one
line-2 cell physically missing under the expiry check digit and a crack through line 3. It is on
the gitignored private track and was measured locally on release builds of `9a639c6` and
`bd0dcd8` with `provider-bench --real-specimens --include-private --mrz-only`. There is no OCR
dump, because `provider-bench` refuses one for the private track.

- In both builds the loop accepted the same pass, pass-05 (`variant_valid`).
- **Before #535** Tier 1 refused the card: `checksum_failed`, on the expiry check digit.
- **With #535** it was a checksum-valid hit. The missing expiry cell was recovered correctly and
  marked unsupported, but the sex and both names were wrong. No check digit covers those cells.

## Invocations

Worktree `m6-dump`. Each arm was built after `git switch --detach <sha>`, with
`CARGO_INCREMENTAL=0`:

```
cargo build --release -p synthpass-bench --bin synthpass-bench --bin provider-bench
# A 9a639c6: 450 s from an empty target/; B 0a8f7ff: 161 s on top of A
# sha256 A synthpass-bench b6fb4b61f696ae87129e772cca2d80765f67efa9a4b5c9aeaba8923db9637280
#          provider-bench  8797751fd926bc8586a1093dd6036974f54e250b616d99dacca33da50b01cb34
#        B synthpass-bench eb6079d8642a9ff85d30564e773a2237ecc60cb75ef501d3222fb3bd1a53d379
#          provider-bench  febf6a0fe58d12752b75f11117429b54adfea260cc85a4ddd0e579bd7c15b496
```

Both binaries resolve `samples/` and the `.rten` models from the build-time
`CARGO_MANIFEST_DIR`. In both arms that is `m6-dump`, so both read the same untracked corpus and
the same models. The runs used the copied binaries, one arm after the other: B at 16:03–16:35
UTC, A at 16:35–17:08 UTC, with the worktree's `HEAD` at the arm's own sha. A 30-second sampler
found no competing `cargo`, `rustc`, bench or `native_ocr_e2e` process during any run. Every run
had `SYNTHPASS_OCR_MAX_PASSES` and `SYNTHPASS_OCR_MODEL_DIR` unset, and no other
`SYNTHPASS_OCR_*` variable set.

```
env -u SYNTHPASS_OCR_MAX_PASSES -u SYNTHPASS_OCR_MODEL_DIR <arm>/provider-bench --real-specimens \
  --mrz-only --progress --dump-ocr --dump-ocr-hits --out 508-real-<arm>.json
env -u … <arm>/synthpass-bench --document-type td3 --profile clean --count 100 --seed 0 --out 508-td3x100-<arm>.json
env -u … <arm>/synthpass-bench --count 50 --seed 0 --profile clean --out 508-m4x50-<arm>.json
env -u … <arm>/synthpass-bench --document-type td3 --profile clean --count 1 --seed <20|40|50|88|98> --dump-ocr
python tools/synth_ab_diff.py 508-td3x100-armA.json 508-td3x100-armB.json
```

The real invocation adds `--dump-ocr --dump-ocr-hits` to the gate's. The dump is written after
each document is classified, and arm A's 261/261 match with the ledger shows the flags moved
nothing. The dump carries no `document_number_mismatch` row, so Cyprus 2010's pass-02 lines
come from arm A's dump. Its raw text is the same in both arms.

## What this does not claim

- **Not a CI measurement of the synthetic populations.** They are local only. The real-corpus
  move is CI-confirmed. The five real name movers are confirmed only through the strict count.
- **Not a measurement of `65c7944`.** The #508 commit was rebased onto `4732d9f` during the
  runs. The range-diff shows the same change in `synthpass-ocr`, `synthpass-imageprep` and
  `synthpass-bench`. The new base adds #533, an LLM prompt change, and #534, which reports OCR
  config overrides.
  - **Inferred: neither added change reaches Tier 1.** The CI run on `65c7944` agrees with local
    B on every count.
- **Not the effect under `clean`.** Only `first-valid` was run, where `repair_unconfirmed` never
  occurs.
- **The TD2 prefix of Cyprus 2020 is inferred.** `provider-bench` reports `mrz_format: TD2` and
  38 of 72 zone cells wrong, but not the parsed `document_type` or `issuing_country`. A TD2
  layout over `CYPPOLITIS…` would give document code `CY` and issuer `PPO`.
- **"Names exact" is the harness's verdict.** India and Somalia still carry a single `<`
  separator under B, and the harness's name split scores them exact.
- **TD3 only on the synthetic side.** TD1, TD2, MRV-A and MRV-B were not run.

## Rejected on the way

- **"#508 only restores hits".** The replay's six documents were `clean`-mode losses. At the
  default stop mode, the fix restores no hit on any population. It trades four synthetic
  refusals for two wrong accepts and two valid misses, turns one correct read wrong, and turns
  one real hit into a checksum-valid wrong document number.
- **Re-blessing to 138.** It would make a checksum-valid wrong document number the baseline.
  The owner reverted #535 instead.
- **Fixing forward with #536 alone.** It reaches only Cyprus 2010. It would leave `main` with
  three more synthetic TD3 wrong accepts, one more M4 wrong accept, and Cyprus 2020 read as TD2.
- **Rebuild noise as the explanation.** Both arms took the same retry path on all 411 documents,
  and the OCR text is byte-identical on all 170 dumped documents. CI reproduced the real move.
