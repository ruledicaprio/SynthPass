- **`tools/apply_cohort.py`** now finds a packet's `screened-cNN.jsonl` from the packet's own
  name rather than the cohort branch's, so a later cycle folded into an accumulating branch
  (packet c12 on `cohort-c10`) gets its `origin.url` / `origin.page` filled instead of null,
  which the manifest test rejects. A missing sidecar is now warned about up front.
