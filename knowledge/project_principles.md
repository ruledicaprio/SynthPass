# Project principles

The project's constitution. Seven principles, each with the place in the codebase
where it is already enforced — because a principle nothing checks is a preference,
not a principle. When a design decision is contested, this file is the tiebreaker.

Proposed in [archive/KNOWLEDGE.md](archive/KNOWLEDGE.md); grounded here against the tree.

---

## 1. Deterministic whenever possible. AI only fills uncertainty.

Tier 1 is ICAO 9303 check-digit arithmetic, not a model. A checksum-valid MRZ
read is *mathematically proven* and the LLM is never consulted for it. The model
exists to recover what deterministic code cannot.

**Enforced by:** `crates/synthpass-pipeline/src/lib.rs`'s `ocr_and_tier1` — the
Tier-1 gate consults `synthpass_die`'s `ProviderCatalog`/`MrzReader` for the
checksum verdict, then `RoutingPolicy::default().decide(&evidence)` short-circuits
Tier 2 entirely on an `Accept`. `crates/mrz` has zero dependencies and zero model
weights.

**Consequence we accept:** Tier 2's measured field-exact-match rate is ~45%. We
publish that number rather than hiding it behind the Tier-1 hit rate.

---

## 2. Every claim exposes the strength of its evidence.

Not "every AI decision exposes a confidence" — stronger and narrower. A value is
reported alongside *why it is believed*, and the scale is ordinal because that is
what the data supports.

**Enforced by:** `synthpass_core::fusion::Support`
(`CheckDigit` > `CrossField` > `Structural`) and `v2::FieldConfidence`'s named
bands. The doc comment at `fusion.rs:45-54` is the canonical statement of why
this is not a float: *"Inventing likelihoods and combining them with Bayes' rule
would launder a guess into a number with decimal places."*

**Corollary:** `FieldConfidence::mrz_checksum_scope()` is honest that TD3 check
digits cover only four of ten fields. `proven()` — all ten at 1.0 — is used only
by the WASM demo, and its doc comment says so.

**The one permitted exception:** a *routing* score may be an uncalibrated number,
because being wrong costs CPU seconds rather than truth. It is never serialized
and never compared against a reported confidence — the split is enforced by the
type system, not by convention.

---

## 3. Models are replaceable. Architecture is permanent.

The product is the pipeline, the provider contract, the confidence model and the
benchmark — not any one model. A customer asking *"can I use my own model?"*
should get "yes" without us shipping a new release.

**Enforced by:** the `IntelligenceProvider` / `Recognizer` / `FieldReader`
contract in `crates/synthpass-die`, and the `ProviderCatalog` that holds them.
Nothing in `synthpass-die` names `ocrs`, `rten`, or `llama.cpp`.

---

## 4. Synthetic data before private data.

We own a generator, therefore we own perfectly labelled ground truth we are
legally allowed to keep. That is the scarce resource in this field — not models.

**Enforced by:** `crates/synthpass-gen` (seeded, deterministic) feeding
`crates/synthpass-bench`. `CONTRIBUTING.md`'s corpus rules forbid real PII in
`samples/`.

---

## 5. Offline-first, or it does not ship.

No cloud calls, no telemetry, no runtime downloads on the extraction path. The
deployment story is *copy one static binary to an air-gapped machine*.

**Enforced by:** the `x86_64-unknown-linux-musl` static build; `ocr-embedded`
baking `.rten` weights in via `include_bytes!`; prompts compiled in via
`include_str!` with no runtime override. A feature that requires a network call
at extraction time is out of scope by definition.

---

## 6. Benchmark everything. Never trust anecdotes.

A constant with no measurement behind it does not ship. Neither does a claim.

**Enforced by:** the M4 CI gate (`synthpass-bench --min-hit-rate 0.30`);
`MRZ_BAND_CONFIDENT_SCORE` in `crates/synthpass-ocr/src/geometry.rs`, whose doc
comment carries the sweep that produced it; and — the harder half —
`fusion.rs:25-39`, which records a candidate check that was **measured and
rejected**, with the reason. Publishing what we discarded is part of the
principle.

---

## 7. No hidden magic. Every extraction is explainable.

Given an output, it must be possible to say which provider produced each value,
on what evidence, and why anything more expensive was or wasn't run.

**Enforced by:** `v2::Provenance`, `v2::ExtractionTrace` (providers consulted,
escalation reason, prompt version), and `line1_integrity` carrying the specific
`Finding`s rather than a bare boolean.

**Boundary:** explainability is owed to the *operator*, not the log stream.
Findings carry PII and stay inside the zeroized document JSON; logs and metric
labels get the fieldless `FindingKind`. See `CONTRIBUTING.md`'s PII checklist and
`crates/synthpass-pipeline/tests/pii_logging.rs`.

---

## What these rule out

Stated so they don't get re-litigated:

- **No face recognition, biometrics, or liveness detection.** ([VISION.md](VISION.md) §non-goals)
- **No authenticity or forgery detection.** A checksum proves a faithful *read*,
  never a genuine *document*.
- **No cloud inference, ever** — including "optional" cloud fallback.
- **No calibrated-looking confidence numbers without a reliability diagram**
  behind them (principle 2).
- **No new dependency without written justification.** Precedent: the `tracing`
  block in the root `Cargo.toml`, and `synthpass-bench` hand-rolling Levenshtein
  in ~20 lines rather than adding `strsim`.
