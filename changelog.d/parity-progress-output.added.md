- **The parity harness now says how far in it is.** The run takes tens of minutes and is routinely
  interrupted; it already printed a header per fixture, but with no counter, no elapsed and no
  estimate, so an interrupted run could not be told apart from a stalled one and there was no way
  to judge whether waiting was worthwhile. A run killed partway through this week left output that
  looked, at a glance, like it had never started — it had actually reached 49 of 72 documents.

  Each fixture header now carries `[n/total]`, its own duration, elapsed wall-clock and an ETA.
  The estimate uses the mean so far rather than the last document, because per-fixture cost varies
  several-fold with OCR text length and a trailing estimate would swing wildly enough to read as
  noise. Same information `provider-bench` gained for the same reason.
