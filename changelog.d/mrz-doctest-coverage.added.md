- **`mrz` docs: every public function now carries a runnable example.** The 15 that had none
  — `parse_td3`, all five `parse_*_with` two-digit-year-pivot variants, `transliterate_char`,
  `transliterate_cyrillic_char`, `transliterations`, `encode_name_component`,
  `expand_date_with_pivot`, `class_of`, `substitution_candidates`, `solve_substitution`,
  `width_candidates` — gained a compiled doctest, plus one on the `CyrillicLanguage` enum. No
  API change; docs.rs "items with examples" goes 25/40 → 40/40.
