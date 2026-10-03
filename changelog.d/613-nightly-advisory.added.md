- **The nightly benchmark now reports what each night changed.** `tools/bench_nightly_advisory.py`
  reads the rows on `bench-data` after the nightly appends them. It pairs the fixed slice (clean,
  seeds 0-99) seed by seed against the last night with the same generator fingerprint and names every
  flipped seed, and it tests the fresh slice's last 7 nights against the prior 28 with a two-proportion
  z-test (a flag at z <= -3 on the hit or correct-read rate, z >= +3 on the wrong-accept rate). It
  writes a job summary, one `::warning::` per flag, and one aggregate line per night to
  `advisory.jsonl` on `bench-data`, with counts, rates and seed numbers only. The run goes red only,
  and only after its rows are pushed, for a fixed seed newly read as a wrong accept or a prefix-wrong
  read, or for an invalid measurement (a retry-budget stop in the fixed slice, missing or duplicate
  rows, or a re-render no generator change explains). It never opens an issue and is not a required
  check (#613).
