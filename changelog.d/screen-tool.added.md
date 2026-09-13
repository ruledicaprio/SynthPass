- **Automated screen for agent-found specimen candidates.** `tools/screen_candidates.py`
  mechanically re-derives each candidate a scouting worker reports before any human review:
  re-fetches the page and image itself, confirms the image is actually linked from the page,
  checks host and byte-hash denylists/duplicates against the ledger and `samples/corpus.jsonl`,
  runs `check_sample` (failing closed to `vendor` if its `VENDOR` line is missing), and writes a
  survivors-only Markdown packet with the provenance/licence/verdict columns left blank for a
  human. It never decides public/local/drop, never decides a licence, and never writes a holder
  value to any output file. Implements the "Automated screen" step in
  `knowledge/SPECIMEN_SOURCES.md`.
