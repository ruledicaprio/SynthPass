- **The nightly bench rows record only named measurement knobs.** A run header's `ocr_env` copied
  every `SYNTHPASS_*` variable on the runner into the public `bench-data` rows, and its value check
  would have let a short credential through. `tools/bench_nightly_rows.py` now records only the
  knobs `tools/bench_ab_args.py` allows plus the pinned wall-clock budget, and refuses a context
  that names any other variable. A test pins the two lists together.
