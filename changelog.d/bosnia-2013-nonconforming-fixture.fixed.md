- **Bosnia and Herzegovina 2013 card back is scored as a non-conforming specimen.** Its printed
  zone fails its own TD1 composite check digit. The benchmark previously counted it as a Tier-1
  hit, because damaged-zone recovery invented optional-data characters that made the zone verify.
  With that recovery fixed, the specimen has a reviewed `non_conforming` fixture, promoted from
  the unreviewed `derived/` one. It now sits in `checksum_failed_specimen`, off the scored
  denominator, like the other specimens whose printed zone fails its own check digits.
