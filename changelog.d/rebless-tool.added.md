- **`tools/rebless.py`** takes a cohort branch from "PR open, images pushed" to "ready for review"
  in one command: dispatches `real-specimen-gate.yml -f mode=write-baseline`, waits for it,
  downloads the artifacts, classifies the diff against the committed baseline (`identical` /
  `non-scored delta` / `scored delta`), installs the new baseline, and for the first two classes
  mechanically rewrites the numbers this loop has hand-edited every cohort PR so far -- README.md's
  gap sentence and corpus-wide rate, `knowledge/benchmarks/README.md`'s live block, and a templated
  dated `## Weak-spot findings` entry -- before committing, pushing, dispatching the `mode=assert`
  run, and marking the PR ready. A `scored delta` (`tier1_hits` or a scored miss bucket moved)
  always stops after installing the baseline and printing the diff table: that class needs prose
  from `synthpass-analyst`, not a template. Two independent safety gates, same shape as
  `tools/apply_cohort.py`: `--dry-run` performs only the worktree/branch check and prints the rest;
  `--confirm` separately gates every dispatch, commit, push, `gh pr ready` and PR-body PATCH.
  Reuses `apply_cohort.py`'s `gh`/git-bash helpers by import. Standard library only; see
  `tools/test_rebless.py`.
- **`scripts/check-headline-numbers.sh`** now also checks `knowledge/benchmarks/README.md`'s own
  "Current headline numbers" live block against the committed baseline -- both rates, the outcomes
  heading's document count, all six outcome-table bucket counts, and the "N of the `<documents>`
  specimens cannot produce" sentence. That block is the document README.md and ROADMAP.md only
  summarize, and it went stale on cohort c12 with nothing catching it until a hand check.
