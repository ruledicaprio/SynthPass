- **`mrz` no longer drops the secondary identifier when truncating a long name (all five formats;
  no API change).** ICAO 9303 requires that when a name overflows the MRZ name field, characters
  **shall** be removed from the *primary* identifier until three positions are freed, so that `<<`
  and at least the first character of the secondary identifier still fit
  (`Doc_9303_Part4...:407`, the Data Element Directory row §4.2.3 cross-references; repeated
  verbatim in Part 5 `:346`, Part 6 `:299`, Part 7 `:282`/`:640`). `emit::truncate_name_components`
  filled the field left-to-right and stopped at the first overflow instead, so any name whose
  primary identifier alone reached the field width consumed all of it and the `<<` separator was
  never written.

  **The consequence was corruption, not just loss.** With no `<<` to split on, `parse_*`'s
  `clean_name` falls back to the first single `<` — a fallback that exists to recover
  OCR-collapsed separators — and reports the *first primary component* as the surname with the
  remaining **primary** components as given names. ICAO's own §4.2.3.3(b) example:

  ```text
  ICAO Part 4 §4.2.3.3(b):  PPUTOBENNELONG<WOOLOOM<WARRAND<WARNAM<<DINGO
  before this fix:          PPUTOBENNELONG<WOOLOOMOOLOO<WARRANDYTE<WARNA
    round-tripped as:       surname "BENNELONG"
                            given_names "WOOLOOMOOLOO WARRANDYTE WARNA"
  after this fix:           surname "BENNELONG WOOLOOM WARRAND WARNAM"
                            given_names "DINGO"
  ```

  **This changes emitted output** of `format_td1`/`format_td2`/`format_td3`/`format_mrv_a`/
  `format_mrv_b` for names that overflow the name field — and only for those; a name that fits is
  byte-identical, as are the already-conformant cases where the primary fits but the combined name
  does not (ICAO §4.2.3.2, the secondary-truncation examples, which the previous code already
  reproduced exactly and which are untouched).

  The primary is now shrunk by keeping its first component whole and water-filling the rest to an
  equal cap, spreading the remainder one character at a time left to right. That specific rule is
  not a preference — it is what reproduces ICAO's published per-component lengths
  (`[9,12,10,10]` → `9,7,7,6`); handing the whole remainder to the first component that can take it
  gives a conformant but different answer that fails the golden vector.

  Six ICAO worked examples are now pinned byte-for-byte at width 39, through two different emitters
  (TD3 §4.2.3.2(a)/(b) and §4.2.3.3(b); MRV-A §4.2.3.1(a)/(b) and §4.2.3.2(b)). The narrow formats
  satisfy the four normative rules but are deliberately *not* pinned to ICAO's narrow-field
  illustrations, which contradict each other at the same width 31 — Part 6 §4.2.3.2(b) shows
  `<<D<P` where Part 7 §7.2.3.2(b) shows `<<DINGO`. See
  `knowledge/docs9303/CONFORMANCE_BASIS.md` for the full stance and what changed in it.

  The property test that should have caught this
  (`truncated_td3_name_field_always_ends_alphabetic`) forced truncation on every case yet asserted
  only that the last character was alphabetic. It is replaced by
  `truncated_td3_names_keep_the_secondary_identifier`, which uses multi-token names and asserts the
  secondary identifier survives the round trip, plus a fixpoint property (re-emitting from a parsed
  record reproduces the identical line). Verified to fail against the previous emitter.
