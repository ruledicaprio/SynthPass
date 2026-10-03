//! The per-seed ledger of a synthetic run and its report-only diff (issue #557,
//! Phase 2).
//!
//! `synthpass-bench --ledger <path>` writes one JSONL row per seed;
//! `--diff-ledger <committed.jsonl>` prints how this run's rows differ from a
//! committed copy. The M4 gate's report already carries per-document fields,
//! but nothing compared them from one run to the next: a run could move a
//! seed's check states or its retry path and still print the same hit rate.
//! The rows here are text-free (field *names*, enumerated values, booleans and
//! a timing), so the diff is safe to print to a public step summary.
//!
//! **The diff never fails anything.** Every function that diffs returns lines
//! to print, never a `Result` that a caller could turn into an exit code; a
//! missing, unreadable or malformed committed ledger is one more line. The
//! gate rules (`--min-hit-rate`, `--max-prefix-wrong-accepts`) are decided
//! elsewhere and this module cannot change them.
//!
//! The committed ledger is written by CI only, never by hand and never from a
//! local run: OCR floats can round differently across machines, so a ledger
//! from another machine could diff against CI on noise, and ADR-0027 decision
//! 4 says a CI arm is never compared with a local arm. See
//! `knowledge/benchmarks/README.md`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// One seed's row. Every key is always written (`null` when absent), in this
/// declaration order, which is #557's field list; nothing else is added. No
/// expected or read value and no OCR text: `wrong_fields` names fields, it
/// never quotes them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerRow {
    /// The generated MRZ format label (`--document-type`), as
    /// `DocumentType::as_str` writes it: for example `TD3`.
    pub format: String,
    pub seed: u64,
    pub profile: String,
    pub hit: bool,
    pub miss_kind: Option<String>,
    pub wrong_accept: bool,
    pub prefix_wrong_accept: bool,
    /// Names of the scored fields that differ from truth. [`ledger_text`]
    /// writes them sorted.
    pub wrong_fields: Vec<String>,
    /// The state of each check digit (`true` verified, `false` failed, `null`
    /// not printed in this layout), keyed by field name; `null` when no MRZ
    /// parsed. A `BTreeMap`, so the keys are always sorted and the JSON is
    /// compact.
    pub check_states: Option<BTreeMap<String, Option<bool>>>,
    pub names_exact: bool,
    pub name_error: Option<String>,
    pub line1_flagged: bool,
    pub retry_stop: Option<String>,
    pub retry_variant_id: Option<String>,
    pub retry_damaged_recovery: Option<bool>,
    pub tier1_damaged_recovery: Option<bool>,
    /// Wall-clock, so it moves on every run: reported as a timing line, never
    /// as a deterministic change.
    pub elapsed_ms: u128,
}

impl LedgerRow {
    /// The row as the ledger file writes it: `wrong_fields` sorted. One definition, used by
    /// [`ledger_text`] and by the per-document archive, whose record carries the same row.
    #[must_use]
    pub fn normalised(mut self) -> Self {
        self.wrong_fields.sort_unstable();
        self
    }
}

/// The rows as the file's exact text: one compact JSON object per line, sorted
/// by (`format`, `seed`), each row's `wrong_fields` sorted, ending in a newline
/// (an empty slice is an empty string). The `Err` is `serde_json`'s message. A
/// `LedgerRow` holds only strings, booleans, integers and a map of those, so it
/// is not expected, but it is returned rather than skipped: a silently shorter
/// ledger would read as seeds that vanished.
pub fn ledger_text(rows: &[LedgerRow]) -> Result<String, String> {
    let mut sorted: Vec<LedgerRow> = rows.iter().cloned().map(LedgerRow::normalised).collect();
    sorted.sort_by(|a, b| (&a.format, a.seed).cmp(&(&b.format, b.seed)));
    let mut text = String::new();
    for row in &sorted {
        let line = serde_json::to_string(row)
            .map_err(|e| format!("ledger row for seed {} does not serialize: {e}", row.seed))?;
        text.push_str(&line);
        text.push('\n');
    }
    Ok(text)
}

/// Writes [`ledger_text`] to `path`, creating its parent directories.
pub fn write_ledger(path: &Path, rows: &[LedgerRow]) -> std::io::Result<()> {
    let text = ledger_text(rows).map_err(std::io::Error::other)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, text)
}

/// The inverse of [`ledger_text`]: one row per non-empty line. A malformed row
/// is an error the caller reports; it never panics.
pub fn parse_ledger(text: &str) -> Result<Vec<LedgerRow>, String> {
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).map_err(|e| format!("malformed ledger row: {e}")))
        .collect()
}

/// Diffs this run's rows against the committed file at `path` and returns the
/// lines to print. No file, an unreadable file and a malformed file are each
/// one line: **there is no failure to return**.
pub fn diff_against_file(path: &Path, actual: &[LedgerRow]) -> Vec<String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return vec![format!(
                "synthetic ledger diff: no committed ledger at {}; nothing to compare against \
                 (bootstrap: CI installs the first one, it is never written by hand)",
                path.display()
            )];
        }
        Err(e) => {
            return vec![format!(
                "synthetic ledger diff: could not read the committed ledger at {}: {e}; \
                 skipping the diff",
                path.display()
            )];
        }
    };
    match parse_ledger(&text) {
        Ok(committed) => diff_ledgers(&committed, actual),
        Err(e) => vec![format!(
            "synthetic ledger diff: could not parse the committed ledger at {}: {e}; \
             skipping the diff",
            path.display()
        )],
    }
}

/// Most seed lines any one group prints; the per-field totals above them are
/// never capped.
const SEED_LINE_CAP: usize = 20;

/// The fields [`diff_ledgers`] compares, in [`LedgerRow`] schema (declaration)
/// order, which is the order its totals print in. `format` and `seed` are the
/// join key and `elapsed_ms` has its own timing line, so none of those is here.
const DIFFED_FIELDS: &[&str] = &[
    "profile",
    "hit",
    "miss_kind",
    "wrong_accept",
    "prefix_wrong_accept",
    "wrong_fields",
    "check_states",
    "names_exact",
    "name_error",
    "line1_flagged",
    "retry_stop",
    "retry_variant_id",
    "retry_damaged_recovery",
    "tier1_damaged_recovery",
];

/// One field that differs on one seed, already rendered as `field old -> new`.
/// Built only from enumerated values, field names and booleans: the step
/// summary this reaches is public.
struct FieldChange {
    field: &'static str,
    text: String,
}

fn render_str(value: Option<&str>) -> String {
    value.unwrap_or("null").to_string()
}

fn render_bool(value: Option<bool>) -> String {
    value.map_or_else(|| "null".to_string(), |b| b.to_string())
}

fn render_names(names: &[String]) -> String {
    format!("[{}]", names.join(","))
}

fn render_states(states: &Option<BTreeMap<String, Option<bool>>>) -> String {
    match states {
        Some(map) => serde_json::to_string(map).unwrap_or_else(|_| "unrenderable".to_string()),
        None => "null".to_string(),
    }
}

fn push_transition(changes: &mut Vec<FieldChange>, field: &'static str, old: String, new: String) {
    if old != new {
        changes.push(FieldChange {
            field,
            text: format!("{field} {old} -> {new}"),
        });
    }
}

/// Every [`DIFFED_FIELDS`] entry that differs between two rows of one seed, in
/// schema order.
fn field_changes(old: &LedgerRow, new: &LedgerRow) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    push_transition(
        &mut changes,
        "profile",
        old.profile.clone(),
        new.profile.clone(),
    );
    push_transition(
        &mut changes,
        "hit",
        old.hit.to_string(),
        new.hit.to_string(),
    );
    push_transition(
        &mut changes,
        "miss_kind",
        render_str(old.miss_kind.as_deref()),
        render_str(new.miss_kind.as_deref()),
    );
    push_transition(
        &mut changes,
        "wrong_accept",
        old.wrong_accept.to_string(),
        new.wrong_accept.to_string(),
    );
    push_transition(
        &mut changes,
        "prefix_wrong_accept",
        old.prefix_wrong_accept.to_string(),
        new.prefix_wrong_accept.to_string(),
    );
    push_transition(
        &mut changes,
        "wrong_fields",
        render_names(&sorted_names(&old.wrong_fields)),
        render_names(&sorted_names(&new.wrong_fields)),
    );
    push_transition(
        &mut changes,
        "check_states",
        render_states(&old.check_states),
        render_states(&new.check_states),
    );
    push_transition(
        &mut changes,
        "names_exact",
        old.names_exact.to_string(),
        new.names_exact.to_string(),
    );
    push_transition(
        &mut changes,
        "name_error",
        render_str(old.name_error.as_deref()),
        render_str(new.name_error.as_deref()),
    );
    push_transition(
        &mut changes,
        "line1_flagged",
        old.line1_flagged.to_string(),
        new.line1_flagged.to_string(),
    );
    push_transition(
        &mut changes,
        "retry_stop",
        render_str(old.retry_stop.as_deref()),
        render_str(new.retry_stop.as_deref()),
    );
    push_transition(
        &mut changes,
        "retry_variant_id",
        render_str(old.retry_variant_id.as_deref()),
        render_str(new.retry_variant_id.as_deref()),
    );
    push_transition(
        &mut changes,
        "retry_damaged_recovery",
        render_bool(old.retry_damaged_recovery),
        render_bool(new.retry_damaged_recovery),
    );
    push_transition(
        &mut changes,
        "tier1_damaged_recovery",
        render_bool(old.tier1_damaged_recovery),
        render_bool(new.tier1_damaged_recovery),
    );
    changes
}

/// The names in a stable order, so a ledger written by another writer (or by
/// hand) that left them unsorted does not diff against a sorted one.
fn sorted_names(names: &[String]) -> Vec<String> {
    let mut sorted = names.to_vec();
    sorted.sort_unstable();
    sorted
}

/// A seed that stopped on the OCR retry-pass time budget on either side. Which
/// passes finished depends on runner speed, so nothing about such a seed is a
/// stable signal.
fn is_budget_limited(old: &LedgerRow, new: &LedgerRow) -> bool {
    old.retry_stop.as_deref() == Some("budget") || new.retry_stop.as_deref() == Some("budget")
}

/// `field count, field count, …` in [`DIFFED_FIELDS`] order, zero counts left
/// out. Counts are seeds and are never capped.
fn field_totals(seeds: &[(u64, Vec<FieldChange>)]) -> String {
    DIFFED_FIELDS
        .iter()
        .filter_map(|field| {
            let count = seeds
                .iter()
                .filter(|(_, changes)| changes.iter().any(|c| c.field == *field))
                .count();
            (count > 0).then(|| format!("{field} {count}"))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `  seed N: change; change` per seed (at most [`SEED_LINE_CAP`]), then
/// `  ... and N more`.
fn push_seed_lines(lines: &mut Vec<String>, seeds: &[(u64, Vec<FieldChange>)]) {
    for (seed, changes) in seeds.iter().take(SEED_LINE_CAP) {
        let texts: Vec<&str> = changes.iter().map(|c| c.text.as_str()).collect();
        lines.push(format!("  seed {seed}: {}", texts.join("; ")));
    }
    if seeds.len() > SEED_LINE_CAP {
        lines.push(format!("  ... and {} more", seeds.len() - SEED_LINE_CAP));
    }
}

/// Median of `values`, sorted in place; the integer mean of the two middle
/// values (rounded down) when the count is even. `0` for an empty slice.
fn median(values: &mut [u128]) -> u128 {
    values.sort_unstable();
    let n = values.len();
    match n {
        0 => 0,
        _ if n % 2 == 1 => values[n / 2],
        _ => (values[n / 2 - 1] + values[n / 2]) / 2,
    }
}

/// The report-only diff of two ledgers, joined on (`format`, `seed`), as lines
/// to print, in this order:
///
/// 1. One line: how many seeds changed a deterministic field, and per-field
///    totals in schema order (never capped).
/// 2. Up to 20 `seed N: field old -> new; ...` lines, then
///    `... and N more`.
/// 3. One timing line: `elapsed_ms` differs on N of M seeds, the median
///    |delta| and both totals. Always printed.
/// 4. Only when there is one: the *budget-limited* group, every change on a
///    seed whose `retry_stop` is `budget` on either side, in the same shape as
///    1 and 2. Those seeds are timing-sensitive, so they are not in the count
///    on line 1.
/// 5. One line with the seeds present on one side only, as counts. Such seeds
///    are never compared.
///
/// Every field but `elapsed_ms` is deterministic. The return type has no
/// failure variant: a diff is informational whatever it finds.
pub fn diff_ledgers(committed: &[LedgerRow], actual: &[LedgerRow]) -> Vec<String> {
    type Key<'a> = (&'a str, u64);
    let committed_by_key: BTreeMap<Key<'_>, &LedgerRow> = committed
        .iter()
        .map(|r| ((r.format.as_str(), r.seed), r))
        .collect();
    let actual_by_key: BTreeMap<Key<'_>, &LedgerRow> = actual
        .iter()
        .map(|r| ((r.format.as_str(), r.seed), r))
        .collect();

    let mut deterministic: Vec<(u64, Vec<FieldChange>)> = Vec::new();
    let mut budget_limited: Vec<(u64, Vec<FieldChange>)> = Vec::new();
    let mut elapsed_deltas: Vec<u128> = Vec::new();
    let mut common = 0usize;
    let mut only_committed = 0usize;
    let mut elapsed_old_total = 0u128;
    let mut elapsed_new_total = 0u128;
    for (key, old) in &committed_by_key {
        let Some(new) = actual_by_key.get(key) else {
            only_committed += 1;
            continue;
        };
        common += 1;
        elapsed_old_total += old.elapsed_ms;
        elapsed_new_total += new.elapsed_ms;
        if old.elapsed_ms != new.elapsed_ms {
            elapsed_deltas.push(old.elapsed_ms.abs_diff(new.elapsed_ms));
        }
        let changes = field_changes(old, new);
        if changes.is_empty() {
            continue;
        }
        if is_budget_limited(old, new) {
            budget_limited.push((key.1, changes));
        } else {
            deterministic.push((key.1, changes));
        }
    }
    let only_actual = actual_by_key
        .keys()
        .filter(|k| !committed_by_key.contains_key(*k))
        .count();

    let mut lines = Vec::new();
    let mut head = format!(
        "synthetic ledger diff vs committed (report-only): {} seed(s) changed",
        deterministic.len()
    );
    if !deterministic.is_empty() {
        head.push_str("; ");
        head.push_str(&field_totals(&deterministic));
    }
    lines.push(head);
    push_seed_lines(&mut lines, &deterministic);
    lines.push(format!(
        "timing (report-only): elapsed_ms differs on {} of {common} seed(s), median |delta| \
         {} ms, total {elapsed_old_total} ms -> {elapsed_new_total} ms",
        elapsed_deltas.len(),
        median(&mut elapsed_deltas),
    ));
    if !budget_limited.is_empty() {
        lines.push(format!(
            "budget-limited seed(s) (report-only; every change on them is timing-sensitive): \
             {} seed(s); {}",
            budget_limited.len(),
            field_totals(&budget_limited),
        ));
        push_seed_lines(&mut lines, &budget_limited);
    }
    lines.push(format!(
        "seeds only in the committed ledger: {only_committed}, only in this run: {only_actual}"
    ));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(seed: u64) -> LedgerRow {
        LedgerRow {
            format: "td3".to_string(),
            seed,
            profile: "clean".to_string(),
            hit: true,
            miss_kind: None,
            wrong_accept: false,
            prefix_wrong_accept: false,
            wrong_fields: Vec::new(),
            check_states: None,
            names_exact: true,
            name_error: None,
            line1_flagged: false,
            retry_stop: Some("general_valid".to_string()),
            retry_variant_id: Some("general".to_string()),
            retry_damaged_recovery: Some(false),
            tier1_damaged_recovery: Some(false),
            elapsed_ms: 100,
        }
    }

    fn row_with(seed: u64, edit: impl FnOnce(&mut LedgerRow)) -> LedgerRow {
        let mut r = row(seed);
        edit(&mut r);
        r
    }

    fn scratch_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "synthpass-bench-synthetic-ledger-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    #[test]
    fn a_row_has_every_key_in_the_contract_order_with_nulls_for_the_absent() {
        let miss = row_with(7, |r| {
            r.hit = false;
            r.names_exact = false;
            r.retry_stop = None;
            r.retry_variant_id = None;
            r.retry_damaged_recovery = None;
            r.tier1_damaged_recovery = None;
            r.elapsed_ms = 42;
        });
        assert_eq!(
            ledger_text(&[miss]).expect("serializes"),
            "{\"format\":\"td3\",\"seed\":7,\"profile\":\"clean\",\"hit\":false,\
             \"miss_kind\":null,\"wrong_accept\":false,\"prefix_wrong_accept\":false,\
             \"wrong_fields\":[],\"check_states\":null,\"names_exact\":false,\
             \"name_error\":null,\"line1_flagged\":false,\"retry_stop\":null,\
             \"retry_variant_id\":null,\"retry_damaged_recovery\":null,\
             \"tier1_damaged_recovery\":null,\"elapsed_ms\":42}\n"
        );
    }

    #[test]
    fn wrong_fields_are_written_sorted_and_rows_by_seed() {
        let rows = vec![
            row_with(9, |r| {
                r.wrong_fields = vec!["surname".into(), "document_type".into()]
            }),
            row_with(2, |r| r.wrong_fields = vec!["nationality".into()]),
        ];
        let text = ledger_text(&rows).expect("serializes");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"seed\":2"), "{}", lines[0]);
        assert!(
            lines[1].contains("\"wrong_fields\":[\"document_type\",\"surname\"]"),
            "{}",
            lines[1]
        );
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn check_states_are_compact_json_with_sorted_keys() {
        let states: BTreeMap<String, Option<bool>> = [
            ("personal_number".to_string(), None),
            ("document_number".to_string(), Some(true)),
            ("composite".to_string(), Some(false)),
        ]
        .into_iter()
        .collect();
        let text =
            ledger_text(&[row_with(1, |r| r.check_states = Some(states))]).expect("serializes");
        assert!(
            text.contains(
                "\"check_states\":{\"composite\":false,\"document_number\":true,\
                 \"personal_number\":null}"
            ),
            "{text}"
        );
    }

    #[test]
    fn the_text_round_trips_and_an_empty_ledger_is_an_empty_string() {
        let rows = vec![
            row(0),
            row_with(1, |r| {
                r.hit = false;
                r.miss_kind = Some("checksum_failed".to_string());
                r.wrong_fields = vec!["surname".to_string()];
                r.check_states = Some(
                    [("composite".to_string(), Some(false))]
                        .into_iter()
                        .collect(),
                );
            }),
        ];
        let text = ledger_text(&rows).expect("serializes");
        assert_eq!(parse_ledger(&text).expect("parses"), rows);
        assert_eq!(ledger_text(&[]).expect("serializes"), "");
        assert_eq!(parse_ledger("").expect("empty parses"), Vec::new());
    }

    #[test]
    fn write_ledger_creates_parent_directories() {
        let dir = scratch_dir("write");
        let path = dir.join("nested").join("m4-ledger.jsonl");
        write_ledger(&path, &[row(3)]).expect("write");
        let text = std::fs::read_to_string(&path).expect("read back");
        assert_eq!(text, ledger_text(&[row(3)]).expect("serializes"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_identical_ledger_reports_no_changed_seed() {
        let rows: Vec<LedgerRow> = (0..3).map(row).collect();
        let lines = diff_ledgers(&rows, &rows);
        assert_eq!(
            lines[0],
            "synthetic ledger diff vs committed (report-only): 0 seed(s) changed"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains("elapsed_ms differs on 0 of 3 seed(s)")),
            "{lines:?}"
        );
        assert!(
            lines
                .last()
                .expect("a last line")
                .contains("only in the committed ledger: 0, only in this run: 0"),
            "{lines:?}"
        );
        assert!(
            !lines.iter().any(|l| l.starts_with("  seed")),
            "no seed line when nothing moved: {lines:?}"
        );
    }

    #[test]
    fn per_field_totals_come_in_schema_order_and_each_seed_is_named() {
        let committed: Vec<LedgerRow> = (0..6).map(row).collect();
        let mut actual = committed.clone();
        actual[3] = row_with(3, |r| {
            r.hit = false;
            r.miss_kind = Some("checksum_failed".to_string());
        });
        actual[5] = row_with(5, |r| {
            r.names_exact = false;
            r.name_error = Some("given_names_swapped".to_string());
            r.wrong_fields = vec!["given_names".to_string(), "surname".to_string()];
        });
        let lines = diff_ledgers(&committed, &actual);
        assert_eq!(
            lines[0],
            "synthetic ledger diff vs committed (report-only): 2 seed(s) changed; \
             hit 1, miss_kind 1, wrong_fields 1, names_exact 1, name_error 1"
        );
        assert_eq!(
            lines[1],
            "  seed 3: hit true -> false; miss_kind null -> checksum_failed"
        );
        assert_eq!(
            lines[2],
            "  seed 5: wrong_fields [] -> [given_names,surname]; names_exact true -> false; \
             name_error null -> given_names_swapped"
        );
    }

    /// The field names exist twice, in `DIFFED_FIELDS` (the order the totals
    /// print in) and as the literals in `field_changes`, and are only matched
    /// as strings. One seed that differs on every diffed field pins that the
    /// two lists agree: a name that drifts in either place drops out of the
    /// totals line or comes out in the wrong place.
    #[test]
    fn a_seed_that_differs_on_every_field_is_totalled_under_every_name_in_schema_order() {
        let committed = vec![row(0)];
        let actual = vec![row_with(0, |r| {
            r.profile = "mobile".to_string();
            r.hit = false;
            r.miss_kind = Some("checksum_failed".to_string());
            r.wrong_accept = true;
            r.prefix_wrong_accept = true;
            r.wrong_fields = vec!["surname".to_string()];
            r.check_states = Some(
                [("composite".to_string(), Some(false))]
                    .into_iter()
                    .collect(),
            );
            r.names_exact = false;
            r.name_error = Some("other".to_string());
            r.line1_flagged = true;
            r.retry_stop = Some("pass_cap".to_string());
            r.retry_variant_id = Some("pass-01".to_string());
            r.retry_damaged_recovery = Some(true);
            r.tier1_damaged_recovery = Some(true);
        })];
        let lines = diff_ledgers(&committed, &actual);
        assert_eq!(
            lines[0],
            "synthetic ledger diff vs committed (report-only): 1 seed(s) changed; \
             profile 1, hit 1, miss_kind 1, wrong_accept 1, prefix_wrong_accept 1, \
             wrong_fields 1, check_states 1, names_exact 1, name_error 1, line1_flagged 1, \
             retry_stop 1, retry_variant_id 1, retry_damaged_recovery 1, \
             tier1_damaged_recovery 1"
        );
        assert_eq!(DIFFED_FIELDS.len(), 14);
        let seed_line = &lines[1];
        assert_eq!(
            seed_line.matches("; ").count() + 1,
            14,
            "one change per diffed field on the seed line: {seed_line}"
        );
        for field in DIFFED_FIELDS {
            assert!(
                seed_line.contains(&format!("{field} ")),
                "{field} missing from {seed_line}"
            );
        }
    }

    #[test]
    fn a_check_state_change_is_shown_as_compact_json() {
        let with_states = |composite: bool| {
            row_with(0, |r| {
                r.check_states = Some(
                    [("composite".to_string(), Some(composite))]
                        .into_iter()
                        .collect(),
                )
            })
        };
        let lines = diff_ledgers(&[with_states(true)], &[with_states(false)]);
        assert_eq!(
            lines[1],
            "  seed 0: check_states {\"composite\":true} -> {\"composite\":false}"
        );
    }

    #[test]
    fn seed_lines_are_capped_at_twenty_but_the_totals_are_complete() {
        let committed: Vec<LedgerRow> = (0..25).map(row).collect();
        let actual: Vec<LedgerRow> = (0..25).map(|s| row_with(s, |r| r.hit = false)).collect();
        let lines = diff_ledgers(&committed, &actual);
        assert_eq!(
            lines[0],
            "synthetic ledger diff vs committed (report-only): 25 seed(s) changed; hit 25"
        );
        assert_eq!(
            lines.iter().filter(|l| l.starts_with("  seed ")).count(),
            20,
            "{lines:?}"
        );
        assert!(lines.contains(&"  ... and 5 more".to_string()), "{lines:?}");
    }

    #[test]
    fn elapsed_ms_alone_is_one_timing_line_and_never_a_changed_seed() {
        let committed: Vec<LedgerRow> = (0..3).map(row).collect();
        let mut actual = committed.clone();
        actual[0].elapsed_ms = 110; // |delta| 10
        actual[1].elapsed_ms = 130; // |delta| 30
        let lines = diff_ledgers(&committed, &actual);
        assert_eq!(
            lines[0],
            "synthetic ledger diff vs committed (report-only): 0 seed(s) changed"
        );
        assert!(
            lines.contains(
                &"timing (report-only): elapsed_ms differs on 2 of 3 seed(s), median |delta| \
                  20 ms, total 300 ms -> 340 ms"
                    .to_string()
            ),
            "{lines:?}"
        );
    }

    #[test]
    fn every_change_on_a_budget_stopped_seed_is_a_timing_change() {
        let committed = vec![
            row(0),
            row_with(1, |r| r.retry_stop = Some("budget".to_string())),
        ];
        let actual = vec![
            row_with(0, |r| r.hit = false),
            row_with(1, |r| {
                r.retry_stop = Some("general_valid".to_string());
                r.hit = false;
            }),
        ];
        let lines = diff_ledgers(&committed, &actual);
        assert_eq!(
            lines[0],
            "synthetic ledger diff vs committed (report-only): 1 seed(s) changed; hit 1"
        );
        assert_eq!(lines[1], "  seed 0: hit true -> false");
        let group = lines
            .iter()
            .position(|l| l.starts_with("budget-limited seed(s)"))
            .expect("a budget-limited group");
        assert!(
            lines[group].contains("1 seed(s); hit 1, retry_stop 1"),
            "{lines:?}"
        );
        assert_eq!(
            lines[group + 1],
            "  seed 1: hit true -> false; retry_stop budget -> general_valid"
        );
    }

    #[test]
    fn a_seed_on_one_side_only_is_counted_and_never_compared() {
        let committed = vec![row(0), row(1), row(2)];
        let actual = vec![row(1), row(2), row(3), row(4)];
        let lines = diff_ledgers(&committed, &actual);
        assert_eq!(
            lines[0],
            "synthetic ledger diff vs committed (report-only): 0 seed(s) changed"
        );
        assert!(
            lines
                .last()
                .expect("a last line")
                .contains("only in the committed ledger: 1, only in this run: 2"),
            "{lines:?}"
        );
    }

    #[test]
    fn rows_are_joined_on_format_and_seed() {
        let committed = vec![row(0)];
        let actual = vec![row_with(0, |r| r.format = "td1".to_string())];
        let lines = diff_ledgers(&committed, &actual);
        assert_eq!(
            lines[0],
            "synthetic ledger diff vs committed (report-only): 0 seed(s) changed"
        );
        assert!(
            lines
                .last()
                .expect("a last line")
                .contains("only in the committed ledger: 1, only in this run: 1"),
            "{lines:?}"
        );
    }

    #[test]
    fn no_committed_file_is_one_bootstrap_line() {
        let dir = scratch_dir("bootstrap");
        let lines = diff_against_file(&dir.join("absent.jsonl"), &[row(0)]);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(
            lines[0].contains("no committed ledger") && lines[0].contains("bootstrap"),
            "{lines:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The diff has no failure to report: every entry point returns the lines
    /// to print and nothing else, so no caller can turn a diff into an exit
    /// code. A malformed or unreadable committed ledger is a line, not an error.
    #[test]
    fn a_broken_committed_ledger_is_a_line_and_never_a_failure() {
        let dir = scratch_dir("broken");
        let malformed = dir.join("malformed.jsonl");
        std::fs::write(&malformed, "{\"seed\": not json}\n").expect("write malformed");
        let lines: Vec<String> = diff_against_file(&malformed, &[row(0)]);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("could not parse"), "{lines:?}");

        // A directory where the file should be: readable as a path, not as text.
        let lines: Vec<String> = diff_against_file(&dir, &[row(0)]);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("could not read"), "{lines:?}");

        let very_different: Vec<LedgerRow> =
            (0..50).map(|s| row_with(s, |r| r.hit = false)).collect();
        let _: Vec<String> = diff_ledgers(&[row(0)], &very_different);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_ledger_read_from_a_file_is_diffed_like_the_rows_it_holds() {
        let dir = scratch_dir("from-file");
        let path = dir.join("committed.jsonl");
        let committed: Vec<LedgerRow> = (0..3).map(row).collect();
        write_ledger(&path, &committed).expect("write committed");
        let mut actual = committed.clone();
        actual[1].hit = false;
        assert_eq!(
            diff_against_file(&path, &actual),
            diff_ledgers(&committed, &actual)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
