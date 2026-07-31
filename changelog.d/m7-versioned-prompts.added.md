- **Tier-2 prompts are now versioned and digest-pinned, closing the first of M7's two remaining
  items.** `synthpass_llm::prompt` exposes `PROMPT_ID` (`"qwen2.5-fields"`), `PROMPT_VERSION`, and
  `prompt_ref() -> synthpass_core::v2::PromptRef`. The digest is `sha256` of the compiled-in ChatML
  `TEMPLATE` with `{content}` left empty — the static system text and field list, not any one
  document's OCR markdown — reusing `synthpass_core::audit::sha256_hex` rather than adding a new
  dependency. `build_prompt` and the digest now fill the same `TEMPLATE` constant, so they cannot
  drift apart.

  `InferBackend` gained `prompt_ref()` (default `None`, mirroring the existing `model_id()`
  pattern so out-of-tree backends stay source-compatible); `NativeInferer` overrides it to return
  `synthpass_llm::prompt::prompt_ref()`. `Pipeline::process_document` and
  `process_document_stream` both now populate `ExtractionTrace.prompt` from
  `self.infer.prompt_ref()` at their Tier-2 call sites, replacing a `prompt: None` stub that had
  sat unpopulated since the field was added.

  A pinned-digest test in `prompt.rs` fails the moment `SYSTEM`, `FIELDS`, or `TEMPLATE` changes
  without a matching `PROMPT_VERSION` bump and digest update — the parity corpus is six documents,
  so a one-word prompt edit was previously indistinguishable from noise for months.

  **Not in this PR:** the multi-provider benchmark harness, M7's other remaining item.
