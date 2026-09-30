- A TD1 line 1 that lost its position-1 filler is now also recovered by the uniform class sweep when
  the shifted search finds nothing. The unshifted reading was skipped whenever the shifted search
  came back empty, so a line whose only repair is the unshifted reading was never offered there;
  it is now searched every time. When no candidate has a resolving issuing state the result is the
  shifted search's, unchanged, so a document with an unknown issuer gains no candidate. This
  affects only the opt-in class sweep (`ParseOptions::with_class_sweep`); the default parse and the
  single-cell damaged-capture search return what they did. No public API change (#580).
