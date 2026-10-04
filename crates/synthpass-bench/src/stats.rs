//! Small pure statistics for the bench summaries.
//!
//! The one function here, [`wilson_interval`], is the interval `synthpass-bench` prints beside
//! a rate. What it measures, and what it does not, is in `knowledge/benchmarks/README.md`
//! ("Intervals in the synthetic summary").

/// `z` for a two-sided 95% interval.
pub const Z_95: f64 = 1.96;

/// The 95% Wilson score interval for `k` successes in `n` trials, as `(low, high)` fractions
/// in `0..=1`. `None` when `n` is `0`: no trials, no interval, never a made-up one.
///
/// Wilson rather than the normal approximation because the counts here are small (tens of
/// documents) and the rates sit near `0` or `1`, where the normal interval leaves `0..=1` or
/// collapses to a point. `k` above `n` is clamped to `n`.
pub fn wilson_interval(k: u64, n: u64) -> Option<(f64, f64)> {
    if n == 0 {
        return None;
    }
    let p = k.min(n) as f64 / n as f64;
    let n = n as f64;
    let z2 = Z_95 * Z_95;
    let denominator = 1.0 + z2 / n;
    let centre = (p + z2 / (2.0 * n)) / denominator;
    let half = Z_95 * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt() / denominator;
    Some(((centre - half).max(0.0), (centre + half).min(1.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(got: (f64, f64), low: f64, high: f64) {
        assert!(
            (got.0 - low).abs() < 0.0005 && (got.1 - high).abs() < 0.0005,
            "got {got:?}, want ({low}, {high})"
        );
    }

    /// The five known values: the M4 run's own counts (19 of 38 wrong on at least one field,
    /// 38 of 50 hits) and the two edges, where the interval must stay inside `0..=1`.
    #[test]
    fn wilson_matches_the_known_values() {
        close(wilson_interval(19, 38).unwrap(), 0.3485, 0.6515);
        close(wilson_interval(0, 38).unwrap(), 0.0000, 0.0918);
        close(wilson_interval(38, 50).unwrap(), 0.6259, 0.8570);
        close(wilson_interval(38, 38).unwrap(), 0.9082, 1.0000);
    }

    #[test]
    fn no_trials_means_no_interval() {
        assert_eq!(wilson_interval(0, 0), None);
    }

    #[test]
    fn the_interval_stays_inside_zero_and_one_and_brackets_the_rate() {
        for n in 1..=60u64 {
            for k in 0..=n {
                let (low, high) = wilson_interval(k, n).unwrap();
                let rate = k as f64 / n as f64;
                assert!((0.0..=1.0).contains(&low) && (0.0..=1.0).contains(&high));
                assert!(low <= rate + 1e-12 && rate <= high + 1e-12, "{k}/{n}");
            }
        }
    }
}
