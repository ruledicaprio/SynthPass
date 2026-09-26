- **Bench reports record the retry-stop confirm budget.** `synthpass_ocr::OcrArms` gains
  `confirm_passes` (`SYNTHPASS_OCR_CONFIRM_PASSES`, default 2), and bench reports print it next to
  `stop`. Two `SYNTHPASS_OCR_STOP=clean` runs with different budgets used to produce identical
  provenance. A moved budget also makes a run non-default, so it cannot write a baseline.
