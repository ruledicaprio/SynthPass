- **`provider-bench`'s real-specimen report records, per document, whether each field the
  ground truth defines was read `exact`, `wrong` or `unread`, and `tools/bench_ab_diff.py`
  reports the transitions.** Each `documents_detail[]` row of a document with ground truth gains
  `field_correctness`, a map from field name to one of the three verdicts; the key is absent for a
  document without truth. It is derived from the same comparison as `accuracy.accepted_reads`, so
  summing the maps reproduces that population's per-field exact counts, and a document without an
  accepted read (or whose reader errored) reads every field `unread`. It holds field names and
  verdicts only, never a value. Report-only: no gate reads it, and `OutcomeRow`, the committed
  ledger and its sha are unchanged. `bench_ab_diff.py` prints, per field, the exact to wrong, wrong
  to exact, exact to unread and unread to exact counts between two arms, and names each document
  with an exact field that stopped being exact, by asset id and field name (arms without the maps
  print "not recorded"). `--expect-identical` does not read `report.json`, so it does not compare
  the maps (#574).
