# synthpass-cli

The `synthpass` command-line binary: a thin front end over
[`synthpass-pipeline`](../synthpass-pipeline) for local, offline passport and MRZ extraction,
plus the synthetic-corpus commands `generate` and `export`. Application code, never published
to crates.io.

```bash
cargo run -p synthpass-cli -- --help
```

## Commands

| Command | Does | License |
| --- | --- | --- |
| `synthpass <image>` | extract one document | required |
| `synthpass batch <dir\|glob>` | extract every matching image | `batch` feature |
| `synthpass decrypt <file.json.enc>` | decrypt an encrypted result (needs `SYNTHPASS_KEY`) | none |
| `synthpass doctor` | preflight: OCR models, Tier-2 inferer, license | none |
| `synthpass fingerprint` | print this machine's fingerprint | none |
| `synthpass verify-license [path]` | verify a license file | none |
| `synthpass generate ...` | synthetic document images + label JSON | none |
| `synthpass export ...` | synthetic corpus as a training dataset | `export` feature |

`SYNTHPASS_LICENSE_SKIP=1` bypasses the license gate for local development.

## Where the contracts live

- Exit codes (0 ok, 1 failure, 2 usage, 3 license refusal): the table in
  [`knowledge/ARCHITECTURE.md` §12](../../knowledge/ARCHITECTURE.md#12-configuration-reference),
  implemented by the `Exit` enum in `src/main.rs`.
- The processing flow per command: `knowledge/ARCHITECTURE.md` §5, "CLI".
- `export` formats and schema: [`knowledge/EXPORTS.md`](../../knowledge/EXPORTS.md).
- Licensing: [`knowledge/LICENSING.md`](../../knowledge/LICENSING.md).

## Tests

Unit tests sit next to the code (`src/main.rs`, `src/generate.rs`, `src/export.rs`); the
integration tests in `tests/` run the built binary and pin exit codes, license round-trips,
decryption and the `doctor` model check.

```bash
cargo test -p synthpass-cli
```
