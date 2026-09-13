#!/usr/bin/env python3
"""
build_review_artifact.py

Turns one or more structured packet files (the JSON twin of a Markdown
`packet-cNN.md` produced by P-PACKET, plan §6.4) into a single, self-contained
HTML file for visual, one-click packet review: one card per candidate, its
staged image embedded inline (base64 -- no relative-path or file:// dependency,
so the page is fully portable), and Public/Local/Drop buttons.

This tool never decides public/local/drop, never decides a licence class, and
never writes a holder value anywhere new -- it only renders whatever the
packet JSON already carries (which itself never carries a holder value; see
`screen_candidates.py`'s header). The Markdown `packet-cNN.md` stays the
plan's official Verdict record; this is a nicer front door onto it. Saved
verdicts are synced back into that file by the sibling `apply_verdicts.py`,
not read directly by any plan procedure.

Standard library only.

Usage:
    python tools/build_review_artifact.py --packet work/scouting/c01/packet-c01.json \\
        --out work/scouting/c01/review-c01.html \\
        [--packet work/scouting/c02/packet-c02.json ...] \\
        [--prior-verdicts work/scouting/c01/verdicts-c01.json]

Multiple --packet files let a review session span more than one cycle, or let
a cycle's packet be regenerated (new rows appended, same file) without losing
already-set verdicts, since --prior-verdicts carries them forward. Two rows
with the same `id` across inputs: the later --packet argument wins, matching
the "regenerate to amend" workflow this tool is for.
"""

from __future__ import annotations

import argparse
import base64
import html
import json
import sys
from pathlib import Path

IMAGE_MIME = {
    ".jpg": "image/jpeg",
    ".jpeg": "image/jpeg",
    ".png": "image/png",
    ".webp": "image/webp",
    ".gif": "image/gif",
}

REQUIRED_FIELDS = (
    "id",
    "code",
    "doc_td",
    "series",
    "host",
    "licence_evidence",
    "specimen_signal",
    "mrz_check",
    "proposed_destination",
    "proposed_licence",
    "local_path",
    "note",
)


def load_rows(packet_paths: list[Path]) -> dict[str, dict]:
    """Reads every --packet file in order; a later file's row overrides an
    earlier one with the same `id`, so re-running against a regenerated
    packet amends in place rather than duplicating."""
    rows: dict[str, dict] = {}
    for path in packet_paths:
        data = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(data, list):
            raise ValueError(f"{path}: expected a JSON array of rows")
        for row in data:
            missing = [f for f in REQUIRED_FIELDS if f not in row]
            if missing:
                raise ValueError(f"{path}: row {row.get('id', '?')!r} missing {missing}")
            rows[row["id"]] = row
    return rows


def load_prior_verdicts(path: Path | None) -> dict[str, dict]:
    if path is None:
        return {}
    if not path.is_file():
        return {}
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"{path}: expected a JSON object mapping id -> verdict")
    return data


def embed_image(repo_root: Path, local_path: str) -> str:
    """Returns a `data:` URI for the image, or an inline SVG placeholder if
    the file can't be found (e.g. staging was already cleared) -- the review
    page should never fail to render just because one image is missing."""
    resolved = (repo_root / local_path).resolve()
    ext = resolved.suffix.lower()
    mime = IMAGE_MIME.get(ext)
    if mime is None or not resolved.is_file():
        return (
            "data:image/svg+xml,"
            "%3Csvg xmlns='http://www.w3.org/2000/svg' width='320' height='200'%3E"
            "%3Crect width='100%25' height='100%25' fill='%23eee'/%3E"
            "%3Ctext x='50%25' y='50%25' text-anchor='middle' fill='%23900'%3E"
            "image not found%3C/text%3E%3C/svg%3E"
        )
    encoded = base64.b64encode(resolved.read_bytes()).decode("ascii")
    return f"data:{mime};base64,{encoded}"


def esc(value: object) -> str:
    return html.escape(str(value), quote=True)


def render_row(row: dict, image_uri: str, prior_verdict: str | None) -> str:
    rid = esc(row["id"])
    prior = esc(prior_verdict) if prior_verdict else ""
    prior_note = (
        f'<div class="prior">previously: <b>{prior}</b></div>' if prior_verdict else ""
    )
    return f"""
    <section class="card" data-id="{rid}">
      <img src="{image_uri}" alt="candidate {rid}" loading="lazy">
      <div class="meta">
        <table>
          <tr><th>Code</th><td>{esc(row['code'])}</td></tr>
          <tr><th>Doc/TD</th><td>{esc(row['doc_td'])}</td></tr>
          <tr><th>Series</th><td>{esc(row['series'])}</td></tr>
          <tr><th>Host</th><td>{esc(row['host'])}</td></tr>
          <tr><th>Licence evidence</th><td>{esc(row['licence_evidence'])}</td></tr>
          <tr><th>Specimen signal</th><td>{esc(row['specimen_signal'])}</td></tr>
          <tr><th>MRZ check</th><td>{esc(row['mrz_check'])}</td></tr>
          <tr><th>Proposed</th><td><b>{esc(row['proposed_destination'])}</b> / {esc(row['proposed_licence'])}</td></tr>
          <tr><th>Note</th><td class="note">{esc(row['note'])}</td></tr>
        </table>
        {prior_note}
        <div class="verdict-buttons" role="group" aria-label="verdict">
          <button type="button" data-verdict="public">Public</button>
          <button type="button" data-verdict="local">Local</button>
          <button type="button" data-verdict="drop">Drop</button>
          <button type="button" data-verdict="" class="clear">Clear</button>
        </div>
        <div class="current">verdict: <span class="current-value">unset</span></div>
      </div>
    </section>"""


PAGE_CSS = """
body { font-family: system-ui, sans-serif; margin: 0; background: #14161a; color: #e8e8e8; }
header { position: sticky; top: 0; background: #1c1f26; padding: 12px 20px; display: flex;
         align-items: center; gap: 16px; box-shadow: 0 2px 8px rgba(0,0,0,.4); z-index: 10; }
header h1 { font-size: 16px; margin: 0; flex: 1; }
#counter { font-variant-numeric: tabular-nums; }
button { cursor: pointer; border: 1px solid #444; background: #262a33; color: #e8e8e8;
         border-radius: 6px; padding: 6px 12px; font-size: 13px; }
button:hover { background: #333947; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(420px, 1fr));
        gap: 16px; padding: 16px; }
.card { background: #1c1f26; border: 1px solid #2c303a; border-radius: 8px; overflow: hidden;
        display: flex; flex-direction: column; }
.card.verdict-public { border-color: #2e7d32; }
.card.verdict-local { border-color: #a06a00; }
.card.verdict-drop { border-color: #8b2020; opacity: .6; }
.card img { width: 100%; max-height: 320px; object-fit: contain; background: #000; }
.meta { padding: 10px 12px; }
table { width: 100%; border-collapse: collapse; font-size: 12.5px; }
th { text-align: left; color: #9aa4b2; vertical-align: top; padding: 3px 8px 3px 0; width: 110px; }
td { padding: 3px 0; }
.note { color: #cfd6e0; }
.prior { font-size: 12px; color: #d9a441; margin: 6px 0; }
.verdict-buttons { margin-top: 10px; display: flex; gap: 6px; }
.verdict-buttons button.active[data-verdict="public"] { background: #2e7d32; border-color: #2e7d32; }
.verdict-buttons button.active[data-verdict="local"] { background: #a06a00; border-color: #a06a00; }
.verdict-buttons button.active[data-verdict="drop"] { background: #8b2020; border-color: #8b2020; }
.current { margin-top: 6px; font-size: 12px; color: #9aa4b2; }
"""

PAGE_JS = """
const verdicts = JSON.parse(document.getElementById('prior-verdicts').textContent);

function cardFor(id) {
  return document.querySelector(`.card[data-id="${CSS.escape(id)}"]`);
}

function applyVerdict(id, verdict, source) {
  if (verdict) {
    verdicts[id] = { verdict, ts: new Date().toISOString(), source: source || 'click' };
  } else {
    delete verdicts[id];
  }
  const card = cardFor(id);
  if (card) {
    card.classList.remove('verdict-public', 'verdict-local', 'verdict-drop');
    if (verdict) card.classList.add('verdict-' + verdict);
    card.querySelectorAll('.verdict-buttons button').forEach(b => {
      b.classList.toggle('active', b.dataset.verdict === verdict);
    });
    card.querySelector('.current-value').textContent = verdict || 'unset';
  }
  updateCounter();
}

function updateCounter() {
  const total = document.querySelectorAll('.card').length;
  const done = Object.keys(verdicts).length;
  document.getElementById('counter').textContent = `${done} of ${total} verdicts set`;
}

document.querySelectorAll('.card').forEach(card => {
  const id = card.dataset.id;
  const prior = verdicts[id];
  if (prior) applyVerdict(id, prior.verdict, 'loaded');
  card.querySelectorAll('.verdict-buttons button').forEach(btn => {
    btn.addEventListener('click', () => applyVerdict(id, btn.dataset.verdict, 'click'));
  });
});
updateCounter();

async function saveVerdicts() {
  const payload = JSON.stringify(verdicts, null, 2);
  const suggestedName = document.body.dataset.verdictsFilename || 'verdicts.json';
  if (window.showSaveFilePicker) {
    try {
      const handle = await window.showSaveFilePicker({
        suggestedName,
        types: [{ description: 'JSON', accept: { 'application/json': ['.json'] } }],
      });
      const writable = await handle.createWritable();
      await writable.write(payload);
      await writable.close();
      return;
    } catch (err) {
      if (err && err.name === 'AbortError') return;
      console.warn('showSaveFilePicker failed, falling back to download', err);
    }
  }
  const blob = new Blob([payload], { type: 'application/json' });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = suggestedName;
  a.click();
  URL.revokeObjectURL(a.href);
}

document.getElementById('save-btn').addEventListener('click', saveVerdicts);

document.getElementById('load-input').addEventListener('change', async (ev) => {
  const file = ev.target.files[0];
  if (!file) return;
  const loaded = JSON.parse(await file.text());
  for (const [id, entry] of Object.entries(loaded)) {
    applyVerdict(id, entry.verdict, 'loaded');
  }
  ev.target.value = '';
});
"""


def build_html(rows: dict[str, dict], prior_verdicts: dict, repo_root: Path, title: str, verdicts_filename: str) -> str:
    cards = "\n".join(
        render_row(row, embed_image(repo_root, row["local_path"]), None)
        for row in rows.values()
    )
    prior_json = json.dumps(prior_verdicts)
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{esc(title)}</title>
<style>{PAGE_CSS}</style>
</head>
<body data-verdicts-filename="{esc(verdicts_filename)}">
<script type="application/json" id="prior-verdicts">{prior_json}</script>
<header>
  <h1>{esc(title)}</h1>
  <span id="counter"></span>
  <label class="load-label">
    <input type="file" id="load-input" accept="application/json" style="display:none">
    <button type="button" onclick="document.getElementById('load-input').click()">Load verdicts</button>
  </label>
  <button type="button" id="save-btn">Save verdicts</button>
</header>
<div class="grid">
{cards}
</div>
<script>{PAGE_JS}</script>
</body>
</html>
"""


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packet", action="append", required=True, type=Path, dest="packets")
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--prior-verdicts", type=Path, default=None)
    parser.add_argument("--title", default="Specimen packet review")
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="Base directory local_path values are resolved against (default: repo root).",
    )
    args = parser.parse_args()

    rows = load_rows(args.packets)
    if not rows:
        print("no rows in the given --packet file(s)", file=sys.stderr)
        raise SystemExit(1)
    prior = load_prior_verdicts(args.prior_verdicts)

    verdicts_filename = args.out.stem.replace("review-", "verdicts-") + ".json"
    html_out = build_html(rows, prior, args.repo_root, args.title, verdicts_filename)
    args.out.write_text(html_out, encoding="utf-8")
    print(f"wrote {len(rows)} row(s) to {args.out}")


if __name__ == "__main__":
    main()
