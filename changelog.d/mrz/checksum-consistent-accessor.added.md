- **`MrzData::checksum_consistent()`** — the named form of what `valid()` computes, and the
  preferred spelling from here on. Both return `checks.all_valid()`; neither is going away
  inside 0.8. The short name is the problem: `valid` sits three letters from `validity`,
  which answers an unrelated question (is the document in date?), so it reads as a verdict on
  the document when it is a verdict on the arithmetic. Its doc example now also shows what
  the verdict does *not* establish — TD3's composite spans line 2 only, so an altered surname
  on line 1 is exactly as checksum-consistent as the genuine one.
