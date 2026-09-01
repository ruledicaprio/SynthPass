- **`default_policy_is_exactly_the_v1_2_0_predicate` now sweeps `mrz_failed`, and its doc
  comment no longer claims more than it proves.** The test enumerates the evidence space to
  show `RoutingPolicy::default()` accepts exactly `mrz_found && mrz_checksums_valid`, but every
  case was built with `..Evidence::default()`, leaving `mrz_failed` empty — so the sweep's
  "whole space" claim excluded a signal that `Evidence::observe_mrz` populates on every parsed
  record. `mrz_failed` is now swept over `[]`, a single-digit failure, and a multi-field one,
  pinning that routing is provably *independent* of it — the same property the sibling
  `blind_positions_never_influences_a_decision` states for the other signal the policy does not
  consult.

  The dimension was added while a since-reverted opt-in (`accept_composite_only_failure`,
  Chunk 7) *did* consult `mrz_failed`, where its absence made "the opt-in is off"
  indistinguishable from "the opt-in was never offered a case to act on". That opt-in is gone;
  the dimension stays, because it costs one loop and any future signal reading `mrz_failed`
  starts out covered rather than silently uncovered.

  The de-duplication of the composite-only predicate that originally shipped alongside this
  (`mrz::Checks::is_only_composite`, added so `Evidence::checksum_partially_valid` and
  `Checks::only_composite_failed` could not drift apart) is **withdrawn**: the measurement in
  `--escalation-report` rejected Chunk 7, both of those functions are reverted, and a helper
  whose only two callers are gone would be dead public API on a published crate. Neither
  function ever reached a release — `mrz`'s published version is still 0.6.4 — so nothing
  user-visible was added and then removed.
