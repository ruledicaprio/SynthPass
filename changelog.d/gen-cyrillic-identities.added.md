- **`synthpass-gen` renders Cyrillic-script identities.** Six issuing states (`RUS`, `SRB`,
  `BGR`, `MKD`, `UKR`, `BLR`) now draw a native-script name and store its ICAO 9303 Part 3
  §6 B transliteration — computed with that state's language via `mrz::transliterate_cyrillic`
  — as the Latin `surname`/`given_names` the MRZ and the romanized VIZ line carry. The VIZ
  paints the native Cyrillic name (PT Sans already covers the block); the MRZ stays clean
  `[A-Z]` and round-trips checksum-valid. `Passport` and `Labels` gain
  `surname_native`/`given_names_native` (`None` for Latin-script identities), the `generate`
  sidecar JSON surfaces them, and `synthpass export` emits `surname_native`/`given_names_native`
  blocks alongside the romanized ones. This is the round-trip that makes §6 B measurable.
