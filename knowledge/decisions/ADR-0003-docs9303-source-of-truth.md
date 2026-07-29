# ADR-0003 — Adopt `knowledge/docs9303/` as the ICAO spec source of truth

**Status:** Accepted
**Date:** 2026-07-29

## Context

The MRZ parser (`crates/mrz`) and the M7 Document Intelligence Engine
(`synthpass-die`) already cite ICAO Doc 9303 precisely in their own doc
comments — down to `part N §M.M.M`, e.g. `checksum.rs`'s "ICAO 9303 part 3
§4.9: 7-3-1 repeating weights" or `mrz_reader.rs`'s "The ICAO 9303 Part 4 TD3
specimen." That precision has nowhere to point *from*: the closest thing to
a consolidated reference was `crates/mrz/README.md` (a crate-consumer
primer, not a spec) and the constitution's §10 "MRZ Standards" (project
policy on handling MRZ, not the spec itself). Neither is precise enough to
plan a redesign of the MRZ/document-intelligence architecture against.

Thirteen raw, machine-converted Markdown files for ICAO Doc 9303 Parts 1–13
had already landed in `knowledge/docs9303/`, untracked, ahead of that
redesign work. They were not usable as a source of truth as they stood:
three incompatible TOC/table styles across the 13 files, real extraction
damage (duplicated front matter and page-header bleed in Part 2, a leftover
LLM preamble in Part 9, an editorial note embedded in Part 13's own TOC),
zero real cross-file links despite 300+ prose mentions of other parts,
inconsistent figure handling, and no index tying any of it back to the code
that depends on it.

## Decision

Adopt `knowledge/docs9303/` as a first-class subject folder, following the
pattern `ADR-0001-knowledge-tree.md` established: a README stating what
belongs and what doesn't, indexed in `knowledge/README.md`'s subject-folder
table.

Concretely:

- **Standardize the corpus on one house style** — the cleanest of the three
  styles found in the raw files (matching the original Parts 10–13): plain
  numbered-list TOCs, simple 3-column AMENDMENTS tables, blockquote+bold
  figure captions (bold-only for table captions), consistent code fencing
  for MRZ/ASN.1/APDU examples. Fix the extraction damage rather than
  standardize around it.
- **Link cross-file citations at the granularity the source material
  actually supports.** The 13 parts cite each other only at whole-part
  granularity in prose (`Doc 9303-11`, `see Part 4`) — never a specific
  section number across parts. All 247 such mentions are now real Markdown
  links. Section-anchor-level linking (`part N §M.M.M` → a specific heading)
  is left as a documented convention in the README rather than forced into
  prose that doesn't contain that precision, because headings do carry their
  section number, so the anchors are derivable when a citation is ever
  written precisely enough to need one — most plausibly from a future Rust
  doc comment, not from these files citing each other.
- **Extract figures selectively, not exhaustively.** ~19 diagrams that are
  either load-bearing for `crates/mrz`'s field-offset logic (MRZ zone-layout
  and dimensional diagrams, Parts 4–7) or were previously unreadable
  (Part 2 Appendix C's diagrams, OCR'd into run-on prose before this
  cleanup) were rendered from the source PDFs and wired in with alt text.
  Diagrams already well-described in the existing bracketed placeholder
  text, or purely decorative, were left as text.

## Alternatives rejected

**Ship the raw converted files as-is, cross-link later.** Rejected: the
three incompatible TOC styles and the extraction damage (especially Part 2)
would have made the corpus actively misleading as a "source of truth" —
worse than not having one, since a reader has no signal that a section is
damaged versus normative.

**Link every "Part N" mention regardless of the source's own citation
granularity**, i.e. attempt to force section-anchor precision everywhere.
Rejected as fabricating precision the ICAO text doesn't have; the source
documents simply don't cite each other at that granularity, and pretending
otherwise would mean guessing which section a bare "Part 4" reference
actually meant.

**Extract every figure in every part.** Rejected as disproportionate effort
for figures with no clear bearing on the redesign — many bracketed
placeholders already carry a full prose description of the diagram, written
against the source PDF, which serves a model reader as well as an image
would.

## Consequences

**Positive**

- `crates/mrz` and `synthpass-die`'s existing `part N §M.M.M` doc-comment
  citations now have a real, precisely-anchored target to eventually link
  to, closing a gap `af09055` (the research pass behind this ADR) surfaced:
  the most spec-precise material in the repo was scattered across ~10 Rust
  files with no consolidated index.
- The redesign work this corpus exists to support can cite ICAO text
  directly and verifiably, rather than through the crate README's
  paraphrase or the constitution's policy language.

**Negative**

- A cleanup script bug (line-count arithmetic, not a research error) briefly
  clipped real content from several files' endings during this work —
  caught only by diffing every file's tail against its source PDF. Recorded
  here as a reason the corpus should be spot-checked against source again
  before being treated as fully authoritative, not just trusted because it
  looks clean.
- Part 8's Visible Digital Seal byte-dump example remains ~13 bytes short of
  the 134 the source PDF states; the source's two-column hex layout doesn't
  extract into a reliable byte order from text alone. Documented in place
  rather than guessed at — tracked here, not silently left for the next
  reader to discover.
- Section-anchor-level cross-references are a documented *convention*, not
  yet exercised by any real link. If a future pass wires Rust doc comments
  to `knowledge/docs9303/...#anchor` targets, `scripts/check-doc-links.sh`
  will need those paths to resolve — anchors were derived from heading text
  but not validated against GitHub's actual rendering.
