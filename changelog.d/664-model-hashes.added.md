- **The per-document archive's run header records the SHA-256 of each OCR model file.** A new
  header key, `model_sha256` (`detection` and `recognition`), follows `model_paths` and is the
  hash of the same two `.rten` files the run loaded; a file that cannot be read is `null` for its
  key, and a replay, which loads no model, records `null`. The schema stays 1, and a file written
  before it has no such key. `tools/archive_query.py diff` names a differing hash as a prefix, and
  `tools/promotion_gate.py` compares models by hash instead of by path (#664).
