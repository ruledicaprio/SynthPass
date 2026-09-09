- `provider-bench --real-specimens` sub-classifies a `checksum_failed` miss. When the specimen
  carries a hand-transcribed `mrz_line` ground-truth label and the run's OCR recovered that
  exact zone, the miss is reported as `checksum_failed_specimen` — the printed document's own
  check digits are non-conforming, not an OCR error. `MissReason::ChecksumFailed` gains a
  `specimen_nonconforming` flag; `--dump-ocr` rows gain `ground_truth_mrz` and `zone_mismatch`
  (the character-mismatch count against the transcription). Unlabelled specimens and the
  synthetic corpus are unaffected.
