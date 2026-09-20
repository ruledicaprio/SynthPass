# MRZ class-sweep ceiling and date-veto analysis

**Date:** 2026-09-20 · **MAIN:** `bc78ef7` · **DATA:** `396b22f3b97ee36275a5329667ee37cbd0449b40` · **Evidence:** Static analysis of the 2026-09-19 attribution dump and the three C37 release reports · **Status:** current

This record scopes the class sweep to the 12 scored misses: 10 `checksum_failed` documents and the
two `no_mrz_found` documents. The checksum-failed rows below are `CF-01` through `CF-10` in dump
order. The labels are deliberately anonymous; the table records only the format, candidate damaged
fields, and properties relevant to the repair.

## Per-document classification

“Uniform” means every mismatched cell in the candidate field has the same glyph-class mapping.
“Two+” counts occurrences across the field and its check digit, as required by
`solve_class_sweep`.

| row | format | candidate field(s) | covered | uniform | two+ | dates | bucket | reason |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :---: | :--- |
| CF-01 | TD1 | composite | yes | no | yes | well-formed | C | mixed damage; no single class substitution |
| CF-02 | TD1 | document number | yes | yes | yes | **placeholder** | A | sweepable, then vetoed by date validity |
| CF-03 | TD1 | expiry date | yes | yes | no | well-formed | C | one damaged cell, below the two-occurrence threshold |
| CF-04 | TD3 | document number | yes | yes | no | well-formed | C | one damaged cell, below the two-occurrence threshold |
| CF-05 | TD3 | personal number | yes | no | yes | well-formed | C | candidate damage is not one uniform class |
| CF-06 | TD3 | document number | yes | yes | no | well-formed | C | one damaged cell; other damage is outside the candidate field |
| CF-07 | TD3 | document number, personal number | yes | no | yes | well-formed | C | multiple confusable mappings in the candidate fields |
| CF-08 | TD3 | document number | yes | yes | no | well-formed | C | one damaged cell; other damage is outside the candidate field |
| CF-09 | TD3 | personal number | yes | no | no | well-formed | C | one cell and non-uniform surrounding damage |
| CF-10 | TD1 | document number, dates | no | no | yes | malformed, not a confirmed placeholder | C | damage includes fields with no checksum coverage |

The covered/uniform/two-plus tests are applied to the field that could drive a checksum repair;
unrelated field damage remains a reason for bucket C when it makes the candidate non-uniform or
unscorable. Dates use TD1 line 2 spans `[0,6)` and `[8,14)`, and TD3 line 2 spans `[13,19)` and
`[21,27)`.

The two `no_mrz_found` rows are bucket D. They have no fallback `MrzData`, so `damaged_pass` and the
class sweep are unreachable for them.

## Ceiling

| bucket | count | interpretation |
| :--- | ---: | :--- |
| A — sweepable and date-vetoed | **1** | the one document the sweep could move if the date veto changed |
| B — sweepable and accepted today | **0** | consistent with the observed zero crossings |
| C — not sweepable | **9** | fails coverage, uniformity, or the two-occurrence requirement |
| D — unreachable | **2** | no fallback MRZ was found |

The maximum number of scored documents this class sweep could ever move on this corpus is therefore
**one**. The arm's observed 0 crossings is consistent with that ceiling: its sole candidate is
discarded downstream by `accept_damaged` because its dates are not well-formed.

## The broader damaged-pass gate

Across all 12 scored misses, only **one** document has confirmed placeholder dates: the TD1 row with
`000000` for both date fields. The other nine checksum-failed rows have well-formed dates or, in one
case, malformed spans caused by the fixture's recorded layout rather than a confirmed placeholder;
the two no-MRZ rows have no parsed dates at all. Thus the date clause excludes one of twelve scored
misses from every current and future `damaged_pass` repair path, while the sweep itself has only that
same one-document ceiling.

Removing the date clause would be a bad trade if it admitted a second candidate without the sweep's
coverage, uniformity, and two-occurrence evidence: the measured ceiling is one, so weakening a guard
that rejects ambiguous damaged reads would buy no additional scored document on this corpus while
removing the protection that `damaged_pass` applies to every repair strategy.
