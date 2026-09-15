- **`tools/apply_cohort.py` reaches a no-hand-edits cohort (T11).** The `--confirm` commit now
  stages an explicit list of every path the run itself wrote -- images, `samples/corpus.jsonl`,
  the local-track manifest, `knowledge/CORPUS_COVERAGE.md`, the changelog fragment -- instead of
  `git add -A samples changelog.d`. The release examples (`corpus_manifest`, `check_sample`) are
  always rebuilt rather than only a missing one, since cargo's incremental no-op is fast and a
  stale binary silently produced zero cover rows on cohort c12. A cohort row whose target is
  under `samples/covers/` now gets real `knowledge/CORPUS_COVERAGE.md` Docs/Note prose written
  directly (never a Status move -- ADR-0012), and a covers-only cohort gets a real changelog
  fragment instead of a `<<< fill in >>>` draft; both fragment names are unified on
  `corpus-cohort-cNN.added.md`, the name already used on main (the tool previously wrote
  `corpus-cNN.added.md`, which never matched). `--reuse-existing --confirm` now PATCHes the
  existing PR's title and body via `gh api -X PATCH` (`gh pr edit` lacks the `read:org` scope on
  this machine). `scripts/check-doc-links.sh` and `scripts/check-changelog.sh` now run under Git
  for Windows' `bash.exe`, resolved from wherever `git` itself is found on PATH, instead of
  silently resolving to WSL bash and failing on a `D:\` path.
- **Scout prefix v3.1** (`tools/scout_prefix.txt`), after cycle c13 returned several non-document
  photos and mislabelled cover claims: the candidate image must show the document itself (a
  cover, a data page, a card face, a visa sticker), not a flag, building, official, application
  form or screenshot; a cover is reported only as a fallback, when no data page was found for
  that code, and at most one per code; and `"side"` must report what the worker actually saw in
  the image rather than a page title's claim, with `"side":"unknown"` as the honest answer when
  it was inferred from text. `knowledge/SPECIMEN_SOURCES.md`'s signal section and the
  `synthpass-screener` agent carry the same rules.
- **`tools/host_denylist.json`** (parallel in shape to `tools/legal_portals.json`): hosts that
  are not an issuing authority and surfaced non-document images in a screening cycle.
  `tools/screen_candidates.py` auto-rejects a candidate on one of these hosts with reason
  `denylisted-host`, ledgered like any other auto-reject. Seeded with the two hosts cycle c13's
  screener report named: `guineaecuatorialpress.com` (a press agency) and `digitalinvea.com` (a
  third-party vendor).
