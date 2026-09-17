- **Fixed-grid MRZ name-line repair (`synthpass_ocr::chargrid`).** New pure module: fits a
  fixed-pitch character grid to the glyphs `ocrs` did read, and reconstructs the collapsed `<`
  fillers the recognizer drops on the MRZ name line, with a check-digit-free arbitration
  gate (document-code/issuer prefix protection, plus a mandatory ink check: the empty cells carrying ink must equal the missing-glyph count)
  so an unrecoverable line is rejected rather than guessed at. Not yet wired into
  `NativeOcr` — that is a follow-up.
