- Hand-transcribed ground-truth fixtures for the 23 real passport / ID specimens whose
  printed MRZ lands in the real-corpus `checksum_failed` bucket (`samples/ocr_fixtures/`).
  Each records the true printed MRZ zone read from the specimen scan, so an OCR misread can
  be told apart from a specimen whose printed check digits are wrong by design. Seven of the
  23 carry a checksum-valid MRZ (every `checksum_failed` on those is an OCR error); the other
  16 are non-conforming `TEMPLATE` / `SPECIMEN` / `ÖRNEK` zones. `samples/corpus.jsonl` gains
  `ground_truth_stem` and `expected_document_number` for all 23.
