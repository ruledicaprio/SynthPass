# Where the real-specimen gate spends its time, by outcome class

**Date:** 2026-09-17 · **MAIN:** `b2a0afd` · **DATA:** `469a4ee7723148917cf023f1a7ab2af80a0ef7d3` · **Evidence:** Observed · **Status:** current

**2026-09-17.** [ADR-0010](../decisions/ADR-0010-benchmark-cost-split-by-role.md)'s step 5 —
measure the gate's cost after ADR-0008 chunk 2 before building the role split. Read-only: no
code, no corpus file and no baseline was touched; the numbers are read from a CI artifact.

**Observed.** CI run 35169452610 (`real-specimen-gate.yml`, MAIN `b2a0afd`, `samples-data`
`469a4ee7723148917cf023f1a7ab2af80a0ef7d3`, 2026-09-17), invocation
`provider-bench --real-specimens --mrz-only --progress --assert-baseline`, the `mrz` provider,
261 documents. Per-document OCR time is `documents_detail[].ocr_ms` (added by #315), summed by
the outcome class its `miss_reason` maps to.

| Outcome class | documents | OCR ms (sum) | share | mean per document |
| --- | --: | --: | --: | --: |
| `redacted_mrz` | 39 | 776,279 | 28.6 % | 19.9 s |
| Tier-1 HIT | 140 | 722,899 | 26.6 % | 5.2 s |
| `no_mrz_expected` | 49 | 657,247 | 24.2 % | 13.4 s |
| `checksum_failed_specimen` | 19 | 287,666 | 10.6 % | 15.1 s |
| `checksum_failed` | 10 | 170,636 | 6.3 % | 17.1 s |
| `no_mrz_found` | 4 | 104,171 | 3.8 % | 26.0 s |
| **Total** | **261** | **2,718,898** (45.3 min) | 100 % | 10.4 s |

By role: the **scored population** (154 documents — hits plus the two scored miss kinds) costs
997,706 ms = **16.6 min = 36.7 %**; the **off-denominator population** (107 documents) costs
1,721,192 ms = **28.7 min = 63.3 %**. The serial OCR total matches the job's observed 39–49 minute
wall-clock, so `ocr_ms` is the whole cost to within the job's fixed overhead.

## What this does and does not say

- **Cost tracks failure, not document count.** A document that yields a checksum-valid MRZ breaks
  out of the retry loop early (5.2 s mean); one that never does runs the whole chain (13–26 s).
  The three off-denominator classes never yield one by definition, so 41 % of the documents pay
  63 % of the time to produce counts the headline rate excludes.
- **ADR-0010's model (−74 % without the off-denominator set) overstated the saving; the measured
  figure is −63 %.** The gap is the scored misses, which pay the full chain too and stay in the
  per-PR gate. The direction, and the decision's shape, are unchanged.
- **Not a timing benchmark of the reader.** These are CI-runner numbers under the retry loop's
  time budget (`SYNTHPASS_OCR_MAX_SECONDS`), with `retry_budget_hit` recorded per document; local
  numbers differ. They answer where the gate's time goes, not what latency a user would see.
- **Not a reason to sample.** A uniform sample removes cheap and expensive documents in
  proportion; ADR-0010's rejection of random sampling stands on this data.

Reproduce: download the run's `real-specimen-gate-report` artifact, then sum
`documents_detail[].ocr_ms` by outcome class for `provider_id == "mrz"`.
