//! Renders a deterministic Markdown benchmark report from three static
//! artifacts — the `provider-bench` gate report JSON, the committed
//! real-specimen baseline, and `samples/corpus.jsonl` — with no corpus image
//! read and no OCR run. See `synthpass_bench::bench_report` for the design.
fn main() -> std::process::ExitCode {
    match synthpass_bench::bench_report::run(std::env::args().skip(1).collect()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("bench-report: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
