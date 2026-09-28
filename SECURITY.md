# Security Policy

**SynthPass** processes **personally identifiable information** (passports, ID cards).
Security is a first-class concern; this document describes the posture and how to report
issues.

## Supported versions

Solo-maintained project, patch releases only — the latest `1.5.x` release is the only supported
one. Pre-1.5 versions are unmaintained; upgrade to `1.5.x` for any security-relevant fix.

| Version | Supported |
| --- | --- |
| 1.5.x | ✅ |
| < 1.5 | ❌ |

## Reporting a vulnerability

Please report privately — **do not** open a public issue for a security bug.

- Email **rusmirskopljak@gmail.com** with a description, reproduction steps, and impact.
- Or use GitHub's [private vulnerability reporting](https://docs.github.com/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability) on the repository.

Expect an acknowledgment within a few days. Coordinated disclosure is appreciated; please allow
time for a fix before any public write-up.

## Security posture

- **Air-gapped by design.** No cloud calls in the processing path; all OCR and LLM inference run on
  the local host / loopback. No telemetry.
- **Loopback by default.** `synthpass-serve` binds `127.0.0.1`. It **refuses a non-loopback bind unless
  `SYNTHPASS_TOKEN` is set**, and then enforces `Authorization: Bearer <token>` on every request.
- **Transport security.** Optional rustls TLS via `SYNTHPASS_TLS_CERT` / `SYNTHPASS_TLS_KEY`. When exposed
  beyond loopback, terminate TLS (directly or via a reverse proxy) and keep the bearer token secret.
- **PII hygiene.** Uploaded files and intermediate artifacts are deleted after each request
  (`KEEP_WORK=1` retains them only for local debugging).
- **At-rest options.** `SYNTHPASS_AUDIT_LOG` writes a **PII-free** SHA-256 audit trail (document
  fingerprint + method + timestamp, never names/numbers). `SYNTHPASS_KEY` (base64 32-byte AES-256-GCM)
  encrypts the output JSON to `<input>.json.enc`; read it back with `synthpass decrypt`.
- **Deterministic core.** Tier 1 (ICAO 9303 MRZ) is checksum-verified math, not a model — no
  hallucinated identity fields when a valid MRZ is present.
- **Model & license integrity.** The GGUF, both OCR `.rten` weight files (or their compile-time
  embedded equivalents, see below), and every license file are SHA-256/Ed25519-verified before
  use — a tampered or substituted file fails closed rather than running silently.
- **PII memory hardening (v0.9.0, best-effort).** The highest-value in-memory PII carriers
  (extracted fields, the AES key, raw Tier-2 output) are wiped on drop via `zeroize`. This does
  not cover every intermediate copy (`serde_json::Value` internals) or on-disk plaintext
  artifacts (only the optional `SYNTHPASS_KEY`-encrypted output is protected at rest) — see
  [the exact scope](#pii-memory-hardening-scope) below, stated plainly rather than oversold.
- **Fuzz-tested ingest path (v0.9.0).** The untrusted-OCR-text repair logic in `mrz` (also the
  public WASM demo's parser) is covered by an always-on `proptest` suite plus opt-in
  coverage-guided `cargo-fuzz` — [scope below](#fuzzing-scope).
- **Static, air-gapped binary (v1.0.0).** The `x86_64-unknown-linux-musl` release build embeds
  OCR models at compile time and makes zero runtime network calls — nothing to compromise over
  the wire because there is no wire. Licensing is not hardware-bound (root can read the machine
  fingerprint, a from-source rebuild bypasses the check) — a compliance/metering mechanism, not
  DRM; see [knowledge/LICENSING.md](knowledge/LICENSING.md#threat-model) for the full threat
  model.

### PII memory hardening scope

Wiped on drop (`zeroize`): `synthpass_core::Extraction` (`ZeroizeOnDrop`); the AES-256 key
(`Zeroizing<[u8; 32]>`, from `key_from_base64` through `Pipeline`); the pretty-printed JSON
string persisted or encrypted in `write_outputs`; and the raw Tier-2 output and prompt in
`synthpass-llm`'s `generate`. `mrz::MrzData` gets the same treatment behind the optional
`zeroize` feature, so `mrz-wasm`'s `wasm32-unknown-unknown` build, which never enables it,
stays dependency-free. Its `date_of_birth`, `date_of_expiry` and `sex` are typed values
(`MrzDate`, `Sex`, ADR-0019) wiped through their own `Zeroize` impls: `MrzDate` clears its
payload but keeps its variant, and `Sex` is fully overwritten. `format`,
`document_number_legacy_encoding` and `checks` are `#[zeroize(skip)]`.

Not covered:

- the internal copies `serde_json::Value`, `to_string` and `to_value` allocate while building
  JSON — wiping them would mean threading a custom secret type through `serde`;
- the `<input>.md` and `<input>.json` plaintext files on disk — only the optional
  `SYNTHPASS_KEY`-encrypted `.json.enc` is protected at rest;
- `PipelineResult.markdown`, deliberately a plain `String`: wrapping it would force a
  caller-side API break or an extra unwiped copy;
- a live-process attacker, or the OS paging memory to swap before a value drops.

### Fuzzing scope

`mrz::find_and_parse` and `parse_td1`/`parse_td2`/`parse_td3` are the one ingest component that
both handles untrusted OCR text with byte-index repair logic and compiles to WASM for the
public demo. They carry:

- `crates/mrz/tests/fuzz_props.rs` — `proptest` "never panics" properties over domain-biased
  generators and mutations of the crate's own ICAO specimens, run by `cargo test --workspace`
  on every push;
- `fuzz/` — a `cargo-fuzz` crate in its own detached workspace, seeded from the same specimens
  plus OCR-garbled samples. It runs in the opt-in `workflow_dispatch` `fuzz` CI job, not a
  required check, because it needs nightly and its timing is not deterministic.

Out of scope, as documented follow-ups: image decoding (the `image` crate, fuzzed upstream) and
`synthpass-llm`'s `parse_extraction`, which would drag the `llama-cpp-2` build into the fuzz
job.

## Hardening checklist for production

- [ ] Set a strong, unique `SYNTHPASS_TOKEN`.
- [ ] Enable TLS (`SYNTHPASS_TLS_*`) or front with a TLS-terminating reverse proxy.
- [ ] Set `SYNTHPASS_KEY` to encrypt outputs at rest; store the key in a secrets manager.
- [ ] Enable `SYNTHPASS_AUDIT_LOG` and ship the log to your SIEM.
- [ ] Keep the GGUF model and containers/binaries on trusted, access-controlled hosts.
- [ ] Run `cargo deny check advisories` in CI (no Python dependencies remain as of v0.7.5).
