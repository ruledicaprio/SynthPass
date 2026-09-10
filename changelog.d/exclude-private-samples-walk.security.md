- **`provider-bench --real-specimens` walked `samples/private/` with no exclusion.** The moment a
  user drops real-PII specimens under `samples/private/` for their own local benchmarking, the
  default corpus silently grows to include them — no warning, no opt-in. `find_image_files` now
  skips any path component containing `private` (case-insensitive), covering both the local
  `samples/private/` directory and the corpus naming convention's `_Private_` filename token.
  Nothing in the tracked repo or CI ever saw these files; this closes the same gap in the bench's
  own walk.
