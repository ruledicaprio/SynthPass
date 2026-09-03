- **The parity harness can now measure the escalation case it was blind to.** Every parity fixture
  has a checksum-valid MRZ — that is where its ground truth comes from — but Tier 2 only runs when
  Tier 1 *fails*, so the corpus was by construction the set of documents Tier 2 never sees in
  production. Fine as a regression check, wrong for predicting escalation accuracy, which is the
  question the VIZ work exists to answer.

  `SYNTHPASS_PARITY_HOLDOUT=1` strips the MRZ-derived lines from each fixture's OCR text before the
  model sees it, and scores against the ground truth those lines produced. The input becomes the
  escalation case; the truth stays checksum-proven because it was derived beforehand; and the
  proven fields are all printed in the visual zone in human-readable form, so they remain
  recoverable in principle.

  Built as a transform rather than the variant fixture set `VIZ_TIER2_DESIGN.md` §5.2 originally
  scoped, because the generator needs the source images from the `samples-data` branch and a re-OCR
  pass is not byte-deterministic — whereas every fixture's `.json` already carries `mrz_line`
  verbatim, right where the harness reads it. No new files, no schema change, no prompt change.

  The strip predicate is a union of a shape test and similarity to the fixture's own known MRZ,
  deliberately biased toward stripping. The two errors are not symmetric: a leaked MRZ fragment
  hands the model the answer and *inflates* the score, which destroys the number's meaning, while
  over-stripping a visual-zone line only costs context. Measured over all 72 fixtures, the shape
  test alone strips 400 lines but misses 17 short corrupted fragments such as
  `KOVACEVIC<AZRA<MARINA<<<` that spell out surname and given names; the union strips 417 and
  leaves 2553. The charset test deliberately does not upper-case first — an MRZ is uppercase-only,
  so lowercase is evidence *against*, and normalising case would let prose like
  `Passaport/Passport/Passeport` score as MRZ and strip the visual zone the measurement depends on.

  `holdout_strips_every_fixture_on_disk` is a model-free pre-flight that asserts no fixture keeps
  its MRZ and none is stripped to nothing. It runs in milliseconds and guards a ~31-minute run
  against its own worst failure mode. Holdout runs apply no accuracy floor: the floors gate
  regressions on the standard corpus, and a holdout measures a harder task where a low number is
  the finding rather than a red build.
