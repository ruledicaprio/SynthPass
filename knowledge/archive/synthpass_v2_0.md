# SynthPass v2.0 Roadmap (M3 → M5)

> **Superseded (2026-08-02).** Retained as a design record only. [`ROADMAP.md`](../ROADMAP.md) is
> the current, linear M1–M7 plan and wins wherever the two disagree — see its intro for the
> reconciliation note.

## Vision

SynthPass is an offline, air-gapped identity document intelligence platform.
Its competitive advantage is **synthetic, deterministic, perfectly-labelled data**.

The long-term goal is not simply better OCR, but ownership of the complete
document-AI lifecycle:

Synthetic documents → Ground truth → Benchmarking → OCR → MRZ → AI extraction → Continuous improvement.

## M3 – Synthetic Document Factory

### Objectives
- Deterministic synthetic document generation.
- Rich fictional identities (never real PII).
- Capture profiles (mobile, scanner, worn document, border control).
- Modular degradation pipeline.
- Parallel generation.
- Automatic MRZ validation.
- Optional full Tier-1/Tier-2 validation.

### Deliverables
- `synthpass-gen`
- `synthpass generate`
- JSON metadata
- Reproducible seeds
- Capture profiles
- Watermarking

## M4 – Regression & Benchmarking

### Objectives
- Golden datasets.
- Continuous regression testing.
- Performance benchmarking.
- Adversarial red-team generation.
- Documentation and licensing.

### Deliverables
- `synthpass bench`
- CI quality gates
- Benchmark reports
- `knowledge/SYNTHPASS.md`
- `knowledge/ADVERSARIAL.md`

## M5 – Platform Expansion

### Objectives
- TD1, TD2, MRVA/MRVB.
- Declarative layouts.
- Dataset exports (COCO, YOLO, JSONL, HF).
- Fine-tuning dataset generation.
- Plugin architecture.

Commercial direction:
- OSS
- Professional SDK
- Enterprise
- Certification
- Consulting
- Custom models
