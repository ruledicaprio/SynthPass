# ADR-0011 — Split M6: the deterministic core keeps the number, packaging becomes M8

**Status:** Accepted (amended 2026-09-16, 2026-09-17)
**Date:** 2026-09-13

## Context

[`ADR-0008`](ADR-0008-mrz-detection-track.md)'s 2026-09-12 amendment ends by handing on a
decision it declines to take: *"The deferral this ADR accepted is due for review … choosing
between a third accuracy chunk and the packaging half of M6 is exactly that decision. It belongs
in its own ADR."* This is that ADR. Five facts decide it.

**1. The ordering premise has expired twice, and the second time it inverted.**
[`ADR-0006`](ADR-0006-m6-accuracy-first.md) put accuracy first on the evidence that
`checksum_failed` was the largest miss. `ADR-0008` replaced that premise with `no_mrz_found`,
2.6 : 1 after the denominator correction. Chunk 2's orientation fix brought the two level; a
reclassification on 2026-09-13 then took four documents that could never be read out of the
denominator and left detection the *smaller* half, at a Tier-1 real-specimen rate of
**143 / 153 = 93.5%** on documents that can yield a hit
([`benchmarks/README.md`](../benchmarks/README.md#current-headline-numbers) — the one measured
figure this ADR states). "Point the accuracy track at the largest measured miss" was the property
both earlier ADRs used to order M6. It now points away from the track `ADR-0008` opened.

**2. What remains of the accuracy work is a list of documents, not a track.**
[`orientation-fix-2026-09-12.md`](../benchmarks/orientation-fix-2026-09-12.md) "What is left"
named every remaining scored miss individually. The 2026-09-13 manifest review
([`manifest-review-no-mrz-found-2026-09-13.md`](../benchmarks/manifest-review-no-mrz-found-2026-09-13.md))
and the reclassification that followed removed four of the seven detection candidates: a Swedish
card *front* with no zone, a Dutch licence whose single line is not ICAO 9303 in any format, an
Egyptian specimen whose zone the publisher pixel-masked, and an Argentine child passport whose
printed zone fails its own check digits. What is left is three documents whose zone is not found —
France and Italy, both check-digit-verified TD1 zones, and Moldova — against seven whose zone is
found and read wrong. The unit of work is now a named document.

A milestone DoD phrased *"`no_mrz_found` is measurably reduced"* cannot be satisfied honestly at
this size. The same review shows why: a genuine improvement passes the tolerance-zero gate
**silently**, because the gate fails only on an increase. Named documents, and a re-bless in the
same pull request, are the only units left that mean anything.

**3. M6's Definition of Done contains a clause the project has already rejected.** The milestone
table asks for *"Pro-beta feedback collected"* and lists *"commercial 'Pro' closed beta"* among
the deliverables. [`BRANDING.md` §5](../BRANDING.md#5-commercial-strategy) rejected a
feature-gated paid tier outright — the gate is bypassable by recompiling, the bypass is in the
README quickstart, and no licence has ever been issuable. `ROADMAP.md`'s own M6 section body
already says so; the table row above it does not. A milestone cannot close against a DoD one of
whose clauses contradicts the commercial strategy.

**4. The two halves have disjoint blockers.** COCO/YOLO need geometry `synthpass_gen::Labels`
does not surface, an ink-tight-versus-slot box decision and a class taxonomy
([`EXPORTS.md`](../EXPORTS.md), "Deferred: COCO / YOLO"). An official binary is blocked on the
placeholder Ed25519 verifying key and a Windows machine fingerprint that binds nothing
([`technical_debt.md`](../technical_debt.md), High). None of those gets closer by reading more
MRZs — and `BRANDING.md` §5's sequencing note states the converse directly: *"None of them
requires the Tier-1 extraction accuracy number to improve first."* Disjoint blockers are what a
milestone boundary is for.

**5. `ADR-0008` pre-committed the trigger.** Its Negative consequences: *"if accuracy work
displaces packaging a third time, that is evidence M6 should have been split, and `ADR-0006`'s
rejected M8 option should be revisited rather than re-rejected."* The choice in front of us is
that third displacement, arriving on schedule.

## Decision

**Split M6 along the line `ADR-0006` drew and declined to cut.**

**M6 keeps its number and takes the deterministic core.** Its heading becomes
`## M6 — Deterministic core: Tier-1 accuracy and MRZ formats`. Its deliverables are Tier-1
real-document accuracy against the named residual, and TD1/TD2/MRVA/MRVB reading through
registered `synthpass-die` providers. Its Definition of Done:

- Every scored miss on the committed baseline is either a Tier-1 HIT or carries a dated
  attribution in `knowledge/benchmarks/` naming the mechanism that defeats it — the standard
  `ADR-0008` chunk 1 already met when it attributed eleven documents to page orientation.
  Attribution closes a document as legitimately as a fix does; what does not close it is silence.
- No Tier-1 HIT regression, and **every chunk that moves an outcome count re-blesses
  `real-specimen-mrz-baseline.json` in the same pull request** — the gate does not ask, so the
  milestone does.
- Each of TD1 / TD2 / MRVA / MRVB reads through a registered `synthpass-die` provider against the
  M7 contract, with its per-format rate published in `benchmarks/README.md`.
- No OCR engine replacement, vision provider or new dependency enters under this milestone.

**M8 — Expansion & Enterprise readiness** takes declarative layout plugins, the remaining dataset
exports, the air-gapped deployment guide, and the first commercial engagement. It **inherits the
old M6 heading text verbatim**, so the three in-tree citations of that anchor —
[`EXPORTS.md`](../EXPORTS.md), [`research/long-horizon-parsing.md`](../research/long-horizon-parsing.md)
and [`ADR-0007`](ADR-0007-dataset-export-format.md), all of which cite it *for the dataset-export
commitment* — are repointed to the section that now carries their commitment rather than left
resolving to one that does not. `scripts/check-doc-links.sh` fails the build if one is missed.
Its Definition of Done:

- A third-party *layout* definition drives generation without a code change.
- At least one export format consumed by an external trainer end-to-end.
- An air-gapped install performed from a source build on a machine with no network, and written
  up — not merely described.
- One commercial engagement delivered with feedback collected, in `BRANDING.md` §5's terms — a
  labelled corpus, an independent benchmark, or an air-gapped integration.

**Distribution is source-build only.** Recorded in [`technical_debt.md`](../technical_debt.md) on
2026-09-13: the verifying key compiled into every binary is still a placeholder and the Windows
machine fingerprint binds nothing, so no official binary is a deliverable of M8 or any other
milestone. That is why M8's air-gapped criterion names a source build. Shipping a binary is its own
decision later, and generating the real keypair is its first step — not a condition of closing M8.

**"Pro beta" leaves the record.** The phrase appears in the M6 table row, its DoD and the
"suggested order"; all three are replaced by the engagement wording above, which the `ROADMAP.md`
section body already uses.

**Linearity is preserved, not excepted.** M6 closes, then M8 opens — still one milestone at a
time, which is the rule `ADR-0002` and `ADR-0006` both declined to license an exception to.
Pulling an M8 item ahead of M6's close is an ordering exception and needs its own ADR meeting
`ADR-0002`'s bar. That includes a commercial engagement that arrives early: the pull
`BRANDING.md` §5 describes is real, but it is argued in writing when it comes, not absorbed here.

**Explicitly not licensed by this decision:** replacing `ocrs`/`rten`, adding tesseract or any
dependency, enabling a vision provider ([`ADR-0005`](ADR-0005-vision-provider-readiness.md) is
still Proposed), or starting the PaddleOCR/PP-OCRv5 recognizer swap. The recognizer question is
open and benchmark-first — a `synthpass-bench` bake-off, per `technical_debt.md` — and it would
be its own ADR, not an M6 chunk.

**What would change this decision.**

1. **One mechanism explaining both halves of the residual.** The untested band-squeeze hypothesis
   in `orientation-fix-2026-09-12.md` is about France, the only unread document from a scan wider
   than 1600 px. If a measurement showed one pure-Rust change moving both detection and
   recognition misses together, M6's core half closes in a single chunk and the split's overhead
   exceeds its benefit.
2. **A requirement for an official binary.** If an engagement needs a signed binary, the real
   keypair and a real Windows fingerprint become M8 scope and its DoD grows. That changes what M8
   contains, not whether it should exist.

## Alternatives rejected

**A third accuracy chunk first; packaging deferred again.** The work is real — France and Italy
print fully conforming TD1 zones whose four check digits validate, and all seven
`checksum_failed` documents carry checksum-valid printed zones, so each is a genuine recognition
error. Rejected as a *milestone-level* choice for three reasons. First, `ADR-0008` named this
exact move as the evidence that M6 should have been split; taking it again without answering the
trigger converts a recorded condition into a habit, which is the thing its Negative consequences
warned about. Second, the recognition half — now the larger one, seven documents attributed to
native OCR on low-resolution scans, not to `crates/mrz` — leads directly to a recognizer bake-off
that no accepted ADR licenses, so the track's most likely next step crosses a line this project
has deliberately drawn twice. Third, and decisively, the split does not stop this work: bounded
accuracy chunks against named documents are exactly what M6 now *is*. Rejecting this option
rejects the deferral, not the chunk.

**Packaging first; accuracy paused at the current gate.** `ADR-0008` rejected this with *"packaging
a reader that finds no MRZ at all on 37% of real specimens sells the wrong thing at the wrong
time"* — and that framing was retracted by its own 2026-09-09 amendment as *"never true"*. The
number it was made against has since moved a long way, and `BRANDING.md` §5 argues affirmatively
for selling the provable thing first. So the original objection no longer holds and this option is
genuinely stronger than it was. It is rejected anyway, because *pausing* is the wrong verb: the
residual is a short list of named documents, several of which are check-digit-verified targets,
and abandoning them mid-list leaves the milestone with the same unbounded DoD the split exists to
fix. Everything worth taking from this option is taken — the packaging half stops waiting on an
accuracy number that no longer gates it, and gets a DoD of its own — without discarding a
finishable list.

**Interleave inside one milestone; no structural change.** The cheapest option on paper, and the
one `ADR-0006` chose: *"a sequencing decision inside one milestone, not a structural one."*
Rejected because it has been tried and its own author priced the failure. `ADR-0006`'s Negative
consequences: *"M6 remains broad. The split would have bounded each half's Definition of Done more
tightly; this reframe relies on the 'suggested order' and the sequencing language holding."* It did
not hold — across two ADRs the packaging half stayed untouched while the accuracy half moved
through two complete tracks. More decisively, interleaving is the one structure both `ADR-0002` and
`ADR-0006` explicitly refuse to license: *"Explicitly not licensed by this decision: parallel
tracks."* Two sequential milestones keep one-at-a-time intact; small parallel tracks inside one
milestone do not. `ADR-0002`'s warning that "an exception invites a second one" applies to
interleaving more sharply than to a split, because a split needs no exception to the linearity rule
at all.

**Keep M6's heading for anchor stability and let the prose go stale.** `ADR-0006` kept the heading
on exactly this reasoning, and it was right then: *"the deliverable is still in M6, just later"*, so
every inbound link stayed true. Once the deliverable moves, an anchor that resolves to the wrong
section is worse than a link edit, and the rule here is that a document and the thing it describes
are never left inconsistent. Three one-line edits, checked by CI, is the cheaper error.

**Renumber so the built-first milestone is M6.** Rejected for the reason `ADR-0002` rejected it:
`ROADMAP.md`, `CHANGELOG.md`, closed pull requests and commit messages already refer to M6 by
number, and renumbering rewrites the meaning of existing references. M8 is the next free number and
is the one `ADR-0006` reserved for this.

## Consequences

**Positive**

- Each half gets a Definition of Done that can be checked and can close — the benefit `ADR-0006`
  named when it rejected the split and could not obtain by reframing.
- The accuracy DoD matches the unit the evidence now comes in: named documents, each read or
  attributed, with a mandatory re-bless. The silent-improvement failure the 2026-09-13 manifest
  review found becomes impossible to repeat without breaking the milestone's own rule.
- The packaging half stops being gated on an accuracy number that `BRANDING.md` §5 says it never
  depended on, and its real blockers — `Labels` geometry, and the placeholder key that makes
  distribution source-build only — become milestone-visible instead of technical-debt-visible.
- "Pro beta" is removed from the roadmap, which had been contradicting `BRANDING.md` §5 and its
  own section body in the same file.
- Linearity survives the change rather than being excepted for it.

**Negative**

- A second milestone-table change in under a week, which is the cost `ADR-0006` correctly priced
  as reader trust. Mitigated only by the fact that `ADR-0008` published the condition in advance
  and the condition was met, so this is a recorded trigger firing rather than a re-litigation.
- Three inbound anchors and one `README.md` sentence must move in the same pull request. CI
  catches the links; nothing catches the prose but review.
- **The split does not make packaging start sooner.** Under linearity M8 opens when M6 closes, and
  M6's close depends on attributing or fixing a list of documents. That ordering was chosen
  deliberately with this ADR; an engagement that cannot wait is argued in its own ADR.
- Source-build-only distribution is now a recorded position rather than an unnoticed gap. It
  narrows who can adopt SynthPass without a Rust toolchain, and it stays that way until someone
  decides otherwise.
- `ADR-0002` and `ADR-0006` both acquire a forward link and a partially revised conclusion. The
  record shows a decision made, tested against evidence, and changed — which is what this directory
  is for, but it does mean three ADRs must be read together to reconstruct M6's shape.

## Amendment (2026-09-16)

The Decision's third M6 criterion — *"Each of TD1 / TD2 / MRVA / MRVB reads through a
registered `synthpass-die` provider against the M7 contract, with its per-format rate
published in `benchmarks/README.md`"* — is met by **one** provider, not four.

`MrzReader` (provider id `mrz`) already reads all five ICAO 9303 formats: it calls
`mrz::find_and_parse`, which tries them in the fixed order `ARCHITECTURE.md` §13.3 records
(TD3 → MRV-B → MRV-A → TD1 → TD2) to avoid cross-format cannibalization, and it reports the
format it read in `Evidence::mrz_format`. `ROADMAP.md` described that provider as TD3-only,
which the code does not bear out. What the criterion was actually missing is measurement: the
published per-format synthetic rates came from `synthpass-bench`, which calls
`mrz::find_and_parse` directly and never touches the catalog.

The criterion therefore reads: *each of TD1/TD2/MRVA/MRVB reads through the registered
deterministic MRZ provider, with its per-format synthetic rate measured through that provider
(`provider-bench --mrz-only --document-type <format>`) and published in
`benchmarks/README.md`.* Before the published row changes source, both harnesses are run once
on identical seeds and the comparison is recorded in `knowledge/benchmarks/`, so a changed
figure is never also a silently changed method. `synthpass-bench` and its `--min-hit-rate`
gate are unchanged.

**Rejected: one registered provider per format** (`mrz-td1` … `mrz-mrvb`, each wrapping the
same parser). Each wrapper would either run the full five-format search and discard the
others — five parses per document, with catalog insertion order silently becoming the
cross-format tiebreak §13.3 keeps inside the parser — or require per-format entry points in
the published `mrz` crate, which M7 declined to add. It would also replace the single `mrz`
identity that the Tier-1 routing, the real-specimen baseline gate and the benchmark history
all key on. `ADR-0002`'s concern was formats arriving as branches in the pipeline; none do.

**Rejected for now: a `formats` field on `Capability`.** Nothing would read it — the router
cannot select by format before a read, and the benchmark does not snapshot it. It becomes
worth adding when a second MRZ-capable provider exists and routing must choose between them.

## Amendment (2026-09-17)

**M6's Definition of Done is frozen against the v1.5.0 baseline snapshot.** As written, the
criterion — *every scored miss on the committed baseline is either a Tier-1 HIT or carries a dated
attribution* — is evaluated against a baseline the specimen-acquisition loop re-blesses with every
cohort. Each cohort can add scored misses, so the target recedes as fast as the loop runs, and the
milestone may never converge by construction. The 2026-09-17 repository assessment named this and
the user decided to freeze.

**The snapshot**, pinned per the benchmark maintenance contract's Freeze rule (MAIN revision,
`samples-data` revision, provider configuration, invocation, population):

- MAIN `a3951b3` (tag `v1.5.0`); its committed baseline was measured on CI sha `2f14e00`,
  2026-09-16, `samples_data_sha 469a4ee7723148917cf023f1a7ab2af80a0ef7d3`.
- Provider and configuration: the registered `mrz` provider via
  `provider-bench --real-specimens --mrz-only`, every `SYNTHPASS_OCR_*` arm at its default.
- Population: 261 documents, 154 scored, 140 Tier-1 hits.
- **The residual — the 14 scored misses, by asset:**

  Detection (`no_mrz_found`, 4):

  - `id_cards/France_ID_Specimen_2020_back_mrz.png`
  - `id_cards/Italy_ID_Specimen_2022_back_mrz.jpg`
  - `passports/Moldova_Passport_Specimen_PA_MDA_2014_mrz.jpeg`
  - `id_cards/San_Marino_ID_Specimen_2017_back_mrz.jpg` — a blank filler template with nothing
    printed in its zone; it closes by attribution (reclassified `checksum_failed_specimen` in the
    data-only PR that follows this amendment).

  Recognition (`checksum_failed`, 10):

  - `passports/Afghanistan_Passport_Specimen_P0_AFG_2016_mrz.webp`
  - `id_cards/Belgium_ID_Specimen_2021_back_mrz.png`
  - `id_cards/Croatia_ID_Specimen_2021_back_mrz.jpg`
  - `passports/Czechia_Passport_Specimen_P0_CZE_2005_mrz.jpg`
  - `passports/Germany_Passport_Specimen_P0_D00_2024_mrz.jpg`
  - `passports/Hong_Kong_Passport_Specimen_P0_HKG_2007_mrz.png`
  - `passports/Hong_Kong_Passport_Specimen_P0_HKG_2019_mrz.png`
  - `passports/Romania_Passport_Specimen_PE_ROU_2024_mrz.jpg`
  - `passports/Russian_Federation_Passport_Specimen_P0_RUS_2019_mrz.jpg`
  - `id_cards/Sweden_ID_Specimen_2022_back_mrz.jpg`

**The criterion now reads:** *each of these 14 documents is either a Tier-1 HIT on the current
committed baseline or carries a dated attribution in `knowledge/benchmarks/` naming the mechanism
that defeats it.* Nothing else in the Decision changes: no Tier-1 HIT regression, every chunk that
moves an outcome count re-blesses the baseline in the same PR, and no OCR engine replacement,
vision provider or new dependency enters under M6.

**What the freeze does not do.** The gate and the baseline keep moving as they do today — every
cohort or fixture PR still re-blesses `real-specimen-mrz-baseline.json`, and the gate still fails
on any HIT loss or bucket growth. A scored miss that a post-freeze cohort adds is recorded in a
dated "post-M6 residual" entry in `knowledge/benchmarks/README.md` and worked after M6 closes; it
does not reopen M6. Name accuracy ([`ADR-0013`](ADR-0013-names-are-scored-against-mrz-form-truth.md))
is not part of this criterion either: it is measured separately and its repair is judged on its
own metric.

**Sequencing around the freeze.** Specimen acquisition pauses at this amendment (recorded in
`CONTRIBUTING.md`) so the denominator stops moving while the residual is worked, and ground-truth
transcription for documents already in the corpus takes its place. Each of the ten recognition
misses gets a hand-transcribed fixture first, because the transcription is what decides whether
the printed zone conforms (a recognition target) or does not (`checksum_failed_specimen`, closed
by attribution).

**Rejected: leaving the criterion open-ended.** Honest, and the loop's cohorts are worth having,
but a milestone whose target recedes is not a milestone; M8's commercial deliverables wait on
"M6 closes", so an unclosable M6 blocks them indefinitely.

**Rejected: freezing the corpus itself.** Freezing the Definition of Done costs nothing the
project wants; freezing the corpus would stop the ground-truth work and the fixture PRs that make
every future delta attributable.
