- **The OCR-pass dump records what the chargrid name-line repair did, when that arm runs.** With
  `SYNTHPASS_OCR_CHARGRID` set to `on` or `control`, the pass that read the MRZ carries a `chargrid`
  object in `provider-bench-ocr-passes.jsonl` and in `synthpass-bench --ocr-passes`. It holds the
  matched line's box, each glyph's position, the fitted pitch and origin, the cell each glyph was placed
  in, the ink in each cell and each column, and the repair outcome, with a hash of the measured band
  rather than its pixels. Each `provider-bench-ocr-passes.jsonl` row also gains `chargrid`, the verdict
  the page reports. Off by default and report-only: the arm stays off, and with it on, recording changes
  neither the OCR text nor any outcome (#575).
