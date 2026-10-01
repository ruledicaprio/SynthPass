- **`provider-bench` keeps a per-document archive of every run, outside the working tree.** Each run
  writes one JSON Lines file per track, `public/` (the public corpus, covers and synthetic runs)
  and `local/` (`samples/local/`), under `synthpass-bench-archive/` in the git common directory or
  under `SYNTHPASS_BENCH_ARCHIVE`: a run header, then one record per document per provider with
  the OCR text the provider read, its outcome row and field values, and mismatch positions for a
  labelled specimen. There is no private track, and a `--include-private` run writes no archive.
  A file is `.partial` until its run finishes. No run reads the archive, and a problem with it is
  a `warning: archive:` line, never a changed exit code, report or ledger. `--no-archive` or
  `SYNTHPASS_BENCH_ARCHIVE=off` turns it off (ADR-0024, #420).
