- **`provider-bench --real-specimens --include-local`** opts `samples/local/` back into a
  real-specimen run. That directory holds specimens usable locally whose source does not allow
  redistribution: it is gitignored, never mirrored to `samples-data`, and left out of the default
  walk, so the committed baseline keeps measuring the public corpus only. Off by default and
  rejected without `--real-specimens`. `synthpass_bench::load_real_specimens` now takes an
  `OptInTracks { private, local }` in place of its `include_private: bool`.
- **`samples/corpus.jsonl` rows carry `origin`** — the fetched URL, the publishing page, a licence
  class, the fetch date and what found the image. Rows that predate it read `unrecorded`, and the
  manifest validator rejects an origin recorded only in part. The same validator now refuses an
  image recorded twice under two names; the five such pairs already in the corpus are pinned as
  known until a data PR removes the stale copies.
