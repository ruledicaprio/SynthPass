"""Audit clean-checkout default-walk candidates, without decoding or OCR.

Default: committed HEAD metadata/fixtures plus the samples_data_sha in HEAD's
baseline. --data-ref explicitly selects another data revision. This is not an
inventory of untracked/local files, nor a reconstruction of successful report rows.
"""
import argparse
from collections import defaultdict
import hashlib
import json
from pathlib import Path, PurePosixPath
import subprocess

BASELINE_PATH = "knowledge/benchmarks/real-specimen-mrz-baseline.json"
# Equivalence to the actual Rust find_image_files function is tested by compiling
# that function directly in test_audit_benchmark_identity.py (no OCR dependencies).
IMAGE_EXTENSIONS = {"jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff"}
SYNC_DIRS = {"passports", "id_cards", "driving_licenses", "misc", "covers", "ocr_fixtures"}


def git(*args):
    return subprocess.check_output(["git", *args])


def ascii_lower(value):
    return value.translate(str.maketrans("ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz"))


def included(path):
    """Rust default-walk predicate for a regular file relative to samples_root."""
    p = PurePosixPath(path)
    return (p.suffix.removeprefix(".").lower() in IMAGE_EXTENSIONS
            and not any("private" in ascii_lower(c) for c in p.parts)
            and not any(ascii_lower(c) in {"covers", "local"} for c in p.parts[:-1]))


def groups(rows, key):
    result = defaultdict(list)
    for row in rows:
        result[row[key]].append(row["asset_id"])
    return {k: sorted(v) for k, v in sorted(result.items()) if len(v) > 1}


def tree(ref):
    result = {}
    if ref == "WORKTREE":
        records = git("ls-files", "-s", "-z", "--", "samples/")
        for record in records.split(b"\0"):
            if not record:
                continue
            meta, path = record.split(b"\t", 1)
            mode, blob, _stage = meta.decode().split()
            result[path.decode("utf-8").removeprefix("samples/")] = (mode, "blob", blob)
        return result
    for record in git("ls-tree", "-rz", ref, "--", "samples/").split(b"\0"):
        if not record:
            continue
        meta, path = record.split(b"\t", 1)
        mode, kind, blob = meta.decode().split()
        result[path.decode("utf-8").removeprefix("samples/")] = (mode, kind, blob)
    return result


def merge_asset(assets, asset_id, digest, source):
    if asset_id in assets:
        if assets[asset_id]["sha256"] != digest:
            raise ValueError(f"main/samples-data byte conflict: {asset_id}")
        assets[asset_id]["sources"].append(source)
    else:
        assets[asset_id] = dict(asset_id=asset_id, stem=PurePosixPath(asset_id).stem,
                                sha256=digest, sources=[source])


def resolve_data_ref(data_ref, baseline):
    selected = data_ref if data_ref is not None else baseline["samples_data_sha"]
    if data_ref is None and (len(selected) != 40 or any(c not in "0123456789abcdef" for c in selected)):
        raise ValueError("baseline lacks a pinned samples_data_sha; provide --data-ref explicitly")
    # No implicit fetch or fallback to a moving remote ref if the object is absent.
    return git("rev-parse", "--verify", selected + "^{commit}").decode().strip()


def audit(data_ref=None, working_tree=False):
    main_sha = git("rev-parse", "--verify", "HEAD^{commit}").decode().strip()
    baseline_bytes = git("show", f"{main_sha}:{BASELINE_PATH}")
    baseline = json.loads(baseline_bytes)
    sha = resolve_data_ref(data_ref, baseline)
    main_tree = tree("WORKTREE" if working_tree else main_sha)
    assets = {}
    unsynced = []
    for source, entries in (("samples-data", tree(sha)), ("main", main_tree)):
        for asset_id, (mode, kind, blob) in sorted(entries.items()):
            if not included(asset_id):
                continue
            if source == "samples-data" and PurePosixPath(asset_id).parts[0] not in SYNC_DIRS:
                unsynced.append(asset_id)
                continue
            if kind != "blob" or mode not in {"100644", "100755"}:
                raise ValueError(f"cannot model non-regular image entry: {source}:{asset_id}")
            digest = hashlib.sha256(git("cat-file", "blob", blob)).hexdigest()
            merge_asset(assets, asset_id, digest, source)
    manifest_bytes = (Path("samples/corpus.jsonl").read_bytes() if working_tree else git("show", f"{main_sha}:samples/corpus.jsonl"))
    manifest = {}
    for line in manifest_bytes.decode("utf-8").splitlines():
        row = json.loads(line)
        asset_id = row["dir"] + "/" + row["filename"]
        if included(asset_id):
            if asset_id in manifest:
                raise ValueError(f"duplicate manifest asset: {asset_id}")
            manifest[asset_id] = row
    rows = sorted(assets.values(), key=lambda r: r["asset_id"])
    for row in rows:
        m = manifest.get(row["asset_id"], {})
        row["manifest_sha_matches"] = m.get("sha256") == row["sha256"]
        row["ground_truth_stem"] = m.get("ground_truth_stem")
        row["loader_label_exists"] = "ocr_fixtures/" + row["stem"] + ".json" in main_tree
    return dict(population="clean-checkout default-walk candidates; before decode/preparation",
                main_sha=main_sha, metadata_source="working tree (staged assets/manifest)" if working_tree else "committed main_sha (not working tree)",
                baseline_path=BASELINE_PATH, baseline_sha256=hashlib.sha256(baseline_bytes).hexdigest(),
                baseline_measured_on_ci_sha=baseline["measured_on_ci_sha"],
                baseline_samples_data_sha=baseline["samples_data_sha"],
                data_ref_selection="explicit override" if data_ref is not None else "committed baseline",
                requested_data_ref=data_ref, samples_data_sha=sha,
                manifest_sha256=hashlib.sha256(manifest_bytes).hexdigest(),
                candidate_assets=len(rows), baseline_reported_documents=baseline["documents"],
                manifest_assets=len(manifest), unsynced_data_assets=sorted(unsynced),
                missing_assets=sorted(manifest.keys() - assets.keys()),
                unlisted_assets=sorted(assets.keys() - manifest.keys()),
                hash_mismatches=[r["asset_id"] for r in rows if r["asset_id"] in manifest and not r["manifest_sha_matches"]],
                duplicate_stems=groups(rows, "stem"), duplicate_sha256=groups(rows, "sha256"),
                assets=rows)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-ref", help="override committed baseline samples_data_sha")
    parser.add_argument("--working-tree", action="store_true", help="read staged MAIN samples/ assets and manifest")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    result = json.dumps(audit(args.data_ref, args.working_tree), indent=2) + "\n"
    if args.output:
        args.output.write_text(result, encoding="utf-8")
    else:
        print(result, end="")
