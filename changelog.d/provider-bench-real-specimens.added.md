- **`provider-bench` can now run over real specimens in `samples/`, not only the synthetic
  corpus.** `synthpass_bench::load_real_specimens` recursively finds every image under
  `samples/` and attaches optional ground truth from a sibling `samples/ocr_fixtures/<stem>.json`
  label file (the v1 `Extraction` shape) when one exists — a missing label means "unlabelled," not
  a load error, since most of `samples/` (every MRZ-less ID-card front in particular) has never
  been hand-labelled. Run with `provider-bench --real-specimens`.

  This closes a real gap: the old harness derived ground truth exclusively by parsing a
  document's MRZ lines, so a document with no MRZ had no path through it at all — exactly why a
  serious Tier-2 failure mode (the local LLM inventing entire identities on MRZ-less ID-card
  fronts) was found by hand instead of by CI.

  `unsupported_assertion_rate` and `field_match_rate`/`mean_cer` are now reported over the
  populations they actually have, honestly: `AccuracyStats` carries `labelled_documents` and makes
  `field_match_rate`/`mean_cer`/per-field CER `Option`, so an unlabelled specimen contributes zero
  field comparisons instead of being scored as a miss, and an all-unlabelled run reports `None`
  rather than a fabricated `0%`. The unsupported-assertion check itself no longer needs — or is
  gated behind — ground truth at all, so it now runs on every specimen including MRZ-less ones (a
  latent coupling in the old code silently skipped it for any field without ground truth, which
  this also fixes for the synthetic corpus).

  The unsupported-assertion metric is now capability-dispatched: it is computed only for
  `!Capability::vision` providers. The predicate — "the answer must appear, verbatim, in the OCR
  text" — is correct for a text-only provider but invalid for a vision-capable one, which may
  correctly report a value that appears nowhere in the OCR text by reading pixels directly. A
  vision provider now reports `UnsupportedAssertion::NotApplicable { reason }` instead of a
  number, so the next phase's Qwen-vs-Moondream comparison won't score a working VLM as maximally
  hallucinating.

  `DocumentContext::with_image` — present since M7 but called from nowhere in the workspace — is
  now wired: in `provider-bench` (via a temp file kept alive across the whole reader loop, since
  every provider revisits every document) and in `synthpass-pipeline`'s Tier-2 `LlmFieldReader`
  call site (`Pipeline::process_document`), passing the original on-disk file. Harmless for every
  provider shipped today (all text-only, so `DocumentContext::image` is ignored), and required
  before any vision-capable `FieldReader` can work at all.

  Note: `synthpass-pipeline`'s *streaming* Tier-2 path (`process_document_stream`, used by
  `synthpass-serve`) does not go through `DocumentContext`/`FieldReader` at all — it calls the
  older `InferBackend::extract_stream` interface directly. `with_image` is wired at the one
  production call site that actually builds a `DocumentContext` for Tier 2; extending the
  streaming path onto the same contract is a larger, separate change.

  The unsupported-assertion rate is additionally **split by whether the document gave the
  provider an MRZ to anchor on** (`mrz::find_and_parse` succeeding on the OCR text — the
  found/not-found line, matching `EscalationKind::MrzNotFound` versus `MrzChecksumFailed`, not
  the checksum-valid line). A single blended figure hides that the anchored and unanchored
  populations fail in different kinds rather than different degrees, and the unanchored half —
  where a text-only provider has no deterministic anchor at all — is the one nobody had measured.
  Each half carries its own document count, since the two are never the same size. Both are
  `null` when the corpus contains no document of that kind, which is always true of
  `without_mrz_anchor` on the synthetic corpus.
