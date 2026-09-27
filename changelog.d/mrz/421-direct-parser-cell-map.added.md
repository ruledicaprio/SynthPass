- **README table: what a passing `parse_*` call guarantees, and what it does not.** Apart from the
  MRZ alphabet, only line 1's first cell (the document code) is checked outside the check digits.
  The second document-code character, issuing country, nationality, sex vocabulary and calendar
  plausibility are accepted exactly as printed by every direct `parse_td1`/`parse_td2`/
  `parse_td3`/`parse_mrv_a`/`parse_mrv_b` call — those guards live only in `find_and_parse`'s
  repair gates. `1`/`L` and `6`/`G` are the only `CONFUSABLES` pairs a check digit cannot
  separate, and no check digit covers the document code, issuer, name, nationality or sex on any
  of the five formats. No behaviour changes.
