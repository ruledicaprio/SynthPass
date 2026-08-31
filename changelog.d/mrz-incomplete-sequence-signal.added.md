- **`mrz` distinguishes "found part of an MRZ" from "found nothing at all" (`mrz`, additive, no
  breaking change).** `find_and_parse`/`find_and_parse_with` previously collapsed both cases into
  the same `MrzError::NotFound` — a line matching a format's document-code prefix with no
  companion line anywhere in the text looked identical to genuinely MRZ-less input. New variant
  `MrzError::IncompleteSequence { format, lines_found, lines_expected }` is now returned instead,
  whenever the scan saw a shape-matching line 1 but nothing (not even a partial, checksum-failed
  reading) ever came of it. `lines_found` is always `1` today: only line 1's document-code prefix
  is a reliably shape-checkable signal in isolation, the other lines carry no distinguishing
  prefix.

  Purely additive bookkeeping alongside the existing scan — it never changes which `MrzData` (or
  whether a fully-valid one) is returned, only which error variant comes back when nothing parses
  at all. `#[non_exhaustive]` on `MrzError` already protects every caller in this workspace (and
  any external one) from breaking on the new variant; verified with a full `cargo check
  --workspace`. Same-binary check: `cargo test -p mrz`'s full 213-test suite is unchanged, bit for
  bit, before and after.

  Six new regressions in `crates/mrz/tests/incomplete_sequence.rs`, one per format. First step of
  `knowledge/MRZ_SEQUENCE_COMPLETENESS.md` chunk 4-5's line-count signal; chunk 5's unified
  `SequenceCompleteness` type will build on `lines_found`/`lines_expected` here.
