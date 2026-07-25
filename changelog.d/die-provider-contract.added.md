- **`synthpass-die`: the provider contract every intelligence provider implements.** A new crate
  holding `IntelligenceProvider` (identity + declared `Capability`) and the two work shapes:
  `Recognizer` (image → text and layout geometry) and `FieldReader` (context → fields). MRZ and an
  LLM are the *same* shape; OCR is the other one. A future vision model reads the image already
  sitting in `DocumentContext` rather than needing a third trait — which is why the context is a
  struct and not a `&str`.

  Deliberately **not** a single `run(Request) -> Response`: that would force every provider to
  `match` on request variants it structurally cannot serve and answer "unsupported" at runtime,
  demoting a compile-time fact to a runtime failure.

  **The dependency list is the deliverable, not an accident.** The crate depends on
  `synthpass-core`, `mrz`, `async-trait` and `serde` — no tokio, no OCR engine, no llama.cpp — and
  CI asserts their absence with a `cargo tree` gate. A contract that names one engine cannot be a
  contract for the others, and "write a provider" must not begin with "install llama.cpp". The
  doc-test on `FieldReader` implements a working third-party provider in about twenty lines against
  this crate alone, driven by a hand-rolled executor to prove no runtime is required. It is the
  roadmap's *"a third-party provider builds against the published contract"* criterion, executed on
  every CI run rather than asserted in prose.

  **Reported confidence and routing evidence are different things, and the split is structural.**
  `Reading` carries both an `ExtractionV2` (the reported half — the ordinal confidence model,
  unchanged, part of the published JSON contract) and an `Evidence` (the routing half). Neither
  `Reading` nor `Evidence` derives `Serialize`, so there is no code path — not a careless one, not
  a future one — that can serialize routing signal into the document JSON. A routing score may be
  uncalibrated, because being wrong costs CPU seconds rather than truth; a number reaching a
  consumer next to `confidence` may not. See
  [`project_principles.md`](knowledge/project_principles.md) §2.

  `Evidence` collects signals that **already existed and were being discarded**: the MRZ band
  score, whole-page text sanity, portrait detection, rotation, `check_line1_integrity`'s verdict as
  PII-free `FindingKind`s, and — the one the old inline `.filter(|m| m.valid())` threw away —
  *which specific check digits failed*, which is the most actionable escalation reason available.
  Every field is a count, a bool, a bounded score or a fieldless enum, so there is nowhere in the
  struct to put a name or a document number.

  `ProviderCatalog` is builder-constructed, sorted cheapest-first with a stable sort so equal-cost
  providers keep registration order, and rejects duplicate ids at construction. Consultation order
  is therefore fully determined by the wiring: a benchmark that consults providers in a different
  order on two runs is not a benchmark. Link-time registration (`inventory`/`linkme`) was
  considered and rejected — it is invisible to Cargo features, its ordering is unspecified, and it
  works through linker sections, which is exactly what gets fragile under this project's
  `lto = true` / `codegen-units = 1` / static-musl release profile.

  `MrzReader` is the deterministic ICAO 9303 provider: `Capability::deterministic`, `CostClass::Free`,
  zero configuration and zero state. It never returns `Err` — "I looked and there is no MRZ" is an
  answer, not a failure, and reporting it as an error would leave the router unable to tell a broken
  provider from an empty page. It also records `blind_positions`, the count of characters whose OCR
  confusions are congruent mod 10 and therefore *invisible to the check digits*. That value ships
  **observed-only**: recording a signal before anyone has measured what it predicts is how you earn
  a threshold; routing on it first is how you invent one.

  Nothing is wired to this yet — `synthpass-pipeline` keeps its inline Tier-1 path, so behaviour is
  unchanged. Note that `extraction_v2_from_mrz` now exists in both places until that wiring lands.
