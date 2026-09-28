- **Line 1 of TD3/TD2/MRV-A/MRV-B no longer corrupts the issuing state or document code when
  fitting a short OCR line.** Recovering a short line 1 used to inflate whichever filler run was
  longest to make up the missing length; when OCR noise broke the trailing name-field padding
  into isolated single-cell runs, the one-cell document-code filler could tie for longest and win,
  pushing the issuing-state slot's own bytes into the name field and reading the issuer back as
  `<<<`. A short line now pads the missing cells onto the tail instead whenever the longest run
  starts inside the document-code/issuing-state prefix (before the name field), leaving the prefix
  untouched.
