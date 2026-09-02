- **`scripts/sync-samples.ps1 -Push -DryRun`** stages the mirrored corpus, prints the
  rename-detected diffstat and a renamed/added/deleted/modified tally, and stops without committing
  or pushing — leaving the worktree in place to inspect. `-Push` previously went straight to the
  remote with no way to see what it would do first, which matters more than it sounds: `Copy-Mirror`
  deletes by *filename*, so a renamed specimen reads as a delete plus an add. A diffstat without
  `-M` makes a lossless corpus-wide rename look like mass data loss, and would make genuine data
  loss look like a rename.
