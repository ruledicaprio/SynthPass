- **`mrz` now emits and reads ICAO 9303-conformant long document numbers (BREAKING, `mrz` 0.6.0).**
  Doc 9303 part 5 note j, part 6 note j and part 5 §4.2.4 put all **nine** principal characters of a
  long document number in the number field and replace only the check-digit position with a filler
  (part 4 defines no overflow encoding for TD3 at all; applying the TD1/TD2 rule there is a
  deliberate extension, because issuers do it in practice). This crate was
  writing eight characters plus a filler instead, and computing the reassembly check digit over the
  wrong input. Because the parser also required the number field to end with `<` to recognize an
  overflow — true for the crate's own wrong encoding but not for a conformant nine-character field —
  it could not read a spec-conformant long-document-number MRZ at all, silently falling back to a
  truncated, check-failing nine-character reading.

  `format_td3`/`format_td2`/`format_td1` now print the full nine principal characters before the
  filler. `parse_td3`/`parse_td2`/`parse_td1` now try the spec-form reassembly first, fall back to
  the pre-0.6 eight-character legacy form so zones written by an older version of this crate still
  parse, and — if neither form's check digit verifies — surface the spec-form reassembly with
  `checks.document_number == false` rather than silently truncating. `MrzData` gains
  `document_number_legacy_encoding: bool` (additive, `#[non_exhaustive]`) reporting which form a
  reassembled number came from.

  Consumers that inspect the raw MRZ bytes of a long document number (upper-line positions 6-14 for
  TD1, lower-line positions 1-9 for TD2 and TD3) will see the ninth principal character where a
  filler used to be, and correspondingly one fewer character at the start of the optional-data
  remainder. The check-digit position itself is unchanged and, as before, always holds `<`.
  `document_number_full`/`full_document_number()` values do not
  change for any number that already round-tripped correctly through this crate's own emit/parse
  pair; only zones written by 0.5.x or earlier are affected, and those still parse via the legacy
  fallback.
