//! Offline human review and validated promotion of specimen ground truth.
fn main() -> std::process::ExitCode {
    match synthpass_bench::ground_truth::run(std::env::args().skip(1).collect()) {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::FAILURE,
        Err(error) => {
            eprintln!("ground-truth: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
