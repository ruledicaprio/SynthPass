# Configuration reference

Every environment variable the workspace's crates read, grouped by subsystem, and the
`synthpass` CLI's exit codes. Checked against the tree at v1.7.0 (`e33b177`, 2026-09-28). This
is [`ARCHITECTURE.md`](../ARCHITECTURE.md) §12.

`synthpass doctor` reports the resolved OCR, inferer and license state at runtime, so nobody
has to cross-reference this table by hand. A variable added to the code is added here in the
same PR.

## Environment variables

### OCR

| Variable | Default | Purpose |
| --- | --- | --- |
| `SYNTHPASS_OCR_MODEL_DIR` | `.` | Directory holding `text-detection.rten` / `text-recognition.rten` (README quickstart convention: `models/`); also where `synthpass fetch-models` stages them. Also honoured by both `synthpass-bench` and `provider-bench` (issue #541); their fallback when it is unset is the build tree's own repo root (`crates/synthpass-bench`'s grandparent directory), not `.` |
| `SYNTHPASS_OCR_DETECTION_SHA256` / `..._RECOGNITION_SHA256` | *(built-in)* | Override expected checksums |
| `SYNTHPASS_OCR_MODEL_SKIP_VERIFY` | *(unset)* | Skip OCR model checksum verification |
| `SYNTHPASS_OCR_MAX_PASSES` / `SYNTHPASS_OCR_MAX_SECONDS` | `14` / `52` | Allow at most 14 pass starts; stop starting retry passes once 52 s have passed since the general pass started; a pass in flight still finishes, and rotation and band detection run before the clock starts |
| `SYNTHPASS_OCR_TEXTURE` | `on` | Trailing MRZ-band texture-suppression (median filter) passes after every other retry variant — `off`/`on`/`control`; see `synthpass-ocr`'s `TextureMode` doc comment |
| `SYNTHPASS_OCR_ORDER` | `default` | Retry-variant ordering — `default`/`band-first`/`control`; see `synthpass-ocr`'s `OcrOrder` doc comment |
| `SYNTHPASS_OCR_ROTATE` | `default` | Page-rotation detection — `default`/`legacy`/`off`; see `synthpass-ocr`'s `RotateMode` doc comment |
| `SYNTHPASS_OCR_SKEW` | `default` | Deskew correction — `default`/`legacy`; see `synthpass-ocr::preprocess`'s `SkewMode` doc comment |
| `SYNTHPASS_OCR_CHARGRID` | `off` | Post-hit repair of the MRZ name line's missing fillers on a fixed-pitch character grid — `off`/`on`/`control`; see `synthpass-ocr`'s `ChargridMode` doc comment |
| `SYNTHPASS_OCR_THREADS` | host cores − 1, floored at 1 | OCR-stage concurrency cap (`synthpass-pipeline`'s `env_ocr_threads`) |
| `SYNTHPASS_OCR_VERBOSE` | *(unset)* | `1` logs each pass's timing and detected-region count to stderr, and on a Tier-1 miss the MRZ-band candidate lines and what each pass's MRZ parse returned (#554); see `synthpass-ocr`'s module docs |
| `SYNTHPASS_OCR_DUMP_VARIANTS` | *(unset)* | Diagnostic: writes every preprocessed image the recognizer actually saw (general pass and each retry variant) as a PNG under this directory |
| `SYNTHPASS_OCR_ENGINE` | `rust` | Only `rust` since v1.2.0; any other value warns and falls back |

The five measurement-arm knobs above plus `SYNTHPASS_OCR_MAX_PASSES`/`SYNTHPASS_OCR_MAX_SECONDS` — seven
in total — are behaviour-changing: the same binary on the same image can read differently depending on
them, and nothing else in the output says why (issue #495). Whichever of the seven hold a
non-default *effective* value are recorded in each OCR-derived record's `trace.config_overrides`
(`synthpass-core`'s `ExtractionTrace`, [§13.1](../ARCHITECTURE.md#131-crate-responsibilities)) and
echoed once on stderr by `synthpass`/`synthpass batch` (`⚙️  [Rust] non-default OCR knobs: NAME=value …`) —
`SYNTHPASS_OCR_THREADS` and `SYNTHPASS_OCR_DUMP_VARIANTS` are concurrency/diagnostics, not
measurement arms, and are not part of either. The `SYNTHPASS_MRZ_*` arms in the next section are
recorded in the same `trace.config_overrides` (Tier-1 and Tier-2 records alike) when not `off`
(`synthpass_die::mrz_config_overrides`), but not in the stderr echo, which lists OCR knobs only.

`SYNTHPASS_OCR_AUTO_DOWNLOAD` is gone (issue #491): the extraction path never downloads
models, full stop — a missing `.rten` file fails with an actionable message naming
`synthpass fetch-models` (exit 1, the runtime/extraction-failure bucket below) instead of
fetching one lazily. If the variable is still set, `synthpass` logs one warning that it no
longer has any effect. `synthpass fetch-models` is the only place a model file is ever
fetched: a default build prints each model's pinned URL and SHA-256 (and the target
directory) for a manual `curl` + `sha256sum`; a build with the non-default `download` cargo
feature (propagated from `synthpass-ocr` through `synthpass-pipeline` to `synthpass-cli`, so
a default binary has no `reqwest` in its dependency graph at all) downloads and verifies them
itself, deleting and failing on a checksum mismatch.

`SYNTHPASS_OCR_STOP` and `SYNTHPASS_OCR_CONFIRM_PASSES` are gone (issue #473): the `clean`
retry-stop arm they configured was removed rather than promoted
([re-run](../benchmarks/retry-stop-rerun-2026-09-27.md)). The retry loop always stops on the
first checksum-valid reading, the only default it ever had. If either variable is still set,
loading the OCR models prints one stderr line per variable saying it no longer has any effect,
in every binary.

### MRZ parsing

| Variable | Default | Purpose |
| --- | --- | --- |
| `SYNTHPASS_MRZ_CLASS_SWEEP` | `off` | `mrz`'s uniform confusable-class sweep — `off`/`on`/`control` (`control` is a placebo identical to `off`); an unrecognised value falls back to `off` silently. Read by `synthpass-die`'s `mrz_parse_options`, so it applies to the product's `MrzReader` as well as the benches. Behaviour-changing: recorded in `trace.config_overrides` when not `off`, and refused by `--write-baseline`/`--assert-baseline`; bench reports record the arm as `mrz_class_sweep_arm` |
| `SYNTHPASS_MRZ_LINE1_SELECT` | `on` | The line-1 selector (#574) — `off`/`control`/`on`. After Tier 1 accepts a TD3, TD2, MRV-A or MRV-B zone whose name field breaks the name grammar, `mrz::select_line1` looks in the same OCR text for exactly one other line of the same width, document code and issuer whose name field is well formed; `control` records the proposal and discards it, `on` (the default) applies it, `off` never consults the selector and restores the read as it was before it. It never changes a non-name field and runs no OCR, and a text whose accepted name field is grammatical is untouched. An unset, empty or unrecognised value falls back to `on` silently. Read by `synthpass-die`'s `read_tier1`, so it applies to the product's `MrzReader` as well as the benches; the pipeline's v1 `extracted` record and `PipelineResult.mrz` are routed through the same read, so the v1 and v2 records agree under every arm (the Tier-2 hint is built from it too, but carries check-digit fields only, which the selector never changes). Measured: [line-1 selection A/B, 2026-09-29](../benchmarks/line1-selection-ab-2026-09-29.md) — no outcome changed, no field went from correct to wrong, strict names 13/45 → 15/45, every synthetic format identical seed for seed. Behaviour-changing off its default: `control` and `off` are recorded in `trace.config_overrides`; bench reports record the arm as `mrz_line1_select_arm`, and the OCR-dump run manifest records both MRZ arms as `mrz_arms`; `--write-baseline`/`--assert-baseline` refuse any value but `on` |
| `SYNTHPASS_MRZ_REFUSE_REPEATED_LINE` | `off` | `mrz`'s opt-in refusal of a checksum-valid zone in which two lines are near-identical (#579; `ParseOptions::refuse_repeated_line`) — `off`/`control`/`on` (`control` is a placebo identical to `off`); an unrecognised value falls back to `off` silently. Read by `synthpass-die`'s `mrz_parse_options`, so it applies to the product's `MrzReader` and both benches, but not to the OCR retry loop's stopping rule or the printed-zone validity parse. Both benches count a zone it refuses as a `checksum_failed` miss whose reason names the rule. Not measured as a default. Behaviour-changing: recorded in `trace.config_overrides` when not `off`; bench reports record it as `mrz_refuse_repeated_line_arm` and the run manifest as `mrz_arms.refuse_repeated_line`; `--write-baseline`/`--assert-baseline` refuse any value but `off` |
| `SYNTHPASS_MRZ_DATE_DIGITS` | `off` | `mrz`'s opt-in date-digits rule (#579, the C6 rule; `ParseOptions::date_digits`) — `off`/`control`/`on` (`control` is a placebo identical to `off`); an unrecognised value falls back to `off` silently. With it `on`, a read whose date of birth or date of expiry holds any character other than an ASCII digit or the filler `<` is not `MrzData::valid()`, though its check digits are unchanged (a check digit is arithmetic, and letters have values); a partially unknown or all-filler date stays valid. Read by `synthpass-die`'s `mrz_parse_options`, so it applies to the product's `MrzReader` and both benches, but not to the OCR retry loop's stopping rule or the printed-zone validity parse. Both benches count a read it rejects as a `checksum_failed` miss whose reason names the rule and the date field. Not measured as a default. Behaviour-changing: recorded in `trace.config_overrides` when not `off`; bench reports record it as `mrz_date_digits_arm` and the run manifest as `mrz_arms.date_digits`; `--write-baseline`/`--assert-baseline` refuse any value but `off` |

### Tier-2 model

| Variable | Default | Purpose |
| --- | --- | --- |
| `SYNTHPASS_MODEL_PATH` | `./qwen2.5-1.5b-instruct-q4_k_m.gguf` | GGUF path — any GGUF works (README quickstart convention: `models/`). Setting it also makes a failed Tier-2 check fail `synthpass doctor` (#496) |
| `SYNTHPASS_MODEL_N_CTX` | `2048` | Context window in tokens. The Tier-2 prompt is capped so that it plus the 500-token output budget fits (#506) |
| `SYNTHPASS_MODEL_SHA256` / `SYNTHPASS_MODEL_SKIP_VERIFY` | *(built-in)* / *(unset)* | Re-pin or skip the integrity check |
| `SYNTHPASS_LLM_CONTEXTS` | `1` | Concurrent Tier-2 contexts; raise only if the hardware has room |
| `SYNTHPASS_LLM_GRAMMAR` | on | `0` turns off grammar-constrained decoding. An escape hatch and a measurement switch, not a tuning knob |
| `SYNTHPASS_LLM_MRZ_HINT` | off | `1` folds checksum-verified MRZ fields into the Tier-2 prompt. A rollout gate for a change still being measured (issue #102) |
| `SYNTHPASS_INFERER` | *(unset)* | Read only for old env files: `native` is the only backend, and any other value logs one warning |

### Output

| Variable | Default | Purpose |
| --- | --- | --- |
| `SYNTHPASS_JSON_V1` | *(unset)* | `1` writes the legacy v1 JSON to `<input>.json` instead of `ExtractionV2` ([`V2-DESIGN.md`](../V2-DESIGN.md)) |

### Benchmarks

| Variable | Default | Purpose |
| --- | --- | --- |
| `SYNTHPASS_BENCH_ARCHIVE` | unset | Where `provider-bench` writes its per-document archive ([ADR-0024](../decisions/ADR-0024-per-document-benchmark-archive.md)): a directory (relative to the working directory, like `--out`), `off` (trimmed, any case) to turn it off, or unset or empty for `synthpass-bench-archive/` in the git common directory, which every worktree of a clone shares and git never tracks. A directory inside the working tree is refused unless git ignores it, and a value that is not valid Unicode is a warning, not a root. Any problem is one `warning: archive:` line on stderr and no archive, never a changed exit code, report, ledger, dump or manifest; `--no-archive` wins over the variable, and a `--include-private` run writes none. Not an arm: it changes no read, so it is not recorded in `trace.config_overrides` and does not enter the baseline refusal |

### Server (`synthpass-serve`)

| Variable | Default | Purpose |
| --- | --- | --- |
| `BIND_ADDR` | `127.0.0.1:8080` | Listen address |
| `SYNTHPASS_TOKEN` | *(unset)* | Require `Authorization: Bearer <token>`; **mandatory for non-loopback binds** |
| `SYNTHPASS_TLS_CERT` / `SYNTHPASS_TLS_KEY` | *(unset)* | Enable rustls TLS |
| `SYNTHPASS_MAX_QUEUE_DEPTH` | `4` | Reject uploads with `503` + `Retry-After` once this many Tier-2 requests are queued or in flight. Also read, with a deprecation warning, as the job-queue size when `SYNTHPASS_QUEUE_CAPACITY` is unset |
| `SYNTHPASS_QUEUE_CAPACITY` | `100` | How many completed batch jobs `GET /api/jobs/{id}` can still find |
| `WORK_DIR` / `KEEP_WORK` | `work` / *(unset)* | Scratch directory; keep intermediates for debugging |
| `SYNTHPASS_LOG` | `info` | `tracing` filter directive |
| `SYNTHPASS_LOG_FORMAT` | *(unset)* | `json` for JSON log lines |

### Security and licensing

| Variable | Default | Purpose |
| --- | --- | --- |
| `SYNTHPASS_AUDIT_LOG` | *(unset)* | Append PII-free SHA-256 audit records (JSONL) |
| `SYNTHPASS_KEY` | *(unset)* | Base64 32-byte AES-256 key → encrypt output to `<input>.json.enc` |
| `SYNTHPASS_LICENSE_PATH` | `license.synthpass` | Path to the signed license file |
| `SYNTHPASS_LICENSE_SKIP` | *(unset)* | `1` bypasses license enforcement (development/CI) |
| `SYNTHPASS_LICENSE_PUBKEY` | *(embedded)* | Override the embedded verifying key, for testing |
| `SYNTHPASS_INSTANCE_ID_PATH` | `/var/lib/synthpass/instance-id` | Linux only: where the fingerprint fallback persists its id on a host with no machine-id |
| `SYNTHPASS_LICENSE_PRIVKEY` | *(unset)* | Vendor issuer only (`synthpass-license-issuer`): the signing key. Never read by a shipped binary |

The customer and vendor walkthroughs are in [`LICENSING.md`](../LICENSING.md).

## Exit codes

Decided in #492. Every `synthpass` subcommand follows the same convention, so a script can
branch on the process exit status alone instead of parsing stderr:

| Exit | Meaning |
| --- | --- |
| 0 | success |
| 1 | runtime or extraction failure, including any failed document in a `batch` |
| 2 | usage error (unknown option, bad/missing/surplus arguments, `generate`/`export` argument errors) |
| 3 | license refusal on the extraction path — single-document or `batch` — for a missing, invalid, expired or fingerprint-mismatched license, or a `verify-license` failure |

Notable specifics, where the bucket isn't obvious from the table alone:

- `decrypt`: a missing or malformed `SYNTHPASS_KEY` is a usage/config error (2), since nothing
  about the input file has been touched yet; a missing input file or a decrypt failure (wrong
  key, corrupt ciphertext) is a runtime failure (1).
- `batch`: needs the same valid license single-document extraction does (batch is extraction;
  see [`LICENSING.md`](../LICENSING.md#design)) — that's the one exit-3 case left in this
  command. Exits 1 if *any* document in the batch failed, even when the rest succeeded — the
  summary line still reports the per-document breakdown, but the exit code is what a script
  actually branches on.
- A license feature (`batch` for `synthpass batch`, `export` for `synthpass export`) missing from an otherwise
  valid license is **metered, not refused** (#494, [`BRANDING.md` §5](../BRANDING.md#5-commercial-strategy)): one
  stderr warning, exit code unaffected. `export` is not extraction and never refuses on license grounds; a
  missing or invalid license only adds the same warning.
- `verify-license`: a missing license *file* also exits 3, not 1 — this command's whole purpose
  is to report license validity, so "no license to check" is itself a refusal, the same as an
  invalid or expired one.
- `doctor`: exits 1 if any required check failed (OCR models, license); each failed check
  already printed its own diagnostic line, so the exit code carries no separate message. The
  Tier-2 model is required only when `SYNTHPASS_MODEL_PATH` is set (#496); otherwise a failed
  Tier-2 check prints a `⚠️` line and leaves the exit code alone.
- `generate`/`fingerprint`/`fetch-models` need no license and never exit 3.
- `--json` (single-document and `batch`, issue #493, [ADR-0023](../decisions/ADR-0023-cli-output-contract.md))
  changes only what reaches stdout, never the exit code — the same table above applies unchanged.
- extraction (single-document or `batch`) with a missing OCR model file exits 1, the same
  runtime/extraction-failure bucket any other pipeline error uses — not a separate code, since
  the model was never fetched implicitly and this is exactly the "the pipeline could not run"
  case that bucket already covers (issue #491). `synthpass fetch-models`'s own exit code
  follows the same table: 0 once every model is present and verified (whether freshly fetched
  or already staged), 1 if any fetch or verification fails.

Implemented as one small typed `Exit` enum in `synthpass-cli/src/main.rs`, converted to
`std::process::ExitCode` exactly once in `main` — no `std::process::exit` call exists anywhere in
the crate.
