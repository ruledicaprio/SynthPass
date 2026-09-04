The escalated Tier-2 provider now receives the same page recognition the Tier-1 provider gets.
Both contexts are built from the same OCR stage, but only the Tier-1 one carried the recognizer's
observations — MRZ-band score, portrait box, page rotation, text sanity — so the provider asked
the harder question, on exactly the documents the first tier could not answer, was told less
about the page than the one that had already succeeded. No output changes today, since the
shipped text-only reader builds its prompt from the page text alone.
