#!/usr/bin/env bash
# Assemble the GitHub Pages demo into _site/ — the single definition of how
# the site is built, used by BOTH .github/workflows/pages.yml (what ships) and
# tests/web (what gets measured). Keeping one definition is the point: a
# harness that assembled the site its own way would measure something other
# than what deploys.
#
# Usage: bash scripts/build-site.sh [outdir]   (default: _site)
set -euo pipefail
cd "$(dirname "$0")/.."

OUT=${1:-_site}

echo "==> building WASM package"
wasm-pack build crates/mrz-wasm --target web --release --no-typescript

# The demo's OCR runtime (tesseract.js + cores + eng traineddata) is fetched
# and SHA-256-verified here, never committed — the same pin-and-verify pattern
# the native pipeline uses for its .rten models. The assembled site then makes
# zero CDN requests.
echo "==> fetching + verifying vendored OCR runtime"
bash web/fetch-vendor.sh

echo "==> assembling $OUT"
rm -rf "$OUT"
mkdir -p "$OUT/pkg"
cp -r web/* "$OUT/"
cp crates/mrz-wasm/pkg/mrz_wasm.js crates/mrz-wasm/pkg/mrz_wasm_bg.wasm "$OUT/pkg/"

echo "site ready in $OUT"
