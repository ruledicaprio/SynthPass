- `--dump-ocr`'s `field_mismatch_counts`, `field_mismatch_positions` and
  `field_mismatch_coverage` are now absent when the resolved MRZ format does not match the shape of
  the document's ground-truth zone, instead of attributing the zone against a layout that does not
  describe it. A reader that misdetects the format previously produced a well-formed but
  meaningless breakdown — measured on a TD3 specimen read as TD1, it reported differing characters
  on line 3 of a two-line zone and populated TD1-only fields the printed format does not have.
  `Some` now carries a guarantee: the zone really is the declared format.
