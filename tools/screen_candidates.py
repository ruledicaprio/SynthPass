#!/usr/bin/env python3
"""
screen_candidates.py

Mechanically re-derives an agent-found specimen candidate before any human
review: fetches the page itself (never trusting the worker's copy), downloads
the image itself, hashes it, and runs the existing OCR sanity checker
(`check_sample`). This is the "Automated screen" step in
`knowledge/SPECIMEN_SOURCES.md`'s "Review gates for agent-found candidates".

This tool never decides public/local/drop, never decides a licence class, and
never writes a holder value (a name, a document number, a date of birth) to
any output file. Those decisions -- and everything a human needs to make
them -- belong to the "proposed destination" / "proposed licence" / "Verdict"
columns of the Markdown packet this tool produces, which it always leaves
blank.

Standard library only, except PyMuPDF (`pip install pymupdf`) for the PDF
lane, imported lazily so everything else runs without it. See
tools/fetch_commons_mrz_specimens_v2.py for the
sibling acquisition tool's User-Agent / retry-and-backoff conventions; this
tool follows the same shape with urllib instead of requests.

Usage:
    python tools/screen_candidates.py --candidates <path.jsonl> --staging <dir> \\
        --ledger <ledger.jsonl> --corpus samples/corpus.jsonl \\
        --out <screened.jsonl> --packet <packet.md>

Pipeline per candidate, cheapest check first. The first mechanical reject
stops that candidate: a ledger row is appended and any staged file for it is
deleted.

    1. denylist            host is on the "never a source" list, or the screener-grown
                            host_denylist.json list (reason "denylisted-host")
    2. ledger duplicate    image_url/page_url already rejected or admitted
    3. fetch page_url      one retry, 15s timeout, polite per-host delay --
                            unless --page-html-dir holds a session-fetched copy
                            of that page, which is used instead (see below)
    4. linked-from-page    image_url must actually appear on the fetched page
    5. evidence snippets   licence/specimen words, for human review only
    6. download image_url  same fetch discipline as the page; a PDF (magic
                            bytes) takes the PDF lane: pages with specimen
                            words, embedded images extracted at their original
                            bytes (PyMuPDF, optional import), each then
                            screened as its own candidate with `#page=N`
    7. byte duplicate      sha256 against --corpus and --ledger
    8. check_sample        parses its stdout contract; missing VENDOR fails
                            closed to "vendor", same as VENDOR BLOCKED
    9. variant hint         in-memory only; the matched document number is
                            never written to any output
   10. needs_eyes           missing/conflicting signal, for the human packet

Session-fetched pages (`--page-html-dir`)
-----------------------------------------
Many official hosts answer this tool's plain urllib fetch with 403/503, an
anti-bot 202, or a JavaScript-only shell, so step 3 fails and a real candidate
is never screened at all. The maintainer's own session can fetch such a page by
other means and drop the HTML into a directory; this tool then uses that copy
instead of fetching the page itself. The retry is always the session's, never
this tool's: no repository code calls a scraping service or a search API (H14,
`knowledge/SPECIMEN_SOURCES.md`'s "The tooling boundary").

    <dir>/pages.jsonl   one record per page:
                        {"page_url": ..., "file": "<relative filename>",
                         "fetched_at": "<ISO>", "fetched_by": "session",
                         "note": "..."}
    <dir>/<file>        the HTML itself, named `page_html_filename(page_url)`
                        (sha256 of the URL, first 12 hex chars, + ".html") so
                        the session and this tool agree without a lookup.

A screened record whose page came from such a copy carries
`page_source: "session-fetched"` and `page_fetched_at`; the normal path sets
`page_source: "tool"`. The packet's Note column says "page: session-fetched" so
the screener knows the page was not re-derived by the tool.

Out of scope, deliberately: the IMAGE is still fetched by this tool (step 6),
because that is the byte stream everything downstream hashes and OCRs. A
candidate whose image fetch is also blocked stays blocked -- it is not a
candidate this directory can rescue.
"""

from __future__ import annotations

import argparse
import hashlib
import html.parser
import json
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request
from datetime import date, datetime, timezone
from urllib.parse import unquote, urljoin, urlparse

try:
    import fitz  # PyMuPDF -- only the PDF lane uses it; see "PDF lane" below
except ImportError:  # pragma: no cover
    fitz = None

USER_AGENT = (
    "SynthPass-Specimen-Screener/1.0 "
    "(https://github.com/ruledicaprio/SynthPass; rusmirskopljak@gmail.com)"
)

FETCH_TIMEOUT = 15  # seconds
FETCH_RETRIES = 1  # one retry beyond the first attempt = 2 tries total
HOST_DELAY = 1.0  # polite delay between requests to the same host, seconds

CHECK_SAMPLE_TIMEOUT = 120  # seconds; OCR model load + inference

# Domains knowledge/SPECIMEN_SOURCES.md's "Never a source" section forbids
# outright. PRADO's own copyright notice prohibits harvesting or
# redistributing its material outside official, non-commercial use -- it is
# consulted only as a manual human reference, never scraped, never stored,
# not even locally.
DENYLISTED_HOSTS = {
    "consilium.europa.eu",  # PRADO
    "www.consilium.europa.eu",
}

# The screener-grown counterpart to DENYLISTED_HOSTS above: hosts that are not
# an issuing authority and have surfaced non-document images in a screening
# cycle, kept in their own file (tools/host_denylist.json, parallel in shape
# to tools/legal_portals.json) rather than hardcoded here, since this list
# grows from screener reports rather than SPECIMEN_SOURCES.md's fixed "Never
# a source" set. Loaded once at import time; missing file means no host is
# denylisted this way, same "absence is not an error" convention
# load_legal_portals uses in tools/scout_cycle.py.
HOST_DENYLIST_FILENAME = "host_denylist.json"


def load_host_denylist(path) -> dict:
    """host (lowercase) -> reason, from `tools/host_denylist.json`. Keys
    starting with `_` are documentation, not hosts. Missing file: no
    denylisted hosts, never an error -- mirrors `load_legal_portals` in
    `tools/scout_cycle.py`."""
    if not os.path.isfile(path):
        return {}
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    return {k.lower(): v for k, v in data.items() if not k.startswith("_")}


def is_host_denylisted(host: str, denylist: dict) -> str | None:
    """Returns the reason string when `host` (or a parent domain of it)
    matches an entry in `denylist`, else `None`. Same exact-or-subdomain
    matching shape as `is_denylisted` above."""
    host = (host or "").lower()
    if not host or not denylist:
        return None
    if host in denylist:
        return denylist[host]
    for domain, reason in denylist.items():
        if host.endswith("." + domain):
            return reason
    return None


# --------------------------------------------------------------------------
# Session-fetched page HTML (--page-html-dir); see the module docstring
# --------------------------------------------------------------------------

PAGES_INDEX_FILENAME = "pages.jsonl"


def page_html_filename(page_url: str) -> str:
    """The filename a session-fetched copy of `page_url` is stored under:
    the first 12 hex chars of the URL's sha256, plus `.html`. A single
    function both sides call, so the session naming a file and this tool
    looking one up can never disagree -- and so a URL with a query string,
    percent-escapes or a fragment needs no escaping rules of its own."""
    digest = hashlib.sha256(page_url.encode("utf-8")).hexdigest()
    return f"{digest[:12]}.html"


def load_page_html_dir(dir_path: str | None) -> dict[str, dict]:
    """`page_url -> record` from `<dir>/pages.jsonl`, each record's `path`
    resolved to the HTML file next to the index.

    A missing directory or index means "no session-fetched pages" and is
    never an error, the same convention `load_host_denylist` uses. A record
    that names a file which is not there IS an error and raises: the session
    said it saved that page, and silently falling back to this tool's own
    fetch would re-block the very candidate the copy exists to unblock."""
    if not dir_path:
        return {}
    index_path = os.path.join(dir_path, PAGES_INDEX_FILENAME)
    if not os.path.isfile(index_path):
        return {}
    pages: dict[str, dict] = {}
    for record in load_jsonl(index_path):
        page_url = record.get("page_url")
        if not page_url:
            continue
        filename = record.get("file") or page_html_filename(page_url)
        path = os.path.join(dir_path, filename)
        if not os.path.isfile(path):
            raise FileNotFoundError(
                f"{index_path}: record for {page_url} names {filename!r}, which is not in {dir_path}"
            )
        entry = dict(record)
        entry["path"] = path
        pages[page_url] = entry
    return pages


def read_page_html(record: dict) -> str:
    """The HTML of a session-fetched page, decoded the way a fetched body is
    (UTF-8, latin-1 replacement fallback), so the link check sees exactly the
    text it would have seen from this tool's own fetch."""
    with open(record["path"], "rb") as f:
        body = f.read()
    try:
        return body.decode("utf-8")
    except UnicodeDecodeError:
        return body.decode("latin-1", errors="replace")


LICENCE_WORDS = ["licen", "copyright", "\u00a9", "reuse", "creative commons", "public domain"]

SPECIMEN_WORDS = [
    "specimen",
    "sample",
    "muster",
    "sp\u00e9cimen",
    "esp\u00e9cimen",
    "\u043e\u0431\u0440\u0430\u0437\u0435\u0446",
    "uzorak",
    "paraugs",
    "pavyzdys",
    "n\u00e4idis",
    "wz\u00f3r",
    "vzor",
    "minta",
]


# --------------------------------------------------------------------------
# Denylist
# --------------------------------------------------------------------------


def is_denylisted(host: str) -> bool:
    host = (host or "").lower()
    if not host:
        return False
    if host in DENYLISTED_HOSTS:
        return True
    return any(host.endswith("." + d) for d in DENYLISTED_HOSTS)


# --------------------------------------------------------------------------
# HTML parsing: link collection and plain-text extraction
# --------------------------------------------------------------------------


class _LinkCollector(html.parser.HTMLParser):
    """Collects every src/href-style URL referenced by a page's markup."""

    ATTR_NAMES = {"src", "href", "srcset", "data-src", "data-href"}

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.links: list[str] = []

    def handle_starttag(self, tag, attrs):
        for name, value in attrs:
            if value is None or name.lower() not in self.ATTR_NAMES:
                continue
            # srcset holds comma-separated "url descriptor" pairs.
            for piece in value.split(","):
                url = piece.strip().split(" ")[0].strip()
                if url:
                    self.links.append(url)


def _filename_of(url: str) -> str:
    return unquote(url.split("?")[0].rstrip("/").split("/")[-1])


def image_linked_from_page(page_url: str, page_html: str, image_url: str) -> bool:
    """True if `image_url` (absolute or by decoded filename) is among the
    resources the fetched page actually links to."""
    collector = _LinkCollector()
    try:
        collector.feed(page_html)
    except Exception:
        return False

    image_decoded = unquote(image_url)
    image_filename = _filename_of(image_url)

    for link in collector.links:
        absolute = urljoin(page_url, link)
        if absolute == image_url or unquote(absolute) == image_decoded:
            return True
        if image_filename and _filename_of(absolute) == image_filename:
            return True
    return False


class _TextExtractor(html.parser.HTMLParser):
    """Strips tags, dropping script/style contents, for evidence-snippet scanning."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.chunks: list[str] = []
        self._skip_depth = 0

    def handle_starttag(self, tag, attrs):
        if tag.lower() in ("script", "style"):
            self._skip_depth += 1

    def handle_endtag(self, tag):
        if tag.lower() in ("script", "style") and self._skip_depth > 0:
            self._skip_depth -= 1

    def handle_data(self, data):
        if self._skip_depth == 0:
            self.chunks.append(data)


def strip_tags(page_html: str) -> str:
    extractor = _TextExtractor()
    try:
        extractor.feed(page_html)
    except Exception:
        return ""
    return " ".join(extractor.chunks)


def find_snippets(text: str, words: list[str], limit: int = 3, context: int = 80, max_len: int = 200) -> list[str]:
    """At most `limit` short, deduplicated snippets around a case-insensitive
    match of any of `words`. Human-review evidence only -- never a verdict."""
    snippets: list[str] = []
    lower = text.lower()
    for word in words:
        word_lower = word.lower()
        idx = lower.find(word_lower)
        while idx != -1 and len(snippets) < limit:
            start = max(0, idx - context)
            end = min(len(text), idx + len(word) + context)
            snippet = re.sub(r"\s+", " ", text[start:end]).strip()
            if len(snippet) > max_len:
                snippet = snippet[:max_len].rstrip() + "\u2026"
            if snippet and snippet not in snippets:
                snippets.append(snippet)
            idx = lower.find(word_lower, idx + len(word_lower))
        if len(snippets) >= limit:
            break
    return snippets[:limit]


# --------------------------------------------------------------------------
# Fetching
# --------------------------------------------------------------------------

_last_request_time: dict[str, float] = {}


def _polite_wait(host: str) -> None:
    now = time.monotonic()
    last = _last_request_time.get(host)
    if last is not None:
        elapsed = now - last
        if elapsed < HOST_DELAY:
            time.sleep(HOST_DELAY - elapsed)
    _last_request_time[host] = time.monotonic()


def fetch_url(url: str, opener=None):
    """Fetch `url` with one retry and a polite per-host delay.

    Returns (final_url, status_code, body_bytes) on success, or None after
    retries are exhausted. `opener` is injectable for offline tests --
    defaults to `urllib.request.urlopen`.
    """
    host = urlparse(url).netloc
    opener_fn = opener or urllib.request.urlopen
    last_error = None
    for _attempt in range(FETCH_RETRIES + 1):
        _polite_wait(host)
        request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        try:
            with opener_fn(request, timeout=FETCH_TIMEOUT) as resp:
                body = resp.read()
                final_url = resp.geturl() if hasattr(resp, "geturl") else url
                status = getattr(resp, "status", None)
                if status is None and hasattr(resp, "getcode"):
                    status = resp.getcode()
                return final_url, status, body
        except urllib.error.HTTPError as e:
            # An HTTPError is itself a response object with a body/status.
            try:
                body = e.read()
            except Exception:
                body = b""
            return e.geturl() if hasattr(e, "geturl") else url, e.code, body
        except (urllib.error.URLError, TimeoutError, OSError) as e:
            last_error = e
            continue
    del last_error
    return None


# --------------------------------------------------------------------------
# Image magic-byte detection
# --------------------------------------------------------------------------


def detect_image_type(data: bytes) -> str | None:
    """Returns 'jpg' / 'png' / 'webp' / 'gif' from magic bytes, or None."""
    if data[:3] == b"\xff\xd8\xff":
        return "jpg"
    if data[:8] == b"\x89PNG\r\n\x1a\n":
        return "png"
    if data[:6] in (b"GIF87a", b"GIF89a"):
        return "gif"
    if data[:4] == b"RIFF" and data[8:12] == b"WEBP":
        return "webp"
    return None


# --------------------------------------------------------------------------
# JSONL helpers
# --------------------------------------------------------------------------


def load_jsonl(path: str) -> list[dict]:
    records = []
    if not path or not os.path.exists(path):
        return records
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            records.append(json.loads(line))
    return records


def append_jsonl(path: str, record: dict) -> None:
    with open(path, "a", encoding="utf-8") as f:
        f.write(json.dumps(record, ensure_ascii=False) + "\n")


# --------------------------------------------------------------------------
# Duplicate detection
# --------------------------------------------------------------------------


def candidate_url(candidate: dict) -> str | None:
    """The URL identity used for ledger comparisons: image_url when present,
    page_url otherwise -- consistently, in both directions."""
    return candidate.get("image_url") or candidate.get("page_url")


def ledger_url_duplicate(candidate: dict, ledger_records: list[dict]) -> bool:
    image_url = candidate.get("image_url")
    page_url = candidate.get("page_url")
    for row in ledger_records:
        url = row.get("url")
        if url and ((image_url and url == image_url) or (page_url and url == page_url)):
            return True
    return False


def sha_duplicate(sha256: str, corpus_records: list[dict], ledger_records: list[dict]) -> str | None:
    """Returns a human-readable match description, or None."""
    for row in corpus_records:
        if row.get("sha256") == sha256:
            return row.get("filename") or "corpus-entry"
    for row in ledger_records:
        if row.get("sha256") == sha256:
            return row.get("url") or "ledger-entry"
    return None


# --------------------------------------------------------------------------
# check_sample stdout parsing
# --------------------------------------------------------------------------


def parse_check_sample_output(text: str) -> dict:
    """Parses check_sample's stdout contract (crates/synthpass-ocr/examples/check_sample.rs).

    Returns a dict:
        mrz_status: 'hit' | 'miss' | 'error' | None
        doc_number: str | None   (present only for 'hit'; keep this in memory
                                   only -- never write it to an output file)
        watermark:  True | False | None
        vendor:     'clear' | 'blocked' | None

    `vendor` is None when no VENDOR line appears at all (e.g. the process
    crashed or exited before printing one, such as after OCR-ERROR). Callers
    must fail closed on that -- treat a missing VENDOR line exactly like
    VENDOR BLOCKED, never like VENDOR CLEAR.
    """
    result: dict = {"mrz_status": None, "doc_number": None, "watermark": None, "vendor": None}
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if line.startswith("OCR-ERROR"):
            result["mrz_status"] = "error"
        elif line.startswith("MRZ HIT"):
            result["mrz_status"] = "hit"
            m = re.search(r"doc_number=(\S+)", line)
            if m:
                result["doc_number"] = m.group(1)
        elif line.startswith("MRZ MISS"):
            result["mrz_status"] = "miss"
        elif line.startswith("WATERMARK"):
            result["watermark"] = "contains" in line.lower()
        elif line.startswith("VENDOR"):
            if "BLOCKED" in line:
                result["vendor"] = "blocked"
            elif "CLEAR" in line:
                result["vendor"] = "clear"
    return result


def run_check_sample(binary_path: str, image_path: str, timeout: int = CHECK_SAMPLE_TIMEOUT) -> dict:
    try:
        proc = subprocess.run(
            [binary_path, image_path],
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
        output = (proc.stdout or "") + "\n" + (proc.stderr or "")
    except (subprocess.TimeoutExpired, OSError) as e:
        # Fails closed: no VENDOR line was ever produced.
        return {"mrz_status": "error", "doc_number": None, "watermark": None, "vendor": None, "error": str(e)}
    return parse_check_sample_output(output)


def find_check_sample_binary(repo_root: str) -> str | None:
    for name in ("check_sample.exe", "check_sample"):
        candidate = os.path.join(repo_root, "target", "release", "examples", name)
        if os.path.isfile(candidate):
            return candidate
    return None


# --------------------------------------------------------------------------
# doc-claims-an-MRZ heuristic (for needs_eyes)
# --------------------------------------------------------------------------


def doc_claims_mrz(candidate: dict) -> bool:
    """Whether the candidate's own metadata implies this image carries an
    MRZ at all. A TD1 ID-card *front* image does not -- the MRZ is on the
    back -- which is exactly the case a worker flagged in review_note for
    the LVA proving candidates. Everything else is assumed to claim one."""
    doc = (candidate.get("doc") or "").lower()
    td = (candidate.get("td") or "").upper()
    if td == "TD1" and "front" in doc and "back" not in doc:
        return False
    return True


# --------------------------------------------------------------------------
# PDF lane (gazette annexes, regulations, guides)
# --------------------------------------------------------------------------
#
# The scout worker cannot open PDFs, so prefix v3 lets it report a PDF it saw
# linked from an opened page. Here the PDF is downloaded like any other
# candidate, its pages are scanned for the same specimen words the page text
# is scanned for, and the images on those pages are pulled out -- the embedded
# raster at its original bytes when there is one (a stable sha256, no
# resampling), a page render only when a page has none. Every extracted image
# then goes through exactly the checks a directly linked image gets. PyMuPDF
# is the only non-standard-library dependency in tools/ and only this lane
# imports it; the extraction path never does.

PDF_MAX_PAGES = 200  # pages scanned per PDF; PassV.pdf (the German passport regulation) has 148
PDF_MAX_IMAGES = 40  # extracted images per PDF, after the size filter; a booklet annex draws one per page
PDF_MIN_IMAGE_SIDE = 150  # px; drops logos, seals and icons
PDF_RASTER_DPI = 200
# A raster fallback page whose own text runs longer than this, and which draws
# no image at all, is a text leaflet, not a specimen plate: a Liberia PDF on
# cycle c14 put a full page of prose in front of the screener that way. The
# page text is already in hand from `get_text()`, so this costs no OCR.
PDF_TEXT_PAGE_MAX_WORDS = 120
PDF_SKIP_TEXT_PAGE = "pdf-text-page"


def detect_pdf(data: bytes) -> bool:
    return data[:5] == b"%PDF-"


def select_pdf_pages(page_texts: list[str], max_pages: int = PDF_MAX_PAGES) -> list[int]:
    """Zero-based indexes of the pages worth extracting from: those whose
    text mentions a specimen word. A PDF with no such page (a scanned
    gazette without a text layer, an image-only annex) falls back to every
    page, so a specimen is never missed for want of OCR -- the caps bound
    the cost."""
    texts = page_texts[:max_pages]
    hits = [i for i, t in enumerate(texts) if any(w in (t or "").lower() for w in SPECIMEN_WORDS)]
    return hits if hits else list(range(len(texts)))


def extract_pdf_images(pdf_bytes: bytes) -> tuple[list[dict], list[str], dict[str, int]]:
    """Returns ([{page, xref, kind, ext, width, height, data, page_text}],
    page_texts of the pages that mention a specimen word, {skip reason: count}).
    `page` is 1-based for humans and ledger URLs.

    Images are taken from what a page actually *draws* (`get_image_info`),
    not from its resource dictionary: a gazette PDF commonly shares one
    resource dictionary across every page, so `get_images` lists all 173
    images of a 148-page regulation on page 1 (seen on PassV.pdf,
    2026-09-15). Pages that mention a specimen word come first, then the
    rest in page order, because an annex's heading sits on its first page
    while its images run on for pages; an xref is extracted once. Only when
    the whole PDF yields no usable embedded image (a scanned gazette, vector
    artwork) are the specimen-word pages rendered instead -- rendering a
    table of contents because it says "Muster" is noise, not a specimen. A
    rendered page that draws no image at all and carries more than
    `PDF_TEXT_PAGE_MAX_WORDS` words of its own text is skipped as
    `pdf-text-page` and counted in the returned skip map: it is a leaflet
    page, and a full-page raster of prose costs the screener a packet row and
    an open (a Liberia PDF did exactly that on cycle c14).
    Requires PyMuPDF; the caller checks `fitz is not None` first."""
    doc = fitz.open(stream=pdf_bytes, filetype="pdf")
    try:
        page_count = min(doc.page_count, PDF_MAX_PAGES)
        texts = [doc[i].get_text() or "" for i in range(page_count)]
        has_words = [any(w in (t or "").lower() for w in SPECIMEN_WORDS) for t in texts]
        with_words = [i for i in range(page_count) if has_words[i]]
        order = with_words + [i for i in range(page_count) if not has_words[i]]
        specimen_texts = [texts[i] for i in with_words]
        images: list[dict] = []
        skipped: dict[str, int] = {}
        draws_an_image: dict[int, bool] = {}
        seen_xrefs: set[int] = set()
        for i in order:
            if len(images) >= PDF_MAX_IMAGES:
                break
            page = doc[i]
            try:
                drawn = page.get_image_info(xrefs=True)
            except Exception:  # noqa: BLE001
                drawn = []
            draws_an_image[i] = bool(drawn)
            for info in drawn:
                xref = info.get("xref")
                if not xref or xref in seen_xrefs or len(images) >= PDF_MAX_IMAGES:
                    continue
                seen_xrefs.add(xref)
                try:
                    extracted = doc.extract_image(xref)
                except Exception:  # noqa: BLE001 -- a broken stream is skipped, not fatal
                    continue
                width, height = extracted.get("width", 0), extracted.get("height", 0)
                if min(width, height) < PDF_MIN_IMAGE_SIDE:
                    continue
                ext = (extracted.get("ext") or "").lower()
                data = extracted.get("image") or b""
                if ext in ("jpeg", "jpg") and detect_image_type(data) == "jpg":
                    ext = "jpg"
                elif detect_image_type(data) is None:
                    # Uncommon encodings (JBIG2, CCITT, raw): go through a pixmap instead.
                    try:
                        pix = fitz.Pixmap(doc, xref)
                        if pix.n - pix.alpha >= 4:
                            pix = fitz.Pixmap(fitz.csRGB, pix)
                        data, ext = pix.tobytes("png"), "png"
                    except Exception:  # noqa: BLE001
                        continue
                images.append(
                    {"page": i + 1, "xref": xref, "kind": "embedded", "ext": ext, "width": width, "height": height, "data": data, "page_text": texts[i]}
                )
        if not images:
            for i in with_words or list(range(page_count)):
                if len(images) >= PDF_MAX_IMAGES:
                    break
                if not draws_an_image.get(i, False) and len((texts[i] or "").split()) > PDF_TEXT_PAGE_MAX_WORDS:
                    skipped[PDF_SKIP_TEXT_PAGE] = skipped.get(PDF_SKIP_TEXT_PAGE, 0) + 1
                    continue
                pix = doc[i].get_pixmap(dpi=PDF_RASTER_DPI)
                images.append(
                    {"page": i + 1, "xref": None, "kind": "raster", "ext": "png", "width": pix.width, "height": pix.height, "data": pix.tobytes("png"), "page_text": texts[i]}
                )
        return images, specimen_texts, skipped
    finally:
        doc.close()


def expand_pdf_candidate(
    parent: dict,
    staging_dir: str,
    ledger_records: list[dict],
    corpus_records: list[dict],
    binary_path: str,
    skipped_counter: dict[str, int] | None = None,
) -> list[dict]:
    """Turns a screened PDF parent (auto_reject None, `is_pdf` set) into one
    result per extracted image, each run through the byte-duplicate, staging,
    check_sample, variant and needs_eyes steps like a directly linked image.
    The child's `image_url` gains `#page=N` so the ledger and later origin
    rows point at the page, not just the file. A PDF with no usable image is
    returned as a single off-scope reject.

    `skipped_counter`, when given, is a caller-owned `{reason: count}` dict
    this function adds each PDF's skipped pages to (`pdf-text-page` today), so
    the run's summary can state them without this function returning a second
    value every caller would have to unpack."""
    pdf_bytes = parent.pop("_pdf_bytes", b"")
    parent.pop("is_pdf", None)
    pdf_url = parent.get("image_url_final") or parent.get("image_url") or ""
    pdf_sha = parent.get("sha256")

    # A byte-duplicate of a ledgered or corpus file needs no PDF parsing at
    # all, so it is decided before the PyMuPDF check: the verdict is the
    # same on a machine without pymupdf (Linux CI found this order inverted).
    dup = sha_duplicate(pdf_sha, corpus_records, ledger_records) if pdf_sha else None
    if dup:
        parent["auto_reject"] = "duplicate"
        parent["note"] = f"byte-duplicate-of:{dup}"
        return [parent]

    if fitz is None:
        parent["auto_reject"] = "unresolvable"
        parent["note"] = "pdf: PyMuPDF is not installed (pip install pymupdf); retry after installing"
        return [parent]

    try:
        images, page_texts, skipped = extract_pdf_images(pdf_bytes)
    except Exception as e:  # noqa: BLE001 -- a malformed PDF is a reject, not a crash
        parent["auto_reject"] = "off-scope"
        parent["note"] = f"pdf: could not be read ({type(e).__name__})"
        return [parent]

    if skipped_counter is not None:
        for reason, count in skipped.items():
            skipped_counter[reason] = skipped_counter.get(reason, 0) + count
    skipped_note = "".join(f" ({count} skipped: {reason})" for reason, count in sorted(skipped.items()))

    if not images:
        parent["auto_reject"] = "off-scope"
        parent["note"] = f"pdf: no usable image on its pages{skipped_note}"
        return [parent]

    pdf_specimen_snippets: list[str] = []
    pdf_licence_snippets: list[str] = []
    for text in page_texts:
        pdf_specimen_snippets += [f"pdf: {s}" for s in find_snippets(text, SPECIMEN_WORDS, limit=2)]
        pdf_licence_snippets += [f"pdf: {s}" for s in find_snippets(text, LICENCE_WORDS, limit=2)]
        if len(pdf_specimen_snippets) >= 3 and len(pdf_licence_snippets) >= 3:
            break

    children: list[dict] = []
    for img in images:
        child = dict(parent)
        child["image_url"] = f"{pdf_url}#page={img['page']}"
        child["pdf_source"] = {
            "url": pdf_url,
            "pdf_sha256": pdf_sha,
            "page": img["page"],
            "xref": img["xref"],
            "kind": img["kind"],
            "width": img["width"],
            "height": img["height"],
        }
        own_page = [f"pdf p.{img['page']}: {s}" for s in find_snippets(img.get("page_text") or "", SPECIMEN_WORDS, limit=2)]
        child["specimen_snippets"] = (parent.get("specimen_snippets") or []) + own_page + [s for s in pdf_specimen_snippets[:3] if s not in own_page]
        child["licence_snippets"] = (parent.get("licence_snippets") or []) + pdf_licence_snippets[:3]
        child_sha = hashlib.sha256(img["data"]).hexdigest()
        child["sha256"] = child_sha
        children.append(
            _screen_image_bytes(
                child, child, img["data"], child_sha, img["ext"], staging_dir, ledger_records, corpus_records, binary_path
            )
        )
    return children


# --------------------------------------------------------------------------
# Per-candidate pipeline
# --------------------------------------------------------------------------


def screen_candidate(
    candidate: dict,
    staging_dir: str,
    ledger_records: list[dict],
    corpus_records: list[dict],
    binary_path: str,
    fetch=None,
    host_denylist: dict | None = None,
    page_html: dict[str, dict] | None = None,
) -> dict:
    result = dict(candidate)
    result.update(
        {
            "auto_reject": None,
            "note": None,
            "sha256": None,
            "staged_path": None,
            "page_url_final": None,
            "page_status": None,
            "page_source": None,
            "page_fetched_at": None,
            "image_url_final": None,
            "image_status": None,
            "licence_snippets": [],
            "specimen_snippets": [],
            "mrz_check": "none",
            "needs_eyes": False,
            "variant_of": None,
        }
    )

    image_url = candidate.get("image_url")
    page_url = candidate.get("page_url")

    if not image_url or not page_url:
        result["auto_reject"] = "off-scope"
        result["note"] = "no image_url/page_url on this candidate row"
        return result

    # 1. denylist
    for u in (image_url, page_url):
        host = urlparse(u).netloc
        if is_denylisted(host):
            result["auto_reject"] = "denylisted"
            return result
        reason = is_host_denylisted(host, host_denylist)
        if reason:
            result["auto_reject"] = "denylisted-host"
            result["note"] = reason
            return result

    # 2. ledger duplicate
    if ledger_url_duplicate(candidate, ledger_records):
        result["auto_reject"] = "duplicate"
        result["note"] = "url already in ledger"
        return result

    fetch_fn = fetch or fetch_url

    # 3. fetch page_url -- unless the session already fetched it for us
    supplied = (page_html or {}).get(page_url)
    if supplied is not None:
        # The session's copy of a page this tool's plain fetch cannot get
        # (403/503, an anti-bot 202, a JavaScript-only shell). The image is
        # still fetched below by this tool; only the page is borrowed.
        page_text = read_page_html(supplied)
        final_page_url = page_url
        result["page_url_final"] = page_url
        result["page_source"] = "session-fetched"
        result["page_fetched_at"] = supplied.get("fetched_at")
    else:
        page_fetch = fetch_fn(page_url)
        if page_fetch is None:
            result["auto_reject"] = "unresolvable"
            result["note"] = "page fetch failed after retry"
            return result
        final_page_url, page_status, page_body = page_fetch
        result["page_url_final"] = final_page_url
        result["page_status"] = page_status
        result["page_source"] = "tool"

        try:
            page_text = page_body.decode("utf-8")
        except UnicodeDecodeError:
            page_text = page_body.decode("latin-1", errors="replace")

    # 4. linked-from-page
    if not image_linked_from_page(final_page_url or page_url, page_text, image_url):
        result["auto_reject"] = "unresolvable"
        result["note"] = "not-linked-from-page"
        return result

    # 5. evidence snippets (human review only)
    plain_text = strip_tags(page_text)
    result["licence_snippets"] = find_snippets(plain_text, LICENCE_WORDS)
    result["specimen_snippets"] = find_snippets(plain_text, SPECIMEN_WORDS)

    # 6. download image_url
    image_fetch = fetch_fn(image_url)
    if image_fetch is None:
        result["auto_reject"] = "unresolvable"
        result["note"] = "image fetch failed after retry"
        return result
    final_image_url, image_status, image_body = image_fetch
    result["image_url_final"] = final_image_url
    result["image_status"] = image_status

    sha256 = hashlib.sha256(image_body).hexdigest()
    result["sha256"] = sha256

    if detect_pdf(image_body):
        # PDF lane: main() expands this into one result per extracted image
        # (expand_pdf_candidate). Nothing is staged for the PDF itself.
        result["is_pdf"] = True
        result["_pdf_bytes"] = image_body
        return result

    image_type = detect_image_type(image_body)
    if image_type is None:
        result["auto_reject"] = "off-scope"
        result["note"] = "downloaded bytes are not a recognized image format"
        return result

    return _screen_image_bytes(
        result, candidate, image_body, sha256, image_type, staging_dir, ledger_records, corpus_records, binary_path
    )


def _screen_image_bytes(
    result: dict,
    candidate: dict,
    image_body: bytes,
    sha256: str,
    image_type: str,
    staging_dir: str,
    ledger_records: list[dict],
    corpus_records: list[dict],
    binary_path: str,
) -> dict:
    """Steps 7-10 on image bytes already in hand: byte duplicate, staging,
    check_sample, the variant hint and needs_eyes. Shared by directly linked
    images and by every image extracted from a PDF."""
    # 7. byte duplicate (checked before anything is written to staging)
    dup = sha_duplicate(sha256, corpus_records, ledger_records)
    if dup:
        result["auto_reject"] = "duplicate"
        result["note"] = f"byte-duplicate-of:{dup}"
        return result

    os.makedirs(staging_dir, exist_ok=True)
    staged_path = os.path.join(staging_dir, f"{sha256[:12]}.{image_type}")
    with open(staged_path, "wb") as f:
        f.write(image_body)
    result["staged_path"] = staged_path

    # 8. check_sample (fails closed on a missing VENDOR line)
    check = run_check_sample(binary_path, staged_path)
    if check.get("vendor") != "clear":
        result["auto_reject"] = "vendor"
        result["note"] = "VENDOR BLOCKED" if check.get("vendor") == "blocked" else "no VENDOR line in check_sample output"
        _delete_staged(staged_path)
        result["staged_path"] = None
        return result

    if check.get("mrz_status") == "hit":
        result["mrz_check"] = "valid"
    elif check.get("mrz_status") == "miss":
        result["mrz_check"] = "invalid"
    else:
        result["mrz_check"] = "none"

    # 9. variant hint -- in-memory comparison only, doc_number never leaves
    # this function.
    doc_number = check.get("doc_number")
    if doc_number:
        for row in corpus_records:
            row_mrz = row.get("mrz") or {}
            if row.get("expected_document_number") == doc_number and row_mrz.get("issuing_state") == candidate.get(
                "code"
            ):
                result["variant_of"] = row.get("filename")
                break
    del doc_number

    # 10. needs_eyes
    has_specimen_signal = bool(result["specimen_snippets"]) or check.get("watermark") is True
    if not has_specimen_signal:
        result["needs_eyes"] = True
    if check.get("mrz_status") == "miss" and doc_claims_mrz(candidate):
        result["needs_eyes"] = True

    return result


def _delete_staged(path: str | None) -> None:
    if not path:
        return
    try:
        if os.path.exists(path):
            os.remove(path)
    except OSError:
        pass


# --------------------------------------------------------------------------
# Packet (Markdown, survivors only)
# --------------------------------------------------------------------------


def _escape_md(text: str | None) -> str:
    return (text or "").replace("|", "\\|").replace("\n", " ").strip()


def write_packet(path: str, survivors: list[dict]) -> None:
    lines = []
    lines.append("# Specimen candidates -- automated screen survivors\n\n")
    lines.append(
        f"Generated {datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%MZ')} by "
        "`tools/screen_candidates.py`. "
        f"{len(survivors)} candidate(s) survived the automated screen. "
        "\"Proposed destination\", \"proposed licence\" and \"Verdict\" are left blank for the "
        "human reviewer -- see `knowledge/SPECIMEN_SOURCES.md`'s \"Review gates for agent-found "
        "candidates\". No holder value (name, document number, date of birth) appears in this "
        "file.\n\n"
    )
    header = (
        "| # | Code | Doc/TD | Series | Host | Licence evidence | Specimen signal | MRZ check | "
        "Proposed destination | Proposed licence | Local path | Note | Verdict |"
    )
    sep = "|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    lines.append(header + "\n")
    lines.append(sep + "\n")

    for i, rec in enumerate(survivors, 1):
        code = rec.get("code", "")
        td = rec.get("td", "")
        doc_td = f"{rec.get('doc', '')} / {td}" if td else rec.get("doc", "")
        series = rec.get("series", "")
        host = urlparse(rec.get("page_url_final") or rec.get("page_url") or "").netloc
        licence = "; ".join(rec.get("licence_snippets") or []) or rec.get("licence_evidence", "")
        specimen = "; ".join(rec.get("specimen_snippets") or []) or rec.get("specimen_signal", "")
        mrz_check = rec.get("mrz_check", "none")
        staged_path = rec.get("staged_path") or ""
        local_link = f"[{os.path.basename(staged_path)}]({staged_path})" if staged_path else ""

        note_parts = []
        if rec.get("needs_eyes"):
            note_parts.append("needs eyes")
        if rec.get("h5_check"):
            # The worker saw a source that may carry a photograph and left the
            # real-person call to the screener, who opens the image (prefix v3.2).
            note_parts.append("h5 check")
        if rec.get("variant_of"):
            note_parts.append(f"variant of {rec['variant_of']}")
        if rec.get("pdf_source"):
            note_parts.append(f"from PDF p.{rec['pdf_source'].get('page')} ({rec['pdf_source'].get('kind')})")
        if rec.get("page_source") == "session-fetched":
            note_parts.append("page: session-fetched")
        note = "; ".join(note_parts)

        row = (
            f"| {i} | {_escape_md(code)} | {_escape_md(doc_td)} | {_escape_md(series)} | "
            f"{_escape_md(host)} | {_escape_md(licence)} | {_escape_md(specimen)} | {mrz_check} | "
            f"| | {local_link} | {_escape_md(note)} | |"
        )
        lines.append(row + "\n")

    with open(path, "w", encoding="utf-8") as f:
        f.writelines(lines)


# --------------------------------------------------------------------------
# Ledger row
# --------------------------------------------------------------------------


def build_ledger_row(candidate: dict, reason: str, sha256: str | None, cycle: str | None) -> dict:
    return {
        "url": candidate_url(candidate),
        "sha256": sha256,
        "code": candidate.get("code"),
        "reason": reason,
        "cycle": cycle,
        "date": date.today().isoformat(),
    }


# --------------------------------------------------------------------------
# Output record: never write a holder value
# --------------------------------------------------------------------------


def screened_record_for_output(result: dict) -> dict:
    """Every field in `result` is already safe to write -- doc_number is
    never stored on `result` (see screen_candidate's step 9 comment) -- but
    this makes that guarantee explicit and in one place."""
    record = dict(result)
    record.pop("doc_number", None)  # defence in depth; never expected to be present
    record.pop("_pdf_bytes", None)  # the PDF lane's in-memory hand-off, never an output field
    record.pop("is_pdf", None)
    return record


# --------------------------------------------------------------------------
# Main
# --------------------------------------------------------------------------


def find_repo_root() -> str:
    return os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--candidates", required=True, help="JSONL file of worker-found candidates")
    parser.add_argument("--staging", required=True, help="directory to stage downloaded images into")
    parser.add_argument("--ledger", required=True, help="JSONL ledger of prior admit/reject decisions (appended to)")
    parser.add_argument("--corpus", required=True, help="samples/corpus.jsonl, for byte-duplicate and variant checks")
    parser.add_argument("--out", required=True, help="where to write one screened JSON record per candidate")
    parser.add_argument("--packet", required=True, help="where to write the survivors-only Markdown packet")
    parser.add_argument(
        "--check-sample-bin",
        default=None,
        help="override the check_sample binary path (default: target/release/examples/check_sample[.exe])",
    )
    parser.add_argument(
        "--page-html-dir",
        default=None,
        help=(
            "directory of session-fetched page HTML (pages.jsonl + <sha12>.html) used instead of "
            "fetching page_url; the session, never this tool, does that retry (H14)"
        ),
    )
    args = parser.parse_args(argv)

    repo_root = find_repo_root()
    binary_path = args.check_sample_bin or find_check_sample_binary(repo_root)
    if not binary_path:
        print(
            "check_sample binary not found. Build it first:\n"
            "  cargo build -p synthpass-ocr --release --example check_sample",
            file=sys.stderr,
        )
        return 1

    candidates = load_jsonl(args.candidates)
    ledger_records = load_jsonl(args.ledger)
    corpus_records = load_jsonl(args.corpus)
    host_denylist = load_host_denylist(os.path.join(os.path.dirname(os.path.abspath(__file__)), HOST_DENYLIST_FILENAME))
    try:
        page_html = load_page_html_dir(args.page_html_dir)
    except FileNotFoundError as e:
        print(f"--page-html-dir: {e}", file=sys.stderr)
        return 1
    if page_html:
        print(f"{len(page_html)} session-fetched page(s) available from {args.page_html_dir}")

    if fitz is None and any((c.get("format") or "").lower() == "pdf" for c in candidates):
        print(
            "a candidate declares format=pdf but PyMuPDF is not installed; nothing was screened.\n"
            "  pip install pymupdf   (tools-only dependency; the extraction path never imports it)",
            file=sys.stderr,
        )
        return 1

    os.makedirs(args.staging, exist_ok=True)

    screened: list[dict] = []
    survivors: list[dict] = []
    pdf_skipped: dict[str, int] = {}

    for candidate in candidates:
        result = screen_candidate(
            candidate,
            args.staging,
            ledger_records,
            corpus_records,
            binary_path,
            host_denylist=host_denylist,
            page_html=page_html,
        )
        outcomes = [result]
        if result.get("is_pdf") and result["auto_reject"] is None:
            outcomes = expand_pdf_candidate(
                result, args.staging, ledger_records, corpus_records, binary_path, skipped_counter=pdf_skipped
            )
        for outcome in outcomes:
            screened.append(outcome)
            if outcome["auto_reject"] is None:
                survivors.append(outcome)
            else:
                # A PDF child carries its own `image_url#page=N`, so the ledger row names the page.
                row = build_ledger_row(outcome, outcome["auto_reject"], outcome.get("sha256"), candidate.get("cycle"))
                append_jsonl(args.ledger, row)
                ledger_records.append(row)  # visible to later candidates in this same run

    with open(args.out, "w", encoding="utf-8") as f:
        for rec in screened:
            f.write(json.dumps(screened_record_for_output(rec), ensure_ascii=False) + "\n")

    write_packet(args.packet, survivors)

    rejected = len(screened) - len(survivors)
    expanded = f" ({len(screened)} after PDF expansion)" if len(screened) != len(candidates) else ""
    print(f"Screened {len(candidates)} candidate(s){expanded}: {len(survivors)} survived, {rejected} auto-rejected.")
    if pdf_skipped:
        print("PDF pages skipped: " + ", ".join(f"{count} {reason}" for reason, count in sorted(pdf_skipped.items())))
    session_pages = sum(1 for rec in screened if rec.get("page_source") == "session-fetched")
    if session_pages:
        print(f"{session_pages} candidate(s) used a session-fetched page (page_source=session-fetched).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
