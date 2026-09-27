# ADR-0023 — The CLI's output contract: `--json`, and stdout/stderr for everything else

**Status:** Accepted (the owner's #493 decision of 2026-09-26)
**Date:** 2026-09-27

## Context

`synthpass`'s extraction path has always printed a mix of progress ("🔄 Processing local
file..."), diagnostics ("✅ OCR successful!", "🎉 Pipeline completed via ...!"), and content (the
pretty-printed `extracted` JSON, for a Tier-1 result) to the same stream, stdout. That was fine
while nothing consumed the output but a human at a terminal. It stops being fine the moment a
script wants to pipe `synthpass`'s output into `jq`, a log shipper, or anything else that expects
one JSON value (or one JSON value per line) and nothing else on the stream it reads — a stray
"🔄 [Rust] Processing local file: ..." line ahead of the JSON breaks every one of those
consumers, silently, because the failure mode is a parse error on the *first* line, not a missing
field on a later one.

Issue #493 asked for a script-parseable contract. Issue #497 asked for contract tests of the
CLI's main surface, and its other items have shipped: the glob and batch-input tests (#503), exit
codes (#505), `parse_args` (#517), licence exit 3 and metering (#523) and `doctor` (#525). Its
golden stdout test waited on this contract, the part with real design surface: *what*, exactly,
goes on which stream, in which mode, and whether it changes what ships today.

Two things this project already ships constrain the answer:

- **The v1→v2 schema migration** (`knowledge/V2-DESIGN.md`) established `ExtractionV2`
  (`PipelineResult::extracted_v2`) as the canonical extraction record — per-field confidence,
  provenance, the line-1 integrity verdict — and kept the v1 `extracted` shape only for one
  release's compatibility (breaking change B3). A new machine-readable contract should speak the
  schema the rest of the codebase is migrating toward, not perpetuate the v1 shape a scripting
  consumer would have to migrate away from again.
- **`knowledge/ARCHITECTURE.md` §12's exit-code table** (issue #492) is already the CLI's
  script-facing contract for *outcome*. A new contract for *content* should not reopen it.

## Decision

**stdout is human-readable by default. It becomes a contract only under an explicit `--json`.**
In both modes, every progress and diagnostic line — the config echo, "Processing local
file...", "OCR successful!", batch's per-run summary line, in short every line carrying an emoji
— goes to stderr, never stdout. Only extraction *content* is stdout material, and what counts as
content is exactly what changes between the two modes:

| | stdout content |
| --- | --- |
| `synthpass <image>` (default) | unchanged: the legacy pretty-printed v1 `extracted` JSON for a Tier-1 result; nothing for a Tier-2 result (an existing asymmetry this ADR does not fix — see Consequences) |
| `synthpass <image> --json` | exactly one line of compact JSON: the v2 [`ExtractionV2`](../../crates/synthpass-core/src/v2.rs) object, when extraction produced a usable result; nothing when it didn't (the error still reaches stderr) |
| `synthpass batch <dir\|glob>` (default) | unchanged: one pretty-printed per-document record per line, including a failed document's `{"input", "error"}` |
| `synthpass batch <dir\|glob> --json` | [JSON Lines](https://jsonlines.org/): one compact `ExtractionV2` object per line, one line per document that actually produced one, in the same deterministic input order the non-`--json` output already uses |

Framing is the same for both `--json` paths: one compact (not pretty-printed) JSON object per
line, each terminated by `\n`. Single-document `--json` is a *degenerate* JSON Lines stream (one
line), not a differently-framed pretty object, so a consumer never has to special-case which
command produced a line it's reading.

**A document that fails extraction writes no stdout line under `--json`** — in `batch`, that
covers a genuine `Failed`/`Pending` job status *and* a `Done` result whose Tier-2 fallback itself
failed (`extracted_v2` is `None` exactly when `llm_error` is set); in single-document mode, the
same condition (`print_completion`'s success/failure verdict, issue #492) gates the one possible
line. Either way the error still goes to stderr — a script parsing stdout as JSON Lines must
never see a non-JSON or partial line for a document that failed, and must never lose the error
either.

**Exit codes are unchanged** (`knowledge/ARCHITECTURE.md` §12) — `--json` changes what this
process prints to its own stdout, not what it returns. **The `<input>.json` sidecar file
extraction writes to disk is unchanged either way** — `--json` is a flag on the process's stdout,
not a new persistence format.

**Default mode's stdout content is unchanged from before this issue**, deliberately: this is an
additive change (a new flag), not a breaking one, so nothing that scripts today's default stdout
sees a different value. That includes keeping the existing asymmetry where a Tier-2 (LLM) result
prints nothing to stdout in default mode at all — fixing that is a separate, breaking decision
this ADR does not make.

## Alternatives considered

- **Make `--json` the only way to get JSON, breaking default mode's existing pretty-print.**
  Rejected: default mode's stdout content is a de facto contract for anyone already piping
  `synthpass`'s output, even without `--json` having existed to name it. Changing it silently
  under the same invocation is a breaking change with no flag to opt into or out of; issue #493's
  framing ("additive, not 2.0") rules it out directly.
- **Also print the v2 object to stdout for a Tier-2 result in default (non-`--json`) mode**,
  closing the asymmetry noted above while we were already touching this code. Rejected for the
  same reason as the previous point: "keep stdout's content as today" means *today's* content,
  quirk included. Closing the asymmetry is real cleanup, but it changes what a script watching
  default-mode stdout for a Tier-2 document sees, which makes it its own (likely breaking)
  decision, not a rider on an additive one.
- **Pretty-print `--json`'s output** (`serde_json::to_string_pretty`), matching default mode's
  style. Rejected: multi-line pretty JSON is not one-value-per-line, so a single `--json`
  document's output could not share framing with `batch --json`'s JSON Lines without either
  making single-document `--json` its own special case or making `batch --json` pretty-print too
  (defeating JSON Lines' point — one line per record is what makes it streamable/`grep`-able
  without a JSON parser in the loop). Compact output is also the conventional choice for
  machine-readable CLI output generally (`jq -c`, `docker inspect --format`, etc.).
- **A document that fails extraction under `batch --json` still writes a stdout line**, e.g.
  `{"error": "..."}`, mirroring default mode's behavior. Rejected: default mode's per-document
  record is not typed — it is one of two ad hoc shapes (`{"input","method","extracted","error"}`
  or `{"input","error"}`) that a human (or a lenient parser) can tell apart, but `--json`'s
  contract is specifically "one `ExtractionV2` object per line" — mixing in a differently-shaped
  error object breaks that contract for the same reason a stray progress line would. The error
  still has to go somewhere machine-visible, so it goes to stderr instead, keeping the two
  streams' jobs separate: stdout is successful records, stderr is everything else including
  failures.
- **v1 `extracted` (not v2 `ExtractionV2`) as `--json`'s content.** Rejected per the schema
  context above: a new contract should target the schema the rest of the codebase treats as
  canonical, not the one already scheduled to be dropped after v1's one-release compatibility
  window.

## Consequences

- **A script that greps `synthpass`'s stdout for its old progress lines breaks.** That is the
  intended effect of the change, not a side effect: stdout no longer carries those lines in
  *either* mode, and they are still on stderr for anyone who wants them.
- **The Tier-2-prints-nothing-to-stdout asymmetry in default mode survives this ADR unfixed.**
  Anyone who wants uniform content for every result, tier included, should use `--json`, whose
  content rule is not tier-conditional: a usable v2 result prints, tier or no tier.
- **Golden tests pin the contract, not a live extraction.** A true end-to-end `--json` extraction
  needs the real `.rten`/GGUF models this workspace's `cargo test --workspace` job cannot assume
  are staged, and debug-mode `rten` inference runs on the order of minutes per pass — the same
  reason `native_ocr_e2e`/`rust_ocr_smoke` are `#[ignore]`d and run in release mode as separate CI
  steps. So the content contract (item 1/2 of #497's ask) is pinned against a constructed
  `ExtractionV2`/`DocumentStatus`, and the stream-split contract (item 3) against a real subprocess
  whose OCR stage is made to fail fast and deterministically (an empty model directory,
  `SYNTHPASS_OCR_AUTO_DOWNLOAD=0`) rather than run real inference. See
  `crates/synthpass-cli/tests/json_contract.rs`'s module doc and the `src/main.rs` unit tests it
  points to for the split in full.

## What would reverse it

Closing the Tier-2 default-mode asymmetry, or breaking default mode's stdout shape outright, are
both real possibilities this ADR leaves open — they are just breaking changes this one declines to
bundle with an additive flag. Either would get its own ADR (and its own `!` changelog fragment)
when someone decides it's worth the break.
