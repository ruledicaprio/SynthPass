- **`synthpass_imageprep::geometry::detect_mrz_band_range`.** Returns the winning MRZ line group's
  `(start, end_exclusive, avg_score)` index range instead of its union bounding box, for a caller
  that needs to know *which* lines composed the band — the bbox alone cannot say. `detect_mrz_band`
  and `detect_mrz_band_scored` are unchanged for existing callers; `detect_mrz_band_scored` is now
  a thin wrapper over the new function, so the two can never disagree about which group won.
- **`examples/probe_matrix.rs` (`synthpass-ocr`), a read-only CTC matrix diagnostic.** Reads the
  recognition model's probability matrix directly, via the public `prepare_recognition_input` plus
  `rten::Model::run_one`, rather than the beam decode — so a per-cell read can be compared against
  the beam on the same cells, and probability mass can be measured at cells the beam resolved some
  other way. Example-only: no default code path changes, and nothing is added to the shipped crate.
  Its measured summary is committed as `knowledge/benchmarks/mrz-matrix-probe-2026-09-18.json`.
