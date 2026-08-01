- **Tier-2 cross-check no longer downgrades correct reads on four classes of document.**
  `synthpass_core::fusion::check_tier2_against_mrz` compared normalized values as plain strings,
  which reported a contradiction — and dropped the field to `IMPLAUSIBLE` confidence — whenever
  the two sides were the same value written two legitimate ways. Fixed for: **German documents**
  (the MRZ prints the legacy single-letter `D`, while any name-based resolution of "Germany"
  yields `DEU`; now compared through the new `mrz::codes_equivalent`, which treats codes naming
  the same entity as equal); **two-character document codes** (`PO` official, `PD` diplomatic,
  `PS` service, `ID` on most TD1/TD2 cards, against a normalizer that can only emit `P`/`I`/`V`
  — now compared on the document-class character only); and **ICAO §4.6 name encoding**
  (apostrophes are dropped with no filler, so `O'Brien` is printed `OBRIEN`, and long names are
  truncated into the fixed-width field — now compared through the new
  `mrz::encode_name_component`, the same encoder that writes MRZ name fields, with a
  truncation-prefix match accepted). Interior filler-width differences are also folded away.
  A genuine disagreement is still flagged, in both directions.
