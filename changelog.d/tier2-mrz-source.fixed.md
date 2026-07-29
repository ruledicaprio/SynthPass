- **Tier-2 JSON's `mrz` field no longer disagrees with the pipeline's own MRZ read.** When Tier 1
  escalates to Tier 2, the persisted `ExtractionV2.mrz` used to come from the LLM's own `mrz_line`
  guess (`synthpass_llm::prompt::FIELDS` asks for one) rather than `stage.mrz_data` — the
  deterministic `mrz::find_and_parse` result already computed in `ocr_and_tier1` and used to drive
  the escalation decision itself. The two could — and, on real photos, reliably did — disagree: the
  CLI's stdout diagnostic reported the accurate partial-checksum read while the saved JSON showed a
  short, unrelated fragment with every check digit `false`. `process_document` now builds `v2.mrz`
  (and `v2.document.mrz_format`) from `stage.mrz_data` directly, the same construction
  `synthpass_die::mrz_reader::extraction_v2_from_mrz` already uses for Tier 1: a real but
  checksum-partial candidate when one was found, `None` (not a fabricated guess) when it wasn't.
  `mrz_format_of` is now `pub` in `synthpass_die::mrz_reader` so the pipeline can reuse the mapping.
