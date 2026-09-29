- **The synthetic TD1 MRZ is now drawn at Doc 9303's TD1 line pitch.** `synthpass-gen` used
  to space TD1's three MRZ lines the same way as every other format (55 px, 2.565 cap), well
  outside TD1's own conforming band (1.71-1.84 cap). It now draws TD1 at a 37 px line pitch
  (1.726 cap), matching Doc 9303-5 Figure 6's 4.23 mm: six lines per inch, as tight as ISO 1831
  packs size-I lines. The MRZ block starts lower on the card, and TD1's watermark moves down
  24 px, at the same size, to stay centred in the wider gap above it. TD1 pixels and
  per-character label boxes change for every seed; TD2, TD3, MRV-A and MRV-B are
  byte-identical (#411).
