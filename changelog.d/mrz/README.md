# changelog.d/mrz/ — fragments for the `mrz` crate

`crates/mrz` is published to crates.io on **its own version line**, decoupled from the
workspace (0.7.1 against the workspace's 1.4.0). Its changelog entries therefore belong to a
different release than everything else, and they live here rather than one directory up.

Same grammar as the workspace set (see [`../README.md`](../README.md)):

```
changelog.d/mrz/<id>.<category>[!].md
```

Two things differ.

**The `!` matters more here.** `mrz` is pre-1.0, so cargo treats the **minor** slot as the
breaking slot — `^0.7` resolves `0.7.x` but never `0.8.0`, which means a careless bump forces a
`Cargo.toml` edit on every consumer. A fragment named `api-rename.changed!.md` moves the crate
0.7.1 → **0.8.0**; without the `!` it would move to 0.7.2 and quietly ship a breaking change in
a patch. `scripts/next-version.sh --scope mrz --explain` shows the derivation, and
`cargo-semver-checks` (CI's `semver` job) is the independent check that the categorisation was
honest.

**The version bumps in the same PR.** Because the `semver` job diffs the branch's public API
against the latest crates.io release and derives the permitted bump from
`crates/mrz/Cargo.toml`, a PR that breaks the API must bump the version *in that same PR* to go
green. `scripts/check-changelog.sh` accounts for this: it derives the implied version from the
union of the fragments pending at the base commit and the ones this PR adds.

Test-only and CI-only changes under `crates/mrz/` ship no release and need no fragment.

Full release procedure: [`../../RELEASING.md`](../../RELEASING.md).
