- **`mrz` gains `Checks::is_only_composite`, and `synthpass-die`'s two Chunk 7 gates stop
  encoding the same predicate twice (no behaviour change).** `MrzReader` gates partial-read
  surfacing on `mrz::Checks::only_composite_failed`, while `RoutingPolicy` gates its half on
  `Evidence::checksum_partially_valid` — which re-derived the identical comparison as
  `mrz_failed == [Field::Composite]`. The two are required to be enabled *together* for the
  gated experiment to change any routing decision, so two independent copies of the test were
  a standing invitation to drift. `Checks::is_only_composite(&[Field])` is the single
  implementation both now use; `Evidence` keeps `mrz_failed: Vec<Field>` (a PII-free
  projection) and defers to it.

- **`default_policy_is_exactly_the_v1_2_0_predicate` now sweeps `mrz_failed`, and its doc
  comment no longer claims more than it proves.** The test enumerates the evidence space to
  show `RoutingPolicy::default()` accepts exactly `mrz_found && mrz_checksums_valid`, but
  every case was built with `..Evidence::default()`, leaving `mrz_failed` empty — so "the
  Chunk 7 opt-in is off" and "the opt-in was never offered a case it could act on" were
  indistinguishable. Chunk 7's commit message cited this test as proof that flag-off
  behaviour was byte-identical; that proof actually came from the `&&` short-circuit in
  routing clause 2.

  Sweeping `mrz_failed` over `[]`, `[Composite]`, and `[DateOfBirth, Composite]` makes an
  accidental flip of `accept_composite_only_failure`'s default fail *in this test* —
  verified by flipping it and confirming the failing case is `mrz_failed: [Composite]`, an
  input the previous sweep never generated. The doc comment now also states what the sweep
  does **not** do: it never reaches `Evidence::checksum_partially_valid`, because clause 2
  short-circuits before consulting it, so that function's own behaviour stays pinned by
  `checksum_partially_valid_is_true_only_for_a_composite_only_failure` in `evidence.rs`.
