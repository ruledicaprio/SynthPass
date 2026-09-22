"""Offline tests for scripts/check-changelog.sh.

The regression under test: check 2 (and its check-3 mirror) decides "this PR
touches crates/ but nothing to flag" by grepping `git diff --name-only
$base...HEAD` for a non-Markdown crates/ path and taking the "documentation
only" branch whenever that grep finds nothing. An *empty* diff -- no
committed crates/ change at all -- makes that grep fail the exact same way a
genuinely Markdown-only diff does, so a bare local run against an uncommitted
change (exactly what CONTRIBUTING.md and CLAUDE.md tell a contributor to run
before opening a PR) printed "skipped: crates/ changes are documentation
only" for a change that was not documentation and was not even committed.
That confident, specific, wrong message is what got quoted in a PR body that
CI then failed.

The fix separates "no crates/ path in the diff" from "crates/ paths in the
diff are all Markdown", and adds a loud (non-fatal) warning whenever there is
an uncommitted or untracked change under crates/ or changelog.d/, since the
checks below only ever see commits.

Weighted toward the cases that must refuse or warn -- a permissive bug like
this one looks exactly like a passing test until the negative case is
checked directly.

Everything here runs against a throwaway git repository: no network, no
cargo, and no dependency on this repository's own pending changelog
fragments. Skips cleanly where bash or git is unavailable.
"""

import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SCRIPT = REPO / "scripts" / "check-changelog.sh"

BASH = shutil.which("bash")
GIT = shutil.which("git")


def posix(path) -> str:
    """Bash reads a backslash as an escape, so Windows paths go in as D:/... ."""
    return str(path).replace("\\", "/")


@unittest.skipUnless(BASH and GIT, "bash and git are required for check-changelog.sh")
class CheckChangelogTest(unittest.TestCase):
    """scripts/check-changelog.sh over a throwaway git repository.

    check-changelog.sh derives its own repo root from its script location
    (`cd "$(dirname "${BASH_SOURCE[0]}")/.."`) and diffs against that repo's
    history, so it cannot be pointed at a fixture tree with flags the way
    check-release-ready.sh can. Each test copies the script under test into a
    fresh throwaway repo's scripts/ directory and runs it from there.
    """

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="check-changelog-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)

        self.git("init", "-q", ".")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "user.name", "Test")

        (self.tmp / "scripts").mkdir()
        shutil.copyfile(SCRIPT, self.tmp / "scripts" / "check-changelog.sh")

        (self.tmp / "crates" / "mrz").mkdir(parents=True)
        (self.tmp / "changelog.d" / "mrz").mkdir(parents=True)

        (self.tmp / "Cargo.toml").write_text(
            '[workspace.package]\nversion = "1.0.0"\n', encoding="utf-8"
        )
        (self.tmp / "crates" / "mrz" / "Cargo.toml").write_text(
            '[package]\nname = "mrz"\nversion = "1.0.0"\n', encoding="utf-8"
        )
        (self.tmp / "crates" / "mrz" / "lib.rs").write_text("fn main() {}\n", encoding="utf-8")
        (self.tmp / "changelog.d" / "README.md").write_text("# fragments\n", encoding="utf-8")
        (self.tmp / "changelog.d" / "mrz" / "README.md").write_text(
            "# mrz fragments\n", encoding="utf-8"
        )

        self.git("add", "-A", ".", ":!scripts")
        self.git("commit", "-qm", "initial")
        self.git("branch", "-M", "main")

    # ------------------------------------------------------------- helpers

    def git(self, *args) -> str:
        r = subprocess.run(
            ["git", *args], cwd=str(self.tmp), capture_output=True, text=True
        )
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        return r.stdout.strip()

    def branch(self, name: str):
        self.git("checkout", "-qb", name, "main")

    def commit_all(self, message: str):
        self.git("add", "-A", ".", ":!scripts")
        self.git("commit", "-qm", message)

    def run_check(self, *args) -> subprocess.CompletedProcess:
        return subprocess.run(
            [BASH, posix(self.tmp / "scripts" / "check-changelog.sh"), *args],
            cwd=str(self.tmp),
            capture_output=True,
            text=True,
        )

    # --------------------------------------------------- the regression itself

    def test_empty_diff_reports_no_crates_changes_not_documentation_only(self):
        """The bug. main against itself must not claim a documentation-only diff."""
        r = self.run_check("--base", "main")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("skipped: no crates/ changes", r.stdout)
        self.assertNotIn("documentation only", r.stdout)
        self.assertIn("skipped: no crates/mrz/ changes", r.stdout)
        self.assertNotIn("test-only or documentation", r.stdout)

    def test_uncommitted_crates_change_warns_loudly_and_does_not_claim_docs_only(self):
        """The exact reproduction from the brief: append a non-md line, don't commit."""
        self.branch("feature-dirty")
        with open(self.tmp / "crates" / "mrz" / "lib.rs", "a", encoding="utf-8") as f:
            f.write("fn helper() {}\n")

        r = self.run_check("--base", "main")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        # The false, specific, confident claim must be gone.
        self.assertNotIn("documentation only", r.stdout)
        # A loud warning must replace it, naming the mechanism and the count.
        self.assertIn("WARNING", r.stderr)
        self.assertIn("1 uncommitted/untracked change", r.stderr)
        self.assertIn("crates/mrz/lib.rs", r.stderr)
        self.assertIn("do NOT see these changes", r.stderr)

    def test_untracked_file_under_changelog_d_also_warns(self):
        """The warning covers untracked files, not only modified tracked ones."""
        self.branch("feature-untracked")
        (self.tmp / "changelog.d" / "stray.added.md").write_text(
            "- stray.\n", encoding="utf-8"
        )
        r = self.run_check("--base", "main")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("WARNING", r.stderr)
        self.assertIn("changelog.d/stray.added.md", r.stderr)

    def test_clean_checkout_has_no_dirty_warning(self):
        """Positive control: an ordinary committed-only diff triggers no warning."""
        self.branch("feature-clean")
        with open(self.tmp / "crates" / "mrz" / "lib.rs", "a", encoding="utf-8") as f:
            f.write("fn helper() {}\n")
        (self.tmp / "changelog.d" / "mrz" / "feature-clean.added.md").write_text(
            "- add a helper.\n", encoding="utf-8"
        )
        self.commit_all("add helper with fragment")

        r = self.run_check("--base", "main")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertNotIn("WARNING", r.stderr)

    # ------------------------------------------------------- check 2 states

    def test_committed_non_md_crates_change_without_fragment_fails(self):
        self.branch("feature-no-fragment")
        with open(self.tmp / "crates" / "mrz" / "lib.rs", "a", encoding="utf-8") as f:
            f.write("fn helper2() {}\n")
        self.commit_all("touch mrz lib, no fragment")

        r = self.run_check("--base", "main")
        self.assertEqual(r.returncode, 1)
        self.assertIn("touches crates/ but adds no changelog.d/ fragment", r.stderr)

    def test_committed_non_md_crates_change_with_fragment_passes(self):
        self.branch("feature-with-fragment")
        with open(self.tmp / "crates" / "mrz" / "lib.rs", "a", encoding="utf-8") as f:
            f.write("fn helper3() {}\n")
        (self.tmp / "changelog.d" / "mrz" / "feature-with-fragment.added.md").write_text(
            "- add a helper function to mrz.\n", encoding="utf-8"
        )
        self.commit_all("add helper with fragment")

        r = self.run_check("--base", "main")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("ok: 1 fragment(s) added", r.stdout)
        self.assertIn("ok: mrz fragment present", r.stdout)

    def test_committed_markdown_only_crates_change_is_skipped_as_documentation(self):
        """Distinguish this from the empty-diff case: real changes exist, all .md."""
        self.branch("feature-doc-only")
        (self.tmp / "crates" / "mrz" / "NOTES.md").write_text(
            "crate-local documentation\n", encoding="utf-8"
        )
        with open(self.tmp / "changelog.d" / "mrz" / "README.md", "a", encoding="utf-8") as f:
            f.write("typo fix\n")
        self.commit_all("docs only")

        r = self.run_check("--base", "main")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("skipped: crates/ changes are documentation only", r.stdout)
        self.assertIn("skipped: mrz changes are test-only or documentation", r.stdout)

    def test_skip_changelog_label_short_circuits_both_checks(self):
        self.branch("feature-labelled")
        with open(self.tmp / "crates" / "mrz" / "lib.rs", "a", encoding="utf-8") as f:
            f.write("fn helper4() {}\n")
        self.commit_all("touch mrz lib, labelled")

        r = self.run_check("--base", "main", "--labels", "skip-changelog")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("skipped: 'skip-changelog' label present", r.stdout)

    # ------------------------------------------------------- check 3 states

    def test_committed_non_mrz_crates_change_reports_no_mrz_changes(self):
        """A crates/ change outside crates/mrz/ must not touch the mrz-scoped check."""
        self.branch("feature-non-mrz")
        (self.tmp / "crates" / "other").mkdir()
        (self.tmp / "crates" / "other" / "lib.rs").write_text("fn main() {}\n", encoding="utf-8")
        (self.tmp / "changelog.d" / "feature-non-mrz.added.md").write_text(
            "- add another crate.\n", encoding="utf-8"
        )
        self.commit_all("add unrelated crate")

        r = self.run_check("--base", "main")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("skipped: no crates/mrz/ changes", r.stdout)

    def test_mrz_test_only_change_is_skipped_not_conflated_with_empty(self):
        """crates/mrz/tests/ owes no *mrz-scoped* fragment (check 3), but check 2's
        general "a code change carries a fragment" rule has no tests/ exclusion, so
        it still needs an ordinary fragment -- add one so only check 3's behaviour
        is under test here."""
        self.branch("feature-mrz-tests")
        (self.tmp / "crates" / "mrz" / "tests").mkdir()
        (self.tmp / "crates" / "mrz" / "tests" / "extra.rs").write_text(
            "#[test]\nfn t() {}\n", encoding="utf-8"
        )
        (self.tmp / "changelog.d" / "feature-mrz-tests.added.md").write_text(
            "- add mrz test coverage.\n", encoding="utf-8"
        )
        self.commit_all("add an mrz test")

        r = self.run_check("--base", "main")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("skipped: mrz changes are test-only or documentation", r.stdout)
        self.assertNotIn("skipped: no crates/mrz/ changes", r.stdout)


if __name__ == "__main__":
    sys.exit(unittest.main())
