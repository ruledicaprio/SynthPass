- **docs.rs shows which items need the `serde` or `zeroize` feature again, and every item has an
  example (#428).** Built on docs.rs (`--cfg docsrs`), the crate enables `doc_cfg`, so each
  feature-gated impl carries an "Available on crate feature … only" badge, derived impls
  included. The attribute is inert on stable and on the 1.82 MSRV, which is why the
  nightly-only `doc_auto_cfg` removed in 0.8.0 is not coming back. Example coverage goes from
  85 of 89 items to 89 of 89: `Format::check_digit_applicability`, `Checks::applicable`,
  `Checks::verified` and `Checks::failed` gain doctests.
