- **`tools/apply_cohort.py`.** Automates the mechanical parts of the specimen-acquisition loop's
  "P-DATA-PR" procedure once a cohort packet has a Verdict on every row: worktree setup, a
  cross-PR duplicate guard (excludes filenames still pending in another open `cohort-*` PR before
  regenerating the manifest, so two open cohort branches can no longer silently fold each other's
  not-yet-merged rows together), image placement, manifest regeneration and `origin` patching, the
  `samples/local/` track, the mechanical checks, and a draft changelog fragment. Licence class,
  `origin.notes` and the `CORPUS_COVERAGE.md` prose Note column always stay human-reviewed drafts,
  never auto-committed. Before any real `sync-samples.ps1 -Push`, it always runs `-Push -DryRun`
  first and refuses to proceed if it reports a deletion unless `--allow-deletions` and `--confirm`
  are both given and the exact file list has been printed -- the guardrail that would have caught a
  near-miss mirror-deletion of another PR's images by hand. Standard library only; see
  `tools/test_apply_cohort.py` for its offline unit tests. Dispatching the real-specimen re-bless
  workflow, waiting for it, and merging the PR stay manual steps, on purpose.
