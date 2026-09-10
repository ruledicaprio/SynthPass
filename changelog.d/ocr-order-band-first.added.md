- **`SYNTHPASS_OCR_ORDER=default|band-first|control`** — a same-binary A/B knob for
  where the untreated `plain_band` crop sits in the native retry chain, added for
  ADR-0008 chunk 1C. `default` (the shipped value) is unchanged production
  behaviour. The measurement it enabled:
  [`ocr-order-band-first-2026-09-10.md`](knowledge/benchmarks/ocr-order-band-first-2026-09-10.md)
  — `plain_band` tried first recovers nothing for `ocrs` on its own merit, confirming
  the long-standing "`ocrs` gains nothing from an untreated pass" assertion and
  redirecting the OCR-gap attribution at the recognizer.
