- **Latin national-character transliteration (ICAO 9303 Part 3 §6 A).** New `crates/mrz` module
  `translit` exposes `transliterations`, `transliterate_char`, `transliterate`, and
  `TransliterationStyle` (`Expanded`/`Simple`/`XxSuffix`), implementing the 95-entry table ICAO
  recommends for transliterating accented/national Latin characters (`Ä`, `Ñ`, `ß`, `Ø`, …) into the
  MRZ's `[A-Z]` alphabet. Cyrillic (§6 B) and Arabic (§6 C) are not implemented.

  **Behaviour change:** `format_td3`/`format_td2`/`format_td1`/`format_mrv_a`/`format_mrv_b` now
  transliterate Table A characters in the name field (the `Expanded` style) instead of silently
  dropping them as unrecognized punctuation. `MÜLLER` used to emit as `MLLER`; it now emits as
  `MUELLER`. `Térèsa` used to emit as `TRSA`; it now emits as `TERESA`. This is a deliberate fix for
  non-conformant, silent data loss (Part 3 `:493` requires transliteration, not deletion, of national
  characters) — anyone relying on the old truncating behaviour for names containing accented
  characters will see different (and now spec-conformant) MRZ bytes. Document numbers and
  optional-data fields are unaffected; only the name field's cleaning function changed.
