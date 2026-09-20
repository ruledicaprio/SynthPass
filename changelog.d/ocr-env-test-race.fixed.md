- **The `synthpass-ocr` env-var tests no longer race each other.** They mutate process-global
  `std::env`, and `cargo test` runs a binary's tests on parallel threads, so one test writing
  `SYNTHPASS_OCR_ORDER=band-first` could read back a value a concurrently-running test had just
  cleared. Measured before the fix: **2 failures in 12 consecutive runs**, on two *different*
  tests — so this was the whole env-test set racing, not one bad case. After: 0 in 12.

  The previous arrangement was a convention, stated in a comment beside the tests: give each
  variable exactly one test. **It could not hold.** `ocr_arms_from_env_is_default_when_every_knob_is_unset`
  and its sibling have to clear *every* knob to assert their default, so they necessarily collide
  with each per-knob test. A rule the tests cannot obey is replaced by a lock that enforces itself
  — `crate::env_lock()`, taken by all twelve env-mutating tests across `lib.rs` and `verify.rs`.

  Mutex poisoning is recovered deliberately: without that, one panicking env test would leave every
  later one failing on the poisoned lock instead of its own assertion, turning a single real failure
  into a dozen misleading ones and hiding which test actually broke.

  Tests only — no production code, and no behaviour changes.
