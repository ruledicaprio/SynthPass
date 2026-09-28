- **The synthetic MRZ is now drawn at the ISO character pitch.** `synthpass-gen` used to space
  MRZ characters by dividing each line's width by its character count: 24 px on TD3 and TD1,
  25 px on TD2 and MRV-B, 23 px on MRV-A. That is 4-13% wider than ISO 1073-2 / ISO 1831's
  2.54 mm pitch. Every format now draws a fixed 22 px cell (1.026 cap), so generated MRZ pixels
  and per-character label boxes change for every seed, on every document type. The watermark
  band's position and size are unchanged in every format (#411).
