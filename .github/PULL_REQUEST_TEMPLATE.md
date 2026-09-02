<!--
Keep the description tight. One logical change per PR is the norm; if this PR
bundles several, say so and why.
-->

## Summary

<!-- What changed and why. Link the issue/ADR if there is one. -->

## Testing

<!-- What you ran, and the result. Paste the relevant output for anything that
     isn't obvious from the diff. -->

## Checklist

- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace --all-targets` is clean
- [ ] `cargo test --workspace` passes
- [ ] `bash scripts/check-doc-links.sh` passes — if this PR touches Markdown or moves a cited file. Run it **after `git add`**: it scans git-tracked files only, so a citation in a brand-new file is invisible to it locally and fails in CI instead
- [ ] Added a `changelog.d/<slug>.<category>.md` fragment — or this change is purely internal and needs none
- [ ] One logical change — or the PR body explains the bundle
