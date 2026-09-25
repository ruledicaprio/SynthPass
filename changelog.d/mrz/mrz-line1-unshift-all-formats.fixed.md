- **A dropped line-1 filler is now undone in the damaged-zone recovery of every format, not just
  TD3 (#476).** When OCR drops the `<` at line-1 cell 1, line 1 shifts left: the document code
  gains a letter, and the issuer becomes the wrong three cells. TD2 and MRV-B line 1 carries no
  check digit, so recovery used to accept the shifted reading. #468's TD3 fix is now applied to
  TD1, TD2, MRV-A and MRV-B in `damaged_pass` and `class_sweep_pass`:
  - the unshifted line 1 is tried alongside the plain one;
  - a candidate whose issuer doesn't resolve is dropped when another one does;
  - two candidates whose issuers both resolve and disagree are refused, not guessed (#440).

  TD1 recovery also tries the unshifted line 1. The ordinary scan is unchanged, so a shifted
  line 1 whose issuer happens to be a real code (`GBR` read as `BRN`) can still be accepted
  there.
