- **MRZ name-field punctuation now follows ICAO 9303 Part 3 §4.6.** `format_td3`/`format_td2`/
  `format_td1`/`format_mrv_a`/`format_mrv_b` used to map every non-alphanumeric character in a
  name — apostrophe included — to the `<` filler, so `O'CONNOR` emitted as `O<CONNOR`. It now
  drops apostrophes with no filler (`OCONNOR`, matching ICAO's own worked example) and drops all
  other punctuation the same way, while hyphens, commas, and whitespace still become a single `<`
  separator. This changes the emitted bytes for any name containing an apostrophe or other
  punctuation beyond hyphen/comma/space; document numbers and optional-data fields are unaffected.

  Name truncation also changed. A name too long for its field used to be cut mid-character-run,
  which could leave the field ending on a filler; Part 4 §4.2.3 requires the last character of a
  truncated name field to be alphabetic, as the reader's only signal that truncation happened.
  Truncation now drops whole `<`-separated components from the end and shortens the last one if
  needed, so the field always ends on a letter. Note that Part 4 offers several issuer-discretionary
  truncation strategies and prefers none of them — this is one conformant choice, not "the" ICAO
  algorithm — and that a name which exactly fills its field is indistinguishable from a truncated
  one, which the standard states explicitly rather than treating as a defect.

  All 33 non-truncating VIZ→MRZ worked examples published across Parts 4, 5, 6 and 7 are now pinned
  as tests and reproduce byte-for-byte.
