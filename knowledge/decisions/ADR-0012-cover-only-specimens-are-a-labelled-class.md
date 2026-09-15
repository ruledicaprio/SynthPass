# ADR-0012 — Cover-only specimens are a labelled class, not a drop

**Status:** Accepted
**Date:** 2026-09-15

## Context

The specimen-acquisition loop's first twelve scouting cycles (proving run, c01–c11) produced 19
candidates and 66 "none found" reports. Classifying the reports by the worker's own explanation,
the single largest bucket — 20 of 66 — was *cover or design image only*: the issuing authority
publishes a rendering of the passport's cover or the card's face, but not the data page. Under
the loop's rules those images were discarded at the worker, because a cover carries no MRZ and
no specimen wording.

Three facts argue against discarding them.

1. **A cover carries signal the pipeline will want.** Country name, document type, the e-passport
   chip symbol, coat of arms and series colour are exactly the inputs of a visual issuer or
   document-type classifier — the VIZ facet [`ROADMAP.md`](../ROADMAP.md)'s M4 describes, and a
   routing hint a Tier-2 provider could use before any MRZ is read.
2. **A cover is a clean negative control.** The real-specimen benchmark already scores images
   with no MRZ as `no_mrz_expected` ([`synthpass-bench`](../../crates/synthpass-bench/src/lib.rs)),
   and a checksum-valid read off one would be a false positive worth catching. Covers are the
   purest such control: official, varied, and guaranteed MRZ-free.
3. **The corpus has held covers before, badly.** Seven `passports/` rows today are `no_mrz`
   images, and an earlier audit found covers filed among data pages as an unlabelled
   contamination: nothing in the name said what the image was, so a reader could not tell a
   cover from a failed read. The problem was the missing label, not the cover.

## Decision

Cover-only images are collected as their own **labelled class**, on the same terms as any other
specimen, and never as a coverage claim.

- **The label is a filename variant token, `cover`, next to the mandatory `no_mrz`:**
  `Country_DocType_Specimen_Code_State_YYYY_no_mrz_cover.ext`. The manifest's `variants` field
  carries it (it already parses every free-form token after the fixed slots), so a query for
  covers is `"cover" in variants`, and `integrity_survey.rs --mrz-only` skips them through the
  `no_mrz` token it already honours. Where the MRZ document code and state are unknown from a
  cover, the `XX`/`XXX`/`XXXX` placeholders the convention already allows stand in.
- **The directory stays the document type's** (`passports/`, `id_cards/`): a cover is an image
  of a passport, and `classify_specimen` reads the class off the directory.
- **A cover never changes a code's status in [`CORPUS_COVERAGE.md`](../CORPUS_COVERAGE.md).**
  It is recorded in the row's Docs column as `Passport (cover)`, the way Kenya's front page is
  today; the status column and the Summary denominators are untouched.
- **The scout worker reports covers** (`side: cover`, prefix v3), the screen tool treats them as
  any image, and the `synthpass-screener` agent proposes `public` / `local` / `drop` on the
  source's licence exactly as for a data page, with a note that it is a cover. **The verdict
  vocabulary does not grow**: whether an image may be redistributed and what the image shows are
  two different questions, and the second is answered by the filename.
- **Provenance is unchanged:** `origin` in the manifest, the same review gates, H5 still applies
  (a cover with a visible holder name is still a real person's document).

## Alternatives

- **Keep dropping them.** Zero cost today, but it discards the largest bucket of what workers
  actually find, and the VIZ facet would later have to re-scout every one of them.
- **A separate `samples/covers/` directory.** Cleanest to eyeball, but it splits one document's
  images across directories, needs a Rust change in the manifest's directory list and in
  `classify_specimen` for every consumer, and the directory-equals-document-type rule that
  `classify_specimen` depends on would gain an exception. A token costs none of that.
- **A fourth verdict value, `cover`.** Conflates the content class with the licence decision:
  a cover from Commons under CC-BY is `public`, the same cover on a ministry page with no terms
  is `local`. Two axes, two labels.
- **A sidecar or manifest-only flag without the filename token.** The corpus convention is that
  the filename is the lookup key and says what an image is; a reader listing the directory would
  again be unable to tell a cover from a data page, which is the contamination this ADR closes.

## Consequences

- Positive: the 20-of-66 bucket becomes inventory instead of waste; the benchmark's negative
  controls grow with official, varied material; the VIZ facet starts with a labelled set; the
  earlier cover-vs-biodata mixup cannot recur, because the token names the class.
- Negative: more rows in the manifest and the review packets that will never move a Tier-1
  number, which the dated benchmark entries must keep saying plainly; reviewers spend verdict
  time on images with no MRZ. The packet cap and the batch-size floor bound that cost.
- The corpus README's variant list, the screener agent and the scout prefix name the token.
  No crate changes; no ADR is needed for the first *reader* of covers until one exists outside a
  test, per [`ADR-0009`](ADR-0009-generator-as-a-service.md)'s shape-only rule for the registry.
