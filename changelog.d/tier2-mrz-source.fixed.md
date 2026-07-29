- **Tier-2 JSON's `mrz` field no longer disagrees with the pipeline's own MRZ read.** When Tier 1
  escalates to Tier 2, the persisted `ExtractionV2.mrz` used to come from the LLM's own `mrz_line`
  guess (`synthpass_llm::prompt::FIELDS` asks for one) rather than `stage.mrz_data` — the
  deterministic `mrz::find_and_parse` result already computed in `ocr_and_tier1` and used to drive
  the escalation decision itself. The two could — and, on real photos, reliably did — disagree: the
  CLI's stdout diagnostic reported the accurate partial-checksum read while the saved JSON showed a
  short, unrelated fragment with every check digit `false`.

  Both Tier-2 entry points now install the deterministic read through one shared
  `apply_deterministic_mrz`, which mirrors `synthpass_die::mrz_reader::extraction_v2_from_mrz` field
  for field: `mrz`, `document.mrz_format`, `line1_integrity`, and the confidence downgrade that
  verdict implies. A real but checksum-partial candidate when one was found, `None` (not a
  fabricated guess) when it wasn't. `mrz_format_of` and the new `mrz_block_from` are `pub` in
  `synthpass_die::mrz_reader` so there is exactly one construction of an `MrzBlock`.

  This covers `process_document_stream` — and therefore `synthpass-serve`, which uses only the
  streaming path — where the first pass of the fix had not reached: every web/API upload that
  escalated to Tier 2 was still writing the divergent record. It also no longer derives that path's
  v1 projection from the raw model output, so the v1 and v2 records ship in agreement. A test now
  asserts the two paths produce *equal* extractions for the same document.

- **`line1_integrity` is populated on the Tier-2 path.** Escalating to a model does not make the
  zone unreadable, so a suspect line 1 on an escalated document is now reported rather than
  silently dropped — in the JSON, and in the CLI's `NeedsReview` warning, which previously printed
  only on the Tier-1 branch.

- **v1's `mrz_checksums_valid` reports the real verdict on the Tier-2 path.** It was hardcoded
  `null` on the grounds that the model is never asked for the checksum bit. That was true while
  `mrz_line` was the model's guess; it stopped being true once `mrz_line` became the deterministic
  read whose failing check digits are what triggered the escalation.
