- **The documentation no longer claims a check digit proves a faithful read.** It does not: a
  valid check digit establishes that the candidate is *checksum-consistent* with the printed
  check digit, over the field that digit covers. It does not establish byte-identity with the
  printed zone — two substitutions the crate classifies as `Caught` can cancel mod 10, and on
  TD2 and TD3 the whole of line 1 (document code, issuing state, both name fields) carries no
  check digit at all. `Blindspot` and `CLASSES` already documented the arithmetic correctly;
  the crate-level, `README`, `checksum`, `parser` and `dates` headlines contradicted them, as
  did `Checks::all_valid`'s own summary line. All are restated in terms of what the
  arithmetic establishes. `MrzData::mrz_lines` likewise no longer calls itself "raw": it holds
  the repaired zone when `find_and_parse` normalized the input.
