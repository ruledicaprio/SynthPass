//! Hand-computed 7-3-1 known answers checked against the crate.
//!
//! [`icao_vectors.rs`](icao_vectors) pins the worked examples Doc 9303
//! publishes: the expected digits there are external constants, so it is a
//! known-answer test and it is genuinely independent — at exactly those
//! points. The literals below include independent hand-computed values and an
//! invalid-character rejection case.
//!
//! **Why these checks are integration tests.** Each fixed vector pins a
//! hand-computed public result, while the invalid-character case pins the
//! exact error payload. `cargo test --workspace` runs these contracts; examples
//! are not part of the default workspace test suite.

use mrz::MrzError;

/// `(field, expected)` — known answers computed independently by hand.
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
fn the_crate_matches_the_hand_computed_vectors_and_rejects_invalid_characters() {
    for &(field, expected) in VECTORS {
        assert_eq!(
            mrz::check_digit(field),
            Ok(expected),
            "{field}: mrz::check_digit disagrees with the hand-computed answer"
        );
    }
    assert_eq!(
        mrz::check_digit("AB?C"),
        Err(MrzError::BadCharacter {
            character: '?',
            line: None,
            position: 2
        }),
    );
}
