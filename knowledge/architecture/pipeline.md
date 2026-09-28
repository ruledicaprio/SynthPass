# The extraction pipeline

How one document moves from an image to JSON, and what runs where. Checked against the tree at
v1.7.0 (`86987fe`, 2026-09-28). This is [`ARCHITECTURE.md`](../ARCHITECTURE.md) §5.

Each crate's rustdoc is the source of truth for its API. This page is the map between them.
It carries no measured numbers; those are in
[`benchmarks/README.md`](../benchmarks/README.md).

## The sequence

*Who calls whom for one document, in order?* `Pipeline::process_document` and its
streaming twin, `process_document_stream` (`crates/synthpass-pipeline/src/lib.rs`), run
every step. The CLI and the web server call them; neither holds extraction logic of its
own. License checks happen in the front-end before this sequence starts
([`LICENSING.md`](../LICENSING.md#design)).

```mermaid
sequenceDiagram
    autonumber
    participant F as synthpass-cli / synthpass-serve
    participant P as synthpass-pipeline
    participant O as OcrEngine (synthpass-ocr)
    participant C as ProviderCatalog (synthpass-die)
    participant L as LlmFieldReader → InferBackend (synthpass-llm)

    F->>P: process_document(path)
    P->>O: recognize_detailed(image)
    O-->>P: text + layout geometry (general pass, then MRZ retry passes)
    Note right of P: writes <input>.md
    P->>C: deterministic reader (MrzReader) reads the text
    C-->>P: Reading + Evidence
    Note right of P: RoutingPolicy::decide(evidence)
    alt Accept — checksum-valid MRZ
        Note right of P: builds ExtractionV2 from the MRZ, Tier 2 skipped
    else Escalate { reason, budget }
        Note right of P: acquires an llm_semaphore permit (queue depth +1)
        P->>L: read(markdown, optional MRZ hint)
        L-->>P: Extraction (already normalized)
        Note right of P: releases the permit (queue depth −1),<br/>then apply_deterministic_mrz overwrites the model's<br/>fields with the check-verified MRZ ones
    end
    Note right of P: writes <input>.json (or .json.enc),<br/>then appends the audit record
    P-->>F: PipelineResult
```

## The Tier-1 gate

*How does an image become `Accept` or `Escalate`?* OCR runs a general full-page pass.
When that pass holds no checksum-valid MRZ, a second recognizer, constrained to the MRZ
character set, re-reads preprocessed crops of the image until one validates or the pass
and time budgets run out. The retry loop, its variants and its budgets are documented in
`synthpass-ocr`'s crate docs. The preprocessing is `synthpass-imageprep`.

```mermaid
flowchart TD
    IMG["Image"] --> GEN["General OCR pass<br/>(OcrEngine::recognize_detailed)"] --> CHK{"Checksum-valid<br/>MRZ in this pass?"}
    CHK -- "yes" --> TEXT["OCR text (markdown)"]
    CHK -- "no" --> RETRY["MRZ-charset retry loop over preprocessed crops<br/>— stops on: a crop validates; the pass budget<br/>(SYNTHPASS_OCR_MAX_PASSES); the time budget<br/>(SYNTHPASS_OCR_MAX_SECONDS); or variants exhausted"]
    RETRY --> TEXT
    TEXT --> READER["MrzReader<br/>(synthpass-die)"] --> PARSE["mrz::find_and_parse"] --> DECIDE{"RoutingPolicy::decide"}
    DECIDE -- "MRZ found, checksums valid" --> ACCEPT["Accept — Tier 2 skipped"]
    DECIDE -- "not found, or a check digit failed" --> ESCALATE["Escalate { reason, budget }"]
```

**Sources:** `crates/synthpass-pipeline/src/lib.rs` `Pipeline::ocr_and_tier1`; `crates/synthpass-ocr/src/lib.rs` `NativeOcr::recognize_detailed`, `SYNTHPASS_OCR_MAX_PASSES`, `SYNTHPASS_OCR_MAX_SECONDS`; `crates/synthpass-die/src/mrz_reader.rs` `MrzReader::read`; `crates/synthpass-die/src/routing.rs` `RoutingPolicy::decide`, `Decision`; `crates/synthpass-core/src/v2.rs` `EscalationKind`.

The pipeline then asks the catalog for its free, deterministic reader, `MrzReader`. It calls
`mrz::find_and_parse`, which tries the five formats in the fixed order
[`ARCHITECTURE.md` §13.3](../ARCHITECTURE.md#133-mrz-handling-policy) records.
`RoutingPolicy::default().decide` turns the reader's `Evidence` into `Accept` or
`Escalate { reason, budget }`. `Accept` means the MRZ was found and its checksums are valid,
and Tier 2 is not run at all. `Evidence` and `Reading` never reach the output JSON
([§13.2](../ARCHITECTURE.md#132-synthpass-die-dependency-boundary)).

`EscalationKind` has five variants, checked in this order: `MrzNotFound`, `MrzChecksumFailed`,
`Line1Flagged`, `OcrBelowSanityFloor`, `FieldsMissing`. Only the first two are reachable under
the default policy above — the rest are opt-in signals `RoutingPolicy::default()` leaves off.

## Two Tier-2 paths

*Where do the unary and streaming paths split and rejoin? (Dashed: the streaming path's
token-delta side channel, sent while it runs, not instead of its final `Extraction`.)*
Tier 2 is assembled on two paths:

- **`process_document`** — the unary path. It goes through the catalog to `LlmFieldReader`.
  The CLI and batch jobs (`POST /api/extract/batch`) use it.
- **`process_document_stream`** — the streaming path. It calls
  `InferBackend::extract_stream` directly and forwards token deltas as they arrive.
  `POST /api/extract` uses it.

```mermaid
flowchart LR
    subgraph "Unary — process_document"
        PD["process_document"] --> CAT["ProviderCatalog::find_reader"] --> LFR["LlmFieldReader::read"] --> EXT["InferBackend::extract"]
    end
    subgraph "Streaming — process_document_stream"
        PDS["process_document_stream"] --> EXTS["InferBackend::extract_stream"]
        EXTS -. "token deltas, in-flight<br/>(ProcessEvent::Delta)" .-> SSE["SSE 'delta' events<br/>(synthpass-serve)"]
    end
    EXT --> NORM["normalize<br/>(synthpass_core::normalize::extraction)"]
    EXTS --> NORM
    NORM --> MRZFIX["apply_deterministic_mrz"] --> OUT["outputs<br/>(&lt;input&gt;.json, or the SSE 'result' event)"]
```

**Sources:** `crates/synthpass-pipeline/src/lib.rs` `Pipeline::process_document`, `Pipeline::process_document_stream`, `run_tier2`, `Pipeline::extract_via_inferer_stream`, `apply_deterministic_mrz`; `crates/synthpass-pipeline/src/llm_reader.rs` `LlmFieldReader::read`; `crates/synthpass-pipeline/src/infer.rs` `InferBackend`; `crates/synthpass-core/src/normalize.rs` `extraction`; `crates/synthpass-serve/src/main.rs` (the `delta`/`result` SSE events); `crates/synthpass-pipeline/src/jobs.rs` (batch jobs call `process_document`).

**A Tier-2 change lands on both paths.** A fix applied to one path once missed the other, and
every single-document web upload kept the bug. `apply_deterministic_mrz` is the step both paths
share, and `both_tier2_paths_produce_the_same_extraction` (a pipeline test) asserts that they
agree. Both paths normalize the raw `Extraction` *before* `apply_deterministic_mrz` runs — not
after, which would let the model's own formatting overwrite a field the MRZ read had just fixed.
Why streaming sits outside the provider contract is in
[`technical_debt.md`](../technical_debt.md#streaming-bypasses-the-provider-contract).

`NativeInferer` runs `llama.cpp` generation inside `spawn_blocking`, because it is CPU-bound
and would stall the async executor. The model loads once, on first use, and stays warm.
Decoding is greedy and grammar-constrained by default. Streaming deltas use a non-blocking
`try_send`, so a stalled browser can never extend how long a Tier-2 permit is held.

## Concurrency

Two semaphores bound the work. A document holds at most one permit at a time, so the two
can never deadlock against each other.

| Stage | Bounded by | Default |
| --- | --- | --- |
| OCR | `ocr_semaphore`, sized by `SYNTHPASS_OCR_THREADS` | host cores − 1, at least 1 |
| Tier 2 | `llm_semaphore`, sized by `SYNTHPASS_LLM_CONTEXTS` | 1 |

`llm_queue_depth` counts Tier-2 calls queued or in flight. `synthpass-serve` reads it to
refuse new uploads with `503` before they queue behind the semaphore. Batch jobs run through
`crates/synthpass-pipeline/src/jobs.rs` against the same two semaphores.

## Outputs

| Artifact | Written when | Contents |
| --- | --- | --- |
| `<input>.md` | always | the OCR text |
| `<input>.json` | an extraction succeeds and `SYNTHPASS_KEY` is unset | `ExtractionV2` ([`V2-DESIGN.md`](../V2-DESIGN.md)); the v1 shape only under `SYNTHPASS_JSON_V1=1` |
| `<input>.json.enc` | `SYNTHPASS_KEY` is set | the same JSON, AES-256-GCM encrypted; `synthpass decrypt` reads it back |
| audit record | with the JSON, when `SYNTHPASS_AUDIT_LOG` is set | one PII-free JSONL line: document hash, method, timestamp |

The in-memory result is `PipelineResult`. It carries both the v1 `extracted` value and
`extracted_v2`; new consumers read `extracted_v2`.

## Front-ends

**`synthpass` (CLI).** The commands are in `synthpass --help` and the
[README](../../README.md#using-the-cli). What reaches stdout versus stderr, and `--json`, is
[ADR-0023](../decisions/ADR-0023-cli-output-contract.md). Exit codes are in
[`configuration.md`](configuration.md#exit-codes).

**`synthpass-serve` (axum).**

| Route | Does |
| --- | --- |
| `GET /` | the embedded upload page |
| `POST /api/extract` | one multipart upload (20 MB limit), answered as an SSE stream: `delta` events while Tier 2 generates, then one `result` event |
| `POST /api/extract/batch` | submits a batch job |
| `GET /api/jobs/{id}` | a batch job's state |
| `GET /metrics` | counters and latency histograms; counts and durations only, never field content |
| `GET /health` | OCR engine, inference backend and license expiry. Outside the auth layer on purpose: infrastructure probes rarely carry credentials |

The `result` event carries `filename`, `markdown`, `extracted`, `extracted_v2`, `method`,
`mrz` and `error`. A client that negotiates the v1 schema gets no `extracted_v2`
([`V2-DESIGN.md`](../V2-DESIGN.md)). A Tier-2 failure still returns the OCR text beside the
error. Uploads and intermediates are deleted after each request unless `KEEP_WORK=1`.

A non-loopback bind without `SYNTHPASS_TOKEN` is refused at startup. The server's variables
are in [`configuration.md`](configuration.md#server-synthpass-serve).
