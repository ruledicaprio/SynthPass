- **Schema vocabulary for provider traces, and the first test that pins the v2 wire contract.**
  `synthpass-core` gains `v2::CoreField` (the ten ICAO fields as a value — `mrz::Field` covers only
  the five check-digited ones, so it cannot say `surname`), `v2::ProviderId`, `v2::EscalationKind`,
  `v2::PromptRef`, `v2::ExtractionTrace`, and an optional `ExtractionV2.trace` recording which
  providers ran, why anything beyond the first was consulted, and which prompt version produced a
  model-generated record. `ExtractionFields` gains `get(CoreField)` and `missing()`, which treat a
  whitespace-only value as absent — an empty string is not something a consumer can act on.

  `trace` is additive and optional: `schema_version` stays `2`, the key is omitted entirely unless
  populated, and records written before it existed still deserialize. **It carries no routing
  score, and cannot.** A decision to spend compute may rest on an uncalibrated heuristic; a number
  reaching a consumer alongside `confidence` may not (see
  [`project_principles.md`](knowledge/project_principles.md) §2) — the routing type has no
  `Serialize` impl, so the omission is enforced by the type system rather than by a comment.

  `fusion::FindingKind` + `Finding::kind()` + `Verdict::kinds()` add the fieldless projection of
  `Finding`. `Finding` carries real ICAO country codes read off a document — it derives `Zeroize`
  for that reason — which makes it safe inside the zeroized extraction JSON and unsafe in a log
  line or a metric label. Routing decisions get the payload-free form. `Finding::kind()` is
  exhaustive with no catch-all, mirroring `FieldConfidence::downgrade_flagged`: a new variant must
  be a compile error rather than silently mapping to a default.

  `Provenance` becomes `#[non_exhaustive]` at the enum, so a future producer kind is additive for
  Rust consumers while existing variants stay constructible. This constrains *source* consumers
  only — a new `"kind"` still breaks a client with a mirror enum, so the wire tags are pinned by
  test to keep that a deliberate act.

  **`crates/synthpass-core` had no `tests/` directory**, so nothing checked the serialized shape of
  the type that *is* the published contract: a renamed field or a changed `skip_serializing_if`
  would have shipped silently. New `tests/schema_keys.rs` pins the exact top-level key set for the
  Tier-1 and Tier-2 paths, that `trace` stays invisible until populated, that no routing score
  leaks into it under any spelling, that `Provenance` wire tags are stable, and that `CoreField`'s
  names match `ExtractionFields`' JSON keys. 11 tests. They are meant to fail when the schema
  changes — that is the point.
