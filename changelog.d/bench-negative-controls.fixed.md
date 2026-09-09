- **42 correct refusals were being scored as detection failures, and a hallucinated MRZ was
  being scored as a hit.** `provider-bench --real-specimens` walks the image directory and never
  consulted `samples/corpus.jsonl`, so the 42 specimens that carry no machine-readable zone at
  all — ID-card fronts, border passes, driving-license faces — were scored exactly like a
  passport whose MRZ the pipeline failed to find. Reading no MRZ off a document that has none is
  the *correct* Tier-1 answer, but it landed in `no_mrz_found`, inside the hit-rate denominator.
  A new `no_mrz_expected` outcome scores them out the way `redacted_mrz` already is, resolved
  from the manifest's `mrz.present` (with the `_no_mrz` filename tag as a fallback — two
  driving-license fronts predate that convention and only the manifest gets them right).
- **The same gap hid a real defect in the other direction.** A document with no MRZ has no
  `ocr_fixtures/` label to be checked against, so a checksum-**valid** read off one passed the
  found gate, passed the checksum gate, found no ground truth to contradict it, and was counted
  as a **Tier-1 hit**. That is a hallucinated record scored as a success. It is now
  `false_positive_mrz`: inside the denominator, in the regression buckets, and printed as a
  warning rather than a count. `Monaco_ID_Specimen_XXXX_front_no_mrz.png` really did produce one
  (it turned out to be a mislabelled file), which is precisely why the harness has to say
  something when it happens.
- **`--assert-baseline` now reports the denominator moving.** `scored` was recorded in the
  baseline and compared against nothing, so a reclassification that leaves the HIT count intact
  and grows no bucket passed every check in silence — while changing the published hit rate.
  Both off-denominator populations now warn on drift, and the two places that compute `scored`
  carry a note that they must agree.
