- **The docs no longer overstate which misreads a check digit misses.** `CONFUSABLES` said most of
  its pairs share a residue class. Only `1`/`L` and `6`/`G` do; every other pair crosses residues,
  so a check digit rejects a single such misread. `Blindspot` said a weight-7/weight-3 pair
  cancels at three document-number positions. It is nine of the 36 position pairs, and a shared
  shift of 5 (two `2`↔`7` misreads) cancels at all 36. No behaviour changes.
