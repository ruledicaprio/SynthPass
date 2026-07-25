- **`synthpass-die::routing`: the type that turns routing `Evidence` into a spend decision.**
  `RoutingPolicy::decide(&Evidence) -> Decision` — `Decision` is either `Accept` or
  `Escalate { reason: EscalationKind, budget: CostClass }`. **Nothing consumes this yet** —
  `synthpass-pipeline` keeps its inline Tier-1 path, so this PR changes no behaviour; wiring it in
  is a later chunk.

  The default policy (`RoutingPolicy::v1_2_0_compatible()`) reproduces v1.2.0's routing exactly:
  escalate if and only if the MRZ did not parse or a check digit did not verify. Every opt-in
  signal — `escalate_on_missing_fields`, `escalate_on_line1_flagged`, `sanity_floor` — defaults off,
  because introducing this crate must not itself be a behaviour change. A signal gets enabled only
  once the benchmark harness (a later PR) has produced data saying what it actually predicts.
  `sanity_floor` in particular stays `None`: `text_sanity` is a character-plausibility proxy, not a
  model confidence, and no threshold on a proxy can be better than the proxy itself.

  `decide`'s clause order is normative and documented as such: MRZ-not-found is checked before
  MRZ-checksum-failed, because a blank page — no MRZ-shaped text anywhere — would otherwise be
  reported as a checksum failure, which is false. `Evidence::blind_positions` is never read here,
  matching its observed-only status from the MRZ reader: it is uncalibrated, and routing on it
  before anything has measured what it predicts would be inventing a threshold rather than acting
  on one.

  `Decision` derives no `Serialize`, matching `Evidence` and `Reading` — `EscalationKind` (already
  serializable, in `synthpass-core`) remains the only wire-legal projection of a routing decision.
