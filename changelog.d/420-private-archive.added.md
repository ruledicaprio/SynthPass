- **`provider-bench --include-private` now writes the per-document archive, with the private
  track as text-free records.** Public and local records are unchanged. Each private document is
  one record in `private/`, a file of its own with the run's stem: keyed by the image's SHA-256,
  with the outcome, check states, retry facts, timings, band score, mismatch counts and positions,
  and the recovered zone as character classes (letter, digit, filler, other). The record type has
  no field for a name, an asset ID, OCR text, a zone line or a field value, and its header drops
  `argv`. Before this, such a run wrote no archive at all. `--no-archive` or
  `SYNTHPASS_BENCH_ARCHIVE=off` still turns it off (ADR-0024 Decision 7, #420).
