- **`synthpass-bench --max-prefix-wrong-accepts N` gates the line-1 prefix.** A Tier-1 hit whose
  `document_type` or `issuing_country` differs from the generator's truth is now a *prefix wrong
  accept*. No check digit covers those two fields in any ICAO format. The run exits non-zero when
  more than `N` occur. The JSON report carries `prefix_wrong_accepts` and
  `prefix_wrong_accept_seeds`, and each `results[]` entry carries `prefix_wrong_accept`. The M4 CI
  job pins `N` at 0, the count CI measured, as a ratchet: raising it is a reviewed workflow edit
  (issue #453, ADR-0013's 2026-09-26 amendment). Wrong names, optional data and check-digit collisions
  stay report-only, and `hit`, `hit_rate` and `--min-hit-rate` are unchanged.
