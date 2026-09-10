# ADR-0008 chunk 1C analysis scripts

Companion to [`../ocr-order-band-first-2026-09-10.md`](../ocr-order-band-first-2026-09-10.md)
(cell a) and [`../ocr-gap-is-detection-2026-09-10.md`](../ocr-gap-is-detection-2026-09-10.md)
(cell b).

- `analyze.py <default.json> <band-first.json> [control.json …]` — positional per-document
  diff of `provider-bench --real-specimens --mrz-only` reports (the corpus walk is
  deterministic and sorted, so row *i* is the same document in every report; keying by
  `name` loses the ~3 duplicate stems the corpus carries across sub-directories).
- `passes2.py` — joins `SYNTHPASS_OCR_VERBOSE=1` stderr (which retry pass validated, and OCR
  wall-time per document) with the JSON reports (authoritative hit/miss). Input paths are
  hardcoded to the run that produced the writeup; edit them to re-point.

Both expect the run artifacts under `artifacts/ocr-order-ab/` (gitignored).
