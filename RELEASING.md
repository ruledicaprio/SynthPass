# Releasing SynthPass

Two independent release lines:

| Line | Version source | Tag | Published to |
| --- | --- | --- | --- |
| **workspace** — every `synthpass-*` crate | `Cargo.toml` `[workspace.package]` | `vX.Y.Z` | GitHub Release |
| **`mrz`** — the standalone ICAO 9303 engine | `crates/mrz/Cargo.toml` | `mrz-vX.Y.Z` | crates.io + tag |

They move independently on purpose: `mrz` is a public library other people depend on, and
tying it to the application's release cadence would force consumers through churn that has
nothing to do with them.

## The short version

```bash
scripts/next-version.sh --explain          # what the pending fragments imply, and why
scripts/assemble-changelog.sh --write      # splice fragments into CHANGELOG.md, delete them
# edit Cargo.toml's [workspace.package] version to the number above
cargo check --workspace                    # refresh Cargo.lock
git commit -am "release: vX.Y.Z — <title>"
# open a PR, merge it
```

Merging is the whole release. `.github/workflows/release.yml` notices the version changed,
confirms the release is actually assembled, creates the annotated tag **on the merge commit**,
and opens the GitHub Release with that version's CHANGELOG section as the body. Nobody creates
a tag by hand.

The version changing is necessary and **not sufficient** — see "A version change is not a
release" below, which is the difference between a release and an open breaking window.

## Why nobody creates a tag by hand

Every release tag from `v0.4.0` to `v1.4.0` was made locally, and every one of them landed on a
history with **no common ancestor with `main`**:

```console
$ git merge-base v1.4.0 HEAD ; echo $?
1
$ git describe --tags
fatal: No tags can describe 'f735dc2'.
```

So `git log v1.3.0..v1.4.0` is fatal, GitHub's tag compare is meaningless, and fifteen of the
sixteen tags never got a GitHub Release. `crates/mrz` published fourteen versions and tagged two
of them: 0.1.0–0.3.0 and 0.5.1–0.7.0 have no tag at all.

Automating it is not about saving keystrokes. A workflow triggered by the version change can
only ever tag the commit that made it, and that commit is on `main` by construction. The class
of bug disappears rather than being remembered about.

## A version change is not a release

The trigger above watches the version *field*. For the workspace that is nearly the same thing
as a release, because the version moves in the release PR and nowhere else. For `mrz` it is not:
pre-1.0, the minor slot is the breaking slot, and CI's `semver` job only goes green once the
version already reflects a breaking change — so `mrz` bumps in the PR that makes the **first**
breaking change, and the release follows however many commits later the window takes to close.

The workflow used to read that staged bump as a release. On 2026-09-19 the 0.8.0 window opened;
the workflow tagged `mrz-v0.8.0` on that commit and queued a crates.io publish behind the
`crates-io` approval gate. Twenty-three commits of 0.8 work landed after it, none of them in the
tag. Nothing went red at any point — a publish waiting for approval looks exactly like a publish
nobody has got round to yet.

So the workflow asks two separate questions now, and `scripts/check-release-ready.sh` answers
the second one from the repository itself:

| | Ready | Not ready |
| --- | --- | --- |
| manifest version | is the version being released | disagrees with it |
| CHANGELOG | an assembled `## [X.Y.Z]` section, with entries | absent, or present and empty |
| `[Unreleased]` | empty | still holds this version's entries |
| fragments | consumed | still pending |

A staged bump fails the last two by construction, which is the whole design: readiness is
witnessed by the same fragments that already derive the version number, so there is no new flag
for anyone to remember to set. Run it yourself at any point:

```bash
scripts/check-release-ready.sh --scope mrz
```

`NOT READY` is the *normal* answer while a window is open, and the release run that reports it
is **green** — it releases nothing and says so in the job summary. It flips to `READY` on the
commit where the release PR assembles the changelog and consumes the fragments.

### A tag that already exists is not "nothing to do"

`scripts/tag-release.sh` owns the other half. Three states, and only the first two are ordinary:

- **absent** — create it, annotated, on the release commit, and push it;
- **present, naming this commit** — a notice; re-running a release is not an error;
- **present, naming a different commit** — hard failure; nothing tagged, nothing published.

The old workflow printed `already exists; nothing to do` and exited 0 for all three, which is
how a stale `mrz-v0.8.0` kept a green publish job aimed at the wrong commit. Repairing such a
tag stays deliberate and manual — see "Repairing the historical tags" below — because moving a
tag someone has already fetched rewrites history they hold.

### Everything binds to one commit

The package, the tag, the approval and the GitHub Release all name the SHA recorded when the run
started, never a branch: an approval can sit in the queue for days while `main` moves on
underneath it. Before the publish job becomes available, a separate `evidence` job packages the
crate from that commit, checks that the `.cargo_vcs_info.json` *inside the `.crate`* names it
too, runs `cargo publish --dry-run`, and attaches the `.crate` and its file list to the run. So
an approver is looking at a specific artifact built from a specific commit — and the publish job
re-derives both from scratch before it publishes, rather than trusting the jobs above it.

## The version number is derived, not chosen

Every user-visible change already drops a fragment in
[`changelog.d/`](changelog.d/README.md). Those fragments carry enough information to determine
which slot moves:

| Pending fragments | workspace (1.x) | `mrz` (0.x) |
| --- | --- | --- |
| any `<id>.<category>!.md` — breaking | major | **minor** (pre-1.0 breaking slot) |
| any `<id>.added.md` | minor | patch |
| otherwise | patch | patch |

The `mrz` column is not a typo. Pre-1.0, cargo treats the minor slot as the breaking slot —
`^0.7` resolves `0.7.x` but never `0.8.0` — so a breaking change there must move the minor.
`CONTRIBUTING.md`'s "Semver policy" section is the long form.

`scripts/check-changelog.sh` enforces this on every PR: a version bump that disagrees with its
fragments fails CI. It derives the implied version from the union of the fragments pending at
the base commit and any the PR adds, because both flows are real — a release PR *consumes*
fragments, while an `mrz` API change *adds* one alongside its bump in the same PR.

What that check does **not** do is verify a change was categorised honestly; it only checks the
number follows from the category. `cargo-semver-checks` (CI's `semver` job, diffing against the
published crates.io API) is the independent check on the categorisation itself, and it only
covers `mrz` — which is the crate where getting it wrong reaches other people.

## Releasing the workspace

1. **Check what the fragments imply.**
   ```bash
   scripts/next-version.sh --explain
   ```
   If the number surprises you, the fragments are wrong, not the script. A breaking change
   that was not named with a `!` is the usual cause.

2. **Assemble the changelog.**
   ```bash
   scripts/assemble-changelog.sh          # preview
   scripts/assemble-changelog.sh --write  # splice in and delete the fragments
   ```
   `--write` inserts into the topmost `## ` section. Add the new
   `## [X.Y.Z] — YYYY-MM-DD — <title>` heading yourself first if you want a titled release;
   the recent ones carry a title and a lead paragraph, the older ones do not.

3. **Bump `[workspace.package] version` in `Cargo.toml`**, then `cargo check --workspace` to
   refresh `Cargo.lock`.

4. **Commit as `release: vX.Y.Z — <title>`**, open a PR, get CI green, merge.

5. **The workflow does the rest.** Confirm with `gh release list` and:
   ```bash
   git fetch --tags && git describe --tags   # must resolve, not fatal
   ```

## Releasing `mrz`

Same shape, with three differences.

- Fragments live in [`changelog.d/mrz/`](changelog.d/mrz/README.md) and assemble into
  `crates/mrz/CHANGELOG.md`, not the root one. `--write` inserts into the topmost section, which
  is `## [Unreleased]`; rename it to `## [X.Y.Z] — YYYY-MM-DD` and open a fresh
  `## [Unreleased]` above it. The fragments that PR deletes are its entries, so
  `check-changelog.sh` accepts the `crates/mrz/CHANGELOG.md` edit in place of a new fragment.
- **A breaking change bumps in the PR that makes it**, not in a separate release PR. CI's
  `semver` job derives the permitted bump from `crates/mrz/Cargo.toml` and diffs against
  crates.io, so a breaking change only goes green once the version already reflects it. That
  bump **opens the breaking window and releases nothing**: the release run it triggers reports
  a staged bump and stops. Everything else in the window then ships under that same number, and
  the release is the later commit that assembles the changelog.
- **Cross-check the changelog against git before publishing.** Every commit that shipped should
  be covered by an entry:
  ```bash
  git log --oneline mrz-v<previous>..HEAD -- crates/mrz ':!crates/mrz/tests'
  ```
  When the previous version has no tag, read its commit from the published crate: every
  `.crate` carries a `.cargo_vcs_info.json` naming the SHA it was packaged from. That is how
  `crates/mrz/CHANGELOG.md`'s sections for 0.1.0–0.7.0 were reconstructed.

Merging the *assembled* release tags `mrz-vX.Y.Z`. Publishing to crates.io is a **separate,
gated step**: the `publish-mrz` job runs in the `crates-io` GitHub environment, which requires a
manual approval, because a publish cannot be undone — a version can be yanked but never
replaced. Before that approval is even offered, the `evidence` job must have packaged the crate
from the release commit and dry-run the publish; the job itself then re-verifies the tag names
that commit, re-runs the full `mrz` test suite and a `-D warnings` rustdoc build, and re-checks
that what it is about to upload was packaged from it.

Two things the workflow cannot do for itself:

- **The environment and its token must exist.** Create the `crates-io` environment with a
  required reviewer, and give it a `CARGO_REGISTRY_TOKEN` secret. GitHub creates an environment
  a workflow names on first use *without* protection rules, so a missing environment removes the
  approval gate silently, and a missing token fails the publish. Neither existed on 2026-09-14.
- **It only sees versions that move.** The trigger is a commit that changes the version. A
  version bumped before `release.yml` existed — `mrz` 0.7.1, bumped in #245 — is never tagged
  or published by it; that one is published by hand with `cargo publish -p mrz` from a clean,
  merged `main`.

## When something is deliberately not in the changelog

Internal changes a user could never notice need no fragment. Label the PR `skip-changelog` and
`check-changelog.sh` stands down. Use it sparingly: the label is a claim that nobody
downstream is affected, and it is not checked by anything but judgement.

## Repairing the historical tags

The pre-`v1.4.0` tags remain on the orphaned history described above. `scripts/repair-tags.sh`
re-points them onto their tree-identical commits on `main` and backfills the missing GitHub
Releases. It is a one-shot operation, kept in the repo for provenance rather than for reuse —
read its header before running it, because it force-pushes tags.
