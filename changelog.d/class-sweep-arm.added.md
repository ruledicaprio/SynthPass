- **`SYNTHPASS_MRZ_CLASS_SWEEP` — a three-arm switch for the `mrz` confusable-class sweep.**
  `on` enables it, `off` (the default) does not, and `control` is a **placebo**: behaviourally
  identical to `off`, present so a run can tell a real effect from the noise between two nominally
  identical arms. Decision rule `on > control >= off`, the same-binary A/B shape this repo uses for
  every measured change.

  The arm lives in `synthpass-die`, beside the MRZ provider, because **that is the parse the
  real-specimen benchmark actually exercises**. `provider-bench --real-specimens --mrz-only` reads
  through `MrzReader`, not through the synthetic corpus path — so wiring only the latter would have
  produced an A/B that moved nothing and read as a clean null. The synthetic path in
  `synthpass-bench` and the two `mrz_found` diagnostics in `provider-bench` reuse the same helper
  rather than reading the variable again, so no two call sites can drift into measuring different
  arms.

  **An unrecognised value falls back to `off` silently**, which is why `class_sweep_arm()` returns
  the arm by name: quote what the binary reports it measured, never the variable you believe you
  set. Ground-truth parsing is deliberately left alone — it reads known-good transcriptions, and
  applying a repair there would corrupt the reference the run is scored against.

  Nothing changes unless the variable is set.
