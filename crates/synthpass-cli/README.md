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
| `synthpass <image> [--json]` | extract one document | required |
| `synthpass batch <dir\|glob> [--json]` | extract every matching image | required (same as above); a license missing the `batch` feature is metered — one warning, still runs |
| `synthpass decrypt <file.json.enc>` | decrypt an encrypted result (needs `SYNTHPASS_KEY`) | none |
| `synthpass doctor` | preflight: OCR models, license, config; the Tier-2 model is optional unless `SYNTHPASS_MODEL_PATH` is set | none |
| `synthpass fingerprint` | print this machine's fingerprint | none |
| `synthpass verify-license [path]` | verify a license file | none |
| `synthpass generate ...` | synthetic document images + label JSON | none |
| `synthpass export ...` | synthetic corpus as a training dataset | none required; a license missing the `export` feature (or missing/invalid entirely) is metered — one warning, still runs |

`SYNTHPASS_LICENSE_SKIP=1` bypasses the license check (and its warnings) entirely, for local
development. A `batch`/`export` license feature is never a hard gate (issue #494,
`knowledge/BRANDING.md` §5): it's metered with a stderr warning, never refused.

## The output contract: `--json`

stdout is human-readable by default; it becomes a script-parseable contract only under
`--json`. In both modes, every progress/diagnostic line (the config echo, "Processing local
file...", batch's summary line — every emoji line) goes to stderr, never stdout.

- `synthpass <image> --json` prints exactly one line of compact JSON — the v2 `ExtractionV2`
  object — to stdout, when extraction produced a usable result; nothing to stdout otherwise (the
  error still goes to stderr).
- `synthpass batch <dir|glob> --json` prints [JSON Lines](https://jsonlines.org/): one compact
  `ExtractionV2` object per successfully extracted document, in input order. A document that
  failed extraction writes no stdout line, only a stderr error.
- Default (non-`--json`) stdout content, exit codes, and the `<input>.json` sidecar file are all
  unchanged — see [ADR-0023](../../knowledge/decisions/ADR-0023-cli-output-contract.md).

## Where the contracts live

- Exit codes (0 ok, 1 failure, 2 usage, 3 license refusal): the table in
  [`knowledge/ARCHITECTURE.md` §12](../../knowledge/ARCHITECTURE.md#12-configuration-reference),
  implemented by the `Exit` enum in `src/main.rs`.
- The output contract (`--json`, stdout vs stderr):
  [ADR-0023](../../knowledge/decisions/ADR-0023-cli-output-contract.md).
- The processing flow per command: `knowledge/ARCHITECTURE.md` §5, "CLI".
- `export` formats and schema: [`knowledge/EXPORTS.md`](../../knowledge/EXPORTS.md).
- Licensing: [`knowledge/LICENSING.md`](../../knowledge/LICENSING.md).

## Tests

Unit tests sit next to the code (`src/main.rs`, `src/generate.rs`, `src/export.rs`) — including
the `--json` output contract's golden tests (`json_line_is_one_compact_line_that_round_trips` and
the `batch_json_*` tests, pinned against a constructed `ExtractionV2`, not a real extraction); the
integration tests in `tests/` run the built binary and pin exit codes, license round-trips,
decryption, the `doctor` model check, and the `--json`/default stdout-vs-stderr stream split
(`json_contract.rs`).

```bash
cargo test -p synthpass-cli
```
