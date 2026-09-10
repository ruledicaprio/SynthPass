- **`provider-bench --real-specimens --include-private`** opts the gitignored
  `samples/private/` specimens back into a real-specimen run — they carry damage
  classes (a covered MRZ line, kiosk glare, a missing character) the public corpus
  doesn't have. Off by default, rejected without `--real-specimens`, and never set
  by CI: a private-track report names real identity documents. Ground truth sits
  beside each image as `private/<stem>.json` in a richer nested schema, projected
  onto the flat `Extraction` the accuracy loop reads. Deliberately a
  `provider-bench` flag only, not a `run-bench.ps1` track — that script commits and
  pushes its results to the `bench-data` branch, which a private-derived run must
  never do.
