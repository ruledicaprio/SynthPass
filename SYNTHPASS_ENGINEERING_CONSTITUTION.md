# SynthPass Engineering Constitution

This document is the authoritative guide for all development on SynthPass. It is intended for both human contributors and AI assistants (Claude Code, etc.).

All design decisions, code reviews, and implementation work must align with this constitution. When in doubt, consult this document.

---

## 1. Mission

SynthPass is an **offline-first, privacy-preserving, deterministic document verification platform**.

It extracts and validates machine-readable zone (MRZ) data and visual fields from identity documents
(passports, ID cards, visas) using a hybrid approach: deterministic algorithms for validation and (optionally)
local LLMs for repair and normalization.

We prioritize user privacy: no data ever leaves the device. We prioritize correctness: every output is backed by confidence scores and validation chains. We prioritize reproducibility: the system is deterministic given the same inputs.

---

## 2. Engineering Philosophy

Whenever multiple solutions exist, rank them in this order — this matches `CLAUDE.md`'s Mission
priorities exactly; `CLAUDE.md` is canonical and this list must not drift from it:

1. **Correctness** – Must produce the right answer for all valid inputs.
2. **Security** – No vulnerabilities, no side‑channel leaks, no data exfiltration. Privacy (no
   data ever leaves the device) is a security property here, not a separate tier.
3. **Deterministic behavior** – Same input → same output, always.
4. **Maintainability** – Code is understandable and changeable. Simplicity is part of this: prefer
   straightforward over clever, avoid speculative abstractions.
5. **Performance** – Meet budgets; do not sacrifice higher priorities for speed.
6. **Developer Experience** – Good tooling and ergonomics, but not at the cost of the above.

**Examples**:
- ✓ Slower but deterministic parsing beats faster heuristic parsing.
- ✓ Explicit validation beats implicit assumptions.
- ✓ A 30‑line readable implementation beats a clever 10‑line abstraction.
- ✓ A local dependency beats a cloud API.
- ✓ Compile‑time guarantees beat runtime checks whenever practical.

---

## 3. Project Goals

- Fully offline, local processing of identity documents.
- Accurate MRZ extraction (TD1, TD2, TD3) with ICAO 9303 compliance.
- High‑quality OCR with confidence values for visual fields.
- Hybrid validation: MRZ checksums take precedence; OCR is evidence.
- Optional local LLM repair (e.g., Qwen) for normalizing OCR output.
- Benchmark‑driven performance optimisation.
- Production‑grade reliability: extensive tests, error handling, logging.
- Extensible architecture to support new document types and data sources.

---

## 4. Non‑goals

- **Cloud integration** – No external APIs, no network calls.
- **Model training** – We do not train OCR or LLM models; we consume them.
- **Real‑time video processing** – Batch or single‑image only.
- **Mobile‑first UI** – CLI and library first; GUIs are secondary.
- **Support for non‑ICAO documents, for the milestones currently committed (M1–M7)** – Focus on
  passports, national IDs, and visas under ICAO Doc 9303. Driving licences, residence permits, and
  other non-9303 document classes are a stated longer-horizon direction, not a permanent exclusion —
  see [§25 Future Vision](#25-future-vision) below and `knowledge/ROADMAP.md`'s Future Work section.

---

## 5. Repository Map

`knowledge/` is the documentation root, not `docs/` — a deliberate decision (PR #72,
`knowledge/decisions/ADR-0001-knowledge-tree.md`). There is no top-level `docs/`, `benches/`,
`resources/`, or `tests/`; per-crate tests live under each crate, and `synthpass-bench` is itself
a workspace member, not a `benches/` directory.

```
synthpass/
├── .github/                 # GitHub workflows, issue templates
├── crates/                  # Workspace crates (see below)
├── knowledge/               # Documentation root: architecture, roadmap, vision, ADRs, papers
│   ├── ARCHITECTURE.md
│   ├── ROADMAP.md
│   ├── VISION.md
│   ├── decisions/           # ADRs
│   └── papers/               # Literature notes backing design decisions
├── samples/                 # Test fixtures, sample documents
├── scripts/                 # CI, dev, and release scripts
├── changelog.d/              # Unreleased changelog fragments
├── fuzz/                     # cargo-fuzz targets
├── web/                       # Browser (WASM) demo
├── Cargo.toml                # Workspace manifest
├── CLAUDE.md
├── CONTRIBUTING.md
├── SECURITY.md
├── LICENSE
└── README.md
```

---

## 6. Workspace Architecture

The workspace (`Cargo.toml` → `members`) is organised into the following crates. Purposes below
are taken from each crate's own top-of-file rustdoc, not invented:

| Crate                | Purpose                                                                                          |
|-----------------------|---------------------------------------------------------------------------------------------------|
| `synthpass-core`      | Canonical extraction schema (`ExtractionV2`, `CoreField`, ...) shared by every producer/consumer. |
| `mrz`                 | Zero-dependency ICAO 9303 MRZ parser/emitter/check-digit validator (TD1/TD2/TD3, MRV-A/B). Published standalone. |
| `mrz-wasm`             | WebAssembly bindings for `mrz` — powers the GitHub Pages browser demo.                            |
| `synthpass-ocr`        | In-process pure-Rust OCR for Tier 1 (`ocrs`/`rten`).                                               |
| `synthpass-llm`        | In-process llama.cpp inference for Tier 2 (Qwen2.5-1.5B-Instruct GGUF via `llama-cpp-2`).          |
| `synthpass-die`        | Document Intelligence Engine: provider contract, capability model, catalog, and `RoutingPolicy` that MRZ/OCR/LLM providers implement (M7). |
| `synthpass-pipeline`   | Orchestrates OCR → Tier 1 MRZ validation → Tier 2 LLM fallback → structured JSON.                  |
| `synthpass-gen`        | Deterministic synthetic TD3 passport data-page generator (test/benchmark data with ground truth).  |
| `synthpass-bench`      | Measures whether `synthpass-gen` output survives the real Tier-1 pipeline (OCR + MRZ checksum).    |
| `synthpass-cli`        | Command-line front-end for the extraction pipeline.                                                |
| `synthpass-serve`      | Web front-end (HTTP API) for the extraction pipeline.                                              |
| `synthpass-license`    | Offline Ed25519-signed licensing for metered enterprise binaries; no phone-home.                   |

Each crate's own rustdoc (`cargo doc --workspace --open`) is the source of truth for its contract;
only the constraints below are independently verified (e.g. CI-enforced), so this section does not
assert per-crate "must not" rules beyond these.

---

## 7. Crate Responsibilities (Verified Contracts)

### 7.1 `synthpass-core`
- Defines the canonical `ExtractionV2` schema, `CoreField`, `ProviderId`, `EscalationKind`,
  `PromptRef`, `ExtractionTrace` vocabulary that every provider and consumer shares.

### 7.2 `mrz`
- Parses/validates MRZ per ICAO 9303 (TD1/TD2/TD3, MRV-A/MRV-B). Zero dependencies by design —
  it is published standalone and consumed outside this workspace too.

### 7.3 `synthpass-ocr`
- In-process OCR (`ocrs`/`rten`); output is raw observations (text + confidence + position). Does
  not interpret data semantically — that is `synthpass-pipeline`'s job.

### 7.4 `synthpass-die`
- **Must not** depend on `tokio`, an OCR engine, `llama.cpp`, or `tracing` — CI-enforced by a
  `cargo tree`-based dependency-boundary check, so that writing a third-party provider never
  starts with "install llama.cpp".
- `Reading` and `Evidence` derive **no** `Serialize` — routing signal cannot reach the document
  JSON by any path. A routing score may be uncalibrated; a number next to `confidence` may not
  (see `knowledge/project_principles.md` §2).

### 7.5 `synthpass-llm`
- Tier 2 only: runs when Tier 1 (MRZ checksum) does not validate. Repairs/normalises; does not
  invent data absent from the input.

### 7.6 `synthpass-pipeline`
- Orchestrates OCR → Tier 1 MRZ validation → Tier 2 LLM fallback → structured JSON. Enforces
  validation rules; MRZ checksum success skips the LLM step entirely.

### 7.7 `synthpass-cli` / `synthpass-serve`
- Thin front-ends (CLI arg parsing / HTTP handlers) over `synthpass-pipeline`. No business logic
  of their own.

---

## 8. Pipeline Architecture

The main data flow is:

```
Image (file/stdin)
  ↓
Normalisation (resize, grayscale)
  ↓
Orientation Detection (auto‑rotate)
  ↓
Deskew
  ↓
Contrast Equalization
  ↓
Region‑of‑Interest Detection (find MRZ zone and visual fields)
  ↓
OCR (extract all text with positions and confidences)
  ↓
Confidence Analysis (per‑field aggregate)
  ↓
Field Detection (map OCR boxes to semantic fields)
  ↓
MRZ Detection (extract MRZ lines)
  ↓
ICAO Validation (check digits, format)
  ↓
Cross Validation (MRZ vs OCR fields)
  ↓
Field Repair (if MRZ valid, use MRZ to correct OCR; else attempt heuristic)
  ↓
LLM Repair (if enabled and needed, normalise fields)
  ↓
JSON Generation (with confidence scores and provenance)
  ↓
Consistency Validation (field‑level checks)
  ↓
Final Confidence Score (overall document trust)
  ↓
Output (JSON / stdout)
```

Each stage is a separate function/struct; they can be tested independently.

---

## 9. OCR Standards

- OCR output is **never** trusted unconditionally.
- Every recognised field must carry:
  - `confidence` (0.0–1.0)
  - `bounding_box` (pixel coordinates)
  - `source_page` (if multi‑page)
  - `preprocessing_profile` (which filters were applied)
- OCR is only **evidence**. Validation decides correctness.
- Confidence thresholds are configurable; default 0.7 for acceptance.

---

## 10. MRZ Standards

- MRZ is **authoritative** when valid.
- If visual OCR conflicts with a valid MRZ, **prefer MRZ**.
- If MRZ fails checksum:
  - Do **not** silently accept it.
  - Attempt repair (e.g., correct single‑digit errors).
  - Record confidence of repair.
  - Explain failure in output metadata.
  - Never fabricate values.
- Transcribe names using ICAO transliteration rules.

---

## 11. JSON Standards

All output JSON must conform to a versioned schema. There is no standalone schema file — the schema
is defined in code as `synthpass-core`'s `v2` module (`ExtractionV2`, `CoreField`, ...) and pinned by
that crate's `tests/schema_keys.rs`, which locks the exact JSON key set. `ExtractionV2.trace`
(providers used, escalation reason if Tier 2 ran) is populated by the pipeline as of the M7 wiring
chunk. The field list below is illustrative of intent, not a verified enumeration — check
`synthpass-core`'s `v2` module and `tests/schema_keys.rs` for the current authoritative shape
before relying on specific field names:
- `document_type` (e.g., "P", "ID", "V")
- `fields` – map of field name → `{ value, confidence, source, bbox? }`
- `mrz` – full MRZ string and parsed data
- `confidence` – overall score
- `processing_metadata` – time, pipeline version, OCR engine, LLM used
- `warnings` – list of issues (e.g., low confidence, checksum failure)

---

## 12. Synthetic Document Standards

For testing and benchmarking, we generate synthetic documents using:
- Realistic MRZ strings (with valid checksums).
- Randomised visual variations (font, skew, noise).
- Ground truth labels for every field.
All synthetic data is stored in `resources/synthetic/` and used in integration tests.

---

## 13. Security Model

- **No network access** – all processing is local.
- **No external dependencies** except those vetted and pinned.
- **Memory safety** – Rust’s ownership model; `unsafe` only in well‑documented FFI wrappers.
- **Input sanitisation** – all image files are validated before processing; malformed files are rejected.
- **No persistent storage** of user data; outputs are written only on user request.

---

## 14. Licensing Model

SynthPass is dual‑licensed under **Apache 2.0** and **MIT** at your option. All contributions must be under these terms.

---

## 15. Performance Budgets

| Metric                      | Budget        |
|-----------------------------|---------------|
| Cold start (first run)      | < 2 s         |
| OCR (per image)             | < 700 ms      |
| MRZ parsing                 | < 1 ms        |
| LLM repair (if enabled)     | < 2 s         |
| End‑to‑end (typical)        | < 3 s         |
| Memory usage (peak)         | < 1.5 GB      |
| Binary size (release)       | < 50 MB       |

These are targets for a typical passport image (2‑4 MB). We monitor them in CI. Prefer streaming and buffer reuse; avoid heap allocation in hot paths.

---

## 16. Error Handling

- Use `thiserror` for library error types; `anyhow` for binaries.
- Errors are recoverable where possible; pipeline should continue with partial results.
- Every error must be logged with context.
- Do not panic in library code; use `Result` and `Option`.

---

## 17. Logging

- Use `tracing` for structured logging.
- Log levels:
  - `error`: unrecoverable failures.
  - `warn`: recoverable but noteworthy (e.g., low confidence, checksum mismatch).
  - `info`: major stages (pipeline start/end, LLM call).
  - `debug`: detailed per‑stage data.
  - `trace`: raw images, bounding boxes.
- Logs are written to stderr; JSON output goes to stdout.

---

## 18. Testing Standards

- **Unit tests**: in each crate, exercise all public functions.
- **Integration tests**: end‑to‑end on synthetic and real (sanitised) documents.
- **Property‑based tests**: e.g., round‑trip MRZ generation/parsing.
- **Fuzzing**: for MRZ, JSON, and image decoding.
- Coverage target: > 80% for core crates (`mrz`, `pipeline`).
- All tests must pass in CI.

---

## 19. Benchmarking

- Use Criterion for micro‑benchmarks.
- Maintain a baseline to detect regressions.
- Benchmarks are run in CI and results are compared against `main`.
- Standard datasets (e.g., MIDV‑2020) are used for macro‑benchmarks.

---

## 20. Documentation Standards

- Every public item must have `rustdoc` comments.
- The `knowledge/` directory contains design documents, ADRs, and roadmaps.
- The `README.md` gives a high‑level overview.
- `CONTRIBUTING.md` explains development workflow.
- **Knowledge‑Driven Development**: before changing architecture, consult — this order matches
  `CLAUDE.md`'s "Repository knowledge priority":
  1. `README.md`
  2. `knowledge/` (`ARCHITECTURE.md`, `ROADMAP.md`, `VISION.md`, `decisions/` for ADRs)
  3. `knowledge/ROADMAP.md`
  4. `knowledge/ARCHITECTURE.md`
  5. `CONTRIBUTING.md`
  6. `SECURITY.md`

When code diverges from documented architecture: identify the mismatch, explain it, and either update the code or update the documentation. Never leave them inconsistent.

---

## 21. GitHub Workflow

We follow a predictable lifecycle:

1. **Issue** – filed by user or maintainer.
2. **Design proposal** – if non‑trivial, an ADR is drafted in `knowledge/decisions/`.
3. **Discussion** – on issue/PR.
4. **Implementation branch** – `feature/<name>` or `fix/<name>`.
5. **Claude CLI** or human developer writes code.
6. **Pre‑commit checks**:
   - `cargo fmt`
   - `cargo clippy -- -D warnings`
   - `cargo test`
   - `cargo bench` (compare to baseline)
7. **Documentation update** – if needed.
8. **Pull Request** – against `main`.
9. **Claude Web Review** – or human review.
10. **Merge** – squash or rebase.

---

## 22. Claude Web (GitHub) Role

When acting in a GitHub context (issues, PRs, discussions), Claude should act as:

- **Architect** – evaluate design proposals.
- **Reviewer** – check code against constitution.
- **Planner** – break down large tasks.
- **Historian** – summarise past decisions.
- **CI Investigator** – analyse test/benchmark failures.
- **Release Manager** – prepare release notes.
- **Documentation Maintainer** – ensure docs stay current.

Before coding, inspect: open issues, discussions, PRs, milestones, and architecture docs. After coding, review diff, tests, benchmarks, documentation, propose improvements, and summarise impact.

---

## 23. Claude CLI Role

When using Claude Code locally, Claude should act as:

- **Engineer** – implement code.
- **Debugger** – locate and fix bugs.
- **Refactoring assistant** – improve code quality.
- **Benchmark runner** – run and interpret benchmarks.
- **Cargo expert** – manage dependencies and workspace.
- **Test writer** – write new tests.

CLI should avoid redesigning architecture without first consulting the documented project direction.

---

## 24. Review Checklist

Every pull request must be checked against this list:

- [ ] Code follows crate contracts (section 7).
- [ ] No unnecessary external dependencies.
- [ ] No `unsafe` unless explicitly justified.
- [ ] All new public items documented.
- [ ] Unit/integration tests cover new functionality.
- [ ] Benchmarks show no regression (or regression is explained).
- [ ] Logging is appropriate (not too noisy).
- [ ] Error handling is comprehensive.
- [ ] Documentation updated (README, knowledge/, rustdoc).
- [ ] CI passes (fmt, clippy, test, bench).

---

## 25. Future Vision

SynthPass will evolve to support:

- More document types (driver's licenses via AAMVA PDF417, residence permits, mobile driving
  licences under ISO/IEC 18013 — see `knowledge/ROADMAP.md`'s "Beyond ICAO 9303" Future Work
  entry for the scoping detail on each).
- Additional barcode parsing (PDF417, Aztec).
- GPU acceleration for OCR and LLM.
- Plugin system for custom pre/post‑processing.
- WebAssembly compilation for browser deployment.
- Formal verification of MRZ parsing.

All additions must remain true to the core philosophy: offline, private, deterministic, correct.
This is the longer horizon §4's non-ICAO-documents non-goal points at: not committed for M1–M7, but
not ruled out either.

---

**This constitution is a living document.** Updates are made via PRs and require consensus among maintainers.

*Last amended: 2026‑07‑28*