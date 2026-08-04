- **TD1 and TD2 are now reachable and separated at the Tier-1 gate (M6).** `synthpass-gen`
  could already build TD1/TD2 MRZ lines internally, but nothing could ask it to: `synthpass
  generate` had no flag, both bench binaries hardcoded TD3, and the sidecar's only type key
  (the MRZ document *code*, `"P"`/`"I"`) could not tell a TD1 from a TD2 — both correctly emit
  `"I"` per ICAO 9303. The MRZ *format* (`mrz::Format`/`synthpass_gen::DocumentType`), not the
  document code, is now the load-bearing separator everywhere: generation, labels, the sidecar,
  the bench corpus, and the Tier-1 report.

  **New:** `synthpass generate --document-type td1|td2|td3` (default `td3`); the sidecar JSON
  gains `"mrz_format"` and an `"mrz_lines"` array (`mrz_line1`/`mrz_line2`/`mrz_line3` kept for
  back-compat). `--document-type` on both `synthpass-bench` and `provider-bench` (distinct from
  `provider-bench --format`, which scopes real `samples/` specimens — the two are rejected
  together). `synthpass_gen::DocumentType` gains `as_str`/`parse` and bridges to/from
  `mrz::Format`. `synthpass_gen::Labels` gains `mrz_format`, the join key every downstream
  consumer now reads.

  **Fixed:** `build_td1_lines`/`build_td2_lines` hardcoded their MRZ document code instead of
  reading it from the `Passport` being rendered, so a `Passport` carrying `"P"` pushed through
  TD1 assembly silently rendered `"I"` under a `"P"` label — a label/pixel divergence that broke
  M2's "labels are 100% accurate by construction" invariant. A debug assertion in
  `build_mrz_lines` now catches the mismatch instead of silently rendering it.

  **New card geometry:** TD1 (ID-1, 822×518 px) and TD2 (ID-2, 1008×710 px) now render on their
  own ICAO card canvases — portrait at the left edge, VIZ fields in a column beside it, MRZ
  band spanning the full card width at the bottom — instead of TD3's 1200×840 passport page with
  a shorter MRZ band swapped in. TD3 output is unchanged (`synthpass_gen::layout::PageLayout`
  for TD3 is byte-identical to the pre-M6 constants). The mandatory synthetic watermark still
  renders unconditionally on every format.

  **New:** `synthpass-bench`'s report records which `document_type` a run measured (a run is
  always a single format); `provider-bench`'s per-document report gains `mrz_format`, resolved
  from the label for the synthetic corpus and from the deterministic MRZ provider's own read
  (falling back to a ground-truth guess) for real specimens. `scripts/run-bench.ps1` gains `td1`/
  `td2` synthetic tracks alongside the existing real-specimen ones.

  **Vision-provider readiness:** the Tier-1/routing `DocumentContext` now carries the on-disk
  image path (`crates/synthpass-pipeline`'s `ocr_and_tier1`), closing a gap where a
  vision-capable provider consulted at that stage would have received `None` regardless of
  whether one existed. No provider ships vision in this release — `Capability.vision` stays
  `false` everywhere, per `knowledge/ROADMAP.md`'s stated non-goal.

  **Docs:** `knowledge/decisions/ADR-0004-gpu-acceleration.md` (numbered, indexed, corrected —
  the pinned `rten` 0.24.0 has no `cuda`/`vulkan` feature or API surface at all, contrary to an
  earlier draft; `llama-cpp-2`'s GPU features do exist and are documented accurately) and
  `knowledge/decisions/ADR-0005-vision-provider-readiness.md` (new: `llama-cpp-2` 0.1.151
  already carries a real `mtmd` multimodal Rust API, gated behind an off-by-default feature, no
  version bump required — Moondream-specific support is unconfirmed, not proven). README trimmed
  from 471 to 189 lines (471 → 295 in this milestone, 295 → 189 in the M6 TD1 follow-up, which
  also collapsed the README's two `mermaid` diagrams into one), with its moved content now living
  in `knowledge/ARCHITECTURE.md`/`VISION.md`/`LICENSING.md`; several self-declared-superseded
  `knowledge/` docs moved to `knowledge/archive/`.
