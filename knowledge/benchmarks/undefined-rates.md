# Undefined provider-bench rates

A zero denominator means no measurement: terminal rates display `n/a` and JSON
rates are `null`. A measured zero with a positive denominator remains `0.0`.
The provider report's tagged status also explains why Tier-1 or strict-name
rates are unavailable.

| Rate | Denominator |
| --- | --- |
| Tier-1 hit rate (`read_ok_rate` in flattened history) | Documents other than `no_mrz_expected`, `redacted_mrz` and `checksum_failed_specimen` |
| Strict Tier-1 hit rate | Documents in that scored population with truth for both name fields |
| Names exact among hits | Tier-1 hits with truth for both name fields |
| Field match rate | Field comparisons with available truth |
| Mean CER | Character-error-rate observations for fields with available truth |
| Per-field mean CER | Observations with truth for that specific field |
| Unsupported-assertion rate, overall and each anchor bucket | Nonempty asserted field values in that population |
| JSON-repair fallback rate | Documents processed by the nondeterministic provider |

These populations differ. A labelled miss contributes to strict Tier-1 accuracy,
but it contributes nothing to the denominator for names among accepted hits.
An off-denominator MRZ document can still carry field truth or assertions; those
rates remain measurable when their own denominators are positive.

`bench-chart` skips null points in every rate panel. A null-only series has no
bar; a measured zero remains a real plotted point or zero-height bar. The history
flattening script preserves nulls without a workflow change.

## Historical driving-licence rows

Read-only inspection on 2026-09-26 covered all 26 rows in
`results/driving_license-bench/history.jsonl` on `origin/bench-data`:
15 rates were zero, 9 were null, and 2 older rates were one. The rows carry total
documents and labelled documents, but no scored denominator or exclusion counts.
Neither a three-document run nor zero labels proves that no document was scored.

The chart reader therefore preserves historical numeric values. It cannot safely
identify fabricated zeroes from those rows alone. Null points from corrected runs
are skipped; historical repair would require separately reviewed evidence and a
data-migration decision. No historical rows are rewritten by this change.
