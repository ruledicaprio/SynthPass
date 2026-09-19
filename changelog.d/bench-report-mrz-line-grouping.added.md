- **`synthpass-bench`'s per-field CER table groups by MRZ line.** The "mean character error rate
  by field" table (stdout and `bench-report.json`) now annotates each field with the physical MRZ
  line it lives on for the run's format, and prints/reports the mean CER per line plus their
  ratio — the split between a format's lines has been the single most informative fact in this
  table and was previously invisible unless the reader already knew the format's layout by heart.
  Format-aware: TD1 assigns different fields to line 1 than TD2/TD3/MRV-A/MRV-B do. The line
  lookup (`synthpass_bench::provider_bench::mrz_field_line`) shares the existing ICAO field-layout
  tables rather than duplicating their offsets. `mrz_lines` (a whole-zone aggregate) and any field
  with no span in a given format's layout are reported under a clearly-labelled "no single line"
  group and excluded from the per-line means, never assigned a line they do not have. Additive
  reporting only — `hit`/`hit_rate`, the strict-name figures and every existing baseline are
  unchanged.
