#!/usr/bin/env python3
"""
Deterministic ICAO Doc 9303 Part-PDF -> Markdown extractor.

Given one of ICAO's consolidated Part PDFs (the same files this repo reads at
`D:\\Users\\Rusmir\\pdf_docs9303\\9303_pNN_cons_en.pdf` on the machine this was
built on, never committed here — see `knowledge/docs9303/README.md`), this
tool reconstructs the front matter, an auto-generated table of contents, and
the body as clean Markdown. The same PDF byte-for-byte always produces the
same Markdown byte-for-byte: nothing here samples randomness, iterates a
`dict`/`set` whose order is not already sorted, or depends on wall-clock time.

What is mechanical (done by this tool, safe to regenerate) vs. what would be
a hand judgement call (deliberately NOT done here, since Part 10's own
rebuild — see the audit note in `knowledge/docs9303/README.md` — found that
"structure only" is the only safe kind of post-processing; a paraphrase is a
paraphrase even when it is a well-meaning one):

* Page furniture (running headers, folio numbers, the `App X-N` appendix
  page markers) is stripped by pattern match, not by hand.
* ICAO's own numbered clause headings (`4.2.1 Title`) are promoted to
  Markdown headings by numbering depth. The two-line "Appendix X to Part N /
  TITLE" construct is recognised as a special case (see `is_appendix_start`).
* Body text is reflowed from the PDF's word-wrapped lines back into
  paragraphs. The **only** transformation applied to the words themselves is
  joining a line that PDF justification wrapped mid-word or mid-token back
  together (`join_wrapped`); nothing is reworded, summarised or corrected.
* Tables that PyMuPDF's table detector can cleanly recover (see
  `find_confident_tables`) become Markdown pipe tables. A handful of the
  widest, most visually complex tables in Part 12 (its certificate-extension
  profile tables) defeat that detector — it explodes past 30 spurious
  columns — and are deliberately left as reading-order paragraph text rather
  than rendered into a table structure invented by a low-confidence table
  detector; see the module-level `MAX_TABLE_COLUMNS` comment.

Usage:
    python tools/docs9303_extract.py <input.pdf> <output.md> \
        [--part N] [--title "Part N: ..."]

`--part` and `--title` are optional; both are otherwise read from the PDF's
own title page, which repeats "Part N: <title>" verbatim on page 1 and 3.

Requires PyMuPDF (`import fitz`) for the PDF-reading half only; the pure
text-transformation functions below have no such dependency and are what
`tools/test_docs9303_extract.py` exercises without touching a PDF at all.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

try:
    import fitz  # PyMuPDF
except ImportError:  # pragma: no cover - the CI tools job installs nothing
    fitz = None

REPO_ROOT = Path(__file__).resolve().parent.parent

# A detected table wider than this is treated as a false positive: PyMuPDF's
# grid detector, given a dense multi-line-per-cell table with merged header
# cells (Part 12's Table 6, "Certificate Extensions Profile", and its
# neighbours), fragments the header row into one spurious column per short
# header word instead of the ~10 real columns. Every genuine table found in
# Parts 10-12 has 9 columns or fewer; the pathological detections measured
# during this rebuild were 22, 33 and 37 columns on the same three pages.
MAX_TABLE_COLUMNS = 9
MIN_TABLE_ROWS = 2

BULLET_GLYPHS = ("\u2022", "-")


# ===========================================================================
# Pure helpers: no PDF, no filesystem, no fitz. tools/test_docs9303_extract.py
# exercises all of these directly.
# ===========================================================================


def is_blank_line(line: str) -> bool:
    """A line PyMuPDF's `text` mode emits for pure vertical whitespace."""
    return line.strip() == ""


_PAGE_NUMBER_RE = re.compile(
    r"^(?:\(?[ivxlcdm]+\)?|\d{1,4}|App\s+[A-Z]-\d+)$", re.IGNORECASE
)


def is_page_number_line(line: str) -> bool:
    """True for a line that is only a folio marker: `12`, `(vii)`, `App D-3`."""
    s = line.strip()
    if not s:
        return False
    return bool(_PAGE_NUMBER_RE.match(s))


_REVISION_DATE_RE = re.compile(r"^\d{1,2}/\d{1,2}/\d{2,4}$")
_REVISION_NUMBER_RE = re.compile(r"^No\.\s*\d+$")


def is_revision_date_line(line: str) -> bool:
    """True for a bare `D/M/YY` date: the margin revision-date stamp ICAO
    prints at the bottom of any page carrying text an amendment touched
    (`23/2/26`), paired with `is_revision_number_line` on the very next
    line (`No. 2`). Distinct from `is_page_number_line`, whose folio
    numbers are bare integers or roman numerals, never slash-separated."""
    return bool(_REVISION_DATE_RE.match(line.strip()))


def is_revision_number_line(line: str) -> bool:
    """True for a bare `No. N` amendment-number stamp. Only trustworthy as
    furniture directly after a matching `is_revision_date_line` — `No. 1` is
    also how ICAO's own text refers to real standards ("Latin alphabet No.
    1"), so `strip_furniture` only strips this pattern when the line right
    before it was already recognised as the paired revision date."""
    return bool(_REVISION_NUMBER_RE.match(line.strip()))


def is_running_header_line(line: str, verso_header: str, recto_header: str) -> bool:
    """True for the two alternating page-header lines ICAO prints on every
    body page: the plain document title on verso pages, `Part N.  <title>`
    on recto pages."""
    s = line.strip()
    return s == verso_header.strip() or s == recto_header.strip()


def recto_header_for(part_number: int, part_title: str) -> str:
    """The running header ICAO prints on recto (odd) body pages, derived
    from the same "Part N: <title>" line the title page repeats verbatim."""
    return f"Part {part_number}.    {part_title}"


_TITLE_LINE_RE = re.compile(r"^Part\s+(\d+)\s*:\s*(.+?)\s*$")


def parse_part_title_line(line: str) -> Optional[tuple[int, str]]:
    """Parse ICAO's title-page line `Part 11: Security Mechanisms for MRTDs`
    into `(11, "Security Mechanisms for MRTDs")`."""
    m = _TITLE_LINE_RE.match(line.strip())
    if not m:
        return None
    return int(m.group(1)), m.group(2)


# A numbered clause heading: `4.2.1    Title text`, or an appendix
# subsection: `A.1    Title text`. Requires a 2+ space gap between the
# number and the title, and a capitalised title, which is what distinguishes
# a real heading line from a wrapped sentence that merely starts with a
# cross-reference number (`9.5.1 SHOULD be used.` has a single-space gap;
# `56 Bit for a numeric Document Number` has a lower-case-only continuation
# either way) — both patterns were found and ruled out by hand against the
# Part 11/12 PDF text during this rebuild.
_HEADING_RE = re.compile(
    r"^(?P<number>\d+(?:\.\d+){0,5}|[A-Z](?:\.\d+){1,5})\.?[ \t]{2,}(?P<title>\S.*?)[ \t]*$"
)


@dataclass(frozen=True)
class Heading:
    number: str
    title: str

    @property
    def is_appendix_subsection(self) -> bool:
        return self.number[0].isalpha()

    @property
    def depth(self) -> int:
        return len(self.number.split("."))

    @property
    def anchor_text(self) -> str:
        return f"{self.number} {self.title}"


def detect_heading(line: str) -> Optional[Heading]:
    """Detect a numbered ICAO clause heading on a single already-furniture-
    stripped line. Returns None for anything else, including a wrapped
    sentence that happens to start with a number.

    The title is required to start with a letter (rejecting a stray
    fragment like a lone symbol), but NOT specifically an upper-case one:
    "3.1    eMRTD PKI" is a real ICAO heading whose title starts with a
    deliberately lower-case term. The 2+ space gap in `_HEADING_RE` is
    already what tells a heading apart from a wrapped sentence that merely
    starts with a cross-reference number (verified against the two real
    false positives found in Parts 11-12's PDF text, `9.5.1 SHOULD be
    used.` and `56 Bit for a numeric Document Number...`, both of which
    have only a single-space gap and are excluded by the regex itself, not
    by a case check).
    """
    m = _HEADING_RE.match(line)
    if not m:
        return None
    title = m.group("title").strip()
    if not title or not title[0].isalpha():
        return None
    return Heading(number=m.group("number"), title=title)


def heading_markdown_level(h: Heading) -> int:
    """Markdown heading level from numbering depth, matching the convention
    already used by `knowledge/docs9303/Doc_9303_Part10_LDS_for_Storage_of_Biometrics_and_Other_Data_in_the_Contactless_IC.md`: a top-level
    clause (`4`) is `##`, each further numbering level is one level deeper,
    capped at `#####` (both a 4-deep and a 5-deep clause number render as
    `#####` in Part 10; verified against `4.7.3.2` and `4.7.3.2.1`, both
    `#####`). An appendix subsection (`A.1`) starts one level below its
    parent `### Appendix A to Part N — ...` heading, so `A.1` is `####` and
    `A.1.1` is `#####`.
    """
    if h.is_appendix_subsection:
        after_letter = h.depth - 1
        return min(after_letter + 3, 5)
    return min(h.depth + 1, 5)


_APPENDIX_START_RE = re.compile(r"^Appendix\s+([A-Z])\s+to\s+Part\s+(\d+)\s*$", re.IGNORECASE)


def is_appendix_start(line: str) -> Optional[tuple[str, str]]:
    """Detect the first line of ICAO's two-line appendix heading construct:
    `Appendix A to Part 11` on its own line, followed (after a blank line)
    by the appendix's real title in a following block. Returns
    `(letter, part_number)` or None."""
    m = _APPENDIX_START_RE.match(line.strip())
    if not m:
        return None
    return m.group(1), m.group(2)


def is_bullet_marker_line(line: str) -> bool:
    """ICAO's PDFs print a bullet glyph alone on its own line, with the
    item's text starting on the next line: both `\u2022` and a plain `-` are
    used (as two different list styles) in Parts 11-12."""
    return line.strip() in BULLET_GLYPHS


def join_wrapped(prev: str, nxt: str) -> str:
    """Join two word-wrapped lines of the same paragraph back together.

    If `prev` ends in a hyphen, that hyphen is real ICAO text in every
    instance found across Parts 11 and 12's body prose: a compound word
    ("trust-points", "non-transferable", "pseudo-randomly"), a standard
    number ("ISO/IEC 9796-2", "[ISO/IEC 7816-4]"), or a hyphenated ASN.1
    identifier ("id-icao-mrtd-security-aaProtocolObject"). None of them is a
    single word that PDF justification broke and that should read without
    the hyphen at all (contrast Part 10's own narrow-table-column artefacts,
    "Chip Authenti-cation" and "Key-length", which come from cells handled
    by `find_confident_tables` instead and never reach this function). So
    the rule kept here is deliberately not "guess": a trailing hyphen is
    always real, the two lines are glued together with no space, and
    nothing is ever silently deleted from ICAO's text.
    """
    if prev.endswith("-"):
        return prev + nxt
    return prev.rstrip() + " " + nxt.lstrip()


def reflow(lines: list[str]) -> str:
    """Reflow a list of word-wrapped lines (all belonging to one paragraph)
    into a single line of text."""
    text = lines[0].strip()
    for line in lines[1:]:
        text = join_wrapped(text, line.strip())
    return text


def slugify_heading(text: str) -> str:
    """A GitHub-heading-slug approximation, sufficient for the plain
    "N. TITLE" top-level headings this tool links from its generated table
    of contents (none of which contain characters an exact reproduction of
    GitHub's per-character algorithm, in scripts/check-doc-links.sh, would
    treat differently: no em dashes, no repeated headings)."""
    s = text.strip().lower()
    s = re.sub(r"[^a-z0-9 _-]", "", s)
    s = re.sub(r"\s+", "-", s)
    return s.strip("-")


def linkify_url(line: str) -> str:
    """Turn a line ending in a bare URL/domain into `text [label](href)`
    Markdown, matching the style already used for this line in
    `knowledge/docs9303/Doc_9303_Part10_LDS_for_Storage_of_Biometrics_and_Other_Data_in_the_Contactless_IC.md`. The visible label is left
    exactly as printed; a `https://` scheme is added to the link target only
    when the line has none, since a bare `www.icao.int/...` domain is not a
    followable link without one."""
    m = re.search(r"(https?://\S+|www\.\S+)$", line.strip())
    if not m:
        return line
    url = m.group(1).rstrip(".")
    prefix = line[: m.start()].rstrip()
    href = url if url.startswith("http") else f"https://{url}"
    return f"{prefix} [{url}]({href})".strip()


def chunk_amendment_rows(cells: list[str]) -> list[tuple[str, str, str]]:
    """Group a flat list of amendment/corrigenda table cell values into
    (No., Date, Entered by) rows. ICAO's front-matter amendments table is
    linearised by PDF text extraction into one cell value per line with no
    column markers, but every row it has ever printed in Parts 10-12 is a
    complete (No., Date, Entered by) triple, so a straight 3-at-a-time chunk
    reconstructs it losslessly."""
    rows = []
    for i in range(0, len(cells) - len(cells) % 3, 3):
        rows.append((cells[i], cells[i + 1], cells[i + 2]))
    return rows


# ===========================================================================
# PDF-reading half. Needs fitz.
# ===========================================================================


def _require_fitz() -> None:
    if fitz is None:  # pragma: no cover - exercised only when fitz is absent
        raise RuntimeError(
            "PyMuPDF (fitz) is required to read a PDF; the pure text-transform "
            "functions above do not need it and are what the unit tests cover."
        )


def _page_lines(page) -> list[str]:
    return page.get_text("text").split("\n")


def find_confident_tables(page) -> list["fitz.table.Table"]:
    """Tables PyMuPDF's grid detector found on this page that pass the
    sanity check described at `MAX_TABLE_COLUMNS`, sorted top-to-bottom."""
    tables = []
    try:
        finder = page.find_tables()
    except Exception:  # pragma: no cover - defensive; fitz raises rarely
        return []
    for table in finder.tables:
        rows = table.extract()
        if len(rows) < MIN_TABLE_ROWS:
            continue
        if not rows or len(rows[0]) > MAX_TABLE_COLUMNS:
            continue
        tables.append(table)
    tables.sort(key=lambda t: t.bbox[1])
    return tables


def render_table_markdown(rows: list[list[Optional[str]]]) -> list[str]:
    """Render a fitz table's extracted rows as a Markdown pipe table. Cell
    text keeps its internal PDF line breaks as `<br>` (a pipe-table cell
    cannot contain a literal newline); `None` (an empty/merged cell) becomes
    an empty string."""

    def cell(v: Optional[str]) -> str:
        text = (v or "").strip()
        text = re.sub(r"\s*\n\s*", "<br>", text)
        text = text.replace("|", "\\|")
        return text

    out = []
    header = [cell(c) for c in rows[0]]
    out.append("| " + " | ".join(header) + " |")
    out.append("| " + " | ".join([":---"] * len(header)) + " |")
    for row in rows[1:]:
        cells = [cell(c) for c in row]
        while len(cells) < len(header):
            cells.append("")
        out.append("| " + " | ".join(cells[: len(header)]) + " |")
    return out


def page_segments(page, tables: list) -> list[tuple[str, object]]:
    """The page's content as an ordered list of `("text", lines)` and
    `("table", rows)` segments, top to bottom. A confident table's own
    vertical band is read once via `table.extract()` (never duplicated into
    the surrounding text segments) by clipping the text passes to the
    horizontal strips above, between and below the tables rather than
    filtering individual words. ICAO's tables all span (close to) the
    page's full text width, so a strip covering a table's `y` range and the
    full page width does not clip text that sits visibly beside it.

    Keeping this order — rather than collecting all of a page's text and
    then appending its tables afterwards — matters: a table is not always
    the last thing on its page, and Part 11's Appendix K ends with a table
    that sits BEFORE the page's closing "-- END --" line.
    """
    if not tables:
        return [("text", _page_lines(page))]
    rect = page.rect
    segments: list[tuple[str, object]] = []
    cursor = rect.y0
    for table in tables:
        y0, y1 = table.bbox[1], table.bbox[3]
        if y0 > cursor:
            strip = fitz.Rect(rect.x0, cursor, rect.x1, y0)
            segments.append(("text", page.get_text("text", clip=strip).split("\n")))
        segments.append(("table", table.extract()))
        cursor = max(cursor, y1)
    if cursor < rect.y1:
        strip = fitz.Rect(rect.x0, cursor, rect.x1, rect.y1)
        segments.append(("text", page.get_text("text", clip=strip).split("\n")))
    return segments


def strip_furniture(lines: list[str], verso_header: str, recto_header: str) -> list[str]:
    """Drop running headers, folio-number lines and margin revision-date
    stamps, keeping every blank line so paragraph/list boundaries are
    unaffected (a stripped furniture line was always already flanked by
    blank lines, or by the tail of the paragraph it marks, in ICAO's own
    layout)."""
    out = []
    previous_was_revision_date = False
    for line in lines:
        if is_blank_line(line):
            out.append(line)
            previous_was_revision_date = False
            continue
        if is_page_number_line(line):
            previous_was_revision_date = False
            continue
        if is_running_header_line(line, verso_header, recto_header):
            previous_was_revision_date = False
            continue
        if is_revision_date_line(line):
            previous_was_revision_date = True
            continue
        if previous_was_revision_date and is_revision_number_line(line):
            previous_was_revision_date = False
            continue
        previous_was_revision_date = False
        out.append(line)
    return out


@dataclass
class Block:
    kind: str  # "heading" | "para" | "list" | "table" | "appendix_pending"
    heading: Optional[Heading] = None
    text: Optional[str] = None
    items: Optional[list[str]] = None
    table_rows: Optional[list[list[Optional[str]]]] = None
    appendix_letter: Optional[str] = None
    appendix_part: Optional[str] = None


def _flush_group_simple(non_blank: list[str]) -> list[Block]:
    """Turn a run of lines already known to contain no heading into a single
    Block (a bullet item, an appendix marker, or an ordinary paragraph)."""
    if not non_blank:
        return []
    if len(non_blank) == 1:
        appendix = is_appendix_start(non_blank[0])
        if appendix is not None:
            return [
                Block(
                    kind="appendix_pending",
                    appendix_letter=appendix[0],
                    appendix_part=appendix[1],
                )
            ]
    if is_bullet_marker_line(non_blank[0]):
        item_text = reflow(non_blank[1:]) if len(non_blank) > 1 else ""
        return [Block(kind="list", items=[item_text])]
    return [Block(kind="para", text=reflow(non_blank))]


def _flush_group(group: list[str]) -> list[Block]:
    """Turn one blank-line-delimited group of raw lines into zero or more
    Blocks (zero for a group that was only furniture/blank).

    Normally a heading sits alone in its own group, with a blank line before
    the body text starts and another before whatever follows it. It is not
    always: a heading can run on directly into a figure/diagram that PDF
    text extraction flattens into body-less label text, or directly out of
    the bullet item above it, with no vertical gap the extractor renders as
    a blank line either side. Scanning every line of the group for a
    heading match — not just the first — and splitting the group there
    keeps a heading like this from being silently swallowed into the
    bullet item or paragraph it happens to be glued to; both the found
    heading and its surrounding text are still emitted, split at the
    heading's own line rather than folded into one Block together.
    """
    non_blank = [l for l in group if not is_blank_line(l)]
    if not non_blank:
        return []
    heading_idx = next((i for i, l in enumerate(non_blank) if detect_heading(l)), None)
    if heading_idx is None:
        return _flush_group_simple(non_blank)
    before, heading_line, after = (
        non_blank[:heading_idx],
        non_blank[heading_idx],
        non_blank[heading_idx + 1 :],
    )
    blocks = _flush_group_simple(before)
    blocks.append(Block(kind="heading", heading=detect_heading(heading_line)))
    blocks.extend(_flush_group(after))
    return blocks


def build_blocks(pages: list[tuple[int, list[str]]]) -> list[tuple[int, Block]]:
    """Group a document's furniture-stripped, per-page line lists into
    `(page_number, Block)` pairs in reading order. `pages` is a list of
    `(1-indexed page number, lines)`, with no tables — the shape every unit
    test in this file uses. `build_blocks_from_segments` is the version the
    real PDF-reading pipeline calls, which additionally interleaves table
    segments in their true vertical position."""
    return build_blocks_from_segments([(page_number, "text", lines) for page_number, lines in pages])


def build_blocks_from_segments(
    segments: list[tuple[int, str, object]]
) -> list[tuple[int, Block]]:
    """Like `build_blocks`, but over `(page_number, kind, payload)` triples
    where `kind` is `"text"` (`payload` is a line list) or `"table"`
    (`payload` is a fitz table's already-`extract()`-ed rows) — the ordering
    `page_segments` produces, so a table is emitted exactly where it sits on
    the page relative to the surrounding text, not always last.

    A pending appendix marker (`Appendix A to Part 11` on its own line) is
    merged with the very next block, which is its title, into one combined
    appendix heading — this is the one place a Block absorbs its neighbour,
    because ICAO prints the appendix number and its title as two separate
    text blocks with nothing else to tell them apart.
    """
    raw: list[tuple[int, str, object]] = []
    for page_number, kind, payload in segments:
        if kind == "table":
            raw.append((page_number, "table", payload))
            continue
        group: list[str] = []
        for line in payload:
            if is_blank_line(line):
                if group:
                    raw.append((page_number, "group", group))
                    group = []
                continue
            group.append(line)
        if group:
            raw.append((page_number, "group", group))

    blocks: list[tuple[int, Block]] = []
    pending_appendix: Optional[tuple[int, str, str]] = None
    for page_number, kind, payload in raw:
        group_blocks = (
            [Block(kind="table", table_rows=payload)] if kind == "table" else _flush_group(payload)
        )
        for block in group_blocks:
            if pending_appendix is not None:
                appendix_page, letter, part_no = pending_appendix
                title = block.text if block.kind == "para" else reflow(payload)
                heading = Heading(number=f"Appendix{letter}", title=title)
                blocks.append(
                    (
                        appendix_page,
                        Block(kind="heading", heading=heading, appendix_letter=letter, appendix_part=part_no),
                    )
                )
                pending_appendix = None
                continue
            if block.kind == "appendix_pending":
                pending_appendix = (page_number, block.appendix_letter, block.appendix_part)
                continue
            blocks.append((page_number, block))

    # Merge adjacent list blocks (each bullet item arrived as its own group)
    # into one list per contiguous run.
    merged: list[tuple[int, Block]] = []
    for page_number, block in blocks:
        if (
            block.kind == "list"
            and merged
            and merged[-1][1].kind == "list"
        ):
            merged[-1][1].items.extend(block.items)
        else:
            merged.append((page_number, block))
    return merged


def render_appendix_heading(letter: str, part_no: str, title: str) -> str:
    return f"### Appendix {letter} to Part {part_no} \u2014 {title}"


def render_heading(h: Heading, appendix_letter: Optional[str], appendix_part: Optional[str]) -> str:
    if appendix_letter is not None:
        return render_appendix_heading(appendix_letter, appendix_part, h.title)
    level = heading_markdown_level(h)
    # A top-level clause number keeps ICAO's own trailing period ("1.
    # SCOPE"); a subsection number never had one to begin with ("3.1
    # Minimum Requirements..."), matching Part 10's existing convention.
    number = f"{h.number}." if h.depth == 1 else h.number
    return f"{'#' * level} {number} {h.title}"


def render_body(blocks: list[tuple[int, Block]]) -> str:
    """Render the ordered blocks to Markdown, with a `<!-- page N -->`
    marker inserted before the first block that starts on each new page (so
    a paragraph that spans a page break carries a single marker at its
    start, never splitting it)."""
    out: list[str] = []
    last_page = None
    for page_number, block in blocks:
        if page_number != last_page:
            out.append(f"<!-- page {page_number} -->")
            out.append("")
            last_page = page_number
        if block.kind == "heading":
            out.append(render_heading(block.heading, block.appendix_letter, block.appendix_part))
        elif block.kind == "para":
            out.append(block.text)
        elif block.kind == "list":
            for item in block.items:
                out.append(f"* {item}")
        elif block.kind == "table":
            out.extend(render_table_markdown(block.table_rows))
        out.append("")
    while out and out[-1] == "":
        out.pop()
    return "\n".join(out) + "\n"


def build_table_of_contents(blocks: list[tuple[int, Block]]) -> str:
    """Auto-generate a table of contents from the detected headings: the
    top-level numbered clauses as linked entries, everything else nested and
    unlinked, matching the convention already used in
    `knowledge/docs9303/Doc_9303_Part10_LDS_for_Storage_of_Biometrics_and_Other_Data_in_the_Contactless_IC.md`."""
    lines = ["## TABLE OF CONTENTS", ""]
    for _, block in blocks:
        if block.kind != "heading":
            continue
        h = block.heading
        if block.appendix_letter is not None:
            lines.append(
                f"- Appendix {block.appendix_letter} to Part {block.appendix_part} "
                f"\u2014 {h.title}"
            )
            continue
        indent = "  " * (h.depth - 1)
        if h.depth == 1:
            anchor = slugify_heading(h.anchor_text)
            lines.append(f"- [{h.number}. {h.title}](#{anchor})")
        else:
            lines.append(f"{indent}- {h.number} {h.title}")
    lines.append("")
    return "\n".join(lines)


def find_body_start_page(pages_text: list[str]) -> int:
    """The 0-indexed PDF page where clause 1 begins: the first page whose
    text contains a heading numbered exactly `1`."""
    for i, text in enumerate(pages_text):
        for line in text.split("\n"):
            heading = detect_heading(line)
            if heading is not None and heading.number == "1":
                return i
    raise ValueError("no top-level '1.' heading found; is this an ICAO Doc 9303 Part PDF?")


def extract_front_matter(pages_text: list[str], part_number: int, part_title: str) -> str:
    """Reconstruct the title page / bibliographic block / amendments table /
    disclaimer paragraph from the PDF's first few pages. This block is near-
    identical boilerplate across every Part; only the values pulled out of
    the PDF text below (title, edition, order number, ISBNs, amendment
    rows) vary."""
    page0_lines = [l.strip() for l in pages_text[0].split("\n") if l.strip()]
    # page0_lines: [authority line, org name, "Doc 9303", subtitle, "Part N: Title", edition]
    authority_line, org_name, _doc9303, subtitle, _part_line, edition_line = page0_lines[:6]

    biblio_page = next(t for t in pages_text if "ISBN" in t)
    biblio_lines = [l.strip() for l in biblio_page.split("\n") if l.strip()]
    isbn_lines = [l for l in biblio_lines if l.startswith("ISBN")]
    order_line = next(l for l in biblio_lines if l.startswith("Order No"))
    part_line = next(l for l in biblio_lines if l.startswith(f"Part {part_number} "))
    copyright_line = next(l for l in biblio_lines if l.startswith("\u00a9"))
    downloads_line = next(l for l in biblio_lines if l.startswith("Downloads"))
    address_lines = biblio_lines[: biblio_lines.index(downloads_line)]
    disclaimer_start = biblio_lines.index(
        next(l for l in biblio_lines if l.startswith("All rights reserved"))
    )
    # Stop at the sentence's own end ("... Civil Aviation Organization."),
    # not at the end of the page: Part 11's biblio page additionally carries
    # a trailing revision stamp ("23/2/26" / "No. 2") below the copyright
    # paragraph that is not part of it.
    disclaimer_lines = []
    for line in biblio_lines[disclaimer_start:]:
        disclaimer_lines.append(line)
        if line.rstrip().endswith("Organization."):
            break
    disclaimer_text = reflow(disclaimer_lines)

    amend_page = next(t for t in pages_text if "RECORD OF AMENDMENTS" in t)
    amend_lines = [l.strip() for l in amend_page.split("\n") if l.strip()]
    header_idx = amend_lines.index(
        next(l for l in amend_lines if l.startswith("RECORD OF AMENDMENTS"))
    )
    designations_idx = amend_lines.index(
        next(l for l in amend_lines if l.startswith("The designations employed"))
    )
    # Skip past the two/three column-header trios ("No.", "Date", "Entered by",
    # optionally repeated once for a Corrigenda column) to the actual values.
    entered_by_positions = [i for i, l in enumerate(amend_lines) if l == "Entered by"]
    values_start = entered_by_positions[-1] + 1 if entered_by_positions else header_idx + 1
    cell_values = amend_lines[values_start:designations_idx]
    rows = chunk_amendment_rows(cell_values)
    disclaimer_para = reflow(amend_lines[designations_idx:])

    lines = [
        "# ICAO Doc 9303",
        f"## {subtitle}",
        f"### Part {part_number}: {part_title}",
        f"**{edition_line}**",
        "",
        authority_line + "  ",
        f"**{org_name}**  ",
        "",
    ]
    # The address block is 4 PDF lines wrapping 3 real clauses (publisher
    # sentence, org name, street address) in both Part 11 and Part 12 -- the
    # only two inputs this tool is asked to handle; reflow the first two
    # back into the one sentence they are, and keep org name / street
    # address as their own hard-broken lines, matching Part 10's style.
    address_out = [reflow(address_lines[:2])] + address_lines[2:] if len(address_lines) >= 2 else address_lines
    for address_line in address_out:
        lines.append(address_line + "  ")
    lines.append(linkify_url(downloads_line))
    lines.append("")
    lines.append("**Doc 9303, Machine Readable Travel Documents**  ")
    lines.append(part_line + "  ")
    lines.append(order_line + "  ")
    for isbn in isbn_lines:
        lines.append(isbn + "  ")
    lines.append("")
    lines.append(copyright_line + "  ")
    lines.append(disclaimer_text)
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("### AMENDMENTS AND CORRIGENDA")
    lines.append("")
    lines.append("| No. | Date | Entered by |")
    lines.append("| :--- | :--- | :--- |")
    for no, date, by in rows:
        lines.append(f"| {no} | {date} | {by} |")
    lines.append("")
    lines.append(f"*{disclaimer_para}*")
    lines.append("")
    lines.append("---")
    lines.append("")
    return "\n".join(lines) + "\n"


def extract(pdf_path: Path, part_number: Optional[int], part_title: Optional[str]) -> str:
    _require_fitz()
    doc = fitz.open(str(pdf_path))
    pages_text = [doc[i].get_text("text") for i in range(doc.page_count)]

    if part_number is None or part_title is None:
        parsed = None
        for line in pages_text[0].split("\n"):
            parsed = parse_part_title_line(line)
            if parsed:
                break
        if parsed is None:
            raise ValueError("could not read 'Part N: Title' from the PDF's title page")
        part_number, part_title = parsed

    front_matter = extract_front_matter(pages_text, part_number, part_title)

    body_start = find_body_start_page(pages_text)
    verso_header = "Machine Readable Travel Documents"
    recto_header = recto_header_for(part_number, part_title)

    segments: list[tuple[int, str, object]] = []
    for i in range(body_start, doc.page_count):
        page = doc[i]
        tables = find_confident_tables(page)
        for kind, payload in page_segments(page, tables):
            if kind == "text":
                payload = strip_furniture(payload, verso_header, recto_header)
            segments.append((i + 1, kind, payload))

    blocks = build_blocks_from_segments(segments)

    toc = build_table_of_contents(blocks)
    body = render_body(blocks)
    return front_matter + toc + "\n" + body + "\n---\n*\u2014 END \u2014*\n"


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pdf", type=Path, help="path to an ICAO Doc 9303 Part PDF")
    parser.add_argument("output", type=Path, help="path to write the generated Markdown")
    parser.add_argument("--part", type=int, default=None, help="Part number (else read from the PDF)")
    parser.add_argument("--title", type=str, default=None, help="Part title (else read from the PDF)")
    args = parser.parse_args(argv)

    markdown = extract(args.pdf, args.part, args.title)
    args.output.write_text(markdown, encoding="utf-8", newline="\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
