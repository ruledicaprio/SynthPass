- **The Tier-1 denominator counted 94 documents that could never be read, and the hit rate was
  30 points low as a result.** `provider-bench --real-specimens` scored every image under
  `samples/` against "did you produce a checksum-valid MRZ", including specimens where no
  pipeline could: 42 carry no machine-readable zone at all (ID-card fronts, border passes,
  driving-licence faces), 36 have the zone blacked out by whoever published the specimen, and 16
  print a zone whose own ICAO check digits fail. Reading nothing off those is the *correct*
  answer on a product whose whole positioning is that it refuses rather than guesses. Each is now
  scored out under its own outcome — `no_mrz_expected`, `redacted_mrz`,
  `checksum_failed_specimen` — and both rates are published: **119 / 144 = 82.6%** on documents
  that can yield a hit, and **119 / 238 = 50.0%** across the whole corpus. The HIT count did not
  move and no extraction code changed. Full analysis in
  `knowledge/benchmarks/denominator-correction-2026-09-09.md`.
- **Two of the three were decided by OCR noise rather than by the document.** The redaction gate
  sat *after* the "was an MRZ found" check, so a blacked-out specimen was scored out only if its
  redaction bar happened to OCR into parseable noise — 9 of 36 did, and the other 27 were counted
  as detection failures. That ran backwards: the cleaner the blackout, the worse the document
  scored. Non-conformance required OCR to recover the hand transcription byte for byte, which
  recognised 1 of the 16 specimens the 2026-09-08 writeup had already concluded "belong outside
  the denominator". Both are now derived from the document alone.
- **A hallucinated MRZ was being counted as a Tier-1 hit.** A document with no MRZ has no
  `ocr_fixtures/` label to contradict — that population is precisely the unlabelled one — so a
  checksum-**valid** read off one passed the found gate, passed the checksum gate, found no ground
  truth, and fell out of the classification as a success. It is now `false_positive_mrz`: inside
  the denominator, in the regression buckets, and printed as a warning rather than a count. The
  current corpus has zero. `Monaco_ID_Specimen_XXXX_front_no_mrz.png` really did produce one once.
- **`ADR-0008`'s target metric shrank from 85 documents to 18**, and the 18 are now named. Its
  decision is unchanged — detection still outnumbers character accuracy, 2.6:1 — but four fifths
  of the number the track was aimed at could never have moved, and a detector that fixed every
  genuine failure would have read as a failure against the old metric.
- **`--assert-baseline` now reports the denominator moving**, and `check-headline-numbers.sh`
  checks both published rates, the dominant-miss multiplier, and fails outright if a committed
  baseline ever records a false positive. `scored` was previously recorded in the baseline and
  compared against nothing, so a reclassification that left the HIT count intact and grew no
  bucket passed every check in silence — which is exactly what this change is.
