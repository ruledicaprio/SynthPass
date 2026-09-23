#!/usr/bin/env python3
"""
Per-glyph geometry of the vendored OCR-B font over the MRZ alphabet, read from
the font's own outlines: no rendering, no model, no OCR.

What it measures
----------------
For every MRZ character (A-Z, 0-9, `<`): the advance width, the ink bounding
box, the number of contours and of holes, the ink area, the share of the
outline that is straight rather than curved, and ink "runs" (how many strokes a
line crosses, and how much of it is ink) along five horizontal and five
vertical scanlines through the glyph's ink box. Then:

- **classes**: letters vs digits vs the filler, and the gap between the two
  height classes (in this font every digit is taller than every letter);
- **pairs**: each confusable pair from `mrz::CONFUSABLES`
  (`crates/mrz/src/repair.rs`, parsed, not copied) plus the filler pairs
  chargrid meets, ranked by which features separate them;
- **chargrid**: the ink share a perfectly printed glyph leaves in one grid
  cell, next to chargrid's `DEFAULT_INK_FLOOR`
  (`crates/synthpass-ocr/src/chargrid.rs`, parsed), which that constant's own
  doc comment records as "not a probe-measured value".

What it does not measure
------------------------
`crates/synthpass-gen/fonts/ocr-b.ttf` is an OCR-B-*style* font (Raisty 2019,
OFL), the one `synthpass-gen` renders synthetic MRZs with. Issuers print their
own OCR-B cuts, and print, blur, perspective and laminate glare all erode small
geometric differences. So every number here describes the synthetic corpus's
glyphs exactly, and real documents only as a hypothesis to measure on real
crops. The synthetic corpus has already been measured not to predict real
specimens for a repair class, so treat a separation found here as a candidate,
never as evidence.

Stdlib only (the CI tools job installs nothing): the TrueType tables are read
with `struct`. `tools/test_ocrb_metrics.py` cross-checks the decoder against
fontTools when that happens to be installed.

Usage:
    python tools/ocrb_metrics.py            # human-readable report
    python tools/ocrb_metrics.py --json     # the same data, machine-readable
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import statistics
import struct
import sys
from dataclasses import dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_FONT = REPO_ROOT / "crates" / "synthpass-gen" / "fonts" / "ocr-b.ttf"
REPAIR_RS = REPO_ROOT / "crates" / "mrz" / "src" / "repair.rs"
CHARGRID_RS = REPO_ROOT / "crates" / "synthpass-ocr" / "src" / "chargrid.rs"

MRZ_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789<"

# Pairs chargrid meets that `CONFUSABLES` does not list, because the MRZ
# repair never substitutes the filler: these are recognizer confusions, not
# check-digit repairs. The provenance is the 2026-09-18 hand count on the Hong
# Kong 2007 specimen: 26 of 29 line-1 errors were `<` read as `K`.
EXTRA_PAIRS: list[tuple[str, str, str]] = [
    ("<", "K", "filler read as K: 26 of 29 line-1 errors, Hong Kong 2007"),
    ("<", "C", "filler read as a round letter"),
]

# Scanline positions, as fractions of the glyph's own ink box.
PROFILE_FRACTIONS = (0.1, 0.3, 0.5, 0.7, 0.9)

# Line pieces per quadratic Bezier when flattening an outline. Areas and
# lengths converge well before this; it only has to be fixed, for determinism.
QUAD_STEPS = 16


# --------------------------------------------------------------------------
# TrueType decoding (the subset this font uses: glyf outlines, cmap format 4)
# --------------------------------------------------------------------------


class FontError(ValueError):
    """The file is not a TrueType font this decoder can read."""


@dataclass
class RawGlyph:
    advance: int
    header_bbox: tuple[int, int, int, int]  # xMin, yMin, xMax, yMax from glyf
    contours: list[list[tuple[int, int, bool]]]  # (x, y, on_curve)


@dataclass
class Font:
    units_per_em: int
    glyphs: dict[str, RawGlyph]
    sha256: str
    path: str


def _tables(data: bytes) -> dict[str, tuple[int, int]]:
    if len(data) < 12:
        raise FontError("file too short for an sfnt header")
    (version, num_tables) = struct.unpack_from(">IH", data, 0)
    if version not in (0x00010000, 0x74727565):  # 1.0 or 'true'
        raise FontError(f"not a TrueType-outline font (sfnt version {version:#x})")
    out = {}
    for i in range(num_tables):
        tag, _checksum, offset, length = struct.unpack_from(">4sIII", data, 12 + 16 * i)
        out[tag.decode("latin-1")] = (offset, length)
    for needed in ("head", "maxp", "hhea", "hmtx", "loca", "glyf", "cmap"):
        if needed not in out:
            raise FontError(f"missing required table {needed!r}")
    return out


def _cmap_format4(data: bytes, cmap_off: int) -> dict[int, int]:
    """Code point -> glyph id, from the first Unicode BMP format-4 subtable."""
    (_version, n) = struct.unpack_from(">HH", data, cmap_off)
    records = [struct.unpack_from(">HHI", data, cmap_off + 4 + 8 * i) for i in range(n)]
    # Windows Unicode BMP first, then Unicode platform.
    records.sort(key=lambda r: {(3, 1): 0, (0, 3): 1}.get((r[0], r[1]), 2))
    for platform, encoding, sub_off in records:
        if (platform, encoding) not in ((3, 1), (0, 3)):
            continue
        base = cmap_off + sub_off
        if struct.unpack_from(">H", data, base)[0] != 4:
            continue
        seg_x2 = struct.unpack_from(">H", data, base + 6)[0]
        seg = seg_x2 // 2
        ends_off = base + 14
        starts_off = ends_off + seg_x2 + 2  # + reservedPad
        deltas_off = starts_off + seg_x2
        ranges_off = deltas_off + seg_x2
        mapping: dict[int, int] = {}
        for s in range(seg):
            end = struct.unpack_from(">H", data, ends_off + 2 * s)[0]
            start = struct.unpack_from(">H", data, starts_off + 2 * s)[0]
            delta = struct.unpack_from(">h", data, deltas_off + 2 * s)[0]
            range_addr = ranges_off + 2 * s
            range_off = struct.unpack_from(">H", data, range_addr)[0]
            for code in range(start, end + 1):
                if code == 0xFFFF:
                    continue
                if range_off == 0:
                    gid = (code + delta) & 0xFFFF
                else:
                    gid = struct.unpack_from(">H", data, range_addr + range_off + 2 * (code - start))[0]
                    if gid != 0:
                        gid = (gid + delta) & 0xFFFF
                if gid != 0:
                    mapping[code] = gid
        return mapping
    raise FontError("no Unicode BMP format-4 cmap subtable")


def _decode_glyph(data: bytes, off: int, length: int) -> tuple[tuple[int, int, int, int], list]:
    if length == 0:
        return (0, 0, 0, 0), []
    n_contours, x_min, y_min, x_max, y_max = struct.unpack_from(">hhhhh", data, off)
    if n_contours < 0:
        raise FontError("composite glyph: not needed for this font, so not decoded")
    p = off + 10
    ends = list(struct.unpack_from(f">{n_contours}H", data, p))
    p += 2 * n_contours
    n_points = ends[-1] + 1 if ends else 0
    instr_len = struct.unpack_from(">H", data, p)[0]
    p += 2 + instr_len

    flags: list[int] = []
    while len(flags) < n_points:
        f = data[p]
        p += 1
        flags.append(f)
        if f & 0x08:  # REPEAT_FLAG
            repeat = data[p]
            p += 1
            flags.extend([f] * repeat)
    flags = flags[:n_points]

    def coords(short_bit: int, same_bit: int) -> list[int]:
        nonlocal p
        out, value = [], 0
        for f in flags:
            if f & short_bit:
                d = data[p]
                p += 1
                value += d if f & same_bit else -d
            elif not f & same_bit:
                value += struct.unpack_from(">h", data, p)[0]
                p += 2
            out.append(value)
        return out

    xs = coords(0x02, 0x10)
    ys = coords(0x04, 0x20)
    contours, start = [], 0
    for end in ends:
        contours.append([(xs[i], ys[i], bool(flags[i] & 0x01)) for i in range(start, end + 1)])
        start = end + 1
    return (x_min, y_min, x_max, y_max), contours


def load_font(path: Path, chars: str = MRZ_ALPHABET) -> Font:
    data = path.read_bytes()
    t = _tables(data)
    head = t["head"][0]
    units_per_em = struct.unpack_from(">H", data, head + 18)[0]
    loc_format = struct.unpack_from(">h", data, head + 50)[0]
    num_glyphs = struct.unpack_from(">H", data, t["maxp"][0] + 4)[0]
    n_hmetrics = struct.unpack_from(">H", data, t["hhea"][0] + 34)[0]
    hmtx = t["hmtx"][0]
    loca = t["loca"][0]
    glyf = t["glyf"][0]
    cmap = _cmap_format4(data, t["cmap"][0])

    def loca_at(i: int) -> int:
        if loc_format == 0:
            return 2 * struct.unpack_from(">H", data, loca + 2 * i)[0]
        return struct.unpack_from(">I", data, loca + 4 * i)[0]

    glyphs: dict[str, RawGlyph] = {}
    for ch in chars:
        gid = cmap.get(ord(ch))
        if gid is None or gid >= num_glyphs:
            raise FontError(f"font has no glyph for {ch!r}")
        adv_index = min(gid, n_hmetrics - 1)
        advance = struct.unpack_from(">H", data, hmtx + 4 * adv_index)[0]
        start, end = loca_at(gid), loca_at(gid + 1)
        bbox, contours = _decode_glyph(data, glyf + start, end - start)
        glyphs[ch] = RawGlyph(advance, bbox, contours)
    return Font(units_per_em, glyphs, hashlib.sha256(data).hexdigest(), str(path))


# --------------------------------------------------------------------------
# Outline geometry
# --------------------------------------------------------------------------

Point = tuple[float, float]


def segments(contour: list[tuple[int, int, bool]]) -> list[tuple[str, tuple[Point, ...]]]:
    """A closed TrueType contour as ("line", (p0, p1)) and ("quad", (p0, c, p1))
    pieces, with the implied on-curve midpoint between consecutive off-curve
    points made explicit."""
    n = len(contour)
    if n == 0:
        return []
    pts = [(float(x), float(y), on) for x, y, on in contour]
    # Start on an on-curve point; if there is none, on the midpoint of the first two.
    first_on = next((i for i, p in enumerate(pts) if p[2]), None)
    if first_on is None:
        a, b = pts[0], pts[1 % n]
        start: Point = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
        order = pts[1:] + pts[:1]
    else:
        start = (pts[first_on][0], pts[first_on][1])
        order = pts[first_on + 1 :] + pts[: first_on + 1]
    out: list[tuple[str, tuple[Point, ...]]] = []
    cur, ctrl = start, None
    for x, y, on in order:
        if on:
            if ctrl is None:
                out.append(("line", (cur, (x, y))))
            else:
                out.append(("quad", (cur, ctrl, (x, y))))
                ctrl = None
            cur = (x, y)
        else:
            if ctrl is not None:
                mid = ((ctrl[0] + x) / 2, (ctrl[1] + y) / 2)
                out.append(("quad", (cur, ctrl, mid)))
                cur = mid
            ctrl = (x, y)
    if ctrl is not None:
        out.append(("quad", (cur, ctrl, start)))
    elif cur != start:
        out.append(("line", (cur, start)))
    return out


def flatten(contour: list[tuple[int, int, bool]]) -> tuple[list[Point], float, float]:
    """(polygon, straight outline length, curved outline length)."""
    poly: list[Point] = []
    straight = curved = 0.0
    for kind, ps in segments(contour):
        if kind == "line":
            p0, p1 = ps
            poly.append(p0)
            straight += math.dist(p0, p1)
        else:
            p0, c, p1 = ps
            prev = p0
            for k in range(QUAD_STEPS):
                t = k / QUAD_STEPS
                u = 1 - t
                q = (u * u * p0[0] + 2 * u * t * c[0] + t * t * p1[0],
                     u * u * p0[1] + 2 * u * t * c[1] + t * t * p1[1])
                poly.append(q)
                if k:
                    curved += math.dist(prev, q)
                prev = q
            curved += math.dist(prev, p1)
    return poly, straight, curved


def signed_area(poly: list[Point]) -> float:
    """Shoelace area: positive counter-clockwise, negative clockwise (y up)."""
    s = 0.0
    for i, (x0, y0) in enumerate(poly):
        x1, y1 = poly[(i + 1) % len(poly)]
        s += x0 * y1 - x1 * y0
    return s / 2


def crossings(polys: list[list[Point]], axis: int, at: float) -> list[float]:
    """Sorted positions where the line `coord[axis] == at` crosses the outline.
    Half-open vertex rule, so a line through a vertex counts it once."""
    other = 1 - axis
    hits = []
    for poly in polys:
        for i, a in enumerate(poly):
            b = poly[(i + 1) % len(poly)]
            if (a[axis] <= at) != (b[axis] <= at):
                t = (at - a[axis]) / (b[axis] - a[axis])
                hits.append(a[other] + t * (b[other] - a[other]))
    return sorted(hits)


def ink_runs(polys: list[list[Point]], axis: int, at: float) -> tuple[int, float]:
    """(number of ink runs, total ink length) along one scanline, by the
    even-odd pairing of sorted crossings. TrueType fills by
    non-zero winding, but this font's contours never overlap, so even-odd and
    non-zero agree; `test_ocrb_metrics` pins that by comparing the summed run
    area with the shoelace area."""
    xs = crossings(polys, axis, at)
    runs = [(xs[i], xs[i + 1]) for i in range(0, len(xs) - 1, 2)]
    return len(runs), sum(b - a for a, b in runs)


# --------------------------------------------------------------------------
# Features
# --------------------------------------------------------------------------


@dataclass
class Metrics:
    ch: str
    advance: int
    x_min: float
    y_min: float
    x_max: float
    y_max: float
    contours: int
    holes: int
    ink_area: float
    straight_fraction: float
    # per PROFILE_FRACTIONS: (runs, ink length as a fraction of the ink box side)
    rows: list[tuple[int, float]] = field(default_factory=list)
    cols: list[tuple[int, float]] = field(default_factory=list)

    @property
    def ink_w(self) -> float:
        return self.x_max - self.x_min

    @property
    def ink_h(self) -> float:
        return self.y_max - self.y_min


def measure(ch: str, g: RawGlyph) -> Metrics:
    polys, areas = [], []
    straight = curved = 0.0
    for contour in g.contours:
        poly, s, c = flatten(contour)
        polys.append(poly)
        areas.append(signed_area(poly))
        straight += s
        curved += c
    if not polys:  # a blank glyph: no ink at all
        return Metrics(ch, g.advance, 0, 0, 0, 0, 0, 0, 0.0, 0.0,
                       [(0, 0.0)] * len(PROFILE_FRACTIONS), [(0, 0.0)] * len(PROFILE_FRACTIONS))
    xs = [p[0] for poly in polys for p in poly]
    ys = [p[1] for poly in polys for p in poly]
    x_min, x_max, y_min, y_max = min(xs), max(xs), min(ys), max(ys)
    # The spec says outer contours run clockwise, but font editors disagree
    # (this font's run counter-clockwise). The largest contour is always an
    # outer one, so take its orientation as "ink" and count every contour
    # wound the other way as a hole, rather than assume contours - 1.
    outer_sign = 1.0 if max(areas, key=abs) > 0 else -1.0
    holes = sum(1 for a in areas if a * outer_sign < 0)
    ink_area = outer_sign * sum(areas)
    w, h = x_max - x_min, y_max - y_min
    rows = []
    for f in PROFILE_FRACTIONS:
        n, length = ink_runs(polys, 1, y_min + f * h)
        rows.append((n, length / w if w else 0.0))
    cols = []
    for f in PROFILE_FRACTIONS:
        n, length = ink_runs(polys, 0, x_min + f * w)
        cols.append((n, length / h if h else 0.0))
    total = straight + curved
    return Metrics(ch, g.advance, x_min, y_min, x_max, y_max, len(polys), holes, ink_area,
                   straight / total if total else 0.0, rows, cols)


def feature_vector(m: Metrics, upm: int) -> dict[str, float]:
    """Scalar features compared across pairs, lengths in em units so the
    numbers read the same for any units-per-em."""
    v = {
        "ink_h": m.ink_h / upm,
        "ink_w": m.ink_w / upm,
        "top": m.y_max / upm,
        "bottom": m.y_min / upm,
        "left": m.x_min / upm,
        "right": (m.advance - m.x_max) / upm,
        "holes": float(m.holes),
        "ink_area": m.ink_area / (upm * upm),
        "straight_fraction": m.straight_fraction,
    }
    for f, (n, cover) in zip(PROFILE_FRACTIONS, m.rows):
        v[f"row{int(f * 100)}_runs"] = float(n)
        v[f"row{int(f * 100)}_cover"] = cover
    for f, (n, cover) in zip(PROFILE_FRACTIONS, m.cols):
        v[f"col{int(f * 100)}_runs"] = float(n)
        v[f"col{int(f * 100)}_cover"] = cover
    return v


# --------------------------------------------------------------------------
# The two Rust constants this tool is read against
# --------------------------------------------------------------------------


def parse_confusables(rust_source: str) -> list[tuple[str, str]]:
    """`mrz::CONFUSABLES` rows, as written in repair.rs."""
    block = re.search(r"pub const CONFUSABLES[^=]*=\s*&\[(.*?)\];", rust_source, re.S)
    if not block:
        raise ValueError("CONFUSABLES not found")
    rows = re.findall(r"\('(.)',\s*\"([^\"]*)\"\)", block.group(1))
    if not rows:
        raise ValueError("CONFUSABLES has no rows")
    return rows


def confusable_pairs(rows: list[tuple[str, str]]) -> list[tuple[str, str]]:
    """Unordered pairs, first-seen order, no self pairs or duplicates."""
    out: list[tuple[str, str]] = []
    for key, group in rows:
        for other in group:
            pair = (key, other)
            if key != other and pair not in out and (other, key) not in out:
                out.append(pair)
    return out


def parse_ink_floor(rust_source: str) -> float:
    m = re.search(r"pub const DEFAULT_INK_FLOOR:\s*f32\s*=\s*([0-9.]+)\s*;", rust_source)
    if not m:
        raise ValueError("DEFAULT_INK_FLOOR not found")
    return float(m.group(1))


# --------------------------------------------------------------------------
# Analysis
# --------------------------------------------------------------------------


def classify(ch: str) -> str:
    if ch == "<":
        return "filler"
    return "digit" if ch.isdigit() else "letter"


def analyse(font: Font, pairs: list[tuple[str, str, str]], ink_floor: float | None) -> dict:
    upm = font.units_per_em
    metrics = {ch: measure(ch, g) for ch, g in font.glyphs.items()}
    vectors = {ch: feature_vector(m, upm) for ch, m in metrics.items()}
    advances = sorted({m.advance for m in metrics.values()})

    classes = {}
    for cls in ("letter", "digit", "filler"):
        ms = [m for m in metrics.values() if classify(m.ch) == cls]
        classes[cls] = {
            "n": len(ms),
            "ink_h": [min(m.ink_h for m in ms), max(m.ink_h for m in ms)],
            "ink_w": [min(m.ink_w for m in ms), max(m.ink_w for m in ms)],
            "top": [min(m.y_max for m in ms), max(m.y_max for m in ms)],
            "bottom": [min(m.y_min for m in ms), max(m.y_min for m in ms)],
        }
    tallest_letter = max((m for m in metrics.values() if classify(m.ch) == "letter"), key=lambda m: m.ink_h)
    shortest_digit = min((m for m in metrics.values() if classify(m.ch) == "digit"), key=lambda m: m.ink_h)
    height_gap = {
        "tallest_letter": tallest_letter.ch,
        "shortest_digit": shortest_digit.ch,
        "gap_units": shortest_digit.ink_h - tallest_letter.ink_h,
        "gap_fraction_of_letter": (shortest_digit.ink_h - tallest_letter.ink_h) / tallest_letter.ink_h,
    }

    # A feature's scale is its spread over the whole alphabet, so a difference
    # reads as "how unusual is this gap" rather than in raw units.
    names = list(next(iter(vectors.values())).keys())
    spread = {n: statistics.pstdev(v[n] for v in vectors.values()) for n in names}
    pair_rows = []
    for a, b, why in pairs:
        va, vb = vectors[a], vectors[b]
        scored = [
            {"feature": n, "a": va[n], "b": vb[n], "separation": abs(va[n] - vb[n]) / spread[n]}
            for n in names
            if spread[n] > 0
        ]
        scored.sort(key=lambda r: (-r["separation"], r["feature"]))
        pair_rows.append({
            "a": a,
            "b": b,
            "why": why,
            "same_height_class": classify(a) == classify(b) or "filler" in (classify(a), classify(b)),
            "top": scored[:4],
        })

    band_bottom = min(m.y_min for m in metrics.values())
    band_top = max(m.y_max for m in metrics.values())
    band = band_top - band_bottom
    # chargrid's cells all have one width (the pitch), so the share is taken
    # over that, not over each glyph's own advance.
    pitch = max(advances)
    cell_ink = {ch: m.ink_area / (pitch * band) for ch, m in metrics.items()}
    thinnest = min(cell_ink, key=lambda c: (cell_ink[c], c))
    filler = metrics["<"]
    # synthpass-gen centres each glyph's advance box in a fixed cell
    # (`draw_mrz_glyphs`, crates/synthpass-gen/src/render.rs), and issuers
    # centre glyphs in fixed cells too. So a glyph's left ink edge sits at
    # `x_min - advance / 2` from its cell's centre, whatever the cell width.
    # chargrid's `align` keys on exactly that left edge, so the spread of this
    # offset over the alphabet is jitter the grid fit has to absorb even on a
    # perfect print.
    left_from_centre = {ch: (m.x_min - m.advance / 2) / pitch for ch, m in metrics.items()}
    earliest = min(left_from_centre, key=lambda c: (left_from_centre[c], c))
    latest = max(left_from_centre, key=lambda c: (left_from_centre[c], c))
    chargrid = {
        "band": [band_bottom, band_top],
        "cell_ink": cell_ink,
        "filler_cell_ink": cell_ink["<"],
        "thinnest_glyph": thinnest,
        "thinnest_cell_ink": cell_ink[thinnest],
        "ink_floor": ink_floor,
        "filler_over_floor": (cell_ink["<"] / ink_floor) if ink_floor else None,
        "filler_ink_w_of_cell": filler.ink_w / filler.advance,
        "filler_ink_h_of_band": filler.ink_h / band,
        "left_edge_from_centre": left_from_centre,
        "left_edge_earliest": earliest,
        "left_edge_latest": latest,
        "left_edge_spread_of_pitch": left_from_centre[latest] - left_from_centre[earliest],
    }

    return {
        "font": {
            "path": font.path,
            "sha256": font.sha256,
            "units_per_em": upm,
            "advances": advances,
            "monospaced": len(advances) == 1,
            "advance_groups": {
                str(adv): "".join(ch for ch, m in metrics.items() if m.advance == adv) for adv in advances
            },
        },
        "glyphs": {
            ch: {
                "class": classify(ch),
                "advance": m.advance,
                "bbox": [m.x_min, m.y_min, m.x_max, m.y_max],
                "ink_w": m.ink_w,
                "ink_h": m.ink_h,
                "contours": m.contours,
                "holes": m.holes,
                "ink_area": m.ink_area,
                "straight_fraction": m.straight_fraction,
                "rows": m.rows,
                "cols": m.cols,
            }
            for ch, m in metrics.items()
        },
        "classes": classes,
        "height_gap": height_gap,
        "pairs": pair_rows,
        "chargrid": chargrid,
    }


# --------------------------------------------------------------------------
# Report
# --------------------------------------------------------------------------


def render(r: dict) -> str:
    f = r["font"]
    out = [
        f"font     {Path(f['path']).name}  sha256 {f['sha256'][:12]}  unitsPerEm {f['units_per_em']}",
        f"advance  monospaced: {f['monospaced']}",
        *(f"  {adv:>5}: {chars}" for adv, chars in f["advance_groups"].items()),
        "",
        "Glyph geometry (font units; rows/cols = ink runs at 10/30/50/70/90% of the ink box)",
        f"{'ch':<3}{'inkW':>6}{'inkH':>6}{'top':>6}{'bot':>5}{'holes':>6}{'area%':>7}{'line%':>7}  rows        cols",
    ]
    for ch, g in r["glyphs"].items():
        rows = "".join(str(n) for n, _ in g["rows"])
        cols = "".join(str(n) for n, _ in g["cols"])
        area = 100 * g["ink_area"] / (g["advance"] * f["units_per_em"])
        out.append(
            f"{ch:<3}{g['ink_w']:>6.0f}{g['ink_h']:>6.0f}{g['bbox'][3]:>6.0f}{g['bbox'][1]:>5.0f}"
            f"{g['holes']:>6}{area:>7.1f}{100 * g['straight_fraction']:>7.1f}  {rows:<11} {cols}"
        )
    out += ["", "Classes"]
    for cls, c in r["classes"].items():
        out.append(
            f"  {cls:<7} n={c['n']:<3} inkH {c['ink_h'][0]:.0f}..{c['ink_h'][1]:.0f}  "
            f"inkW {c['ink_w'][0]:.0f}..{c['ink_w'][1]:.0f}  top {c['top'][0]:.0f}..{c['top'][1]:.0f}"
        )
    g = r["height_gap"]
    verdict = "every digit is taller than every letter" if g["gap_units"] > 0 else "the classes overlap"
    out.append(
        f"  height gap: shortest digit {g['shortest_digit']!r} minus tallest letter "
        f"{g['tallest_letter']!r} = {g['gap_units']:.0f} units ({100 * g['gap_fraction_of_letter']:.1f}%): {verdict}"
    )
    out += ["", "Confusable pairs: the features that separate them (separation = |a-b| / alphabet spread)"]
    for p in r["pairs"]:
        note = "same height class" if p["same_height_class"] else "digit vs letter: height separates"
        out.append(f"  {p['a']} vs {p['b']}  ({note}; {p['why']})")
        for t in p["top"]:
            out.append(f"      {t['feature']:<18} {t['a']:>8.3f} vs {t['b']:<8.3f} sep {t['separation']:.2f}")
    c = r["chargrid"]
    out += [
        "",
        "Chargrid: a perfectly printed glyph's ink share of one grid cell (pitch x ink band)",
        f"  filler '<'      {100 * c['filler_cell_ink']:.1f}%  (its ink spans {100 * c['filler_ink_w_of_cell']:.0f}% "
        f"of the cell width, {100 * c['filler_ink_h_of_band']:.0f}% of the band height)",
        f"  thinnest glyph  {c['thinnest_glyph']!r} {100 * c['thinnest_cell_ink']:.1f}%",
    ]
    out.append(
        f"  left ink edge from the cell centre (glyph centred in a fixed cell): earliest "
        f"{c['left_edge_earliest']!r}, latest {c['left_edge_latest']!r}, spread "
        f"{100 * c['left_edge_spread_of_pitch']:.0f}% of a pitch: jitter chargrid's `align` sees on a perfect print"
    )
    if c["ink_floor"] is not None:
        out.append(
            f"  DEFAULT_INK_FLOOR {100 * c['ink_floor']:.1f}%: the ideal filler clears it {c['filler_over_floor']:.1f}x. "
            "A real band adds margin rows and loses thin strokes to blur, so this is a ceiling, not a measurement."
        )
    return "\n".join(out)


def build(font_path: Path, repo_root: Path = REPO_ROOT) -> dict:
    font = load_font(font_path)
    repair = repo_root / "crates" / "mrz" / "src" / "repair.rs"
    rows = parse_confusables(repair.read_text(encoding="utf-8"))
    pairs = [(a, b, "mrz::CONFUSABLES") for a, b in confusable_pairs(rows)]
    pairs += [p for p in EXTRA_PAIRS if (p[0], p[1]) not in {(a, b) for a, b, _ in pairs}]
    chargrid = repo_root / "crates" / "synthpass-ocr" / "src" / "chargrid.rs"
    floor = parse_ink_floor(chargrid.read_text(encoding="utf-8")) if chargrid.exists() else None
    return analyse(font, pairs, floor)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--font", type=Path, default=DEFAULT_FONT, help="TrueType file (default: the vendored OCR-B)")
    ap.add_argument("--json", action="store_true", help="print the full result as JSON")
    args = ap.parse_args(argv)
    try:
        result = build(args.font)
    except (OSError, FontError, ValueError) as e:
        print(f"ocrb_metrics: {e}", file=sys.stderr)
        return 1
    print(json.dumps(result, indent=2) if args.json else render(result))
    return 0


if __name__ == "__main__":
    sys.exit(main())
