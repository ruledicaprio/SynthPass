- **Chargrid MRZ name-line repair, wired in behind `SYNTHPASS_OCR_CHARGRID` (off by default).**
  `ocrs` never emits an isolated `<` filler, so a name line like `KOVALENKO<<ANDRII` can come back
  `KOVALENKOANDRII` even on an otherwise checksum-valid Tier-1 hit. Setting
  `SYNTHPASS_OCR_CHARGRID=on` re-recognizes the matched name line's characters on the image the
  valid MRZ was actually read from, fits them to the format's fixed-pitch grid, and — only when
  the fit is corroborated by the image's own ink and re-verified to change nothing else the MRZ
  parsed — prepends a corrected MRZ block ahead of the original OCR text. `control` runs the
  identical pass as a cost-matched placebo with no repair applied. This is a measurement arm, not
  yet promoted: the default (`off`, or the variable unset) is byte-for-byte unchanged, and
  `--write-baseline`/`--assert-baseline` now refuse to run unless every `SYNTHPASS_OCR_*`
  measurement knob — including this one — is at its default.
