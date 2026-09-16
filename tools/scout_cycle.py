#!/usr/bin/env python3
"""
scout_cycle.py

Drives one cycle of the specimen-acquisition loop's lane S end to end -- the
mechanical half of P-SCOUT, P-SCREEN and P-PACKET from
`~/.claude/plans/specimen-acquisition-loop.md` (§6.2-6.4) -- so a cycle is one
command instead of a dozen hand-typed steps that each had a way to go wrong:

    suggest   pick uncovered codes from CORPUS_COVERAGE.md, minus what STATE.md
              already marks exhausted or tried, grouped by region; `--plan 3x6`
              prints ready --slice arguments (legal-portal codes first,
              low-web-presence micro-states last)
    task      write work/scouting/cNN/task-wK.txt = the byte-stable PREFIX
              (tools/scout_prefix.txt) + the SUFFIX (codes with full names, held
              series from samples/corpus.jsonl)
    scout     run one dsh scout worker in its own worktree, capture its output,
              extract the JSONL candidates, read its token usage, count faults,
              remove the worktree, record the cycle in STATE.md
    blocked   print the cycle's BLOCKED URLs grouped by reason, so the session
              can pick which to retry; --to-fetch prints just the page URLs
    screen    run tools/screen_candidates.py over the cycle's candidates and
              write the packet's JSON twin for the review artifact; picks up
              work/scouting/cNN/pages/ (session-fetched page HTML) when present
    review    render work/scouting/cNN/review-cNN.html for the user's verdicts
    run       task -> scout (one worker per --slice, in parallel) -> screen
    add-candidates
              append candidates the session found itself (a Firecrawl retry of
              the worker's BLOCKED pages, a hand-found URL) to a cycle, tagged
              as worker "session", before `screen` -- the retry itself is done
              by the Claude session, never by repository code (H14)

What stays with a human or a Claude session, deliberately:

  * the judgement pass (P-SCREEN step 2): host check, opening every staged
    image for H5, the proposed destination and licence class. `screen` stops
    and says so; `review` is run only after the proposal columns are filled.
  * every verdict (H4) and every merge.
  * the worker never fetches anything into the repo (H3): this tool runs the
    fetches itself through screen_candidates.py, on the orchestrator's side.
  * retrying a page that answered the tool with 403/503/an anti-bot shell
    (H14): `blocked` says which pages those are, the session fetches them by
    its own means and drops the HTML into work/scouting/cNN/pages/, and
    `screen` uses it. No repository code calls a scraping service.

Standard library only, matching the sibling tools. `DSH_HOME` is resolved
from the environment or the user-level registry value and always passed to
the worker explicitly (H13): without it dsh silently creates ~/.dsh and every
route fails with NO_ADAPTER. No key is ever printed.

Usage (from the main checkout, or with --repo-root pointing at it):

    python tools/scout_cycle.py suggest --n 6 --region Asia
    python tools/scout_cycle.py run --cycle c11 --slice "THA MAC TWN" --slice "KHM BRN LAO"
    python tools/scout_cycle.py screen --cycle c11          # re-run screening only
    python tools/scout_cycle.py review --cycle c11          # after the judgement pass

Offline unit tests: python -m unittest tools/test_scout_cycle.py
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlparse

# ==========================================================================
# Constants
# ==========================================================================

PREFIX_FILENAME = "scout_prefix.txt"
SCOUTING_DIR = Path("work") / "scouting"
LEDGER_REL = SCOUTING_DIR / "ledger.jsonl"
STATE_REL = SCOUTING_DIR / "STATE.md"
COVERAGE_REL = Path("knowledge") / "CORPUS_COVERAGE.md"
COUNTRIES_REL = Path("crates") / "mrz" / "src" / "countries.rs"
CORPUS_REL = Path("samples") / "corpus.jsonl"

# Regions in countries.rs that §7 of the plan deprioritised on 2026-09-14
# (organisation and legacy codes ran 0/15 across two attempts each). `suggest`
# skips them unless --include-orgs is given.
DEPRIORITISED_REGIONS = (
    "Territories & dependencies commonly seen on documents",
    "ICAO 9303 special / non-ISO codes",
)

# Codes that ran 0-for-N across c08 and c11 and share a cause the prompt cannot fix: a
# state with almost no web-published document material (Pacific and Caribbean micro-states).
# `suggest --plan` puts them last, never out; a phase-4 monitor still revisits them.
LOW_WEB_PRESENCE_CODES = {
    "FSM", "MHL", "NRU", "PLW", "TUV", "KIR", "WSM", "TON", "SLB", "VUT", "COK", "NIU",
    "ATG", "DMA", "GRD", "KNA", "LCA", "VCT", "BRB", "BHS", "STP", "COM",
}

# Fields every worker candidate must carry to count as a record at all. The
# prefix's OUTPUT block names more, but these are the ones later stages read.
REQUIRED_CANDIDATE_KEYS = ("code", "image_url", "page_url")

# `dir` in samples/corpus.jsonl -> the label the SUFFIX uses for "already held".
DIR_LABELS = {
    "passports": "passport",
    "id_cards": "id-card",
    "driving_licenses": "driving-licence",
    "misc": "other",
    "ocr_fixtures": "fixture",
}

CYCLE_TABLE_HEADER = (
    "| cycle | codes | wall | tokens u/cr/out | scouted | screened | packet | pub/loc/drop | review min | faults |"
)
CYCLE_COL = {"cycle": 0, "codes": 1, "wall": 2, "tokens": 3, "scouted": 4, "screened": 5, "packet": 6}


# ==========================================================================
# Pure helpers (unit-tested, no I/O)
# ==========================================================================


def parse_country_table(rust_source: str) -> list[tuple[str, str, str]]:
    """Reads `CODES` out of crates/mrz/src/countries.rs as (region, code, name)
    triples, in table order. The region is the nearest preceding
    `// ── <region> ──` comment. Parsing the source keeps the two in sync
    without a build step; the table is plain string literals by design."""
    triples: list[tuple[str, str, str]] = []
    region = ""
    for line in rust_source.splitlines():
        m_region = re.match(r"\s*//\s*──\s*(.+?)\s*──\s*$", line)
        if m_region:
            region = m_region.group(1)
            continue
        m_pair = re.match(r'\s*\("([A-Z<]{1,3})",\s*"((?:[^"\\]|\\.)*)"\),', line)
        if m_pair:
            code = m_pair.group(1)
            name = m_pair.group(2).replace('\\"', '"')
            triples.append((region, code, name))
    return triples


def load_prefix_text(text: str) -> str:
    """Normalises line endings and checks the prefix is what the worker's
    cache expects: it must end with the `TASK:` line, and nothing after it."""
    text = text.replace("\r\n", "\n")
    if not text.endswith("TASK:\n"):
        raise ValueError(f"{PREFIX_FILENAME} must end with a 'TASK:' line followed by one newline")
    return text


def filename_country_token(name: str) -> str:
    """`Hong Kong` -> `Hong_Kong`, the way samples/README.md names files."""
    return re.sub(r"\s+", "_", name.strip())


def held_series(corpus_rows: list[dict], code: str, name: str) -> str:
    """What the corpus already holds for a code, as the SUFFIX phrases it:
    `passport 2005, 2014; id-card 2017` or `none held`. A row matches by its
    MRZ issuing state (padding `<` stripped -- Germany reads `D<<`) or, for
    sides without an MRZ, by the country-name prefix of its filename."""
    token = filename_country_token(name) + "_"
    by_label: dict[str, list[str]] = {}
    for row in corpus_rows:
        mrz = row.get("mrz") or {}
        state = (mrz.get("issuing_state") or "").rstrip("<")
        filename = row.get("filename") or ""
        if state != code and not filename.startswith(token):
            continue
        if row.get("dir") == "ocr_fixtures":
            continue  # fixtures duplicate passports/ rows; the hint would only repeat itself
        label = DIR_LABELS.get(row.get("dir") or "", row.get("dir") or "other")
        year = row.get("year") or {}
        year_str = str(year.get("value")) if isinstance(year, dict) and year.get("value") else "year unknown"
        by_label.setdefault(label, [])
        if year_str not in by_label[label]:
            by_label[label].append(year_str)
    if not by_label:
        return "none held"
    parts = []
    for label, years in by_label.items():
        parts.append(f"{label} {', '.join(sorted(years))}")
    return "; ".join(parts)


LEGAL_PORTALS_FILENAME = "legal_portals.json"


def load_legal_portals(path: Path) -> dict[str, str]:
    """code -> portal domain from tools/legal_portals.json; keys starting with
    `_` are documentation. Missing file: no hints, never an error."""
    if not path.is_file():
        return {}
    data = json.loads(path.read_text(encoding="utf-8"))
    return {k: v["portal"] for k, v in data.items() if not k.startswith("_") and isinstance(v, dict) and v.get("portal")}


def build_suffix(codes: list[str], names: dict[str, str], held: dict[str, str], portals: dict[str, str] | None = None) -> str:
    """The variable tail of the task, exactly in the plan's §8 shape. A code
    with a known legal-acts portal gets it as a search hint next to its name:
    the regulation defining a document is where its specimen annex lives."""
    portals = portals or {}

    def label(c: str) -> str:
        return f"{c} ({names[c]}; legal-acts portal: {portals[c]})" if c in portals else f"{c} ({names[c]})"

    scout = ", ".join(label(c) for c in codes)
    held_part = "; ".join(f"{c}: {held.get(c, 'none held')}" for c in codes)
    return f"Codes to scout: {scout}. Already held (skip these series): {held_part}.\n"


def extract_candidates(worker_output: str) -> tuple[list[dict], dict[str, str]]:
    """Pulls the worker's JSONL records and its per-code summary lines out of
    the captured stdout+stderr. The reasoning stream repeats the output
    template with `"..."` URLs and drafts records before the final answer, so:
    a line must parse as a JSON object, carry the required keys, and have
    http(s) URLs; duplicates by (image_url, page_url) keep the LAST
    occurrence, which is the final answer's copy."""
    seen: dict[tuple[str, str], dict] = {}
    per_code: dict[str, str] = {}
    for raw in worker_output.splitlines():
        line = raw.strip()
        if line.startswith("{") and line.endswith("}"):
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue
            if not isinstance(obj, dict) or any(k not in obj for k in REQUIRED_CANDIDATE_KEYS):
                continue
            image_url = str(obj.get("image_url") or "")
            page_url = str(obj.get("page_url") or "")
            if not (image_url.startswith("http") and page_url.startswith("http")):
                continue
            key = (image_url, page_url)
            seen.pop(key, None)
            seen[key] = obj
            continue
        m = re.match(r"^[^A-Za-z0-9]*([A-Z<]{1,3}):\s*(\d+|none)\s+found\b(.*)$", line)
        if m:
            code, count, rest = m.group(1), m.group(2), m.group(3).strip(" —-:")
            per_code[code] = count if count != "none" else ("none found" + (f" — {rest}" if rest else ""))
    return list(seen.values()), per_code


def extract_blocked(worker_output: str) -> list[dict]:
    """`BLOCKED: <url> — <reason>` lines from the worker's final answer (prefix
    v3). The URL is copied verbatim; the reason is free text. Duplicates by
    URL keep the last occurrence, like candidates."""
    seen: dict[str, dict] = {}
    for raw in worker_output.splitlines():
        m = re.match(r"^[^A-Za-z0-9]*BLOCKED:\s*(\S+)\s*(?:[—–-]|:)?\s*(.*)$", raw.strip())
        if not m:
            continue
        url = m.group(1).rstrip(".,;:")
        if not url.startswith("http"):
            continue
        seen.pop(url, None)
        seen[url] = {"url": url, "reason": m.group(2).strip() or "unspecified"}
    return list(seen.values())


def tag_candidates(candidates: list[dict], cycle: str, worker: str) -> list[dict]:
    tagged = []
    for c in candidates:
        rec = dict(c)
        rec["status"] = "unreviewed"
        rec["cycle"] = cycle
        rec["worker"] = worker
        tagged.append(rec)
    return tagged


def parse_token_totals(session_json: dict) -> dict[str, int] | None:
    """dsh's session record keeps running totals under
    record.rows.tokenUsage.val.totals (format version 7, dsh 0.1.5)."""
    try:
        totals = session_json["record"]["rows"]["tokenUsage"]["val"]["totals"]
    except (KeyError, TypeError):
        return None
    if not isinstance(totals, dict):
        return None
    return {
        "uncached": int(totals.get("uncachedInputTokens") or 0),
        "cache_read": int(totals.get("cacheReadTokens") or 0),
        "output": int(totals.get("outputTokens") or 0),
    }


def format_tokens(totals: dict[str, int] | None) -> str:
    """`205.1k / 2.36M / 21.1k`, the Cycles-table convention."""
    if not totals:
        return "unrecorded"

    def fmt(n: int) -> str:
        if n >= 1_000_000:
            return f"{n / 1_000_000:.2f}M"
        if n >= 1_000:
            return f"{n / 1_000:.1f}k"
        return str(n)

    return f"{fmt(totals['uncached'])} / {fmt(totals['cache_read'])} / {fmt(totals['output'])}"


def format_wall(seconds: float) -> str:
    s = int(round(seconds))
    return f"~{s} s ({s // 60}m{s % 60:02d}s)"


def session_matches(session_json: dict, worktree: Path, since_ms: int) -> bool:
    """A session belongs to this worker when dsh recorded our worktree as its
    cwd and it was created after we launched -- two workers in parallel each
    have their own worktree, so this never confuses them."""
    try:
        identity = session_json["record"]["identity"]
        cwd = str(identity.get("cwd") or "")
        created = int(identity.get("createdAt") or 0)
    except (KeyError, TypeError):
        return False
    return _norm_path(cwd) == _norm_path(str(worktree)) and created >= since_ms


def _norm_path(p: str) -> str:
    return p.replace("\\", "/").rstrip("/").lower()


def detect_faults(git_status_short: str) -> list[str]:
    """Anything the worker wrote outside `scratch/` is a fault (H3, plan §3)."""
    faults = []
    for line in git_status_short.splitlines():
        if len(line) < 4:
            continue
        path = line[3:].strip().strip('"')
        if path.startswith("scratch/") or path == "scratch":
            continue
        faults.append(f"wrote outside scratch/: {path}")
    return faults


def packet_rows_from_screened(screened: list[dict]) -> list[dict]:
    """The JSON twin of the Markdown packet, in the row shape
    tools/build_review_artifact.py requires. Mirrors
    screen_candidates.py::write_packet cell for cell; proposal columns are
    left blank for the judgement pass. No holder value is ever present in a
    screened record, so none can reach this file."""
    rows = []
    for rec in screened:
        if rec.get("auto_reject") is not None:
            continue
        staged = (rec.get("staged_path") or "").replace("\\", "/")
        row_id = Path(staged).stem if staged else (rec.get("sha256") or "")[:12]
        td = rec.get("td") or ""
        doc_td = f"{rec.get('doc', '')} / {td}" if td else rec.get("doc", "")
        host = urlparse(rec.get("page_url_final") or rec.get("page_url") or "").netloc
        licence = "; ".join(rec.get("licence_snippets") or []) or rec.get("licence_evidence", "")
        specimen = "; ".join(rec.get("specimen_snippets") or []) or rec.get("specimen_signal", "")
        note_parts = []
        if rec.get("needs_eyes"):
            note_parts.append("needs eyes")
        if rec.get("h5_check"):
            note_parts.append("h5 check")
        if rec.get("variant_of"):
            note_parts.append(f"variant of {rec['variant_of']}")
        if rec.get("pdf_source"):
            note_parts.append(f"from PDF p.{rec['pdf_source'].get('page')} ({rec['pdf_source'].get('kind')})")
        if rec.get("page_source") == "session-fetched":
            note_parts.append("page: session-fetched")
        if (rec.get("side") or "").lower() == "cover":
            note_parts.append("cover only (ADR-0012: labelled class, never a coverage claim)")
        rows.append(
            {
                "id": row_id,
                "code": rec.get("code", ""),
                "doc_td": doc_td,
                "series": rec.get("series", ""),
                "host": host,
                "licence_evidence": licence,
                "specimen_signal": specimen,
                "mrz_check": rec.get("mrz_check", "none"),
                "proposed_destination": "",
                "proposed_licence": "",
                "local_path": staged,
                "note": "; ".join(note_parts),
            }
        )
    return rows


COVER_ONLY_PREFIX = "cover only, data page still wanted"


def cover_only_codes_line(cycle: str, codes: list[str]) -> str:
    """The STATE.md `## Codes` line a cohort that placed only covers leaves
    behind, in the wording already used by hand for c12/c13/c14. A cover never
    moves a code's CORPUS_COVERAGE.md status (ADR-0012), so without this line
    the code looks untried and `suggest` re-proposes it at full priority --
    which is exactly what happened to all fourteen c13 codes on c14."""
    return f"{COVER_ONLY_PREFIX} ({cycle}): {' '.join(codes)}"


def cover_only_codes(state_text: str, known_codes: set[str]) -> set[str]:
    """Codes whose ONLY history in STATE.md's `## Codes` section is a
    `cover only, data page still wanted (cNN): ...` line. A code named on such
    a line and also anywhere else in the section (exhausted, 1/2, a narrative
    line) keeps that other history and stays excluded -- the cover line is a
    demotion, never a promotion."""
    m = re.search(r"^## Codes\s*$(.*?)(?=^## |\Z)", state_text, re.M | re.S)
    if not m:
        return set()
    on_cover_lines: set[str] = set()
    elsewhere: set[str] = set()
    for line in m.group(1).splitlines():
        tokens = set(re.findall(r"(?<![A-Za-z0-9_])([A-Z<]{1,3})(?![A-Za-z0-9_])", line)) & known_codes
        if line.strip().startswith(COVER_ONLY_PREFIX):
            on_cover_lines |= tokens
        else:
            elsewhere |= tokens
    return on_cover_lines - elsewhere


BLOCKED_REASON_BUCKETS = (
    ("403", "403 forbidden"),
    ("401", "401 unauthorized"),
    ("429", "429 rate-limited"),
    ("503", "503 unavailable"),
    ("500", "500 server error"),
    ("202", "202 anti-bot interstitial"),
    ("cloudflare", "anti-bot / Cloudflare"),
    ("captcha", "anti-bot / Cloudflare"),
    ("timeout", "timeout"),
    ("timed out", "timeout"),
    ("content type", "unreadable content type"),
    ("javascript", "JavaScript-only page"),
    ("404", "404 not found"),
)


def blocked_reason_bucket(reason: str) -> str:
    """The worker writes free text after `BLOCKED: <url> -- `, so grouping on
    the raw string gives one group per URL. This buckets it to the handful of
    causes that decide what the session should do: a 403/503/anti-bot page is
    worth a session-side refetch, a 404 is not."""
    text = (reason or "").lower()
    for needle, bucket in BLOCKED_REASON_BUCKETS:
        if needle in text:
            return bucket
    return "other"


def group_blocked(rows: list[dict]) -> dict[str, list[dict]]:
    """Blocked rows grouped by `blocked_reason_bucket`, buckets in first-seen
    order, rows in file order."""
    groups: dict[str, list[dict]] = {}
    for row in rows:
        groups.setdefault(blocked_reason_bucket(row.get("reason", "")), []).append(row)
    return groups


def coverage_uncovered_codes(coverage_text: str) -> list[str]:
    """Codes whose CORPUS_COVERAGE.md row reads `No specimen yet`."""
    codes = []
    for line in coverage_text.splitlines():
        m = re.match(r"^\| ([A-Z<]{1,3}) \|[^|]*\|[^|]*\| ([^|]*)\|", line)
        if m and m.group(2).strip().startswith("No specimen yet"):
            codes.append(m.group(1))
    return codes


def state_codes_mentioned(state_text: str, known_codes: set[str]) -> set[str]:
    """Every known code named anywhere in STATE.md's `## Codes` section --
    exhausted, 1/2, tried once, resolved, claimed, or in a narrative line such
    as `CHL/CRI/ECU/BOL/KEN all none found`. Conservative on purpose: a code
    that appears there at all has a history the orchestrator should read
    before re-scouting it, so `suggest` leaves it out. Intersecting with the
    countries.rs table is what keeps `PR`, `HIT`, `MRZ` and friends out."""
    m = re.search(r"^## Codes\s*$(.*?)(?=^## |\Z)", state_text, re.M | re.S)
    if not m:
        return set()
    tokens = set(re.findall(r"(?<![A-Za-z0-9_])([A-Z<]{1,3})(?![A-Za-z0-9_])", m.group(1)))
    return tokens & known_codes


def suggest_codes(
    table: list[tuple[str, str, str]],
    uncovered: list[str],
    exclude: set[str],
    region: str | None,
    include_orgs: bool,
) -> list[tuple[str, str, str]]:
    uncovered_set = set(uncovered)
    out = []
    for reg, code, name in table:
        if code not in uncovered_set or code in exclude:
            continue
        if not include_orgs and reg in DEPRIORITISED_REGIONS:
            continue
        if region and region.lower() not in reg.lower():
            continue
        out.append((reg, code, name))
    return out


def prioritise_codes(
    picks: list[tuple[str, str, str]],
    portals: dict[str, str],
    cover_only: set[str] | frozenset[str] = frozenset(),
) -> list[tuple[str, str, str]]:
    """Pace v3 (plan §7, T10): codes with a legal-acts portal first (both gazette hits
    so far came from one), then every other fresh code in countries.rs order, then
    codes a past cycle found a cover for but no data page (T13 B3 -- worth
    revisiting, but after anything untried), then the low-web-presence
    micro-states last. Stable within each tier."""
    fresh = [t for t in picks if t[1] not in cover_only and t[1] not in LOW_WEB_PRESENCE_CODES]
    tier1 = [t for t in fresh if t[1] in portals]
    tier2 = [t for t in fresh if t[1] not in portals]
    tier3 = [t for t in picks if t[1] in cover_only and t[1] not in LOW_WEB_PRESENCE_CODES]
    tier4 = [t for t in picks if t[1] in LOW_WEB_PRESENCE_CODES]
    return tier1 + tier2 + tier3 + tier4


def plan_slices(ordered: list[str], workers: int, per_worker: int) -> list[list[str]]:
    """Deals the first workers*per_worker codes round-robin into `workers`
    slices, so each worker gets a mix of the priority tiers rather than one
    worker getting every good code."""
    take = ordered[: workers * per_worker]
    slices: list[list[str]] = [[] for _ in range(workers)]
    for i, code in enumerate(take):
        slices[i % workers].append(code)
    return [sl for sl in slices if sl]


def parse_plan(spec: str) -> tuple[int, int]:
    m = re.fullmatch(r"\s*(\d+)\s*[xX×]\s*(\d+)\s*", spec or "")
    if not m or int(m.group(1)) < 1 or int(m.group(2)) < 1:
        raise RuntimeError(f"--plan wants WORKERSxCODES, e.g. 3x6, got {spec!r}")
    return int(m.group(1)), int(m.group(2))


def now_iso() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%MZ")


def _int_or_zero(value: str | None) -> int:
    """A Cycles cell read back as a count. Anything that is not a plain integer
    (the placeholder dash, a merged note) counts as zero rather than raising --
    a malformed cell must not cost a cycle its candidates."""
    try:
        return int((value or "").strip())
    except ValueError:
        return 0


def cycle_row(cycle: str, codes: str, wall: str, tokens: str, scouted: int, faults: int) -> str:
    return f"| {cycle} | {codes} | {wall} | {tokens} | {scouted} | — | — | TBD | TBD | {faults} |"


def insert_cycle_row(state_text: str, row: str) -> str:
    """Appends a row to the end of the `## Cycles` table (before its footnotes
    and the `## Log` heading). Missing table: the row is appended to the end
    of the file with a marker line, never lost."""
    lines = state_text.splitlines(keepends=True)
    start = None
    for i, line in enumerate(lines):
        if line.strip() == "## Cycles":
            start = i
            break
    if start is None:
        return state_text.rstrip("\n") + "\n\n## Cycles\n" + CYCLE_TABLE_HEADER + "\n|---|---|---|---|---|---|---|---|---|---|\n" + row + "\n"
    last_row = None
    for j in range(start + 1, len(lines)):
        stripped = lines[j].strip()
        if stripped.startswith("## "):
            break
        if stripped.startswith("|"):
            last_row = j
    if last_row is None:
        lines.insert(start + 1, row + "\n")
    else:
        lines.insert(last_row + 1, row + "\n")
    return "".join(lines)


def update_cycle_cells(state_text: str, cycle: str, cells: dict[str, str]) -> str:
    """Fills named columns of an existing `| cNN | ...` row in place; a
    missing row leaves the text unchanged (the caller logs instead)."""
    lines = state_text.splitlines(keepends=True)
    for i, line in enumerate(lines):
        if not line.startswith(f"| {cycle} |"):
            continue
        parts = line.rstrip("\n").split("|")
        # parts[0] is '' before the first pipe; column k is parts[k+1]
        for name, value in cells.items():
            idx = CYCLE_COL[name] + 1
            if idx < len(parts):
                parts[idx] = f" {value} "
        lines[i] = "|".join(parts) + "\n"
        return "".join(lines)
    return state_text


def read_cycle_cells(state_text: str, cycle: str) -> dict[str, str] | None:
    """The named columns of an existing `| cNN | ...` Cycles row, stripped, or
    `None` when the cycle has no row yet. The counterpart to
    `update_cycle_cells`: a rerun of one failed slice has to add to what the
    finished workers already recorded, which means reading it first."""
    for line in state_text.splitlines():
        if not line.startswith(f"| {cycle} |"):
            continue
        parts = line.rstrip().split("|")
        return {name: parts[idx + 1].strip() if idx + 1 < len(parts) else "" for name, idx in CYCLE_COL.items()}
    return None


def merge_codes_cell(old: str, new: str) -> str:
    """The Cycles row's codes cell after a rerun: the codes already recorded,
    then any the rerun added, first occurrence wins."""
    seen: list[str] = []
    for code in (old or "").split() + (new or "").split():
        if code and code not in seen:
            seen.append(code)
    return " ".join(seen)


def merge_token_cells(old: str, new: str) -> str:
    """The Cycles row's token cell after a rerun. The cell holds already
    formatted values (`205.1k / 2.36M / 21.1k`), which cannot be added back up
    without inventing precision the format threw away -- so two runs are
    stated as a sum of terms rather than one wrong number."""
    old, new = (old or "").strip(), (new or "").strip()
    if not old or old in ("-", "—", "unrecorded"):
        return new or "unrecorded"
    if not new or new == "unrecorded":
        return old
    return f"{old} + {new}"


def merge_worker_candidates(existing: list[dict], incoming: list[dict]) -> tuple[list[dict], int]:
    """Appends already-tagged worker candidates to a cycle's list, skipping any
    (image_url, page_url) pair already present, and returns `(merged, added)`.
    `merge_candidates` above is the session-side twin (it tags and validates
    rows the session hands in); this one takes rows a worker produced and a
    rerun must not duplicate or overwrite (T13 B2)."""
    seen = {(c.get("image_url"), c.get("page_url")) for c in existing}
    merged = list(existing)
    added = 0
    for c in incoming:
        key = (c.get("image_url"), c.get("page_url"))
        if key in seen:
            continue
        seen.add(key)
        merged.append(c)
        added += 1
    return merged, added


def add_codes_line(state_text: str, line: str) -> str:
    """Appends a line to the end of STATE.md's `## Codes` section (just before
    the next heading), so `suggest` and the next session see it."""
    lines = state_text.splitlines(keepends=True)
    start = None
    for i, l in enumerate(lines):
        if l.strip() == "## Codes":
            start = i
            break
    if start is None:
        return state_text.rstrip("\n") + "\n\n## Codes\n" + line + "\n"
    end = len(lines)
    for j in range(start + 1, len(lines)):
        if lines[j].startswith("## "):
            end = j
            break
    insert_at = end
    while insert_at > start + 1 and lines[insert_at - 1].strip() == "":
        insert_at -= 1
    lines.insert(insert_at, line + "\n")
    return "".join(lines)


def append_log(state_text: str, line: str) -> str:
    text = state_text if state_text.endswith("\n") else state_text + "\n"
    if "## Log" not in text:
        text += "\n## Log (append-only, one line per step)\n"
    return text + f"{now_iso()} {line}\n"


def touch_updated(state_text: str) -> str:
    return re.sub(r"^updated: \S+", f"updated: {now_iso()}", state_text, count=1, flags=re.M)


# ==========================================================================
# Environment resolution
# ==========================================================================


def resolve_dsh_home(env: dict | None = None) -> str:
    """H13: DSH_HOME must be set for every dsh call. Shells started before the
    install don't carry it, so fall back to the user-level registry value; if
    neither exists, refuse rather than let dsh create ~/.dsh silently."""
    env = os.environ if env is None else env
    value = env.get("DSH_HOME")
    if value:
        return value
    if sys.platform == "win32":
        try:
            import winreg  # type: ignore

            with winreg.OpenKey(winreg.HKEY_CURRENT_USER, "Environment") as key:
                value, _ = winreg.QueryValueEx(key, "DSH_HOME")
                if value:
                    return str(value)
        except OSError:
            pass
    raise RuntimeError("DSH_HOME is not set (environment or user registry); see HARNESS.md §5")


def resolve_dsh_command() -> list[str]:
    """The npm shim (`dsh.cmd`) goes through cmd.exe, which mangles the quotes
    inside the task text. Call node on dsh's bin.js directly instead, so the
    task reaches the worker byte for byte."""
    shim = shutil.which("dsh.cmd") or shutil.which("dsh")
    if not shim:
        raise RuntimeError("dsh not found on PATH (npm global)")
    prefix_dir = Path(shim).resolve().parent
    bin_js = prefix_dir / "node_modules" / "@deepseek-ai" / "dsh" / "lib" / "bin.js"
    if not bin_js.is_file():
        raise RuntimeError(f"dsh entry point not found at {bin_js}")
    node = prefix_dir / "node.exe"
    node_cmd = str(node) if node.is_file() else (shutil.which("node") or "node")
    return [node_cmd, str(bin_js)]


def find_repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def worktrees_root(repo_root: Path) -> Path:
    """`$PROJECTS_ROOT/worktrees/<repo>/` per worktrees/CLAUDE.md."""
    return repo_root.parent / "worktrees" / repo_root.name


# ==========================================================================
# Subprocess wrappers
# ==========================================================================


def run_cmd(cmd: list[str], cwd: Path | None = None, check: bool = True, env: dict | None = None) -> str:
    proc = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, check=False, env=env, encoding="utf-8", errors="replace")
    if check and proc.returncode != 0:
        raise RuntimeError(f"command failed ({proc.returncode}): {' '.join(cmd)}\n{proc.stdout}\n{proc.stderr}")
    return proc.stdout


_state_lock = threading.Lock()


def edit_state(repo_root: Path, fn) -> None:
    """Read-modify-write STATE.md under a lock (parallel workers log from
    threads). STATE.md is gitignored; a missing file is created from the
    plan's §10 skeleton so nothing is lost."""
    path = repo_root / STATE_REL
    with _state_lock:
        if path.is_file():
            text = path.read_text(encoding="utf-8")
        else:
            text = (
                "# Specimen loop — state   (truth for status; plan = truth for procedure)\n"
                f"updated: {now_iso()} · plan: ~/.claude/plans/specimen-acquisition-loop.md (v2)\n\n"
                "## Locks\nsamples-data lock: free\nWIP packet: none\n\n## Tasks\n\n## Codes\n\n"
                "## Cycles\n" + CYCLE_TABLE_HEADER + "\n|---|---|---|---|---|---|---|---|---|---|\n\n"
                "## Log (append-only, one line per step)\n"
            )
        text = touch_updated(fn(text))
        path.write_text(text, encoding="utf-8")


# ==========================================================================
# Steps
# ==========================================================================


def load_table(repo_root: Path) -> list[tuple[str, str, str]]:
    return parse_country_table((repo_root / COUNTRIES_REL).read_text(encoding="utf-8"))


def load_jsonl(path: Path) -> list[dict]:
    if not path.is_file():
        return []
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line:
            rows.append(json.loads(line))
    return rows


def step_suggest(args, repo_root: Path) -> int:
    table = load_table(repo_root)
    coverage = (repo_root / COVERAGE_REL).read_text(encoding="utf-8")
    uncovered = coverage_uncovered_codes(coverage)
    state_path = repo_root / STATE_REL
    known = {code for _, code, _ in table}
    state_text = state_path.read_text(encoding="utf-8") if state_path.is_file() else ""
    mentioned = state_codes_mentioned(state_text, known)
    covers = cover_only_codes(state_text, known)
    # A cover-only code is not exhausted -- a cycle found a cover for it and no
    # data page -- so it stays in the suggestion list and is ranked down instead
    # (prioritise_codes' third tier). Everything else named in `## Codes` is out.
    exclude = (mentioned - covers) | set(args.exclude or [])
    picks = suggest_codes(table, uncovered, exclude, args.region, args.include_orgs)
    print(
        f"{len(uncovered)} codes read 'No specimen yet'; {len(mentioned)} already appear in STATE.md's "
        f"Codes section ({len(covers)} of them only as '{COVER_ONLY_PREFIX}', kept and ranked after fresh codes)."
    )
    if not picks:
        print("nothing left to suggest under these filters")
        return 0
    if args.plan:
        workers, per_worker = parse_plan(args.plan)
        portals = load_legal_portals(Path(__file__).resolve().parent / LEGAL_PORTALS_FILENAME)
        ordered = [code for _, code, _ in prioritise_codes(picks, portals, covers)]
        slices = plan_slices(ordered, workers, per_worker)
        names = {code: name for _, code, name in picks}
        print(
            f"\nplan {workers}x{per_worker}: portal codes first, then fresh codes, then cover-only "
            "codes, low-web-presence micro-states last"
        )

        def tag(c: str) -> str:
            marks = []
            if c in portals:
                marks.append("portal")
            if c in covers:
                marks.append("cover only")
            return ("; " + ", ".join(marks)) if marks else ""

        for i, sl in enumerate(slices, 1):
            print(f"  w{i}: " + "  ".join(f"{c} ({names[c]}{tag(c)})" for c in sl))
        cycle = args.cycle or "cNN"
        print("\n  python tools/scout_cycle.py run --cycle " + cycle + " " + " ".join(f'--slice "{" ".join(sl)}"' for sl in slices))
        return 0
    by_region: dict[str, list[tuple[str, str]]] = {}
    for reg, code, name in picks:
        by_region.setdefault(reg, []).append((code, name))
    for reg, items in by_region.items():
        shown = items[: args.n] if args.n else items
        print(f"\n{reg} ({len(items)} open):")
        print("  " + " ".join(code for code, _ in shown))
        for code, name in shown:
            print(f"    {code}  {name}")
    return 0


def build_task_text(repo_root: Path, codes: list[str]) -> str:
    table = load_table(repo_root)
    names = {code: name for _, code, name in table}
    unknown = [c for c in codes if c not in names]
    if unknown:
        raise RuntimeError(f"not in countries.rs: {' '.join(unknown)}")
    prefix_path = Path(__file__).resolve().parent / PREFIX_FILENAME
    prefix = load_prefix_text(prefix_path.read_text(encoding="utf-8"))
    corpus_rows = load_jsonl(repo_root / CORPUS_REL)
    held = {c: held_series(corpus_rows, c, names[c]) for c in codes}
    portals = load_legal_portals(Path(__file__).resolve().parent / LEGAL_PORTALS_FILENAME)
    return prefix + build_suffix(codes, names, held, portals)


def warn_on_state_history(repo_root: Path, codes: list[str]) -> None:
    state_path = repo_root / STATE_REL
    if not state_path.is_file():
        return
    mentioned = state_codes_mentioned(state_path.read_text(encoding="utf-8"), set(codes))
    hits = [c for c in codes if c in mentioned]
    if hits:
        print(f"note: {' '.join(hits)} already appear in STATE.md's Codes section -- check their history before scouting again")


def step_task(args, repo_root: Path) -> Path:
    codes = [c.upper() for c in args.codes]
    warn_on_state_history(repo_root, codes)
    text = build_task_text(repo_root, codes)
    cycle_dir = repo_root / SCOUTING_DIR / args.cycle
    cycle_dir.mkdir(parents=True, exist_ok=True)
    out = cycle_dir / f"task-{args.worker}.txt"
    out.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {out} ({len(text)} chars): {text[text.index('TASK:') + 6:].strip()}")
    return out


class WorkerResult:
    def __init__(self, cycle: str, worker: str, codes: list[str]):
        self.cycle = cycle
        self.worker = worker
        self.codes = codes
        self.wall_s = 0.0
        self.exit_code: int | None = None
        self.candidates: list[dict] = []
        self.blocked: list[dict] = []
        self.per_code: dict[str, str] = {}
        self.tokens: dict[str, int] | None = None
        self.faults: list[str] = []
        self.out_path: Path | None = None


def find_session_tokens(dsh_home: str, worktree: Path, since_ms: int) -> dict[str, int] | None:
    sessions = Path(dsh_home) / "storages" / "session_projcache" / "sessions"
    if not sessions.is_dir():
        return None
    best: dict[str, int] | None = None
    for p in sorted(sessions.glob("session-*.json"), key=lambda q: q.stat().st_mtime, reverse=True):
        try:
            data = json.loads(p.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        if session_matches(data, worktree, since_ms):
            best = parse_token_totals(data)
            break
    return best


CONFIG_LOCK_RETRY_DELAY_S = 2.0


def is_git_config_lock_error(message: str) -> bool:
    """True for git's "could not lock config file .git/config: File exists".
    Three worktree creations issued from three pool threads at once race on
    that one lock file, and the loser dies -- w1 was lost that way on cycle
    c14. Creation is sequential now (see `step_scout`); this is the second
    line of defence, for a lock another process happens to hold."""
    text = (message or "").lower()
    return "could not lock config file" in text or ("config.lock" in text and "file exists" in text)


def worker_worktree(repo_root: Path, cycle: str, worker: str) -> tuple[str, Path]:
    """`(branch name, worktree path)` for one scout worker -- one place, so the
    sequential setup, the run and the sequential teardown cannot disagree."""
    name = f"scout-{cycle}-{worker}"
    return name, worktrees_root(repo_root) / name


def add_scout_worktree(repo_root: Path, name: str, worktree: Path) -> None:
    """Creates one worker's worktree off `origin/main`, retrying once after
    `CONFIG_LOCK_RETRY_DELAY_S` when git reports a config lock. Called
    sequentially from the main thread before the pool starts."""
    if worktree.exists():
        raise RuntimeError(f"worktree already exists: {worktree} (a previous run did not clean up)")
    cmd = ["git", "-C", str(repo_root), "worktree", "add", "-b", name, str(worktree), "origin/main"]
    try:
        run_cmd(cmd)
    except RuntimeError as e:
        if not is_git_config_lock_error(str(e)):
            raise
        print(f"git config was locked creating {name}; retrying once in {CONFIG_LOCK_RETRY_DELAY_S:.0f} s")
        time.sleep(CONFIG_LOCK_RETRY_DELAY_S)
        run_cmd(cmd)


def remove_scout_worktree(repo_root: Path, name: str, worktree: Path) -> None:
    """Removes one worker's worktree and its branch. Never raises: a leftover
    worktree is a nuisance the next run reports, not a reason to lose a
    cycle's candidates."""
    run_cmd(["git", "-C", str(repo_root), "worktree", "remove", "--force", str(worktree)], check=False)
    run_cmd(["git", "-C", str(repo_root), "branch", "-D", name], check=False)


def run_worker(repo_root: Path, cycle: str, worker: str, task_path: Path, codes: list[str], route: str | None, worktree: Path) -> WorkerResult:
    """P-SCOUT steps 3-5 for one worker, in the worktree `step_scout` already
    created for it: dsh headless with the task text, output captured,
    candidates extracted, tokens and faults recorded. Creating and removing
    the worktree is deliberately NOT done here -- that writes `.git/config`,
    and three of these running in parallel raced on its lock (cycle c14)."""
    result = WorkerResult(cycle, worker, codes)
    dsh_home = resolve_dsh_home()
    dsh_cmd = resolve_dsh_command()
    name = worktree.name
    cycle_dir = repo_root / SCOUTING_DIR / cycle
    out_path = cycle_dir / f"worker-{worker}.out"
    result.out_path = out_path

    env = dict(os.environ)
    env["DSH_HOME"] = dsh_home
    cmd = list(dsh_cmd) + ["--profile", "headless"]
    if route:
        cmd += ["--patch", str(Path(dsh_home) / "routes" / f"{route}.yml")]
    task_text = task_path.read_text(encoding="utf-8")
    cmd.append(task_text)

    since_ms = int(time.time() * 1000) - 5_000
    print(f"[{worker}] launched {name} for {' '.join(codes)} (route={route or 'default'})")
    started = time.monotonic()
    try:
        with open(out_path, "w", encoding="utf-8", errors="replace") as out:
            proc = subprocess.run(cmd, cwd=worktree, stdout=out, stderr=subprocess.STDOUT, env=env, check=False)
        result.exit_code = proc.returncode
    finally:
        result.wall_s = time.monotonic() - started

    output = out_path.read_text(encoding="utf-8", errors="replace")
    candidates, per_code = extract_candidates(output)
    result.candidates = tag_candidates(candidates, cycle, worker)
    result.blocked = [dict(b, cycle=cycle, worker=worker) for b in extract_blocked(output)]
    result.per_code = per_code
    result.tokens = find_session_tokens(dsh_home, worktree, since_ms)

    status = run_cmd(["git", "-C", str(worktree), "status", "--short", "--untracked-files=all"], check=False)
    result.faults = detect_faults(status)
    if result.exit_code != 0:
        result.faults.append(f"dsh exited {result.exit_code}")
    missing = [c for c in codes if c not in per_code]
    if missing:
        # No `<CODE>: n found` / `none found` line for a code means the worker never reported
        # on it -- a route error or a truncated run looks exactly like "none found" otherwise.
        result.faults.append(f"no summary line for {' '.join(missing)} (see {out_path.name})")

    print(f"[{worker}] done in {format_wall(result.wall_s)}: {len(result.candidates)} candidate(s), faults {len(result.faults)}")
    return result


def record_scout(repo_root: Path, results: list[WorkerResult]) -> Path:
    """Writes candidates-cNN.jsonl (all workers), then the Cycles row and Log
    lines in STATE.md.

    Both files and the Cycles row are MERGED, never overwritten: when one
    worker of a cycle failed and its slice is rerun on its own
    (`scout --cycle cNN --slice "..."`), the workers that did finish have
    already been recorded, and this second write must add to them. Candidates
    are keyed by (image_url, page_url), blocked URLs by url, and the row's
    codes/scouted/tokens cells are combined with the values already there."""
    cycle = results[0].cycle
    cycle_dir = repo_root / SCOUTING_DIR / cycle
    cand_path = cycle_dir / f"candidates-{cycle}.jsonl"
    prior_candidates = load_jsonl(cand_path)
    all_candidates, added = merge_worker_candidates(prior_candidates, [c for r in results for c in r.candidates])
    with open(cand_path, "w", encoding="utf-8") as f:
        for c in all_candidates:
            f.write(json.dumps(c, ensure_ascii=False) + "\n")

    blocked_path = cycle_dir / f"blocked-{cycle}.jsonl"
    prior_blocked = load_jsonl(blocked_path)
    seen_blocked = {b.get("url") for b in prior_blocked}
    blocked_all = list(prior_blocked)
    for b in (b for r in results for b in r.blocked):
        if b.get("url") in seen_blocked:
            continue
        seen_blocked.add(b.get("url"))
        blocked_all.append(b)
    if blocked_all:
        with open(blocked_path, "w", encoding="utf-8") as f:
            for b in blocked_all:
                f.write(json.dumps(b, ensure_ascii=False) + chr(10))

    codes = " ".join(c for r in results for c in r.codes)
    wall = max(r.wall_s for r in results)
    tokens_sum: dict[str, int] | None = None
    for r in results:
        if r.tokens:
            tokens_sum = tokens_sum or {"uncached": 0, "cache_read": 0, "output": 0}
            for k in tokens_sum:
                tokens_sum[k] += r.tokens[k]
    faults = sum(len(r.faults) for r in results)
    tokens_cell = format_tokens(tokens_sum)
    row = cycle_row(cycle, codes, format_wall(wall), tokens_cell, len(all_candidates), faults)

    none_found_all = [c for r in results for c, v in r.per_code.items() if v.startswith("none found")]

    def apply(text: str) -> str:
        prior_cells = read_cycle_cells(text, cycle)
        if prior_cells is None:
            text = insert_cycle_row(text, row)
        else:
            # A rerun of one slice: add to the row the first run left behind.
            text = update_cycle_cells(
                text,
                cycle,
                {
                    "codes": merge_codes_cell(prior_cells.get("codes", ""), codes),
                    "scouted": str(_int_or_zero(prior_cells.get("scouted")) + added),
                    "tokens": merge_token_cells(prior_cells.get("tokens", ""), tokens_cell),
                },
            )
        if none_found_all:
            text = add_codes_line(text, f"tried ({cycle}), none found: {' '.join(none_found_all)}")
        for r in results:
            none_found = [c for c, v in r.per_code.items() if v.startswith("none found")]
            found = {c: v for c, v in r.per_code.items() if not v.startswith("none found")}
            line = (
                f"{cycle} scout {r.worker} done ({' '.join(r.codes)}): {format_wall(r.wall_s)}, "
                f"{len(r.candidates)} candidate(s) {json.dumps(found) if found else ''}; "
                f"none found: {' '.join(none_found) or '—'}; tokens {format_tokens(r.tokens)}; "
                f"faults {len(r.faults)}{' -- ' + '; '.join(r.faults) if r.faults else ''}"
                f"{'; blocked ' + str(len(r.blocked)) + ' (see blocked-' + cycle + '.jsonl, retry session-side)' if r.blocked else ''}"
            )
            text = append_log(text, line)
        return text

    edit_state(repo_root, apply)
    if prior_candidates:
        print(f"merged {added} new candidate(s) into {cand_path.name} ({len(all_candidates)} total); STATE.md row updated")
    else:
        print(f"wrote {cand_path} ({len(all_candidates)} candidate(s)); STATE.md updated")
    if blocked_all:
        print(f"{len(blocked_all)} blocked URL(s) -> {blocked_path.name}; retry them session-side (Firecrawl), never from repository code")
    return cand_path


def step_scout(args, repo_root: Path) -> tuple[list[WorkerResult], list[str]]:
    """Runs one worker per `--slice` and records whatever finished.

    Returns `(results, failed_workers)`. Two cycle-c14 lessons are built in:
    every worktree is created (and removed) sequentially in this thread, since
    parallel creation raced on `.git/config` and killed w1; and a worker that
    raises is turned into an empty `WorkerResult` with a fault line instead of
    propagating, so `record_scout` still runs and the workers that did finish
    keep their candidates. The caller reports the failure and exits non-zero."""
    slices = [s.split() for s in args.slice]
    if not slices:
        raise RuntimeError("at least one --slice \"CODE CODE ...\" is required")
    all_codes = [c.upper() for s in slices for c in s]
    if len(set(all_codes)) != len(all_codes):
        raise RuntimeError("a code appears in more than one slice")
    tasks = []
    for i, codes in enumerate(slices, 1):
        worker = f"w{i}"
        task_args = argparse.Namespace(cycle=args.cycle, worker=worker, codes=codes)
        tasks.append((worker, [c.upper() for c in codes], step_task(task_args, repo_root)))
    if args.dry_run:
        print("[dry-run] task files written; no worker launched")
        return [], []

    run_cmd(["git", "-C", str(repo_root), "fetch", "origin", "--quiet"])
    prepared: list[tuple[str, list[str], Path, str, Path]] = []
    for worker, codes, path in tasks:
        name, worktree = worker_worktree(repo_root, args.cycle, worker)
        add_scout_worktree(repo_root, name, worktree)
        prepared.append((worker, codes, path, name, worktree))

    results: list[WorkerResult] = []
    failed: list[str] = []
    try:
        with ThreadPoolExecutor(max_workers=len(prepared)) as pool:
            futures = [
                (worker, codes, pool.submit(run_worker, repo_root, args.cycle, worker, path, codes, args.route, worktree))
                for worker, codes, path, _name, worktree in prepared
            ]
            for worker, codes, fut in futures:
                try:
                    results.append(fut.result())
                except Exception as e:  # noqa: BLE001 -- one worker's failure must not cost the others theirs
                    print(f"[{worker}] FAILED: {type(e).__name__}: {e}", file=sys.stderr)
                    failure = WorkerResult(args.cycle, worker, codes)
                    failure.faults.append(f"worker failed before reporting: {type(e).__name__}: {e}")
                    results.append(failure)
                    failed.append(worker)
    finally:
        if not args.keep_worktree:
            for _worker, _codes, _path, name, worktree in prepared:
                remove_scout_worktree(repo_root, name, worktree)

    record_scout(repo_root, results)
    if failed:
        print(f"\n{len(failed)} worker(s) failed: {' '.join(failed)}", file=sys.stderr)
        print("Everything the other workers produced is recorded. Rerun just the failed slice(s):", file=sys.stderr)
        for worker, codes, _path, _name, _worktree in prepared:
            if worker in failed:
                print(f'  python tools/scout_cycle.py scout --cycle {args.cycle} --slice "{" ".join(codes)}"', file=sys.stderr)
        print("(a rerun merges into candidates-cNN.jsonl and the Cycles row, it does not overwrite them)", file=sys.stderr)
    return results, failed


def merge_candidates(existing: list[dict], incoming: list[dict], cycle: str, worker: str) -> tuple[list[dict], int]:
    """Appends session-found candidates (a Firecrawl retry of a BLOCKED page,
    a hand-found URL) to a cycle's candidate list, tagged like a worker's,
    skipping any (image_url, page_url) pair already present. Returns the
    merged list and how many were added. The candidates still go through
    screen_candidates.py like everything else (H2)."""
    seen = {(c.get("image_url"), c.get("page_url")) for c in existing}
    merged = list(existing)
    added = 0
    for c in incoming:
        if not (c.get("code") and c.get("image_url") and c.get("page_url")):
            continue
        key = (c.get("image_url"), c.get("page_url"))
        if key in seen:
            continue
        seen.add(key)
        merged.extend(tag_candidates([c], cycle, worker))
        added += 1
    return merged, added


def step_add_candidates(args, repo_root: Path) -> int:
    cycle_dir = repo_root / SCOUTING_DIR / args.cycle
    cycle_dir.mkdir(parents=True, exist_ok=True)
    cand_path = cycle_dir / f"candidates-{args.cycle}.jsonl"
    incoming = load_jsonl(Path(args.source))
    merged, added = merge_candidates(load_jsonl(cand_path), incoming, args.cycle, args.worker)
    with open(cand_path, "w", encoding="utf-8") as f:
        for c in merged:
            f.write(json.dumps(c, ensure_ascii=False) + chr(10))
    edit_state(
        repo_root,
        lambda t: append_log(t, f"{args.cycle} add-candidates: {added} of {len(incoming)} row(s) from {Path(args.source).name} appended as worker {args.worker}; run screen next"),
    )
    print(f"{added} candidate(s) added to {cand_path.name} ({len(merged)} total); next: python tools/scout_cycle.py screen --cycle {args.cycle}")
    return 0


def step_blocked(args, repo_root: Path) -> int:
    """Prints `blocked-cNN.jsonl` grouped by cause, or (with --to-fetch) just
    the page URLs, one per line, for the session to fetch and drop into
    work/scouting/cNN/pages/. The retry is the session's, never this tool's
    (H14) -- nothing here touches the network."""
    cycle = args.cycle
    blocked_path = repo_root / SCOUTING_DIR / cycle / f"blocked-{cycle}.jsonl"
    rows = load_jsonl(blocked_path)
    if not rows:
        print(f"{cycle}: no blocked URLs recorded ({blocked_path.name} is absent or empty)")
        return 0
    if args.to_fetch:
        seen: set[str] = set()
        for row in rows:
            url = row.get("url")
            if url and url not in seen:
                seen.add(url)
                print(url)
        print(
            f"{len(seen)} URL(s). Fetch each one session-side, save it under "
            f"work/scouting/{cycle}/pages/ named by screen_candidates.py's page_html_filename(page_url), "
            "list them in that directory's pages.jsonl, then: "
            f"python tools/scout_cycle.py screen --cycle {cycle}",
            file=sys.stderr,
        )
        return 0
    groups = group_blocked(rows)
    print(f"{cycle}: {len(rows)} blocked URL(s) in {len(groups)} group(s)")
    for bucket, items in groups.items():
        print()
        print(f"{bucket} ({len(items)}):")
        for row in items:
            code = row.get("code") or "?"
            worker = row.get("worker") or "?"
            print(f"  [{code} {worker}] {row.get('url')}")
            print(f"      {row.get('reason', '')}")
    print()
    print(
        "A 403/503/anti-bot page is worth a session-side refetch (H14): "
        f"python tools/scout_cycle.py blocked --cycle {cycle} --to-fetch"
    )
    return 0


def step_screen(args, repo_root: Path) -> int:
    cycle = args.cycle
    cycle_dir = repo_root / SCOUTING_DIR / cycle
    candidates = cycle_dir / f"candidates-{cycle}.jsonl"
    if not candidates.is_file():
        raise RuntimeError(f"missing {candidates}; run `scout` first")
    n_candidates = len(load_jsonl(candidates))
    if n_candidates == 0:
        print(f"{cycle}: 0 candidates -- nothing to screen")
        edit_state(repo_root, lambda t: append_log(update_cycle_cells(t, cycle, {"screened": "—", "packet": "—"}), f"{cycle} screen: 0 candidates, nothing to screen"))
        return 0
    tools_dir = Path(__file__).resolve().parent
    bin_name = "check_sample.exe" if sys.platform == "win32" else "check_sample"
    check_sample = args.check_sample_bin or (repo_root / "target" / "release" / "examples" / bin_name)
    if not Path(check_sample).is_file():
        raise RuntimeError(
            f"check_sample binary not found at {check_sample}; build it in the main checkout:\n"
            "  cargo build -p synthpass-ocr --release --example check_sample"
        )
    rel = SCOUTING_DIR / cycle
    cmd = [
        sys.executable,
        str(tools_dir / "screen_candidates.py"),
        "--candidates", str(rel / f"candidates-{cycle}.jsonl"),
        "--staging", str(rel / "staging"),
        "--ledger", str(LEDGER_REL),
        "--corpus", str(CORPUS_REL),
        "--out", str(rel / f"screened-{cycle}.jsonl"),
        "--packet", str(rel / f"packet-{cycle}.md"),
        "--check-sample-bin", str(check_sample),
    ]
    # Session-fetched page HTML: explicit --page-html-dir wins, otherwise the
    # cycle's own pages/ directory when the session has put one there. The
    # session does that retry, never this tool (H14).
    explicit_page_html = getattr(args, "page_html_dir", None)
    page_html_dir = Path(explicit_page_html) if explicit_page_html else rel / "pages"
    page_html_abs = page_html_dir if page_html_dir.is_absolute() else repo_root / page_html_dir
    if page_html_abs.is_dir():
        cmd += ["--page-html-dir", str(page_html_dir)]
        print(f"using session-fetched pages from {page_html_dir}")
    elif explicit_page_html:
        raise RuntimeError(f"--page-html-dir {page_html_dir} is not a directory")
    print("$ " + " ".join(cmd))
    proc = subprocess.run(cmd, cwd=repo_root, text=True, capture_output=True, encoding="utf-8", errors="replace")
    print(proc.stdout)
    if proc.stderr:
        print(proc.stderr, file=sys.stderr)
    if proc.returncode != 0:
        raise RuntimeError(f"screen_candidates.py exited {proc.returncode}")

    screened = load_jsonl(cycle_dir / f"screened-{cycle}.jsonl")
    rows = packet_rows_from_screened(screened)
    packet_json = cycle_dir / f"packet-{cycle}.json"
    packet_json.write_text(json.dumps(rows, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    rejects = {}
    for rec in screened:
        if rec.get("auto_reject"):
            rejects[rec["auto_reject"]] = rejects.get(rec["auto_reject"], 0) + 1
    needs_eyes = sum(1 for r in screened if r.get("auto_reject") is None and r.get("needs_eyes"))

    def apply(text: str) -> str:
        text = update_cycle_cells(text, cycle, {"screened": str(len(rows)), "packet": str(len(rows))})
        return append_log(
            text,
            f"{cycle} screen: {n_candidates} candidate(s), {len(rows)} survived, auto-rejects {json.dumps(rejects) if rejects else 'none'}; "
            f"packet-{cycle}.md/.json written, proposal columns blank; needs_eyes {needs_eyes}",
        )

    edit_state(repo_root, apply)
    print(f"{len(rows)} survivor(s) -> {packet_json.name}; auto-rejects: {rejects or 'none'}")
    print(
        "\nNEXT (not automated, by design): the judgement pass -- P-SCREEN step 2.\n"
        f"  Open every image under {rel / 'staging'} (H5), check each host is official, fill\n"
        f"  proposed_destination / proposed_licence in packet-{cycle}.json and packet-{cycle}.md,\n"
        "  then: python tools/scout_cycle.py review --cycle " + cycle
    )
    return 0


def step_review(args, repo_root: Path) -> int:
    cycle = args.cycle
    cycle_dir = repo_root / SCOUTING_DIR / cycle
    packet_json = cycle_dir / f"packet-{cycle}.json"
    if not packet_json.is_file():
        raise RuntimeError(f"missing {packet_json}; run `screen` first")
    rows = json.loads(packet_json.read_text(encoding="utf-8"))
    blank = [r["id"] for r in rows if not r.get("proposed_destination")]
    if blank and not args.allow_blank_proposals:
        raise RuntimeError(
            f"{len(blank)} row(s) have no proposed_destination yet ({', '.join(blank)}); finish the judgement "
            "pass first, or pass --allow-blank-proposals"
        )
    tools_dir = Path(__file__).resolve().parent
    out = cycle_dir / f"review-{cycle}.html"
    cmd = [
        sys.executable,
        str(tools_dir / "build_review_artifact.py"),
        "--packet", str(packet_json),
        "--out", str(out),
        "--repo-root", str(repo_root),
        "--title", f"Specimen packet {cycle}",
    ]
    prior = cycle_dir / f"verdicts-{cycle}.json"
    if prior.is_file():
        cmd += ["--prior-verdicts", str(prior)]
    print("$ " + " ".join(cmd))
    print(run_cmd(cmd, cwd=repo_root))
    edit_state(repo_root, lambda t: append_log(t, f"{cycle} review-{cycle}.html built ({len(rows)} rows); WIP packet {cycle} awaiting verdict"))
    minutes = max(2, 2 * len(rows))
    print(f"Packet {cycle}: {len(rows)} row(s), roughly {minutes} min of review. Open {out} and save verdicts-{cycle}.json next to it.")
    return 0


# ==========================================================================
# CLI
# ==========================================================================


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--repo-root", type=Path, default=None, help="the main checkout (default: this file's repo)")
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("suggest", help="list uncovered codes not yet tried, by region")
    p.add_argument("--n", type=int, default=6, help="codes shown per region (0 = all)")
    p.add_argument("--region", default=None, help="substring of a countries.rs region comment, e.g. Asia")
    p.add_argument("--exclude", nargs="*", default=[])
    p.add_argument("--include-orgs", action="store_true", help="include the deprioritised org/territory codes")
    p.add_argument("--plan", default=None, help="WORKERSxCODES, e.g. 3x6: print ready --slice arguments in priority order")
    p.add_argument("--cycle", default=None, help="with --plan: the cycle id to put in the printed run command")

    p = sub.add_parser("task", help="write the task file for one worker")
    p.add_argument("--cycle", required=True)
    p.add_argument("--worker", default="w1")
    p.add_argument("--codes", nargs="+", required=True)

    for name in ("scout", "run"):
        p = sub.add_parser(name, help="run the scout worker(s)" if name == "scout" else "task -> scout -> screen")
        p.add_argument("--cycle", required=True)
        p.add_argument("--slice", action="append", default=[], help='one worker\'s codes, e.g. "THA MAC TWN"; repeat for parallel workers')
        p.add_argument("--route", default=None, help="dsh route overlay name under $DSH_HOME/routes (default: deepseek-flash)")
        p.add_argument("--keep-worktree", action="store_true")
        p.add_argument("--dry-run", action="store_true", help="write task files only")
        p.add_argument("--check-sample-bin", default=None)
        p.add_argument("--page-html-dir", default=None, help="see `screen --page-html-dir`")

    p = sub.add_parser("add-candidates", help="append session-found candidates (e.g. a Firecrawl retry of BLOCKED pages) to a cycle")
    p.add_argument("--cycle", required=True)
    p.add_argument("--from", dest="source", required=True, help="JSONL of candidate records in the worker's shape")
    p.add_argument("--worker", default="session", help="tag for the rows, default 'session'")

    p = sub.add_parser("blocked", help="list the cycle's BLOCKED URLs grouped by reason, for a session-side retry")
    p.add_argument("--cycle", required=True)
    p.add_argument("--to-fetch", action="store_true", help="print just the page URLs, one per line")

    p = sub.add_parser("screen", help="screen the cycle's candidates and write the packet JSON twin")
    p.add_argument("--cycle", required=True)
    p.add_argument("--check-sample-bin", default=None)
    p.add_argument(
        "--page-html-dir",
        default=None,
        help="session-fetched page HTML (default: work/scouting/cNN/pages/ when it exists)",
    )

    p = sub.add_parser("review", help="build the HTML review artifact")
    p.add_argument("--cycle", required=True)
    p.add_argument("--allow-blank-proposals", action="store_true")

    args = parser.parse_args(argv)
    repo_root = (args.repo_root or find_repo_root()).resolve()
    if not (repo_root / COUNTRIES_REL).is_file():
        print(f"{repo_root} does not look like the SynthPass checkout", file=sys.stderr)
        return 2
    try:
        if args.command == "suggest":
            return step_suggest(args, repo_root)
        if args.command == "task":
            step_task(args, repo_root)
            return 0
        if args.command == "scout":
            _results, failed = step_scout(args, repo_root)
            return 1 if failed else 0
        if args.command == "add-candidates":
            return step_add_candidates(args, repo_root)
        if args.command == "blocked":
            return step_blocked(args, repo_root)
        if args.command == "screen":
            return step_screen(args, repo_root)
        if args.command == "review":
            return step_review(args, repo_root)
        if args.command == "run":
            results, failed = step_scout(args, repo_root)
            if args.dry_run:
                return 0
            if not any(r.candidates for r in results):
                print(f"{args.cycle}: every worker returned 0 candidates; nothing to screen")
                edit_state(repo_root, lambda t: update_cycle_cells(t, args.cycle, {"screened": "—", "packet": "—"}))
                return 1 if failed else 0
            # A failed worker never blocks screening what the others found; the
            # non-zero exit is what says a slice still needs a rerun.
            screen_status = step_screen(args, repo_root)
            return 1 if failed else screen_status
    except RuntimeError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
