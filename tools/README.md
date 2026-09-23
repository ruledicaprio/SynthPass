# Developer and corpus-maintenance tools

These Python scripts are developer and corpus-maintenance tools only. They are never on the extraction path: the root README's **No Python** claim describes the Rust extraction runtime, not acquisition, review, tests, or CI maintenance. Some tools invoke the Rust OCR executables to inspect candidate specimens; extraction never invokes these Python scripts.

The [specimen-acquisition loop](../knowledge/SPECIMEN_SOURCES.md) defines the review gates and tooling boundary. Its [admitting a batch procedure](../knowledge/SPECIMEN_SOURCES.md#admitting-a-batch) explains the handoff from [apply_cohort.py](apply_cohort.py) (reviewed packet to cohort PR) to [rebless.py](rebless.py) (cohort PR to baseline and review readiness). The [real-specimen gate procedure](../knowledge/benchmarks/README.md#the-per-pr-real-specimen-regression-gate) documents re-blessing; consult the scripts' module docstrings for options and approval gates (`--help` is available on the maintenance command-line entrypoints other than the Commons downloader).

| Script | What it does, when to run it, and caller |
| --- | --- |
| [fetch_commons_mrz_specimens_v2.py](fetch_commons_mrz_specimens_v2.py) | Downloads Wikimedia Commons specimen candidates during corpus acquisition; run manually in the acquisition loop, then use [ingest-fetched-samples.ps1](../scripts/ingest-fetched-samples.ps1) for ingestion. |
| [scout_cycle.py](scout_cycle.py) | Plans and drives scouting cycles, screens candidates, and builds review packets; maintainer entrypoint for the acquisition loop's P-SCOUT, P-SCREEN, and P-PACKET procedures. |
| [screen_candidates.py](screen_candidates.py) | Fetches, hashes, deduplicates, and OCR-checks candidates before human review; called by `scout_cycle.py screen` or directly for the automated review gate. |
| [build_review_artifact.py](build_review_artifact.py) | Renders packet JSON as portable HTML for human verdicts after screening and the judgement pass; called by `scout_cycle.py review` or directly for packet review. |
| [apply_verdicts.py](apply_verdicts.py) | Copies saved HTML-review verdicts into the Markdown packet before admission; run manually for the packet-verdict handoff to P-APPLY-VERDICTS. |
| [apply_cohort.py](apply_cohort.py) | Prepares the cohort worktree, images, manifest, checks, and guarded push/PR steps once every packet row has a verdict; maintainer entrypoint for P-DATA-PR / admitting a batch. |
| [rebless.py](rebless.py) | Dispatches the real-specimen gate, classifies baseline changes, and prepares the cohort PR for review after images are pushed; maintainer entrypoint for re-blessing, driving [real-specimen-gate.yml](../.github/workflows/real-specimen-gate.yml). |
| [audit_benchmark_identity.py](audit_benchmark_identity.py) | Audits committed corpus identity and pinned image revisions without OCR before measurement; called by [real-specimen-gate.yml](../.github/workflows/real-specimen-gate.yml), [web-ocr.yml](../.github/workflows/web-ocr.yml), and [bench-charts.yml](../.github/workflows/bench-charts.yml), or manually for diagnosis. |
| [ocrb_metrics.py](ocrb_metrics.py) | Measures the vendored OCR-B font's per-glyph geometry over the MRZ alphabet (ink box, holes, ink area, straight-outline share, scanline ink runs) and ranks what separates each `mrz::CONFUSABLES` pair and the filler pairs, next to chargrid's `DEFAULT_INK_FLOOR`; run manually when working on glyph-level repair. It describes the synthetic corpus's font, not real print: the source for [mrz-geometric-elimination.md](../knowledge/research/mrz-geometric-elimination.md). |
| [test_apply_cohort.py](test_apply_cohort.py) | Tests cohort admission helpers with fixtures and fakes when changing admission tooling; run through the local unittest procedure below. |
| [test_audit_benchmark_identity.py](test_audit_benchmark_identity.py) | Tests revision selection, asset reconciliation, and parity with the Rust image walk when changing the auditor; run through the local unittest procedure below (requires `rustc`). |
| [test_build_review_artifact.py](test_build_review_artifact.py) | Tests HTML review generation and verdict application when changing either review script; run through the local unittest procedure below. |
| [test_ocrb_metrics.py](test_ocrb_metrics.py) | Tests the font decoder against the font's own stored bounding boxes and the geometry against the shoelace area, and pins the font findings, when changing the metrics tool; run through the local unittest procedure below (compares with `fontTools` only when it is installed). |
| [test_rebless.py](test_rebless.py) | Tests baseline-diff classification, document updates, and re-bless orchestration with fakes when changing re-blessing; run through the local unittest procedure below. |
| [test_scout_cycle.py](test_scout_cycle.py) | Tests cycle planning, packets, and state helpers with fixtures when changing the scouting driver; run through the local unittest procedure below. |
| [test_screen_candidates.py](test_screen_candidates.py) | Tests candidate screening with offline fixtures and fake network responses when changing the screener; run through the local unittest procedure below. |

Run the maintenance tests from the repository root:

```sh
python -m unittest discover -s tools -p 'test_*.py'
```

CI runs the same command on every pull request (the offline tools job in [ci.yml](../.github/workflows/ci.yml), which installs no Python packages). The tests do not run a corpus measurement. The Commons downloader requires `requests`; the screener's optional PDF lane requires PyMuPDF. Other maintenance scripts use the Python standard library, with external commands required by their individual procedures.

[scout_prefix.txt](scout_prefix.txt), [host_denylist.json](host_denylist.json), and [legal_portals.json](legal_portals.json) are supporting scouting and screening inputs, not executable scripts.
