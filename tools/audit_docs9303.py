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


def pdf_headings(lines: list[str]) -> dict[str, str]:
    found = {}
    for line in lines:
        heading = extract.detect_heading(line)
        if heading is not None:
            found.setdefault(heading_key(heading), heading.anchor_text)
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