Texture suppression is now on by default. Measured over 232 real specimens it lifts the
deterministic Tier-1 hit rate from 111 to 114 documents (48.5% to 50.0%) with no document
regressing, and it costs nothing on a document that reads successfully — the extra passes run
only after every existing treatment has already failed. `SYNTHPASS_OCR_TEXTURE=off` restores
the previous behaviour.
