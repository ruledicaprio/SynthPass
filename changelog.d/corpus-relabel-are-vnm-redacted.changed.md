- **Two passport pages relabelled `_no_mrz` -> `_redacted_mrz`** -- the United Arab Emirates 2018
  page (zone under a full-width black bar) and the Vietnam 2022 page (two printed MRZ lines, only
  the data blurred), found by the T15 cover-like report. As labelled, a checksum-valid read on the
  Vietnam page would have been a `false_positive_mrz` build failure; under `_redacted_mrz` it is
  absorbed by the redacted rung. The pre-migration baseline measured 51 `no_mrz_expected` and 37 `redacted_mrz` specimens, both outside the scored denominator. The relabel is expected to produce 49 and 39 respectively; those post-relabel bucket counts remain unmeasured until an actual benchmark run and baseline re-bless. `samples/README.md` now spells out the
  convention: `_redacted_mrz` for a zone that is present but blacked out or blurred,
  `_redacted_no_mrz` for a redaction that removed the zone entirely, plain `_no_mrz` for a document
  with no zone by design.
