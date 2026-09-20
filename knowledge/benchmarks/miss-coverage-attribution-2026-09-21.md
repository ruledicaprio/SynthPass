# Miss coverage attribution

**Date:** 2026-09-21 · **MAIN:** `8480772` · **DATA:** `396b22f3b97ee36275a5329667ee37cbd0449b40` · **Evidence:** Observed (release `provider-bench --real-specimens --mrz-only --dump-ocr`, baseline reproduced at 140/152) · **Status:** current

This analysis uses the 31-record redacted coverage input from the baseline-reproducing run. The input
contains only field names, mismatch positions and counts, format, and miss reason; no OCR text or MRZ
zone is used here.

## Independent span check

The independent ICAO derivation was compared with the current `mrz_field_layout` tables. All **65 of
65 spans** agreed, across TD1 (14), TD2 (13), TD3 (14), MRV-A (12), and MRV-B (12), including their
coverage classes. There is no span disagreement to resolve.

## Cells by coverage class

Every position that the span map could place was assigned its field's coverage class. The six records
without `field_mismatch_positions` are counted separately below and do not contribute invented cells.

| format | OwnCheckDigit | CompositeOnly | Uncovered | placed cells |
| :--- | ---: | ---: | ---: | ---: |
| TD1 | 32 | 21 | 99 | 152 |
| TD2 | 0 | 0 | 0 | 0 |
| TD3 | 246 | 6 | 261 | 513 |
| MRV-A | 6 | 0 | 42 | 48 |
| MRV-B | 0 | 0 | 0 | 0 |
| **total** | **284** | **27** | **402** | **713** |

The positioned damage in `Uncovered` fields is **402/713 = 56.4%**. This is the measured lower
bound on damage that checksum validation cannot see: the six no-position records may add cells, but
their locations cannot be inferred from this redacted input.

## Documents by worst class touched

Among the 31 records, 23 touched at least one `Uncovered` field, two touched only
`OwnCheckDigit` fields, and six had no positions to place. No record was classified as worst
`CompositeOnly` because every record with composite-only damage also touched an uncovered field or
had no placeable position.

## Fields ranked by mismatched cells

| field | coverage | cells |
| :--- | :--- | ---: |
| name | Uncovered | 347 |
| document_number | OwnCheckDigit | 99 |
| personal_number | OwnCheckDigit | 55 |
| date_of_expiry | OwnCheckDigit | 48 |
| date_of_birth | OwnCheckDigit | 48 |
| nationality | Uncovered | 27 |
| optional_data_2 | CompositeOnly | 17 |
| sex | Uncovered | 11 |
| document_number_cd | OwnCheckDigit | 10 |
| composite_cd | CompositeOnly | 9 |
| date_of_expiry_cd | OwnCheckDigit | 9 |
| date_of_birth_cd | OwnCheckDigit | 9 |
| document_code | Uncovered | 8 |
| issuing_country | Uncovered | 8 |
| personal_number_cd | OwnCheckDigit | 6 |
| optional_data_1 | CompositeOnly | 1 |
| optional_data | Uncovered | 1 |

## Second opinion and unplaceable positions

The dump's `field_mismatch_coverage` agreed with the independent span derivation for every field in
every row where positions were available. There are **zero disagreements**.

There were also **zero positions outside the declared format grids**. The six records without
positions are not silently treated as zero damage: they are two `no_mrz_found` records, three
`checksum_failed_specimen` records, and one `checksum_failed` record. Their redacted input contains
no coordinates from which a field or coverage class could be reconstructed.

## Unresolved

The exact all-cell uncovered share across all 31 records is unresolved because six records provide no
positions. The 56.4% figure is therefore a lower bound over the 713 placeable cells. No span or
coverage disagreement remains unresolved.
