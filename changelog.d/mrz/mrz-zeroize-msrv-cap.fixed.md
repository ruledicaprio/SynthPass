- **`cargo check -p mrz --all-features` builds again on the declared MSRV (1.82).** The optional
  `zeroize` feature now caps `zeroize` to `>=1.8.1, <1.9` and `zeroize_derive` to `>=1.4, <1.5`:
  `zeroize` 1.9.0 and `zeroize_derive` 1.5.0 both moved to edition 2024 / rust-version 1.85, which
  cargo 1.82 cannot even parse. If you already pin `zeroize` yourself, make sure it resolves to
  1.8.x while depending on `mrz`'s `zeroize` feature on a pre-1.85 toolchain.
