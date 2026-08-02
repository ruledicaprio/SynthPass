# This document contains proposals that should be implemented along DIE (Document Intelligence Engine)

> **Superseded in part (2026-08-02).** [`ROADMAP.md`](ROADMAP.md)'s M7 section is the current,
> shipped Document Intelligence Engine plan and explicitly rejects this doc's vision-provider
> push for v1.3.0 ("No vision-language model in v1.3.0 ... Moondream is a v1.4.0 spike"). Treat
> the proposals below as historical brainstorming, not a current commitment.

---

# Versioning

I actually think about this.

```
v1.2.x
│
├── bugfixes
├── OCR improvements
├── MRZ stability
└── existing Qwen fallback

↓

v1.3.0
```

I think **1.3.0** is justified because you're introducing a new subsystem, not merely swapping one model for another.

Something like

```
SynthPass 1.3.0

Major Features

✓ Vision Provider Interface (VPI)
✓ Provider registry
✓ Confidence engine
✓ Multi-provider architecture
✓ Moondream provider
✓ Benchmark framework
✓ Prompt versioning
```

That is definitely a **minor release** under SemVer.

Cargo MRZ remaining at **0.5.1** is perfectly fine. Don't churn dependencies without reason.

---

# I'd avoid making Moondream or other LLM "the feature"

Instead I'd advertise

```
SynthPass

Vision Provider Interface™

Compatible with

✓ Moondream
✓ Qwen-VL
✓ Gemma Vision
✓ SmolVLM
✓ future providers
```

Customers buy flexibility.

Not Moondream.

---

# Claude context inference

Claude should create **lots of context** divided into layers.

Claude performs dramatically better when information is modular.

Instead of

```
150k tokens
```

structure it like,

```
Project

README.md

Architecture

ARCHITECTURE.md

Current OCR

OCR.md

Tier 2 Vision

VISION.md

Roadmap

ROADMAP.md

Current APIs

API.md

Benchmark goals

BENCHMARK.md

Provider interface

VISION_PROVIDER_INTERFACE.md
```

---

# Token estimation for Opus 5 medium effort

For Opus 5 I'd roughly target:

## Session start

```
15–30k tokens
```

Repository philosophy

Architecture

Goals

Current implementation

Roadmap

Perfect.

---

## During implementation

Each coding task

```
5–15k tokens
```

Rather than

```
100k every prompt
```

---

## Entire feature

I could easily see

```
200k–400k tokens
```

over many conversations.

That's normal.

---

# OCR book

I actually think this is where people waste time.

The book is

> Open-Set Text Recognition: Concepts, Framework and Algorithms

This is an academic reference.

Your question is

> Is it worth summarizing?

My answer is

## YES...

...but **not** because Claude needs OCR theory.

Claude already knows most OCR architectures.

The value is extracting

```
Design principles
```

not

```
Definitions
```

---

I'd ask Claude something like

> Ignore introductory material.
>
> Ignore historical OCR methods.
>
> Extract only reusable architectural ideas that could improve SynthPass.

---

I'd want things like

```
Open-set recognition

Unknown script detection

Confidence calibration

Character uncertainty

Language independence

Layout reasoning

Recognition fusion

Error recovery

Confidence propagation

Multi-stage recognition

Recognition voting

Hierarchical decoding

Adaptive preprocessing
```

Those become engineering features.

---

# I would NOT dump the entire book into VPI

Instead I'd create

```
research/

open-set-ocr.md

contains

Only actionable ideas.
```

Maybe

```
20 pages

instead of

450 pages
```

---

# My "80/20" rule

Whenever I read an academic paper I ask

Can this become

```
trait

struct

algorithm

pipeline

benchmark

configuration

heuristic
```

If not...

Don't keep it.

---

# I think VPI deserves its own design document

Honestly I'd create

```
docs/

VISION_PROVIDER_INTERFACE.md
```

containing

```
Goals

Non-goals

Provider lifecycle

Capability discovery

Prompt abstraction

Inference abstraction

JSON schema

Grammar support

Bounding boxes

Streaming

Memory estimation

Benchmark API

Provider registration

Future providers

Compatibility matrix
```

This becomes your "contract."

Everything else implements it.

---

# Something I would push even further

This is my favorite idea.

Don't call it

```
Vision Provider Interface
```

Call it

```
Document Intelligence Engine
```

Inside it

```
Providers

OCR Provider

Vision Provider

MRZ Provider

Barcode Provider

Face Provider

Security Feature Provider

Layout Provider
```

Then

```
trait IntelligenceProvider
```

instead of

```
trait VisionProvider
```

Now in two years someone can plug in

```
Passport UV feature detector

Chip reader

NFC parser

PDF417 reader

Hologram detector
```

without changing the architecture.

That scales beautifully.

---

# Last suggestion (the one I'd absolutely implement)

Create a repository called

```
knowledge/
```

not

```
docs/
```

Structure it like this:

```
knowledge/

vision/

ocr/

benchmarks/

research/

hardware/

providers/

prompts/

evaluation/

papers/
```
## SUGGESTION TO ME AND Claude

Every time when we read a paper, benchmark a model, or discover a useful technique,
distill it into a concise Markdown note (1–4 pages) focused on actionable engineering insights.

Over time, this becomes a curated knowledge base for both you and me,
rather than a collection of hundreds of scattered PDFs.

I have a feeling that in a year's time, the real competitive advantage of SynthPass won't
just be the Rust code—it will be that accumulated engineering knowledge and the clean architecture
built around it. That's much harder to replicate than swapping in the latest vision model.
