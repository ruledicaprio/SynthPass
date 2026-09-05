- **`vocab_replay` no longer mangles a value containing a quote.** It stripped the `Some("…")`
  wrapper `parity.rs` prints with `{:?}` but never reversed Debug's `\"` escaping, so a genuine
  hit on a document whose value contains a literal `"` replayed with a stray backslash still in
  it — misreporting a hit as a new miss, which trips the tool's own blocker and could stop (or
  mask) a safe vocabulary change from shipping. It also now refuses to parse a log with no
  `log-format:` marker in its header, or one from an incompatible version of `parity.rs`'s
  `field expected=… actual=… OK|MISMATCH` line shape, rather than silently mis-parsing it.
- **`scripts/measure-parity.sh --arm` no longer silently defaults to `both` on a mistake.**
  Running `--arm` with the value omitted, or any unrecognized first argument, used to fall
  through to the full ~1 hour two-arm measurement instead of erroring — the exact "silently
  measure the wrong thing" failure this script exists to prevent. Also: invokes `python3` (the
  project's Linux dev container ships no `python`), and the running-binary guard now filters
  `tasklist` to `parity-*` image names instead of grepping that substring over the whole process
  list, which could both false-positive on an unrelated process and miss a real conflict under a
  different process name.
