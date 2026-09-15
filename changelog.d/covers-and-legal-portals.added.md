- **Cover-only specimens are a labelled class** (ADR-0012). A cover or design rendering with no
  data page -- the largest bucket of what the scout workers find -- is now collected instead of
  dropped: filed in its document type's directory with the `cover` variant token next to
  `no_mrz`, recorded as `Passport (cover)` in `CORPUS_COVERAGE.md`'s Docs column, never a
  coverage claim, never a new verdict value. The screener agent and the packet note name it.
- **Legal-acts portal hints for the scout** (`tools/legal_portals.json`, 70 codes). A code with a
  known official gazette or consolidated-acts portal gets it in the worker's task next to the
  country name; both gazette hits of the loop's first twelve cycles came from such portals.
