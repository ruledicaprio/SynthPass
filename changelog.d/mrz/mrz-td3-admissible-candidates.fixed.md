- **A damaged TD3 line 1's still-shifted reading no longer forces a needless refusal.** #468
  made the damaged/class-sweep repair try both the corrected, unshifted line 1 and the original
  left-shifted one, then refuse when they disagreed. But the left-shifted reading's document
  code and issuing country are never real — the same test #447 already applies elsewhere — so
  when it is the only one that fails, it is now dropped before the unanimity check instead of
  causing a refusal. A genuine disagreement (for example two readings that both resolve to real,
  but different, countries or names) still refuses exactly as before.
