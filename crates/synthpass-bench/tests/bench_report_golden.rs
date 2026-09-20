//! Golden-file test for `bench-report`'s Markdown generator.
//!
//! Fixtures under `tests/fixtures/bench_report/` are hand-synthesized, not a
//! reduced real artifact: every SHA, count, and manifest row is a made-up
//! number chosen to exercise every rendered section (a hit, every outcome
//! bucket including several zeros, ADR-0013 `strict_names` present, more
//! than one `dir`/`provenance` combination, a row with no recorded year, and
//! more than one `origin.licence` value). No real specimen zone text, read
//! name, or document number appears anywhere in this file or its fixtures.
//!
//! The 10 `documents_detail` rows in `gate-report.json` are built to
//! exercise every branch of the per-format/per-issuer accuracy cut at once:
//! three ICAO formats plus one "no format resolved" document; one
//! `asset_id` with no corresponding `corpus.jsonl` row (a join failure); and
//! one document with no `asset_id` at all. Every group's documents/scored/
//! hits sums to the same totals as the Headline and Outcome table — worked
//! out by hand in the branch's handoff report and confirmed against this
//! file's own generated output before being frozen as the golden file.
//!
//! This test proves [`synthpass_bench::bench_report::render_markdown`] is a
//! pure, deterministic function of its three inputs: same bytes in, same
//! bytes out, byte for byte, every run. It does **not** prove the binary's
//! CLI argument parsing or file I/O are correct (see
//! `bench_report_cli_smoke` below for that), and it does not prove the
//! *content* is materially correct for a real gate report + baseline pair —
//! only that whatever this generator currently produces from these fixed
//! inputs does not silently drift.

use synthpass_bench::bench_report::{
    parse_baseline, parse_corpus, parse_gate_report, render_markdown,
};

const REPORT_JSON: &str = include_str!("fixtures/bench_report/gate-report.json");
const BASELINE_JSON: &str = include_str!("fixtures/bench_report/baseline.json");
const CORPUS_JSONL: &str = include_str!("fixtures/bench_report/corpus.jsonl");
const GOLDEN_MD: &str = include_str!("fixtures/bench_report/golden.md");

#[test]
fn render_markdown_matches_the_golden_file_byte_for_byte() {
    let report = parse_gate_report(REPORT_JSON).expect("parse fixture gate report");
    let baseline = parse_baseline(BASELINE_JSON).expect("parse fixture baseline");
    let corpus = parse_corpus(CORPUS_JSONL).expect("parse fixture corpus");

    let markdown = render_markdown(&report, &baseline, &corpus).expect("render fixture inputs");

    assert_eq!(
        markdown, GOLDEN_MD,
        "bench-report's Markdown output changed — if the change is intentional, regenerate \
         tests/fixtures/bench_report/golden.md from this generator's own output and review the \
         diff by hand (never edit the golden file directly)"
    );
}

#[test]
fn render_markdown_is_deterministic_across_repeated_calls() {
    let report = parse_gate_report(REPORT_JSON).expect("parse fixture gate report");
    let baseline = parse_baseline(BASELINE_JSON).expect("parse fixture baseline");
    let corpus = parse_corpus(CORPUS_JSONL).expect("parse fixture corpus");

    let first = render_markdown(&report, &baseline, &corpus).expect("first render");
    let second = render_markdown(&report, &baseline, &corpus).expect("second render");
    assert_eq!(
        first, second,
        "two renders of the same inputs must be byte-identical"
    );
}

#[test]
fn a_document_count_mismatch_between_report_and_baseline_is_rejected() {
    let report = parse_gate_report(REPORT_JSON).expect("parse fixture gate report");
    let mut baseline = parse_baseline(BASELINE_JSON).expect("parse fixture baseline");
    // The fixture pair agrees on `documents: 10` by construction; break that
    // agreement deliberately to prove a mismatched artifact pair is a hard
    // error, never a report that silently mixes two runs' numbers.
    baseline.documents += 1;
    let corpus = parse_corpus(CORPUS_JSONL).expect("parse fixture corpus");

    let err = render_markdown(&report, &baseline, &corpus)
        .expect_err("a report/baseline document-count mismatch must be rejected");
    assert!(
        err.contains("do not appear to describe the same run"),
        "unexpected error message: {err}"
    );
}

#[test]
fn a_documents_detail_length_mismatch_with_documents_is_rejected() {
    let report = parse_gate_report(
        r#"{"source":"real-specimens","mrz_class_sweep_arm":"off","providers":[
            {"provider_id":"mrz","documents":10,
             "capability":{"deterministic":true,"vision":false,"cost":"free"},
             "ocr_arms":{"texture":"on","order":"default","rotate":"default","skew":"default","chargrid":"off"},
             "documents_detail":[]}
        ]}"#,
    )
    .expect("parse a gate report whose documents_detail is shorter than documents claims");
    let baseline = parse_baseline(BASELINE_JSON).expect("parse fixture baseline");
    let corpus = parse_corpus(CORPUS_JSONL).expect("parse fixture corpus");

    let err = render_markdown(&report, &baseline, &corpus)
        .expect_err("documents=10 with an empty documents_detail must be rejected");
    assert!(
        err.contains("internally inconsistent"),
        "unexpected error message: {err}"
    );
}

#[test]
fn a_scored_or_hit_count_that_does_not_reconcile_with_the_baseline_is_rejected() {
    let report = parse_gate_report(REPORT_JSON).expect("parse fixture gate report");
    let mut baseline = parse_baseline(BASELINE_JSON).expect("parse fixture baseline");
    // The fixture's 10 documents_detail rows recompute to 7 scored / 5 hits,
    // matching the baseline by construction; break the baseline's own count
    // without touching `documents` (already covered by the mismatch test
    // above) to prove the *per-document* reconciliation is checked too, not
    // just the top-level document count.
    baseline.tier1_hits += 1;
    let corpus = parse_corpus(CORPUS_JSONL).expect("parse fixture corpus");

    let err = render_markdown(&report, &baseline, &corpus)
        .expect_err("a baseline hit count that disagrees with documents_detail must be rejected");
    assert!(
        err.contains("does not reconcile to the baseline"),
        "unexpected error message: {err}"
    );
}

#[test]
fn the_issuer_cut_states_not_applicable_when_no_document_carries_an_asset_id() {
    // A synthetic-corpus report: every document lacks `asset_id` by
    // construction (`OutcomeRow`'s doc: never set for the synthetic corpus).
    let report = parse_gate_report(
        r#"{"source":"synthetic-corpus","mrz_class_sweep_arm":"off","providers":[
            {"provider_id":"mrz","documents":1,
             "capability":{"deterministic":true,"vision":false,"cost":"free"},
             "ocr_arms":{"texture":"on","order":"default","rotate":"default","skew":"default","chargrid":"off"},
             "documents_detail":[{"name":"seed-0","mrz_found":true,"mrz_format":"TD3","read_ok":true,"mrz_checksums_valid":true,"assertions_total":0,"assertions_unsupported":0,"unsupported_fields":[],"ocr_ms":1,"retry_budget_hit":false,"retry_stop":null}]}
        ]}"#,
    )
    .expect("parse a one-document synthetic-corpus gate report");
    let baseline_json = r#"{
        "note": "n", "measured_on_ci_sha": "a", "measured_date": "2026-01-01",
        "samples_data_sha": "b", "documents": 1, "scored": 1, "tier1_hits": 1,
        "tolerance": 0, "by_miss_kind": {}
    }"#;
    let baseline = parse_baseline(baseline_json).expect("parse a minimal one-hit baseline");
    let corpus = parse_corpus(CORPUS_JSONL).expect("parse fixture corpus");

    let markdown =
        render_markdown(&report, &baseline, &corpus).expect("render a synthetic-corpus report");
    let issuer_section = markdown
        .split("### By issuing state")
        .nth(1)
        .expect("the By issuing state heading is always present")
        .split("\n## ")
        .next()
        .expect("split always yields at least one piece");
    assert!(
        issuer_section.contains("Not applicable"),
        "a run with no asset_id anywhere must say the issuer cut is not applicable, not print an \
         empty or fabricated table:\n{markdown}"
    );
}

#[test]
fn a_gate_report_with_no_mrz_provider_is_rejected() {
    let report = parse_gate_report(
        r#"{"source":"real-specimens","mrz_class_sweep_arm":"off","providers":[]}"#,
    )
    .expect("parse a gate report with an empty providers list");
    let baseline = parse_baseline(BASELINE_JSON).expect("parse fixture baseline");
    let corpus = parse_corpus(CORPUS_JSONL).expect("parse fixture corpus");

    let err = render_markdown(&report, &baseline, &corpus)
        .expect_err("a gate report with no `mrz` provider row must be rejected");
    assert!(
        err.contains("no `mrz` provider row"),
        "unexpected error message: {err}"
    );
}

#[test]
fn an_unrecognised_outcome_bucket_is_still_printed_not_dropped() {
    let report = parse_gate_report(REPORT_JSON).expect("parse fixture gate report");
    let mut baseline = parse_baseline(BASELINE_JSON).expect("parse fixture baseline");
    baseline
        .by_miss_kind
        .insert("a_future_miss_kind".to_string(), 3);
    let corpus = parse_corpus(CORPUS_JSONL).expect("parse fixture corpus");

    let markdown = render_markdown(&report, &baseline, &corpus)
        .expect("a report/baseline pair with an unrecognised bucket still renders");
    assert!(
        markdown.contains("`a_future_miss_kind` | 3 |"),
        "an unrecognised outcome bucket must still appear in the outcome table, not be dropped \
         silently:\n{markdown}"
    );
}

#[test]
fn a_baseline_with_no_strict_names_prints_not_measured_never_a_fabricated_zero() {
    let report = parse_gate_report(REPORT_JSON).expect("parse fixture gate report");
    let mut baseline = parse_baseline(BASELINE_JSON).expect("parse fixture baseline");
    baseline.strict_names = None;
    let corpus = parse_corpus(CORPUS_JSONL).expect("parse fixture corpus");

    let markdown =
        render_markdown(&report, &baseline, &corpus).expect("render with no strict_names");
    let after_heading = markdown
        .split("## Strict names")
        .nth(1)
        .expect("the Strict names heading is always present");
    let strict_names_section = after_heading
        .split("\n## ")
        .next()
        .expect("split always yields at least one piece");
    assert!(
        strict_names_section.contains("Not measured in this baseline"),
        "an absent `strict_names` must say so, not print a rate:\n{markdown}"
    );
    assert!(
        !strict_names_section.contains('%'),
        "an absent strict-name measurement must never render as a fabricated rate:\n{markdown}"
    );
}

/// End-to-end smoke test of the CLI's own argument parsing and file I/O
/// (the one thing the golden test above does not exercise, since it calls
/// `render_markdown` directly): runs the built `bench-report` binary over the
/// same fixtures and checks its stdout matches the golden file too.
#[test]
fn cli_binary_reproduces_the_golden_file_via_stdout() {
    let exe = env!("CARGO_BIN_EXE_bench-report");
    let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/bench_report");
    let output = std::process::Command::new(exe)
        .arg("--report")
        .arg(format!("{fixtures}/gate-report.json"))
        .arg("--baseline")
        .arg(format!("{fixtures}/baseline.json"))
        .arg("--corpus")
        .arg(format!("{fixtures}/corpus.jsonl"))
        .output()
        .expect("run the bench-report binary");
    assert!(
        output.status.success(),
        "bench-report exited non-zero: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert_eq!(stdout, GOLDEN_MD);
}
