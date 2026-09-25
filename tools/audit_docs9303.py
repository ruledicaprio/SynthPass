"""Audit Doc 9303 Markdown against local ICAO PDF text layers.

Usage: python tools/audit_docs9303.py --pdf-dir DIR [--part N ...]
Only source PDFs are read; this tool never rewrites Markdown or PDFs.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import re

import docs9303_extract as extract


ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "knowledge" / "docs9303"
PAGE_MARKER = re.compile(r"<!--\s*page\s+\d+\s*-->", re.IGNORECASE)
LINK_TARGET = re.compile(r"\]\([^)]+\)")
IMAGE_TAG = re.compile(r"<img\b[^>]*>", re.IGNORECASE)
DIGITS = re.compile(r"\d+")
MD_HEADING = re.compile(r"^#{2,6}\s+((?:\d+(?:\.\d+)*|[A-Z](?:\.\d+)+))\.?\s+(.+?)\s*$")
FIGURE = re.compile(r"^Figure\s+(\d+)\s*[.:]\s*(.+?)(?:\*{2})?\s*$", re.IGNORECASE)
PIPE_ROW = re.compile(r"^\|.*\|$")
WORDS = re.compile(r"\w+", re.UNICODE)
WRAPPED_HYPHEN = re.compile(r"(?<=\w)-\s*\n\s*(?=\w)")


@dataclass(frozen=True)
class FigureTable:
    line: int
    number: str
    title: str


def digit_tokens(markdown: str) -> list[tuple[int, str]]:
    """Digit runs with line numbers, ignoring page comments and link destinations."""
    found = []
    for number, line in enumerate(markdown.splitlines(), 1):
        line = PAGE_MARKER.sub("", line)
        line = LINK_TARGET.sub("]", line)
        line = IMAGE_TAG.sub("", line)
        found.extend((number, match.group()) for match in DIGITS.finditer(line))
    return found


def heading_key(heading: extract.Heading) -> str:
    return heading.number


# Broader than `extract.is_appendix_start`, which only matches the exact
# two-line "Appendix A to Part 11" \n "TITLE" construct Parts 10-12 use.
# Part 3's own appendices are printed inline instead — "APPENDIX B TO PART 3
# — TRANSLITERATION..." all on one line — so `pdf_headings` needs a marker
# that recognises both conventions. Case matters here: ICAO always prints
# its own appendix heading in full caps ("APPENDIX B TO PART 3"), while a
# body sentence that merely mentions an appendix uses sentence case
# ("Appendix A to this Part.", "Appendix B and C.5.10 of Appendix C..."), so
# an all-caps match is what tells a real appendix heading apart from a
# wrapped cross-reference sentence that happens to start a physical PDF
# line with the word "Appendix" (found in Part 3's own body text; an
# case-insensitive first version of this regex caught it and wrongly
# treated everything after it, including clauses 5-8, as "past the
# appendix").
_APPENDIX_MARKER_RE = re.compile(r"^APPENDIX\s+[A-Z]\b")


def _is_appendix_marker_line(line: str) -> bool:
    return extract.is_appendix_start(line) is not None or bool(
        _APPENDIX_MARKER_RE.match(line.strip())
    )


def pdf_headings(lines: list[str]) -> dict[str, str]:
    """Every clause heading `extract.find_headings_in_lines` finds in the raw
    PDF body lines — same-line and split-across-two-lines alike, so a
    detector fix here is automatically what the extractor itself promotes,
    not a second opinion of it (see `number_alone_candidates` below for a
    genuinely independent check that doesn't share this detector at all).

    A bare, no-dot clause number (`9`) found after the first "Appendix X to
    Part N" marker is dropped: ICAO's own appendices number their own
    subsections with a letter prefix (`A.1`, `B.5.5.1`), never a bare
    top-level integer, so one appearing there is something else entirely —
    Part 3 Appendix B's own 16-item name-transliteration list happens to
    number its items "1." through "16.", and item "9.   Mohammed" collides
    with `_HEADING_RE` exactly the way a real top-level heading would.
    Every genuine bare-integer heading in this corpus (verified across all
    13 Parts) appears before its Part's first Appendix.
    """
    appendix_start = next(
        (i for i, line in enumerate(lines) if _is_appendix_marker_line(line)),
        None,
    )
    found = {}
    for index, heading in extract.find_headings_in_lines(lines):
        if appendix_start is not None and index >= appendix_start and heading.depth == 1:
            continue
        found.setdefault(heading_key(heading), heading.anchor_text)
    return found


# Deliberately independent of `extract.detect_heading` /
# `extract.find_headings_in_lines`: a clause number alone on its own PDF
# line, immediately followed by a line that starts with a capital letter —
# nothing more. This exists so a bug in the shared split-heading detector
# above (parent-hierarchy tracking, title-shape rules) does not also hide
# itself from the audit: `heading_diff` checks the PDF against the
# Markdown using the SAME detector on both sides, so a detector miss is, by
# construction, invisible to it. This check's own false positives (an RFC
# 5280 clause number quoted inline in Part 12's Appendix B, immediately
# followed by capitalised quoted prose) are expected and require the same
# by-hand triage against the rendered PDF page every other check here does
# — see `knowledge/docs9303/README.md`.
_NUMBER_ALONE_RE = re.compile(r"^(?:[A-Z]\.)?\d+(?:\.\d+){1,5}\.?$")


def number_alone_candidates(lines: list[str]) -> list[tuple[int, str, str]]:
    """`(line_index, number, next_line)` for every PDF line that is only a
    clause number, immediately followed by a capitalised line."""
    found = []
    for i, line in enumerate(lines[:-1]):
        number = line.strip()
        if not _NUMBER_ALONE_RE.match(number):
            continue
        nxt = lines[i + 1].strip()
        if nxt and nxt[0].isupper():
            found.append((i, number.rstrip("."), nxt))
    return found


def markdown_headings(markdown: str) -> dict[str, str]:
    found = {}
    for line in markdown.splitlines():
        match = MD_HEADING.match(line)
        if match is None:
            continue
        # The extractor's detector remains the single definition of a clause heading.
        heading = extract.detect_heading(f"{match.group(1)}  {match.group(2)}")
        if heading is not None:
            found.setdefault(heading_key(heading), heading.anchor_text)
    return found


def heading_diff(pdf_lines: list[str], markdown: str) -> tuple[list[str], list[str]]:
    expected = pdf_headings(pdf_lines)
    actual = markdown_headings(markdown)
    missing = [expected[key] for key in sorted(expected.keys() - actual.keys())]
    extra = [actual[key] for key in sorted(actual.keys() - expected.keys())]
    return missing, extra


def figure_tables(markdown: str) -> list[FigureTable]:
    """Find pipe tables immediately after a caption, allowing image/editorial markup."""
    lines = markdown.splitlines()
    found = []
    for index, raw in enumerate(lines):
        caption = raw.strip().lstrip(">").strip().strip("*").strip()
        match = FIGURE.match(caption)
        if match is None:
            continue
        for following in lines[index + 1:]:
            candidate = following.strip().lstrip(">").strip()
            if not candidate or candidate.startswith("<img ") or candidate.startswith(
                    "[Editorial description, not ICAO text:"):
                continue
            if PIPE_ROW.match(candidate):
                found.append(FigureTable(index + 1, match.group(1), match.group(2)))
            break
    return found


def word_tokens(text: str) -> list[str]:
    """Normalize case, whitespace and PDF line-wrap hyphens for prose matching."""
    text = WRAPPED_HYPHEN.sub("", text)
    return [match.group().casefold() for match in WORDS.finditer(text)]


def fivegrams(text: str) -> list[tuple[str, ...]]:
    words = word_tokens(text)
    return [tuple(words[i:i + 5]) for i in range(max(0, len(words) - 4))]


def prose_score(paragraph: str, pdf_grams: set[tuple[str, ...]]) -> float:
    grams = fivegrams(paragraph)
    return sum(gram in pdf_grams for gram in grams) / len(grams) if grams else 1.0


def paragraphs(markdown: str) -> list[tuple[int, str]]:
    """Return prose blocks and source line numbers, omitting structure and editorial text."""
    found = []
    current = []
    start = 0
    fenced = False

    def flush():
        nonlocal current
        if current:
            text = "\n".join(current)
            if (len(word_tokens(text)) >= 12
                    and "[Editorial description, not ICAO text:" not in text):
                found.append((start, text))
            current = []

    for number, line in enumerate(markdown.splitlines(), 1):
        stripped = line.strip()
        if stripped.startswith("```"):
            flush()
            fenced = not fenced
            continue
        structural = (not stripped or fenced or stripped.startswith(("#", ">", "|", "-", "*", "<"))
                      or stripped.startswith("![") or stripped.startswith("[Editorial description"))
        if structural:
            flush()
            continue
        if not current:
            start = number
        current.append(line)
    flush()
    return found


def caption_page(caption: FigureTable, page_texts: list[str]) -> int | None:
    """Locate the PDF page carrying a caption by number and title words."""
    title_words = word_tokens(caption.title)
    if not title_words:
        return None
    prefix = title_words[:min(4, len(title_words))]
    for index, text in enumerate(page_texts):
        normalized = word_tokens(text)
        figure_positions = [i for i in range(len(normalized) - 1)
                            if normalized[i:i + 2] == ["figure", caption.number]]
        if any(normalized[i + 2:i + 2 + len(prefix)] == prefix for i in figure_positions):
            return index
    return None


def page_lines_and_title(doc, part: int) -> tuple[list[list[str]], str]:
    pages = [extract._page_lines(page) for page in doc]
    for line in pages[0]:
        parsed = extract.parse_part_title_line(line)
        if parsed is not None and parsed[0] == part:
            return pages, parsed[1]
    raise ValueError(f"Part {part}: PDF title page has no matching Part title")


def audit_part(part: int, pdf_dir: Path) -> tuple[list[str], bool]:
    pdf = pdf_dir / f"9303_p{part}_cons_en.pdf"
    matches = sorted(DOCS.glob(f"Doc_9303_Part{part}_*.md"))
    if len(matches) != 1:
        return [f"Part {part}: expected one Markdown file, found {len(matches)}"], True
    if not pdf.is_file():
        return [f"Part {part}: missing PDF {pdf.name}"], True
    if extract.fitz is None:
        return [f"Part {part}: PyMuPDF (fitz) is required"], True

    markdown = matches[0].read_text(encoding="utf-8")
    lines = [f"Part {part}: {matches[0].name}"]
    bad = False
    with extract.fitz.open(pdf) as doc:
        pages, title = page_lines_and_title(doc, part)
        page_texts = ["\n".join(page) for page in pages]
        pdf_text = "\n".join(page_texts)
        pdf_digits = set(DIGITS.findall(pdf_text))
        absent = [(line, token) for line, token in digit_tokens(markdown)
                  if token not in pdf_digits]
        lines.append(f"  invented numbers ({len(absent)}):")
        lines.extend(f"    line {line}: {token}" for line, token in absent)
        bad |= bool(absent)

        body_start = extract.find_body_start_page(page_texts)
        verso = "Machine Readable Travel Documents"
        recto = extract.recto_header_for(part, title)
        body_lines = [line for page in pages[body_start:]
                      for line in extract.strip_furniture(page, verso, recto)]
        missing, extra = heading_diff(body_lines, markdown)
        lines.append(f"  clause headings missing from Markdown ({len(missing)}):")
        lines.extend(f"    {heading}" for heading in missing)
        lines.append(f"  clause headings absent from PDF ({len(extra)}):")
        lines.extend(f"    {heading}" for heading in extra)
        bad |= bool(missing or extra)

        # Independent of the check above: see `number_alone_candidates`'s
        # own docstring for why this doesn't reuse `extract.detect_heading`.
        markdown_numbers = set(markdown_headings(markdown))
        unresolved_alone = [
            (line, number, title)
            for line, number, title in number_alone_candidates(body_lines)
            if number not in markdown_numbers
        ]
        lines.append(
            f"  number-alone-then-title PDF lines with no matching Markdown heading "
            f"({len(unresolved_alone)}):"
        )
        lines.extend(
            f"    body-line {line}: {number} | {title[:60]}"
            for line, number, title in unresolved_alone
        )
        bad |= bool(unresolved_alone)

        table_findings = []
        for caption in figure_tables(markdown):
            page_index = caption_page(caption, page_texts)
            if page_index is None:
                table_findings.append(f"    line {caption.line}: Figure {caption.number} caption not located in PDF")
            elif not doc[page_index].find_tables().tables:
                table_findings.append(
                    f"    line {caption.line}: Figure {caption.number}, PDF page {page_index + 1} has no table")
        lines.append(f"  tables under figures ({len(table_findings)}):")
        lines.extend(table_findings)
        bad |= bool(table_findings)

        pdf_grams = set(fivegrams(pdf_text))
        low = [(line, prose_score(text, pdf_grams)) for line, text in paragraphs(markdown)]
        low = [(line, score) for line, score in low if score < 0.8]
        lines.append(f"  prose paragraphs below 0.8 ({len(low)}):")
        lines.extend(f"    line {line}: {score:.3f}" for line, score in low)
        bad |= bool(low)
    return lines, bad


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pdf-dir", required=True, type=Path)
    parser.add_argument("--part", action="append", type=int, choices=range(1, 14))
    args = parser.parse_args(argv)
    parts = sorted(set(args.part or range(1, 14)))
    bad = False
    for part in parts:
        lines, failed = audit_part(part, args.pdf_dir)
        print("\n".join(lines))
        bad |= failed
    return int(bad)


if __name__ == "__main__":
    raise SystemExit(main())