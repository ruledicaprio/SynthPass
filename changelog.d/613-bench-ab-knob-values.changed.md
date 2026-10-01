- **`bench-ab.yml` refuses a knob value the code would not recognise.** Every measurement knob falls
  back to its default silently on a value it does not read, so `SYNTHPASS_OCR_ORDER=bandfirst` made
  an A/B of two identical arms that read as a clean null result. `tools/bench_ab_args.py` now holds
  each knob's values (as `configuration.md` lists them, and 1 to 14 for `SYNTHPASS_OCR_MAX_PASSES`)
  and refuses any other spelling with one `::error::` line that names the knob and its values. Only
  the canonical form passes, so `ON` and `014` are refused, and an `arm.json` or plan that holds
  such a value is refused too. Tests pin each list to `configuration.md` and the pass ceiling to the
  OCR crate.
