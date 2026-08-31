- **`mrz` recovers a dropped line-1 filler on TD2 (`mrz`, no API change).** TD2 (2-line
  "official travel document" MRZ) shares TD3/MRV-A/MRV-B's OCR failure — the position-1 filler in
  `document_code` gets dropped outright, shifting `issuing_country` and everything after it one
  position left — but had none of their fix: TD2 shares TD1's genuine two-real-letter document-code
  family, and unlike TD1 has no line-1 check digit to arbitrate an unshifted reading against the
  as-read one.

  Fixed by reusing the issuing-country-name gate TD3/MRV-A/MRV-B's own drop-repair already ships
  (`country_name` on the *unshifted* reading's issuing-country slot), not by building a
  document-code dictionary as originally planned — ICAO 9303 Part 5/6 Note k makes the document
  code's second character issuer-discretionary, so no closed set of "known-legitimate" TD2 codes
  actually exists to arbitrate against. Measured on the synthetic TD2 corpus (`--profile clean
  --count 30 --seed 42`): hit rate unchanged (80.0% → 80.0%), `document_type` CER 100.00% → 26.67%,
  `issuing_country` CER 68.89% → 20.00%, records with a line-1 integrity finding among Tier-1 hits
  100% → 37.5%.

  New regressions in `crates/mrz/tests/td2_line1_repair.rs`. See
  `knowledge/MRZ_SEQUENCE_COMPLETENESS.md`'s Chunk 6 for the full design-pivot writeup.
