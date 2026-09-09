# Tesseract OCR studies

**This is the right moment to study Tesseract before running the A/B measurement**.

ADR-0008's core discipline is correct: **do not replace anything yet; first explain the 64.2% vs 59.5% gap under controlled conditions.** The ADR explicitly identifies four confounders: recognizer model, preprocessing, retry/variant budget, and corpus mismatch. 

Tesseract itself can be usefull for SynthPass, especially the parts that matter for **MRZ detection rather than generic OCR**.

## Main findings

I would **not** go into the experiment thinking:

> "Let's compare Tesseract OCR against ocrs/rten."

I'd go in thinking:

> **"Let's discover which Tesseract behaviors are accidentally giving it an MRZ advantage, then determine which of those behaviors can be reproduced in native Rust without importing Tesseract."**

That is a much more interesting experiment.

Tesseract is not merely "an OCR model." Its result is the product of:

**image conditioning → page segmentation → text-line localization → recognition → language/model constraints → output geometry/confidence**

And several of those layers are highly relevant to your `no_mrz_found` problem.

---

# 1. The biggest Tesseract lesson: segmentation may matter more than recognition

This is probably the first thing we need to investigate.

Tesseract has explicit page-segmentation modes:

* `PSM 3` — fully automatic page segmentation
* `PSM 6` — single uniform block
* `PSM 7` — single text line
* `PSM 11` — sparse text
* `PSM 13` — raw single line, bypassing some Tesseract-specific hacks

The official documentation explicitly recommends changing PSM when OCR'ing a **small region rather than a whole page**. ([Tesseract OCR][1])

That matters enormously for MRZ.

Our product isn't actually asking:

> "What text is on this passport?"

It is asking:

> **"Where is this very special two/three-line machine-readable text block?"**

So if Tesseract's segmentation happens to locate a weak MRZ band that `ocrs/rten` never exposes to the recognizer, then **Tesseract's apparent OCR advantage is actually a localization advantage**.

ADR-0008 already anticipates precisely this possibility:

> preprocessing gap / recognizer gap / band-localization gap. 

That's excellent. Proposal to make **band localization the first hypothesis**.

---

# 2. Tesseract has surprisingly powerful "OCR scaffolding"

One of the things people often miss is that Tesseract performs significant image processing internally through **Leptonica** before recognition. Its own documentation says preprocessing can materially affect accuracy, and discusses rescaling, binarization, noise removal, morphology, rotation/deskewing, borders, etc. ([Tesseract OCR][1])

Therefore:

### Don't compare

`raw image → ocrs/rten`

against

`raw image → Tesseract`

and conclude:

> Tesseract model better.

That would be scientifically weak.

Instead measure:

```text
INPUT
  │
  ├── identical decoded pixels
  │
  ├── identical crop
  │
  ├── identical scale
  │
  ├── identical grayscale/binarization
  │
  └── identical MRZ candidate band
           │
           ├── ocrs/rten
           │
           └── Tesseract
```

Then progressively reintroduce each engine's native preprocessing.

That will tell us **where the magic actually lives**.

---

# 3. OCR-B is a potentially huge clue

This is the part that made me go:

**"Oh. This may be much more interesting than a generic Tesseract comparison."**

There is an actual community Tesseract traineddata specifically targeting **MRZ / OCR-B**.

The project `tessdata_ocrb` describes itself as:

> traineddata for MRZ using OCR-B fonts

and was created by fine-tuning Tesseract's English model. Its synthetic OCR-B evaluation reported **44.95% character error rate with ordinary English data versus 0% with the OCR-B-trained models** on that synthetic evaluation. ([GitHub][2])

There is also a Tesseract issue specifically discussing MRZ traineddata, including an OCR-B-only model constrained to:

```text
A-Z
0-9
<
```

which is essentially the MRZ alphabet. ([GitHub][3])

That is a **massive experimental clue**.

Your current comparison may actually be:

```text
ocrs/rten
    ↓
generic OCR recognition
    ↓
try to discover MRZ characters

versus

Tesseract.js
    ↓
OCR-B-trained model
    ↓
recognition strongly aligned with MRZ glyphs
```

If that's what your browser demo is doing, **64.2% vs 59.5% isn't a generic engine comparison at all.**

It is partly:

> **generic OCR vs MRZ-specialized OCR.**

And that is precisely something SynthPass can potentially learn from without adopting Tesseract.

---

# 4. Character vocabulary constraints are another clue

Tesseract supports character whitelisting through:

`tessedit_char_whitelist`

and its documentation explicitly suggests this for cases where only a subset of characters is expected. ([Tesseract OCR][1])

MRZ is an unusually friendly OCR target because the alphabet is tiny:

```text
A-Z
0-9
<
```

That means SynthPass has something that ordinary OCR systems don't:

### a very strong prior.

We already have the ICAO 9303 parser.

So the architecture could eventually become:

```text
                  IMAGE
                    │
              MRZ candidate
                 bands
                    │
        ┌───────────┴───────────┐
        │                       │
   generic OCR              MRZ OCR
   ocrs/rten                 constrained
        │                       │
        └───────────┬───────────┘
                    │
              MRZ validator
                    │
          ICAO 9303 constraints
                    │
             sequence/checksum
```

The important insight is:

**Don't necessarily teach the OCR engine MRZ semantics. Give it the smallest possible visual alphabet and let your deterministic validator remain the authority.**

That fits SynthPass's architecture beautifully.

---

# 5. Dictionaries may actually be hurting generic OCR

Another fascinating Tesseract trait.

Tesseract is normally optimized around ordinary language recognition. Its documentation specifically says that for things like **receipts, price lists, and codes**, dictionaries may be inappropriate and can be disabled. ([Tesseract OCR][1])

MRZ is basically the ultimate anti-dictionary document:

```text
P<BIOSAMPLE<<JOHN<<<<<<<<<<<<<<<<<<<<
L898902C<3UTO6908061M9406236ZE184226B
```

Those aren't words.

They're structured codes.

So one experiment we should absolutely perform is:

```text
Tesseract generic model
       vs
Tesseract + MRZ vocabulary constraints
       vs
Tesseract + OCR-B model
```

If the gap moves substantially, we have learned something important.

---

# 6. Tesseract's output geometry is extremely useful for SynthPass

Tesseract can produce:

* plain text
* hOCR
* TSV
* bounding boxes
* confidence values

according to its documentation. ([GitHub][4])

This is important because **we don't actually want Tesseract's text**.

We want to know:

> **What did Tesseract see that caused it to succeed?**

For every successful document, record:

```text
candidate band
bounding box
line count
line heights
x/y coordinates
confidence
recognized characters
```

Then compare against the native pipeline.

The interesting question becomes:

```text
Does Tesseract recognize better?

OR

Does Tesseract find the right pixels better?
```

That's exactly the distinction ADR-0008 wants.

---

# 7. PSM 7/13 are especially interesting for MRZ

Once we isolate an MRZ candidate band, I would test:

```text
PSM 6
PSM 7
PSM 11
PSM 13
```

But **not indiscriminately**.

For a known candidate line:

```text
PSM 7
```

is conceptually almost perfect:

> "This image is one text line."

And PSM 13 is interesting because Tesseract describes it as a raw line mode that bypasses Tesseract-specific hacks. ([Tesseract OCR][1])

This could expose whether Tesseract's normal page-layout machinery is helping or hurting.

---

# 8. Don't underestimate scale and binarization

Tesseract's documentation explicitly calls out:

* rescaling
* binarization
* noise removal
* dilation/erosion
* rotation/deskewing

as major OCR-quality factors. ([Tesseract OCR][1])

For your experiment, I would therefore make preprocessing **a factorial variable**, not a vague "preprocessing" bucket.

Something like:

```text
                    RAW
                     │
          ┌──────────┴──────────┐
          │                     │
       grayscale            threshold
          │                     │
       scale 1x             scale 2x
                              │
                         scale 3x
```

We may discover something embarrassingly simple like:

> Tesseract isn't better at recognizing MRZ; its input is effectively 2× upscaled and better binarized.

If so, congratulations: **we just gained a Rust-native optimization without adding Tesseract.**

---

# 9. Tesseract's "best" vs "fast" model distinction is another experiment

Tesseract has several official model families:

* `tessdata`
* `tessdata_fast`
* `tessdata_best`

The official documentation explains that `fast` uses smaller integerized LSTM models, while `best` provides slower, more accurate models. ([GitHub][5])

So your experiment shouldn't simply say:

> Tesseract.

It should identify:

```text
Tesseract version
core version
traineddata version
model name
model variant
PSM
OEM
preprocessing
```

Otherwise we'll eventually get a number that nobody can reproduce.

---

# 10. And tesseract.js itself is another layer of confusion

`tesseract.js` is not a separate OCR algorithm.

It wraps an Emscripten/WASM build of Tesseract. ([GitHub][6])

Current ecosystem implementations can use WASM SIMD as well. ([GitHub][7])

So our experiment needs to distinguish:

```text
Tesseract engine
       │
       ├── model
       ├── Leptonica preprocessing
       ├── segmentation
       └── recognition

versus

tesseract.js wrapper
       │
       ├── WASM compilation
       ├── JS-side image handling
       └── Tesseract engine
```

The **JS wrapper itself should not be credited for OCR accuracy**.

---

# The really important SynthPass lesson

I think there are **four pieces of Tesseract worth potentially stealing**.

Not Tesseract.

Its **ideas**.

### ① Better MRZ candidate localization

Instead of asking OCR to read the entire passport:

```text
passport image
      ↓
text geometry analysis
      ↓
find long horizontal bands
      ↓
MRZ candidates
```

### ② MRZ-specific preprocessing

Possibly:

```text
grayscale
↓
contrast normalization
↓
scale
↓
threshold
↓
morphological cleanup
↓
line extraction
```

### ③ MRZ-specific recognition constraints

```text
[A-Z0-9<]
```

rather than generic language recognition.

### ④ Multi-variant retry

This is perhaps the most valuable.

Instead of:

```text
one image → one OCR attempt
```

use:

```text
candidate band
   ├── native
   ├── upscaled
   ├── thresholded
   ├── sharpened
   └── alternate segmentation
          ↓
      OCR candidates
          ↓
     deterministic MRZ scorer
```

And **only then** choose the winner.

That is very compatible with our existing deterministic architecture.

---

# What I would NOT steal from Tesseract

I would explicitly tell Claude **not** to copy these prematurely:

* C++ dependency
* Leptonica dependency
* Tesseract page-layout machinery wholesale
* generic dictionaries
* generic language models
* Tesseract-specific heuristics without evidence
* another OCR engine simply because it scores higher once

ADR-0008 is explicit that replacing `ocrs/rten` is **not licensed by the current decision**. 

And I strongly agree.

---

# 🔬 The experiment I would actually run

I would expand Claude's measurement into **five layers**:

```text
LEVEL 0 — Current baseline
ocrs/rten native
Tesseract.js current demo

LEVEL 1 — Same pixels
identical decoded image

LEVEL 2 — Same crop
identical MRZ candidate regions

LEVEL 3 — Same preprocessing
identical grayscale / scale / threshold

LEVEL 4 — Tesseract variants
PSM / OCR-B / vocabulary / retry variants

LEVEL 5 — Attribution
localization vs preprocessing vs recognizer
```

For each document:

```text
document_id
ground_truth_MRZ
native_found
tesseract_found

candidate_band_iou
line_count
line_height
crop_dimensions
scale
preprocessing_variant

ocr_engine
model
psm
confidence

sequence_valid
checksum_valid
```

Then produce something like:

| Experiment         | Native | Tesseract |  Delta |
| ------------------ | -----: | --------: | -----: |
| Current pipelines  |  59.5% |     64.2% | +4.7pp |
| Same pixels        |      ? |         ? |        |
| Same crop          |      ? |         ? |        |
| Same preprocessing |      ? |         ? |        |
| OCR-B model        |      ? |         ? |        |
| MRZ constraints    |      ? |         ? |        |

**That table is the real ADR-0008 deliverable.**

---

# Actual prompt for Claude Code <3

Claude only derives *what conclusion to reach*, I dont. Use Tesseract clues but preserve ADR-0008's scientific discipline.

We are about to execute ADR-0008's first engineering chunk: a controlled measurement explaining why the existing tesseract.js browser path reports 122/190 = 64.2% while the native ocrs/rten path reports 113/190 = 59.5% on the same stated corpus.

Do NOT implement a replacement OCR engine yet. Do NOT change the production pipeline. This is reconnaissance + experimental design first.

ADR-0008 explicitly identifies four confounders:

1. recognizer/model,
2. preprocessing path,
3. retry/variant budget,
4. corpus mismatch.

The experiment must eventually hold these constant one at a time, in one corpus and one same-binary A/B harness. The objective is to determine whether the gap is primarily:

* preprocessing,
* recognizer/model,
* MRZ-band localization/segmentation,
* retry/variant budget,
  or some interaction between them.

Before running the measurement, study Tesseract specifically as an engineering system, not merely as "another OCR engine".

Important Tesseract traits to investigate:

1. PAGE SEGMENTATION
   Tesseract has explicit page segmentation modes including:

* PSM 3: fully automatic page segmentation
* PSM 6: single uniform block
* PSM 7: single text line
* PSM 11: sparse text
* PSM 13: raw single line, bypassing some Tesseract-specific hacks

For MRZ this is highly relevant because the product problem is not generic document transcription. It is finding a highly structured 2/3-line machine-readable zone. Determine whether Tesseract's apparent advantage could actually be a segmentation/localization advantage.

2. INTERNAL IMAGE PROCESSING
   Tesseract uses Leptonica and performs substantial image processing before recognition. Investigate the effect of:

* rescaling,
* grayscale conversion,
* binarization,
* noise removal,
* dilation/erosion,
* rotation/deskewing,
* borders/cropping.

Do not assume "Tesseract is a better recognizer" until these effects are separated.

3. OCR-B / MRZ-SPECIFIC TRAINING
   There are community Tesseract models specifically trained/fine-tuned for OCR-B MRZ text, including models using the restricted MRZ character set:
   A-Z, 0-9, <

Investigate whether the existing tesseract.js benchmark uses an OCR-B/MRZ-specific model and record the exact model/traineddata provenance.

This is critical: if the browser demo uses an OCR-B-trained recognizer while ocrs/rten is generic, then 64.2% vs 59.5% is not a generic engine comparison.

4. CHARACTER/VOCABULARY CONSTRAINTS
   Tesseract supports character whitelisting and dictionary controls. Investigate whether restricting recognition to the MRZ alphabet or disabling ordinary language dictionaries materially changes MRZ extraction.

Do NOT assume this should be copied into SynthPass. Treat it as an experimental variable.

5. OUTPUT GEOMETRY
   Tesseract can emit text plus bounding boxes/confidence through TSV/hOCR-style outputs.

For successful cases, investigate:

* text bounding boxes,
* line coordinates,
* line heights,
* confidence,
* segmentation structure.

The goal is to answer:
"Does Tesseract recognize the MRZ better, or does it find the correct pixels/band better?"

6. MODEL VARIANTS
   Identify:

* exact Tesseract version,
* exact tesseract.js/core version,
* exact traineddata,
* model family (tessdata / tessdata_fast / tessdata_best / OCR-B/MRZ custom),
* OEM,
* PSM,
* SIMD/WASM characteristics.

Do not report simply "Tesseract".

7. RETRY / VARIANT BUDGET
   Determine how many effective recognition attempts the browser path performs versus native ocrs/rten.

A single successful variant among several attempts is materially different from one deterministic OCR attempt.

8. MRZ-SPECIFIC HYPOTHESIS
   Consider whether the following architecture could explain the Tesseract advantage:

image
-> candidate MRZ band localization
-> crop
-> scale/preprocess
-> constrained OCR
-> deterministic ICAO/MRZ validation

rather than:

full document
-> generic OCR
-> search resulting text for MRZ

This is a hypothesis only. Do not encode it into production before measurement.

EXPERIMENTAL REQUIREMENTS

Design the measurement as a controlled factorial/A-B experiment.

At minimum separate:

A. Existing native ocrs/rten pipeline
B. Existing tesseract.js pipeline
C. Same decoded pixels
D. Same crop/band
E. Same preprocessing
F. Tesseract model variants
G. PSM variants
H. MRZ character constraints
I. retry/variant budget

For every run record enough metadata to reproduce it:
document_id,
engine,
version,
model,
PSM/OEM,
preprocessing variant,
scale,
crop coordinates,
retry count,
candidate bands,
OCR output,
confidence,
MRZ sequence result,
checksum result,
final classification.

The primary metric remains the ADR-0008 target:
no_mrz_found = 85/229 baseline.

Tier-1 HIT must not regress.

IMPORTANT:
Do not optimize for the 64.2% number.
Do not cherry-pick successful documents.
Do not tune against the evaluation corpus and then report the same corpus as unbiased evidence.
Do not replace ocrs/rten.
Do not add a new dependency.
Do not implement a detector yet.

FIRST DELIVERABLE

Before writing production code, give me:

1. A concise technical explanation of why Tesseract could plausibly outperform ocrs/rten on MRZ detection.
2. A list of Tesseract traits that are relevant to SynthPass.
3. A list of traits that should NOT be copied.
4. The exact variables that must be controlled.
5. The smallest experiment matrix that can distinguish:

   * localization advantage,
   * preprocessing advantage,
   * recognizer/model advantage,
   * retry-budget advantage.
6. The instrumentation/output schema required per document.
7. A proposed same-binary A/B harness design compatible with ADR-0008.
8. A recommendation for which Tesseract findings could eventually be reproduced natively in Rust without importing Tesseract.
9. Explicitly identify whether the current 64.2% tesseract.js result is using OCR-B/MRZ-specific traineddata. If the repository/docs do not establish this, mark it UNKNOWN and tell me exactly how to verify it.

Use the following conceptual principle throughout:

"Learn from Tesseract's behavior; do not assume Tesseract itself is the solution."

The ultimate goal is not "make SynthPass use Tesseract".

The goal is:

"Explain the 4.7 percentage-point gap, then extract the smallest deterministic/native-Rust improvements that recover the useful part of Tesseract's advantage while preserving SynthPass's pure-Rust, air-gapped, deterministic Tier-1 architecture."

Do not move beyond measurement until the evidence identifies the cause.

---

## One more thing for the Claude investigation

There is a **very specific smoking gun** I want Claude to verify before we even start interpreting 64.2%.

Our ADR says the browser demo uses:

> `tesseract.js + an OCR-B-trained model`

and gets 122/190. 

If that statement is accurate, then **the first thing to verify is exactly what `traineddata` file that demo loads**.

Because there are published OCR-B Tesseract models specifically intended for MRZ, and their authors report dramatically better results than generic English models on synthetic OCR-B material. ([GitHub][3])

If our 64.2% path really is using that kind of model, I would actually rename the experiment mentally:

> **"What is the contribution of OCR-B specialization, segmentation, preprocessing and retry budget to MRZ discovery?"**

rather than:

> "Tesseract vs ocrs/rten."

That distinction could save us from drawing the wrong architectural conclusion.

Honestly, **this is a very SynthPass-shaped opportunity**: if the winning ingredients turn out to be *MRZ band localization + OCR-B-aware preprocessing + tiny alphabet constraints + deterministic validation*, we may be able to reproduce most of the benefit in Rust while keeping Tesseract completely outside the product. 

That's the result we'd be hoping for. 😄

[1]: https://tesseract-ocr.github.io/tessdoc/ImproveQuality.html?utm_source=chatgpt.com "Improving the quality of the output | tessdoc"
[2]: https://github.com/Shreeshrii/tessdata_ocrb?utm_source=chatgpt.com "GitHub - Shreeshrii/tessdata_ocrb: tesseract 4 traineddata for MRZ using OCR-B fonts · GitHub"
[3]: https://github.com/tesseract-ocr/tesseract/issues/753?utm_source=chatgpt.com "Community Contributions - MRZ Traineddata · Issue #753 · tesseract-ocr/tesseract"
[4]: https://github.com/tesseract-ocr/tessdoc/blob/main/FAQ.md?utm_source=chatgpt.com "tessdoc/FAQ.md at main · tesseract-ocr/tessdoc · GitHub"
[5]: https://github.com/tesseract-ocr/tessdoc?utm_source=chatgpt.com "GitHub - tesseract-ocr/tessdoc: Tesseract documentation · GitHub"
[6]: https://github.com/gary-jiao/tesseract.js/?utm_source=chatgpt.com "GitHub - gary-jiao/tesseract.js: Pure Javascript OCR for 62 Languages :book::tada::desktop_computer: http://tesseract.projectnaptha.com/ · GitHub"
[7]: https://github.com/naptha/tesseract.js-core/blob/master/examples/web/benchmark/benchmark.wasm-simd.html?utm_source=chatgpt.com "tesseract.js-core/examples/web/benchmark/benchmark.wasm-simd.html at master · naptha/tesseract.js-core · GitHub"
