- **A weekly Windows test run.** `.github/workflows/windows.yml` runs `cargo test --workspace` and
  the `tools/` Python suite on `windows-latest` every Monday and on demand, and on a pull request
  that edits the workflow itself. It is not a required check. It exists because three Windows-only
  failures (#620, #623, #640) passed Linux CI and were found only when a lane ran on Windows.
