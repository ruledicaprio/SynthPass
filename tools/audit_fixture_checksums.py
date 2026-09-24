"""Audit root MRZ fixtures against their printed ICAO check digits."""

import json
from dataclasses import dataclass
from pathlib import Path


FIXTURES = Path(__file__).resolve().parents[1] / "samples" / "ocr_fixtures"
WEIGHTS = (7, 3, 1)


@dataclass(frozen=True)
class Audit:
    format: str
    failing: tuple[str, ...] = ()
    anomaly: str | None = None

    @property
    def valid(self):
        return not self.failing and self.anomaly is None


def check_digit(field):
    """Return the ICAO 7-3-1 check digit for an MRZ field."""
    total = 0
    for index, char in enumerate(field):
        if char == "<":
            value = 0
        elif "0" <= char <= "9":
            value = ord(char) - ord("0")
        elif "A" <= char <= "Z":
            value = ord(char) - ord("A") + 10
        else:
            raise ValueError("invalid MRZ character")
        total += value * WEIGHTS[index % len(WEIGHTS)]
    return str(total % 10)


def format_of(lines):
    widths = tuple(map(len, lines))
    if widths == (30, 30, 30):
        return "TD1"
    if widths == (36, 36):
        return "MRV-B" if lines[0].startswith("V") else "TD2"
    if widths == (44, 44):
        return "MRV-A" if lines[0].startswith("V") else "TD3"
    return None


def long_document_check(number, optional):
    """Part 5/6 overflow: nine principal cells, remainder, digit, filler."""
    end = optional.find("<")
    if end < 2 or not optional[end - 1].isdigit():
        return None
    return check_digit(number + optional[:end - 1]) == optional[end - 1]


def audit_zone(zone):
    if not isinstance(zone, str):
        return Audit("unknown", anomaly="malformed")
    lines = zone.split("\n")
    format_name = format_of(lines)
    if format_name is None:
        return Audit("unknown", anomaly="malformed")
    if any(char not in "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ<" for line in lines for char in line):
        return Audit(format_name, anomaly="malformed")

    failures = []

    def verify(name, field, printed, *, filler_zero=False):
        expected = check_digit(field)
        if printed != expected and not (filler_zero and printed == "<" and expected == "0"):
            failures.append(name)

    if format_name == "TD1":
        line1, line2, _ = lines
        number, printed = line1[5:14], line1[14]
        if printed == "<":
            long_check = long_document_check(number, line1[15:30])
            if long_check is None:
                return Audit(format_name, anomaly="overflow_unchecked")
            if not long_check:
                failures.append("document_number")
        else:
            verify("document_number", number, printed)
        verify("birth", line2[0:6], line2[6])
        verify("expiry", line2[8:14], line2[14])
        verify("composite", line1[5:30] + line2[0:7] + line2[8:15] + line2[18:29], line2[29])
    else:
        line1, line2 = lines
        number, printed = line2[0:9], line2[9]
        if format_name == "TD2" and printed == "<":
            long_check = long_document_check(number, line2[28:35])
            if long_check is None:
                return Audit(format_name, anomaly="overflow_unchecked")
            if not long_check:
                failures.append("document_number")
        else:
            verify("document_number", number, printed)
        verify("birth", line2[13:19], line2[19])
        verify("expiry", line2[21:27], line2[27])
        if format_name == "TD3":
            personal = line2[28:42]
            verify("personal_number", personal, line2[42], filler_zero=set(personal) == {"<"})
            verify("composite", line2[0:10] + line2[13:20] + line2[21:43], line2[43])
        elif format_name == "TD2":
            verify("composite", line2[0:10] + line2[13:20] + line2[21:35], line2[35])
    return Audit(format_name, tuple(failures))


def audit_fixture(fixture):
    try:
        data = json.loads(fixture.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return Audit("unknown", anomaly="malformed"), None
    if not isinstance(data, dict) or type(data.get("mrz_checksums_valid")) is not bool:
        return Audit("unknown", anomaly="malformed"), None
    return audit_zone(data.get("mrz_line")), data["mrz_checksums_valid"]


def main():
    names = ("checked", "agreeing", "mismatched", "malformed", "overflow_unchecked")
    counts = {name: 0 for name in names}
    for fixture in sorted(FIXTURES.glob("*.json")):
        result, claim = audit_fixture(fixture)
        if result.anomaly:
            counts[result.anomaly] += 1
            print(f"{fixture.name}: format={result.format} failing={result.anomaly} claim={claim}")
            continue
        counts["checked"] += 1
        if result.valid == claim:
            counts["agreeing"] += 1
        else:
            counts["mismatched"] += 1
            failing = ",".join(result.failing) or "none"
            print(f"{fixture.name}: format={result.format} failing={failing} claim={claim}")
    print(" ".join(f"{name}={value}" for name, value in counts.items()))
    return int(counts["mismatched"] > 0)


if __name__ == "__main__":
    raise SystemExit(main())
