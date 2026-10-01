- **The crate's tests build from its crates.io source again.** The package shipped two
  integration tests that read SynthPass's `samples/` corpus, which the crate does not include, so
  `cargo test` on the published source failed to compile. Both are now left out of the package;
  they still run in the SynthPass repository.
