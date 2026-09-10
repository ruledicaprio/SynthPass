- **ADR-0008's mandated OCR-stack measurement is complete.** Its second amendment
  records the result: the browser-vs-native MRZ gap is **native page-orientation
  handling** (`choose_rotation` turns the page 90° before OCR on the 11
  detection-failure documents), not the OCR-B recognizer, the band search, retry
  ordering, or — as a first-order effect — scale. Full attribution answering
  `Tesseract_OCR_studies.md`'s nine deliverables:
  [`ocr-stack-gap-attribution-2026-09-10.md`](knowledge/benchmarks/ocr-stack-gap-attribution-2026-09-10.md).
  The 1B writeup carries a correction note (`plain_band` *is* called natively; no
  PSM is set on either side).
