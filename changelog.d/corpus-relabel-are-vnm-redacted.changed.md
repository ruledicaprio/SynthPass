- **Two passport pages relabelled `_no_mrz` -> `_redacted_mrz`** -- the United Arab Emirates 2018
  page (zone under a full-width black bar) and the Vietnam 2022 page (two printed MRZ lines, only
  the data blurred), found by the T15 cover-like report. As labelled, a checksum-valid read on the
  Vietnam page would have been a `false_positive_mrz` build failure; under `_redacted_mrz` it is
  absorbed by the redacted rung. `no_mrz_expected` 51 -> 49 and `redacted_mrz` 37 -> 39 in that pre-migration
  measurement; both were outside the scored denominator. These bucket counts are historical
  evidence, not a post-migration benchmark result. `samples/README.md` now spells out the
  convention: `_redacted_mrz` for a zone that is present but blacked out or blurred,
  `_redacted_no_mrz` for a redaction that removed the zone entirely, plain `_no_mrz` for a document
  with no zone by design.
