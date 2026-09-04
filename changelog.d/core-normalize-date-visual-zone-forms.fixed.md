Two more printed date forms are now read. Whitespace joins `-`, `.` and `/` as a separator, so
`01 10 1990` normalises like `01.10.1990` did; and a date with a named month may now carry a
two-digit year (`22 FEB 78`), resolved by *position* — the digits before the month are the day,
those after are the year — since neither token's width can tell them apart. Because 1978 and 2078
are only distinguishable by which field the date came from, the two-digit form is accepted by the
new `normalize::date_of_birth` / `normalize::date_of_expiry` entry points, which route the
century decision through `mrz::expand_date`'s audited pivot; the field-agnostic `normalize::date`
still declines it rather than guessing. Positional reading runs only after the existing strict
parser has refused, so no date that already normalised can change.

Measured on the 2026-09-04 parity corpus: **14 of the 70 date misses were the model reading the
printed date correctly while the normalizer discarded it** — overall Tier-2 field accuracy
48.1% → 52.5%. See `knowledge/benchmarks/normalize-date-forms-2026-09-04.md`.
