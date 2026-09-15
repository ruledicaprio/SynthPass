# Specimen sources — where a corpus image may come from

This applies to every specimen added under `samples/`, whoever finds it: a person, the Commons
fetcher (`tools/fetch_commons_mrz_specimens_v2.py`), an agent, or a scraping service.
[`CORPUS_COVERAGE.md`](CORPUS_COVERAGE.md) tracks which codes are covered; this file says what
may be used to cover them. The mechanics of adding a file are in
[`CONTRIBUTING.md`](../CONTRIBUTING.md#adding-a-corpus-specimen).

## Allowed sources

| Source | Examples |
|---|---|
| The issuing authority's own publication | Foreign-ministry, interior-ministry and passport-office pages; embassy and consulate pages; the official gazette that publishes the regulation defining a document, whose annex is often the document's specimen |
| An intergovernmental organisation's own pages, for its own travel document | A laissez-passer specimen published by the organisation that issues it |
| ICAO's own Doc 9303 material | Where ICAO's terms allow the use |
| Wikimedia Commons | Each file states its licence on its own page |

## Never a source

- **PRADO** (`consilium.europa.eu/prado`). Its copyright notice prohibits harvesting or
  redistributing its material outside official, non-commercial use. It is consulted only as a
  manual human reference, never scraped and never stored, not even locally.
- **Commercial template and aggregator sites.** Their terms generally forbid scraping, and their
  images are not the issuer's.
- **Novelty and fake-ID vendors.** `VENDOR_BLOCKLIST` in `check_sample.rs` rejects known vendor
  signatures at ingest.
- **Forums, social media, leak and paste sites.** An identity document found there is as likely
  to belong to a real person as to be a specimen.

## The specimen signal

A candidate proposed by an agent or a scraping service must carry at least one of these:

- a specimen watermark, in any language;
- a placeholder holder name or number (`MUSTERMANN`, `000000000`);
- a hosting page that calls the image a specimen or sample — for example, a regulation annex
  titled as the document's specimen;
- publication by the issuing authority itself, on one of the allowed hosts, as an illustration
  of its own document — even when the page says nothing about specimens. Many issuers publish
  their specimens this way. An agent reports such a candidate as `official-host-only`; the
  automated screen and the reviewer then confirm the signal on the image itself, and the
  real-person check applies in full (a real holder's document on an official page is still a
  real holder's document).

Candidates without any of these are not proposed at all. A PDF linked from an allowed host (a
gazette annex, a regulation) may be proposed as a candidate even though the agent cannot open
it; the automated screen extracts its images and checks each one on the same terms. A page on
an allowed host that would not load is reported as blocked, so the maintainer can retry it
session-side (see "The tooling boundary"). A person adding a specimen by hand
follows [`CONTRIBUTING.md`](../CONTRIBUTING.md#adding-a-corpus-specimen), where the watermark is
an advisory signal rather than a requirement.

## Review gates for agent-found candidates

1. **Automated screen.** `tools/screen_candidates.py` performs this step (`tools/scout_cycle.py`
   drives it, together with the worker launch before it and the review page after it): the
   candidate is fetched again by the tool, not taken from the agent's copy, and then:
   - its host is checked against the denylist above;
   - its page is fetched and the image is confirmed to actually be linked from it;
   - its `sha256` is compared against the manifest, which rejects byte duplicates;
   - `check_sample` runs, including the vendor blocklist;
   - one OCR pass reads the MRZ;
   - a candidate that turns out to be a PDF (a gazette annex, a regulation) takes the tool's PDF
     lane: its pages are scanned for the same specimen words, the images on those pages are
     extracted at their original bytes (PyMuPDF, the one non-standard-library dependency in
     `tools/`, imported only there), and each extracted image is then screened exactly as a
     directly linked image would be, with `#page=N` on its URL so the ledger and the manifest
     `origin` name the page.
   Survivors are written to a Markdown packet for a human to fill in the provenance call; the
   tool itself never decides public/local/drop or a licence class.
2. **Maintainer review.** The maintainer — or the `synthpass-screener` subagent, which opens
   every staged image, checks the host and proposes a destination and licence class, and whose
   write scope is limited to `work/scouting/` — makes the provenance call and proposes a
   manifest row. A proposal is never a verdict.
3. **Human verification**, candidate by candidate: *public*, *local* or *drop*. Optionally done
   through `tools/build_review_artifact.py`, which renders the packet as a single self-contained
   HTML page (image plus every column, one click per verdict); `tools/apply_verdicts.py` then
   syncs the saved result back into the packet's own Verdict column, which stays the record of
   truth either way.

Nothing reaches `samples-data` before step 3. The agent or service never ingests, commits or
pushes. Everything it reports — a URL, a transcribed MRZ, a claim that it downloaded a file — is
a claim for the reviewer to check, not evidence. A proving run on 2026-09-13 downloaded a
different image from the one it described.

## Where a verified specimen goes

| Destination | When |
|---|---|
| `samples/` → `samples-data` (public) | The source allows redistribution |
| `samples/local/` (the local-only track) | The source does not allow redistribution, or states no terms and the verifier is not satisfied it does |

Either way, the image's source goes into the manifest's `origin` when it is fetched (see
[`samples/README.md`](../samples/README.md)), because afterwards it cannot be recovered. The
local-only track stays out of the default benchmark walk, and out of every CI run.

## What is kept when a candidate is dropped

| Case | Image | Holder values (names, numbers, dates) | Notes on the document's layout |
|---|---|---|---|
| Verified specimen | kept, in one of the two destinations above | kept | kept |
| Doubtful licence | deleted, or moved to the local-only track | kept only if it is a confirmed specimen | kept |
| Possibly a real person's document | deleted immediately | deleted and never written anywhere | structure only: field placement, label languages, MRZ format, document-number *shape* |
| Any other rejection (vendor, duplicate, unresolvable URL, off-scope) | deleted | none | none |

A possibly-real document never passes through `samples/private/`. That directory holds a user's
own material; it is not a quarantine for scraped images.

## Admitting a batch

Each admitted batch is its own data-only PR. It carries:

- the manifest rows, with `origin`;
- the coverage rows in [`CORPUS_COVERAGE.md`](CORPUS_COVERAGE.md);
- the real-specimen baseline, regenerated by CI.

No code change rides in the same PR. That way a moved number is attributable either to the
corpus or to the code, never to both at once.

`tools/apply_cohort.py` automates the mechanical half of admitting a verified packet: worktree
setup, a cross-PR duplicate guard (so a second open cohort branch cannot fold a first PR's
not-yet-merged images into its own manifest), image placement, manifest regeneration and
`origin` patching, and the `samples-data` push -- gated behind a `-DryRun` check that refuses to
proceed if it would delete anything. It never decides a licence class or writes unreviewed prose
into `origin.notes` or [`CORPUS_COVERAGE.md`](CORPUS_COVERAGE.md)'s Note column; those stay a
human's draft to accept.

## The tooling boundary

Acquisition may use network tools, including a hosted search or scraping service. The extraction
path may not, and that line does not move. No repository code calls a scraping service or a
search API. A tool's terms never widen what a source's terms allow.

In practice: when a scout worker reports an allowed-host page as blocked, the maintainer's own
session retries it with the scraping service and hands any candidate it finds back to the loop
through `tools/scout_cycle.py add-candidates`, in the worker's own record shape and tagged as
found by the session. Those rows then pass the automated screen like every other candidate; the
service is never called from the repository's tools.
