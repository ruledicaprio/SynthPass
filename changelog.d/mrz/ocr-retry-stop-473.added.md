- **`MrzData` reports whether a reading needed damaged-capture recovery.** The new
  `damaged_recovery: bool` field is `true` when `find_and_parse`/`find_and_parse_with` only
  reached a checksum-valid reading through the damaged-capture search (a line one character
  narrower than its format, or a single-glyph substitution swept across `CONFUSABLES`), rather
  than the ordinary scan's fixed-position lookalike repairs. `false` for every other reading,
  including `parse_td1`/`parse_td2`/`parse_td3`/`parse_mrv_a`/`parse_mrv_b` and their `_with`
  siblings called directly. A checksum-consistent reading was never proof it is correct — this
  lets a caller (`synthpass-ocr`'s retry loop, issue #473) tell the weaker kind of evidence apart
  from the ordinary one instead of trusting both equally.
