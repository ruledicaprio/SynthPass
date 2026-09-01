# MRZ_SEQUENCE_COMPLETENESS.md — Tier-1 completeness backlog (M6 sub-track)

Scope: `crates/mrz` **structural completeness** — ensuring an OCR-read MRZ sequence
is detected/repaired/validated as complete and internally consistent (right line
count, right line length, all fields present, checksums valid, cross-field
consistency), not merely that individual characters parsed. See
[`ROADMAP.md`](ROADMAP.md)'s M6 section for how this fits the milestone.

## Why this exists

SynthPass's product is the deterministic Tier-1 MRZ core; Tier-2 LLM inference is
the enterprise add-on for the residual ~1%, not the thing being sold. Real-specimen
benchmarking (`knowledge/benchmarks/README.md`, 2026-08-16 run) already shows
`checksum_failed` (66) as the **largest** miss category — ahead of `no_mrz_found`
(51). In other words: the MRZ is *found* but doesn't fully validate more often than
it isn't found at all. `crates/mrz` already carries serious OCR-repair machinery —
`repair.rs`'s bounded unknown-character solver, `checksum.rs`'s lookalike/defiller
repair, `blindspot.rs`'s mod-10 residue-class analysis — that goes well beyond three
external Python MRZ projects surveyed while scoping this doc
(`mrz`/Arg0s1080, its `mrz-surepass` fork): neither attempts OCR-confusion repair or
confidence scoring at all. Their one genuinely reusable idea — a two-tier
fields/warnings/errors validation report — is itself missing a typed "why" enum,
which is exactly the gap below.

Four separate, uncoordinated "is this record complete" vocabularies exist today:

- `Checks` (`crates/mrz/src/lib.rs:123-134`) — 5 booleans, check-digit pass/fail only.
- `DateCompleteness` (`crates/mrz/src/dates.rs`) — `Complete`/`PartiallyUnknown`/
  `Unknown`/`Malformed`, scoped to date-of-birth only.
- `repair::Resolution` (`crates/mrz/src/repair.rs`) — `Unique`/`Ambiguous{candidates}`/
  `Unresolvable`, per-field, ad hoc.
- `synthpass_core::fusion::check_line1_integrity` — cross-field heuristics with **no**
  check-digit backing (`document_type`/`issuing_country`/`surname`/`given_names`
  carry no check digit at all), living one layer up in `synthpass-core`.

And there is **no typed signal anywhere** for "found N of M expected MRZ lines" —
`MrzError` (`crates/mrz/src/lib.rs:341-368`) has `BadLength`, `BadCharacter`,
`BadDocumentCode`, `BadChecksum` (declared, never returned by `parse_*`), and
`NotFound` — but `find_and_parse`'s free-text scanner can see a line-1-shaped
candidate with no companion line ever showing up, and today that collapses to the
same bare `NotFound` as "nothing MRZ-shaped exists at all."

Finally, the production pipeline (`crates/synthpass-die/src/mrz_reader.rs:108-119`)
discards **all** fields whenever even one check digit fails — even though
`find_and_parse` already computes an honestly check-digit-graded partial reading
internally. Nobody has measured, since that internal capability existed, whether
surfacing it would reduce unnecessary Tier-2 escalation.

## How to work this doc

One chunk = one `changelog.d/` fragment = one PR, per `changelog.d/README.md`'s own
convention (`<id>.<category>.md`, `id` = PR number once known). **Do not batch
multiple chunks into one PR.** Each chunk below is meant to be self-contained: file
paths, a design sketch, measurement command, and acceptance criteria. Line numbers
cited are a starting point, not gospel — **re-verify with Grep before editing**,
since they drift as the crate changes. Chunks 6 and 7 were both gated — chunk 6
on a go/no-go after real-specimen measurement, chunk 7 on a product sign-off
before implementation even started. Both are now done; see each section for
what actually shipped (both diverged from the original proposal) and the
measurement that cleared them.

Suggested order and parallelism:

```
Parallel, first (cheap, foundational, no interdependency):
  1. checksum_failed sub-reason diagnostic
  2. personal_number '0'/'<' regression test (no bug — see chunk 2)
  3. changelog.d housekeeping (PR #138, #142 retroactive fragments)

Sequential (5 needs 4's real shape, not a guess):
  4. IncompleteSequence / line-count signal
       ↓
  5. SequenceCompleteness unified type

Last, independent of each other:
  6. TD2 line-1 repair — done (see its section: design pivoted away from the
     originally-proposed document-code dictionary, measured, cleared)
  7. MrzReader partial-read surfacing — done, narrowed, off by default (see its section)
```

---

## Chunk 1 — Diagnostic: split `ChecksumFailed` into sub-reasons

**Goal.** Turn the single `checksum_failed` bucket into an actionable breakdown of
*which* check digit(s) failed.

**Why.** `checksum_failed` is the dominant, currently undifferentiated real-specimen
miss kind (66 of them in the 2026-08-16 run; 36 of 150 in the representative
synthetic report `samples/synthpass-bench-report-td3-all-150-0.json`). Every later
prioritization call in this doc should read off this breakdown instead of guessing.

**Files.** `crates/synthpass-bench/src/lib.rs` — `MissReason` enum (~line 387-398)
and its construction site right after `mrz::find_and_parse` succeeds but
`decoded.valid()` is `false` (~line 608-617); `miss_kind()` (~line 420-427).

**Design.**
```rust
ChecksumFailed { failing: Vec<&'static str> } // was: ChecksumFailed (unit)
```
populated from `decoded.checks: Checks` (`document_number`/`date_of_birth`/
`date_of_expiry`/`personal_number`/`composite`, whichever are `false`).
`miss_kind()` keeps returning the string `"checksum_failed"` as the top-level
bucket (report shape stays additive/backward-compatible) — the JSON just gains a
`failing_checks` array alongside it.

Optional follow-up (do as a separate 1b chunk only if this data justifies it):
distinguish "repair.rs/checksum.rs attempted candidates and none validated" from
"no repair was even attempted" — needs a small `crates/mrz` addition (thread a
`bool` through `find_and_parse`'s fallback path), not just a bench-side change, so
keep it separate.

**Measurement.** `cargo test -p synthpass-bench`, then `synthpass-bench --profile
all` to confirm `failing_checks` populates and the top-level `checksum_failed`
count is **unchanged** (a regression here means the refactor accidentally changed
which records count as a miss — instrumentation must not alter behavior). Then
re-run the real-specimen survey and report the new breakdown.

**Acceptance criteria.** New field present in bench JSON output; existing
`checksum_failed` count bit-identical before/after; the breakdown reported in the
PR description (e.g. "28 composite-only, 5 document_number, 3 multi-field").

**Size.** Small. **Dependencies.** None — do first.

---

## Chunk 2 — Verify `personal_number_check_digit` `'0'`-or-`'<'` issuer option

**Goal.** Close a normative-spec conformance question with a definitive answer.

**Status: already conformant — this is a test-coverage gap, not a bug.** Do not go
looking for a fix; there isn't one.

`Mrz_Field_Layout.md` §2.3 (`knowledge/docs9303/Mrz_Field_Layout.md:39`): "TD3
unused personal number: pos 43 may be `0` or `<`." Checked directly against
`crates/mrz/src/checksum.rs::verify()` (confirmed live, lines 45-50) this session:

```rust
pub fn verify(field: &str, digit: char) -> bool {
    match (check_digit(field), char_value(digit)) {
        (Ok(expected), Ok(got)) => expected == got && (digit.is_ascii_digit() || digit == '<'),
        _ => false,
    }
}
```

For an all-`<` (unused) `personal_number`, `check_digit(field)` computes `0`
(filler counts as 0 under the ICAO arithmetic — the same rule `dates.rs` documents
for all-filler dates). Both `digit == '0'` and `digit == '<'` satisfy
`expected == got` — the `|| digit == '<'` arm exists precisely for this case (its
own doc comment says "used for empty optional fields"). Called at `parser.rs:223`
for TD3; TD1/TD2/MRV-A/MRV-B don't have this field and hardcode
`personal_number: true` (no check digit exists there per Doc 9303).

**Files.** `crates/mrz/tests/roundtrip.rs` (or a new small test file) —
`crates/mrz/src/checksum.rs::verify` needs no change.

**Design/Steps.** Add a regression test that hand-builds a raw TD3 zone with an
all-filler `personal_number` and a check-digit character of `'0'` in one case and
`'<'` in a sibling case, and asserts both parse paths independently succeed via
`parse_td3`/`find_and_parse` — not just at the `checksum::verify` unit level, so
the test proves the whole parse path honors the issuer option, not only the
primitive.

**Measurement.** `cargo test -p mrz` for the two new cases. No bench delta expected
or required — this doesn't touch parsing/repair behavior on any real corpus.

**Acceptance criteria.** Two new passing tests; no production code changed (if
implementation-time re-verification finds the primitive actually *doesn't* cover
this — re-check `checksum.rs`'s current line numbers first, since they drift — then
this becomes a one-line fix instead, and should be re-scoped accordingly).

**Size.** Small. **Dependencies.** None — do first, it's essentially free.

---

## Chunk 3 — `changelog.d/` housekeeping

**Goal.** Close two fragment-trail gaps before more M6 fragments land on top of a
trail with known holes.

**Why.** Per `changelog.d/README.md`, every PR that changes user-visible behaviour
drops one fragment. Two merged PRs don't have one:

- **PR #138** (`22e1cbc`, "mrz: fix line-1 right-shift (inserted-character)
  corruption on TD3/MRV-A/MRV-B") shipped `shift_line1_right_at_country`/
  `shift_or_unshift_line1` in `parser.rs` plus new tests in
  `crates/mrz/tests/line1_right_shift.rs`. This is distinct from the existing
  `changelog.d/m6-mrv-position1-shift.fixed.md`, which documents only the
  *unshift* (dropped-filler) direction from an earlier commit — #138 adds the
  mirror-image *shift* (inserted-character) direction and has no fragment of its
  own.
- **PR #142** (`938f075`, "docs: professionalize root docs, app polish, and
  screenshots") is mostly README/SECURITY.md/favicon/screenshot polish, but its
  diff also bumps `h2` to 0.4.16, fixing `RUSTSEC-2026-0258` (GHSA-q83h-524g-xf6h,
  unbounded empty DATA frame handling) — a real `security`-category, user-visible
  change with no fragment.

**Files.** Two new files under `changelog.d/`: `138.fixed.md`, `142.security.md`
(naming per `changelog.d/README.md`'s `<id>.<category>.md` convention — these are
retroactive, added now referencing the already-merged PR numbers, not new PRs).

**Steps.** Write the two fragments in the existing style (see
`changelog.d/m6-mrv-position1-shift.fixed.md` for the sibling shift-direction
fragment's tone). Dry-run `scripts/assemble-changelog.sh` (no `--write`) to confirm
they parse and land under the correct `###` heading.

**Measurement.** N/A — documentation-completeness fix only, no code/behavior
change.

**Acceptance criteria.** `scripts/assemble-changelog.sh` (dry run) shows both new
fragments correctly bucketed; `bash scripts/check-doc-links.sh` still passes.

**Size.** Small. **Dependencies.** None — fully parallelizable with everything
else.

---

## Chunk 4 — `IncompleteSequence` / line-count signal

**Goal.** Give "found N of M expected lines" a typed signal — the literal gap this
whole doc is named for.

**Why.** `find_and_parse_with` (`crates/mrz/src/parser.rs`, function starts ~line
987) builds a `lines: Vec<&str>` of candidate physical lines, then per format tries
to match a line-1 shape and looks ahead a bounded window for a companion line
(e.g. TD3's lookahead at ~line 1049). If a line-1-shaped candidate is found but no
companion line within the window ever produces a successful parse for *any*
format, the scan falls through to `MrzError::NotFound` — the same terminal error
as "nothing MRZ-shaped exists at all." There is no typed distinction today between
"found 0 of N lines" and "found 1 (or 2, for TD1) of N lines, shape-consistent, but
the rest never showed up."

**Files.** `crates/mrz/src/lib.rs` (new `MrzError` variant, beside the existing
`#[non_exhaustive]` enum, ~line 341-368); `crates/mrz/src/parser.rs`
(`find_and_parse_with`, ~line 987 onward).

**Design.**
```rust
#[non_exhaustive] // MrzError already carries this attribute
pub enum MrzError {
    // ...existing variants unchanged...
    IncompleteSequence { format: Format, lines_found: u8, lines_expected: u8 },
    NotFound,
}
```
Populate by tracking, during the existing scan loop, the best-shaped partial match
seen per format (reusing the per-format `starts_with`/charset/length guards
already in the loop — do not duplicate line-detection logic in a separate first
pass). Return `IncompleteSequence` instead of bare `NotFound` when such a partial
shape was observed and the ordinary fallback is also `None`.

**Steps.**
1. Add the variant to `MrzError`.
2. In `find_and_parse_with`, track the best partial-shape observation per format
   alongside the existing candidate search.
3. At the final `fallback.ok_or(MrzError::NotFound)` point, substitute
   `IncompleteSequence` when a partial shape was seen.
4. Add unit tests constructing OCR text with exactly N-1 of N expected lines per
   format (mirror the existing damaged/truncated fixtures in `crates/mrz/tests/`,
   e.g. `td1_line_gap.rs`'s style) asserting `IncompleteSequence{..}` instead of
   `NotFound`.

**Measurement.** `cargo test -p mrz` for the new unit tests. Then a **same-binary
A/B** on `synthpass-bench`: run the existing corpus before/after and confirm hit
rate is **bit-identical** — this is purely additive error-typing, not a repair; if
hit rate moves at all, something leaked into parsing logic and that's a bug, not a
feature. Report the `no_mrz_found` bucket (51 in the 2026-08-16 real-specimen run)
re-split into `IncompleteSequence` vs. true zero-shape misses, applying chunk 1's
diagnostic pattern to the `NotFound` side.

**Acceptance criteria.** New tests pass; synthetic-corpus hit rate unchanged to the
bit; real-specimen `no_mrz_found` breakdown reported in the PR description.

**Size.** Medium (touches the hot scanning loop — care needed not to slow down or
change behavior of the common path). **Dependencies.** None strictly, but should
land before chunk 5, whose type shape depends on this signal's real shape.

---

## Chunk 5 — `SequenceCompleteness` unified type

**Goal.** Give `crates/mrz` one coordinated completeness vocabulary instead of
four uncoordinated ones, without breaking any existing consumer.

**Why.** See "Why this exists" above — `Checks`, `DateCompleteness`,
`repair::Resolution`, and `synthpass-core::fusion`'s heuristics don't compose
today.

**Files.** `crates/mrz/src/lib.rs` (new type, additive field on `MrzData`);
`crates/mrz-wasm` (verify projection compat — see below); spot-check
`synthpass-core`/`synthpass-die` build cleanly after.

**Design.**
```rust
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SequenceCompleteness {
    pub lines_found: u8,
    pub lines_expected: u8,
    pub checks: Checks,                    // wraps, does not duplicate
    pub date_of_birth: DateCompleteness,    // wraps, does not duplicate
}
```
Name is a placeholder — check for collisions with `DateCompleteness` before
finalizing. **Wraps** `Checks`/`DateCompleteness` rather than replacing them: both
stay public with unchanged shapes, because `mrz-wasm`, `synthpass-core`, and
`synthpass-die` consume them directly today, and changing either shape would be a
breaking 0.x bump every downstream crate has to absorb at once. Lives beside
`Checks` as a new field on `MrzData`, which is already `#[non_exhaustive]`
precisely so additions like this aren't breaking (see `Format`'s own doc comment:
"Adding MRV-A/MRV-B in 0.3.0 was breaking precisely because this attribute was
missing" — do not repeat that mistake here).

**Open question to resolve during implementation, not guess at now:** does
`date_of_expiry` get an analogous completeness treatment, or does the doc/type
just document the current DOB-only asymmetry as-is? Either is fine; pick one and
say so in the doctest/doc comment rather than leaving it ambiguous.

Deliberately does **not** try to fold in `fusion::check_line1_integrity`'s
verdict — that heuristic layer stays in `synthpass-core` on purpose (no
check-digit backing, needs cross-crate context). Keep that separation; this chunk
unifies only `crates/mrz`'s own internal vocabularies.

**Measurement.** Type addition, not a behavior change: `cargo test -p mrz` green
(existing suite, since the field is additive) plus a new unit/doctest constructing
`SequenceCompleteness` for each of the five `Format` variants. `cargo check`
across `mrz-wasm`/`synthpass-core`/`synthpass-die` to confirm nothing that
pattern-matches `MrzData` without `..` breaks. No bench delta expected.

**Acceptance criteria.** All downstream crates build unchanged; wasm projection
verified the same way `DateCompleteness` itself was projected when it shipped in
0.6.1 (check `crates/mrz-wasm` for that precedent before assuming this is free —
a `#[non_exhaustive]` struct wrapping an enum needs the same treatment).

**Size.** Medium (touches a widely-consumed public type; wasm-compat check is the
risky part). **Dependencies.** Land after chunk 4 — this type's `lines_found`/
`lines_expected` fields should be shaped by the real signal chunk 4 produces, not
guessed in isolation.

---

## Chunk 6 — [DONE, revised] TD2 line-1 repair via issuing-country arbitration

**Status: implemented, measured, not the originally-proposed design.** The
document-code-dictionary arbitration this section originally specified turned out
to rest on a false premise, caught at design time (before writing the dictionary)
by checking ICAO 9303 directly rather than assuming it existed. What actually
shipped, why, and the measurement that cleared the gate are below; the original
proposal is kept struck through afterward for the record.

**Goal.** Give TD2 the line-1 dropped-filler repair TD3/TD1/MRV-A/MRV-B already
have.

**Why deferred, and why harder than its siblings.** `repair_td2_line1`
(`crates/mrz/src/parser.rs`) deliberately did not call `unshift_line1_prefix`,
unlike TD3/MRV-A/MRV-B (unambiguous single-letter-plus-filler document code) and
unlike TD1 (arbitrates via its line-1 document-number check digit). TD2 shares
TD1's genuine two-letter document-code family (e.g. `"ID"`, `"IP"`) so "position 1
isn't `<`" is not unambiguous evidence of a dropped filler — but unlike TD1, TD2
has no check digit anywhere on line 1 to arbitrate between the shifted and
unshifted reading.

**The dictionary premise didn't survive contact with the spec.** Before writing a
"known-legitimate TD2 document code" table, ICAO 9303 Part 5 §Note k / Part 6
§Note k were checked directly: *"The second character shall be at the discretion
of the issuing State or organization except that ... V shall not be used, and C
shall not be used after A."* That's not a closed enumerable set — it's
issuer-discretionary with two exclusions. There is no normative registry to build
a dictionary from, and a table assembled from guessed/observed real-world codes
would be exactly the unproven heuristic this crate's "prove it or don't touch it"
discipline (`repair.rs::solve_substitution`) and `synthpass-core/src/fusion.rs`'s
reverted 69.5%→61.5% heuristic both argue against. This chunk's implementation-time
task ("confirm whether ... a closed registry ... already enumerates valid TD2
document codes") resolved to **no**, and the dictionary approach was dropped
before any table was written — not measured and rejected, ruled out at design
time.

**What shipped instead: reuse the issuing-country gate, not a new dictionary.**
TD3/MRV-A/MRV-B's own drop-repair (`shift_or_unshift_line1`) already arbitrates
its *insertion*-repair half without a document-code table, using
[`country_name`] on the unshifted reading's issuing-country slot (positions 2..5)
as a content-based gate. TD2's line-1 layout puts `issuing_country` at those same
offsets, and `country_name` — unlike a document-code table — *is* a genuine
closed ICAO/ISO registry already in production use. `repair_td2_line1_shifted`
reuses that exact gate (factored out as `unshift_if_country_resolves`, shared
with `shift_or_unshift_line1`): unshift line 1, accept the unshifted reading only
if its issuing-country slot resolves to a real country. A genuine two-letter code
(`"IP"` + `"BRA"`) unshifts to a garbage country slot (`"PBR"`) and is correctly
left untouched; a genuine drop (`"I<"` + `"BRA"` corrupted to `"IBRA..."`) unshifts
back to a resolving country slot (`"BRA"`) and is correctly recovered. Scope is
the dropped-filler direction only — `shift_line1_right_at_country`'s
insertion-repair mirror image is not wired in for TD2, matching this chunk's
original goal statement.

**Measurement (the gate this section originally specified).**
1. *Real-specimen re-scan: not applicable.* `samples/` has no TD2 ("official
   travel document") real-specimen directory — only `passports/`, `id_cards/`,
   `driving_licenses/` exist, matching what this section already anticipated
   ("once TD2 real specimens exist"). Re-run this step if/when a TD2 real corpus
   is added.
2. *Same-binary A/B, synthetic TD2 corpus* (`--document-type td2 --profile clean
   --count 30 --seed 42`, matching `unshift_line1_prefix`'s own measurement
   convention): **hit rate unchanged, 80.0% → 80.0%** (24/30 both builds — no
   regression, the gate's hard requirement). Among the 24 Tier-1 hits, records
   still carrying a line-1 integrity finding dropped **100% (24/24) → 37.5%
   (9/24)**. `document_type` CER **100.00% → 26.67%**; `issuing_country` CER
   **68.89% → 20.00%** — the same shape and magnitude of improvement already
   measured for TD3/MRV-A/MRV-B's sibling fix.
3. New regression tests: `crates/mrz/tests/td2_line1_repair.rs` pins both the
   recovery case (`a_dropped_line_1_filler_position_is_recovered`) and a no-op
   guard (`an_undamaged_td2_zone_is_not_spuriously_unshifted`); the pre-existing
   genuine-two-letter-code guard
   (`crates/mrz/tests/line1_prefix_shift.rs`'s
   `td2_with_a_genuine_two_letter_document_code_is_unaffected`) was re-verified,
   not just re-run — traced through the fix's own gate logic in its doc comment.

**Go/no-go outcome:** zero hit-rate regression, no real-specimen corpus exists to
false-positive against. Cleared to merge.

**Size.** Was expected to be the riskiest chunk in this doc; turned out small once
the dictionary premise was dropped for a reused, already-measured gate.
**Dependencies.** None — implemented directly on `main`, independent of chunks 4/5.

<details>
<summary>Original proposal (superseded — kept for the record, not followed)</summary>

**Proposed arbitration strategy.** A closed-dictionary document-code match, since
check-digit arbitration is unavailable: if the *unshifted* reading's putative
document-code slot matches a known-legitimate TD2 code from a closed registry
**and** the *shifted* (as-read) slot does not, prefer unshift; if both match, or
neither does, leave the reading as-is — do not guess. Implementation-time task:
confirm whether ICAO 9303 Part 6 or this crate's existing registry already
enumerates valid TD2 document codes, or whether a new small table needs building.
(Resolution: no such registry exists — see above.)

**Why this was flagged as genuinely risky.** The arbitration signal (dictionary
match) is strictly weaker than a check digit: a corrupted read can *happen* to
land on another legitimate code in a way arithmetic checksums cannot be fooled
by — the same failure mode `synthpass-core/src/fusion.rs`'s reverted heuristic
hit. This suspicion is why the premise got checked against the spec before any
table was written, rather than after a bad measurement.

</details>

---

## Chunk 7 — [DONE, narrowed] `MrzReader` partial-read surfacing experiment

**Status: sign-off received, implemented, flag-gated and off by default.**
Product-risk sign-off: **yes, add the new `FieldConfidence` band**, flag-gated,
same-binary A/B before merge — obtained before any code was written, per this
chunk's own gate. What shipped is **narrower** than this section originally
proposed; the narrowing and why are below, along with the measurement.

**Goal.** Determine whether surfacing `find_and_parse`'s already-computed partial
read reduces unnecessary Tier-2 escalation without hurting accuracy.

**Current behavior (before this chunk).** `MrzReader::read()`
(`crates/synthpass-die/src/mrz_reader.rs`) called `evidence.observe_mrz(&data)`
(recording per-field evidence including which check digits passed) before the
validity gate, then discarded everything whenever even one check digit failed.

**What shipped — narrower than "partially valid," and deliberately so.** The
original proposal said "when check digits are *partially* valid." Implemented
instead: only when `mrz::Checks::only_composite_failed()` is true — the
composite is the *only* failing digit, and all four individually
check-digited fields (`document_number`/`date_of_birth`/`date_of_expiry`/
`personal_number`) independently verified. A looser "any individual field's
digit failed but let's surface the others anyway" reading of "partially valid"
would report a field sitting *right next to a proven-wrong one* as if nothing
were amiss — the four fields share no arithmetic independence from each other
the way this crate's own repair machinery already assumes elsewhere. The
composite-only case has no such problem: every individual digit that exists
genuinely passed; the only thing wrong is the *separate*, differently-weighted
whole-zone check (`Checks::only_composite_failed`'s doc comment has the exact
arithmetic reasoning — composite recovers the same characters at a different
7-3-1 phase, so it is a real independent check, not a redundant one).

**Confidence band.** `FieldConfidence::mrz_checksum_scope_partial()`
(`crates/synthpass-core/src/v2.rs`) — the private `CHECKSUM_PARTIAL = 0.95`
constant, strictly between `MRZ_STRUCTURAL` (0.9) and `PROVEN` (1.0). Exact
placement is a judgement call, explicitly documented as one (no calibration
curve exists, same honesty `MRZ_STRUCTURAL`'s own doc comment already applies
to itself) — a future real-world sweep may move it, not this chunk.

**Flag surface.** `MrzReader::with_partial_read_surfacing()` (constructor,
default `MrzReader::new()` unaffected) and
`RoutingPolicy::accept_composite_only_failure` (field, `false` in
`v1_2_0_compatible()`) — both off by default, both required together for the
narrow case to actually change a routing decision (the reader narrows
`evidence.missing`; the policy is what stops clause 2 auto-escalating on it).
Neither is wired into `synthpass-pipeline`'s production catalog
(`MrzReader::new()`/`RoutingPolicy::default()` there are untouched) — this
ships as a measurable primitive, not a new default.

**Measurement.**
1. *Same-binary, flag off:* `cargo test -p synthpass-die` — the exhaustive
   `default_policy_is_exactly_the_v1_2_0_predicate` test (all opt-in
   combinations) and dedicated composite-only-failure tests in both
   `mrz_reader.rs` and `routing.rs` all pass; flag-off behavior is provably
   byte-identical to before for every case, not just spot-checked.
2. *Reachability, synthetic corpus* (`--profile all --count 150 --seed 0`,
   150 documents × 5 OCR-noise profiles each): how often the narrow
   composite-only case actually occurs, counted from `failing_checks`
   (Chunk 1's diagnostic) on every `checksum_failed` miss:
   - **TD3: 0 of 150 (0%).** Every observed composite failure co-occurred
     with `document_number` or `personal_number` also failing.
   - **TD1: 11 of 150 (7.3%).**
   - **TD2: 7 of 150 (4.7%).**
   - **MRV-A/MRV-B: not applicable** — `composite` is hardcoded `true` for
     both formats (no composite check digit exists at all per ICAO 9303
     Part 7), so `only_composite_failed()` can never be true there.

   This is real signal, not noise: the scenario this chunk targets is
   near-absent for TD3 under this corpus's noise model, but a meaningful
   single-digit percentage of TD1/TD2 documents. Whether that is worth
   pursuing further depends on TD1/TD2's real-world traffic share, which
   this measurement doesn't have.
3. *Full pipeline-level Tier-2-escalation-rate A/B — not done, and
   deliberately not bundled into this PR.* Running it for real requires
   wiring `with_partial_read_surfacing`/`accept_composite_only_failure`
   through `synthpass-pipeline`'s catalog construction and exposing a
   `provider-bench`/`synthpass-bench` toggle — a separate, materially larger
   integration task than "the code change is small" scoped for this chunk.
   Recommended as the next step before ever flipping either default, using
   the reachability numbers above to decide whether TD1/TD2 alone justify it.

**Files.** `crates/mrz/src/lib.rs` (`Checks::only_composite_failed`);
`crates/synthpass-core/src/v2.rs` (`CHECKSUM_PARTIAL`,
`FieldConfidence::mrz_checksum_scope_partial`); `crates/synthpass-die/src/evidence.rs`
(`Evidence::checksum_partially_valid`); `crates/synthpass-die/src/mrz_reader.rs`
(`MrzReader::with_partial_read_surfacing`, `extraction_v2_from_mrz_partial`);
`crates/synthpass-die/src/routing.rs` (`RoutingPolicy::accept_composite_only_failure`).

**Size.** Was scoped as medium (small code, real measurement work); the actual
code stayed small and additive as expected. **Dependencies.** Used chunk 1's
`failing_checks` diagnostic for the reachability measurement, as anticipated.
