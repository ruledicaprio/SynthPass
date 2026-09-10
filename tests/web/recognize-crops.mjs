// Cell (c) recognizer run (ADR-0008 chunk 1C).
//
// Runs the vendored tesseract.js OCR-B model over the preprocessed crop PNGs
// `crates/synthpass-ocr` dumped under SYNTHPASS_OCR_DUMP_VARIANTS, in real
// headless Chromium, and writes one JSONL row per crop. It answers: can a
// recognizer *other than ocrs/rten* read an MRZ off native's own finished
// crops? If yes, native's crop is fine and the recognizer is the gap; if it
// also produces noise, the whole gap is native detection/preprocessing.
//
//   # 1. dump the crops (from the repo root)
//   $env:SYNTHPASS_OCR_DUMP_VARIANTS = "artifacts/cell-c/crops"
//   cargo run -p synthpass-ocr --release --example dump_variants -- <image>...
//
//   # 2. recognize them
//   node tests/web/recognize-crops.mjs \
//     --crops artifacts/cell-c/crops --out artifacts/cell-c/tesseract-on-native-crops.jsonl
//
// Nothing leaves the machine: the crops are served from localhost and the
// browser runs offline.

import { chromium } from 'playwright';
import { readdir, writeFile, mkdir, access } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve, basename } from 'node:path';
import { startServer } from './static-server.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, '..', '..');

function parseArgs(argv) {
  const args = {
    crops: null,
    out: null,
    site: join(REPO, 'web'), // vendor/ + tessdata/ live here; no _site build needed
    headed: false,
    timeout: 120_000,
    // --best: OCR each crop at all four orientations and keep the best, the
    // way web/scan.js's orientation-retry passes do. The confirmation arm —
    // shows whether native's crop pixels are usable once the rotation native
    // committed to is undone. Also restricts to the `__general` crop per
    // document (one image, four OCR passes) to stay fast.
    best: false,
    only: null, // substring filter on crop filename
  };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--headed') args.headed = true;
    else if (a === '--best') args.best = true;
    else if (a === '--only') args.only = argv[++i];
    else if (a === '--crops') args.crops = resolve(argv[++i]);
    else if (a === '--out') args.out = resolve(argv[++i]);
    else if (a === '--site') args.site = resolve(argv[++i]);
    else if (a === '--timeout') args.timeout = Number(argv[++i]);
    else {
      console.error(`unknown argument: ${a}`);
      process.exit(2);
    }
  }
  if (!args.crops) {
    console.error('--crops <dir> is required (the SYNTHPASS_OCR_DUMP_VARIANTS output)');
    process.exit(2);
  }
  return args;
}

async function exists(p) {
  try { await access(p); return true; } catch { return false; }
}

// A crop file is `<document-stem>__<label>.png`. Recover the two parts so the
// JSONL can be grouped by document.
function splitCropName(file) {
  const stem = basename(file, '.png');
  const sep = stem.lastIndexOf('__');
  return sep === -1
    ? { document: stem, pass: '' }
    : { document: stem.slice(0, sep), pass: stem.slice(sep + 2) };
}

// MRZ-shaped lines only: uppercase A-Z / 0-9 / < , at least 20 chars. Lets the
// writeup see at a glance whether tesseract produced anything MRZ-like without
// re-implementing the parser here (the `mrz` crate scores it downstream).
function mrzShapedLines(text) {
  return text
    .split(/\r?\n/)
    .map((l) => l.replace(/\s+/g, '').toUpperCase())
    .filter((l) => l.length >= 20 && /^[A-Z0-9<]+$/.test(l));
}

async function main() {
  const args = parseArgs(process.argv.slice(2));

  if (!(await exists(join(args.site, 'vendor', 'tesseract.min.js')))) {
    console.error(`no tesseract runtime under ${args.site}/vendor — is --site pointing at web/ ?`);
    process.exit(2);
  }
  if (!(await exists(args.crops))) {
    console.error(`crops directory not found: ${args.crops}`);
    process.exit(2);
  }

  const IMG = /\.(png|jpe?g|webp|bmp)$/i;
  let files = (await readdir(args.crops)).filter((f) => IMG.test(f)).sort();
  if (args.only) files = files.filter((f) => f.includes(args.only));
  if (files.length === 0) {
    console.error(`no images in ${args.crops}${args.only ? ` matching "${args.only}"` : ''} — run the dump_variants example first`);
    process.exit(2);
  }

  const { server, port } = await startServer({
    siteDir: args.site,
    corpusDir: args.crops,
    harnessFile: join(HERE, 'recognize-crops.html'),
  });

  const browser = await chromium.launch({ headless: !args.headed });
  const page = await browser.newPage();
  page.on('pageerror', (e) => console.error(`  ! page error: ${e.message}`));
  await page.goto(`http://127.0.0.1:${port}/harness.html`);
  await page.waitForFunction(() => window.__ready === true, null, { timeout: 60_000 });

  const rows = [];
  let n = 0;
  for (const file of files) {
    n++;
    const { document, pass } = splitCropName(file);
    const url = `/corpus/${encodeURIComponent(file)}`;
    let rec;
    try {
      const fn = args.best
        ? (u) => window.__recognizeBest(u)
        : (u) => window.__recognize(u);
      rec = await Promise.race([
        page.evaluate(fn, url),
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error(`timed out after ${args.timeout} ms`)), args.timeout)),
      ]);
    } catch (e) {
      rec = { text: '', confidence: null, error: `harness: ${e.message}` };
    }
    const text = (rec.text ?? '').trim();
    const mrzLines = mrzShapedLines(text);
    rows.push({
      document,
      pass,
      crop: file,
      chars: text.length,
      confidence: rec.confidence,
      // Only in --best mode: the clockwise rotation (0/90/180/270) that
      // produced this text. A non-zero value that undoes native's own
      // auto-rotation is the finding.
      best_rotation_deg: args.best ? (rec.deg ?? null) : undefined,
      mrz_shaped_lines: mrzLines,
      text,
      error: rec.error ?? null,
    });
    process.stdout.write(
      `[${String(n).padStart(3)}/${files.length}] ${document} ${pass}` +
      (args.best ? ` @${rec.deg ?? '?'}°` : '') + `: ` +
      `${text.length} chars, ${mrzLines.length} MRZ-shaped line(s)\n`,
    );
  }

  await browser.close();
  server.close();

  // Per-document best pass: the crop that produced the most MRZ-shaped text.
  const byDoc = {};
  for (const r of rows) {
    const score = r.mrz_shaped_lines.join('').length;
    if (!byDoc[r.document] || score > byDoc[r.document].score) {
      byDoc[r.document] = { score, pass: r.pass, lines: r.mrz_shaped_lines, chars: r.chars };
    }
  }

  console.log('\n' + '='.repeat(72));
  console.log('tesseract OCR-B over native\'s crops — best pass per document:');
  for (const [doc, best] of Object.entries(byDoc).sort()) {
    const verdict = best.lines.length >= 2 ? 'MRZ-SHAPED' : best.lines.length === 1 ? 'partial' : 'noise';
    console.log(`  ${verdict.padEnd(11)} ${doc}  (pass ${best.pass || 'n/a'}, ${best.chars} chars)`);
    for (const l of best.lines) console.log(`      ${l}`);
  }
  console.log('='.repeat(72));

  if (args.out) {
    await mkdir(dirname(args.out), { recursive: true });
    await writeFile(args.out, rows.map((r) => JSON.stringify(r)).join('\n') + '\n');
    console.log(`\n${rows.length} rows written to ${args.out}`);
  }
}

await main();
