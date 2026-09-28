# V2-DESIGN — the `ExtractionV2` schema

> **Status:** written retroactively (2026-08) to close a citation gap: eleven
> doc comments across `synthpass-core`, `synthpass-pipeline`, and
> `synthpass-serve` cited this file by section number since the schema
> landed, and it had never been committed. See
> [`technical_debt.md`](technical_debt.md) for how that happened and
> [`ROADMAP.md`](ROADMAP.md)'s M5 entry for the milestone this shipped under.
> Section numbers below (`§3`, `§4`, `§9`, `§11`, `§12`, `§13`) are chosen to
> match those existing citations (`§13` added alongside the wiring it
> documents — issue #565), not to imply sections `§1`, `§2`, `§5`–`§8`,
> `§10` were ever written or planned — they weren't; don't add them
> speculatively.
>
> **"v2.0.0" in this file always means the `ExtractionV2` *schema* generation, never a
> workspace or crate release.** The schema shipped inside the 1.x line (under M5); the
> workspace is at 1.4.0 and there is no planned 2.0.0 release of it. Where the text says a
> shim is "scoped to one major release", read that as one schema generation.

## §3 — Why a versioned schema, and what's in it

v1's [`Extraction`](../crates/synthpass-core/src/lib.rs) is MRZ-shaped:
every field is a scalar the deterministic reader either found or didn't.
That was sufficient while Tier 1 (checksum-proven MRZ) was the only serious
producer, but two things outgrew it once Tier 2 (in-process LLM) and OCR
region detection landed:

- **Confidence had nowhere to live.** A Tier-2 field and a Tier-1 field were
  wire-indistinguishable — both just a string. A consumer had no way to tell
  "checksum-proven" from "the model's best guess."
- **New capabilities had no slot.** Portrait cropping, barcode decoding, and
  multi-document input (e.g. a passport photo page plus its visa) were all
  either scoped or in flight, and v1's flat field list had nowhere to add
  them without either breaking the wire format or bolting on ad-hoc optional
  fields per capability.

[`ExtractionV2`](../crates/synthpass-core/src/v2.rs) fixes both without
breaking v1: every v1 scalar field lives verbatim under `ExtractionV2.fields`,
and the additions are either metadata *about* the extraction — `confidence`
(§11 covers what this does and does not claim), `provenance`, per-check-digit
detail — or empty slots for capabilities landing in later milestones:
`portrait` (§4), `barcodes` (§12), and `documents`.

`documents: Vec<ExtractionV2>` is the multi-document slot, reserved for M4's
originally-scoped multi-page input (a passport bio page plus an attached
visa, scanned as one job). It is declared now, always empty in v2.0.0, with
`#[serde(default, skip_serializing_if = "Vec::is_empty")]` so a later
milestone can start populating it without a schema bump — the wire format is
already future-proof for it.

The wire format keeps v1's `snake_case`, and `ExtractionV2::schema_version`
is always serialized (even when it equals the default) so a consumer can
dispatch on schema before touching anything else. See
`crates/synthpass-core/src/v2.rs` module doc for the exact `Zeroize`
discipline this schema follows — unchanged from v1's.

## §4 — The portrait slot

`ExtractionV2.portrait: Option<ImageRef>` is a bounding box of the portrait
(face) region in the source image — a slot, not a feature. The cropping
heuristic that populates it is scoped to a later milestone; in v2.0.0 the
field exists on the wire (so a client written against v2.0.0 doesn't need a
schema change to consume it later) but stays `None`.

`ROADMAP.md`'s M7 escalation-signal list (evidence-driven escalation) already
treats "portrait detection" as a *routing* signal distinct from this slot —
whether a portrait was found on the page can inform tier escalation even
before the crop itself is exposed to the caller. Don't conflate the two: the
routing signal is internal (`Evidence`, never serialized — see
`synthpass-die/src/evidence.rs`); this slot is the public, wire-visible
result once the cropping heuristic ships.

This slot carries **no face-recognition capability, ever** — see §11.

## §9 — The v1 → v2 deprecation shim (breaking changes B2, B3)

v2.0.0 changes the *default* shape served or written in two independent
places. Both are one-release-only compatibility shims — the goal is v1
consumers get a deprecation window, not permanent dual-format support.

**B2 — `synthpass-serve`'s API default.** `/api/extract`'s SSE `result`
event carries `extracted_v2` by default. A legacy client keeps the v1-only
shape by asking for it explicitly: `?v=1` on the query string, or
`Accept: application/vnd.mlis.v1+json`. The negotiation is a pure function
of the request parts (`wants_legacy_v1` in
`crates/synthpass-serve/src/main.rs`), so it's unit-testable without a live
server.

**B3 — `synthpass-pipeline`'s on-disk artifact default.** `write_outputs`
persists the **v2 shape by default**; setting `SYNTHPASS_JSON_V1=1` writes
the legacy v1 shape to `<input>.json` instead. `PipelineResult.extracted:
Option<Value>` (the parsed v1-shape JSON) stays populated for the duration
of this shim specifically so callers reading the old field don't silently
get `None` — new consumers should read `PipelineResult.extracted_v2`
instead.

Both shims are scoped to **one major release**: the plan is v1 output goes
away entirely once v2.0.0's consumers have migrated, not that the two
formats coexist indefinitely. There's no separate `B1`; the label sequence
starts at `B2` because `B1` was the original v1→v2 schema-shape change
itself (introducing `ExtractionV2` as a type), which isn't a *breaking*
change to any deployed default — it shipped alongside v1, not instead of it.

## §11 — Extraction certainty is not document authenticity

`FieldConfidence`'s scores (`1.0` = checksum-proven Tier 1; anything below is
a heuristic Tier-2 model score) describe *extraction certainty*: how
confident the pipeline is that it read the document correctly. They say
nothing about whether the document itself is genuine.

This is a permanent non-goal, not a v2.0.0 scoping decision — stated in
[`VISION.md`](VISION.md)'s non-goals callout:

> SynthPass crops a portrait region; it never *identifies* a person — no
> face recognition, no biometric matching, no liveness. A checksum-valid MRZ is
> *consistent* with its check digits, not proof of a byte-identical *read*; it does not judge
> document *authenticity* — forgery and tamper detection are out of scope. It does not do
> cloud anything. These lines do not move.

and restated in [`project_principles.md`](project_principles.md)'s "what
these rule out" list: "No authenticity or forgery detection. A checksum-valid read is
*consistent* with its check digits, not proof of a faithful *read* — and never proof of a
genuine *document*."

Concretely: a checksum-valid MRZ shows the arithmetic on the check digits agrees — consistent
with the extraction pipeline having transcribed that data correctly, not proof of it (blind
character pairs and multi-cell errors can still cancel mod 10, and TD2/TD3 line 1 carries no
check digit at all). Either way, it proves nothing about whether the document was forged,
altered, or issued to the person presenting it. `ExtractionV2.confidence` and
`FieldConfidence` must never be read, displayed, or documented as an
authenticity signal — every field that inherits this constraint (`portrait`
in §4, `confidence` itself) says so at its own definition site rather than
relying on a reader having found this section first.

## §12 — Barcodes: a slot, not a decoder

`ExtractionV2.barcodes: Vec<BarcodeHit>` reserves space for barcode hits
(PDF417 and similar) on the wire. **No decoder ships in v2.0.0** — the field
is always empty until one does. This mirrors §4's portrait slot: declaring
the shape now means a client written against v2.0.0 doesn't need a schema
change when decoding lands.

The concrete driver is `ROADMAP.md`'s M6 scoping: US/Canada driving licences
carry no MRZ at all — their data lives entirely in an AAMVA PDF417 barcode on
the card back, an independent standard family outside ICAO Doc 9303. No
amount of MRZ work reads one; it needs a PDF417 decoder plus an AAMVA
field-layout parser, scoped as its own provider against the M7
`synthpass-die` contract rather than folded into MRZ work where it would
quietly fail forever. See `ROADMAP.md`'s "Beyond ICAO 9303" section for the
full scoping — this file only documents that the schema slot exists and why
it's still empty.

## §13 — Occlusion: covered MRZ cells are reported, never reconstructed

Issue #565, [ADR-0026](decisions/ADR-0026-covered-cells-are-occluded.md). A
box or a heavy blur inside an MRZ zone reads today as if it were print — the
zone can be checksum-valid while a covered name comes back as garbled text
with no sign anything was covered at all. This section covers the two schema
keys and the one thing v1 cannot say about them. Nothing populates them yet:
detection is not built, and ADR-0026 decision 8 ships it off by default until
it is measured.

**The two keys.**

- `ExtractionTrace.mrz_occlusion: Option<MrzOcclusion>` — the per-document
  *observation*: `{spans: [{line, first, last, kind}]}`, where `kind` is
  `"fill"` or `"blur"`. It sits beside `escalation`, not
  inside `config_overrides` (`config_overrides` is configuration that holds
  for every document a process reads under it; an occlusion span is a fact
  about *this* document). Spans are cell positions only, never the covered
  text, so the trace stays PII-free even though it always accompanies a
  PII-bearing record. Omitted when `None`; `ExtractionTrace::is_empty()`
  returns `false` when only an occlusion is present, so a caller that gates
  attaching a trace on `is_empty` never drops the only occlusion evidence a
  record carries.
- `ExtractionV2.occluded: Vec<CoreField>` — the per-document *consequence*:
  which ICAO fields the observation above caused to be withheld. Sorted and
  deduplicated, omitted when empty. A field named here is `null` in `fields`
  and `FieldConfidence::OCCLUDED` (`0.0`) in `confidence`.

**Three states, kept apart on purpose.** A field can be empty for three
different reasons. Before these keys existed, a covered field and an unread
one were both `null` with nothing to tell them apart:

| State | `fields.<field>` | `confidence.<field>` | listed in `occluded` |
| --- | --- | --- | --- |
| Covered — the image shows it occluded | `null` | `0.0` | yes |
| Read as nothing — the field is genuinely blank (e.g. no personal number) | `""` | whatever the read scored | no |
| Not read at all — no provider reached it | `null` | `0.0` (the struct default) | no |

**The v1 limitation.** v1's `Extraction` has no status vocabulary at all —
every field is `Option<String>`, with no sibling list of *why* a field is
`null`. An occluded field's v1 value is `null`, exactly like a field that was
simply never read; a v1-only consumer cannot recover which. This is
permanent, not a gap this milestone will close later: `ExtractionV2::from(&Extraction)`
(the v1→v2 lift) cannot invent an `occluded` list a v1 record never carried,
and it doesn't try — see `lift_never_recovers_occlusion_v1_has_no_vocabulary_for`
in `crates/synthpass-core/src/v2.rs`. A client that needs to know *which*
fields were covered must read `ExtractionV2`.

**Not covered here.** The detector that fills `mrz_occlusion`, and the
Tier-2 masking that keeps a model from filling a covered field (ADR-0026
decision 6), are not built yet. Until they are, every record serializes
byte-identically to one without these keys. ADR-0026 has the full design,
including why a covered check-digited cell is refused outright rather than
reported (decision 7) and why the wire word is `occluded`, never `redacted`
(decision 1).
