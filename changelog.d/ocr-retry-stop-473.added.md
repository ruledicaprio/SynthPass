- **`synthpass-ocr`: an opt-in retry-stop oracle that does not trust a repaired reading as much as
  a clean one.** The native OCR retry loop used to stop the instant any pass's text contained a
  checksum-valid MRZ, even when that reading only existed because `mrz`'s damaged-capture search
  repaired it — a checksum-consistent reading is not proof it is correct (issue #473). Setting
  `SYNTHPASS_OCR_STOP=clean` (default stays `first-valid`, byte-identical to today) holds a
  damaged-capture hit instead of stopping on it, and keeps searching for up to
  `SYNTHPASS_OCR_CONFIRM_PASSES` (default 2) more passes, stopping early on the first clean read
  or on a second, independent damaged-capture reading that agrees with the held one on every
  field. If neither arrives, the held reading is accepted unconfirmed once the confirm budget (or
  the existing pass/time budget) runs out. `synthpass_ocr::OcrArms` records the arm as `stop` for
  bench reports.
