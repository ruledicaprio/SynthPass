- `SYNTHPASS_MRZ_REFUSE_REPEATED_LINE` (`off` by default, `control`, `on`): a measurement arm for
  `mrz`'s opt-in repeated-line refusal (#579). Read by `synthpass_die::mrz_parse_options`, so it
  applies to the product's `MrzReader` and both benches. Recorded as `mrz_refuse_repeated_line_arm`
  in both bench reports, as `mrz_arms.refuse_repeated_line` in the run manifest, and in
  `trace.config_overrides` when not `off`; `--write-baseline`/`--assert-baseline` refuse any value
  but `off`. The OCR retry loop's stopping rule and the printed-zone validity parse do not take it.
