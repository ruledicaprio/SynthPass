- **`synthpass-bench` writes the per-document archive too.** A run writes one JSON Lines file into
  `public/` of the same archive `provider-bench` uses (`<git common dir>/synthpass-bench-archive/`
  or `SYNTHPASS_BENCH_ARCHIVE`), named for the binary: a run header, then one record per seed with
  the `--ledger` row, the check states, the OCR text with its band score and rotation, the zone
  after repair, the read's field values, and mismatch counts and positions against the generator's
  labels. A field the synthetic path does not have is `null`. Like `provider-bench`'s, the file is
  `.partial` until the run finishes, no run reads it, and a problem with it is a
  `warning: archive:` line, never a changed exit code, report, ledger or `--dump-ocr` output.
  `--no-archive` or `SYNTHPASS_BENCH_ARCHIVE=off` turns it off (ADR-0024, #420).
