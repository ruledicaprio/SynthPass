- **`tools/archive_query.py` reads the per-document benchmark archive.** Standard library only and
  read-only, it answers questions from runs already made, without a new OCR pass: `runs` lists the
  finished run files (and counts the `*.partial` files it skips), `diff A B` names the header arms
  that differ and joins two runs' documents (a real specimen on `source_sha256`, so a rename keeps
  its key; a synthetic seed on format, profile and seed) to list the documents in one run only, the
  outcome changes, the per-field changes of the ledger row (timing apart), the recovered zones that
  differ with their cell positions, and the `truth` mismatch deltas, and `cell RUN --line L --col C`
  counts the classes at one cell of the recovered zone. It prints asset IDs, field names, counts,
  positions and classes, and never OCR text, a zone line, a field value or a `miss_reason` text;
  `--chars` is the one opt-in and refuses the `local/` track (ADR-0024, #420).
