- **OCR noise in an unchecked name cell or one split MRZ row no longer hides the zone.** The
  scanner leaves an unreadable name component uncertified and empty, keeps its raw position in
  the MRZ lines, and rejoins adjacent OCR tokens only when their combined width is exactly
  30, 36, or 44 characters (#408).
