- **The elimination probe can sweep an operating envelope.** `elimination_probe --sweep AXIS`
  (repeatable: `height`, `ink-spread`, `keystone`) degrades the whole public specimen at each step
  of one axis, re-runs the zone locator, checks that the located grid is the native grid carried
  through the step (`grid_ok`, so an accepted but misfitted grid is counted apart from a refusal),
  and measures the cells again. It writes `envelope.jsonl` and one summary per axis beside the
  probe's own `cells.jsonl`, which a run with `--sweep` leaves byte for byte as it is. The signal
  floor of the resolution sweep (fixed segmentation) and the new envelope floor (zone re-located at
  every step) are reported side by side and never combined. The keystone warp is written in the
  example, not added to `synthpass-gen`. A benchmark example only; no product code changes.
