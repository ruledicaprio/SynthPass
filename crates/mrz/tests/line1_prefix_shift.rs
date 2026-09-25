//! Regressions for `unshift_line1_prefix` (`crates/mrz/src/parser.rs`),
//! wired into `repair_td3_line1`/`repair_mrv_line1`.
//!
//! `synthpass-bench --dump-ocr` (`knowledge/ROADMAP.md`'s M6 note) showed
//! OCR dropping line 1's position-1 filler outright — `P<BRAESKANDARI...`
//! read as `PBRAESKANDARI...` — for TD3 and MRV-B alike, the same mechanism
//! `crates/mrz/tests/td1_line_gap.rs`'s
//! `a_dropped_line_1_filler_position_is_recovered` already pins for TD1.
//! Unlike TD1, none of TD3/MRV-A/MRV-B carry a check digit on line 1, so the
//! shift never fails a Tier-1 hit — it silently corrupts `document_type`/
//! `issuing_country`/the name behind a passing read instead, which is why
//! `synthpass-bench`'s per-field CER (not a hit-rate drop) is what surfaced
//! it.
//!
//! **Fixtures are emitted, not transcribed**, same discipline
//! `tests/td1_line_gap.rs` and `tests/name_separator_collapse.rs` follow.

mod support;

use mrz::{
    find_and_parse, format_mrv_a, format_mrv_b, format_td2, format_td3, Format, MrvAFields,
    MrvBFields, Td2Fields, Td3Fields,
};

/// Drop line 1's position-1 filler, reproducing the measured OCR failure —
/// the line is now one character short, the same shape `find_and_parse`'s
/// `variants()`/`fit_length` machinery has to recover from in production
/// (it pads the deficit back at the *end*, which is exactly what leaves the
/// prefix shifted unless `unshift_line1_prefix` corrects it).
fn drop_position_1(line1: &str) -> String {
    let mut damaged = line1.to_string();
    damaged.remove(1);
    damaged
}

#[test]
fn td3_recovers_document_type_and_issuing_country_after_a_dropped_position_1_filler() {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E000000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("301230"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");

    let damaged = drop_position_1(l1);
    let text = format!("{damaged}\n{l2}");
    let data = find_and_parse(&text).expect("a dropped position-1 filler must still parse");

    assert_eq!(data.format, Format::Td3);
    assert_eq!(data.document_type, "P");
    assert_eq!(data.issuing_country, "BRA");
    assert!(data.valid(), "line 2's real check digits are untouched");
}

#[test]
fn mrv_a_recovers_document_type_and_issuing_country_after_a_dropped_position_1_filler() {
    let mrz = format_mrv_a(&MrvAFields {
        document_code: "V".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E00000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("301230"),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_mrv_a emits two lines");

    let damaged = drop_position_1(l1);
    let text = format!("{damaged}\n{l2}");
    let data = find_and_parse(&text).expect("a dropped position-1 filler must still parse");

    assert_eq!(data.format, Format::MrvA);
    assert_eq!(data.document_type, "V");
    assert_eq!(data.issuing_country, "BRA");
    assert!(data.valid());
}

#[test]
fn mrv_b_recovers_document_type_and_issuing_country_after_a_dropped_position_1_filler() {
    let mrz = format_mrv_b(&MrvBFields {
        document_code: "V".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E0000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("301230"),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_mrv_b emits two lines");

    let damaged = drop_position_1(l1);
    let text = format!("{damaged}\n{l2}");
    let data = find_and_parse(&text).expect("a dropped position-1 filler must still parse");

    assert_eq!(data.format, Format::MrvB);
    assert_eq!(data.document_type, "V");
    assert_eq!(data.issuing_country, "BRA");
    assert!(data.valid());
}

/// The guard case: an undamaged zone (position 1 genuinely already `<`)
/// must round-trip unchanged — `unshift_line1_prefix` must never fire when
/// there's nothing to unshift.
#[test]
fn an_undamaged_td3_zone_is_not_spuriously_unshifted() {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E000000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("301230"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");

    let data = find_and_parse(&format!("{l1}\n{l2}")).expect("emitted TD3 parses");
    assert_eq!(data.document_type, "P");
    assert_eq!(data.issuing_country, "BRA");
    assert!(data.valid());
}

/// TD2 shares TD1's ICAO ID-document-family code space and can legitimately
/// carry a genuine two-real-letter document code — `unshift_line1_prefix`
/// is deliberately *not* wired into `repair_td2_line1` (see its doc
/// comment) precisely so a fixture like this one is never corrupted by a
/// future change that naively extends the TD3/MRV-A/MRV-B fix to TD2 too.
/// This pins the current, correct behavior as a regression guard.
#[test]
fn td2_with_a_genuine_two_letter_document_code_is_unaffected() {
    let mrz = format_td2(&Td2Fields {
        document_code: "IP".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E00000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("301230"),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td2 emits two lines");

    let data = find_and_parse(&format!("{l1}\n{l2}")).expect("emitted TD2 parses");
    assert_eq!(data.format, Format::Td2);
    assert_eq!(
        data.document_type, "IP",
        "a genuine two-letter document code must survive untouched"
    );
    assert_eq!(data.issuing_country, "BRA");
}

/// Germany's legacy MRZ issuing-state code is the single letter `D`, which the
/// MRZ pads to `D<<`. Recovering a dropped position-1 filler on such a document
/// requires the repair gate to look the state up *after* trimming that padding.
///
/// It did not. All three gates passed the raw 3-byte slice to `country_name`,
/// which is exact string equality against a table storing `("D", "Germany")`,
/// so `country_name("D<<")` was `None` and the correct repair was produced and
/// then discarded. Nothing caught it: the synthetic generator only ever emits
/// `DEU`, and no corpus specimen exercised the path until
/// `Germany_Passport_Specimen_P0_D00_2018_mrz.webp` — whose line 1 reads
/// `P<D<<HEINKEL<<REYNALD` — was read against the manifest.
#[test]
fn td3_recovers_a_filler_padded_single_letter_issuing_state() {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "D".to_string(),
        document_number: "G28P5FV81".to_string(),
        surname: "HEINKEL".to_string(),
        given_names: "REYNALD".to_string(),
        nationality: "D".to_string(),
        date_of_birth: support::birth("980628"),
        sex: mrz::Sex::Male,
        date_of_expiry: support::expiry("281201"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");
    assert!(
        l1.starts_with("P<D<<"),
        "the emitter must filler-pad a single-letter state: {l1}"
    );

    let damaged = drop_position_1(l1);
    let text = format!("{damaged}\n{l2}");
    let data =
        find_and_parse(&text).expect("a dropped position-1 filler must still parse for `D<<`");

    assert_eq!(data.format, Format::Td3);
    assert_eq!(data.document_type, "P");
    assert_eq!(
        data.issuing_country, "D",
        "the repair gate must resolve `D<<` to Germany, not discard the repair"
    );
    assert_eq!(data.issuing_country_name(), Some("Germany"));
    assert!(data.valid(), "line 2's real check digits are untouched");
}

/// The trimming above must not make an all-filler state resolve: `<<<` trims to
/// the empty string, which is not in the table, so a genuinely damaged zone is
/// still left alone rather than "repaired" into nonsense.
#[test]
fn an_all_filler_issuing_state_still_does_not_resolve() {
    assert_eq!(mrz::country_name(""), None);
    assert_eq!(mrz::country_name("<<<"), None);
}

/// #445: a genuine Part 4 §4.4 document code collides with `unshift_line1_prefix`
/// whenever the code's second letter plus the issuer's first two letters also
/// spell a real ISO/ICAO three-letter state. `unshift_if_country_resolves`
/// only ever checked whether the *unshifted* reading's issuer resolved — never
/// whether the as-read line already had a resolving code and issuer of its
/// own — so a genuine Nigerian `PP` passport (`PPNGA...`) was silently
/// rewritten to a `P<PNG...` (Papua New Guinea) reading. `PP`+`RKS` (Kosovo,
/// whose unshifted form spells `PRK`, North Korea), `PD`+`ZAF` (unshifts to
/// `DZA`, Algeria), `PS`+`AUT` (unshifts to `SAU`, Saudi Arabia) and `PR`+`USA`
/// (unshifts to `RUS`, Russia) are four more of the 67 colliding pairs the
/// architect enumerated against `countries.rs`. Every one of these must
/// survive `find_and_parse` exactly as printed.
///
/// Written first and confirmed to fail on the unfixed code (see the PR/commit
/// this test shipped with): before `td3_line1_is_genuine_table_code`,
/// `PPNGA...` read back as document type `P` / issuer `PNG`.
#[test]
fn genuine_table_code_whose_unshift_collides_is_kept() {
    for (code, issuer) in [
        ("PP", "NGA"),
        ("PP", "RKS"),
        ("PD", "ZAF"),
        ("PS", "AUT"),
        ("PR", "USA"),
    ] {
        let mrz = format_td3(&Td3Fields {
            document_code: code.to_string(),
            issuing_country: issuer.to_string(),
            document_number: "E000000000".to_string(),
            surname: "OKAFOR".to_string(),
            given_names: "ADA".to_string(),
            nationality: issuer.to_string(),
            date_of_birth: support::birth("800101"),
            sex: mrz::Sex::Female,
            date_of_expiry: support::expiry("301230"),
            personal_number: None,
        });
        let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");

        let data = find_and_parse(&format!("{l1}\n{l2}"))
            .unwrap_or_else(|e| panic!("{code}+{issuer} must parse: {e:?}"));
        assert_eq!(
            data.document_type, code,
            "{code}+{issuer}: document code must survive as printed"
        );
        assert_eq!(
            data.issuing_country, issuer,
            "{code}+{issuer}: issuer must survive as printed, not the unshifted collision"
        );
        assert!(
            data.valid(),
            "{code}+{issuer}: line 2's check digits are untouched"
        );
    }
}

/// `PS`/`PO`/`PE` (stateless, official, emergency) are genuine §4.4 codes
/// whose unshifted reading does *not* happen to collide with a real country
/// (`BRA`'s first two letters paired with each code's second letter spell
/// `SBR`/`OBR`/`EBR`, none of which resolve) — pinning that the new guard
/// keeps them for the ordinary reason (a genuine table code plus a resolving
/// issuer), not merely because the collision list happens to be empty here.
#[test]
fn genuine_ps_po_pe_are_kept() {
    for code in ["PS", "PO", "PE"] {
        let mrz = format_td3(&Td3Fields {
            document_code: code.to_string(),
            issuing_country: "BRA".to_string(),
            document_number: "E000000000".to_string(),
            surname: "ESKANDARI".to_string(),
            given_names: "MAREN".to_string(),
            nationality: "BRA".to_string(),
            date_of_birth: support::birth("800101"),
            sex: mrz::Sex::Female,
            date_of_expiry: support::expiry("301230"),
            personal_number: None,
        });
        let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");

        let data = find_and_parse(&format!("{l1}\n{l2}")).expect("emitted TD3 parses");
        assert_eq!(data.document_type, code);
        assert_eq!(data.issuing_country, "BRA");
        assert!(data.valid());
    }
}

/// Status quo, unchanged by the #445 fix: a genuinely dropped position-1
/// filler (`P<GBR...` read as `PGBR...`) must still be unshifted back, since
/// `PG` is not a §4.4 table code and does not pass
/// `td3_line1_is_genuine_table_code`'s first check.
#[test]
fn dropped_filler_is_still_unshifted() {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "GBR".to_string(),
        document_number: "E000000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "GBR".to_string(),
        date_of_birth: support::birth("800101"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("301230"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");
    assert!(l1.starts_with("P<GBR"), "sanity: emitted line 1 is {l1}");

    let damaged = drop_position_1(l1);
    let text = format!("{damaged}\n{l2}");
    let data = find_and_parse(&text).expect("a dropped position-1 filler must still parse");

    assert_eq!(data.format, Format::Td3);
    assert_eq!(data.document_type, "P");
    assert_eq!(data.issuing_country, "GBR");
    assert!(data.valid());
}

/// **Known limitation**, pinned honestly rather than hidden: a genuine `P<RUS`
/// whose surname starts with `A` and whose position-1 filler is dropped reads
/// as `PRUSA...`, which is byte-for-byte indistinguishable from a genuine `PR`
/// (refugee passport) issued by `USA`. `td3_line1_is_genuine_table_code`
/// cannot tell the two apart — both are a real §4.4 code plus a resolving
/// issuer on the as-read line — so this reading is now kept as `PR`/`USA`
/// rather than unshifted back to `P<`/`RUS`. Nothing on line 1 disambiguates
/// this case; only a name/photo cross-check outside this crate's scope could.
#[test]
fn known_limitation_dropped_filler_colliding_with_a_genuine_code_is_kept_as_the_code() {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "RUS".to_string(),
        document_number: "E000000000".to_string(),
        surname: "ALEKSANDROV".to_string(),
        given_names: "IVAN".to_string(),
        nationality: "RUS".to_string(),
        date_of_birth: support::birth("800101"),
        sex: mrz::Sex::Male,
        date_of_expiry: support::expiry("301230"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");
    assert!(
        l1.starts_with("P<RUSALEKSANDROV"),
        "sanity: emitted line 1 is {l1}"
    );

    let damaged = drop_position_1(l1);
    let text = format!("{damaged}\n{l2}");
    let data = find_and_parse(&text).expect("still parses, just to the wrong reading");

    assert_eq!(data.format, Format::Td3);
    assert_eq!(
        data.document_type, "PR",
        "known limitation: indistinguishable from a genuine PR+USA on line 1 alone"
    );
    assert_eq!(data.issuing_country, "USA");
    assert!(
        data.valid(),
        "the collision reading still validates on line 2's real check digits"
    );
}
