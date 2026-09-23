//! Why `valid()` is checksum consistency and not "this is what was printed".
//!
//! [`blindspot.rs`](blindspot) already proves the *pairwise* law empirically:
//! it mutates the ICAO specimen one character at a time across every line-2
//! data position and asserts the parser agrees with `blindspot`'s prediction.
//! That is thorough, and this file deliberately does not repeat it.
//!
//! It closes the two gaps that sweep leaves, both of which are mechanisms by
//! which a checksum-valid zone can differ from the printed one:
//!
//! 1. **Fields no check digit covers.** The sweep only mutates line 2. On TD3
//!    and TD2 the composite spans line 2 alone, so line 1 — document code,
//!    issuing state, and both name fields — is arithmetically unprotected.
//! 2. **Combinations.** The sweep is strictly one character at a time, so it
//!    can only ever exhibit the pairwise law. The shift a check digit sees is
//!    `Σ Δᵢ·wᵢ (mod 10)`, so substitutions that are individually *caught*
//!    cancel. `src/blindspot.rs` and the README both state this; before this
//!    file nothing executed it.

use mrz::{blindspot, parse_td3, Blindspot};

// ICAO 9303 specimen identity (Utopia / Anna Maria Eriksson) — see
// `src/lib.rs`'s test module for the full provenance note.
const L1: &str = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
const L2: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";

#[test]
fn the_specimen_is_the_baseline() {
    let doc = parse_td3(L1, L2).unwrap();
    assert!(doc.valid());
    assert_eq!(doc.surname, "ERIKSSON");
    assert_eq!(doc.document_number, "L898902C3");
}

/// TD3's composite covers line 2 only (positions 1-10, 14-20, 22-43), so
/// nothing on line 1 is constrained by arithmetic at all. A wrong surname, a
/// wrong given name and a wrong issuing state each survive every printed check
/// digit — which is the plainest available refutation of "a valid check digit
/// proves the read is faithful to the printed document".
#[test]
fn td3_line_one_is_covered_by_no_check_digit() {
    // (mutated line 1, what the parse then reports, which field moved)
    let cases = [
        ("P<UTOERIKSSDN<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<", "surname"),
        (
            "P<UTOERIKSSON<<EMMA<MARIA<<<<<<<<<<<<<<<<<<<",
            "given_names",
        ),
        (
            "P<XXXERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
            "issuing_country",
        ),
    ];
    for (mutated_l1, field) in cases {
        assert_ne!(
            mutated_l1, L1,
            "{field}: the test case must actually differ"
        );
        let doc = parse_td3(mutated_l1, L2).unwrap();
        assert!(
            doc.valid(),
            "{field}: a line-1 mutation must survive every check digit, because none covers it"
        );
        assert_eq!(
            doc.checks.composite,
            Some(true),
            "{field}: composite spans line 2 only"
        );
    }

    // And the damage is real: the parsed record disagrees with the specimen.
    let doc = parse_td3(cases[0].0, L2).unwrap();
    assert_eq!(doc.surname, "ERIKSSDN");
    assert!(doc.valid());
}

/// Two substitutions that `blindspot` classifies as **Caught** individually,
/// placed at weights 7 and 3 so their shifts cancel mod 10.
///
/// `L`(21) → `P`(25) and `8`(8) → `C`(12) are each Δ ≡ 4. At document-number
/// positions 0 and 1 the weights are 7 and 3, so the total shift is
/// `4·7 + 4·3 = 40 ≡ 0`. The same two positions open the composite's own span
/// at the same weights, so it cancels there too — which is why this is not
/// caught by the second digit either.
#[test]
fn two_individually_caught_substitutions_cancel_mod_ten() {
    // The premise: each swap, alone, is one the arithmetic can see.
    //
    // `blindspot(a, b)`'s `delta_mod10` is `value(a) - value(b)`, so it reads
    // as the shift induced by putting `a` where `b` was. Substituting `L` with
    // `P` is therefore `blindspot('P', 'L')` — the other order is the same
    // swap and equally `Caught`, but reports the complementary shift (6).
    assert_eq!(blindspot('P', 'L'), Blindspot::Caught { delta_mod10: 4 });
    assert_eq!(blindspot('C', '8'), Blindspot::Caught { delta_mod10: 4 });
    assert!(matches!(blindspot('L', 'P'), Blindspot::Caught { .. }));
    assert!(matches!(blindspot('8', 'C'), Blindspot::Caught { .. }));

    // Each one alone is duly rejected ...
    for single in [
        "P898902C36UTO7408122F1204159ZE184226B<<<<<10",
        "LC98902C36UTO7408122F1204159ZE184226B<<<<<10",
    ] {
        let doc = parse_td3(L1, single).unwrap();
        assert!(
            !doc.valid(),
            "a lone caught substitution must fail: {single}"
        );
        assert_eq!(doc.checks.document_number, Some(false));
    }

    // ... and together they are invisible.
    let both = "PC98902C36UTO7408122F1204159ZE184226B<<<<<10";
    let doc = parse_td3(L1, both).unwrap();
    assert_eq!(doc.document_number, "PC98902C3");
    assert_ne!(
        doc.document_number, "L898902C3",
        "the number really did change"
    );
    assert!(
        doc.valid(),
        "two caught substitutions at weights 7 and 3 cancel: 4*(7+3) = 40 = 0 (mod 10)"
    );
    assert_eq!(doc.checks.document_number, Some(true));
    assert_eq!(doc.checks.composite, Some(true));
}

/// The complement, so the two tests above cannot be read as "check digits do
/// nothing": the mechanism works, and works precisely.
#[test]
fn a_lone_caught_substitution_is_located_not_merely_detected() {
    let doc = parse_td3(L1, "M898902C36UTO7408122F1204159ZE184226B<<<<<10").unwrap();
    assert!(!doc.valid());
    assert_eq!(doc.checks.document_number, Some(false));
    assert_eq!(doc.checks.date_of_birth, Some(true));
    assert_eq!(doc.checks.date_of_expiry, Some(true));
    assert_eq!(doc.checks.composite, Some(false));
    assert_eq!(doc.checks.failed().len(), 2);
}
