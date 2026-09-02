# Visual inspection zone → Tier 2

**Status: design only. No code in this document has been written, no
`PROMPT_VERSION` has moved, and no schema field has been added.**

The visual inspection zone (VIZ) is everything on a travel document that is not
the MRZ — Doc 9303 Part 1's own definition: *"those portions of the MRTD
designed for visual inspection, i.e. front and back (where applicable), not
defined as the MRZ."* It carries the printed name, the dates, the issuing
authority, the place of birth. Everything the MRZ carries, plus things it does
not.

This document says what "use the VIZ in Tier 2" would actually mean, in the
order the pieces have to land, and what has to exist before any of it can be
judged.

---

## 1. The premise everyone starts with is wrong

The natural framing is *"the model only sees the MRZ; feed it the visual zone
too."* That is not the situation.

`NativeOcr::recognize_detailed` runs the general recognition engine over the
**whole page** and returns that text. The MRZ-constrained engine only ever
*appends* candidate lines to it. `NativeOcr::recognize` is a thin wrapper
returning the same string, and it is what `RustOcrEngine::to_markdown` calls,
what `OcrStage.markdown` holds, and what reaches `build_prompt` as
`md_content`.

**So the model already receives the full-page text, VIZ included, and always
has.** Anything built on "feed the VIZ in" would be building something that
already exists.

The real problems are different and more specific: the VIZ text the model
receives is *noisy*, *unlabelled*, *asked about vaguely*, and *incomplete* —
four separable failures with four separable fixes.

---

## 2. What is actually missing

### 2.1 Clean — the noise is measured, and it is systematic

`visual_zone_survey.rs` classifies every geometry-detected line as MRZ or
non-MRZ and scores the non-MRZ ones for noise. Over 229 specimens the
corpus-wide noise-line fraction is **median 0.237, IQR [0.146, 0.386]** — one
line in four of the visual zone is unusable. Re-measuring on a corpus 70% larger
moved the median by 0.006, which is the strongest form that finding can take: it
is a property of document photography, not of which specimens happened to be in
the set. See `knowledge/ROADMAP.md`'s issue #103 entries for the derivation and
`knowledge/visual-zone-survey.jsonl` for the per-specimen data.

`preprocess.rs` already has the deterministic treatment — upscale, contrast,
threshold, deskew — and applies it only to the MRZ band, because the MRZ band is
where a checksum can tell you whether the treatment helped. Extending it to the
non-MRZ zone is the mechanical part; knowing whether it worked is §5.

**This is first for a reason.** Feeding noisier text into a narrower prompt
makes the answer worse, not better, and does it while looking like progress.

### 2.2 Label — the geometry exists and is thrown away

`synthpass_ocr::geometry::OcrPage` carries `lines: Vec<OcrLine>` — per-line text
with bounding boxes, for the whole page. It is computed on every call and
discarded twice on the way to the model:

| Boundary | What happens |
| :-- | :-- |
| `synthpass-pipeline/src/ocr.rs` | `OcrResult` has no `lines` field, so per-line geometry stops at the engine abstraction |
| `synthpass-die`'s `Recognition` | also has no `lines` — it carries whole-page `text`, `mrz_band`, `portrait`, `rotation`, `text_sanity` |
| `synthpass-pipeline/src/lib.rs:701` | the Tier-2 `DocumentContext` is built `.with_image(input)` but **without** `.with_recognition(...)` — the Tier-1 context at `:613` does pass it |

That last one is the cheapest fix in this document and is worth doing on its own
merits: the Tier-2 provider is handed strictly less than the Tier-1 provider
about the same page, for no reason anyone recorded.

Both additions are additive — a new field on a struct, an extra builder call —
and neither changes any output on its own. That is what makes them safe to land
before the measurement in §5 exists.

### 2.3 Ask narrowly — the design is already written down

`knowledge/vision/README.md` states it: don't hand a model the whole page and
say "extract this document". Hand it the OCR text, the detected MRZ, the
bounding boxes, and **an explicit list of the fields that are still missing**,
and ask for only those. A small model asked a narrow question beats a larger one
asked a vague one.

Two pieces of this already exist and are worth not rebuilding:

- `Evidence::missing` (via `ExtractionFields::missing`) already computes
  "which fields came back with nothing usable" and is documented as *"the input
  to an escalation decision: ask a more capable provider for **these**."*
- The MRZ hint landed in issue #102: `build_prompt` takes an optional `hint`
  built from the individual `mrz::Checks` bits, so a checksum-*partial* read
  already contributes its verified fields to the prompt.

The narrow ask is those two joined up, plus §2.2's bounding boxes. It needs
§2.2, and it moves `PROMPT_VERSION`, so it needs §5.

### 2.4 Add the fields only the VIZ has

`place_of_birth`, `issuing_authority` and `date_of_issue` are printed on most
documents and exist nowhere in the schema — not in `Extraction`, not in
`ExtractionFields`, not in `CoreField`. No MRZ format carries any of them, so
they are unreachable by Tier 1 by construction, and they are the clearest answer
to *"what is Tier 2 for?"*

They can never be checksum-proven. They sit permanently at
`LLM_HEURISTIC_CONFIDENCE`, and the confidence model already has the vocabulary
to say so honestly.

This is a wire-contract change guarded by `crates/synthpass-core/tests/schema_keys.rs`,
and it is last because it is the only irreversible one: a published schema key
cannot be taken back.

---

## 3. Order, and why it is not negotiable

```
2.1 clean  →  2.2 label  →  2.3 ask narrowly  →  2.4 new fields
```

Cleaning before labelling, because labelling noise produces labelled noise.
Labelling before the narrow ask, because the narrow ask consumes the labels.
The narrow ask before new fields, because a new field extracted by a vague
prompt is a new field nobody can trust — and unlike the first three, it cannot
be withdrawn.

§2.2's two additive changes are the exception: they alter no output and can land
whenever, which makes them the sensible first commit.

---

## 4. Traps carried forward

- **`prompt::drop_non_latin_noise` discards non-Latin lines rather than
  transliterating them.** On a document whose VIZ is bilingual, that throws away
  the half a VLM would find easiest to read. Not wrong today — it removes real
  noise — but it is a deliberate loss and any VIZ work has to decide about it
  rather than inherit it.
- **`process_document` and `process_document_stream` are separate paths, and a
  fix has already failed to reach both once.** `synthpass-serve` uses only the
  streaming one. `knowledge/technical_debt.md` records the previous occurrence.
- **`RoutingPolicy::sanity_floor` is a deliberately unused hook.** Its doc
  comment says it *"exists so that once the measurement exists, there is
  somewhere to put it"* — `text_sanity` is a character-plausibility proxy and
  nobody has measured what it predicts. A VIZ-quality score is a candidate for
  that slot. Naming it here so it is not rediscovered as a bug.
- **`escalate_on_line1_flagged` is the same shape.** Off by default, waiting on
  data. `ground_truth_candidates.rs`'s rejection list is a first source of that
  data: it records how often a *checksum-valid* read is nonetheless flagged by
  `fusion::check_line1_integrity`.

---

## 5. The measurement, which is the actual prerequisite

`knowledge/prompts/README.md` requires a measurement per `PROMPT_VERSION` bump —
*"a prompt change with no measurement behind it is indistinguishable from
noise."* Everything in §2.3 and §2.4 bumps it.

The only harness is `crates/synthpass-llm/tests/parity.rs`. It is `#[ignore]`d
and CI-silent, and it ran over **six documents**. Six cannot separate a
regression from sampling noise, so in practice Tier-2 accuracy was not
measurable at all — `knowledge/technical_debt.md` states the consequence
plainly: *"the accuracy of the shipped Tier-2 path is unguarded between
releases."*

### 5.1 What the corpus work bought

`ground_truth_candidates.rs` generates a fixture for every specimen whose MRZ
reads with valid check digits *and* survives the crate's own deterministic
cross-checks. Fixtures are split by review status, not by format:

| Directory | Origin | Fields parity scores |
| :-- | :-- | :-- |
| `samples/ocr_fixtures/` | hand-verified against the image | all nine prompt fields |
| `samples/ocr_fixtures/derived/` | generated, unreviewed | `document_number`, `date_of_birth`, `date_of_expiry` |

The split is not bureaucracy. An ICAO check digit covers exactly
`document_number`, `date_of_birth`, `date_of_expiry` and `personal_number`; the
composite covers the same ranges and no more. A derived fixture's *name* is one
OCR pass's reading of characters nothing arbitrates — and this corpus shows the
recogniser getting exactly those characters wrong. Score a model against them
and the measurement inverts: **a model that read the VIZ correctly would be
marked wrong for disagreeing with an OCR error**, so §2.1 succeeding would show
up here as a regression. Promoting a candidate (`git mv` one level up, after a
person compares its names against the image) moves six more fields into the
scored set.

### 5.2 The bias that remains, and how to close it

Every fixture has a checksum-valid MRZ — that is where the ground truth comes
from. But Tier 2 runs only when Tier 1 *fails*. **The parity corpus is, by
construction, the set of documents Tier 2 never sees in production.**

That is fine for a regression check and wrong for predicting escalation
accuracy, which is what VIZ work needs.

The fix uses the same fixtures. Feed the model the OCR text **with the
MRZ-shaped lines removed**, and score it against the ground truth those lines
produced:

- the input is a document whose MRZ is unavailable — the escalation case;
- the truth is still checksum-proven, because it was derived before the holdout;
- and the four proven fields are all recoverable from the VIZ, which prints the
  document number and both dates in human-readable form.

That is a direct measurement of *"can Tier 2 recover verified fields from the
visual zone alone"* — the exact question §2.1 through §2.3 exist to improve, on
ground truth no model was involved in producing. It is the natural next
increment: one flag on the generator, one variant fixture set, no schema change,
no prompt change.

### 5.3 What to record before touching anything

Run the harness and write the numbers into `knowledge/ROADMAP.md` first. A
before is not optional here — every change in §2 is a change to what the model
is shown, and the only honest claim about such a change is a measured one.

---

## 6. Non-goals

- **This is not a vision provider.** Everything above is text in, text out. The
  multimodal track is `knowledge/vision/README.md` and
  `knowledge/decisions/ADR-0005-vision-provider-readiness.md`.
- **The model does not replace deterministic parsing.** `CLAUDE.md` and
  principle 1: Tier 1 runs first, and Tier 2 fills what deterministic code
  cannot recover. Cleaning the VIZ does not make the VIZ authoritative — the MRZ
  stays the checksum-proven source wherever it is readable.
- **No face recognition, ever.** `portrait` is crop coordinates.
  `knowledge/VISION.md` §2 makes this a permanent non-goal, and passing a
  bounding box to a prompt does not change that.
