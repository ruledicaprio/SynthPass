# Web OCR harness

Measures the browser MRZ validator — <https://ruledicaprio.github.io/SynthPass/> —
over the specimen corpus, in real headless Chromium.

## Why it exists

Until this harness, **no measurement of the web OCR path existed at all**: no
corpus run, no CI, no baseline. Every change to `web/scan.js` could only be
justified by "seems better", which is precisely the standard
[`knowledge/prompts/README.md`](../../knowledge/prompts/README.md) rejects for
the Tier-2 path and `crates/synthpass-llm/tests/parity.rs` now enforces there.

The browser is a *separate OCR stack* from the native pipeline — tesseract.js
with an OCR-B-trained model, not `ocrs`/`rten` — so native benchmark numbers say
nothing about it. Only this does.

## What it runs

The real assembled site. `scripts/build-site.sh` produces `_site/` exactly as
`.github/workflows/pages.yml` deploys it, and the harness loads
`harness.html` **from inside that build**, importing the same `scan.js`, the
same `mrz_wasm.js` and the same SHA-256-pinned tesseract.js runtime the demo
imports. The harness page exists only to skip the DOM rendering: `index.html`'s
result panel deliberately clears itself after ten seconds (a privacy feature),
so scraping it would race the measurement against a timer.

`harness.html` lives here, outside `web/`, so `cp -r web/*` never carries it
into the deployed site. The static server aliases it in at test time.

## Running it

```bash
bash scripts/build-site.sh                  # needs wasm-pack + network (vendored runtime)
cd tests/web && npm ci && npx playwright install chromium

node run-corpus.mjs --limit 8               # quick smoke
node run-corpus.mjs --out report.json       # full sweep (~20-30 min)
node run-corpus.mjs --floor 0.30            # exit 1 if the hit rate regresses
```

The specimen images are not on `main` — they live on the orphan `samples-data`
branch. See [`samples/README.md`](../../samples/README.md).

## What it reports

**Primary metric: checksum-valid hit rate** over every manifest row with
`mrz.present == true`. The ICAO check digits are the oracle — a read either
verifies or it does not, with no judgement call in between.

Alongside it, the same documents' **native** result, read from
`samples/corpus.jsonl`'s `mrz.observed.checksums_valid`. That is not measured in
this run; it is what the `ocrs`/`rten` pipeline recorded for the same files, and
it makes the web number mean something. The head-to-head split (both / web only
/ native only) is where the interesting documents are.

Misses are split into **near misses** (MRZ-shaped text found, check digits
failed) and **no MRZ found** — different failures with different fixes.

Field accuracy is scored **only on checksum-valid reads**, using the same
reviewed-vs-derived fixture split as `parity.rs`: reviewed fixtures score all
nine fields, generated ones score only the three ICAO proves. Scoring a
generated fixture's *name* would invert the measurement — the engine reading the
printed name correctly gets marked wrong for disagreeing with an earlier OCR
error. On a valid read the checksummed fields are self-proving, so the
informative signal is a reviewed fixture's unproven fields.

An empty candidate set reports **"not measured"**, never 0%.

## Recorded results

[`knowledge/WEB_OCR_BASELINE.md`](../../knowledge/WEB_OCR_BASELINE.md).

## Privacy

Nothing leaves the machine. The corpus is served from `127.0.0.1`, the browser
runs against that origin only, and the page under test is the same zero-CDN
build that deploys.
