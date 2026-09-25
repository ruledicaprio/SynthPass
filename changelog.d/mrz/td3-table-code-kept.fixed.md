- **A genuine Part 4 §4.4 TD3 document code is no longer rewritten into another country.**
  `find_and_parse` was unshifting a real second document-code letter (`PP`, `PD`, `PS`, `PR`, ...)
  whenever the code's letter plus the issuer's first two letters happened to spell a different
  real ISO/ICAO state — e.g. a genuine Nigerian `PP` passport (`PPNGA...`) was read back as
  document code `P` and issuer `PNG` (Papua New Guinea). The as-read line is now kept whenever it
  already carries a resolving §4.4 code and issuer, before the unshifted reading is even tried.
