- **`Argentina_Passport_Specimen_P0_ARG_2026_mrz` (and its `_mrz_blur` variant) are
  now scored off the Tier-1 denominator** as `checksum_failed_specimen`. Their
  printed MRZ is non-conforming by construction — sex `M` where the VIZ says `F`,
  wrong dates, a document-number check digit that does not compute — so a
  byte-perfect read still fails, the same as the 16 `TEMPLATE`/all-zeros specimens
  already excluded. Each gained a reviewed `samples/ocr_fixtures/` fixture. The
  headline hit rate becomes **118 / 142 = 83.1%**: the count drops by one because
  `ocrs` had been misreading the fake printed zone into a *different*,
  checksum-valid string, counted as a hit only because the document had no ground
  truth to contradict it — the same failure the 2026-09-09 denominator correction
  named for MRZ-less fronts.
  [`denominator-bucket-a-2026-09-10.md`](knowledge/benchmarks/denominator-bucket-a-2026-09-10.md).
