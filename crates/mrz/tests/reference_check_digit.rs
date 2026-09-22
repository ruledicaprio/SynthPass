//! A second, independently written 7-3-1 implementation, checked against the
//! crate's own over arbitrary input.
//!
//! [`icao_vectors.rs`](icao_vectors) pins the worked examples Doc 9303
//! publishes: the expected digits there are external constants, so it is a
//! known-answer test and it is genuinely independent — at exactly those
//! points. This file is the complementary shape. `reference_check_digit`
//! below is written from the rule text in Part 3 §4.9 and shares no code with
//! `src/checksum.rs`, and `proptest` drives the two implementations over
//! arbitrary MRZ strings, so agreement is asserted across the input space
//! rather than at a fixed list of points. A transcription error that happens
//! to miss every published vector still has to survive this.
//!
//! **Why this is a test and not an example.** It was adapted from a
//! standalone audit tool proposed for `examples/`, which carried the same
//! differential idea in a `#[cfg(test)]` module. That placement does not work:
//! `cargo test --workspace`, which is what CI runs, does not execute test
//! modules inside `examples/` — only an explicit `cargo test --example <name>`
//! does, and nothing runs that. The oracle would have read as coverage while
//! never executing.

use proptest::prelude::*;

fn value_of(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'A'..=b'Z' => Some(u32::from(byte - b'A') + 10),
        b'<' => Some(0),
        _ => None,
    }
}

/// ICAO 9303 Part 3 §4.9, implemented from the rule text: values
/// `0-9 → 0..9`, `A-Z → 10..35`, `< → 0`; weights 7, 3, 1 repeating from the
/// **left**; the check digit is the weighted sum mod 10. A filler contributes
/// zero but still consumes its weight position.
fn reference_check_digit(field: &str) -> Option<u32> {
    const WEIGHTS: [u32; 3] = [7, 3, 1];
    let mut sum = 0u32;
    for (index, byte) in field.bytes().enumerate() {
        sum += value_of(byte)? * WEIGHTS[index % WEIGHTS.len()];
    }
    Some(sum % 10)
}

// ---- Two plausible wrong implementations ----
//
// These exist only to prove the vectors below can tell a right implementation
// from a wrong one. A test vector that a *broken* implementation also
// satisfies proves nothing, and both of these are real mistakes people make:
// applying the weights from the wrong end, and treating a filler as absent
// rather than as zero.

/// Weights applied from the right-hand end instead of the left.
fn mutant_weights_from_the_right(field: &str) -> Option<u32> {
    const WEIGHTS: [u32; 3] = [7, 3, 1];
    let mut sum = 0u32;
    for (index, byte) in field.bytes().rev().enumerate() {
        sum += value_of(byte)? * WEIGHTS[index % WEIGHTS.len()];
    }
    Some(sum % 10)
}

/// Fillers skipped entirely, so they do not consume a weight position.
fn mutant_fillers_skipped(field: &str) -> Option<u32> {
    const WEIGHTS: [u32; 3] = [7, 3, 1];
    let mut sum = 0u32;
    let mut weight_index = 0usize;
    for byte in field.bytes() {
        let value = value_of(byte)?;
        if byte == b'<' {
            continue;
        }
        sum += value * WEIGHTS[weight_index % WEIGHTS.len()];
        weight_index += 1;
    }
    Some(sum % 10)
}

/// `(field, expected)` — each computed by hand under the rule text, and each
/// chosen so at least one of the two mutants above gets it wrong.
const VECTORS: &[(&str, u32)] = &[
    // ICAO 9303 Part 4's published TD3 specimen document number.
    ("L898902C3", 6),
    // Nine characters with a letter pair at the two heaviest weights, so
    // applying the weights from the wrong end lands somewhere else:
    // 29·7 + 29·3 + 1·1 + 3·7 + 0 + 0 + 0 + 0 + 5·1 = 317 → 7.
    ("TT1300005", 7),
    // A filler in the middle: 10·7 + 11·3 + 0·1 + 12·7 = 187 → 7. Skipping the
    // filler instead of zeroing it gives 10·7 + 11·3 + 12·1 = 115 → 5.
    ("AB<C", 7),
];

#[test]
fn the_vectors_can_actually_tell_right_from_wrong() {
    let mut separated_by_weights = 0;
    let mut separated_by_fillers = 0;
    for &(field, expected) in VECTORS {
        assert_eq!(
            reference_check_digit(field),
            Some(expected),
            "{field}: the reference implementation disagrees with the hand computation"
        );
        if mutant_weights_from_the_right(field) != Some(expected) {
            separated_by_weights += 1;
        }
        if mutant_fillers_skipped(field) != Some(expected) {
            separated_by_fillers += 1;
        }
    }
    assert!(
        separated_by_weights > 0,
        "no vector detects weights applied from the wrong end"
    );
    assert!(
        separated_by_fillers > 0,
        "no vector detects a filler being skipped rather than zeroed"
    );
}

#[test]
fn the_crate_agrees_with_the_reference_on_every_vector() {
    for &(field, expected) in VECTORS {
        assert_eq!(
            mrz::check_digit(field),
            Ok(expected),
            "{field}: mrz::check_digit disagrees with the independent reference"
        );
    }
}

proptest! {
    /// The differential assertion: over arbitrary MRZ-charset input of any
    /// length, the crate and an implementation that shares no code with it
    /// must produce the same digit.
    #[test]
    fn crate_and_reference_agree_on_arbitrary_mrz_input(field in "[0-9A-Z<]{0,44}") {
        prop_assert_eq!(
            mrz::check_digit(&field).ok(),
            reference_check_digit(&field),
            "field={:?}", field
        );
    }

    /// And they must agree about *rejection*: anything outside the alphabet is
    /// an error on one side and `None` on the other.
    #[test]
    fn crate_and_reference_agree_on_rejection(field in ".{0,32}") {
        let reference = reference_check_digit(&field);
        prop_assert_eq!(
            mrz::check_digit(&field).ok(),
            reference,
            "field={:?}", field
        );
    }
}
