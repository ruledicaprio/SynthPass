- **`examples/bih_mrz_audit.rs`** — a worked example of layering issuer policy *over* the
  generic parser rather than inside it, which is the arrangement this crate's roadmap asks
  for. It recomputes each ICAO 9303 check digit independently, applies an opt-in
  Bosnia-and-Herzegovina profile, and never promotes an observational corpus pattern to a
  conformance requirement — its `<<<` optional-data check is opt-in, reported as a warning,
  and says in the message that it is not an ICAO failure. Run with
  `cargo run -p mrz --example bih_mrz_audit -- --profile auto FILE`.
