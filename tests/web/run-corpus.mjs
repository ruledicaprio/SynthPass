// Measure the browser MRZ validator over the specimen corpus.
//
// Runs the real assembled site in real headless Chromium — same scan.js, same
// mrz_wasm.js, same vendored tesseract.js build that deploys — over every
// corpus specimen the manifest marks as carrying an MRZ, and reports the
// checksum-valid hit rate against the native pipeline's own recorded result
// for the same documents.
//
// Why this exists: before it, no measurement of the web OCR path existed at
// all — no corpus run, no CI, nothing. That is the state crates/synthpass-llm/
// tests/parity.rs was in before it was rebuilt, and it is why preprocessing
// changes to the demo could only ever be justified by "seems better".
//
//   node tests/web/run-corpus.mjs --limit 10        # quick smoke
//   node tests/web/run-corpus.mjs --out report.json # full sweep
//
// Nothing leaves the machine: the corpus is served from localhost and the
// browser runs offline, preserving the deployed page's own guarantee.

import { chromium } from 'playwright';
import { readFile, writeFile, access } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve } from 'node:path';
import { startServer } from './static-server.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, '..', '..');

// The fields ICAO 9303 check digits prove. A checksum-valid read is
// self-proving on exactly these, which is why they are the only fields
// scored against a *generated* fixture — see the reviewed/derived split in
// crates/synthpass-llm/tests/parity.rs for why scoring a model (or here, an
// OCR engine) against unproven generated names inverts the measurement.
const PROVEN_FIELDS = ['document_number', 'date_of_birth', 'date_of_expiry'];
const ALL_FIELDS = [
  'document_type', 'issuing_country', 'document_number', 'surname',
  'given_names', 'nationality', 'date_of_birth', 'sex', 'date_of_expiry',
];

function parseArgs(argv) {
  const args = {
    site: join(REPO, '_site'),
    samples: join(REPO, 'samples'),
    out: null,
    limit: Infinity,
    floor: null,
    rotate: 0,
    headed: false,
    timeout: 120_000,
  };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--headed') args.headed = true;
    else if (a === '--site') args.site = resolve(argv[++i]);
    else if (a === '--samples') args.samples = resolve(argv[++i]);
    else if (a === '--out') args.out = resolve(argv[++i]);
    else if (a === '--limit') args.limit = Number(argv[++i]);
    else if (a === '--floor') args.floor = Number(argv[++i]);
    else if (a === '--rotate') args.rotate = Number(argv[++i]);
    else if (a === '--timeout') args.timeout = Number(argv[++i]);
    else {
      console.error(`unknown argument: ${a}`);
      process.exit(2);
    }
  }
  return args;
}

async function exists(p) {
  try { await access(p); return true; } catch { return false; }
}

/** Ground truth for one specimen, if any, plus how far it can be trusted. */
async function loadFixture(samples, stem) {
  const reviewed = join(samples, 'ocr_fixtures', `${stem}.json`);
  if (await exists(reviewed)) {
    return { reviewed: true, data: JSON.parse(await readFile(reviewed, 'utf8')) };
  }
  const derived = join(samples, 'ocr_fixtures', 'derived', `${stem}.json`);
  if (await exists(derived)) {
    return { reviewed: false, data: JSON.parse(await readFile(derived, 'utf8')) };
  }
  return null;
}

function pct(n, d) {
  return d === 0 ? null : n / d;
}

function fmtRate(n, d, label) {
  // An empty denominator is "not measured", never 0%. Reporting 0/0 as 0.0%
  // reads as a total failure when the truth is an absent corpus — a bug
  // parity.rs actually shipped once.
  if (d === 0) return `${label}: not measured (no candidates)`;
  return `${label}: ${n}/${d} (${(100 * n / d).toFixed(1)}%)`;
}

function quantile(sorted, q) {
  if (!sorted.length) return null;
  const i = Math.min(sorted.length - 1, Math.floor(q * (sorted.length - 1)));
  return sorted[i];
}

async function main() {
  const args = parseArgs(process.argv.slice(2));

  if (!(await exists(join(args.site, 'index.html')))) {
    console.error(
      `no assembled site at ${args.site}\n` +
      `build it first:  bash scripts/build-site.sh`,
    );
    process.exit(2);
  }

  const manifestPath = join(args.samples, 'corpus.jsonl');
  if (!(await exists(manifestPath))) {
    console.error(
      `no corpus manifest at ${manifestPath}\n` +
      `the specimen images live on the orphan samples-data branch — see samples/README.md`,
    );
    process.exit(2);
  }

  const rows = (await readFile(manifestPath, 'utf8'))
    .split('\n').filter(Boolean).map((l) => JSON.parse(l));
  const candidates = rows.filter((r) => r.mrz?.present).slice(0, args.limit);

  if (candidates.length === 0) {
    console.log('corpus has no MRZ-bearing specimens — not measured.');
    process.exit(0);
  }

  const { server, port } = await startServer({
    siteDir: args.site,
    corpusDir: args.samples,
    harnessFile: join(HERE, 'harness.html'),
  });

  const browser = await chromium.launch({ headless: !args.headed });
  const page = await browser.newPage();
  page.on('pageerror', (e) => console.error(`  ! page error: ${e.message}`));

  await page.goto(`http://127.0.0.1:${port}/harness.html`);
  await page.waitForFunction(() => window.__ready === true, null, { timeout: 60_000 });

  const documents = [];
  let n = 0;
  for (const row of candidates) {
    n++;
    const url = `/corpus/${row.dir}/${encodeURIComponent(row.filename)}`;
    const stem = row.filename.replace(/\.[^.]+$/, '');

    // page.evaluate takes no timeout option, so bound it here — one wedged
    // document must not stall a 190-image sweep.
    let scan;
    try {
      scan = await Promise.race([
        page.evaluate(([u, r]) => window.__scan(u, r), [url, args.rotate]),
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error(`timed out after ${args.timeout} ms`)), args.timeout),
        ),
      ]);
    } catch (e) {
      scan = { valid: false, parsed: null, raw: null, error: `harness: ${e.message}`, ms: null, passes: null, winning_pass: null };
    }

    const fixture = await loadFixture(args.samples, stem);
    const fields = {};
    // Only score fields on a checksum-VALID read. On a failed read the field
    // values are garbage by construction, so folding them into an accuracy
    // rate makes the metric move for reasons that have nothing to do with the
    // change being measured. Note what this leaves: on a valid read the three
    // ICAO-checksummed fields are self-proving, so the informative signal is
    // the *unproven* fields of a reviewed fixture — surname, given names,
    // nationality, sex, and the two line-1 fields no check digit covers.
    if (scan.parsed && scan.valid && fixture) {
      const scored = fixture.reviewed ? ALL_FIELDS : PROVEN_FIELDS;
      for (const f of scored) {
        const expected = fixture.data[f];
        if (expected === null || expected === undefined) continue;
        fields[f] = String(scan.parsed[f] ?? '') === String(expected);
      }
    }

    const nativeValid = !!row.mrz?.observed?.checksums_valid;
    documents.push({
      filename: row.filename,
      dir: row.dir,
      web_checksum_valid: scan.valid,
      native_checksum_valid: nativeValid,
      // Strictest available check: the exact two/three MRZ lines, not just
      // the fields parsed out of them.
      mrz_line_matches_fixture:
        fixture && scan.parsed && scan.valid
          ? String(scan.parsed.mrz_lines ?? '') === String(fixture.data.mrz_line ?? '')
          : null,
      // A parse that found MRZ-shaped text but failed its check digits — a
      // near miss, worth separating from "found nothing at all".
      near_miss: !scan.valid && !!scan.parsed,
      fixture: fixture ? (fixture.reviewed ? 'reviewed' : 'derived') : null,
      fields,
      winning_pass: scan.winning_pass,
      passes: scan.passes,
      ms: scan.ms === null ? null : Math.round(scan.ms),
      error: scan.error ?? null,
    });

    const mark = scan.valid ? 'HIT ' : 'miss';
    const vs = nativeValid === scan.valid ? '   ' : (nativeValid ? '<-N' : 'W->');
    process.stdout.write(
      `[${String(n).padStart(3)}/${candidates.length}] ${mark} ${vs} ${row.filename}\n`,
    );
  }

  await browser.close();
  server.close();

  // ---- aggregate -----------------------------------------------------------
  const webHits = documents.filter((d) => d.web_checksum_valid).length;
  const natHits = documents.filter((d) => d.native_checksum_valid).length;
  const both = documents.filter((d) => d.web_checksum_valid && d.native_checksum_valid).length;
  const webOnly = documents.filter((d) => d.web_checksum_valid && !d.native_checksum_valid).length;
  const natOnly = documents.filter((d) => !d.web_checksum_valid && d.native_checksum_valid).length;

  const fieldTally = { reviewed: { ok: 0, total: 0 }, derived: { ok: 0, total: 0 } };
  for (const d of documents) {
    const bucket = d.fixture === 'reviewed' ? fieldTally.reviewed
      : d.fixture === 'derived' ? fieldTally.derived : null;
    if (!bucket) continue;
    for (const ok of Object.values(d.fields)) {
      bucket.total++;
      if (ok) bucket.ok++;
    }
  }

  const byPass = {};
  for (const d of documents) {
    if (!d.winning_pass) continue;
    byPass[d.winning_pass] = (byPass[d.winning_pass] ?? 0) + 1;
  }

  const times = documents.map((d) => d.ms).filter((m) => m !== null).sort((a, b) => a - b);

  const report = {
    generated: new Date().toISOString(),
    // Non-zero means every image was turned by this many degrees before
    // scanning — an orientation test, not a like-for-like corpus run.
    rotate: args.rotate,
    corpus: {
      manifest_rows: rows.length,
      mrz_bearing: rows.filter((r) => r.mrz?.present).length,
      scanned: documents.length,
    },
    web: {
      checksum_valid: webHits,
      near_miss: documents.filter((d) => d.near_miss).length,
      no_mrz_found: documents.filter((d) => !d.web_checksum_valid && !d.near_miss).length,
      mrz_line_exact: documents.filter((d) => d.mrz_line_matches_fixture === true).length,
      scanned: documents.length,
      rate: pct(webHits, documents.length),
    },
    native_reference: {
      checksum_valid: natHits,
      scanned: documents.length,
      rate: pct(natHits, documents.length),
      note: 'from samples/corpus.jsonl mrz.observed.checksums_valid — the native ocrs/rten pipeline on the same files, not measured in this run',
    },
    head_to_head: { both, web_only: webOnly, native_only: natOnly },
    fields: {
      reviewed: { ...fieldTally.reviewed, rate: pct(fieldTally.reviewed.ok, fieldTally.reviewed.total) },
      derived: { ...fieldTally.derived, rate: pct(fieldTally.derived.ok, fieldTally.derived.total) },
      note: 'reviewed fixtures score all nine fields; derived fixtures score only the ICAO-checksummed three',
    },
    by_winning_pass: byPass,
    timing_ms: {
      median: quantile(times, 0.5),
      p90: quantile(times, 0.9),
      total: times.reduce((a, b) => a + b, 0),
    },
    documents,
  };

  console.log('\n' + '='.repeat(66));
  console.log(fmtRate(webHits, documents.length, 'web  checksum-valid'));
  console.log(fmtRate(natHits, documents.length, 'native (recorded) '));
  console.log(`head-to-head: both ${both}, web only ${webOnly}, native only ${natOnly}`);
  console.log(
    `misses: ${report.web.near_miss} parsed but failed check digits, ` +
    `${report.web.no_mrz_found} found no MRZ at all`,
  );
  console.log(fmtRate(fieldTally.reviewed.ok, fieldTally.reviewed.total, 'fields (reviewed) '));
  console.log(fmtRate(fieldTally.derived.ok, fieldTally.derived.total, 'fields (derived)  '));
  if (times.length) {
    console.log(`per-document ms: median ${quantile(times, 0.5)}, p90 ${quantile(times, 0.9)}`);
  }
  console.log('='.repeat(66));

  if (args.out) {
    await writeFile(args.out, JSON.stringify(report, null, 2) + '\n');
    console.log(`report written to ${args.out}`);
  }

  if (args.floor !== null) {
    const rate = pct(webHits, documents.length);
    if (rate === null) {
      console.log('floor not checked: nothing was scanned.');
    } else if (rate < args.floor) {
      console.error(
        `\nweb OCR hit rate regressed: ${(100 * rate).toFixed(1)}% is below the ${(100 * args.floor).toFixed(1)}% floor.`,
      );
      process.exit(1);
    }
  }
}

await main();
