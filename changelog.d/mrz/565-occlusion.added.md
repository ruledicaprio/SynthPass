- **`apply_occlusion`.** Apply an image-derived `CellMask` to an already-parsed `MrzData`: a
  masked check-covered or structural cell refuses the call (`MrzError::OccludedCheckedCell`)
  rather than reporting a value the check-digit arithmetic cannot back up; a masked unverifiable
  cell withholds its `ZoneField` (listed in the new `Occluded::fields` and blanked in
  `Occluded::data`), following ICAO's own name grammar for the surname/given-names split. An
  empty mask is always the identity. Additive: no existing function, type or field changes.
