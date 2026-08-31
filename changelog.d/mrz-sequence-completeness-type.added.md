- **`mrz` gains `SequenceCompleteness`, one type spanning both arms of a `find_and_parse` result
  (`mrz`, additive, no breaking change).** `Checks`/`DateCompleteness` (on a successful `MrzData`)
  and `MrzError::IncompleteSequence` (on a failed one) were two differently-shaped vocabularies a
  caller had to branch on separately to answer "how complete is this read." New
  `SequenceCompleteness::Complete { checks, date_of_birth }` /
  `SequenceCompleteness::Partial { format, lines_found, lines_expected }`
  (`#[non_exhaustive]`), plus `MrzData::sequence_completeness()` and
  `SequenceCompleteness::from_parse_result(&Result<MrzData, MrzError>) -> Option<Self>`.

  Deliberately computed, not stored: `MrzData` keeps its existing `checks`/
  `date_of_birth_completeness` fields exactly as they are (both are consumed directly by
  `mrz-wasm`, `synthpass-core`, `synthpass-die` today, so their shape is untouched) —
  `sequence_completeness()` is a view over them, not a second, independently-stale copy. Does
  **not** fold in `synthpass_core::fusion::check_line1_integrity`'s verdict, which stays in
  `synthpass-core` on purpose (no check-digit backing, needs cross-crate context).

  `mrz-wasm`'s browser demo (`parse_mrz_text`) now injects `sequence_completeness` into its JSON
  output alongside the existing `valid` field, following the same manual-injection pattern that
  field already used — verified with a real `cargo build -p mrz-wasm --target
  wasm32-unknown-unknown`, not just a native check.

  `cargo semver-checks -p mrz`: 196 checks, 0 breaks, "no semver update required." Six new
  regressions in `crates/mrz/tests/sequence_completeness.rs`. Chunk 5 of
  `knowledge/MRZ_SEQUENCE_COMPLETENESS.md`.
