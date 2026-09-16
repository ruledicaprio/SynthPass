- **Every document in the scored `checksum_failed` bucket now carries a hand-transcribed printed
  zone.** Three specimens added by cohorts c03/c07/c09 (Germany `P0_D00_2024`, Hong Kong
  `P0_HKG_2007` and `P0_HKG_2019`) sat in the denominator with no ground truth, so nothing
  distinguished "our OCR misread it" from "the specimen prints a zone no correct read could
  validate". All three are now transcribed and verified — by hand against the ICAO 7-3-1 weights
  and by the repo's own parser — and each prints a zone whose five check digits validate.

  That **refutes** the recorded hypothesis that the published 91.2% was up to three documents
  pessimistic: none of the three is a non-conforming specimen, so none moves off-denominator.
  91.2% is not pessimistic; it is the rate. No bucket count changes.

  Measurement in `knowledge/benchmarks/denominator-t14-2026-09-16.md`, which also records the
  character-level attribution for Hong Kong 2007 (26 of 29 line-1 errors are `<` fillers read as
  letters; 12 of 15 line-2 errors are `0` read as `O` or `D`), confirms Germany 2024 as a genuine
  one-character miss on a sideways-stored page, establishes San Marino 2017 back as MRZ-less, and
  censuses nineteen fixtures whose `mrz_line` is not ICAO-exact. The reclassification, the
  untagged-filename fallback and the wall-clock retry budget are each scoped to their own
  follow-up.
