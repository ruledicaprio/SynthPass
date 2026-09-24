- **Damaged-zone recovery no longer picks one of several disagreeing readings.** When
  `find_and_parse` rebuilds a damaged zone, it returns a reading only if every recovered
  candidate is the same answer. It used to compare six fields: the document number, the two
  dates, the two names and the nationality. Candidates that differed in sex, issuing state,
  document code, optional data, the full document number or format counted as unanimous, and the
  first candidate won. The rest of the zone decided which one that was.

  No check digit covers sex, the issuing state, the document code or the format, so such a
  reading was checksum-consistent and still wrong. Candidates now have to agree on every
  `MrzData` field except `mrz_lines`, format included. Otherwise the recovery refuses and the
  ordinary checksum-failed result is returned.
