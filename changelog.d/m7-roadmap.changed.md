- **Roadmap: M6's "plugin architecture" narrows to declarative *layout* plugins, and a new
  [M7 — Document Intelligence Engine](knowledge/ROADMAP.md) takes over the provider interface.**
  M6 already contained M7's work without the interface to hang it on: its deliverables listed
  "plugin architecture" and its DoD required that "a third-party plugin builds against a stable
  interface", while nothing in the milestone would produce one.

  M7 is **built ahead of M6**, and the roadmap's "linear, M1 through M6 — no parallel tracks"
  statement is amended to record the exception rather than quietly break it. The reason is a
  dependency inversion: M6 adds TD1/TD2/MRVA/MRVB plus the barcode decoder driving licences need,
  and today's extraction path is a hardcoded two-tier branch. Adding those formats as branches
  first and refactoring them into providers afterwards is the same work twice, with the second
  pass landing on shipped behaviour. Alternatives considered and rejected — including keeping
  strict order — are in
  [`ADR-0002`](knowledge/decisions/ADR-0002-provider-model-before-layout-plugins.md). Milestones
  stay numbered by dependency, not build date.

  M7's section carries explicit **non-goals**, so they do not get re-litigated: no
  vision-language model in v1.3.0 (`Capability.vision` is `false` for every registered provider —
  the honest claim is that the interface exists and has zero vision implementations, with
  `llama-cpp-2` staying pinned at `0.1.151`/`sampler` and Moondream deferred to a v1.4.0 spike);
  no hardware auto-recommendation feature; no change to the reported confidence model; and no new
  public API in `crates/mrz`.
