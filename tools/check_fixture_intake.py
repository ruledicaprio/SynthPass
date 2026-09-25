"""Check fixture intake links and derived metadata without opening images."""

from collections import Counter, defaultdict
import json
from pathlib import Path, PurePosixPath
import re

import audit_fixture_checksums as checksum_audit


ROOT = Path(__file__).resolve().parents[1]
CHECK = "python tools/check_fixture_intake.py"
TRAITS_REGEN = "cargo run -p synthpass-ocr --example template_traits"
GOLDEN_BLESS = "MRZ_WIRE_GOLDEN_BLESS=1 cargo test -p synthpass-bench --test mrz_wire_golden"
IMAGE_SUFFIXES = {".jpg", ".jpeg", ".png", ".webp", ".gif", ".bmp", ".tif", ".tiff"}
TRAITS_EXEMPT: dict[str, str] = {
    "India_Passport_Specimen_P0_IND_2013_mrz_boxed.jpg":
        "printed line 1 carries no issuing state (cells 2-4 are `<SP`), "
        "which template_traits refuses by design",
}


def images_present(samples_root):
    """Guard for future image-walking steps; the intake check never calls it.

    Return true only for a default-corpus image, excluding private and
    metadata directories. A future image walker must refuse to run if false.
    """
    if not samples_root.is_dir():
        return False
    for path in samples_root.rglob("*"):
        if not path.is_file() or path.suffix.lower() not in IMAGE_SUFFIXES:
            continue
        parts = {part.lower() for part in path.relative_to(samples_root).parts[:-1]}
        if not parts.intersection({"private", "covers", "local", "ocr_fixtures"}):
            return True
    return False


def add(problems, path, detail, command):
    problems.append(f"{path}: {detail}; fix: {command}")


def read_jsonl(root, path, problems, command):
    label = path.relative_to(root).as_posix()
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeError):
        add(problems, label, "missing or unreadable JSONL", command)
        return []
    rows = []
    for number, line in enumerate(lines, 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            add(problems, f"{label}:{number}", "malformed JSON", command)
            continue
        if not isinstance(row, dict):
            add(problems, f"{label}:{number}", "row is not an object", command)
            continue
        rows.append((number, row))
    return rows


def private_component(value):
    if not isinstance(value, str):
        return False
    return "private" in {part.lower() for part in PurePosixPath(value.replace("\\", "/")).parts}


def safe_component(value):
    return (isinstance(value, str) and bool(value) and value not in {".", ".."}
            and "/" not in value and "\\" not in value)


def pinned_trait_total(path):
    try:
        source = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError):
        return None
    match = re.search(r"const EXPECTED:.*?=\s*&\[(.*?)\];", source, re.DOTALL)
    if match is None:
        return None
    counts = re.findall(r'\(\s*"[^"]+"\s*,\s*(?:true|false)\s*,\s*(\d+)\s*\)',
                        match.group(1))
    return sum(map(int, counts)) if counts else None


def check(root=ROOT):
    """Return one actionable, zone-free problem line for each failed check."""
    problems = []
    samples = root / "samples"
    fixture_dir = samples / "ocr_fixtures"
    corpus_path = samples / "corpus.jsonl"
    traits_path = samples / "template_traits.jsonl"
    trait_test = root / "crates" / "synthpass-bench" / "tests" / "template_traits.rs"
    golden_path = root / "crates" / "synthpass-bench" / "tests" / "fixtures" / "mrz_wire_golden.jsonl"
    corpus_label = corpus_path.relative_to(root).as_posix()
    traits_label = traits_path.relative_to(root).as_posix()
    golden_label = golden_path.relative_to(root).as_posix()

    manifest = read_jsonl(root, corpus_path, problems, CHECK)
    owners = defaultdict(list)
    linked_assets = Counter()
    for number, row in manifest:
        label = f"{corpus_label}:{number}"
        stem = row.get("ground_truth_stem")
        filename = row.get("filename")
        if any(private_component(row.get(key)) for key in
               ("dir", "filename", "ground_truth_stem")):
            add(problems, label, "manifest path enters samples/private", CHECK)
        if not isinstance(filename, str):
            add(problems, label, "missing asset filename", CHECK)
        else:
            mrz = row.get("mrz")
            redacted = mrz.get("redacted") if isinstance(mrz, dict) else None
            if type(redacted) is not bool or redacted != ("redacted" in filename.lower()):
                add(problems, label, "redaction flag disagrees with filename", CHECK)
        if stem is None:
            continue
        if not safe_component(stem):
            add(problems, label, "ground_truth_stem is not a root fixture stem", CHECK)
            continue
        owners[stem].append((number, row))
        fixture = fixture_dir / f"{stem}.json"
        if not fixture.is_file():
            add(problems, label, f"missing root fixture {fixture.name}", CHECK)
        elif isinstance(filename, str):
            linked_assets[filename] += 1

    try:
        root_fixtures = sorted(fixture_dir.glob("*.json"))
    except OSError:
        root_fixtures = []
    if not fixture_dir.is_dir():
        add(problems, "samples/ocr_fixtures", "fixture directory missing", CHECK)
    root_names = {fixture.name for fixture in root_fixtures}
    for fixture in root_fixtures:
        label = fixture.relative_to(root).as_posix()
        if fixture.resolve().is_relative_to((samples / "private").resolve()):
            add(problems, label, "fixture path enters samples/private", CHECK)
        count = len(owners[fixture.stem])
        if count != 1:
            add(problems, label, f"named by {count} manifest rows; expected 1", CHECK)

        audit, claim = checksum_audit.audit_fixture(fixture)
        if audit.anomaly:
            add(problems, label, f"checksum audit cannot verify fixture ({audit.anomaly})",
                "python tools/audit_fixture_checksums.py")
        elif audit.valid != claim:
            failing = ",".join(audit.failing) or "none"
            add(problems, label,
                f"checksum claim disagrees (format={audit.format}, failing={failing}, claim={claim})",
                "python tools/audit_fixture_checksums.py")
        if claim is False and count == 1 and owners[fixture.stem][0][1].get(
                "expected_document_number") is not None:
            add(problems, label, "nonconforming fixture has expected_document_number",
                CHECK)

    trait_rows = read_jsonl(root, traits_path, problems, TRAITS_REGEN)
    trait_assets = Counter()
    for number, row in trait_rows:
        source = row.get("source")
        if (not isinstance(source, list) or len(source) != 1
                or not isinstance(source[0], dict)
                or not isinstance(source[0].get("filename"), str)):
            add(problems, f"{traits_label}:{number}", "missing single source asset",
                TRAITS_REGEN)
            continue
        trait_assets[source[0]["filename"]] += 1
    for asset in sorted(TRAITS_EXEMPT):
        if linked_assets[asset] != 1:
            add(problems, corpus_label,
                f"stale trait exemption for {asset}: linked fixtures={linked_assets[asset]}",
                "remove stale TRAITS_EXEMPT entry, then run " + CHECK)
        if trait_assets[asset] != 0:
            add(problems, traits_label,
                f"stale trait exemption for {asset}: registry rows={trait_assets[asset]}",
                "remove stale TRAITS_EXEMPT entry, then run " + CHECK)
    for asset in sorted((linked_assets.keys() | trait_assets.keys()) - TRAITS_EXEMPT.keys()):
        if linked_assets[asset] != trait_assets[asset]:
            add(problems, traits_label,
                f"asset {asset}: rows={trait_assets[asset]}, linked fixtures={linked_assets[asset]}",
                TRAITS_REGEN)
    pinned = pinned_trait_total(trait_test)
    if pinned is None:
        add(problems, trait_test.relative_to(root).as_posix(),
            "cannot parse EXPECTED row-count pin", TRAITS_REGEN)
    elif len(trait_rows) != pinned:
        add(problems, traits_label,
            f"rows={len(trait_rows)}, EXPECTED pin={pinned}", TRAITS_REGEN)

    golden_rows = read_jsonl(root, golden_path, problems, GOLDEN_BLESS)
    golden_names = Counter()
    for number, row in golden_rows:
        name = row.get("fixture")
        label = f"{golden_label}:{number}"
        if not safe_component(name) and not (
                isinstance(name, str) and name.startswith("derived/")
                and len(PurePosixPath(name).parts) == 2
                and safe_component(PurePosixPath(name).name)):
            add(problems, label, "invalid fixture path", GOLDEN_BLESS)
            continue
        golden_names[name] += 1
        if not (fixture_dir / name).is_file():
            add(problems, label, f"fixture {name} does not exist", GOLDEN_BLESS)
    for name in sorted(root_names):
        if golden_names[name] != 1:
            add(problems, golden_label,
                f"root fixture {name}: rows={golden_names[name]}, expected 1", GOLDEN_BLESS)
    for name, count in sorted(golden_names.items()):
        if count > 1 and name not in root_names:
            add(problems, golden_label, f"fixture {name}: duplicate rows={count}", GOLDEN_BLESS)

    return problems


def main(root=ROOT):
    problems = check(root)
    if TRAITS_EXEMPT:
        print("exempt template_traits assets:")
        for asset in sorted(TRAITS_EXEMPT):
            print(f"  {asset}")
    for problem in problems:
        print(problem)
    return int(bool(problems))


if __name__ == "__main__":
    raise SystemExit(main())