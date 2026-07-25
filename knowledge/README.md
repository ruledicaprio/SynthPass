# knowledge/

Not a docs folder — a technical library. Documentation describes what was built;
this tree is meant to preserve *why*, so the reasoning outlives the code, the
models, and the people who made the decisions.

Start at the top-level [README.md](../README.md) for install and usage.

The premise, argued in [KNOWLEDGE.md](KNOWLEDGE.md): in a year the competitive
advantage will not be the Rust code — it will be the accumulated engineering
knowledge and the architecture built around it. That is much harder to replicate
than swapping in the latest vision model.

## Read these five first

If you are new here (human or model), an hour on these gives you the philosophy,
the constraints and the direction before you open a source file:

1. **[project_principles.md](project_principles.md)** — the constitution. Seven
   principles, each with the place in the codebase that enforces it.
2. **[VISION.md](VISION.md)** — the dual mission and the explicit non-goals.
3. **[ARCHITECTURE.md](ARCHITECTURE.md)** — how it works today, and the
   version-by-version record of what got deleted and why.
4. **[ROADMAP.md](ROADMAP.md)** — the authoritative milestone spine.
5. **[../README.md](../README.md)** — install, usage, the public surface.

## Current & forward-looking

- **[project_principles.md](project_principles.md)** — the seven principles and
  what they rule out. The tiebreaker when a design decision is contested.
- **[VISION.md](VISION.md)** — the dual mission (technical sovereignty +
  compliance-by-design) and long-term direction.
- **[ROADMAP.md](ROADMAP.md)** — the authoritative milestone spine, Definition of
  Done per phase, what's shipped vs. planned. If another doc's roadmap section
  disagrees with this one, this one wins.
- **[ARCHITECTURE.md](ARCHITECTURE.md)** — engineering rationale, trade-offs, and
  the version-by-version design history (why Tier 1 / Tier 2 exist, what got
  deleted and when).
- **[technical_debt.md](technical_debt.md)** — deferred decisions with honest
  severity and effort. Add an entry when you *choose* not to fix something.
- **[BRANDING.md](BRANDING.md)** — the naming model, messaging guardrails,
  commercial tiers.
- **[LICENSING.md](LICENSING.md)** — the offline customer (`fingerprint` →
  `verify-license`) and vendor (`keygen` → `issue-license`) CLI walkthrough.
- **[CORPUS_COVERAGE.md](CORPUS_COVERAGE.md)** — per-country OCR corpus status
  and the checklist for adding a new specimen.
- **[SYNTHPASS.md](SYNTHPASS.md)** — running the `synthpass-bench` corpus runner
  locally, its CLI flags, report format, and how the M4 CI accuracy gate works.
- **[ADVERSARIAL.md](ADVERSARIAL.md)** — the degraded capture profiles
  (mobile/scanner/worn/border-kiosk) as the adversarial corpus: what each
  simulates, and gate status.

## Subject folders

Each holds short, actionable notes (1–4 pages). The rule is the 80/20 filter from
[KNOWLEDGE.md](KNOWLEDGE.md): if a finding cannot become a trait, struct,
algorithm, pipeline, benchmark, configuration or heuristic, it does not get kept.

| Folder | Holds |
|---|---|
| **[decisions/](decisions/)** | Architecture Decision Records. Why, not what. |
| **[providers/](providers/)** | One note per intelligence provider: capability, measured behaviour, licence of the weights. |
| **[prompts/](prompts/)** | Prompt philosophy and per-version evaluation results. The prompts themselves live in `crates/synthpass-llm/prompts/` — they are compiled in. |
| **[ocr/](ocr/)** | Recognition, preprocessing, layout analysis, confidence scoring. |
| **[vision/](vision/)** | Vision-language models and the multimodal track. |
| **[benchmarks/](benchmarks/)** | Methodology, metric definitions, and dated sweep results — including rejected candidates. |
| **[evaluation/](evaluation/)** | How we decide something is good enough to ship. |
| **[hardware/](hardware/)** | Memory budgets, CPU-only and consumer-GPU targets, deployment shapes. |
| **[research/](research/)** | Distilled, actionable summaries drawn from `papers/`. |
| **[papers/](papers/)** | Source academic material, kept as reference. |

## Design notes (informal)

Working notes and creator's remarks that shaped the current direction.
Opinionated and unedited; where they disagree with `ROADMAP.md` or an ADR, those
win.

- **[KNOWLEDGE.md](KNOWLEDGE.md)** — why this tree exists and how to maintain it.
- **[DOCUMENT_INTELLIGENCE_ENGINE.md](DOCUMENT_INTELLIGENCE_ENGINE.md)** and
  **[SYNTHPASS_DOCUMENT_INTELLIGENCE_ENGINE.md](SYNTHPASS_DOCUMENT_INTELLIGENCE_ENGINE.md)**
  — the case for a provider interface rather than a single VLM.
- **[V_1_3_PROPOSALS.md](V_1_3_PROPOSALS.md)** — the v1.3.0 proposal set.

## Historical / origin records

`FOUNDATIONAL_STRATEGY.md` and `synthpass_v2_0.md` are the source notes
VISION/ROADMAP/BRANDING were distilled from. (The earlier
`rebranding_identra_synthpass.md` and `mlis_v2_0_0_preliminary_design.md` scratch
notes and the `REBRAND_MIGRATION.md` execution record have since been removed now
that the rename is long complete and folded into `CHANGELOG.md`.) Kept for
provenance; not maintained as living docs — prefer the current documents above
when they disagree.

Project-wide (not in this folder): [CONTRIBUTING.md](../CONTRIBUTING.md),
[SECURITY.md](../SECURITY.md), [CHANGELOG.md](../CHANGELOG.md).
