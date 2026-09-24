"""Offline tests for the open-breaking-window case in scripts/next-version.sh and
scripts/check-changelog.sh.

`mrz` bumps its version in the PR that makes the first breaking change, and releases many commits
later (RELEASING.md). While that window is open the manifest says the *staged* number (0.8.0) and
the last release is still an older one (0.7.1). Both scripts used to measure the bump from the
manifest:

- next-version.sh answered 0.9.0 for a release that is 0.8.0;
- check-changelog.sh accepted a second bump inside the same window (0.8.0 -> 0.9.0 with a new
  breaking fragment), because it computed "implied" from the staged number.

Both now measure from the last *released* version: the topmost `## [X.Y.Z]` in the scope's
CHANGELOG. The cases that must refuse come first; a permissive bug looks exactly like a passing
test until the negative case is checked directly.

Throwaway git repositories only: no network, no cargo. Skips where bash or git is missing.
"""

import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CHECK = REPO / "scripts" / "check-changelog.sh"
NEXT = REPO / "scripts" / "next-version.sh"

BASH = shutil.which("bash")
GIT = shutil.which("git")

MRZ_CHANGELOG_071 = "# Changelog\n\n## [Unreleased]\n\n## [0.7.1] — 2026-09-14\n\n- a fix.\n"


def posix(path) -> str:
    """Bash reads a backslash as an escape, so Windows paths go in as D:/... ."""
    return str(path).replace("\\", "/")


@unittest.skipUnless(BASH and GIT, "bash and git are required")
class ReleaseWindowTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="release-window-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.git("init", "-q", ".")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "user.name", "Test")
        self.git("config", "core.autocrlf", "false")

        (self.tmp / "scripts").mkdir()
        shutil.copyfile(CHECK, self.tmp / "scripts" / "check-changelog.sh")
        shutil.copyfile(NEXT, self.tmp / "scripts" / "next-version.sh")
        (self.tmp / "crates" / "mrz" / "src").mkdir(parents=True)
        (self.tmp / "changelog.d" / "mrz").mkdir(parents=True)

        self.write("Cargo.toml", '[workspace.package]\nversion = "1.5.0"\n')
        self.write("CHANGELOG.md", "# Changelog\n\n## [1.5.0] — 2026-09-17\n\n- a thing.\n")
        self.set_mrz_version("0.7.1")
        self.write("crates/mrz/CHANGELOG.md", MRZ_CHANGELOG_071)
        self.write("crates/mrz/src/lib.rs", "pub fn a() {}\n")
        self.write("changelog.d/README.md", "# fragments\n")
        self.write("changelog.d/mrz/README.md", "# mrz fragments\n")
        self.commit_all("0.7.1 released")
        self.git("branch", "-M", "main")

    # ------------------------------------------------------------- helpers

    def write(self, rel: str, text: str):
        p = self.tmp / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text, encoding="utf-8", newline="\n")

    def set_mrz_version(self, v: str):
        self.write("crates/mrz/Cargo.toml", f'[package]\nname = "mrz"\nversion = "{v}"\n')

    def git(self, *args) -> str:
        r = subprocess.run(["git", *args], cwd=str(self.tmp), capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        return r.stdout.strip()

    def commit_all(self, message: str):
        self.git("add", "-A", ".", ":!scripts")
        self.git("commit", "-qm", message)

    def run_script(self, name: str, *args) -> subprocess.CompletedProcess:
        return subprocess.run([BASH, posix(self.tmp / "scripts" / name), *args],
                              cwd=str(self.tmp), capture_output=True, text=True)

    def open_window_on_main(self):
        """main: the first breaking change bumps mrz to 0.8.0; nothing released yet."""
        self.set_mrz_version("0.8.0")
        self.write("crates/mrz/src/lib.rs", "pub fn a(x: u8) {}\n")
        self.write("changelog.d/mrz/first-break.changed!.md", "- a breaking change.\n")
        self.commit_all("open the 0.8.0 window")

    # ------------------------------------------------- must refuse / must not over-bump

    def test_second_bump_inside_an_open_window_fails(self):
        self.open_window_on_main()
        self.git("checkout", "-qb", "double-bump", "main")
        self.set_mrz_version("0.9.0")
        self.write("crates/mrz/src/lib.rs", "pub fn a(x: u16) {}\n")
        self.write("changelog.d/mrz/second-break.changed!.md", "- another breaking change.\n")
        self.commit_all("bump again inside the window")

        r = self.run_script("check-changelog.sh", "--base", "main")
        self.assertEqual(r.returncode, 1, r.stdout + r.stderr)
        self.assertIn("mrz version 0.8.0 -> 0.9.0, but the fragments imply 0.8.0", r.stderr)

    def test_next_version_in_an_open_window_is_the_staged_number(self):
        self.open_window_on_main()
        r = self.run_script("next-version.sh", "--scope", "mrz")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "0.8.0")

    # ------------------------------------------------------- ordinary cases still pass

    def test_first_breaking_bump_opens_the_window(self):
        self.git("checkout", "-qb", "first-break", "main")
        self.set_mrz_version("0.8.0")
        self.write("crates/mrz/src/lib.rs", "pub fn a(x: u8) {}\n")
        self.write("changelog.d/mrz/first-break.changed!.md", "- a breaking change.\n")
        self.commit_all("first breaking change")

        r = self.run_script("check-changelog.sh", "--base", "main")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("mrz: 0.7.1 -> 0.8.0 matches the fragments", r.stdout)

    def test_more_work_inside_the_window_needs_no_bump(self):
        self.open_window_on_main()
        self.git("checkout", "-qb", "more-work", "main")
        self.write("crates/mrz/src/lib.rs", "pub fn a(x: u8) {}\npub fn b() {}\n")
        self.write("changelog.d/mrz/more.changed!.md", "- a second breaking change, same window.\n")
        self.commit_all("more breaking work, no bump")

        r = self.run_script("check-changelog.sh", "--base", "main")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("mrz: unchanged at 0.8.0", r.stdout)

    def test_after_release_the_next_breaking_bump_is_measured_from_it(self):
        self.set_mrz_version("0.8.0")
        self.write("crates/mrz/CHANGELOG.md",
                   "# Changelog\n\n## [Unreleased]\n\n## [0.8.0] — 2026-09-30\n\n- released.\n\n"
                   + MRZ_CHANGELOG_071.split("## [Unreleased]\n\n", 1)[1])
        self.commit_all("0.8.0 released")
        self.write("changelog.d/mrz/next-break.changed!.md", "- breaking, after the release.\n")
        self.commit_all("new breaking fragment")

        r = self.run_script("next-version.sh", "--scope", "mrz")
        self.assertEqual(r.stdout.strip(), "0.9.0", r.stderr)

    def test_workspace_is_unaffected(self):
        self.write("changelog.d/new-thing.added.md", "- a new thing.\n")
        self.commit_all("an additive workspace fragment")
        r = self.run_script("next-version.sh")
        self.assertEqual(r.stdout.strip(), "1.6.0", r.stderr)


if __name__ == "__main__":
    sys.exit(unittest.main())
