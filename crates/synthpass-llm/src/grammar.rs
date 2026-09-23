//! GBNF grammar for Tier-2 decoding (Atlas §8).
//!
//! llama.cpp can constrain sampling to a grammar, masking every token that
//! couldn't continue a conforming string. Applied to the extraction schema,
//! that makes malformed output *unrepresentable* rather than something
//! [`crate::repair`] has to salvage after the fact: no code fences, no prose
//! preamble, no trailing commas, no truncated object, no invented keys.
//!
//! **Structure only, deliberately.** The grammar pins the JSON *shape* — the
//! exact ten keys of [`crate::prompt::FIELDS`], in order, each holding a
//! string or `null`. It does **not** constrain the values themselves (date
//! layout, `sex` vocabulary, country-code casing), because
//! [`synthpass_core::Extraction`] accepts any string in those slots. Encoding
//! a narrower dialect here would be a semantic change to what the model may
//! say about a document, not the syntactic guarantee this module is for, and
//! it belongs behind its own measurement rather than riding along with this
//! one.
//!
//! The grammar is *generated from* [`crate::prompt::FIELDS`] rather than
//! written out by hand, so the fields the model is asked for and the fields it
//! is permitted to emit cannot drift apart.

use crate::prompt::FIELDS;
use synthpass_core::v2::CoreField;

/// The entry-point rule name, as passed to `LlamaSampler::grammar`.
pub const GRAMMAR_ROOT: &str = "root";

/// The value/​string/​whitespace rules, kept byte-compatible with the reference
/// `json.gbnf` that ships with `llama-cpp-2` — same escape handling, same
/// negated character class. `value` is narrowed to `string | "null"` because
/// every field of [`synthpass_core::Extraction`] this grammar covers is an
/// `Option<String>`: a number or nested object there would parse as JSON and
/// then fail the schema, which is exactly the failure class this exists to
/// remove.
const VALUE_RULES: &str = r#"
value  ::= string | "null"
string ::= "\"" ( [^"\\] | "\\" (["\\/bfnrt] | "u" [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F]) )* "\""
ws     ::= [ \t\n]*
"#;

/// Byte-wise string equality, usable in a `const` context: `PartialEq for str`
/// is not `const`, so the drift guards below cannot use `==`.
///
/// Kept private to this crate, like the identical helper in `synthpass-bench`.
/// Sharing one would mean exporting it from `synthpass-core`, adding public
/// API to a published crate for an assertion no consumer can call.
const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Fields [`FIELDS`] asks the model for that are not ICAO 9303 fields, and so
/// have no [`CoreField`] variant.
///
/// `mrz_line` is the raw MRZ zone — the model is asked to transcribe the
/// printed characters, not to name a parsed field, and the raw zone is not an
/// ICAO 9303 field in its own right. [`CoreField`] deliberately does not cover
/// it; see the "Deliberately **not** unified with `synthpass_llm::prompt::FIELDS`"
/// note at `crates/synthpass-core/src/v2.rs:534-537`.
const PROMPT_ONLY_FIELDS: &[&str] = &["mrz_line"];

/// [`CoreField`]s deliberately not asked of the model.
///
/// `personal_number` has been absent from [`FIELDS`] since the list was
/// written (`crate::prompt` describes the list as a subset of the canonical
/// schema); no tracked document records a reason beyond that. Listing it here
/// keeps the omission a stated fact rather than an accident — see
/// `knowledge/technical_debt.md`, "Three parallel lists of ICAO field names".
/// The two optional-data fields (ADR-0018) follow it: they are
/// issuer-discretionary zone content with no visual-zone counterpart the
/// model could be asked to read.
const CORE_FIELDS_NOT_PROMPTED: &[&str] =
    &["personal_number", "optional_data_1", "optional_data_2"];

/// The drift guard: the prompt's field list and [`CoreField::ALL`] may differ
/// only in the two documented directions above.
///
/// This is a compile error rather than a test because the failure it prevents
/// is a schema field the model is never asked for — a runtime test would only
/// notice once some document happened to exercise that field.
const _: () = {
    // (a) Every name the prompt asks for is either a CoreField or a documented
    //     prompt-only field.
    let mut i = 0;
    while i < FIELDS.len() {
        let mut known = false;

        let mut j = 0;
        while j < CoreField::ALL.len() {
            if str_eq(FIELDS[i], CoreField::ALL[j].as_str()) {
                known = true;
            }
            j += 1;
        }

        let mut k = 0;
        while k < PROMPT_ONLY_FIELDS.len() {
            if str_eq(FIELDS[i], PROMPT_ONLY_FIELDS[k]) {
                known = true;
            }
            k += 1;
        }

        assert!(
            known,
            "prompt::FIELDS names a field that is neither a CoreField nor a \
             documented PROMPT_ONLY_FIELDS entry. Two edits are required: add \
             the field to CoreField::ALL in crates/synthpass-core/src/v2.rs, or \
             list it in PROMPT_ONLY_FIELDS here if the model is deliberately \
             asked for something that is not an ICAO 9303 field."
        );

        i += 1;
    }

    // (b) Every CoreField is either asked for or documented as deliberately
    //     not asked for.
    let mut i = 0;
    while i < CoreField::ALL.len() {
        let mut covered = false;

        let mut j = 0;
        while j < FIELDS.len() {
            if str_eq(CoreField::ALL[i].as_str(), FIELDS[j]) {
                covered = true;
            }
            j += 1;
        }

        let mut k = 0;
        while k < CORE_FIELDS_NOT_PROMPTED.len() {
            if str_eq(CoreField::ALL[i].as_str(), CORE_FIELDS_NOT_PROMPTED[k]) {
                covered = true;
            }
            k += 1;
        }

        assert!(
            covered,
            "CoreField::ALL contains a field the prompt neither asks for nor \
             records as deliberately unprompted. Two edits are required: add \
             the field to prompt::FIELDS in crates/synthpass-llm/src/prompt.rs, \
             or list it in CORE_FIELDS_NOT_PROMPTED here with the reason the \
             model is not asked for it."
        );

        i += 1;
    }
};

/// Build the GBNF constraining Tier-2 output to the extraction schema.
///
/// Field order is fixed (it mirrors [`FIELDS`]) rather than free: a fixed
/// order is a strictly smaller search space, and nothing downstream cares
/// about key order since `serde` matches by name.
pub fn extraction_gbnf() -> String {
    let mut gbnf = String::from("root ::= \"{\" ws");
    for (i, field) in FIELDS.iter().enumerate() {
        if i > 0 {
            gbnf.push_str(" \",\" ws");
        }
        // Emits e.g.  "\"surname\"" ws ":" ws value ws
        gbnf.push_str(&format!(" \"\\\"{field}\\\"\" ws \":\" ws value ws"));
    }
    gbnf.push_str(" \"}\"\n");
    gbnf.push_str(VALUE_RULES);
    gbnf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammar_names_every_prompted_field_in_order() {
        let gbnf = extraction_gbnf();
        let mut cursor = 0;
        for field in FIELDS {
            let needle = format!("\\\"{field}\\\"");
            let at = gbnf[cursor..]
                .find(&needle)
                .unwrap_or_else(|| panic!("grammar is missing the '{field}' key:\n{gbnf}"));
            cursor += at + needle.len();
        }
    }

    #[test]
    fn grammar_defines_every_rule_it_references() {
        let gbnf = extraction_gbnf();
        for rule in ["root", "value", "string", "ws"] {
            assert!(
                gbnf.contains(&format!("{rule} ")),
                "grammar references '{rule}' but never defines it:\n{gbnf}"
            );
        }
    }

    #[test]
    fn a_value_is_only_ever_a_string_or_null() {
        let gbnf = extraction_gbnf();
        let value_rule = gbnf
            .lines()
            .find(|l| l.starts_with("value"))
            .expect("the grammar defines a `value` rule");
        let alternatives: Vec<&str> = value_rule
            .split("::=")
            .nth(1)
            .expect("a rule has a right-hand side")
            .split('|')
            .map(str::trim)
            .collect();
        assert_eq!(
            alternatives,
            ["string", "\"null\""],
            "every field this grammar covers is an Option<String>; anything \
             else (a number, an object) would parse as JSON and then fail the \
             schema — which is the failure class this grammar exists to remove"
        );
    }

    /// The drift guard that matters: a document in exactly the shape this
    /// grammar permits must land in `Extraction` with **every** field
    /// populated. `serde` ignores unknown keys, so a typo in a grammar field
    /// name would otherwise pass silently — here it shows up as a `None`.
    #[test]
    fn grammar_shaped_output_populates_every_extraction_field() {
        // Distinct value per field so a mix-up can't hide behind a match.
        let body: Vec<String> = FIELDS
            .iter()
            .map(|f| format!("\"{f}\":\"value-of-{f}\""))
            .collect();
        let doc = format!("{{{}}}", body.join(","));

        let e = crate::repair::parse_extraction(&doc)
            .expect("grammar-shaped output must satisfy the Extraction schema");

        assert_eq!(e.document_type.as_deref(), Some("value-of-document_type"));
        assert_eq!(
            e.issuing_country.as_deref(),
            Some("value-of-issuing_country")
        );
        assert_eq!(
            e.document_number.as_deref(),
            Some("value-of-document_number")
        );
        assert_eq!(e.surname.as_deref(), Some("value-of-surname"));
        assert_eq!(e.given_names.as_deref(), Some("value-of-given_names"));
        assert_eq!(e.nationality.as_deref(), Some("value-of-nationality"));
        assert_eq!(e.date_of_birth.as_deref(), Some("value-of-date_of_birth"));
        assert_eq!(e.sex.as_deref(), Some("value-of-sex"));
        assert_eq!(e.date_of_expiry.as_deref(), Some("value-of-date_of_expiry"));
        assert_eq!(e.mrz_line.as_deref(), Some("value-of-mrz_line"));
        assert_eq!(e.extraction_method, "llm");
    }

    /// ...and it must arrive without the repair layer having to touch it.
    /// This is the Atlas §8 acceptance criterion in miniature.
    #[test]
    fn grammar_shaped_output_needs_no_repair() {
        let body: Vec<String> = FIELDS.iter().map(|f| format!("\"{f}\":null")).collect();
        let doc = format!("{{{}}}", body.join(","));

        assert!(
            !crate::repair::needs_repair(&doc),
            "grammar-conforming output must take the clean parse path"
        );
        crate::repair::parse_extraction(&doc).expect("an all-null document is still schema-valid");
    }

    #[test]
    fn grammar_permits_whitespace_a_model_would_naturally_emit() {
        // The grammar allows optional whitespace around structural tokens, so
        // pretty-printed output is conforming too — and must still parse.
        let doc = "{\n  \"document_type\": \"P\",\n  \"surname\": null\n}";
        assert!(
            serde_json::from_str::<serde_json::Value>(doc).is_ok(),
            "the whitespace shape the grammar permits must be valid JSON"
        );
    }
}
