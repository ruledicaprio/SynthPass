// Shared document-scanning module for the browser demos (index.html,
// checkin.html).
//
// This used to be a hand-written JavaScript port of the native pipeline's
// preprocessing — the same band crop, percentile contrast stretch and Otsu
// threshold, written a second time and drifting from the Rust constant by
// constant. The port is gone. `crates/synthpass-imageprep` compiles to WASM,
// so the browser now runs the *identical* preprocessing the native pipeline
// runs, and there is one implementation to fix when it changes.
//
// What stays here is the part that is genuinely browser-specific: getting
// pixels out of a file, blitting a variant to a canvas for tesseract, and the
// retry ordering. The ICAO check digits are the oracle throughout — a retry
// can add a valid read but never break one.
//
// Everything runs in-tab: canvases are in-memory, tesseract.js workers are
// local, and the WASM module is the same `mrz` crate the native pipeline uses.

/** Downscale an image to maxDim on the largest side — all in-memory. */
async function toCanvas(bitmap, maxDim = 1600) {
  const scale = Math.min(1, maxDim / Math.max(bitmap.width, bitmap.height));
  const c = document.createElement('canvas');
  c.width = Math.round(bitmap.width * scale);
  c.height = Math.round(bitmap.height * scale);
  c.getContext('2d').drawImage(bitmap, 0, 0, c.width, c.height);
  return c;
}

/** A canvas's pixels in the RGBA layout the WASM preprocessing takes. */
function rgbaOf(canvas) {
  return canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data;
}

/** A preprocessed variant (from WASM) back onto a canvas for tesseract. */
function variantToCanvas(variant) {
  const c = document.createElement('canvas');
  c.width = variant.width;
  c.height = variant.height;
  c.getContext('2d').putImageData(
    new ImageData(new Uint8ClampedArray(variant.rgba), variant.width, variant.height),
    0,
    0,
  );
  return c;
}

// Two OCR models: 'mrz' is fine-tuned on the OCR-B font MRZs are printed in
// (vendored, BSD-3-Clause © DoubangoTelecom — see tessdata/LICENSE); 'eng'
// is the generic fallback. Both are restricted to the MRZ charset, which now
// comes from the Rust engine's own constant rather than a copy of it. Every
// runtime asset is same-origin: fetched + SHA-256-verified at deploy time by
// fetch-vendor.sh, so the page makes zero CDN requests.
const workers = {};
function getWorker(lang, charset) {
  workers[lang] ??= (async () => {
    const worker = await Tesseract.createWorker(lang, 1, {
      workerPath: './vendor/worker.min.js',
      corePath: './vendor',
      langPath: './tessdata',
      gzip: lang !== 'mrz', // mrz.traineddata is committed uncompressed
    });
    await worker.setParameters({ tessedit_char_whitelist: charset });
    return worker;
  })();
  return workers[lang];
}

/**
 * Scan `file` for an MRZ.
 *
 * `parse(text)` must return a parse result object with a `.valid` boolean, or
 * null (the caller wraps the WASM parser). `prep` is the WASM preprocessing
 * namespace — `{ plain_band, mrz_variants, mrz_charset }`. `setStatus(msg)`
 * receives progress strings.
 *
 * Resolves to `{ result, raw, valid }` — the first checksum-valid parse, or
 * the best parseable-but-invalid one, or `{ result: null }` if nothing
 * MRZ-shaped was found.
 */
export async function scanDocument(file, parse, prep, setStatus) {
  setStatus('Loading image (downscaling locally)…');
  const bitmap = await createImageBitmap(file);
  const full = await toCanvas(bitmap);
  bitmap.close?.();

  const charset = prep.mrz_charset();
  const rgba = rgbaOf(full);
  const { width, height } = full;

  // Ordering is a property of the recognizer, not of the platform.
  //
  // The untreated band crop goes FIRST because this recognizer is already
  // trained on OCR-B: measured over the specimen corpus it produced 112 of
  // 122 checksum-valid reads on its own, with every treated variant together
  // recovering ten more (knowledge/WEB_OCR_BASELINE.md). Native orders these
  // differently because `ocrs` normalizes internally and gains nothing from
  // an untreated pass.
  //
  // Then the engine's own ordered retry variants — contrast stretch,
  // binarize, full page, and the row-density-isolated band passes that only
  // materialize on a scan dense enough for the blind crop to have dragged in
  // neighbouring text. Then the generic model, last, on the band and the
  // full page.
  //
  // Variants are built lazily: each is megabytes of pixels, and the common
  // case stops at the first attempt.
  const attempts = [
    ['mrz', 'Reading MRZ band (OCR-B model)…', () => prep.plain_band(rgba, width, height)],
  ];

  const variants = prep.mrz_variants(rgba, width, height);
  const VARIANT_LABELS = [
    'Retrying with contrast stretch…',
    'Retrying binarized…',
    'Scanning full image…',
    'Retrying with an isolated MRZ band…',
    'Retrying the isolated band, locally thresholded…',
    'Retrying the isolated band, deskewed…',
  ];
  for (let i = 0; i < variants.len; i++) {
    attempts.push([
      'mrz',
      VARIANT_LABELS[i] ?? `Retrying (pass ${i + 2})…`,
      () => variants.get(i),
    ]);
  }

  attempts.push(
    ['eng', 'Retrying with the general model…', () => prep.plain_band(rgba, width, height)],
    ['eng', 'Scanning full image (general model)…', () => null],
  );

  let best = null;
  let bestRaw = null;
  for (const [lang, msg, makeVariant] of attempts) {
    setStatus(msg);
    let worker;
    try {
      worker = await getWorker(lang, charset);
    } catch {
      continue; // model failed to load — try the next attempt
    }
    const variant = makeVariant();
    // `null` means "the original downscaled page, untouched"; a null variant
    // from WASM means the buffer was rejected, which is not recoverable by
    // retrying the same buffer.
    const canvas = variant === null ? full : variant ? variantToCanvas(variant) : null;
    if (!canvas) continue;

    const { data } = await worker.recognize(canvas);
    const parsed = parse(data.text);
    if (parsed) {
      if (parsed.valid) {
        setStatus('');
        return { result: parsed, raw: data.text, valid: true };
      }
      if (!best) { best = parsed; bestRaw = data.text; }
    }
  }
  return { result: best, raw: bestRaw, valid: false };
}
