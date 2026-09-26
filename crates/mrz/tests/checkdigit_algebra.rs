//! The check-digit algebra of
//! `knowledge/benchmarks/checkdigit-blindspots-exact-2026-09-26.md`, pinned
//! against the real parser.
//!
//! Each test predicts from the closed form alone whether a check digit still
//! agrees after a misread, then asks `parse_td3`. A pattern that moves a field's
//! check sum by `Σ wᵢΔᵢ ≡ 0 (mod 10)` is accepted as read, and every other is
//! caught. This is residue prediction (ADR-0021 Amendment 1 §1), carried from
//! one changed cell to two; `src/strip.rs` pins the one-cell case on all five
//! parsers.
//!
//! `cargo run -p mrz --example checkdigit_blindspots_exact` prints the rates
//! these laws produce.

use std::collections::BTreeSet;

use mrz::{blindspot, parse_td3, CLASSES, CONFUSABLES};

// ICAO 9303 specimen identity (Utopia / Anna Maria Eriksson); `src/lib.rs`'s
// test module has the provenance note.
const LINE1: &str = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
const LINE2: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";

const WEIGHTS: [usize; 3] = [7, 3, 1];

/// A TD3 line-2 field under its own check digit: its first cell on line 2,
/// its width, and its first cell's index in the composite's input string.
struct Field {
    start: usize,
    width: usize,
    composite_start: usize,
}

const DOCUMENT_NUMBER: Field = Field {
    start: 0,
    width: 9,
    composite_start: 0,
};
const DATE_OF_BIRTH: Field = Field {
    start: 13,
    width: 6,
    composite_start: 10,
};
const PERSONAL_NUMBER: Field = Field {
    start: 28,
    width: 14,
    composite_start: 24,
};

fn residue(c: char) -> usize {
    CLASSES
        .iter()
        .position(|class| class.contains(&c))
        .expect("MRZ alphabet character")
}

/// A misread of `printed` that moves its value by `delta` (mod 10): another
/// member of the target residue class, never the filler. A shift of 0 picks a
/// different glyph from `printed`'s own class.
fn misread(printed: char, delta: usize) -> char {
    CLASSES[(residue(printed) + delta) % 10]
        .iter()
        .copied()
        .find(|&c| c != printed && c != '<')
        .expect("every residue class has two non-filler members")
}

/// The digit `delta` above `printed` (mod 10): a date cell stays a digit.
fn digit_misread(printed: char, delta: usize) -> char {
    let d = printed.to_digit(10).expect("date cell") as usize;
    char::from(b'0' + ((d + delta) % 10) as u8)
}

fn read_with(misreads: &[(usize, char)]) -> mrz::MrzData {
    let mut cells: Vec<char> = LINE2.chars().collect();
    for &(at, c) in misreads {
        cells[at] = c;
    }
    let line2: String = cells.into_iter().collect();
    parse_td3(LINE1, &line2).expect("a misread line 2 still parses")
}

/// `(i, j)` for every pair of a field's data cells.
fn cell_pairs(width: usize) -> impl Iterator<Item = (usize, usize)> {
    (0..width).flat_map(move |i| (i + 1..width).map(move |j| (i, j)))
}

#[test]
fn only_1_l_and_6_g_among_the_confusables_share_a_residue() {
    // The repair table's other fourteen pairs cross residue classes, so a
    // check digit rejects every one of those misreads on its own.
    let blind: BTreeSet<(char, char)> = CONFUSABLES
        .iter()
        .flat_map(|&(key, group)| group.chars().map(move |alt| (key.min(alt), key.max(alt))))
        .filter(|&(a, b)| blindspot(a, b).is_blind())
        .collect();
    assert_eq!(blind, BTreeSet::from([('1', 'L'), ('6', 'G')]));
}

#[test]
fn two_misreads_in_the_document_number_pass_both_digits_or_neither() {
    let printed: Vec<char> = LINE2.chars().collect();
    let f = DOCUMENT_NUMBER;
    let mut same_shift_passes = [0usize; 10];
    for (i, j) in cell_pairs(f.width) {
        for (d1, same_shift) in same_shift_passes.iter_mut().enumerate() {
            for d2 in 0..10 {
                let passes = (WEIGHTS[i % 3] * d1 + WEIGHTS[j % 3] * d2) % 10 == 0;
                let read = read_with(&[
                    (f.start + i, misread(printed[f.start + i], d1)),
                    (f.start + j, misread(printed[f.start + j], d2)),
                ]);
                assert_eq!(
                    read.checks.document_number,
                    Some(passes),
                    "cells {i}, {j}; shifts {d1}, {d2}"
                );
                // The composite reads this field from its own first cell, in the
                // same 7-3-1 phase: for errors in these cells it is the same
                // equation twice.
                assert_eq!(
                    read.checks.composite,
                    Some(passes),
                    "cells {i}, {j}; shifts {d1}, {d2}"
                );
                if d1 == d2 && passes {
                    *same_shift += 1;
                }
            }
        }
    }
    // Of the 36 cell pairs, every one cancels a shared shift of 0 or 5, and
    // otherwise only the nine pairs of a weight-7 cell with a weight-3 cell.
    assert_eq!(same_shift_passes, [36, 9, 9, 9, 9, 36, 9, 9, 9, 9]);
}

#[test]
fn two_misreads_in_the_personal_number_pass_both_digits_or_neither() {
    let printed: Vec<char> = LINE2.chars().collect();
    let f = PERSONAL_NUMBER;
    assert_eq!(f.composite_start % 3, 0, "aligned with its own 7-3-1 cycle");
    let mut same_shift_passes = [0usize; 10];
    for (i, j) in cell_pairs(f.width) {
        for (d, same_shift) in same_shift_passes.iter_mut().enumerate() {
            let passes = d * (WEIGHTS[i % 3] + WEIGHTS[j % 3]) % 10 == 0;
            let read = read_with(&[
                (f.start + i, misread(printed[f.start + i], d)),
                (f.start + j, misread(printed[f.start + j], d)),
            ]);
            assert_eq!(
                read.checks.personal_number,
                Some(passes),
                "cells {i}, {j}; shift {d}"
            );
            assert_eq!(
                read.checks.composite,
                Some(passes),
                "cells {i}, {j}; shift {d}"
            );
            if passes {
                *same_shift += 1;
            }
        }
    }
    // 91 pairs; five weight-7 and five weight-3 cells make 25 cancelling pairs.
    assert_eq!(same_shift_passes, [91, 25, 25, 25, 25, 91, 25, 25, 25, 25]);
}

#[test]
fn two_misreads_in_a_date_meet_an_independent_composite() {
    let printed: Vec<char> = LINE2.chars().collect();
    let f = DATE_OF_BIRTH;
    assert_ne!(f.composite_start % 3, 0, "out of phase with its own cycle");
    let (mut own_passes, mut both_pass) = (0, 0);
    for (i, j) in cell_pairs(f.width) {
        for d1 in 1..10 {
            for d2 in 1..10 {
                let own = (WEIGHTS[i % 3] * d1 + WEIGHTS[j % 3] * d2) % 10;
                let composite = (WEIGHTS[(f.composite_start + i) % 3] * d1
                    + WEIGHTS[(f.composite_start + j) % 3] * d2)
                    % 10;
                // Every weight is odd, so both sums move by d1 + d2 (mod 2): a
                // second digit over the same cells can only add a mod-5 check.
                assert_eq!(own % 2, composite % 2);
                let read = read_with(&[
                    (f.start + i, digit_misread(printed[f.start + i], d1)),
                    (f.start + j, digit_misread(printed[f.start + j], d2)),
                ]);
                assert_eq!(
                    read.checks.date_of_birth,
                    Some(own == 0),
                    "cells {i}, {j}; shifts {d1}, {d2}"
                );
                assert_eq!(
                    read.checks.composite,
                    Some(composite == 0),
                    "cells {i}, {j}; shifts {d1}, {d2}"
                );
                if d1 == 4 && d2 == 4 {
                    own_passes += usize::from(own == 0);
                    both_pass += usize::from(own == 0 && composite == 0);
                }
            }
        }
    }
    // Two shifts of 4 (the 0→O shift) at 2 of the 6 cells: the own digit passes
    // 4 of 15 cell pairs, and the composite refuses all four.
    assert_eq!((own_passes, both_pass), (4, 0));
}
