- **`provider-bench` now records which `mrz` parse arm it actually measured**, as
  `mrz_class_sweep_arm` in the report JSON and on stdout as
  `mrz class-sweep arm measured: <arm>`.

  `SYNTHPASS_MRZ_CLASS_SWEEP` falls back to `off` on any unrecognised value, **silently** — so a
  three-arm A/B with a misspelling in one arm produces two identical populations and reads as a
  clean null. Verified across all four cases: unset and a deliberate typo both report `off`, `on`
  reports `on`, `control` reports `control`.

  Quote this field, never the variable you believe you set.
