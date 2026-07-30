- **`mrz` adds seven missing ICAO 9303 Part 3 §5 registry codes and an explicit unknown-date
  representation (`mrz` 0.6.1, additive).** A full diff of every 3-letter code in the Part 3 §5
  registry (Parts A-H) against `crates/mrz/src/countries.rs` found seven codes this crate could
  not yet name: `XCE` (Council of Europe), `XES` (Organization of Eastern Caribbean States),
  `XMP` (Parliamentary Assembly of the Mediterranean), `XDC` (Southern African Development
  Community), `ANT` (Netherlands Antilles, deprecated), `NTZ` (Neutral Zone, deprecated), and
  `IAO` (ICAO itself, used only when it digitally signs a master list). All seven now resolve
  through `country_name`/`code_for_name`.

  **New:** `mrz::DateCompleteness` and `mrz::date_completeness` classify a raw six-character MRZ
  date field as `Complete`, `PartiallyUnknown`, `Unknown`, or `Malformed`. Doc 9303 Part 3 §4.8
  lets an issuer fill an unknown date of birth with `<`, and a filler counts as zero for
  check-digit purposes (§4.9) — so an all-filler date of birth with check digit `0`
  (`<<<<<<0`) is a mathematically *valid* field, not a corrupt read, and this crate's arithmetic
  already handled it correctly. What was missing was a way for a caller to tell "the issuer
  conformantly recorded an unknown date of birth" apart from "the OCR read failed":
  `expand_date`/`expand_date_with_pivot` leave non-numeric input untouched, so
  `MrzData::date_of_birth` held the same shape (`"<<<<<<"` or garbage like `"7X0812"`) for both
  cases, and a complete date is reshaped from six characters to ten (`YYYY-MM-DD`) so the
  original raw field is not recoverable after the fact.

  `MrzData` gains `date_of_birth_completeness: DateCompleteness` (additive, `#[non_exhaustive]`),
  populated by all five parsers (`parse_td1`, `parse_td2`, `parse_td3`, `parse_mrv_a`,
  `parse_mrv_b`). `Checks` and `DateValidity` are unchanged — `DateValidity::dates_well_formed`
  correctly stays `false` for an unknown date of birth (it genuinely isn't a parseable calendar
  date); callers distinguish "legitimately unknown" from "garbage" via the new field instead.
