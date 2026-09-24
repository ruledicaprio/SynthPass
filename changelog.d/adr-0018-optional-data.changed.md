- **`optional_data_1` and `optional_data_2` join the extraction schema, and `personal_number` now
  means only a TD3 personal number** (ADR-0018). v2 `fields` and `confidence` carry the two new
  keys on every record; the v1 `Extraction` carries them only when populated, so a record for a
  document without optional data is byte-identical to what it was. On TD1, TD2, MRV-A and MRV-B
  the value that used to arrive under `personal_number` — for TD1, two printed fields joined with
  a space — now arrives in its own slot under `optional_data_1` (and `optional_data_2` for TD1's
  second element), at `0.9` confidence rather than `1.0`: no check digit covers optional data on
  any format, and the old `1.0` was an over-claim. On TD3 nothing moves. `personal_number` is
  `null` off TD3, so a consumer reading it on an identity card or a visa must read
  `optional_data_1`/`optional_data_2` instead. The benchmark scores the two new columns
  (`COMPARED_FIELDS` 10 → 12), which lowers every mean-over-fields CER with no accuracy change
  because both columns are empty on most documents — recorded as a discontinuity under
  `knowledge/benchmarks/`, not an improvement. The live demo, the server UI and the ground-truth
  review form show the two fields; the `mrz-wasm` payload follows the crate's fields.

  **Breaking for JSON consumers, released in a minor version by maintainer decision (2026-09-24):**
  the workspace crates are unpublished and the product is pre-adoption. The published `mrz`
  crate carries its own breaking release (0.8.0). The migration guide covers this change.
