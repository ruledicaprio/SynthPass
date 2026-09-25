# MRZ strip Phase 0: mechanism-classification method

**Status:** method only; no new real-specimen OCR measurement is reported here.

The current [real-specimen baseline](real-specimen-mrz-baseline.json) has 261 assets, of which
152 enter the Tier-1 score. Those counts belong to the pinned baseline, not to this method's
unrun instrumented pass. The [outcome ledger](real-specimen-outcomes.jsonl) uses `asset_id` as its
join key. Names are display labels only: the baseline population has 261 asset IDs and 259 names.

## Acquire the local evidence

Run `provider-bench --real-specimens --mrz-only --dump-ocr --dump-ocr-hits` with `--out` under
`artifacts/`, after scheduling the roughly 50-minute OCR pass. The two dump flags already cover
scored misses and hits; this method adds no new all-documents flag. Dumps cannot be combined with
`--include-private`. Never use `samples/private/` for this track.

The output directory contains `provider-bench-miss-ocr-dump.jsonl`, the same-run
`provider-bench-ocr-outcomes.jsonl` ledger, and a
`provider-bench-ocr-run-<sha256>.json` manifest. A small `provider-bench-ocr-current-run.txt`
pointer connects the current invocation to that content-addressed manifest. Each dump row carries its `asset_id`, SHA-256 of
the original encoded image bytes, and the manifest filename. The manifest pins the start time, Git commit,
dirty-tree state, exact CLI flags, MRZ century pivot, resolved OCR arms, corpus-manifest hash, and
loaded/labelled counts. `raw_ocr_text` means **the text handed to the provider**. It may join
retry attempts; it is not necessarily the first recognizer pass. The dump also carries recovered
and hand-transcribed zones when available. All text-bearing files stay in gitignored `artifacts/`.

Then classify locally:

```powershell
python tools/classify_mrz_mechanisms.py `
  --dump artifacts/provider-bench-miss-ocr-dump.jsonl `
  --ledger artifacts/provider-bench-ocr-outcomes.jsonl `
  --out artifacts/mrz-phase0-mechanisms.json
```

Use the ledger **from the same run**. A changed outcome makes a historical join misleading.
The script rejects missing or duplicate asset IDs in the dump/ledger. It reports the actual total
and scored denominators from the supplied ledger, missing dump rows, and a PII-safe row per asset.
It never copies raw OCR or truth text into its output.

## Read the labels

An asset may have several diagnostic labels: `band location`, `wrong line/attempt association`,
`format/window selection`, `indel`, `substitution`, `inherited repair`, and
`printed non-conformance`. They are **string-level candidate explanations ranked by edit cost;
not observed mechanisms**. In particular, an absent band score is only a band-location indicator; a score
alone does not say whether detection was right. A parser-repaired line absent from provider input
is marked `inherited repair` without assuming the repair helped. Check-digit agreement says only
that a reading is consistent with its printed digits.

For each labelled truth line, the script finds the nearest MRZ-like provider-input line and
compares unit-cost Levenshtein with Hamming distance. A second dynamic program computes the best
edit script that contains at least one insertion or deletion. A shorter/longer candidate, or
Levenshtein strictly below Hamming, indicates an indel. Equal Levenshtein and Hamming distances
are a **tie only when an indel-containing script reaches the same cost**; otherwise the relation
is substitution. Equal-cost competing source lines are a separate `candidate_tie`. The script
does not turn a genuine tie into “no shift.” These comparisons aid manual review, especially when
joined retries have lost per-attempt provenance.

An observed shift needs geometry: per-character OCR boxes or positions from a fitted cell grid
must show a glyph physically occupying a different cell. String comparison alone cannot show
that. Collecting such geometry belongs in the detection layer described by the ADR-0015
amendment and the chargrid grid fit; this Phase 0 script does not collect it. The fixed grid says
where cells should be, geometry says where glyphs are, and check digits judge consistency of a
proposed reading.

Every scored miss is a review target. A hit is a review target when the ledger reports a name
error, or when a labelled zone differs from its fixture on any line. The 2026-09-24 run used the
earlier line-1 rule; its results note records the TD1 case it missed. A hit lacking fixture truth
is explicitly `unlabelled`; it is never counted as correct. `checksum_failed_specimen` is outside
the scored denominator and receives the `printed non-conformance` label from the ledger. The
output retains both the total and scored populations, and never deduplicates by display name.

Manual adjudication remains necessary for whether an MRZ band was visible, which retry or line
produced a candidate, whether a printed zone is non-conforming, and which errors an aligner could
correct. This method collects evidence for that decision; it does not implement or authorize the
aligner.
