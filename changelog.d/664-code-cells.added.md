- **The archive records the printed document code as classes, and `archive_query.py codes` reads it.**
  A labelled record's `truth` gains `code_cells`, the class symbols of the fixture's first two line-1
  cells (`A<` for `P<`, `I<` and `V<`; `AA` for `PS`, `PO` and `ID`), after its three existing keys; it
  is a class string, never the code, so it is written on every track, and the schema stays 1.
  `tools/archive_query.py codes RUN [RUN2]` prints, per provider and format, the printed class by
  the observed class, the code's `document_type` exact / wrong / unread per printed class, the
  retry variant and stop of the reads whose class differs, and the asset ids of the public
  documents that differ (a local or private record is counted, never named). With a second run
  it prints both matrices and the documents whose class changed, and refuses two runs that are
  not comparable. It prints classes, counts and asset ids, never a character of OCR text (#664).
