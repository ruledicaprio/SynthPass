- **Visual, one-click packet review artifact.** `tools/build_review_artifact.py` turns a
  structured `packet-cNN.json` (the JSON twin of a Markdown specimen packet) into a single
  self-contained HTML file: one card per candidate with its staged image embedded inline
  (base64, fully portable, no relative-path dependency) and Public/Local/Drop buttons. Saving
  writes a `verdicts.json` (via the browser's native file-save where supported, falling back to a
  download); reopening the page, or regenerating it against an updated packet, carries prior
  verdicts forward so a review session can be amended without losing earlier decisions.
  `tools/apply_verdicts.py` syncs a saved `verdicts.json` back into the packet's own Verdict
  column, so the plan's existing `packet-cNN.md` stays the single record P-APPLY-VERDICTS reads —
  this is a nicer front door onto it, not a new source of truth. Neither tool decides
  public/local/drop or a licence class, and neither writes a holder value anywhere.
