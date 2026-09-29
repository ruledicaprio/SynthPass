- **Line-1 integrity reports two findings about the zone's own name field.**
  `raw_name_separator_missing` means the field holds no filler at all and is too short for
  `missing_name_separator`. `raw_interior_filler_run` means it holds three or more `<` in a row, or
  a second `<<`. Both are report-only: they lower no field confidence and do not feed the opt-in
  `Line1Flagged` routing clause. A record whose only findings they are now reads `needs_review`,
  and `synthpass` prints its line-1 warning for it (#576).
