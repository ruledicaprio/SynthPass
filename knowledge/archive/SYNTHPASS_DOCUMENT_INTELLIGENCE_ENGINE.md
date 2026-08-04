> **Archived (M6).** Duplicate of the argument in
> [`../DOCUMENT_INTELLIGENCE_ENGINE.md`](../DOCUMENT_INTELLIGENCE_ENGINE.md) (a provider
> interface, not a single VLM) — kept here for provenance, not as a living doc.
> `../ROADMAP.md`'s M7 section is the shipped, authoritative account.

# Creators remarks

I actually think this is a really good point in the project
to stop thinking about "which model is fastest" and instead
define what Tier 2 fallback? is supposed to accomplish.


Yes. I actually think this is a really good point in the project to stop thinking about "which model is fastest" and instead define **what Tier 2 is supposed to accomplish**.

After reading your current README and the attached notes, I think the current proposal is still too focused on *replacing Qwen with Moondream*. I would push Claude much harder.

---

# My assessment

I would **not** design SynthPass around **one** VLM.

I'd design it around a **Vision Provider interface**.

Today that provider might be

* Moondream2
* Qwen2.5-VL
* Gemma3 Vision
* SmolVLM
* NanoVLM
* InternVL
* future Phi-4 Vision

without changing a single line of OCR pipeline logic.

Future customers won't ask

> "Does it use Moondream?"

They'll ask

> "Can I use my own model?"

That is a much more valuable architecture.

---

## Current architecture

Current README implies

```
OCR
 ↓
MRZ validation
 ↓
Qwen fallback
```

I'd replace it with

```
Image
   │
   ▼
Image Quality Analyzer
   │
   ├─────────────┐
   ▼             ▼
MRZ detector   OCR detector
   │             │
   ▼             ▼
ICAO Tier1     OCR extraction
      │
      └──────┐
             ▼
Confidence Engine
             │
      enough confidence?
        │           │
       yes          no
        │           ▼
        │     Vision Provider
        │      (Moondream /
        │      Qwen /
        │      Gemma /
        │      etc.)
        │
        ▼
Document Fusion Engine
        │
        ▼
Structured JSON
```

This is significantly stronger.

---

# Biggest architectural improvement

Your Tier 2 shouldn't be

> "Run LLM."

It should be

> **Ask the LLM only for what deterministic code cannot recover.**

That changes everything.

For example

Instead of sending

Entire passport image

you send

```
OCR text

Detected MRZ

Bounding boxes

Confidence

Image dimensions

Detected country logo

Detected document type

Missing fields:
- surname
- issuing authority

Recover ONLY missing fields.
```

Now even a 2B model performs like a much larger one.

---

# Moondream shouldn't replace OCR

This is the biggest thing I'd change.

Current suggestion:

```
image
↓

Moondream
↓

JSON
```

I don't like this.

Instead

```
image
     │
     ▼
OCRS
     │
     ├─────────────┐
     ▼             ▼
MRZ parser      Image metadata
     │             │
     ▼             ▼
Confidence Engine
        │
        ▼
Need VLM?
        │
      only then
        ▼
Moondream
```

Moondream becomes an intelligent assistant.

Not the OCR.

---

# Even stronger

Introduce document confidence.

Instead of

```
Tier1 failed
↓

Tier2
```

make

```
score = 0

MRZ valid
+60

OCR confidence
+15

Document layout recognized
+10

Photo detected
+5

Country detected
+5

Checksums valid
+20

Missing mandatory fields
-30

Blur
-25

Glare
-15

Perspective distortion
-20
```

If

```
score > 80

skip VLM
```

otherwise

```
run VLM
```

Now your VLM runs maybe 15% of the time instead of every fallback.

Huge speed gain.

---

# Claude should also design model benchmarking

This is where SynthPass can become unique.

Don't benchmark

> "Which VLM is best?"

Benchmark

```
Passport

ID card

Residence permit

Driver license

Synthetic documents

Damaged scans

Watermarked scans

Night photos

Mobile camera

Fax

Photocopy

Thermal printer

Low DPI

High DPI
```

Each model gets

```
Accuracy

Hallucination rate

Speed

Memory

JSON validity

Field accuracy

OCR correction

MRZ correction

Average confidence
```

Now customers can literally choose

```
Default

Balanced

Maximum Accuracy

Low RAM

CPU only

GTX970

RTX2060

RTX4060
```

That is an amazing feature.

---

# Instead of "supported model"

I'd create

```
Model Capability Profile
```

Example

```yaml
name: Moondream2

vision: true

ocr: excellent

layout: excellent

hallucination: low

json: excellent

gpu_memory: 2.7GB

cpu_speed: medium

supports_grammar: yes

supports_tool_calling: no

supports_bbox: partial

license: Apache-2
```

Every provider implements this.

Then CLI simply becomes

```
synthpass benchmark
```

```
Detected hardware:

GTX970
16GB RAM

Recommended:

★★★★★ Moondream2

★★★★☆ Gemma3 Vision

★★★★☆ SmolVLM

★★★☆☆ Qwen2.5-VL
```

That's a killer feature.

---

# Prompt improvements

I would **not** hardcode prompts inside Rust.

I'd version them.

```
prompts/

    moondream.md

    qwen-vl.md

    gemma3.md

    internvl.md

    generic.md
```

Then providers load prompts dynamically.

Future models become plug-ins.

---

# JSON schema

Don't ask

> Extract this document

Ask

```
Return ONLY JSON.

Unknown values MUST be null.

Never guess.

Confidence must be 0.0–1.0.

For every field include

"value"

"confidence"

"source"

Where source is one of

ocr

mrz

vision

inference

validation

Return no explanations.
```

Example

```json
{
  "surname": {
    "value": "SMITH",
    "confidence": 0.99,
    "source": "mrz"
  },
  "date_of_birth": {
    "value": "1995-02-14",
    "confidence": 0.82,
    "source": "vision"
  }
}
```

This is far more valuable than plain JSON because your Fusion Engine can always prefer deterministic sources (MRZ, OCR) over inferred ones.

---

# Future-proofing

The most important addition I'd ask Claude to design is a **Model Registry**:

```rust
trait VisionProvider {
    fn load(...);

    fn supports_json();

    fn supports_grammar();

    fn supports_bbox();

    fn max_context();

    fn infer(...);

    fn benchmark(...);

    fn estimate_memory();
}
```

With this abstraction, Moondream becomes just one implementation. Supporting the next generation of VLMs is reduced to adding another provider rather than rewriting the pipeline.

---

# Final recommendation

If I were writing the architecture specification for **SynthPass v2**, I would ask Claude to design around these principles rather than around a specific model:

1. **Provider-based VLM architecture** (Moondream is the default, not the only option).
2. **Confidence-driven escalation**, where deterministic OCR/MRZ handles as much as possible before invoking a VLM.
3. **Fusion Engine** that merges OCR, MRZ, and VLM outputs with confidence weighting instead of trusting any single source.
4. **Benchmarking framework** that evaluates models across document types, image quality conditions, hallucination rates, speed, memory usage, and JSON compliance.
5. **Versioned prompts and schemas**, allowing prompt updates independently of code releases.
6. **Structured field provenance** (`value`, `confidence`, `source`) for every extracted attribute, enabling explainable and auditable extraction.
7. **Hardware-aware recommendations**, allowing SynthPass to recommend the best model automatically based on available CPU, RAM, and GPU resources.

That direction turns SynthPass from "an OCR project with an LLM fallback" into a **document intelligence platform**. The OCR engine, MRZ validator, confidence engine, fusion logic, benchmarking suite, and pluggable VLM providers become the product—not any single model. Given the vision you've outlined for future customers choosing the best local model for their hardware, I think that's the strongest long-term architecture.
