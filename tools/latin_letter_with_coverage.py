"""Coverage audit: every Unicode `LATIN (CAPITAL|SMALL) LETTER ... WITH ...`
character against ICAO Doc 9303 Part 3 section 6, Table A
(`crates/mrz/src/translit.rs`'s `TABLE_A`).

Usage: python tools/latin_letter_with_coverage.py [--out PATH]

Reads only two things: the Python standard library's own `unicodedata`
module (the Unicode Character Database compiled into the running
interpreter -- never a network fetch, and never the private ISO 10646
character-name list) and this repository's `crates/mrz/src/translit.rs`, to
learn which code points Table A already covers by parsing its own `TABLE_A`
array. It never keeps a second, hand-copied list of those 95 code points, so
this script cannot drift from the table it audits.

Doc 9303 Part 3 section 6, Table A ("Transliteration of Multinational
Latin-based Characters") lists 95 Latin-based national characters and a
recommended MRZ transliteration for each. This script classifies every
`LATIN ... WITH ...`-named character in the running interpreter's UCD into
three groups:

1. **direct row** -- the character is itself one of Table A's 95 keys.
2. **case-folded hit** -- not itself a Table A key, but
   `mrz::transliterate` upper-cases every input character before looking it
   up, and this character's single-character uppercase form *is* a Table A
   key (this is true of most Table A rows' lowercase counterparts, e.g.
   U+00E0 `a` with grave upper-cases to U+00C0, a Table A key). These are
   transliterated correctly today, just not through their own row.
3. **gap** -- neither of the above. `mrz::transliterate` does not drop
   these; it passes them through unchanged (upper-cased if they have a
   distinct uppercase form) -- confirmed for a sample by
   `crates/mrz/tests/table_a_iso10646_names.rs::gap_characters_pass_through_unchanged`.

This script does not change that behaviour; it only lists what Doc 9303 is
silent on. Doc 9303 itself is silent on every gap here -- it names no
recommended transliteration for any of them.

Run manually; this is not on CI's regenerate-and-diff path. The Unicode
Character Database version bundled with the Python interpreter varies
release to release, so two runs on two Python versions can produce
different gap *counts* against the same, unmodified `translit.rs` -- see
`tools/README.md`. `test_latin_letter_with_coverage.py` instead pins the
classification and rendering logic against a handful of code points old
enough (Unicode 1.1, 1993) to be stable across every UCD version Python has
ever shipped.
"""

from __future__ import annotations

import argparse
import re
import unicodedata
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TRANSLIT_RS = ROOT / "crates" / "mrz" / "src" / "translit.rs"
DEFAULT_OUT = ROOT / "knowledge" / "docs9303" / "TABLE_A_LATIN_WITH_COVERAGE.md"

TABLE_A_START = "const TABLE_A"
TABLE_A_KEY = re.compile(r"\\u\{([0-9A-Fa-f]+)\}")
LATIN_WITH_NAME = re.compile(r"^LATIN (CAPITAL|SMALL) LETTER .* WITH .+$")


def table_a_codepoints(translit_rs_source: str) -> set[int]:
    """The code points `TABLE_A` in `translit_rs_source` maps, parsed from the
    array literal itself -- never a second, hand-maintained copy of the list."""
    start = translit_rs_source.index(TABLE_A_START)
    end = translit_rs_source.index("];", start)
    block = translit_rs_source[start:end]
    return {int(hexdigits, 16) for hexdigits in TABLE_A_KEY.findall(block)}


def is_latin_with_name(name: str) -> bool:
    return bool(LATIN_WITH_NAME.match(name))


def all_latin_with_codepoints() -> list[int]:
    """Every code point in the running interpreter's UCD named
    `LATIN (CAPITAL|SMALL) LETTER ... WITH ...`, ascending."""
    found = []
    for codepoint in range(0x110000):
        try:
            name = unicodedata.name(chr(codepoint))
        except ValueError:
            continue
        if is_latin_with_name(name):
            found.append(codepoint)
    return found


@dataclass(frozen=True)
class Classification:
    direct_rows: list[int]
    case_folded: list[int]
    gaps: list[int]


def classify(latin_with_codepoints: list[int], table_a: set[int]) -> Classification:
    """Sort `latin_with_codepoints` into the three groups this module's
    docstring describes. `table_a` is the set of code points Table A itself
    covers (`table_a_codepoints`'s return value)."""
    direct_rows = []
    case_folded = []
    gaps = []
    for codepoint in latin_with_codepoints:
        if codepoint in table_a:
            direct_rows.append(codepoint)
            continue
        upper = chr(codepoint).upper()
        if len(upper) == 1 and ord(upper) in table_a:
            case_folded.append(codepoint)
        else:
            gaps.append(codepoint)
    return Classification(direct_rows, case_folded, gaps)


def _row(codepoint: int) -> str:
    char = chr(codepoint)
    name = unicodedata.name(char)
    return f"| U+{codepoint:04X} | {char} | {name} |"


def render_markdown(classification: Classification, ucd_version: str, total: int) -> str:
    lines = [
        "# Table A `LATIN ... WITH ...` coverage",
        "",
        "Generated by `tools/latin_letter_with_coverage.py`. Do not hand-edit; "
        "regenerate instead. See that script's module docstring for method and "
        "scope, and `tools/README.md` for why CI does not regenerate-and-diff it.",
        "",
        f"Unicode Character Database version: **{ucd_version}** (the version "
        "bundled with the Python interpreter that generated this file -- expect "
        "the exact counts below to shift slightly on a different Python version; "
        "see the script docstring).",
        "",
        "Audits ICAO Doc 9303 Part 3 section 6, Table A (\"Transliteration of "
        "Multinational Latin-based Characters\") against every "
        "`LATIN (CAPITAL|SMALL) LETTER ... WITH ...` character the Unicode "
        "Character Database names, using `crates/mrz/src/translit.rs`'s own "
        "`TABLE_A` array (never a duplicated copy of it) as the source of what "
        "Table A covers.",
        "",
        f"- Total `LATIN ... WITH ...` characters: **{total}**",
        f"- Direct Table A row: **{len(classification.direct_rows)}**",
        "- Not a Table A row, but transliterated correctly today because "
        f"`mrz::transliterate` upper-cases before lookup and the upper-cased "
        f"form is a Table A row (\"case-folded hit\"): **{len(classification.case_folded)}**",
        f"- Gap -- Doc 9303 recommends no transliteration, and "
        "`mrz::transliterate` passes the character through unchanged "
        f"(upper-cased): **{len(classification.gaps)}**",
        "",
        "## Direct Table A rows",
        "",
        "`LATIN ... WITH ...` characters that are themselves one of Table A's "
        "95 keys.",
        "",
        "| Code point | Character | Unicode name |",
        "| --- | --- | --- |",
    ]
    lines.extend(_row(cp) for cp in classification.direct_rows)
    lines += [
        "",
        "## Case-folded hits",
        "",
        "Not themselves a Table A row, but `mrz::transliterate` upper-cases "
        "every input character before looking it up, and this character's "
        "upper-case form is a Table A row -- so today's behaviour already "
        "transliterates these correctly, just not through their own row.",
        "",
        "| Code point | Character | Unicode name |",
        "| --- | --- | --- |",
    ]
    lines.extend(_row(cp) for cp in classification.case_folded)
    lines += [
        "",
        "## Gaps",
        "",
        "Doc 9303 Part 3 section 6, Table A recommends no transliteration for "
        "any of these. `mrz::transliterate` does not drop them: it passes each "
        "one through unchanged (upper-cased, if it has a distinct upper-case "
        "form). This is a coverage listing, not a behaviour change -- a gap "
        "Doc 9303 is silent on stays a gap.",
        "",
        "| Code point | Character | Unicode name |",
        "| --- | --- | --- |",
    ]
    lines.extend(_row(cp) for cp in classification.gaps)
    lines.append("")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args(argv)

    source = TRANSLIT_RS.read_text(encoding="utf-8")
    table_a = table_a_codepoints(source)
    latin_with = all_latin_with_codepoints()
    classification = classify(latin_with, table_a)
    markdown = render_markdown(classification, unicodedata.unidata_version, len(latin_with))
    args.out.write_text(markdown, encoding="utf-8")
    print(f"wrote {args.out} ({len(latin_with)} characters, "
          f"{len(classification.gaps)} gaps, UCD {unicodedata.unidata_version})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
