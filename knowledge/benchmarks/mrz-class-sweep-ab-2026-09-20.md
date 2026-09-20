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
| hit → hit | 139 | all unchanged hit records |
| miss → miss | 122 | all unchanged miss/refusal records |
| miss → hit | 0 | none |
| hit → miss | 0 | none |

No document changed `miss_reason` without changing outcome, and no document changed any other
reported semantic result between off and on. The known Croatia target remained a
`checksum_failed` miss in all three arms.

The placebo control was identical to off in both outcome counts and per-document semantic records.
The on report was also identical to both controls, despite correctly recording
`mrz_class_sweep_arm: "on"`.

## Hypothesized

This is a null measurement for the real corpus: the arm produced no observed gains and no observed
regressions. Because the treatment report identifies itself as `on` while matching both controls,
the run gives no evidence that the sweep improves or harms these documents. It also does not justify
claiming that the repair mechanism is ineffective on its motivating synthetic shape; that question
requires a separate controlled synthetic case. The real-specimen decision rule `on > control >= off`
is not met.

The equal on/control/off outputs are the signature of an arm with no observable effect in this
population. Keep the baseline unchanged and do not promote the arm based on this run.
