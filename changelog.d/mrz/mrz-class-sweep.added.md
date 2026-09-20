- **`solve_class_sweep`.** Resolves a check-digited field where OCR misread every occurrence of
  one character as its `CONFUSABLES` partner, including the check-digit cell itself — the shape
  measured on a TD1 card back whose document number is nine cells of one digit plus a
  check-digit cell of the same digit, all read as one confusable letter. Field-scoped only (never
  a whole-line sweep), requires at least two occurrences of the swept character across the field
  and its check digit, and is additive alongside `solve_substitution`'s single-position repair —
  `MAX_SUBSTITUTIONS` is unchanged. Not wired into `find_and_parse`/`find_and_parse_with` yet.
