// Zero-dependency static server for the web OCR harness.
//
// Serves three things, all same-origin so the page behaves exactly as the
// deployed site does (tesseract.js workers and the WASM module both require
// same-origin, correctly-typed responses):
//
//   /                 -> the assembled _site/ (what pages.yml deploys)
//   /harness.html     -> tests/web/harness.html, aliased in without ever
//                        being copied into the deployed site
//   /corpus/<file>    -> a specimen image from the local samples corpus
//
// Serving the corpus over HTTP rather than pushing bytes through the CDP
// bridge keeps a 232-image sweep from spending most of its time on
// serialization.

import { createServer } from 'node:http';
import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { extname, join, normalize, resolve, sep } from 'node:path';

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.css': 'text/css; charset=utf-8',
  '.traineddata': 'application/octet-stream',
  '.gz': 'application/gzip',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.png': 'image/png',
  '.webp': 'image/webp',
  '.bmp': 'image/bmp',
};

/** Resolve `rel` under `root`, refusing anything that escapes it. */
function safeJoin(root, rel) {
  const full = resolve(join(root, normalize(rel).replace(/^([/\\])+/, '')));
  const base = resolve(root);
  return full === base || full.startsWith(base + sep) ? full : null;
}

export function startServer({ siteDir, corpusDir, harnessFile, port = 0 }) {
  const server = createServer(async (req, res) => {
    try {
      const url = new URL(req.url, 'http://localhost');
      let file;

      if (url.pathname === '/harness.html') {
        file = resolve(harnessFile);
      } else if (url.pathname.startsWith('/corpus/')) {
        file = safeJoin(corpusDir, decodeURIComponent(url.pathname.slice('/corpus'.length)));
      } else {
        const rel = url.pathname === '/' ? '/index.html' : url.pathname;
        file = safeJoin(siteDir, decodeURIComponent(rel));
      }

      if (!file) {
        res.writeHead(403).end('forbidden');
        return;
      }
      const info = await stat(file);
      if (!info.isFile()) {
        res.writeHead(404).end('not found');
        return;
      }
      res.writeHead(200, {
        'content-type': TYPES[extname(file).toLowerCase()] ?? 'application/octet-stream',
        'content-length': info.size,
        'cache-control': 'no-store',
      });
      createReadStream(file).pipe(res);
    } catch {
      res.writeHead(404).end('not found');
    }
  });

  return new Promise((ok) => {
    server.listen(port, '127.0.0.1', () => {
      ok({ server, port: server.address().port });
    });
  });
}
