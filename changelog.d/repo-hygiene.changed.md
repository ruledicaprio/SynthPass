- **Repo hygiene.** `SYNTHPASS_ENGINEERING_CONSTITUTION.md` is dissolved — its still-current
  content moved into `knowledge/ARCHITECTURE.md` §13 (crate contracts, `synthpass-die` dependency
  boundary, MRZ-handling policy) and `CLAUDE.md` (priority order + worked examples); the duplicated
  and stale sections (including a wrong "dual Apache 2.0 + MIT" licence claim — the project is
  MIT-only) are gone. Bench-report JSON now defaults to `artifacts/` and model weights to `models/`
  (both gitignored) instead of the repo root. Added `.github/PULL_REQUEST_TEMPLATE.md`,
  `.github/ISSUE_TEMPLATE/`, and `.github/dependabot.yml`; `.gitignore` now explicitly guards
  `.claude/` local settings. Trimmed the README badge row and retired the "formerly
  multi-level-id-strip" notes from the front-page docs.
