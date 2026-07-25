- **`docs/` is now [`knowledge/`](knowledge/README.md) — a technical library rather than a docs
  folder.** Documentation describes what was built; this tree preserves *why*, so the reasoning
  outlives the code and the models. Renamed with `git mv`, so `git log --follow` still works.
  Rationale, rejected alternatives and consequences:
  [`ADR-0001`](knowledge/decisions/ADR-0001-knowledge-tree.md).

  Three living artefacts land with it:
  [`project_principles.md`](knowledge/project_principles.md) (seven principles, each tied to the
  place in the codebase that enforces it — a principle nothing checks is a preference),
  [`technical_debt.md`](knowledge/technical_debt.md) (deferred decisions with honest severity and
  effort), and [`decisions/`](knowledge/decisions/README.md) for ADRs. Ten subject folders
  (`providers/`, `prompts/`, `ocr/`, `vision/`, `benchmarks/`, `evaluation/`, `hardware/`,
  `research/`, `papers/`) each carry a README stating what belongs in them **and what does not**,
  so an empty folder reads as a visible gap rather than decoration.

  ~116 `docs/...` references across 35 files were repaired — workflow files, six crate manifests,
  `.rs` doc comments, and historical changelog entries whose links would otherwise 404. New
  `scripts/check-doc-links.sh`, wired into CI, is what stops that rotting again: it verifies
  relative Markdown links resolve, that `knowledge/...` paths cited in prose exist, and that no
  stale `docs/...` survives outside an allowlist that requires a written reason per entry.

  Two pre-existing defects were found and are recorded rather than silently patched:
  `docs/V2-DESIGN.md`, cited by section number in 11 source doc comments, **has never existed in
  this repository** (filed as High in `technical_debt.md`; deliberately not rewritten, since
  moving a broken link is not fixing it); and a paper in `papers/` was filed under the filename of
  an unrelated crowd-counting paper, renamed to match its actual contents.

  No behaviour change — the only source edits are doc comments.
