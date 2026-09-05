- **`vocabulary_fingerprint()` now covers the `mrz` country-name table.** It previously hashed
  only `MONTH_NAMES`/`DEMONYMS`/`SEX_FORMS`/`DOCUMENT_TYPE_FORMS` — every table declared in
  `synthpass-core::normalize` — but `country_code` (used by `nationality`/`issuing_country`)
  consults `mrz::code_for_name`'s table *first*, before `DEMONYMS`. A change to that table could
  silently move parity numbers with the fingerprint reporting no change — the same failure shape
  the fingerprint exists to catch, one level down. **The fingerprint's value has changed** as a
  result: a fingerprint recorded before this fix is not comparable to one recorded after it, even
  with no other vocabulary change.
- **A genuinely accented demonym token (e.g. a real `Ñ` in `ESPAÑOLA`) now resolves.**
  `code_from_demonym_tokens` split on `!c.is_ascii_alphabetic()`, so the accent itself was a token
  boundary and fragmented the word before it ever reached the table. Tokens are now
  transliterated (Doc 9303 Part 3 §6 A, `Simple` style) before matching, the same transliteration
  `mrz` already uses elsewhere.
- **A demonym conflicting with a plain country name is now caught.** The "two tokens naming
  different countries → leave the string alone" safety net only checked tokens present in
  `DEMONYMS` itself; `"CANADIAN FRANCE"` resolved to `CAN` because `FRANCE` isn't a demonym and
  was silently skipped. The conflict check now also resolves each token through
  `mrz::code_for_name`, though only a `DEMONYMS` match can still be *returned* — this does not
  turn `DEMONYMS` into a second country-name table, it only widens what counts as a conflict.
