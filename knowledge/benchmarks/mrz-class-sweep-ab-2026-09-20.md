# MRZ class-sweep A/B on real specimens

**Date:** 2026-09-20 · **MAIN:** `bc78ef7` · **DATA:** `396b22f3b97ee36275a5329667ee37cbd0449b40` · **Evidence:** Observed (three-arm release `provider-bench --real-specimens --mrz-only` run) · **Status:** current

This measurement compares the field-scoped MRZ class sweep on the same release binary and the same
261 real specimens. The three runs changed only `SYNTHPASS_MRZ_CLASS_SWEEP`; no baseline was written.
The reports remain in `artifacts/ab-sweep-off.json`, `artifacts/ab-sweep-control.json`, and
`artifacts/ab-sweep-on.json`.

## Observed

The reports self-identify the measured arms:

| run | `mrz_class_sweep_arm` |
| :--- | :--- |
| off | `off` |
| placebo | `control` |
| treatment | `on` |

All three runs processed 261 documents and reported **140/152 Tier-1 hits (92.1%)**. Their miss
buckets were identical: 10 `checksum_failed`, 2 `no_mrz_found`, 19 `checksum_failed_specimen`,
39 `redacted_mrz`, and 51 `no_mrz_expected`. Strict name results and field-level summary metrics
were also identical.

The required per-document off→on crossing is:

| off → on | count | documents |
| :--- | ---: | :--- |
| hit → hit | 140 | all unchanged hit records |
| miss → miss | 121 | all unchanged miss/refusal records |
| miss → hit | 0 | none |
| hit → miss | 0 | none |

No document changed `miss_reason` without changing outcome, and no document changed any other
reported semantic result between off and on. The known Croatia target remained a
`checksum_failed` miss in all three arms.

The crosstab is keyed by each report's unique `asset_id`, rather than `name`. The 261 records contain
259 distinct names: the Türkiye JPG/WEBP pair and the Azerbaijan JPG/WEBP pair share names. Name-keying
silently merges those assets and produces the misleading 139/122 split.

The placebo control was identical to off in both outcome counts and per-document semantic records.
The on report was also identical to both controls, despite correctly recording
`mrz_class_sweep_arm: "on"`.

## Execution proof and veto finding

The treatment report identifies `mrz_class_sweep_arm: "on"`; that value reaches the parser through
`synthpass_die::mrz_parse_options()` at the real-specimen parse call. Croatia is a
`checksum_failed` record with `mrz_found = true`, which means a parsed `MrzData` with failing check
digits reached the `fallback` path. `class_sweep_pass` is the first statement inside `damaged_pass`,
gated only on `opts.class_sweep`. Thus the sweep executed on the canary. The two `no_mrz_found`
documents (France and Italy) have no fallback and are outside its reach by construction; the
addressable denominator for this repair was 10, not 12.

Croatia's recovered line 1 contains ten letter `O` characters where ground truth has ten digit `0`
characters, including the document-number check-digit cell. Sweeping `O`→`0` produces a unique,
check-digit-valid result. The result is then discarded by `accept_damaged`, which requires
`data.valid() && data.validity(..).dates_well_formed`. Croatia's line 2 is read perfectly and its
printed dates are genuinely `000000` and `000000`; `Date::is_well_formed` rejects month/day `00`, so
the correct repaired reading never reaches `hits`.

I checked all 29 dumped misses with ground truth using the format-specific spans (TD1 line 2
`[0,6)` and `[8,14)`; TD3 line 2 `[13,19)` and `[21,27)`). Three documents have confirmed
placeholder dates: Croatia (`000000`/`000000`), Türkiye ID (`123456`/`123456`), and India's
expiry (`230000`). Four other apparent malformed values came from misaligned format/line extraction,
not from invalid dates, and are not counted.

Croatia is therefore a real-corpus instance of ADR-0019's hypothetical where the date value is
complete but `dates_well_formed` is false.

## Hypothesized

This is a null measurement for the real corpus: the arm produced no observed gains and no observed
regressions. Because the treatment report identifies itself as `on` while matching both controls,
the run gives no evidence that the sweep improves or harms these documents. It also does not justify
claiming that the repair mechanism is ineffective on its motivating synthetic shape; that question
requires a separate controlled synthetic case. The real-specimen decision rule `on > control >= off`
is not met.

The equal on/control/off outputs are the signature of an arm with no observable effect in this
population. Keep the baseline unchanged and do not promote the arm based on this run.
