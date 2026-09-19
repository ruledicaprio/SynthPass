- **Per-field MRZ mismatch attribution in `provider-bench`'s OCR dump.** Each `checksum_failed`/
  `no_mrz_found` row with both ground truth and a resolved format now carries
  `field_mismatch_counts` (differing characters per ICAO field, e.g. `{"optional_data_2": 7,
  "composite_cd": 1}`), `field_mismatch_positions` (0-based columns per 1-based line), and
  `field_mismatch_coverage` (whether a check digit — dedicated, composite-only, or none — would
  have caught an error in each differing field). All three are `Option`, absent under the same
  conditions as the existing `zone_mismatch`, plus when the format itself is unknown. Positions,
  field names and coverage states only — never a character value. The field-layout table (TD1,
  TD2, TD3, MRV-A, MRV-B) is pinned against `crates/mrz`'s actual parser behaviour by a
  mutate-and-reparse test, not duplicated as a second, driftable copy of its offsets.
