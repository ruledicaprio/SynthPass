- **Texture suppression, measured on real specimens: +2 documents, and a better idea from the
  control arm.** The three-arm run over 232 real specimens satisfies the pre-registered decision
  rule — `on` 113/229 (49.3%) > `control` 112 >= `off` 111 — with zero hit→miss regressions. Both
  gained documents came from `checksum_failed`, and the miss-kind delta is exactly the predicted
  signature (`checksum_failed -2`, `no_mrz_found 0`), which is what makes a small gain believable
  rather than coincidental. Real, and small: +0.9pp.

  The placebo arm turned out to be the most informative one. `plain_band`, used purely as a
  no-new-pixel-maths control, fixes a *different* miss class — `no_mrz_found`, a detection failure
  — where the median filter fixes `checksum_failed`, a recognition failure. They currently compete
  for the same variant slot, so neither run gets both: one Swiss specimen validates under the
  placebo and not under the treatment, and two others do the reverse. Running both as variants 9
  and 10 is the next step, and it needs its own measurement rather than an assumption.

  The stage stays default-off until that lands, so the default flips once rather than twice.
