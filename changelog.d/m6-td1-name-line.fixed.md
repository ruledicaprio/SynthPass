- **`mrz` reads TD1's name line again (`mrz` 0.6.3, no API change).** A TD1's third MRZ line —
  the only place `surname` and `given_names` live — was routinely lost, and the failure was
  invisible because **TD1's check digits do not cover line 3 at all** (document number, date of
  birth, expiry and composite all read from lines 1-2). A TD1 could therefore satisfy every check
  digit while reporting a name read off the watermark. Measured on the synthetic TD1 corpus, the
  Tier-1 hit rate rises from **26.7% to 56.7%** (30 documents, `--seed 42 --profile clean`);
  mean `surname` character error rate was 113% before the fix — above 100%, i.e. the read was
  longer than the truth.

  Two independent causes, both found with the new `synthpass-bench --dump-ocr` probe:

  **A dropped filler broke line 1's own checksum.** OCR does not misread the position-1 filler in
  `I<UTO...` as a lookalike — it drops the glyph outright, reading `IUTO...`, which shifts every
  subsequent field one position left. Unlike TD3, whose check digits all live on line 2, TD1
  carries its document-number check digit *on line 1*, so this single lost character failed the
  checksum rather than merely corrupting `issuing_country`. `variants`'s length-fitting could not
  undo it: a short line is padded by extending its **longest** filler run, which is the trailing
  one, never by reinserting a filler at position 1. A new candidate reading reinserts it and drops
  the compensating trailing character; as with every repair in that module it is offered
  *alongside* the unrepaired reading, and the check digits decide which is real.

  **The three-line scan required strict adjacency.** TD1 was the only format demanding its three
  candidate lines be exactly consecutive in the OCR text — TD2, TD3, MRV-A and MRV-B all tolerate
  a gap, and all four accept the whole zone merged onto one physical line. TD1 now does both
  (bounded to the same three-line lookahead the other formats use). This matters because the OCR
  engine's internal multi-pass retry concatenates several attempts into one text blob: a pass that
  fails to detect line 3 as its own region leaves the watermark, or a repeat of line 1, sitting
  where line 3 should be next to a line 1/line 2 pair read in a *different* pass.

  `synthpass-gen` also gives TD1's mandatory `SYNTHETIC / SPECIMEN` watermark real separation from
  the MRZ band — it sat 8px above it, where TD2 gets ~145px and TD3 ~190px, and was drawn 28px
  tall into a 26px rect. It is what supplied the letters-only decoy lines in the first place. The
  watermark still renders unconditionally on every format; TD3 and TD2 geometry are unchanged.

  Five regressions in `crates/mrz/tests/td1_line_gap.rs` pin the fix, with fixtures emitted by
  `format_td1` rather than transcribed from any real document. They are deliberately honest about
  one thing they do **not** attempt: choosing between two candidate line 3s that are both
  check-digit-silent is not something a parser can decide, and that blind spot is documented
  rather than papered over.
