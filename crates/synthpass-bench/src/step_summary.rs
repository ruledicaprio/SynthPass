//! The GitHub Actions step summary, shared by the two bench binaries.
//!
//! `provider-bench` and `synthpass-bench` both print a report-only ledger diff
//! and copy the same lines into the job's step summary. This module is that
//! copy: the one place that knows `GITHUB_STEP_SUMMARY` and the fenced-block
//! shape, so the two binaries cannot drift apart on it.

use std::io::Write as _;
use std::path::{Path, PathBuf};

/// The step summary file GitHub Actions names in `GITHUB_STEP_SUMMARY`; `None`
/// outside Actions (unset or empty).
pub fn step_summary_path() -> Option<PathBuf> {
    std::env::var_os("GITHUB_STEP_SUMMARY")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// Appends `lines` to the GitHub step summary at `path` under the heading
/// `title` (a full Markdown heading line, for example `### Ledger diff`) as one
/// fenced block, or does nothing when `path` is `None` (`GITHUB_STEP_SUMMARY`
/// unset). The caller warns and carries on if this fails: the summary is
/// evidence, never a gate condition.
///
/// The repository is public, so the summary is public too: `lines` must
/// already be limited to asset ids, field names, enumerated values and counts.
pub fn append_step_summary(
    path: Option<&Path>,
    title: &str,
    lines: &[String],
) -> std::io::Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let mut body = format!("{title}\n\n```text\n");
    for line in lines {
        body.push_str(line);
        body.push('\n');
    }
    body.push_str("```\n");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(body.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "synthpass-bench-step-summary-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    /// Moved here from `provider-bench`, where the helper used to live, with
    /// the heading now a parameter. Same assertions, plus the heading.
    #[test]
    fn append_step_summary_writes_the_diff_lines_and_nothing_without_a_path() {
        let dir = scratch_dir("append");
        let summary = dir.join("summary.md");
        let title = "### Some ledger diff (report-only)";
        let lines = vec![
            "outcome ledger diff vs committed: 0 document(s) changed outcome".to_string(),
            "  a: mrz_format null -> TD3".to_string(),
        ];

        append_step_summary(None, title, &lines).expect("no path is a no-op");
        assert!(!summary.exists(), "an unset variable must write nothing");

        append_step_summary(Some(summary.as_path()), title, &lines).expect("write the summary");
        let text = std::fs::read_to_string(&summary).expect("summary exists");
        assert!(text.starts_with(&format!("{title}\n\n```text\n")), "{text}");
        for line in &lines {
            assert!(text.contains(&format!("{line}\n")), "{text}");
        }
        assert!(text.ends_with("```\n"));

        // GitHub steps append to one shared file; a second call must not truncate.
        append_step_summary(Some(summary.as_path()), title, &lines).expect("append again");
        let twice = std::fs::read_to_string(&summary).expect("summary exists");
        assert_eq!(twice.matches("  a: mrz_format null -> TD3\n").count(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
