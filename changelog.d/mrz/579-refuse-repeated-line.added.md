- **`ParseOptions::refuse_repeated_line`, an opt-in refusal of a zone in which one line repeats
  another.** Off by default; the default parse returns what it did. With
  `ParseOptions::default().with_refuse_repeated_line(true)`, `find_and_parse_with` drops a
  checksum-valid zone, from the ordinary scan or the damaged-capture pass, when two of its lines
  are near-identical: the repeated-line check that has ranked such a zone below an alternative
  since 0.9.1 (#593). The scan continues as if that zone had not validated. An unflagged valid zone
  wins, then another flagged one, then the checksum-failed reading; when nothing is left, the new
  `MrzError::RepeatedLine` says which two lines repeat. Additive: `ParseOptions` and `MrzError` are
  `#[non_exhaustive]`. Not measured as a default (#579).
