- **`mrz::find_and_parse` no longer surfaces non-MRZ text as a checksum-failed record.** When
  the best-scoring reading never validates *and* fails every structural signal at once — the
  issuing state and the nationality are both unrecognized and the date of birth is not even
  six digits — it is OCR that matched MRZ-shaped visual-inspection-zone text (card boilerplate,
  a printed legend), not a real MRZ read too badly to verify, and `find_and_parse` now returns
  `MrzError::NotFound`. A genuine non-conformant line 1 (a shifted issuing-state field, a
  document with no state) still parses: it keeps a resolving nationality and a real date of
  birth. Measured against a 210-document real-specimen corpus: 18 of 19 no-MRZ documents move
  from `checksum_failed` to correctly not-found, with no regression on any hit. `mrz` 0.7.0.
