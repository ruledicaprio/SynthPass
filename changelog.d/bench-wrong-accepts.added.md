- **`synthpass-bench`'s M4 synthetic gate reports `wrong_accepts`.** A Tier-1 `hit` proves only a
  checksum-consistent zone whose document number matches truth. `document_type`,
  `issuing_country`, both names, `nationality` and `sex` carry no check digit at all, and even a
  check-digited field can still be wrong — a check digit is consistency, not proof — so a hit can
  still be a wrong read on any of the 12 scored fields (issue #453). The JSON report now
  carries `wrong_accepts`/`wrong_accept_rate` alongside `strict_hits`, and each `results[]` entry
  carries `wrong_accept`/`wrong_fields` naming which of the 12 scored fields diverged. The stdout
  summary prints the count next to the strict-hit-rate line. Report-only: `hit`, `hit_rate` and the
  `--min-hit-rate` gate are unchanged.
