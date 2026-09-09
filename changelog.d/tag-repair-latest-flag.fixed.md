- **`scripts/repair-tags.sh` no longer steals the "Latest release" badge when backfilling.**
  `gh release create` marks the most recently *created* release as Latest unless told
  otherwise, so backfilling twelve historical releases in chronological order left **v1.3.0**
  badged Latest on the repository front page instead of v1.4.0. Observed on the real run and
  undone with `gh release edit v1.4.0 --latest`; the script now passes `--latest=false` for
  every backfilled release so a re-run cannot repeat it.
