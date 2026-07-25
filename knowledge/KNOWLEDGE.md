# /knowledge/

> **Editor's note.** This is the original proposal, kept in the author's voice.
> It was adopted with variations — see
> [decisions/ADR-0001-knowledge-tree.md](decisions/ADR-0001-knowledge-tree.md)
> for what was actually built and why it differs. Two differences matter when
> reading the paths below: the proposed `architecture/` folder was not created
> (that material lives in [ARCHITECTURE.md](ARCHITECTURE.md) and in
> `decisions/`), and `benchmarking/` shipped as
> [benchmarks/](benchmarks/). Where this note and
> [README.md](README.md) disagree, README.md describes the tree that exists.

We're making one of the highest ROI decisions for a long-lived project.

What we build now isn't just documentation—it's **institutional memory**.

Codebase will evolve, models will come and go, but if the reasoning
behind your architecture is preserved, future work becomes much easier.

## `knowledge/` is like a technical library rather than a docs folder

```
knowledge/
│
├── architecture/
│   ├── vision-provider-interface.md
│   ├── confidence-engine.md
│   ├── document-fusion.md
│   ├── benchmark-framework.md
│   ├── prompt-system.md
│   └── roadmap-v1.3.md
│
├── providers/
│   ├── moondream2.md
│   ├── qwen-vl.md
│   ├── gemma3-vision.md
│   ├── smolvlm.md
│   ├── internvl.md
│   └── provider-comparison.md
│
├── ocr/
│   ├── preprocessing.md
│   ├── confidence-scoring.md
│   ├── layout-analysis.md
│   ├── open-set-recognition.md
│   └── recognition-pipeline.md
│
├── research/
│   ├── papers/
│   ├── summaries/
│   └── experiments/
│
├── prompts/
│   ├── philosophy.md
│   ├── json-schema.md
│   ├── moondream.md
│   ├── qwen.md
│   └── generic.md
│
├── hardware/
│   ├── gtx970.md
│   ├── cpu-only.md
│   ├── memory-budget.md
│   └── deployment.md
│
├── benchmarking/
│   ├── methodology.md
│   ├── datasets.md
│   ├── metrics.md
│   └── results/
│
└── decisions/
    ├── ADR-0001-vpi.md
    ├── ADR-0002-confidence-engine.md
    └── ADR-0003-provider-registry.md
```

---

## Suggestion I strongly recommend: Architecture Decision Records (ADRs)

This is something many mature projects use, but it works especially well with AI-assisted development.

Instead of just documenting *what* you built, document *why*.

Example:

```
ADR-0001

Title:
Introduce Vision Provider Interface

Status:
Accepted

Date:
2026-07-25

Context

Current Tier 2 depends directly on Qwen.

This tightly couples OCR fallback to a single model family.

Decision

Introduce a provider abstraction.

Consequences

Positive

✓ easier benchmarking

✓ future models

✓ customer flexibility

✓ cleaner code

Negative

slightly more abstraction
```

Six months from now, when someone asks "Why didn't we just call Moondream directly?", the answer is already there.

---

# Let's build, not throw rocks...

I'd rather avoid one enormous prompt.

Instead, i choose to treat Claude like a new senior engineer joining the team.

I'd feed information in this order:

## 1. Philosophy

```
What is SynthPass?

Why does it exist?

Who is it for?

What problems are we solving?

What are we deliberately NOT building?
```

Claude is very good once it understands the philosophy.

---

## 2. Current architecture

Only after philosophy.

```
Current crates

Current OCR

Current MRZ

Current Tier 2

Current API
```

---

## 3. Desired architecture

Then

```
Vision Provider Interface

Provider Registry

Confidence Engine

Fusion Engine

Benchmarking

Prompt abstraction
```

---

## 4. Constraints

This is often forgotten.

For example

```
Runs air-gapped

No cloud

MIT license

Rust-first

Cross-platform

No Python runtime

Minimal dependencies

CPU-first

Consumer GPUs
```

Claude should know these before proposing solutions.

---

# I'd also create something called

```
knowledge/project_principles.md
```

This becomes your project's constitution.

Something like:

```
Principle 1

Deterministic whenever possible.

AI only fills uncertainty.

---

Principle 2

Every AI decision should expose confidence.

---

Principle 3

Models are replaceable.

Architecture is permanent.

---

Principle 4

Synthetic data before private data.

---

Principle 5

Offline-first.

---

Principle 6

Benchmark everything.

Never trust anecdotes.

---

Principle 7

No hidden magic.

Every extraction should be explainable.
```

Those seven principles will influence hundreds of implementation decisions.

---

# One thing I'd ask Claude to help maintain

A living file:

```
knowledge/technical_debt.md
```

Not bugs.

Not TODOs.

Technical debt.

Example

```
High

Current OCR confidence too simplistic.

Need weighted scoring.

Estimated effort:
2 days.

--------------------

Medium

Prompt versioning tied to providers.

Need abstraction.

--------------------

Low

Current JSON parser assumes strict output.
```

Claude is surprisingly good at keeping this current if you update it periodically.

---

## Finally, one small but potentially very impactful idea

As your knowledge base grows, don't treat it as "documentation"—treat it as the **source of truth**.

The code should implement the architecture described there, not the other way around.

If I were onboarding a new engineer (or another AI) to SynthPass, I'd want them to spend their first hour reading just five files:

1. `knowledge/project_principles.md`
2. `knowledge/architecture/vision-provider-interface.md`
3. `knowledge/architecture/document-fusion.md`
4. `knowledge/benchmarking/methodology.md`
5. `README.md`

If those five documents are clear and kept current, someone can understand the project's philosophy,

constraints, and direction before they ever open a Rust source file. That tends to produce much more 

consistent design decisions over the lifetime of a project.
