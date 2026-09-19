# Architecture Decision Records

Document *why*, not *what*. The code shows what was built; six months from now the
question is always "why didn't we just do the obvious thing?" — and the answer
should already be written down.

## When to write one

When a decision is (a) hard to reverse, (b) contested or non-obvious, or (c) rules
out an approach a reasonable person would try. Not for routine choices.

A rejected alternative recorded with its reason is worth more than the decision
itself. That is the part nobody remembers.

## Format

Numbered `ADR-NNNN-kebab-title.md`, with:

```
Title, Status (Proposed | Accepted | Superseded by ADR-NNNN), Date
Context      — what forced the decision, with evidence
Decision     — what we chose, stated plainly
Alternatives — what we rejected and why (the load-bearing section)
Consequences — positive and negative, honestly
```

Never delete an ADR. Supersede it and link both ways: the wrong turns are part of
the record.

## Index

| ADR | Title | Status |
|---|---|---|
| [0001](ADR-0001-knowledge-tree.md) | Adopt `knowledge/` as the documentation root | Accepted |
| [0002](ADR-0002-provider-model-before-layout-plugins.md) | Build the provider model (M7) ahead of M6 | Accepted |
| [0003](ADR-0003-docs9303-source-of-truth.md) | Adopt `knowledge/docs9303/` as the ICAO spec source of truth | Accepted |
| [0004](ADR-0004-gpu-acceleration.md) | GPU acceleration as an optional, feature-gated path | Accepted (`cuda` only) |
| [0005](ADR-0005-vision-provider-readiness.md) | Vision-provider readiness: can `llama-cpp-2` drive a multimodal GGUF | Accepted (2026-09-17) |
| [0006](ADR-0006-m6-accuracy-first.md) | Reframe M6 to lead with Tier-1 real-document accuracy | Accepted |
| [0007](ADR-0007-dataset-export-format.md) | Dataset export format: adopt the DeepSeek-OCR 0–1000 convention, JSONL first | Accepted |
| [0008](ADR-0008-mrz-detection-track.md) | MRZ detection succeeds sequence completeness as M6's accuracy track | Accepted |
| [0009](ADR-0009-generator-as-a-service.md) | Generator-as-a-service: what it would cost the non-goals | Accepted (2026-09-17, as the pricing) |
| [0010](ADR-0010-benchmark-cost-split-by-role.md) | Split the real-specimen benchmark by role, not by random sample | Accepted (2026-09-17, measured) |
| [0011](ADR-0011-split-m6-packaging-into-m8.md) | Split M6: the deterministic core keeps the number, packaging becomes M8 | Accepted (amended 2026-09-16, 2026-09-17) |
| [0012](ADR-0012-cover-only-specimens-are-a-labelled-class.md) | Cover-only specimens are a labelled class, not a drop | Accepted (amended 2026-09-15) |
| [0013](ADR-0013-names-are-scored-against-mrz-form-truth.md) | Name accuracy is a separate axis, scored only against MRZ-form truth | Accepted |
| [0014](ADR-0014-per-cell-ocrb-classification.md) | `mrz-cell`: per-cell OCR-B classification for the MRZ band, as a benchmark-first prototype | Proposed |
| [0015](ADR-0015-geometric-mrz-band-location.md) | `mrz-locate`: find the MRZ band by geometry, before recognition | Proposed |
| [0016](ADR-0016-card-backs-declare-recoverable-fields.md) | Which fields a document side may be scored on | Accepted |
