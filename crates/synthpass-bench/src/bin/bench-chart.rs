//! `bench-chart` — reads one track's `results/<track>/history.jsonl`
//! (accumulated by `scripts/run-bench.ps1 -Track <track>` on the
//! `bench-data` branch) and renders a static SVG trend chart, embedded in
//! `knowledge/benchmarks/README.md`'s live-tracks dashboard. Track-agnostic:
//! it only knows the row shape, not which provider-bench invocation produced
//! it, so the same binary serves every track (`passport`, `real-specimens`,
//! `id_card`, ...).
//!
//! Two panels are always drawn (both computable with zero ground truth):
//! read-success rate (fraction of documents where OCR + parse produced a
//! result at all) and unsupported-assertion rate. A third panel for
//! `field_match_rate` is drawn only when at least one row in the whole
//! history has a non-null value — accuracy starts sparse (few
//! `samples/ocr_fixtures/` labels exist for most tracks yet) and this panel
//! would otherwise render an empty, confusing plot area.
//!
//! ```text
//! bench-chart --history PATH --out PATH
//!   --history PATH   path to results/<track>/history.jsonl
//!   --out PATH       output SVG path
//! ```
//!
//! A second mode — added for the M6 per-format comparison
//! (`knowledge/img/format-comparison.svg`) — draws one bar per track instead
//! of a trend line over time:
//!
//! ```text
//! bench-chart --bars --series LABEL=PATH [--series LABEL=PATH ...] --out PATH
//!   --series LABEL=PATH   one track's history.jsonl, labelled LABEL on the
//!                         chart; repeat for each bar
//!   --out PATH            output SVG path
//! ```
//!
//! Each bar's height is that series' **latest** row's `read_ok_rate` — the
//! same field the trend mode's first panel already plots, so a track's
//! trend chart and its bar here are always reading the same number. Reuses
//! [`read_history`] verbatim: a malformed history line costs that line, not
//! the whole chart, in both modes alike.

use plotters::prelude::*;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct HistoryRow {
    run_timestamp_unix: i64,
    provider_id: String,
    #[serde(default)]
    read_ok_rate: Option<f64>,
    #[serde(default)]
    field_match_rate: Option<f64>,
    #[serde(default)]
    unsupported_assertion_rate: Option<f64>,
    /// How many documents the run covered — carried through to the bar
    /// mode's caption so a small corpus never gets to look bigger than it
    /// is (see the module doc comment / the M6 plan's "draw it honestly").
    /// Absent in no real row (every row shape in
    /// `knowledge/benchmarks/README.md` includes it) but `#[serde(default)]`
    /// all the same, matching every other optional field here: an absent
    /// count renders as "n=?" rather than refusing to chart.
    #[serde(default)]
    documents: Option<u64>,
}

#[derive(Debug)]
enum Mode {
    /// The original mode: one track's history, plotted as a trend over time.
    Trend { history: String },
    /// One bar per named series, each series' latest `read_ok_rate`.
    Bars { series: Vec<(String, String)> },
}

#[derive(Debug)]
struct Args {
    mode: Mode,
    out: String,
}

fn usage() {
    eprintln!("Usage: bench-chart --history PATH --out PATH");
    eprintln!("       bench-chart --bars --series LABEL=PATH [--series LABEL=PATH ...] --out PATH");
    eprintln!("  --history PATH        path to results/<track>-bench/history.jsonl (trend mode)");
    eprintln!("  --bars                bar mode: one bar per --series, instead of a trend line");
    eprintln!(
        "  --series LABEL=PATH   one bar's label and history.jsonl path (bar mode; repeatable)"
    );
    eprintln!("  --out PATH            output SVG path");
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut history = None;
    let mut out = None;
    let mut bars = false;
    let mut series: Vec<(String, String)> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--history" => {
                history = Some(
                    args.get(i + 1)
                        .ok_or_else(|| "--history requires a value".to_string())?
                        .clone(),
                );
                i += 2;
            }
            "--out" => {
                out = Some(
                    args.get(i + 1)
                        .ok_or_else(|| "--out requires a value".to_string())?
                        .clone(),
                );
                i += 2;
            }
            "--bars" => {
                bars = true;
                i += 1;
            }
            "--series" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--series requires a value".to_string())?;
                let (label, path) = v
                    .split_once('=')
                    .ok_or_else(|| format!("--series must be LABEL=PATH, got: {v}"))?;
                if label.is_empty() || path.is_empty() {
                    return Err(format!("--series must be LABEL=PATH, got: {v}"));
                }
                series.push((label.to_string(), path.to_string()));
                i += 2;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    let out = out.ok_or_else(|| "--out is required".to_string())?;
    let mode = if bars {
        if series.is_empty() {
            return Err("--bars requires at least one --series LABEL=PATH".to_string());
        }
        Mode::Bars { series }
    } else {
        Mode::Trend {
            history: history.ok_or_else(|| "--history is required".to_string())?,
        }
    };
    Ok(Args { mode, out })
}

/// Parses `path` as newline-delimited JSON, one [`HistoryRow`] per line.
///
/// A line that fails to parse is skipped with a warning, not treated as a
/// fatal error — same "one bad entry costs one entry, not the run" contract
/// `synthpass_bench::load_ground_truth` already follows, since this file is
/// appended to by hand-run scripts and a single truncated line (e.g. from an
/// interrupted push) should not block every chart regeneration after it.
fn read_history(path: &Path) -> Result<Vec<HistoryRow>, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let mut rows = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<HistoryRow>(line) {
            Ok(row) => rows.push(row),
            Err(e) => eprintln!(
                "warning: {}:{} did not parse as a history row ({e}) — skipping",
                path.display(),
                n + 1
            ),
        }
    }
    Ok(rows)
}

/// Groups rows by `provider_id`, preserving first-seen order (so the legend
/// and color assignment are stable across regenerations rather than
/// depending on `HashMap` iteration order).
fn group_by_provider(rows: Vec<HistoryRow>) -> Vec<(String, Vec<HistoryRow>)> {
    let mut groups: Vec<(String, Vec<HistoryRow>)> = Vec::new();
    for row in rows {
        match groups.iter_mut().find(|(id, _)| *id == row.provider_id) {
            Some((_, group)) => group.push(row),
            None => groups.push((row.provider_id.clone(), vec![row])),
        }
    }
    for (_, group) in &mut groups {
        group.sort_by_key(|r| r.run_timestamp_unix);
    }
    groups
}

const PALETTE: [RGBColor; 4] = [RED, BLUE, GREEN, MAGENTA];

fn draw_panel(
    area: &DrawingArea<SVGBackend, plotters::coord::Shift>,
    title: &str,
    groups: &[(String, Vec<HistoryRow>)],
    x_range: (i64, i64),
    value: impl Fn(&HistoryRow) -> Option<f64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut chart = ChartBuilder::on(area)
        .caption(title, ("sans-serif", 18))
        .margin(10)
        .x_label_area_size(28)
        .y_label_area_size(45)
        .build_cartesian_2d(x_range.0..x_range.1, 0.0f64..1.0f64)?;

    chart
        .configure_mesh()
        .x_desc("run timestamp (unix seconds)")
        .y_desc(title)
        .draw()?;

    for (idx, (provider_id, rows)) in groups.iter().enumerate() {
        let color = PALETTE[idx % PALETTE.len()];
        let points: Vec<(i64, f64)> = rows
            .iter()
            .filter_map(|r| value(r).map(|v| (r.run_timestamp_unix, v)))
            .collect();
        if points.is_empty() {
            continue;
        }
        chart
            .draw_series(LineSeries::new(points.iter().copied(), &color))?
            .label(provider_id.clone())
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color));
        chart.draw_series(
            points
                .iter()
                .map(|&(x, y)| Circle::new((x, y), 3, color.filled())),
        )?;
    }

    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()?;
    Ok(())
}

fn run_trend(history: &str, out: &str) -> Result<(), Box<dyn std::error::Error>> {
    let rows = read_history(Path::new(history))?;
    if rows.is_empty() {
        return Err(format!("{history} has no valid rows — nothing to chart").into());
    }
    let groups = group_by_provider(rows);

    let all_timestamps: Vec<i64> = groups
        .iter()
        .flat_map(|(_, rows)| rows.iter().map(|r| r.run_timestamp_unix))
        .collect();
    let x_min = *all_timestamps.iter().min().expect("non-empty");
    let x_max = *all_timestamps.iter().max().expect("non-empty");
    // A single run has x_min == x_max; a degenerate (zero-width) axis range
    // is invalid for plotters' cartesian coordinate system, so widen it by a
    // token amount purely for a valid, still-readable single-point plot.
    let x_range = if x_min == x_max {
        (x_min - 1, x_max + 1)
    } else {
        (x_min, x_max)
    };

    let has_accuracy = groups
        .iter()
        .flat_map(|(_, rows)| rows.iter())
        .any(|r| r.field_match_rate.is_some());

    let panel_count = if has_accuracy { 3 } else { 2 };
    let root = SVGBackend::new(out, (900, 320 * panel_count as u32)).into_drawing_area();
    root.fill(&WHITE)?;
    let panels = root.split_evenly((panel_count, 1));

    draw_panel(&panels[0], "MRZ read-success rate", &groups, x_range, |r| {
        r.read_ok_rate
    })?;
    draw_panel(
        &panels[1],
        "Unsupported-assertion rate",
        &groups,
        x_range,
        |r| r.unsupported_assertion_rate,
    )?;
    if has_accuracy {
        draw_panel(
            &panels[2],
            "Field match rate (labelled specimens only)",
            &groups,
            x_range,
            |r| r.field_match_rate,
        )?;
    }

    root.present()?;
    Ok(())
}

/// One bar's resolved data: its label, its latest row's `read_ok_rate`, and
/// that row's document count (for the caption).
struct Bar {
    label: String,
    rate: f64,
    documents: Option<u64>,
}

/// Resolves one `--series LABEL=PATH` into a [`Bar`]: the **latest** row (by
/// `run_timestamp_unix`) in that path's history that has a `read_ok_rate` at
/// all. Every synthetic-track row always sets `read_ok_rate` (see
/// `knowledge/benchmarks/README.md`'s row-shape doc), so in practice this is
/// simply the most recent row — the `filter_map` only matters for a
/// real-specimen track history fed in by mistake, where a row's
/// `read_ok_rate` can be legitimately absent.
fn resolve_bar(label: &str, path: &str) -> Result<Bar, String> {
    let rows = read_history(Path::new(path))?;
    let latest = rows
        .into_iter()
        .filter(|r| r.read_ok_rate.is_some())
        .max_by_key(|r| r.run_timestamp_unix)
        .ok_or_else(|| format!("{path} has no row with a read_ok_rate — nothing to chart"))?;
    Ok(Bar {
        label: label.to_string(),
        rate: latest.read_ok_rate.expect("filtered above"),
        documents: latest.documents,
    })
}

/// Draws one bar per `--series`, each bar's height its track's latest
/// `read_ok_rate`. A fixed 0.0-1.0 y-axis (never cropped to the data's own
/// range) so bars are honestly comparable to each other and to the trend
/// panels above — the M6 plan's explicit "no bar is exaggerated by a
/// cropped axis" requirement. Each bar is labelled with its percentage;
/// the caption states every series' document count, so a 30-document corpus
/// never gets to look like a larger claim than it is.
fn run_bars(series: &[(String, String)], out: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bars: Vec<Bar> = series
        .iter()
        .map(|(label, path)| resolve_bar(label, path))
        .collect::<Result<_, String>>()?;

    let root = SVGBackend::new(out, (700, 460)).into_drawing_area();
    root.fill(&WHITE)?;

    let n = bars.len() as i32;
    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Tier-1 hit rate by MRZ format (latest run of each track)",
            ("sans-serif", 20),
        )
        .margin(20)
        .x_label_area_size(36)
        .y_label_area_size(50)
        .build_cartesian_2d(0i32..n, 0.0f64..1.0f64)?;

    chart
        .configure_mesh()
        .y_desc("Tier-1 hit rate")
        .disable_x_mesh()
        .x_labels(bars.len().max(1))
        .x_label_formatter(&|x| {
            bars.get(*x as usize)
                .map(|b| b.label.clone())
                .unwrap_or_default()
        })
        .draw()?;

    for (i, bar) in bars.iter().enumerate() {
        let i = i as i32;
        let color = PALETTE[i as usize % PALETTE.len()];
        chart.draw_series(std::iter::once(Rectangle::new(
            [(i, 0.0), (i + 1, bar.rate)],
            color.filled(),
        )))?;
        let label_y = (bar.rate + 0.03).min(0.97);
        chart.draw_series(std::iter::once(Text::new(
            format!("{:.1}%", bar.rate * 100.0),
            (i, label_y),
            ("sans-serif", 16).into_font(),
        )))?;
    }

    // Document counts, spelled out per series rather than assumed uniform —
    // a corpus size difference between tracks must stay visible, not get
    // averaged away.
    let caption = bars
        .iter()
        .map(|b| match b.documents {
            Some(n) => format!("{}: n={n}", b.label),
            None => format!("{}: n=?", b.label),
        })
        .collect::<Vec<_>>()
        .join("  ·  ");
    root.draw(&Text::new(
        caption,
        (20, 440),
        ("sans-serif", 13).into_font(),
    ))?;

    root.present()?;
    Ok(())
}

fn run(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    match &args.mode {
        Mode::Trend { history } => run_trend(history, &args.out),
        Mode::Bars { series } => run_bars(series, &args.out),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match parse_args(&args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ {e}");
            usage();
            std::process::exit(1);
        }
    };
    if let Err(e) = run(&parsed) {
        eprintln!("❌ {e}");
        std::process::exit(1);
    }
    println!("chart written to {}", parsed.out);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_history_skips_malformed_lines_without_aborting() {
        let dir = std::env::temp_dir().join(format!(
            "passport-bench-chart-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("history.jsonl");
        std::fs::write(
            &path,
            "{ not valid json\n\
             {\"run_timestamp_unix\": 1, \"provider_id\": \"mrz\", \"read_ok_rate\": 1.0}\n\
             \n\
             {\"run_timestamp_unix\": 2, \"provider_id\": \"llm\", \"field_match_rate\": 0.5}\n",
        )
        .unwrap();

        let rows = read_history(&path).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(
            rows.len(),
            2,
            "the malformed line and blank line are skipped"
        );
        assert_eq!(rows[0].provider_id, "mrz");
        assert_eq!(rows[1].provider_id, "llm");
    }

    #[test]
    fn group_by_provider_preserves_first_seen_order_and_sorts_by_time() {
        let rows = vec![
            HistoryRow {
                run_timestamp_unix: 20,
                provider_id: "llm".into(),
                read_ok_rate: None,
                field_match_rate: None,
                unsupported_assertion_rate: None,
                documents: None,
            },
            HistoryRow {
                run_timestamp_unix: 10,
                provider_id: "mrz".into(),
                read_ok_rate: None,
                field_match_rate: None,
                unsupported_assertion_rate: None,
                documents: None,
            },
            HistoryRow {
                run_timestamp_unix: 5,
                provider_id: "llm".into(),
                read_ok_rate: None,
                field_match_rate: None,
                unsupported_assertion_rate: None,
                documents: None,
            },
        ];
        let groups = group_by_provider(rows);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "llm", "first-seen provider stays first");
        assert_eq!(
            groups[0]
                .1
                .iter()
                .map(|r| r.run_timestamp_unix)
                .collect::<Vec<_>>(),
            vec![5, 20],
            "rows within a provider are time-sorted"
        );
        assert_eq!(groups[1].0, "mrz");
    }

    /// End-to-end smoke test: renders a tiny fixture history to a real SVG
    /// file and checks it looks like one. Needs no OCR models/GGUF (unlike
    /// most of this crate's `#[ignore]`d tests), so it runs by default.
    #[test]
    fn renders_a_well_formed_svg_from_a_small_fixture() {
        let dir = std::env::temp_dir().join(format!(
            "passport-bench-chart-svg-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let history = dir.join("history.jsonl");
        std::fs::write(
            &history,
            "{\"run_timestamp_unix\": 1, \"provider_id\": \"mrz\", \"read_ok_rate\": 1.0, \"unsupported_assertion_rate\": 0.1}\n\
             {\"run_timestamp_unix\": 2, \"provider_id\": \"mrz\", \"read_ok_rate\": 0.9, \"unsupported_assertion_rate\": 0.2, \"field_match_rate\": 0.8}\n",
        )
        .unwrap();
        let out = dir.join("trend.svg");

        run(&Args {
            mode: Mode::Trend {
                history: history.to_string_lossy().to_string(),
            },
            out: out.to_string_lossy().to_string(),
        })
        .expect("chart renders");

        let svg = std::fs::read_to_string(&out).expect("svg written");
        std::fs::remove_dir_all(&dir).ok();
        assert!(svg.trim_start().starts_with("<?xml") || svg.trim_start().starts_with("<svg"));
        assert!(svg.contains("<svg"));
    }

    /// Bar mode's own smoke test: two tiny fixture histories, rendered as a
    /// two-bar comparison chart. Confirms `--bars`/`--series` produces a
    /// well-formed SVG using the latest row of each series, same "runs by
    /// default, no OCR models needed" property as the trend-mode test above.
    #[test]
    fn bar_mode_renders_a_well_formed_svg_using_each_series_latest_row() {
        let dir = std::env::temp_dir().join(format!(
            "bench-chart-bars-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let td1_history = dir.join("td1-history.jsonl");
        std::fs::write(
            &td1_history,
            "{\"run_timestamp_unix\": 1, \"provider_id\": \"mrz\", \"read_ok_rate\": 0.2667, \"documents\": 30}\n\
             {\"run_timestamp_unix\": 2, \"provider_id\": \"mrz\", \"read_ok_rate\": 0.7, \"documents\": 30}\n",
        )
        .unwrap();
        let td2_history = dir.join("td2-history.jsonl");
        std::fs::write(
            &td2_history,
            "{\"run_timestamp_unix\": 1, \"provider_id\": \"mrz\", \"read_ok_rate\": 0.8, \"documents\": 30}\n",
        )
        .unwrap();

        let out = dir.join("format-comparison.svg");
        run(&Args {
            mode: Mode::Bars {
                series: vec![
                    ("TD1".to_string(), td1_history.to_string_lossy().to_string()),
                    ("TD2".to_string(), td2_history.to_string_lossy().to_string()),
                ],
            },
            out: out.to_string_lossy().to_string(),
        })
        .expect("bar chart renders");

        let svg = std::fs::read_to_string(&out).expect("svg written");
        std::fs::remove_dir_all(&dir).ok();
        assert!(svg.contains("<svg"));
        // The TD1 bar must reflect the *latest* row (0.7), not the first
        // (0.2667) — this is the whole point of resolve_bar's max_by_key.
        assert!(
            svg.contains("70.0%"),
            "expected the latest TD1 read_ok_rate (70%) in the rendered SVG"
        );
        assert!(
            !svg.contains("26.7%"),
            "the stale first TD1 row must not be the one plotted"
        );
    }

    #[test]
    fn parse_args_requires_at_least_one_series_in_bar_mode() {
        let err = parse_args(&[
            "--bars".to_string(),
            "--out".to_string(),
            "out.svg".to_string(),
        ])
        .unwrap_err();
        assert!(err.contains("--series"));
    }

    #[test]
    fn parse_args_rejects_a_series_without_an_equals_sign() {
        let err = parse_args(&[
            "--bars".to_string(),
            "--series".to_string(),
            "TD1".to_string(),
            "--out".to_string(),
            "out.svg".to_string(),
        ])
        .unwrap_err();
        assert!(err.contains("LABEL=PATH"));
    }
}
