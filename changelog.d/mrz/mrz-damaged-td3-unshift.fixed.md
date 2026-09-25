- **TD3's damaged-line-pair repair no longer returns a left-shifted line 1.** When line 1's
  position-1 filler was dropped by OCR *and* line 2 separately needed a single-substitution
  repair to validate, the recovery pass only ever tried line 1's ordinary repair, never the
  shift/unshift repair the ordinary TD3 scan already applies — so it silently returned the
  wrong `document_type`/`issuing_country`/name instead of the correct, unshifted reading (#461).
  It now tries both, exactly as the ordinary scan does; when the two disagree, the read is
  refused rather than guessed.
