//! Re-scores a finished parity run against the *current* normalizers, without
//! running the model again.
//!
//! # Why this can work at all
//!
//! `crates/synthpass-llm/tests/parity.rs` normalizes the model's answer before
//! comparing it, and `synthpass_core::normalize` passes anything it cannot
//! resolve through **verbatim**. So every mismatch line in a parity log carries
//! the model's own string, unmodified — and a change confined to the normalizers
//! can be scored by replaying those strings. The model is deterministic and its
//! answers are fixed; only the code under them moved.
//!
//! That turns a 35-minute measurement into a sub-second one for the specific
//! class of change that touches normalization only. It is not a replacement for
//! the real run: it is how you find out whether the real run is worth starting.
//!
//! **It has been checked against reality once.** Before PR #206 this technique
//! predicted 14 recovered date fields; the full run produced exactly 14, split
//! +7/+7 across the two fixture sets.
//!
//! # What it cannot tell you
//!
//! - **Nothing about a prompt change.** Different prompt, different answers, and
//!   every string here would be stale. Any change to `synthpass-llm`'s prompt,
//!   grammar, or sampling invalidates a replay completely.
//! - **Nothing about a field the normalizer already resolved wrongly.** The log
//!   records the *normalized* value, so if a normalizer converted a raw answer
//!   into the wrong thing, the raw answer is gone and cannot be re-scored. Only
//!   values that passed through unresolved are faithfully replayable, which is
//!   exactly the population a vocabulary change addresses.
//!
//! # The accept rule
//!
//! Mirrors the texture A/B (`knowledge/benchmarks/texture-suppression-ab-2026-09-03.md`):
//! a change ships only if it flips **at least one miss to a hit and zero hits to
//! misses**. A single hit→miss is a blocker and exits non-zero, because an
//! aggregate that nets a gain against an unnoticed loss is the exact failure
//! this project has already been burned by.
//!
//! A candidate that flips *nothing* is rejected too — not as a blocker, but it
//! has no evidence behind it and does not belong in a table that claims to be
//! measured. That property is what makes this harness safe to point at a
//! generated word list: an entry no document exercises cannot get in.
//!
//! # Usage
//!
//! ```powershell
//! cargo run -p synthpass-bench --release --example vocab_replay -- artifacts/parity4-both-arms.log
//! ```
//!
//! Reads the log named on the command line (a run captured with `--nocapture`),
//! re-scores every comparison it contains, and prints the flips in both
//! directions. Exits 1 if any hit became a miss. Exits 2 (before parsing
//! anything) if the log has no `log-format:` marker in its
//! `normalizer vocabulary: …` header line, or one this parser doesn't
//! understand — see [`check_log_format`] — since `comparison`/`optional`
//! below have no shared struct with `parity.rs` and would otherwise
//! mis-parse a log whose line shape had changed rather than failing loudly.

use std::collections::BTreeMap;

use synthpass_core::normalize;

/// One `field expected=… actual=… OK|MISMATCH` line from a parity log.
struct Comparison {
    document: String,
    field: String,
    expected: Option<String>,
    /// As the log recorded it — already normalized by the run that produced it.
    actual: Option<String>,
    was_hit: bool,
}

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: vocab_replay <parity log>");
        std::process::exit(2);
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            std::process::exit(2);
        }
    };

    if let Err(e) = check_log_format(&text) {
        eprintln!("{path}: {e}");
        std::process::exit(2);
    }

    let comparisons = parse(&text);
    if comparisons.is_empty() {
        eprintln!("no comparison lines found in {path} — was the run captured with --nocapture?");
        std::process::exit(2);
    }

    let mut gained = Vec::new();
    let mut lost = Vec::new();
    let mut by_field: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let (mut hits_before, mut hits_after) = (0usize, 0usize);

    for c in &comparisons {
        let now_hit = renormalized(&c.field, c.actual.as_deref())
            == renormalized(&c.field, c.expected.as_deref());
        hits_before += usize::from(c.was_hit);
        hits_after += usize::from(now_hit);
        match (c.was_hit, now_hit) {
            (false, true) => {
                by_field.entry(&c.field).or_default().0 += 1;
                gained.push(c);
            }
            (true, false) => {
                by_field.entry(&c.field).or_default().1 += 1;
                lost.push(c);
            }
            _ => {}
        }
    }

    let total = comparisons.len();
    println!("replayed {total} comparison(s) from {path}\n");
    println!(
        "before: {hits_before}/{total} ({:.1}%)",
        pct(hits_before, total)
    );
    println!(
        "after:  {hits_after}/{total} ({:.1}%)   {:+}",
        pct(hits_after, total),
        hits_after as i64 - hits_before as i64
    );

    if !by_field.is_empty() {
        println!("\nper field (gained / lost):");
        for (field, (up, down)) in &by_field {
            println!("  {field:16} +{up} / -{down}");
        }
    }

    report("RECOVERED (miss -> hit)", &gained);
    report("REGRESSED (hit -> miss)  ** BLOCKER **", &lost);

    if !lost.is_empty() {
        println!(
            "\nBlocked: {} hit(s) became misses. The accept rule is >=1 gain AND 0 losses \
             -- a net gain that hides a loss is the failure this rule exists to catch.",
            lost.len()
        );
        std::process::exit(1);
    }
    if gained.is_empty() {
        println!(
            "\nNo change. Nothing in this corpus exercises the edit, so nothing here \
             supports shipping it."
        );
    }
}

/// Re-run the field's normalizer over a value the log already normalized once.
///
/// Safe because every normalizer in `synthpass_core::normalize` is documented
/// and tested as idempotent: re-normalizing an already-normalized value is a
/// no-op, so a value the *old* code resolved is unchanged here and only the
/// ones it passed through can move.
fn renormalized(field: &str, value: Option<&str>) -> Option<String> {
    let v = value?;
    Some(match field {
        "issuing_country" => normalize::issuing_country(v),
        "nationality" => normalize::nationality(v),
        "date_of_birth" => normalize::date_of_birth(v),
        "date_of_expiry" => normalize::date_of_expiry(v),
        "given_names" => normalize::given_names(v),
        "sex" => normalize::sex(v),
        "document_type" => normalize::document_type(v),
        // `document_number` and `surname` have no normalizer, by design — see
        // `normalize::extraction`'s doc comment.
        _ => v.to_string(),
    })
}

fn report(heading: &str, rows: &[&Comparison]) {
    if rows.is_empty() {
        return;
    }
    println!("\n{heading}: {}", rows.len());
    for c in rows {
        println!(
            "  {:38} {:16} expected={:?} actual={:?}",
            elide(&c.document, 38),
            c.field,
            c.expected.as_deref().unwrap_or("<none>"),
            c.actual.as_deref().unwrap_or("<none>")
        );
    }
}

fn pct(n: usize, d: usize) -> f64 {
    if d == 0 {
        0.0
    } else {
        n as f64 * 100.0 / d as f64
    }
}

fn elide(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
}

/// The `log-format` value this parser understands — must match
/// `crates/synthpass-llm/tests/parity.rs`'s `PARITY_LOG_FORMAT_VERSION`. That
/// constant's doc comment is the other half of this contract: bump both
/// together whenever the `field expected=… actual=… OK|MISMATCH` line shape
/// changes, since `comparison`/`optional` below parse that shape with
/// hand-written string splitting and no shared struct with `parity.rs`.
const EXPECTED_LOG_FORMAT_VERSION: u32 = 1;

/// Find and validate the `log-format: N` marker in `parity.rs`'s run header
/// (`"normalizer vocabulary: … log-format: … prompt: …"`), so a log from
/// before that marker existed — or one printed by a version of `parity.rs`
/// whose line shape this parser no longer matches — is refused up front
/// rather than silently mis-parsed or silently dropped to zero comparisons.
fn check_log_format(text: &str) -> Result<(), String> {
    let line = text
        .lines()
        .find(|l| l.starts_with("normalizer vocabulary:"))
        .ok_or(
            "no 'normalizer vocabulary:' header line found in this log — it predates the \
             log-format guard and cannot be trusted to parse correctly",
        )?;
    let version: u32 = line
        .split("log-format:")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| format!("header line has no parseable 'log-format: N' marker: {line:?}"))?;
    if version != EXPECTED_LOG_FORMAT_VERSION {
        return Err(format!(
            "log format {version} does not match the {EXPECTED_LOG_FORMAT_VERSION} this parser \
             understands — the line shape this tool parses probably changed; update this file's \
             `optional`/`comparison` alongside it"
        ));
    }
    Ok(())
}

/// Pull every comparison line out of a `--nocapture` parity log, attributing
/// each to the document header (`--- [n/m] <stem> (reviewed|derived) ... ---`)
/// that most recently preceded it.
fn parse(text: &str) -> Vec<Comparison> {
    let mut out = Vec::new();
    let mut document = String::from("<unknown>");
    for line in text.lines() {
        if let Some(stem) = document_header(line) {
            document = stem;
            continue;
        }
        let Some(c) = comparison(line, &document) else {
            continue;
        };
        out.push(c);
    }
    out
}

fn document_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("--- [")?;
    let rest = rest.split_once("] ")?.1;
    Some(rest.split_whitespace().next()?.to_string())
}

fn comparison(line: &str, document: &str) -> Option<Comparison> {
    // "  <field>  expected=<opt> actual=<opt> OK|MISMATCH"
    let body = line.strip_prefix("  ")?;
    if body.starts_with(' ') {
        return None;
    }
    let (field, rest) = body.split_once(char::is_whitespace)?;

    // Parsed right-to-left. A recorded value can contain anything the document
    // printed, including `")` and the word `actual=`, so the fixed points are
    // the trailing verdict and the *last* ` actual=` separator — not the first
    // delimiter that happens to match inside a value.
    let (rest, was_hit) = match rest.trim_end() {
        r if r.ends_with(" OK") => (r.trim_end_matches(" OK"), true),
        r if r.ends_with(" MISMATCH") => (r.trim_end_matches(" MISMATCH"), false),
        _ => return None,
    };
    let (expected, actual) = rest.trim_start().rsplit_once(" actual=")?;
    let expected = optional(expected.trim().strip_prefix("expected=")?)?;
    let actual = optional(actual.trim())?;
    Some(Comparison {
        document: document.to_string(),
        field: field.to_string(),
        expected,
        actual,
        was_hit,
    })
}

/// Parse a whole `None` or `Some("…")` field, already isolated by
/// [`comparison`].
///
/// Anchored at both ends rather than scanning for a delimiter: the caller has
/// established where this value starts and stops, so the quotes are stripped
/// and whatever sits between them is the value — including a `")` the document
/// itself printed. `Option<Option<_>>` because "this is not a field" and "this
/// field is absent" are different answers.
///
/// `parity.rs` prints these values with `{:?}` (Debug), which escapes an
/// embedded `"` as `\"` — so the text between the stripped delimiters is not
/// the raw value yet, and [`unescape_debug`] reverses exactly that.
fn optional(s: &str) -> Option<Option<String>> {
    if s == "None" {
        return Some(None);
    }
    let inner = s.strip_prefix("Some(\"")?.strip_suffix("\")")?;
    Some(Some(unescape_debug(inner)))
}

/// Reverse Rust's `{:?}` Debug-string escaping: `\"` -> `"`, `\\` -> `\`.
///
/// [`optional`] isolates exactly the slice between a `Some("` / `")`
/// wrapper's delimiters, so this is the only escaping that slice can contain
/// — `parity.rs` prints single OCR/LLM field values, never a string with a
/// newline or tab a Debug-escaped log would need `\n`/`\t` to represent.
/// Without this, a genuine hit on a document whose value contains a quote
/// (e.g. `O" NEIL`) replays with a stray backslash still in it, comparing
/// unequal to the unescaped original and misreporting a hit as a new miss.
fn unescape_debug(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some(escaped @ ('"' | '\\')) => out.push(escaped),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "\
--- [1/72] Canada_Passport_Specimen_P0_CAN_2020_mrz (reviewed) 24.1s, 0m elapsed ---
  nationality      expected=Some(\"CAN\") actual=Some(\"CANADIAN/CANADIENNE\") MISMATCH
  sex              expected=Some(\"F\") actual=None MISMATCH
  document_number  expected=Some(\"X1\") actual=Some(\"X1\") OK
";

    #[test]
    fn parses_fields_documents_and_verdicts() {
        let rows = parse(LOG);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].document, "Canada_Passport_Specimen_P0_CAN_2020_mrz");
        assert_eq!(rows[0].field, "nationality");
        assert_eq!(rows[0].actual.as_deref(), Some("CANADIAN/CANADIENNE"));
        assert!(!rows[0].was_hit);
        assert_eq!(rows[1].actual, None);
        assert!(rows[2].was_hit);
    }

    #[test]
    fn a_value_containing_a_debug_escaped_quote_is_unescaped() {
        // Real OCR/LLM output can contain a literal `"`. `parity.rs` prints
        // with `{:?}` (Debug), which escapes it as `\"` — this is the shape a
        // real log line actually has; a raw, unescaped embedded quote (what
        // this test used before) is not something `{:?}` ever produces.
        let line = r#"  surname          expected=Some("A") actual=Some("O\" NEIL") MISMATCH"#;
        let c = comparison(line, "doc").expect("parses");
        assert_eq!(c.actual.as_deref(), Some("O\" NEIL"));
    }

    #[test]
    fn unescape_debug_reverses_quote_and_backslash_escaping() {
        assert_eq!(unescape_debug(r#"O\" NEIL"#), "O\" NEIL");
        assert_eq!(unescape_debug(r"back\\slash"), r"back\slash");
        assert_eq!(unescape_debug("plain"), "plain");
    }

    #[test]
    fn check_log_format_accepts_the_current_version() {
        let text = "normalizer vocabulary: abc123   log-format: 1   prompt: x v1   fixtures: 72\n";
        assert!(check_log_format(text).is_ok());
    }

    #[test]
    fn check_log_format_rejects_a_log_that_predates_the_marker() {
        assert!(check_log_format("no header here at all\n").is_err());
    }

    #[test]
    fn check_log_format_rejects_a_mismatched_version() {
        let text = "normalizer vocabulary: abc123   log-format: 99   prompt: x v1   fixtures: 72\n";
        assert!(check_log_format(text).is_err());
    }

    #[test]
    fn ignores_progress_and_summary_lines() {
        let noise = "reviewed fixtures (all nine prompt fields): 18 document(s), 85/162 fields";
        assert!(comparison(noise, "doc").is_none());
        assert!(document_header(noise).is_none());
    }

    #[test]
    fn renormalizing_an_already_normalized_value_is_a_no_op() {
        // The property that makes replay valid at all.
        for (field, value) in [
            ("nationality", "HRV"),
            ("date_of_birth", "1974-08-12"),
            ("sex", "F"),
            ("document_type", "P"),
        ] {
            assert_eq!(
                renormalized(field, Some(value)),
                Some(value.to_string()),
                "{field} must be idempotent for replay to mean anything"
            );
        }
    }

    #[test]
    fn a_field_with_no_normalizer_replays_unchanged() {
        assert_eq!(
            renormalized("document_number", Some("L898902C3")),
            Some("L898902C3".to_string())
        );
    }
}
