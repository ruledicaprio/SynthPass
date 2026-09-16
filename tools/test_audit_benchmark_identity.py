"""Focused auditor tests. Run: python -m unittest discover -s tools -p test_audit_benchmark_identity.py"""
import json
from pathlib import Path
import re
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import audit_benchmark_identity as audit

ROOT = Path(__file__).resolve().parents[1]


class IdentityAuditTests(unittest.TestCase):
    def test_equal_bytes_preserve_both_sources(self):
        assets = {}
        audit.merge_asset(assets, "ocr_fixtures/same.gif", "abc", "samples-data")
        audit.merge_asset(assets, "ocr_fixtures/same.gif", "abc", "main")
        self.assertEqual(assets["ocr_fixtures/same.gif"]["sources"], ["samples-data", "main"])
        self.assertEqual(len(assets), 1)

    def test_conflicting_bytes_fail(self):
        assets = {}
        audit.merge_asset(assets, "ocr_fixtures/same.gif", "abc", "samples-data")
        with self.assertRaisesRegex(ValueError, "byte conflict"):
            audit.merge_asset(assets, "ocr_fixtures/same.gif", "def", "main")
        self.assertEqual(assets["ocr_fixtures/same.gif"]["sources"], ["samples-data"])

    def test_baseline_default_and_explicit_override(self):
        pinned = "a" * 40
        with patch.object(audit, "git", return_value=(pinned + "\n").encode()) as git:
            self.assertEqual(audit.resolve_data_ref(None, {"samples_data_sha": pinned}), pinned)
            git.assert_called_once_with("rev-parse", "--verify", pinned + "^{commit}")
            git.reset_mock()
            audit.resolve_data_ref("origin/samples-data", {"samples_data_sha": pinned})
            git.assert_called_once_with("rev-parse", "--verify", "origin/samples-data^{commit}")

    def test_unknown_pin_requires_explicit_override(self):
        with self.assertRaisesRegex(ValueError, "pinned"):
            audit.resolve_data_ref(None, {"samples_data_sha": "unknown"})

    def test_missing_pin_never_falls_back(self):
        with patch.object(audit, "git", side_effect=subprocess.CalledProcessError(128, "git")) as git:
            with self.assertRaises(subprocess.CalledProcessError):
                audit.resolve_data_ref(None, {"samples_data_sha": "a" * 40})
            self.assertEqual(git.call_count, 1)

    def test_tree_paths_are_nul_delimited(self):
        listing = b'100644 blob abc\tsamples/ocr_fixtures/a\tquote".gif\0'
        with patch.object(audit, "git", return_value=listing):
            self.assertEqual(audit.tree("ref"), {'ocr_fixtures/a\tquote".gif': ('100644', 'blob', 'abc')})

    def test_sync_directories_match_script(self):
        script = (ROOT / "scripts/sync-samples.ps1").read_text(encoding="utf-8")
        dirs = set(re.findall(r'"([a-z_]+)"', re.search(r'\$imageDirs = @\(([^)]*)\)', script)[1]))
        self.assertIn('samples\\ocr_fixtures', script)
        self.assertEqual(audit.SYNC_DIRS, dirs | {"ocr_fixtures"})

    def test_inclusion_matches_actual_rust_walk(self):
        # Compile the production function, constants and tracks struct verbatim.
        # This avoids testing a second hand-written interpretation of Rust rules,
        # and avoids building OCR/LLM dependencies just to exercise a file walk.
        source = (ROOT / "crates/synthpass-bench/src/lib.rs").read_text(encoding="utf-8")
        start = source.index("fn find_image_files(")
        walk = source[start:source.index("\n/// Reads", start)]
        constants = "\n".join(re.findall(r'(?:pub )?const (?:IMAGE_EXTENSIONS|LOCAL_TRACK_DIR|COVERS_TRACK_DIR):[^;]+;', source))
        start = source.index("pub struct OptInTracks {")
        tracks = source[start:source.index("\n}", start) + 2]
        self.assertRegex(source[:start], r'#\[derive\([^\]]*Default[^\]]*\)\]\s*$')
        program = '''use std::path::{Path, PathBuf};
''' + constants + '\n#[derive(Default, Clone, Copy)]\n' + tracks + '\n' + walk + '''
fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap());
    for path in find_image_files(&root, OptInTracks::default()) {
        println!("{}", path.strip_prefix(&root).unwrap().display().to_string().replace('\\\\', "/"));
    }
}
'''
        extensions = audit.IMAGE_EXTENSIONS | {"avif", "json", "md", "svg"}
        paths = [f"{directory}/{name}.{ext}"
                 for directory in ("passports", "ocr_fixtures", "ocr_fixtures/derived", "new/nested",
                                   "PRIVATE", "some_Private_track", "LOCAL", "nested/CoVeRs",
                                   "locality", "covers_extra", "prİvate")
                 for name in ("ordinary", "Private_holder", "local", "covers")
                 for ext in extensions | {e.upper() for e in extensions}]
        # Extensionless / dotfiles: Rust Path::extension agrees with PurePosixPath.
        paths += ["ordinary", ".gif", "nested/.jpg", "nested/name."]
        # Exercise every committed manifest path too (including the real GIFs).
        paths += [r["dir"] + "/" + r["filename"] for r in map(json.loads,
                  (ROOT / "samples/corpus.jsonl").read_text(encoding="utf-8").splitlines())]
        with tempfile.TemporaryDirectory(prefix="identity-walk-") as tmp:
            tmp = Path(tmp)
            root = tmp / "Private_parent" / "samples"  # root ancestors are not filtered
            root.mkdir(parents=True)
            for relative in paths:
                p = root / relative
                p.parent.mkdir(parents=True, exist_ok=True)
                p.touch()
            # A directory whose suffix looks like an image is not an image asset.
            (root / "directory.gif").mkdir()
            actual_paths = [p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file()]
            src = tmp / "walk.rs"
            src.write_text(program, encoding="utf-8")
            binary = tmp / "walk.exe"
            compiled = subprocess.run(["rustc", "--edition=2021", str(src), "-o", str(binary)], capture_output=True, text=True)
            self.assertEqual(compiled.returncode, 0, compiled.stderr)
            rust = subprocess.check_output([str(binary), str(root)], text=True, encoding="utf-8").splitlines()
            expected = [p for p in actual_paths if audit.included(p)]
            self.assertEqual(set(rust), set(expected))
            self.assertTrue(any(p.endswith(".gif") for p in rust))
            self.assertEqual(len(rust), len(expected))


if __name__ == "__main__":
    unittest.main()
