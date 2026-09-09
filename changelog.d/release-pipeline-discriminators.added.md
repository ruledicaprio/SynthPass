- **The release process is enforced rather than remembered.** Cutting a release was entirely
  manual — a hand-edited version, a hand-run assembler, a hand-made tag — and it had gone wrong
  in every way it could. All fifteen release tags sit on a history sharing **no common ancestor
  with `main`** (`git merge-base v1.4.0 HEAD` exits 1; `git describe` fails outright), so
  `git log v1.3.0..v1.4.0` is fatal and GitHub's tag compare is meaningless. Fifteen of sixteen
  tags have no GitHub Release, and `crates/mrz` shipped nine versions with no tag at all.

  **`scripts/next-version.sh`** derives the next version from the pending changelog fragments
  rather than asking a human to recall the semver rules at the moment they are least likely to:
  breaking → major, `added` → minor, otherwise patch — except for `mrz`, where pre-1.0 makes the
  *minor* slot the breaking slot, so a breaking change moves 0.7.1 → 0.8.0 rather than 0.7.2.
  Fragments mark breaking with a trailing `!` (`api-rename.changed!.md`), which is the whole
  discriminator.

  **`scripts/check-changelog.sh`** is the new `changelog` CI job. It fails a PR that touches
  `crates/` without a fragment (escape hatch: a `skip-changelog` label), one that touches
  `crates/mrz/` without an `mrz`-scoped fragment, one whose fragment is malformed, and — the
  point — one whose version bump disagrees with what its fragments imply. It found four
  existing fragments that did not open with a `-` bullet and would have been spliced into
  `CHANGELOG.md` as dangling paragraphs under someone else's heading; those are repaired here.

  **`.github/workflows/release.yml`** watches for a version change on `main` and creates the
  annotated tag on the commit that made it — which is on `main` by construction, so the
  orphaned-tag failure cannot recur. It opens the GitHub Release with that version's CHANGELOG
  section as the body. Publishing `mrz` to crates.io is a separate job behind a manually
  approved `crates-io` environment, because a publish cannot be taken back.

- **`crates/mrz` has its own changelog and fragment directory.** `changelog.d/mrz/` assembles
  into `crates/mrz/CHANGELOG.md` via `scripts/assemble-changelog.sh --scope mrz`. A library
  consumer reading docs.rs should not have to filter application changes out of a library's
  changelog. Pre-0.7.1 history stays in the root `CHANGELOG.md`, where it was recorded, and the
  new file says so rather than pretending to be complete.

- **`RELEASING.md`** documents both release lines, why the version is derived rather than
  chosen, and why nobody creates a tag by hand any more. `scripts/repair-tags.sh` is the
  one-shot repair for the historical tags: it maps each to its **tree-identical** commit on
  `main` (exact for 14 of 15; `v1.3.0` falls back to the commit that introduced
  `version = "1.3.0"`, reported explicitly) and backfills the missing GitHub Releases. It is a
  dry run unless given `--apply`.
