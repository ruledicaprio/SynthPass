- **`mrz` recovers a dropped MRZ line-1 position-1 filler on TD3/MRV-A/MRV-B (`mrz` 0.6.3, no API
  change).** The same OCR-glyph-drop mechanism `repair_td1_line1_unshifted` already fixes for
  TD1 (`P<BRA...` read as `PBRA...`, shifting every field from `issuing_country` onward one
  position left) was never generalized to TD3/MRV-A/MRV-B. It was silent there: TD1 needed the
  fix because TD1's own document-number check digit lives on line 1, so the shift broke checksums
  outright; TD3/MRV-A/MRV-B have no check digit on line 1 at all, so the identical corruption
  never failed a Tier-1 hit, it just sat behind one — `document_type`/`issuing_country` CER of
  6.7%-76.7% across formats, previously (incorrectly) attributed to VIZ-zone OCR legibility rather
  than the MRZ zone.

  Fix: `repair_td3_line1_unshifted`/`repair_mrv_a_line1_unshifted`/`repair_mrv_b_line1_unshifted`,
  tried as a genuine second candidate *before* the ordinary repair at every TD3/MRV-A/MRV-B call
  site — ordering matters here, unlike TD1, because none of these formats have a line-1 check
  digit to arbitrate between candidates, so whichever is tried first wins whenever both parse. An
  earlier version applied the unshift unconditionally in-place and, despite fixing the CER to 0%,
  measured a real Tier-1 hit-rate regression (TD3: 76.7% → 63.3%, 30 documents) from a `variants()`
  candidate-deduplication interaction — not shipped. The safe version measures, same corpus: hit
  rate unchanged on all three formats; `document_type`/`issuing_country` CER: TD3 76.67%/52.22% →
  23.33%/16.67%, MRV-A 6.67%/4.44% → 0%/0%, MRV-B 40.00%/26.67% → 3.33%/2.22%. Partial, not
  complete, on TD3/MRV-B — some corrupted readings still win when nothing on line 1 discriminates
  between candidates. TD2 is deliberately excluded: it shares TD1's genuine two-letter
  document-code family (`"ID"`, `"AC"`, ...) with no checksum to disambiguate a wrong unshift, a
  harder case left for later.

  Four regressions in `crates/mrz/tests/line1_prefix_shift.rs` pin the recovery on TD3/MRV-A/MRV-B
  and confirm a genuine TD2 two-letter document code survives untouched.
