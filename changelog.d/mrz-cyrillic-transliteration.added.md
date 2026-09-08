- **`mrz` transliterates Cyrillic national characters (Doc 9303 Part 3 §6 B).** `ИВАНОВ` now
  emits into the MRZ name field as `IVANOV` rather than being silently dropped to fillers — the
  same class of fix §6 A got for Latin (`MÜLLER`→`MUELLER`). New public
  `mrz::transliterate_cyrillic(&str, CyrillicLanguage)` /
  `transliterate_cyrillic_char(char, CyrillicLanguage, is_first)` and the `CyrillicLanguage`
  enum (`Russian` default, `Belarusian`, `Bulgarian`, `Serbian`, `Ukrainian`, `Macedonian`).
  Unlike §6 A's issuer-choice styles, §6 B has 12 language-conditional rows (`Ж` is `ZH` in
  Russian but `Z` in Serbian; `Щ` is `SHCH` but `SHT` in Bulgarian) and five word-initial
  Ukrainian rules, so it needs a language rather than a style. The emitters
  (`format_td3`/`format_td2`/`format_td1`/`format_mrv_a`/`format_mrv_b` and
  `encode_name_component`) apply §6 B's base (≈ Russian) column; a caller that knows the
  language should run `transliterate_cyrillic` with the right `CyrillicLanguage` before
  emitting. §6 B carries no worked example in Doc 9303, so its 48 rows are pinned by
  table-integrity and conditional-set checks — see `knowledge/docs9303/CONFORMANCE_BASIS.md`.
  Arabic (§6 C) is still not implemented.
