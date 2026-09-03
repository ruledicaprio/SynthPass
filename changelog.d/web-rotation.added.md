- **A sideways or upside-down photo scans now.** The browser demo had no orientation handling at
  all, and the failure was a collapse rather than a degradation: turning the corpus 90° took it from
  25/40 reads to **1/40**, because "bottom 45 %" is the wrong edge of a turned page and the whole
  band strategy has nothing left to stand on. Turned documents now read at 85–92 % of the upright
  rate — 24/40 at 90°, 23/40 at 180°, 22/40 at 270° — against an upright ceiling of 26/40.

  Upright scanning went **125 → 127** with **zero losses**: two corpus documents
  (`Australia_..._2015_redacted`, `Iran_..._2017_redacted`) turn out to have been misoriented all
  along. Median latency is unchanged (a successful scan never reaches a rotated pass); p90 rises
  12.8 s → 22.1 s, paid only by documents that were already failing every upright attempt.

- **Rotation is strictly trailing, and that ordering is load-bearing.** The first version detected
  page orientation up front and rotated before scanning. The probe — `projection_contrast`, the
  row-density variance `deskew` uses — was wrong often enough to **cost 9 upright documents**
  (125 → 116); instrumenting them showed 8 of 10 upright passport pages reading as sideways, at both
  landscape and portrait aspect ratios.

  The cause is not tuning. `projection_contrast` measures variance of the density profile, which
  captures *any* large-scale layout structure, and a passport data page has a dark portrait photo on
  one side — so column density varies far more than row density. The statistic was built for
  `deskew`, where the page is already near-upright and the comparison is between small angles of the
  *same* image; that assumption does not survive comparing an image against its transpose.
  `synthpass_ocr::choose_rotation` avoids it by scoring detected text-line geometry, which needs a
  word detector the browser does not have.

  So the probe was removed rather than re-tuned, and the upright chain is left untouched: rotations
  are appended and reached only after every upright attempt has failed, in fixed order 90 → 270 →
  180, two cheapest treatments each. The additive contract now holds by construction rather than by
  measurement luck, and the ICAO check digits decide which orientation was right. **Detection that
  can be wrong must not be allowed to rewrite the input.**

- **The harness can rotate the corpus** (`--rotate 90|180|270`), re-encoding each image turned before
  the scanner sees it. The corpus holds exactly one rotated specimen, so without this there was no
  way to measure orientation handling at all.
