// Verify that a file the browser cannot decode produces a message naming the
// real format, end to end through the shipped scan path.
//
// `createImageBitmap` rejects HEIC in most engines with a bare "The source
// image could not be decoded", which reads as a corrupt file rather than an
// unsupported one — and HEIC is the iPhone default. These assertions pin the
// better messages, and pin that unrecognised bytes get NO invented diagnosis.
//
//   node tests/web/check-format-errors.mjs
import { chromium } from 'playwright';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve } from 'node:path';
import { startServer } from './static-server.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, '..', '..');

const ftyp = (brand) => [0, 0, 0, 0x18, ...Buffer.from('ftyp' + brand)];

const CASES = [
  { name: 'HEIC (iPhone default)', bytes: ftyp('heic'), expect: /HEIC\/HEIF photo/ },
  { name: 'HEIF variant brand', bytes: ftyp('mif1'), expect: /HEIC\/HEIF photo/ },
  { name: 'PDF', bytes: [...Buffer.from('%PDF-1.7\n%\xe2\xe3\xcf\xd3')], expect: /This is a PDF/ },
  // An MP4 is ISO-BMFF too but is not HEIF: it must NOT be named as one.
  { name: 'MP4 (not HEIF)', bytes: ftyp('isom'), expect: /could not be decoded as an image/ },
  { name: 'garbage', bytes: [...Buffer.from('not any known header at all')], expect: /could not be decoded as an image/ },
];

const { server, port } = await startServer({
  siteDir: join(REPO, '_site'),
  corpusDir: join(REPO, 'samples'),
  harnessFile: join(HERE, 'harness.html'),
});
const browser = await chromium.launch();
const page = await browser.newPage();
await page.goto(`http://127.0.0.1:${port}/harness.html`);
await page.waitForFunction(() => window.__ready === true, null, { timeout: 60_000 });

let failed = 0;
for (const c of CASES) {
  const { error } = await page.evaluate((b) => window.__scanBytes(b), c.bytes);
  const ok = error !== null && c.expect.test(error);
  if (!ok) failed++;
  console.log(`${ok ? 'ok  ' : 'FAIL'}  ${c.name}\n      ${error ?? '(no error thrown)'}`);
}

// The negative cases above are only meaningful if a real image still decodes.
const { error: jpegError } = await page.evaluate(async () => {
  const c = document.createElement('canvas');
  c.width = 40; c.height = 20;
  const ctx = c.getContext('2d');
  ctx.fillStyle = '#fff'; ctx.fillRect(0, 0, 40, 20);
  const blob = await new Promise((ok) => c.toBlob(ok, 'image/jpeg'));
  return window.__scanBytes([...new Uint8Array(await blob.arrayBuffer())]);
});
const decodes = jpegError === null;
if (!decodes) failed++;
console.log(`${decodes ? 'ok  ' : 'FAIL'}  a real JPEG still decodes\n      ${jpegError ?? '(scanned, no MRZ — as expected)'}`);

await browser.close();
server.close();
console.log(failed === 0 ? '\nall format-error messages correct' : `\n${failed} case(s) failed`);
process.exit(failed === 0 ? 0 : 1);
