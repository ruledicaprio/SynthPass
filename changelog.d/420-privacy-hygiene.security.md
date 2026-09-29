- **`provider-bench` keeps document numbers out of mismatch reasons and guards every OCR dump
  destination.** Document-number mismatch diagnostics now name only the mismatch kind, and
  `--dump-ocr` / `--dump-ocr-hits` refuse an in-tree output directory unless Git ignores it,
  matching `--dump-ocr-passes`.
