//! BiH/ICAO MRZ conformance audit helper for SynthPass.
//!
//! Intended repository location:
//!   crates/mrz/examples/bih_mrz_audit.rs
//!
//! Run from the SynthPass repository root:
//!   cargo run -p mrz --example bih_mrz_audit -- --profile auto sample.txt
//!   cat sample.txt | cargo run -p mrz --example bih_mrz_audit -- --profile bih-id
//!
//! Design:
//! - The existing `mrz` crate remains the parser.
//! - This tool independently recomputes ICAO 9303 7-3-1 check digits from the
//!   canonical parsed MRZ lines, so it can catch regressions instead of merely
//!   asking the parser whether the parser agrees with itself.
//! - BiH-specific rules are a separate policy layer.
//! - Observational claims (for example, a recurring `<<<` prefix in TD1
//!   optional data) are never promoted to normative ICAO rules.

use mrz::{find_and_parse, Format, MrzData};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    fn label(self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warning => "WARN ",
            Self::Info => "INFO ",
        }
    }
}

#[derive(Debug, Clone)]
struct Finding {
    severity: Severity,
    rule: &'static str,
    message: String,
}

impl Finding {
    fn error(rule: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            rule,
            message: message.into(),
        }
    }

    fn warning(rule: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            rule,
            message: message.into(),
        }
    }

    fn info(rule: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            rule,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Profile {
    Auto,
    IcaoOnly,
    BihIdentityCard,
    BihPassport,
}

impl Profile {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "icao" | "icao-only" => Ok(Self::IcaoOnly),
            "bih-id" | "bih-identity-card" => Ok(Self::BihIdentityCard),
            "bih-passport" => Ok(Self::BihPassport),
            _ => Err(format!(
                "unknown profile {value:?}; expected auto, icao, bih-id, or bih-passport"
            )),
        }
    }
}

#[derive(Debug)]
struct Args {
    profile: Profile,
    observed_bih_td1_fillers: bool,
    path: Option<String>,
}

fn usage() -> &'static str {
    "\
BiH/ICAO MRZ conformance audit

USAGE:
  cargo run -p mrz --example bih_mrz_audit -- [OPTIONS] [FILE]

OPTIONS:
  --profile auto|icao|bih-id|bih-passport
      auto          Apply ICAO checks, then infer a BiH profile only when the
                    parsed issuing state is BIH.
      icao           Apply ICAO/reference checks only.
      bih-id         Require the document to look like a BiH TD1 identity card.
      bih-passport   Require the document to look like a BiH TD3 passport.

  --observed-bih-td1-fillers
      Check whether TD1 optional-data positions 16-18 begin with <<<.
      IMPORTANT: this is an observational/corpus rule, not an ICAO conformance
      requirement and not treated as an error.

  -h, --help
      Show this help.

INPUT:
  FILE may contain clean MRZ lines or surrounding OCR text. If FILE is omitted,
  stdin is read.
"
}

fn parse_args() -> Result<Args, String> {
    let mut profile = Profile::Auto;
    let mut observed_bih_td1_fillers = false;
    let mut path = None;

    let mut it = env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{}", usage());
                std::process::exit(0);
            }
            "--profile" => {
                let value = it
                    .next()
                    .ok_or_else(|| "--profile requires a value".to_string())?;
                profile = Profile::parse(&value)?;
            }
            "--observed-bih-td1-fillers" => {
                observed_bih_td1_fillers = true;
            }
            _ if arg.starts_with('-') => {
                return Err(format!("unknown option {arg:?}\n\n{}", usage()));
            }
            _ => {
                if path.replace(arg).is_some() {
                    return Err(format!("only one input FILE is supported\n\n{}", usage()));
                }
            }
        }
    }

    Ok(Args {
        profile,
        observed_bih_td1_fillers,
        path,
    })
}

fn read_input(path: Option<&str>) -> io::Result<String> {
    match path {
        Some(path) => fs::read_to_string(path),
        None => {
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            Ok(input)
        }
    }
}

/// Independent ICAO 9303 check-digit implementation.
///
/// ICAO's algorithm is:
/// - `0..9` -> 0..9
/// - `A..Z` -> 10..35
/// - `<` -> 0
/// - weights repeat 7, 3, 1 from LEFT TO RIGHT
/// - the check digit is `sum % 10`
///
/// A filler contributes zero, but still consumes its position in the weight
/// sequence. Do not skip fillers and do not reverse the field.
fn reference_check_digit(field: &str) -> Result<u8, String> {
    const WEIGHTS: [u32; 3] = [7, 3, 1];

    let mut sum = 0u32;
    for (index, byte) in field.bytes().enumerate() {
        let value = match byte {
            b'0'..=b'9' => u32::from(byte - b'0'),
            b'A'..=b'Z' => u32::from(byte - b'A') + 10,
            b'<' => 0,
            _ => {
                return Err(format!(
                    "non-MRZ byte {:?} at field offset {index}",
                    byte as char
                ))
            }
        };

        sum += value * WEIGHTS[index % WEIGHTS.len()];
    }

    Ok((sum % 10) as u8)
}

fn printed_check_value(ch: char) -> Option<u8> {
    match ch {
        '0'..='9' => Some(ch as u8 - b'0'),
        '<' => Some(0),
        _ => None,
    }
}

fn check_reference_field(
    findings: &mut Vec<Finding>,
    rule: &'static str,
    field_name: &'static str,
    field: &str,
    printed: char,
) {
    let reference = match reference_check_digit(field) {
        Ok(value) => value,
        Err(err) => {
            findings.push(Finding::error(
                rule,
                format!("{field_name}: reference algorithm could not evaluate field: {err}"),
            ));
            return;
        }
    };

    // Cross-check the public crate helper against an independently written
    // implementation. This is intentionally redundant.
    match mrz::check_digit(field) {
        Ok(crate_value) if crate_value == u32::from(reference) => {}
        Ok(crate_value) => findings.push(Finding::error(
            "ENGINE-CD-DIVERGENCE",
            format!(
                "{field_name}: mrz::check_digit returned {crate_value}, independent reference returned {reference}"
            ),
        )),
        Err(err) => findings.push(Finding::error(
            "ENGINE-CD-ERROR",
            format!("{field_name}: mrz::check_digit rejected parser-produced data: {err}"),
        )),
    }

    match printed_check_value(printed) {
        Some(actual) if actual == reference => findings.push(Finding::info(
            rule,
            format!("{field_name}: check digit {printed:?} verifies (reference={reference})"),
        )),
        Some(actual) => findings.push(Finding::error(
            rule,
            format!(
                "{field_name}: printed check digit {printed:?} means {actual}, expected {reference}"
            ),
        )),
        None => findings.push(Finding::error(
            rule,
            format!("{field_name}: invalid printed check-digit character {printed:?}"),
        )),
    }
}

fn line<'a>(lines: &'a [&str], index: usize) -> Option<&'a str> {
    lines.get(index).copied()
}

fn audit_reference_checks(data: &MrzData, findings: &mut Vec<Finding>) {
    let lines: Vec<&str> = data.mrz_lines.lines().collect();

    match data.format {
        Format::Td1 => {
            let Some(l1) = line(&lines, 0) else {
                findings.push(Finding::error("ICAO-TD1-SHAPE", "missing TD1 line 1"));
                return;
            };
            let Some(l2) = line(&lines, 1) else {
                findings.push(Finding::error("ICAO-TD1-SHAPE", "missing TD1 line 2"));
                return;
            };
            let Some(l3) = line(&lines, 2) else {
                findings.push(Finding::error("ICAO-TD1-SHAPE", "missing TD1 line 3"));
                return;
            };

            if !audit_fixed_shape(&lines, 3, 30, "ICAO-TD1-SHAPE", findings) {
                return;
            }

            // Ordinary fixed-width document number. A `<` in the check-digit
            // position can signal the ICAO long-number overflow encoding, where
            // the real check digit moves into optional data. In that case a
            // simple 9-character-field check would be the wrong test.
            let doc_cd = l1.as_bytes()[14] as char;
            if doc_cd == '<' {
                findings.push(Finding::warning(
                    "ICAO-TD1-DOCNO-OVERFLOW",
                    "document number NOT independently recomputed: the check position is a                      filler, so the parser is using the ICAO long-number overflow encoding                      and a plain 9-character field check would be the wrong test",
                ));
            } else {
                check_reference_field(
                    findings,
                    "ICAO-TD1-DOCNO",
                    "document number",
                    &l1[5..14],
                    doc_cd,
                );
            }

            check_reference_field(
                findings,
                "ICAO-TD1-DOB",
                "date of birth",
                &l2[0..6],
                l2.as_bytes()[6] as char,
            );
            check_reference_field(
                findings,
                "ICAO-TD1-EXPIRY",
                "date of expiry",
                &l2[8..14],
                l2.as_bytes()[14] as char,
            );

            let composite = format!("{}{}{}{}", &l1[5..30], &l2[0..7], &l2[8..15], &l2[18..29]);
            check_reference_field(
                findings,
                "ICAO-TD1-COMPOSITE",
                "composite",
                &composite,
                l2.as_bytes()[29] as char,
            );

            if !l3.contains("<<") {
                findings.push(Finding::warning(
                    "ICAO-NAME-SEPARATOR",
                    "TD1 name line contains no canonical `<<` primary/secondary identifier separator; parser fallback may still recover a name, but the name line has no check digit protection",
                ));
            }
        }

        Format::Td2 => {
            let Some(l1) = line(&lines, 0) else {
                findings.push(Finding::error("ICAO-TD2-SHAPE", "missing TD2 line 1"));
                return;
            };
            let Some(l2) = line(&lines, 1) else {
                findings.push(Finding::error("ICAO-TD2-SHAPE", "missing TD2 line 2"));
                return;
            };

            if !audit_fixed_shape(&lines, 2, 36, "ICAO-TD2-SHAPE", findings) {
                return;
            }

            let doc_cd = l2.as_bytes()[9] as char;
            if doc_cd == '<' {
                findings.push(Finding::warning(
                    "ICAO-TD2-DOCNO-OVERFLOW",
                    "document number NOT independently recomputed: the check position is a                      filler, so the parser is using the ICAO long-number overflow encoding                      and a plain 9-character field check would be the wrong test",
                ));
            } else {
                check_reference_field(
                    findings,
                    "ICAO-TD2-DOCNO",
                    "document number",
                    &l2[0..9],
                    doc_cd,
                );
            }

            check_reference_field(
                findings,
                "ICAO-TD2-DOB",
                "date of birth",
                &l2[13..19],
                l2.as_bytes()[19] as char,
            );
            check_reference_field(
                findings,
                "ICAO-TD2-EXPIRY",
                "date of expiry",
                &l2[21..27],
                l2.as_bytes()[27] as char,
            );

            let composite = format!("{}{}{}", &l2[0..10], &l2[13..20], &l2[21..35]);
            check_reference_field(
                findings,
                "ICAO-TD2-COMPOSITE",
                "composite",
                &composite,
                l2.as_bytes()[35] as char,
            );

            if !l1[5..36].contains("<<") {
                findings.push(Finding::warning(
                    "ICAO-NAME-SEPARATOR",
                    "TD2 name field contains no canonical `<<` separator; this field is not protected by a check digit",
                ));
            }
        }

        Format::Td3 => {
            let Some(l1) = line(&lines, 0) else {
                findings.push(Finding::error("ICAO-TD3-SHAPE", "missing TD3 line 1"));
                return;
            };
            let Some(l2) = line(&lines, 1) else {
                findings.push(Finding::error("ICAO-TD3-SHAPE", "missing TD3 line 2"));
                return;
            };

            if !audit_fixed_shape(&lines, 2, 44, "ICAO-TD3-SHAPE", findings) {
                return;
            }

            let doc_cd = l2.as_bytes()[9] as char;
            if doc_cd == '<' {
                // SynthPass deliberately supports a TD3 overflow extension by
                // analogy to TD1/TD2. Part 4 itself does not define it.
                findings.push(Finding::warning(
                    "TD3-NONNORMATIVE-OVERFLOW",
                    "TD3 document-number check position is filler; SynthPass may decode an overflow extension here, but ICAO Part 4 does not define the TD1/TD2 overflow rule for TD3",
                ));
            } else {
                check_reference_field(
                    findings,
                    "ICAO-TD3-DOCNO",
                    "document number",
                    &l2[0..9],
                    doc_cd,
                );
            }

            check_reference_field(
                findings,
                "ICAO-TD3-DOB",
                "date of birth",
                &l2[13..19],
                l2.as_bytes()[19] as char,
            );
            check_reference_field(
                findings,
                "ICAO-TD3-EXPIRY",
                "date of expiry",
                &l2[21..27],
                l2.as_bytes()[27] as char,
            );
            check_reference_field(
                findings,
                "ICAO-TD3-PERSONAL",
                "personal number",
                &l2[28..42],
                l2.as_bytes()[42] as char,
            );

            let composite = format!("{}{}{}", &l2[0..10], &l2[13..20], &l2[21..43]);
            check_reference_field(
                findings,
                "ICAO-TD3-COMPOSITE",
                "composite",
                &composite,
                l2.as_bytes()[43] as char,
            );

            if !l1[5..44].contains("<<") {
                findings.push(Finding::warning(
                    "ICAO-NAME-SEPARATOR",
                    "TD3 name field contains no canonical `<<` separator; line 1 is not protected by a check digit",
                ));
            }
        }

        Format::MrvA | Format::MrvB => {
            findings.push(Finding::warning(
                "ICAO-MRV-SCOPE",
                "MRV-A/MRV-B: NO field was independently recomputed by this tool. Visa                  layouts are not implemented here, so a clean report for this document                  reflects the core parser alone and carries no independent corroboration",
            ));
        }

        _ => findings.push(Finding::warning(
            "ICAO-FUTURE-FORMAT",
            "format is newer than this audit tool; only parser-level checks were applied",
        )),
    }
}

/// Returns `false` when the canonical zone is not the exact shape the format
/// requires. Callers **must** bail on `false`: every field extraction below is
/// a fixed byte offset into a line of known width, so continuing past a shape
/// failure indexes out of bounds. The parser guarantees the width today, which
/// is why this has never fired — but a check whose result is discarded is not
/// a check.
fn audit_fixed_shape(
    lines: &[&str],
    expected_lines: usize,
    expected_width: usize,
    rule: &'static str,
    findings: &mut Vec<Finding>,
) -> bool {
    if lines.len() != expected_lines {
        findings.push(Finding::error(
            rule,
            format!(
                "canonical MRZ has {} lines; expected {expected_lines}",
                lines.len()
            ),
        ));
        return false;
    }

    let mut ok = true;
    for (index, line) in lines.iter().enumerate() {
        if line.chars().count() != expected_width {
            ok = false;
            findings.push(Finding::error(
                rule,
                format!(
                    "line {} has width {}; expected {expected_width}",
                    index + 1,
                    line.chars().count()
                ),
            ));
        }
        if !line
            .bytes()
            .all(|b| matches!(b, b'A'..=b'Z' | b'0'..=b'9' | b'<'))
        {
            ok = false;
            findings.push(Finding::error(
                rule,
                format!("line {} contains a character outside [A-Z0-9<]", index + 1),
            ));
        }
    }
    ok
}

fn audit_parser_checks(data: &MrzData, findings: &mut Vec<Finding>) {
    let failed = data.checks.failed();
    if failed.is_empty() {
        findings.push(Finding::info(
            "ENGINE-PARSER-CHECKS",
            format!(
                "mrz parser verified {}/{} applicable printed check digits",
                data.checks.verified(),
                data.checks.applicable()
            ),
        ));
    } else {
        for field in failed {
            findings.push(Finding::error(
                "ENGINE-PARSER-CHECKS",
                format!("mrz parser reports failed check digit: {field}"),
            ));
        }
    }

    if data.document_number_legacy_encoding {
        findings.push(Finding::warning(
            "SYNTHPASS-LEGACY-DOCNO",
            "document number was reconstructed using SynthPass's pre-0.6 legacy overflow encoding",
        ));
    }
}

fn audit_country_codes(data: &MrzData, findings: &mut Vec<Finding>) {
    if mrz::country_name(&data.issuing_country).is_none() {
        findings.push(Finding::warning(
            "ICAO-ISSUER-CODE",
            format!(
                "issuing state/organization code {:?} is not recognized by the crate registry",
                data.issuing_country
            ),
        ));
    }

    if mrz::country_name(&data.nationality).is_none() {
        findings.push(Finding::warning(
            "ICAO-NATIONALITY-CODE",
            format!(
                "nationality code {:?} is not recognized by the crate registry",
                data.nationality
            ),
        ));
    }
}

fn resolved_profile(requested: Profile, data: &MrzData, findings: &mut Vec<Finding>) -> Profile {
    if requested != Profile::Auto {
        return requested;
    }

    if data.issuing_country != "BIH" {
        findings.push(Finding::info(
            "BIH-PROFILE-AUTO",
            format!(
                "issuing state is {:?}, not BIH; no BiH-specific policy profile inferred",
                data.issuing_country
            ),
        ));
        return Profile::IcaoOnly;
    }

    match data.format {
        Format::Td1 => Profile::BihIdentityCard,
        Format::Td3 => Profile::BihPassport,
        Format::Td2 => {
            findings.push(Finding::warning(
                "BIH-TD2-UNVERIFIED",
                "BIH-issued TD2 was detected, but this audit intentionally does not infer `driver's licence = ICAO TD2`; that claim requires an authoritative BiH specimen/specification before becoming code",
            ));
            Profile::IcaoOnly
        }
        _ => Profile::IcaoOnly,
    }
}

fn audit_bih_policy(
    data: &MrzData,
    profile: Profile,
    observed_bih_td1_fillers: bool,
    findings: &mut Vec<Finding>,
) {
    match profile {
        Profile::IcaoOnly | Profile::Auto => {}

        Profile::BihIdentityCard => {
            if data.format != Format::Td1 {
                findings.push(Finding::error(
                    "BIH-ID-FORMAT",
                    format!(
                        "BiH identity-card profile requires TD1, parsed format is {:?}",
                        data.format
                    ),
                ));
            }
            require_bih_codes(data, findings);

            if data.document_type != "ID" {
                findings.push(Finding::warning(
                    "BIH-ID-DOCTYPE",
                    format!(
                        "document type is {:?}; `ID` may be expected for the target BiH issuance generation, but this is kept as a policy warning rather than an ICAO failure",
                        data.document_type
                    ),
                ));
            }

            // Deliberately do NOT impose a digit-only or fixed letter-position
            // regex on the 9-character document number. ICAO permits an
            // alphanumeric document-number field.
            if !data
                .document_number
                .bytes()
                .all(|b| matches!(b, b'A'..=b'Z' | b'0'..=b'9'))
            {
                findings.push(Finding::warning(
                    "BIH-ID-DOCNO-CHARSET",
                    format!(
                        "document number {:?} contains something other than A-Z/0-9 after filler trimming",
                        data.document_number
                    ),
                ));
            }

            if observed_bih_td1_fillers {
                let lines: Vec<&str> = data.mrz_lines.lines().collect();
                if let Some(l1) = lines.first() {
                    let observed = &l1[15..18];
                    if observed == "<<<" {
                        findings.push(Finding::info(
                            "OBS-BIH-TD1-OPT-PREFIX",
                            "TD1 optional-data field begins with observed `<<<` prefix",
                        ));
                    } else {
                        findings.push(Finding::warning(
                            "OBS-BIH-TD1-OPT-PREFIX",
                            format!(
                                "TD1 optional-data positions 16-18 are {observed:?}, not `<<<`; this is only an observational corpus mismatch, NOT an ICAO conformance failure"
                            ),
                        ));
                    }
                }
            }
        }

        Profile::BihPassport => {
            if data.format != Format::Td3 {
                findings.push(Finding::error(
                    "BIH-PASSPORT-FORMAT",
                    format!(
                        "BiH passport profile requires TD3, parsed format is {:?}",
                        data.format
                    ),
                ));
            }
            require_bih_codes(data, findings);

            if !data.document_type.starts_with('P') {
                findings.push(Finding::error(
                    "BIH-PASSPORT-DOCTYPE",
                    format!(
                        "passport profile expected P* document code, got {:?}",
                        data.document_type
                    ),
                ));
            }
        }
    }
}

fn require_bih_codes(data: &MrzData, findings: &mut Vec<Finding>) {
    if data.issuing_country != "BIH" {
        findings.push(Finding::error(
            "BIH-ISSUER",
            format!(
                "BiH profile requires issuing-state code BIH, got {:?}",
                data.issuing_country
            ),
        ));
    }

    if data.nationality != "BIH" {
        findings.push(Finding::error(
            "BIH-NATIONALITY",
            format!(
                "BiH citizen-document profile requires nationality BIH, got {:?}",
                data.nationality
            ),
        ));
    }
}

fn print_report(data: &MrzData, findings: &[Finding], requested: Profile, effective: Profile) {
    println!("=== MRZ AUDIT ===");
    println!("requested profile : {requested:?}");
    println!("effective profile : {effective:?}");
    println!("format            : {:?}", data.format);
    println!("document type     : {}", data.document_type);
    println!("issuing country   : {}", data.issuing_country);
    println!("nationality       : {}", data.nationality);
    println!("document number   : {}", data.full_document_number());
    println!(
        "name              : {} / {}",
        data.surname, data.given_names
    );
    println!("date of birth     : {}", data.date_of_birth);
    println!("date of expiry    : {}", data.date_of_expiry);
    println!(
        "parser checks     : {}/{}",
        data.checks.verified(),
        data.checks.applicable()
    );
    println!();
    println!("Canonical MRZ used for reference checks:");
    println!("{}", data.mrz_lines);
    println!();

    let errors = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    let warnings = findings
        .iter()
        .filter(|f| f.severity == Severity::Warning)
        .count();

    println!("Findings ({errors} errors, {warnings} warnings):");
    for finding in findings {
        println!(
            "[{}] {:<28} {}",
            finding.severity.label(),
            finding.rule,
            finding.message
        );
    }
}

fn run(args: Args, input: &str) -> Result<(MrzData, Profile, Vec<Finding>), String> {
    let data = find_and_parse(input).map_err(|err| format!("MRZ parse failed: {err}"))?;

    let mut findings = Vec::new();
    audit_parser_checks(&data, &mut findings);
    audit_reference_checks(&data, &mut findings);
    audit_country_codes(&data, &mut findings);

    let effective = resolved_profile(args.profile, &data, &mut findings);
    audit_bih_policy(
        &data,
        effective,
        args.observed_bih_td1_fillers,
        &mut findings,
    );

    Ok((data, effective, findings))
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(64);
        }
    };

    let requested = args.profile;

    let input = match read_input(args.path.as_deref()) {
        Ok(input) => input,
        Err(err) => {
            eprintln!("failed to read input: {err}");
            return ExitCode::from(66);
        }
    };

    match run(args, &input) {
        Ok((data, effective, findings)) => {
            print_report(&data, &findings, requested, effective);
            if findings.iter().any(|f| f.severity == Severity::Error) {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(err) => {
            // Distinct from 2: the input never became an MrzData, so no audit
            // ran at all. A caller that treats "could not read" the same as
            // "read and found problems" cannot act on the difference.
            eprintln!("{err}");
            ExitCode::from(3)
        }
    }
}

// The differential test suite that used to live here has moved to
// `crates/mrz/tests/reference_check_digit.rs`.
//
// `cargo test --workspace`, which is what CI runs, does NOT execute
// `#[cfg(test)]` modules inside `examples/` -- only an explicit
// `cargo test --example <name>` does, and nothing runs that. Six tests sat
// here reading as coverage while never once executing. The reference
// implementation is now pinned by proptest in `tests/`, against arbitrary MRZ
// input rather than a seven-field list.
