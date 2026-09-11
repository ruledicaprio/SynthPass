- **OCR no longer turns readable pages sideways.** The upfront orientation probe voted on a
  ratio of detected-text shapes, which is as confident about scanner noise as about a full page
  of MRZ — and on measured documents it committed to a wrong quarter-turn, leaving every
  downstream crop vertical and unreadable. The vote is retired from the default path; both
  quarter-turns are now tried late in the retry chain instead and an ICAO check digit decides
  which one was right, so a genuinely sideways page is recovered by evidence rather than by a
  guess. `SYNTHPASS_OCR_ROTATE=legacy` restores the old probe as a measurement baseline.
  EXIF orientation is now applied when a file carries it, losslessly, before any of this.
