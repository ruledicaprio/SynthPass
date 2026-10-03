"""Exercise the public workflow step's failure evidence without running a corpus."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


class PublicDiffStep(unittest.TestCase):
    def test_exit_codes_preserve_evidence(self):
        workflow = (Path(__file__).resolve().parents[1] /
                    ".github/workflows/bench-ab.yml").read_text(encoding="utf-8")
        step = workflow.split("      - name: Diff public projections\n", 1)[1]
        body = step.split("        run: |\n", 1)[1].split("\n      - name:", 1)[0]
        script = "\n".join(line[10:] for line in body.splitlines())
        bash = shutil.which("bash")
        self.assertIsNotNone(bash, "Git Bash is required")
        for code in (0, 2, 3):
            with self.subTest(code=code), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                (root / "tools").mkdir()
                (root / "ab").mkdir()
                (root / "tools/bench_ab_args.py").write_text("print('corpus public')\n")
                (root / "tools/bench_ab_diff.py").write_text(
                    "import sys\n"
                    "print('{}' if '--json' in sys.argv else 'public evidence')\n"
                    f"sys.exit({code})\n")
                # Use the selected Python runtime; Git Bash may not have python3.
                import sys
                script_path = root / "step.sh"
                script_path.write_text(script.replace("python3 ", '"$TEST_PYTHON" '))
                env = os.environ | {"EXPECT_IDENTICAL": "true",
                                    "GITHUB_STEP_SUMMARY": "summary.txt",
                                    "TEST_PYTHON": sys.executable.replace("\\", "/")}
                result = subprocess.run([bash, "-e", "-o", "pipefail", "step.sh"],
                                        cwd=root, env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode, 0 if code == 0 else 1,
                                 result.stdout + result.stderr)
                self.assertEqual((root / "ab/public-diff.json").read_text().strip(), "{}")
                self.assertIn("public evidence", (root / "summary.txt").read_text())
                expected = {0: "A/A verdict: NEUTRAL", 2: "::error::bench_ab_diff --public refused (exit 2)",
                            3: "::error::NOT NEUTRAL"}[code]
                self.assertIn(expected, result.stdout)
