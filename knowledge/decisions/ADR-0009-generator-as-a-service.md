# ADR-0009 — Generator-as-a-service: what it would cost the non-goals

**Status:** Proposed. **This ADR changes no code and commits to no product.** It prices three
proposals against the non-goals each one touches, so the decision is taken against stated costs
rather than enthusiasm.
**Date:** 2026-09-11

## Context

[`VISION.md`](../VISION.md) §2 states the long arc and §3 the commercial positioning;
[`BRANDING.md` §5](../BRANDING.md#5-commercial-strategy) settles that the software is MIT
permanently and revenue comes from corpora, benchmarking, integration, custom models and support.
Both were written when the product was an extraction appliance with a generator attached.

The generator is now the stronger half. It has no accuracy bottleneck — Tier-1 extraction is the
thing still being measured, while a generated document is correct by construction — and
`BRANDING.md` §5's own sequencing note says the generation and benchmarking side is sellable
today while extraction is not. So "sell the generator" is not a departure from the strategy; it
is the strategy, arriving earlier than the roadmap expected.

Three proposals arrived together. They are **not** equally costly, and the purpose of this ADR is
to stop them being discussed as one idea:

1. **A hosted generator app and API** — self-service synthetic documents with ground truth.
2. **Country-accurate templates** — rendering documents that look like a specific state's
   passport or ID, rather than the current generic non-country template.
3. **A metered MRZ-validation API** — a subscription on the validator path.

Each collides with a different line, and the lines are not equally load-bearing. VISION's
non-goals are written as permanent (*"these lines do not move"*), but a constitution that cannot
be amended is a constitution that gets ignored instead. The requirement is not that they never
move; it is that moving one is a **recorded, argued decision** rather than a silent
reinterpretation — which is exactly what an ADR is for.

## Decision

**Price them separately. Adopt none of them here.** The three findings below are what this ADR
records; each would need its own accepted decision, and #2 needs one from the user, not from an
agent.

### 1. Hosted generator API — the cheap one

**It contradicts the letter of a non-goal and not its substance.** *"It does not do cloud
anything"* and *"air-gapped or it does not ship"* exist to protect **user documents**:
`project_principles.md` P5, VISION §1's "no PII egress", the zeroize discipline, the SHA-256-only
audit trail. Generation has no user document. The inputs are a seed and a parameter set; the
identities are fictional by construction; there is nothing to leak because nothing personal ever
arrives.

So the honest framing of the amendment is a **scoping correction, not a reversal**: the invariant
is *"the extraction path is air-gapped and no user document leaves the boundary"*, which is what
the architecture actually enforces and what a regulated buyer actually buys. A hosted generator
sits outside that path entirely.

What it still costs, and these are real:

- **`synthpass-serve` is not a multi-tenant service** and was never threat-modelled as one.
  Hosting means auth, quotas, abuse handling, and an attack surface the air-gapped binary
  deliberately does not have.
- **"CLI and library come first; a GUI is secondary"** would need the same scoping treatment.
- Every hosted request is a request someone could have run locally for free. The service sells
  convenience, not capability — which is a fine business, but it must be *named* as one, because
  it is not defensible the way the corpus-plus-benchmark pairing is.

### 2. Country-accurate templates — the expensive one

The cost here is **structural, not moral**, and it is worth stating precisely because the moral
framing invites a pointless argument while the structural one is simply true:

> **MIT + publicly distributed + country-accurate are mutually incompatible. Pick two.**

Today the generator is safe to publish because its output is *unmistakably synthetic by
construction* — mandatory watermark, generic non-country template, enforced in code. That is what
lets `VISION.md` §4 claim the ethics posture it claims, and it is what makes the repository
publishable at all. Country-accurate templates remove the second guard, and the first is one
line away in a repository anyone may fork: a watermark enforced in MIT source is a watermark a
fork deletes in an afternoon. There is no code-level mitigation for this, because the threat
model is *"the attacker has the source and may modify it"*, which MIT guarantees.

Three resolutions exist. They are genuinely different products:

| Option | What ships | What it costs |
| --- | --- | --- |
| **(a) Templates as service output** | The renderer stays generic and MIT; country-accurate corpora are *delivered* to a contracted buyer, not published | Nothing structural. This is already `BRANDING.md` §5's first revenue line, just taken seriously. The buyer is known, the output is a deliverable, provenance is contractual. |
| **(b) Templates as a separate non-MIT component** | Generic renderer stays MIT; country layouts live behind a licence | A licensing split the project has never had, plus the "you can read every line that touches your documents" claim needs re-wording for the split artifact |
| **(c) Public + country-accurate** | Everything, in the open repository | The regulated-buyer moat. No compliance officer adopts a vendor whose public repository is a passport-rendering toolkit — and that buyer is the entire target market named in VISION §3. Irreversible: it cannot be un-published. |

**(a) is the recommendation**, and it is not a compromise — it reaches the same market. The
customer who wants Dutch-accurate synthetic IDs wants *the corpus*, not the renderer. Selling
the corpus captures the revenue, keeps the repository publishable, and keeps the refusal that
makes the project credible.

Two further facts that belong in the decision rather than being discovered later:

- **Scraped specimen imagery is not uniformly reusable.** The corpus fetchers pull Wikimedia
  Commons, whose licences are known per file. PRADO and several issuing authorities publish
  specimens under terms that do **not** permit derivative reuse. "Scrape and construct templates"
  is a per-source rights question, not one decision — and using a specimen to *test* a reader is
  a different act, legally and ethically, from using it to *reproduce* the document.
- **Accurate templates change what a watermark has to survive.** If this is ever built, the
  guard has to move from "obvious to a human" to something that survives re-encoding and
  cropping, plus signed provenance metadata (C2PA-style) asserting synthetic origin. That is a
  research project of its own, not a flag.

### 3. Metered MRZ-validation API — the weak one

Two problems, and the first is fatal to the pricing:

- **It sells what is already free.** `crates/mrz` is MIT, zero-dependency, published on
  crates.io, and the browser demo validates MRZs client-side at no cost. A subscription competes
  with the project's own published artifacts, and the first customer to notice `cargo add mrz`
  stops paying.
- **An MRZ string is PII.** Name, date of birth, document number, nationality. An MRZ-validation
  API therefore *does* reintroduce PII egress — the thing P5 and the entire compliance posture
  exist to prevent. It is less exposed than an image API, not unexposed.

The defensible version of this is not validation-by-the-request. It is **independent
benchmarking and certification** — `BRANDING.md` §5's second revenue line — where the buyer pays
for an audited accuracy number against a corpus they did not choose. That sells the measurement
discipline, which is the actual moat, rather than re-selling an MIT parser.

## Alternatives rejected

**Amend VISION's non-goals wholesale to "we do cloud now."** Rejected. The non-goals are load-
bearing for the regulated buyer, and three proposals of unequal cost do not justify one blanket
reversal. Scoping the air-gap invariant to the extraction path (#1) is a precise change with a
stated reason; deleting the section is not.

**Refuse all three on the strength of the existing text.** Rejected, and this is the ADR's other
half. *"These lines do not move"* was written to stop silent drift, not to freeze the product in
its 2026 shape. A documented amendment with an argued cost is the mechanism working as intended.

**Decide #2 now.** Rejected: it is the one irreversible item, it is the user's call rather than
an engineering call, and a publishing decision taken at the end of a long working night is the
canonical example of a decision to take in the morning instead.

## Consequences

**Positive**

- The three proposals stop being one conversation. #1 is a scoping amendment, #3 is a pricing
  error with a better nearby product, and only #2 is genuinely hard — which is not visible while
  they are discussed together.
- The commercial path that requires no non-goal to move at all — option (a), corpora as a
  contracted deliverable — turns out to reach the same market, and it is already the documented
  strategy.
- The `web/` demo metrics idea, raised alongside these, needs **no** amendment: browser numbers
  can reach the repository through the existing `tests/web/` harness in CI rather than through
  telemetry, and there is already a `WEB_OCR_BASELINE.md` for them to land in. Tracked separately
  as ordinary roadmap work.

**Negative**

- This ADR settles nothing by itself, which is a real cost when there is momentum to spend.
- Recommending option (a) declines the most exciting version of the idea. If the measured
  conclusion later is that buyers will not pay for corpora without the renderer, (a) fails and
  this decision has to be revisited against that evidence.

**Explicitly not licensed by this decision:** no hosted service, no template scraping, no
licensing split, no VISION edit. Amending a non-goal requires its own accepted ADR naming the
exact sentence that changes and the invariant that replaces it.
