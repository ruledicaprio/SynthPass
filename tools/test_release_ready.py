"""Offline tests for the two release gates in scripts/.

`check-release-ready.sh` decides whether a commit that moved a version field is
a release or a staged bump. `tag-release.sh` decides whether an existing tag may
be treated as "nothing to do". Both answer questions that used to be answered by
assumption, and both failures were silent: mrz-v0.8.0 sits on the commit that
opened the breaking window, and the crates.io publish queued behind it reported
green the whole time.

Silent is the operative word. A gate that is wrong in the permissive direction
looks exactly like one that passed, so these tests spend most of their effort on
the negative cases -- the ones where the scripts must refuse.

Everything here runs against throwaway fixture trees: no network, no cargo, and
no dependency on what this repository's own changelog happens to contain today
(which changes with every release, and would make these tests a tripwire for
ordinary work). The git-backed cases skip cleanly where git is unavailable.
"""

import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CHECK_READY = REPO / "scripts" / "check-release-ready.sh"
TAG_RELEASE = REPO / "scripts" / "tag-release.sh"
ASSEMBLE = REPO / "scripts" / "assemble-changelog.sh"

BASH = shutil.which("bash")
GIT = shutil.which("git")


def posix(path) -> str:
    """Bash reads a backslash as an escape, so Windows paths go in as D:/... ."""
    return str(path).replace("\\", "/")


def run(script: Path, *args: str, cwd=None) -> subprocess.CompletedProcess:
    return subprocess.run(
        [BASH, posix(script), *args],
        cwd=None if cwd is None else str(cwd),
        capture_output=True,
        text=True,
    )


@unittest.skipUnless(BASH, "bash is required to run the release gates")
class CheckReleaseReadyTest(unittest.TestCase):
    """scripts/check-release-ready.sh over synthetic manifests and changelogs."""

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="release-ready-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)

    def tree(self, *, version="0.8.0", changelog: str, fragments=()):
        """Write a fixture and return the argv that points the gate at it."""
        manifest = self.tmp / "Cargo.toml"
        manifest.write_text(f'[package]\nname = "mrz"\nversion = "{version}"\n', encoding="utf-8")

        log = self.tmp / "CHANGELOG.md"
        log.write_text(changelog, encoding="utf-8")

        frags = self.tmp / "fragments"
        frags.mkdir(exist_ok=True)
        # Every real fragment directory has one, and it is not a pending entry.
        (frags / "README.md").write_text("# how fragments work\n", encoding="utf-8")
        for name in fragments:
            (frags / name).write_text("- an entry.\n", encoding="utf-8")

        return [
            "--scope", "mrz",
            "--manifest", posix(manifest),
            "--changelog", posix(log),
            "--fragment-dir", posix(frags),
        ]

    ASSEMBLED = (
        "# Changelog\n\n"
        "## [Unreleased]\n\n"
        "## [0.8.0] — 2026-09-22\n\n"
        "### Changed\n\n"
        "- `ParseOptions` is now non-exhaustive.\n\n"
        "## [0.7.1] — 2026-09-14\n\n"
        "- Older entry.\n"
    )

    # ---------------------------------------------------------------- ready

    def test_assembled_release_is_ready(self):
        r = run(CHECK_READY, *self.tree(changelog=self.ASSEMBLED))
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("READY", r.stdout)

    def test_readme_alone_is_not_a_pending_fragment(self):
        r = run(CHECK_READY, *self.tree(changelog=self.ASSEMBLED, fragments=()))
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("none pending", r.stdout)

    # ------------------------------------------------------------ not ready

    def test_staged_bump_is_refused(self):
        """The shape that actually shipped: version moved, nothing assembled."""
        r = run(CHECK_READY, *self.tree(
            changelog="# Changelog\n\n## [Unreleased]\n\n## [0.7.1] — 2026-09-14\n\n- Older.\n",
            fragments=["parseoptions-non-exhaustive.changed!.md", "mrz-class-sweep.added.md"],
        ))
        self.assertEqual(r.returncode, 1)
        # Both reasons must be reported, not just the first one found.
        self.assertIn("no '## [0.8.0]' section", r.stderr)
        self.assertIn("2 fragment(s) still pending", r.stderr)
        self.assertIn("parseoptions-non-exhaustive.changed!.md", r.stderr)

    def test_pending_fragment_alone_blocks_a_release(self):
        r = run(CHECK_READY, *self.tree(
            changelog=self.ASSEMBLED,
            fragments=["late-arrival.fixed.md"],
        ))
        self.assertEqual(r.returncode, 1)
        self.assertIn("late-arrival.fixed.md", r.stderr)

    def test_section_with_no_entries_is_not_a_release(self):
        r = run(CHECK_READY, *self.tree(
            changelog="# Changelog\n\n## [Unreleased]\n\n## [0.8.0] — 2026-09-22\n\n"
                      "## [0.7.1] — 2026-09-14\n\n- Older.\n",
        ))
        self.assertEqual(r.returncode, 1)
        self.assertIn("no entries", r.stderr)

    def test_unreleased_still_holding_entries_is_refused(self):
        """assemble-changelog.sh ran; nobody renamed the heading."""
        r = run(CHECK_READY, *self.tree(
            changelog="# Changelog\n\n## [Unreleased]\n\n### Changed\n\n"
                      "- `ParseOptions` is now non-exhaustive.\n\n"
                      "## [0.8.0] — 2026-09-22\n\n- Something.\n",
        ))
        self.assertEqual(r.returncode, 1)
        self.assertIn("[Unreleased]", r.stderr)

    def test_version_mismatch_is_refused(self):
        r = run(CHECK_READY, *self.tree(version="0.8.0", changelog=self.ASSEMBLED),
                "--version", "0.9.0")
        self.assertEqual(r.returncode, 1)
        self.assertIn("0.9.0", r.stderr)
        self.assertIn("0.8.0", r.stderr)

    def test_version_is_escaped_before_it_becomes_a_regex(self):
        """'0.8.0' must not match the heading '[0-8-0]' through its dots."""
        r = run(CHECK_READY, *self.tree(
            changelog="# Changelog\n\n## [Unreleased]\n\n## [0-8-0] — 2026-09-22\n\n- Entry.\n",
        ))
        self.assertEqual(r.returncode, 1, "a '.' in the version matched any character")
        self.assertIn("no '## [0.8.0]' section", r.stderr)

    # -------------------------------------------------------- usage errors

    def test_unknown_scope_is_a_usage_error_not_an_answer(self):
        r = run(CHECK_READY, "--scope", "nonsense")
        self.assertEqual(r.returncode, 2)

    def test_missing_changelog_is_a_usage_error_not_an_answer(self):
        args = self.tree(changelog=self.ASSEMBLED)
        (self.tmp / "CHANGELOG.md").unlink()
        r = run(CHECK_READY, *args)
        # Exit 2, never 1: "the gate could not run" must stay distinguishable
        # from "this is not a release", because the workflow treats 1 as an
        # ordinary staged bump and anything else as a broken gate.
        self.assertEqual(r.returncode, 2)


@unittest.skipUnless(BASH and GIT, "bash and git are required for the tag gate")
class TagReleaseTest(unittest.TestCase):
    """scripts/tag-release.sh over a throwaway repository and a local remote."""

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="tag-release-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)

        self.remote = self.tmp / "origin.git"
        self.work = self.tmp / "work"
        self.git("init", "-q", "--bare", posix(self.remote), cwd=self.tmp)
        self.git("init", "-q", posix(self.work), cwd=self.tmp)
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "user.name", "Test")
        self.git("remote", "add", "origin", posix(self.remote))

        (self.work / "f").write_text("a\n", encoding="utf-8")
        self.git("add", "f")
        self.git("commit", "-qm", "open the breaking window")
        self.window = self.head()

        (self.work / "f").write_text("b\n", encoding="utf-8")
        self.git("commit", "-qam", "the release commit")
        self.release = self.head()

    def git(self, *args, cwd=None):
        r = subprocess.run([GIT, *args], cwd=str(cwd or self.work),
                           capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        return r.stdout.strip()

    def head(self):
        return self.git("rev-parse", "HEAD")

    def tag_release(self, *args):
        return run(TAG_RELEASE, *args, cwd=self.work)

    # -------------------------------------------------------------- states

    def test_absent_tag_verify_only_changes_nothing(self):
        r = self.tag_release("--tag", "mrz-v0.8.0", "--sha", self.release, "--verify-only")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("would create", r.stdout)
        self.assertEqual(self.git("tag", "-l"), "", "--verify-only created a tag")

    def test_absent_tag_is_created_and_pushed(self):
        r = self.tag_release("--tag", "mrz-v0.8.0", "--sha", self.release)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertEqual(self.git("rev-list", "-n", "1", "mrz-v0.8.0"), self.release)
        self.assertIn("mrz-v0.8.0", self.git("ls-remote", "--tags", posix(self.remote)))

    def test_existing_tag_on_the_same_commit_is_idempotent(self):
        self.git("tag", "-a", "mrz-v0.8.0", "-m", "mrz-v0.8.0", self.release)
        r = self.tag_release("--tag", "mrz-v0.8.0", "--sha", self.release)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("nothing to do", r.stdout)

    def test_existing_tag_on_a_different_commit_fails_loudly(self):
        """The regression. This exact state exited 0 and published."""
        self.git("tag", "-a", "mrz-v0.8.0", "-m", "mrz-v0.8.0", self.window)
        r = self.tag_release("--tag", "mrz-v0.8.0", "--sha", self.release)
        self.assertEqual(r.returncode, 1, "a mismatched tag was accepted")
        self.assertIn(self.window, r.stderr)
        self.assertIn(self.release, r.stderr)
        # The tag must be left exactly where it was: repairing one is a
        # deliberate, human-approved act, never a side effect of a release run.
        self.assertEqual(self.git("rev-list", "-n", "1", "mrz-v0.8.0"), self.window)

    def test_lightweight_tags_are_peeled_too(self):
        self.git("tag", "mrz-v0.8.0", self.window)
        r = self.tag_release("--tag", "mrz-v0.8.0", "--sha", self.release)
        self.assertEqual(r.returncode, 1)
        self.assertIn(self.window, r.stderr)

    # -------------------------------------------------------- usage errors

    def test_short_sha_is_refused(self):
        """A short sha, a branch or a tag would all bind to something that moves."""
        r = self.tag_release("--tag", "mrz-v0.8.0", "--sha", self.release[:12])
        self.assertEqual(r.returncode, 2)

    def test_branch_name_is_refused(self):
        r = self.tag_release("--tag", "mrz-v0.8.0", "--sha", "main")
        self.assertEqual(r.returncode, 2)

    def test_unknown_commit_is_refused(self):
        r = self.tag_release("--tag", "mrz-v0.8.0", "--sha", "0" * 40)
        self.assertEqual(r.returncode, 2)
        self.assertIn("not a commit", r.stderr)


if __name__ == "__main__":
    unittest.main()

@unittest.skipUnless(BASH, "bash is required to run the assembler")
class AssembleChangelogTest(unittest.TestCase):
    """scripts/assemble-changelog.sh must carry breaking (`!`) fragments.

    The fragment convention marks a breaking change as `<id>.<cat>!.md`. The
    assembler globbed only `*.<cat>.md`, so every breaking entry was left
    behind while the plain ones were spliced in: the release section that
    announces a break omitted the break. check-release-ready.sh would then
    refuse on the leftovers, but only after the section had been written.
    """

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="assemble-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        # The script resolves its repository root from its own location.
        (self.tmp / "scripts").mkdir()
        shutil.copy(ASSEMBLE, self.tmp / "scripts" / ASSEMBLE.name)
        self.frags = self.tmp / "changelog.d" / "mrz"
        self.frags.mkdir(parents=True)
        (self.frags / "README.md").write_text("# how fragments work\n", encoding="utf-8")
        self.log = self.tmp / "crates" / "mrz" / "CHANGELOG.md"
        self.log.parent.mkdir(parents=True)
        self.log.write_text(
            "# Changelog\n\n## [Unreleased]\n\n## [0.7.1] — 2026-09-14\n\n- Older entry.\n",
            encoding="utf-8",
        )

    def test_breaking_fragments_are_assembled_first_and_consumed(self):
        (self.frags / "plain.changed.md").write_text("- A plain change.\n", encoding="utf-8")
        (self.frags / "breaking.changed!.md").write_text("- A breaking change.\n", encoding="utf-8")
        (self.frags / "new.added.md").write_text("- A new thing.\n", encoding="utf-8")

        result = run(self.tmp / "scripts" / ASSEMBLE.name, "--scope", "mrz", "--write")
        self.assertEqual(result.returncode, 0, result.stderr)

        text = self.log.read_text(encoding="utf-8")
        self.assertIn("- A breaking change.", text)
        self.assertIn("- A plain change.", text)
        self.assertLess(text.index("- A breaking change."), text.index("- A plain change."))
        self.assertLess(text.index("- A plain change."), text.index("## [0.7.1]"))
        pending = sorted(p.name for p in self.frags.iterdir() if p.name != "README.md")
        self.assertEqual(pending, [])
