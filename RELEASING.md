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
creates the annotated tag **on the merge commit**, and opens the GitHub Release with that
version's CHANGELOG section as the body. Nobody creates a tag by hand.

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
sixteen tags never got a GitHub Release. `crates/mrz` shipped nine versions — 0.5.1 through
0.7.1 — with no tags at all.

Automating it is not about saving keystrokes. A workflow triggered by the version change can
only ever tag the commit that made it, and that commit is on `main` by construction. The class
of bug disappears rather than being remembered about.

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

Same shape, with two differences.

- Fragments live in [`changelog.d/mrz/`](changelog.d/mrz/README.md) and assemble into
  `crates/mrz/CHANGELOG.md`, not the root one.
- **The bump happens in the PR that makes the change**, not in a separate release PR. CI's
  `semver` job derives the permitted bump from `crates/mrz/Cargo.toml` and diffs against
  crates.io, so a breaking change only goes green once the version already reflects it.

Merging tags `mrz-vX.Y.Z`. Publishing to crates.io is a **separate, gated step**: the
`publish-mrz` job runs in the `crates-io` GitHub environment, which requires a manual approval,
because a publish cannot be undone — a version can be yanked but never replaced. The job re-runs
the full `mrz` test suite and a `-D warnings` rustdoc build before it publishes.

## When something is deliberately not in the changelog

Internal changes a user could never notice need no fragment. Label the PR `skip-changelog` and
`check-changelog.sh` stands down. Use it sparingly: the label is a claim that nobody
downstream is affected, and it is not checked by anything but judgement.

## Repairing the historical tags

The pre-`v1.4.0` tags remain on the orphaned history described above. `scripts/repair-tags.sh`
re-points them onto their tree-identical commits on `main` and backfills the missing GitHub
Releases. It is a one-shot operation, kept in the repo for provenance rather than for reuse —
read its header before running it, because it force-pushes tags.
