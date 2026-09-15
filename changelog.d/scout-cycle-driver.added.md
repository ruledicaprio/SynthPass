- **`tools/scout_cycle.py` and the `synthpass-screener` subagent.** One command now runs the
  mechanical half of a specimen-scouting cycle end to end: `suggest` picks uncovered codes from
  `CORPUS_COVERAGE.md` minus what `STATE.md` already records, `task` writes the worker task from
  the byte-stable prefix (`tools/scout_prefix.txt`, v2) plus full country names and held series,
  `scout` runs one `dsh` worker per `--slice` in its own worktree in parallel and records wall
  time, tokens, faults and candidates, `screen` calls `screen_candidates.py` and writes the
  packet's JSON twin, `review` renders the HTML review page. The judgement pass between `screen`
  and `review` (host check, eyes on every image for a real person, the proposed destination and
  licence class) is now a project subagent, `synthpass-screener`, whose write scope a hook limits
  to `work/scouting/`; every verdict and every merge stays with the user, unchanged.
