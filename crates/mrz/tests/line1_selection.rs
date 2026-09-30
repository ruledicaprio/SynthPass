//! #574 (PR 4b): `select_line1`, the shadow line-1 selector.
//!
//! A two-line format's line 1 carries no check digit, so a misread name field
//! validates as readily as a correct one. `select_line1` looks in the same OCR
//! text for exactly one other line of the same width, document code and issuing
//! state whose name field is well formed, and *reports* it; it never applies
//! anything. Every case here is constructed from `format_td3`, `format_td2`,
//! `format_mrv_a` and `format_mrv_b` with invented names, so the file runs in a
//! clone with no images synced and holds no real document text.
//!
//! Each case builds the accepted read directly from a damaged line 1 and the
//! true line 2, instead of going through `find_and_parse`, so what is pinned is
//! the selector and not the scanner's choice among the lines of the text.

mod support;

use mrz::{
    find_and_parse_with, format_mrv_a, format_mrv_b, format_td1, format_td2, format_td3,
    parse_mrv_a_with, parse_mrv_b_with, parse_td1_with, parse_td2_with, parse_td3_with,
    select_line1, Format, Line1Selection, Line1Unresolved, Line1Verdict, MrvAFields, MrvBFields,
    MrzData, ParseOptions, Sex, Td1Fields, Td2Fields, Td3Fields,
};

const FORMATS: [Format; 4] = [Format::Td3, Format::Td2, Format::MrvA, Format::MrvB];

/// A zone with a well-formed line 1 (`good`) and its line 2, in one format.
struct Fixture {
    format: Format,
    good: String,
    line2: String,
}

impl Fixture {
    fn new(format: Format) -> Self {
        let birth = support::birth("800101");
        let expiry = support::expiry("301230");
        let zone = match format {
            Format::Td3 => format_td3(&Td3Fields {
                issuing_country: "UTO".to_string(),
                document_number: "L898902C".to_string(),
                surname: "SPECIMEN".to_string(),
                given_names: "TEST".to_string(),
                nationality: "UTO".to_string(),
                date_of_birth: birth,
                sex: Sex::Female,
                date_of_expiry: expiry,
                ..Td3Fields::default()
            }),
            Format::Td2 => format_td2(&Td2Fields {
                issuing_country: "UTO".to_string(),
                document_number: "L898902C".to_string(),
                surname: "SPECIMEN".to_string(),
                given_names: "TEST".to_string(),
                nationality: "UTO".to_string(),
                date_of_birth: birth,
                sex: Sex::Female,
                date_of_expiry: expiry,
                ..Td2Fields::default()
            }),
            Format::MrvA => format_mrv_a(&MrvAFields {
                issuing_country: "UTO".to_string(),
                document_number: "L898902C".to_string(),
                surname: "SPECIMEN".to_string(),
                given_names: "TEST".to_string(),
                nationality: "UTO".to_string(),
                date_of_birth: birth,
                sex: Sex::Female,
                date_of_expiry: expiry,
                ..MrvAFields::default()
            }),
            Format::MrvB => format_mrv_b(&MrvBFields {
                issuing_country: "UTO".to_string(),
                document_number: "L898902C".to_string(),
                surname: "SPECIMEN".to_string(),
                given_names: "TEST".to_string(),
                nationality: "UTO".to_string(),
                date_of_birth: birth,
                sex: Sex::Female,
                date_of_expiry: expiry,
                ..MrvBFields::default()
            }),
            other => panic!("no two-line fixture for {other:?}"),
        };
        let (good, line2) = zone.split_once('\n').expect("a two-line format");
        Self {
            format,
            good: good.to_string(),
            line2: line2.to_string(),
        }
    }

    fn width(&self) -> usize {
        self.good.len()
    }

    /// A line 1 with this fixture's document code and issuer and `name_field`
    /// after them, filler-padded to the format's width.
    fn line1(&self, name_field: &str) -> String {
        let mut line = format!("{}{name_field}", &self.good[..5]);
        assert!(line.len() <= self.width(), "name field too long: {line}");
        while line.len() < self.width() {
            line.push('<');
        }
        line
    }

    /// The incumbent: a `<<<<` run inside the name field breaks the grammar, and
    /// nothing else about the line is wrong.
    fn broken_line1(&self) -> String {
        self.line1("SPECIMEN<<<<TEST")
    }

    /// The accepted read for `line1` and this fixture's line 2.
    fn parse(&self, line1: &str) -> MrzData {
        let opts = ParseOptions::default();
        let parsed = match self.format {
            Format::Td3 => parse_td3_with(line1, &self.line2, &opts),
            Format::Td2 => parse_td2_with(line1, &self.line2, &opts),
            Format::MrvA => parse_mrv_a_with(line1, &self.line2, &opts),
            Format::MrvB => parse_mrv_b_with(line1, &self.line2, &opts),
            other => panic!("no two-line fixture for {other:?}"),
        };
        parsed.expect("the fixture lines parse")
    }

    /// The text of an OCR run that read `lines` as its line 1s, each followed by
    /// the true line 2, separated by a blank line.
    fn text(&self, line1s: &[&str]) -> String {
        line1s
            .iter()
            .map(|l1| format!("{l1}\n{}", self.line2))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

fn select(text: &str, accepted: &MrzData) -> Line1Selection {
    select_line1(text, accepted, &ParseOptions::default())
}

fn proposed(selection: Line1Selection) -> MrzData {
    match selection.verdict {
        Line1Verdict::Proposed(data) => data,
        other => panic!("expected a proposal, got {other:?}"),
    }
}

/// `data` with the incumbent's name fields and line 1: what a proposal must
/// equal, field for field.
fn with_names_of(data: &MrzData, incumbent: &MrzData) -> MrzData {
    let mut probe = data.clone();
    probe.surname = incumbent.surname.clone();
    probe.given_names = incumbent.given_names.clone();
    probe.mrz_lines = incumbent.mrz_lines.clone();
    probe
}

#[test]
fn the_incumbent_fixture_is_valid_and_ungrammatical() {
    for format in FORMATS {
        let fx = Fixture::new(format);
        let accepted = fx.parse(&fx.broken_line1());
        assert!(accepted.valid(), "{format:?}");
        assert_eq!(accepted.issuing_country, "UTO", "{format:?}");
        assert_eq!(fx.broken_line1().len(), fx.width(), "{format:?}");
    }
}

/// One applied case per format: widths 44, 36, 44, 36.
#[test]
fn a_single_well_formed_line_of_the_same_width_is_proposed() {
    for format in FORMATS {
        let fx = Fixture::new(format);
        let broken = fx.broken_line1();
        let accepted = fx.parse(&broken);
        let text = fx.text(&[&broken, &fx.good]);

        let selection = select(&text, &accepted);
        assert_eq!(selection.eligible, 1, "{format:?}");
        assert_eq!(selection.distinct, 1, "{format:?}");
        let data = proposed(selection);

        assert_eq!(data.surname, "SPECIMEN", "{format:?}");
        assert_eq!(data.given_names, "TEST", "{format:?}");
        assert_eq!(
            data.mrz_lines,
            format!("{}\n{}", fx.good, fx.line2),
            "{format:?}"
        );
        assert!(data.valid(), "{format:?}");
        assert_eq!(
            with_names_of(&data, &accepted),
            accepted,
            "{format:?}: only the names and line 1 may differ"
        );
        // The proposal is the same read as parsing the good line directly.
        assert_eq!(data, fx.parse(&fx.good), "{format:?}");
    }
}

/// The same reading three times is one distinct name field, not three.
#[test]
fn a_reading_seen_in_several_passes_is_one_distinct_name_field() {
    for format in FORMATS {
        let fx = Fixture::new(format);
        let broken = fx.broken_line1();
        let accepted = fx.parse(&broken);
        let text = fx.text(&[&broken, &fx.good, &fx.good, &fx.good]);

        let selection = select(&text, &accepted);
        assert_eq!(
            (selection.eligible, selection.distinct),
            (3, 1),
            "{format:?}"
        );
        assert_eq!(proposed(selection).surname, "SPECIMEN", "{format:?}");
    }
}

/// Two distinct well-formed name fields: nothing is picked.
#[test]
fn two_distinct_eligible_name_fields_are_ambiguous() {
    for format in FORMATS {
        let fx = Fixture::new(format);
        let broken = fx.broken_line1();
        let accepted = fx.parse(&broken);
        let other = fx.line1("SPECIMEN<<TESTY");
        let text = fx.text(&[&broken, &fx.good, &other]);

        let selection = select(&text, &accepted);
        assert_eq!(selection.verdict, Line1Verdict::Ambiguous, "{format:?}");
        assert_eq!(
            (selection.eligible, selection.distinct),
            (2, 2),
            "{format:?}"
        );
        // The accepted read is not touched.
        assert_eq!(accepted.surname, "SPECIMEN", "{format:?}");
    }
}

#[test]
fn a_question_mark_makes_a_line_ineligible() {
    for format in FORMATS {
        let fx = Fixture::new(format);
        let broken = fx.broken_line1();
        let accepted = fx.parse(&broken);
        let unreadable = fx.line1("SPECIMEN<<TE?T");
        let selection = select(&fx.text(&[&broken, &unreadable]), &accepted);
        assert_eq!(selection.verdict, Line1Verdict::NoCandidate, "{format:?}");
        assert_eq!(
            (selection.eligible, selection.distinct),
            (0, 0),
            "{format:?}"
        );
    }
}

/// One cell narrower or wider than the format: no padding, trimming or repair.
#[test]
fn a_line_one_cell_off_the_format_width_is_ineligible() {
    for format in FORMATS {
        let fx = Fixture::new(format);
        let broken = fx.broken_line1();
        let accepted = fx.parse(&broken);

        let mut narrow = fx.good.clone();
        narrow.pop();
        assert_eq!(narrow.len(), fx.width() - 1);
        let wide = format!("{}<", fx.good);
        assert_eq!(wide.len(), fx.width() + 1);

        for candidate in [&narrow, &wide] {
            let selection = select(&fx.text(&[&broken, candidate]), &accepted);
            assert_eq!(
                selection.verdict,
                Line1Verdict::NoCandidate,
                "{format:?}: {candidate}"
            );
            assert_eq!(selection.eligible, 0, "{format:?}: {candidate}");
        }
    }
}

/// A different issuing state or document code is a different document.
#[test]
fn a_different_issuer_or_document_code_is_ineligible() {
    for format in FORMATS {
        let fx = Fixture::new(format);
        let broken = fx.broken_line1();
        let accepted = fx.parse(&broken);

        let other_issuer = format!("{}UTP{}", &fx.good[..2], &fx.good[5..]);
        let mut other_code = fx.good.clone();
        other_code.replace_range(1..2, "D");
        assert_ne!(other_code, fx.good);

        for candidate in [&other_issuer, &other_code] {
            let selection = select(&fx.text(&[&broken, candidate]), &accepted);
            assert_eq!(
                selection.verdict,
                Line1Verdict::NoCandidate,
                "{format:?}: {candidate}"
            );
        }
    }
}

#[test]
fn a_digit_in_the_name_field_is_ineligible() {
    for format in FORMATS {
        let fx = Fixture::new(format);
        let broken = fx.broken_line1();
        let accepted = fx.parse(&broken);
        let digit = fx.line1("SPEC1MEN<<TEST");
        let selection = select(&fx.text(&[&broken, &digit]), &accepted);
        assert_eq!(selection.verdict, Line1Verdict::NoCandidate, "{format:?}");
    }
}

/// A grammatical incumbent is never replaced, even when the text holds a
/// different grammatical line 1.
#[test]
fn a_well_formed_incumbent_is_kept() {
    for format in FORMATS {
        let fx = Fixture::new(format);
        let accepted = fx.parse(&fx.good);
        let other = fx.line1("SPECIMEN<<TESTY");
        let selection = select(&fx.text(&[&fx.good, &other]), &accepted);
        assert_eq!(selection.verdict, Line1Verdict::Kept, "{format:?}");
        assert_eq!(
            (selection.eligible, selection.distinct),
            (0, 0),
            "{format:?}"
        );
    }
}

/// A single-word surname with no given names and no `<<` is grammatical: the
/// Djibouti caution, an issuer that prints a single filler.
#[test]
fn a_name_field_with_no_double_filler_is_well_formed() {
    let fx = Fixture::new(Format::Td3);
    let accepted = fx.parse(&fx.line1("HASSAN<FARID<RAGUE"));
    assert_eq!(
        select(&fx.text(&[&fx.good]), &accepted).verdict,
        Line1Verdict::Kept
    );
}

#[test]
fn td1_is_out_of_scope() {
    let zone = format_td1(&Td1Fields {
        issuing_country: "UTO".to_string(),
        document_number: "L898902C".to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("301230"),
        ..Td1Fields::default()
    });
    let mut lines = zone.lines();
    let (l1, l2, l3) = (
        lines.next().expect("line 1"),
        lines.next().expect("line 2"),
        lines.next().expect("line 3"),
    );
    let accepted = parse_td1_with(l1, l2, l3, &ParseOptions::default()).expect("parses");
    assert!(accepted.valid());
    let selection = select(&zone, &accepted);
    assert_eq!(selection.verdict, Line1Verdict::OutOfScope);
    assert_eq!((selection.eligible, selection.distinct), (0, 0));
}

#[test]
fn a_read_whose_check_digits_fail_is_out_of_scope() {
    let fx = Fixture::new(Format::Td3);
    let broken = fx.broken_line1();
    // One cell of the expiry date altered: still an `Ok`, no longer valid.
    let mut line2 = fx.line2.clone();
    let expiry_cell = 21;
    let altered = if line2.as_bytes()[expiry_cell] == b'7' {
        "8"
    } else {
        "7"
    };
    line2.replace_range(expiry_cell..expiry_cell + 1, altered);
    let accepted = parse_td3_with(&broken, &line2, &ParseOptions::default()).expect("parses");
    assert!(!accepted.valid());

    let text = format!("{broken}\n{line2}\n\n{}\n{line2}", fx.good);
    assert_eq!(select(&text, &accepted).verdict, Line1Verdict::OutOfScope);
}

/// The zone is flagged for a wrong physical line, so its name field is not
/// what to change: no candidate is looked at.
#[test]
fn an_unresolved_issuer_is_unresolved() {
    let birth = support::birth("800101");
    let expiry = support::expiry("301230");
    let zone = format_td3(&Td3Fields {
        issuing_country: "QQQ".to_string(),
        document_number: "CD987654".to_string(),
        surname: "SPECIMEN".to_string(),
        given_names: "TEST".to_string(),
        nationality: "QQQ".to_string(),
        date_of_birth: birth,
        sex: Sex::Female,
        date_of_expiry: expiry,
        ..Td3Fields::default()
    });
    let (good, line2) = zone.split_once('\n').expect("two lines");
    let broken = format!("{}SPECIMEN<<<<TEST{}", &good[..5], "<".repeat(44 - 21));
    assert_eq!(broken.len(), 44);
    let accepted = parse_td3_with(&broken, line2, &ParseOptions::default()).expect("parses");
    assert!(accepted.valid());

    let selection = select(&format!("{broken}\n{line2}\n\n{good}\n{line2}"), &accepted);
    assert_eq!(
        selection.verdict,
        Line1Verdict::Unresolved(Line1Unresolved::IssuerUnresolved)
    );
    assert_eq!((selection.eligible, selection.distinct), (0, 0));
}

#[test]
fn a_digit_in_the_accepted_name_field_is_unresolved() {
    let fx = Fixture::new(Format::Td3);
    let digit = fx.line1("SPEC1MEN<<TEST");
    let accepted = fx.parse(&digit);
    let selection = select(&fx.text(&[&digit, &fx.good]), &accepted);
    assert_eq!(
        selection.verdict,
        Line1Verdict::Unresolved(Line1Unresolved::DigitInNameField)
    );
}

/// A "line 1" that is line 2 read a second time.
#[test]
fn a_repeated_line_is_unresolved() {
    let fx = Fixture::new(Format::Td3);
    let repeated = format!("{}{}", &fx.good[..5], &fx.line2[5..]);
    let accepted = fx.parse(&repeated);
    assert!(accepted.valid());
    let selection = select(&fx.text(&[&repeated, &fx.good]), &accepted);
    assert_eq!(
        selection.verdict,
        Line1Verdict::Unresolved(Line1Unresolved::RepeatedLine)
    );
}

/// With the opt-in refusal on, no returned zone is a repeated line, so the
/// selector can no longer reach `Unresolved(RepeatedLine)` from a scan's read.
#[test]
fn under_refusal_no_returned_zone_is_a_repeated_line() {
    let fx = Fixture::new(Format::Td3);
    let repeated = format!("{}{}", &fx.good[..5], &fx.line2[5..]);
    let refusing = ParseOptions::default().with_refuse_repeated_line(true);
    let texts = [
        fx.text(&[&repeated]),
        fx.text(&[&repeated, &fx.good]),
        fx.text(&[&repeated, &repeated]),
    ];
    for text in &texts {
        // Off, the scan's read is the repeat and the selector reports it.
        let off = find_and_parse_with(text, &ParseOptions::default()).expect("parses");
        let verdict = select_line1(text, &off, &ParseOptions::default()).verdict;
        let repeated_off = matches!(
            verdict,
            Line1Verdict::Unresolved(Line1Unresolved::RepeatedLine)
        );
        // On, whatever comes back never yields it.
        if let Ok(on) = find_and_parse_with(text, &refusing) {
            let verdict = select_line1(text, &on, &refusing).verdict;
            assert!(
                !matches!(
                    verdict,
                    Line1Verdict::Unresolved(Line1Unresolved::RepeatedLine)
                ),
                "{text}"
            );
            if repeated_off {
                assert_ne!(on, off, "{text}");
            }
        }
    }
}

/// Applying the proposal and asking again finds nothing to do.
#[test]
fn selecting_on_its_own_output_is_kept() {
    for format in FORMATS {
        let fx = Fixture::new(format);
        let broken = fx.broken_line1();
        let accepted = fx.parse(&broken);
        let text = fx.text(&[&broken, &fx.good]);

        let applied = proposed(select(&text, &accepted));
        let again = select(&text, &applied);
        assert_eq!(again.verdict, Line1Verdict::Kept, "{format:?}");
    }
}

/// Order-independent: the verdict, and the proposal, do not depend on which
/// order the OCR passes' lines come in.
#[test]
fn reordering_the_lines_leaves_the_verdict_unchanged() {
    for format in FORMATS {
        let fx = Fixture::new(format);
        let broken = fx.broken_line1();
        let accepted = fx.parse(&broken);
        let other = fx.line1("SPECIMEN<<TESTY");
        let unreadable = fx.line1("SPECIMEN<<TE?T");

        // A proposal, with the good line read twice.
        let a = select(
            &fx.text(&[&broken, &fx.good, &unreadable, &fx.good]),
            &accepted,
        );
        let b = select(
            &fx.text(&[&fx.good, &unreadable, &fx.good, &broken]),
            &accepted,
        );
        let c = select(
            &fx.text(&[&unreadable, &fx.good, &broken, &fx.good]),
            &accepted,
        );
        assert_eq!(a, b, "{format:?}");
        assert_eq!(a, c, "{format:?}");
        assert_eq!(proposed(a).surname, "SPECIMEN", "{format:?}");

        // Ambiguity.
        let a = select(&fx.text(&[&broken, &fx.good, &other]), &accepted);
        let b = select(&fx.text(&[&other, &broken, &fx.good]), &accepted);
        assert_eq!(a, b, "{format:?}");
        assert_eq!(a.verdict, Line1Verdict::Ambiguous, "{format:?}");
    }
}

/// `damaged_recovery` describes how the accepted read was reached, and the
/// proposal is that read with new names, so it carries over.
#[test]
fn the_proposal_keeps_the_accepted_reads_damaged_recovery_flag() {
    let fx = Fixture::new(Format::Td3);
    let broken = fx.broken_line1();
    let mut accepted = fx.parse(&broken);
    accepted.damaged_recovery = true;
    let data = proposed(select(&fx.text(&[&broken, &fx.good]), &accepted));
    assert!(data.damaged_recovery);
    assert_eq!(with_names_of(&data, &accepted), accepted);
}

/// The whole zone can arrive as one physical line, the shape the scanner's own
/// line walk splits: the selector sees the same lines the scanner does.
#[test]
fn candidates_come_from_the_scanners_line_walk() {
    let fx = Fixture::new(Format::Td3);
    let broken = fx.broken_line1();
    let accepted = fx.parse(&broken);
    // Both lines of the second pass on one line, space-separated, with the
    // filler escaped the way a Markdown pipeline writes it.
    let escaped = format!("{} {}", fx.good, fx.line2).replace('<', "&lt;");
    let text = format!("{broken}\n{}\n\n{escaped}", fx.line2);
    let data = proposed(select(&text, &accepted));
    assert_eq!(data.mrz_lines, format!("{}\n{}", fx.good, fx.line2));
}

/// `MrzData`'s fields are public, so `mrz_lines` can be edited after parsing.
/// Anything but two lines of exactly the format's width is out of scope, and
/// is refused before the wrong-line similarity check ever sees it: that check
/// does work quadratic in the line length, so an enormous edited line must not
/// reach it. An out-of-scope verdict for a zone whose name field is ill-formed
/// and whose issuer resolves also shows the guard runs ahead of every other
/// step, since any later step would have proposed or flagged it.
#[test]
fn a_zone_edited_after_parsing_is_out_of_scope() {
    for format in FORMATS {
        let fx = Fixture::new(format);
        let broken = fx.broken_line1();
        let accepted = fx.parse(&broken);
        let text = fx.text(&[&broken, &fx.good]);
        let width = fx.width();
        // Sanity: unedited, the same text yields a proposal.
        assert!(matches!(
            select(&text, &accepted).verdict,
            Line1Verdict::Proposed(_)
        ));

        let huge = format!("{}{}", &broken[..5], "A<".repeat(50_000));
        let edits = [
            String::new(),                                     // no lines
            broken.clone(),                                    // one line
            format!("{broken}\n{}\n{}", fx.line2, fx.line2),   // three lines
            format!("{}\n{}", &broken[..width - 1], fx.line2), // line 1 too short
            format!("{broken}<\n{}", fx.line2),                // line 1 too long
            format!("{broken}\n{}<", fx.line2),                // line 2 too long
            format!("{huge}\n{}", fx.line2),                   // unbounded line 1
            format!("{broken}\n{huge}"),                       // unbounded line 2
        ];
        for mrz_lines in edits {
            let mut edited = accepted.clone();
            edited.mrz_lines = mrz_lines.clone();
            let selection = select(&text, &edited);
            assert_eq!(
                selection.verdict,
                Line1Verdict::OutOfScope,
                "{format:?}: {:?}",
                mrz_lines.chars().take(60).collect::<String>()
            );
            assert_eq!(
                (selection.eligible, selection.distinct),
                (0, 0),
                "{format:?}"
            );
        }
    }
}
