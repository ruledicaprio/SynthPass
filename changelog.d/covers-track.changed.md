- **Cover-only specimens move to their own `samples/covers/` directory**, outside the default
  `provider-bench --real-specimens` walk — opt in with `--include-covers` (rejected without
  `--real-specimens`, and refused together with `--write-baseline`/`--assert-baseline` the same
  way `--include-local` is). A cover never carries an MRZ, so it never enters the scored
  denominator; walking a growing covers track on every PR would cost the real-specimen gate real
  OCR minutes for zero accuracy signal. The `cover` filename token is unchanged — it still says
  what an image is — but it no longer decides where the file lives. `synthpass_bench` gains
  `SpecimenClass::Cover` and `OptInTracks::covers`; `--format cover` selects the track when
  `--include-covers` is also set. This amends [`ADR-0012`](knowledge/decisions/ADR-0012-cover-only-specimens-are-a-labelled-class.md),
  which originally rejected a separate directory.
