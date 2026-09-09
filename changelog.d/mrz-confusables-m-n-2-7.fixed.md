- **`mrz` recovers two more OCR glyph confusions in a checksum-failed read.** The damaged-read
  repair path (`mrz::find_and_parse` only, after nothing validated) now knows `M`↔`N` and
  `2`↔`7` in addition to the round-letter and vertical-stroke pairs it already handled — both
  measured on real specimen scans (`Ghana P0_GHA_2019` reads a printed `M` as `N`; `India
  P0_IND_2013` reads a `2` as `7`). As before, a repair is only accepted when the field's own
  check digit proves it and no other reading also verifies. `mrz` 0.7.1.
