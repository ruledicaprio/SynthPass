- **Occlusion vocabulary on the wire (no detector yet).** `ExtractionTrace` gains
  `mrz_occlusion: {spans: [{line, first, last, kind}]}`, the per-document record of which MRZ
  cells the image showed covered; `ExtractionV2` gains `occluded: [CoreField]`, the ICAO fields
  that observation withheld — `null` in `fields`, `0.0` (the new `FieldConfidence::OCCLUDED` band)
  in `confidence`, distinct from a field that was simply never read (see
  `knowledge/decisions/ADR-0026-covered-cells-are-occluded.md`). `synthpass-die` gained
  `occlusion::apply`, wiring `mrz::apply_occlusion` into `MrzReader`: a refused occlusion over a
  check-digited cell now escalates with the new `EscalationKind::MrzOccluded`, and an occluded
  field is excluded from `Evidence::missing` and from every line-1 integrity check, so it is never
  reported as a misread and a covered name alone never escalates to Tier 2. Nothing in this workspace produces an
  occlusion observation yet, so every existing record serializes byte-identically to before.
