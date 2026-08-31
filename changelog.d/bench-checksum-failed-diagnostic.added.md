- **`synthpass-bench`/`provider-bench` report which check digit(s) failed on a `checksum_failed`
  miss.** `checksum_failed` has been the largest real-specimen miss category (66, ahead of
  `no_mrz_found`'s 51 in the 2026-08-16 real-specimen run), and was one undifferentiated count —
  actionable prioritization needed a breakdown, not a bigger number. `MissReason::ChecksumFailed`
  now carries `failing: Vec<&'static str>` (`mrz::Field::as_str()` names, e.g. `["composite"]` or
  `["document_number", "composite"]`), sourced from `mrz::Checks::failed()`.

  Both bench binaries surface it: `synthpass-bench`'s JSON report gains a per-document
  `failing_checks` array plus a new console "checksum_failed, by failing field" summary;
  `provider-bench`'s `DocumentDetailReport` gains the same array, and its real-specimen summary
  gets the matching breakdown line. Both are purely additive — existing `miss_kind`/`miss_reason`
  strings, and the JSON shape everything else in either report already produces, are unchanged.

  No accuracy change: this is instrumentation only. First chunk of
  `knowledge/MRZ_SEQUENCE_COMPLETENESS.md`, the M6 sub-track every later chunk's prioritization
  reads off.
