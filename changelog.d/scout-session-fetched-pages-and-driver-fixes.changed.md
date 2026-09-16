- **A page the tool cannot fetch can now be handed to it (`--page-html-dir`).** Official hosts
  that answer `tools/screen_candidates.py`'s plain `urllib` request with 403/503, an anti-bot
  interstitial or a JavaScript-only shell used to lose the candidate at the linked-from-page
  check. `tools/scout_cycle.py blocked --cycle cNN` now lists those URLs grouped by cause
  (`--to-fetch` prints just the page URLs); the maintainer's session fetches them by its own
  means -- never repository code -- and drops the HTML into `work/scouting/cNN/pages/` as a
  `pages.jsonl` index plus one file per page named by `page_html_filename(page_url)`. `screen`
  picks that directory up automatically, the screened record then reads
  `page_source: "session-fetched"` with its `page_fetched_at`, and the packet's Note column says
  so. The image itself is still fetched by the tool, so a candidate whose image is also blocked
  stays blocked.
- **`tools/scout_cycle.py` no longer loses a parallel cycle's work.** Worker worktrees are
  created and removed sequentially in the main thread (three parallel creations raced on
  `.git/config` and killed w1 on cycle c14, with one retry after 2 s if the lock is held anyway),
  and a worker that raises now becomes an empty result with a fault line instead of propagating:
  the cycle is recorded with whatever finished, the command exits non-zero and prints the
  `--slice` to rerun. A rerun merges -- candidates by `(image_url, page_url)`, blocked URLs by
  url, and the Cycles row's codes/scouted/tokens cells additively -- rather than overwriting the
  workers that did finish.
- **A cover-only cohort is remembered, so its codes are not re-scouted at full priority.**
  `tools/apply_cohort.py` now writes a `cover only, data page still wanted (cNN): <codes>` line
  into `work/scouting/STATE.md` whenever it places a row under `covers/` (a cover never moves
  `CORPUS_COVERAGE.md`'s Status -- ADR-0012 -- so nothing else recorded it, and `suggest`
  re-proposed all fourteen cycle-c13 codes for c14). `suggest` keeps such a code in the list and
  ranks it in its own tier: after every fresh code, before the low-web-presence micro-states.
- **`tools/apply_cohort.py --confirm` commits again, and `tools/rebless.py` stops flooding the
  log.** The commit stages only the tracked text a run wrote (`samples/corpus.jsonl`,
  `knowledge/CORPUS_COVERAGE.md`, the changelog fragment); the placed images and everything under
  `samples/local/` are not tracked on main -- they reach `samples-data` through
  `scripts/sync-samples.ps1` -- and naming one of them failed the whole staging step. The CI wait
  in `rebless.py` runs at `--interval 60` with its progress output discarded, printing one line
  before and one after instead of the 33,000 a single cohort's wait left behind.
- **Scout prefix v3.2** (`tools/scout_prefix.txt`), after cycles c13 and c14 returned 28 covers
  and no data page: the worker self-excludes an image only when the source itself says it is a
  real individual's document, reports an image that may carry a photograph with `"h5_check":true`
  instead of dropping it (the screener, who opens the image, makes the H5 call -- such rows are
  noted `h5 check` in the packet and get the field-by-field pass first), and treats a specimen's
  placeholder identity as part of the specimen rather than a person. A private upload of a real
  person's document still never enters, redacted or not. An intergovernmental organisation's own
  page about a harmonised document its member states issue (the EAC e-passport on `eac.int`, the
  ECOWAS passport on `ecowas.int`) is now an allowed host, destination public, `origin.licence`
  `gov-published`, attributed to the organisation. `knowledge/SPECIMEN_SOURCES.md` and the
  `synthpass-screener` agent carry the same rules.
- **The PDF lane stops rendering leaflet pages.** A page that draws no image and carries more
  than 120 words of its own text is skipped as `pdf-text-page` and counted in the run's summary,
  instead of reaching the packet as a full-page raster of prose (a Liberia PDF did exactly that
  on cycle c14). No OCR is involved: the page text is already in hand.
