- **The pass-through `mrz` object follows `mrz` 0.8's wire form** (ADR-0019, ADR-0020): in the
  browser demo's `parse_mrz_text` result, and under the `mrz` key of `synthpass-serve`'s
  streamed `done` event and document-status responses. The `date_of_birth_completeness` key is
  gone, and `sex` is the MRZ's own character: `"<"` for an unspecified cell and the character as
  read for a non-conformant one (`"1"`, `"S"`), where both used to arrive as `"X"`. Dates are
  unchanged. The extraction schema (`extracted`, `extracted_v2`), exports and benchmark reports
  keep `M`/`F`/`X` byte-identically through one mapping in `synthpass-core` (`mrz_product::sex`).
  The demo's copied JSON and check-in form keep `M`/`F`/`X`; its results table shows the zone
  character.

  **Breaking for JSON consumers, released in a minor version by maintainer decision (2026-09-24):**
  the workspace crates are unpublished and the product is pre-adoption. The published `mrz`
  crate carries its own breaking release (0.8.0). The migration guide covers this change.
