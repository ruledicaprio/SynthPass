- **Scout prefix v3** (`tools/scout_prefix.txt`, plan §8). Three scope changes learned from
  twelve cycles: an image the issuing authority publishes on an allowed host as an illustration
  of its own document is now reported even when the page says nothing about specimens
  (`specimen_signal: official-host-only`; the screen and the reviewer confirm the signal on the
  image, the real-person check applies in full); a PDF linked from an opened page may be
  reported unopened (`format: pdf`) for the screen to extract; and pages on allowed hosts that
  would not load are listed as `BLOCKED:` lines, which `tools/scout_cycle.py` collects into
  `blocked-cNN.jsonl` for a session-side retry. `knowledge/SPECIMEN_SOURCES.md`'s signal
  section and the `synthpass-screener` agent carry the same rule.
