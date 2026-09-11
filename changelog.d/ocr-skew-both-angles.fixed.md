- **The MRZ deskew pass now tries both skew estimates instead of picking one**, taking the
  real-specimen Tier-1 hit rate from **142/157 to 143/157** with no document lost. The new
  projection estimator (`estimate_skew_deg`, ±10° at 0.5° steps, no per-angle resampling) is
  faster and finer than the search it replaces, but measured on its own it is a wash — **+1/−1**
  against the legacy search, recovering a tilted India passport and losing a Swiss ID card back
  whose 232×57 MRZ band is so nearly information-free that the variance objective is flat and the
  estimator correctly declines to rotate it at all. The legacy contrast score does rotate it, and
  that rotation is what reads. Neither estimate is wrong, so `SkewMode::Default` now appends the
  legacy angle as one further trailing variant whenever the two disagree — compared as *angles*
  before any second rotation, so a clean upright band builds no duplicate, and reached only by
  documents already running the whole retry chain. The retry budget grows accordingly
  (`MAX_RETRY_VARIANTS` 12 → 13), which is what keeps the reordering non-regressive rather than
  silently truncating the legacy angle on dense bilingual scans.
