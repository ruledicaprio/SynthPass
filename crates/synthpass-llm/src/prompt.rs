//! Qwen2.5 ChatML prompt construction — a direct port of
//! `python/inferer/prompts.py`.

const SYSTEM: &str = "You are an expert, highly accurate identity document parser. Your task is \
to extract specific fields from the provided OCR Markdown text into a \
strict, valid JSON object. If a field is not found or is illegible, use \
null. Do not invent data. Some documents are bilingual or print a non-Latin \
script (Hebrew, Arabic, Chinese, etc.) alongside a Latin transliteration — \
for every field, extract the Latin/romanized rendering exactly as printed. \
Never invent or guess a Latin spelling from non-Latin text you cannot read; \
if a field's only legible rendering is in a non-Latin script, use null for \
that field instead of guessing.";

/// Fields requested from the model (subset of the canonical schema;
/// provenance fields are added downstream, not by the model).
///
/// Shared with [`crate::grammar`], which generates the GBNF from this exact
/// list — what the model is *asked* for and what it is *permitted to emit*
/// come from one source, so they cannot drift apart.
pub(crate) const FIELDS: &[&str] = &[
    "document_type",
    "issuing_country",
    "document_number",
    "surname",
    "given_names",
    "nationality",
    "date_of_birth",
    "sex",
    "date_of_expiry",
    "mrz_line",
];

/// ChatML skeleton, filled by [`build_prompt`] and, with `{content}` left
/// empty, by [`prompt_digest`] — one source of static prompt text so the
/// version-pinning digest can never drift from what production sends.
///
/// `{hint}` sits between the field list and the OCR text and expands to the
/// empty string whenever [`build_prompt`] is called with `hint: None` — the
/// surrounding blank line already in the template (`{fields}.\n\n{hint}OCR
/// Markdown Text:`) is what keeps that case byte-identical to the pre-hint
/// v1 output; see `build_prompt_with_no_hint_is_byte_identical_to_v1` below.
const TEMPLATE: &str = "<|im_start|>system\n{system}\n<|im_end|>\n\
     <|im_start|>user\nExtract these fields: {fields}.\n\n\
     {hint}OCR Markdown Text:\n{content}\n\n\
     Output ONLY valid JSON. No markdown formatting, no explanations, no \
     code blocks.\n<|im_end|>\n<|im_start|>assistant";

/// Prefix for the optional MRZ hint (see [`build_prompt`]'s `hint` param).
/// Public so [`prompt_digest`] can fold it into the version-pinning digest
/// without duplicating the literal.
pub(crate) const HINT_PREFIX: &str =
    "Verified from the machine-readable zone (trust these over the OCR text): ";

/// Build the Qwen2.5 ChatML prompt for one document's OCR Markdown.
///
/// `hint`, when present, is a short pre-rendered `k=v` string of
/// checksum-verified MRZ fields (built by `synthpass-pipeline` from
/// `mrz::Checks`, never by this crate — see [`crate`]'s module doc for why
/// this crate stays free of a `mrz`/`synthpass-core`/`synthpass-die`
/// dependency). `hint: None` produces output byte-identical to the
/// pre-hint (v1) prompt.
pub fn build_prompt(md_content: &str, hint: Option<&str>) -> String {
    // docling renders MRZ filler chevrons as HTML entities (`<` -> `&lt;`) in
    // its Markdown output. Left escaped, the model echoes `&lt;` verbatim into
    // fields like mrz_line instead of the literal `<` printed on the
    // document, so unescape before the model ever sees the text.
    let md_content = unescape_html(md_content);
    let md_content = drop_non_latin_noise(&md_content);
    let hint_text = hint
        .map(|h| format!("{HINT_PREFIX}{h}. Confirm or correct the remaining fields.\n\n"))
        .unwrap_or_default();
    fill_template(&md_content, &hint_text)
}

fn fill_template(content: &str, hint: &str) -> String {
    TEMPLATE
        .replace("{system}", SYSTEM)
        .replace("{fields}", &FIELDS.join(", "))
        .replace("{hint}", hint)
        .replace("{content}", content)
}

/// Stable identity of the compiled-in prompt template, paired with
/// `PROMPT_VERSION` and a content digest in [`prompt_ref`].
pub const PROMPT_ID: &str = "qwen2.5-fields";

/// Hand-bumped on any edit to `SYSTEM`, `FIELDS`, `TEMPLATE`, or
/// `HINT_PREFIX`. Must be bumped in the same commit that updates the
/// expected digest in `prompt_digest_is_pinned` below.
///
/// Bumped 1 -> 2 for the MRZ-hint slot (`{hint}`/`HINT_PREFIX`) added to
/// the template — see issue #102. The hint is gated off by default
/// (`SYNTHPASS_LLM_MRZ_HINT`), but the template text itself changed, so the
/// version and digest change regardless of the flag's runtime value.
pub const PROMPT_VERSION: u32 = 2;

/// The [`synthpass_core::v2::PromptRef`] recorded on every Tier-2 extraction:
/// which prompt produced this record, auditable via its digest.
pub fn prompt_ref() -> synthpass_core::v2::PromptRef {
    synthpass_core::v2::PromptRef {
        id: PROMPT_ID.to_string(),
        version: PROMPT_VERSION,
        digest: prompt_digest(),
    }
}

/// First 8 hex characters of `sha256` of the static prompt skeleton (system
/// text + field list + wrapper text + `HINT_PREFIX`, `{content}` empty) —
/// excludes per-document OCR markdown and the per-document hint *values*
/// (both vary every call and would make the digest meaningless as a version
/// fingerprint), but includes `HINT_PREFIX` itself since that text is part
/// of the compiled-in template, not the per-document content.
fn prompt_digest() -> String {
    let skeleton = fill_template("", HINT_PREFIX);
    synthpass_core::audit::sha256_hex(skeleton.as_bytes())[..8].to_string()
}

/// Drop OCR Markdown lines that are overwhelmingly non-Latin (garbled
/// Hebrew/Arabic/CJK recognition noise), so the model's context is dominated
/// by the Latin MRZ/VIZ text it can actually transcribe instead of noise
/// like `NTETUTH` / `JIT DI`. A line is dropped only when more than half of
/// its non-whitespace characters fall outside the Latin/MRZ character set —
/// ordinary Latin lines (including ones with a little punctuation noise)
/// pass through untouched.
fn drop_non_latin_noise(s: &str) -> String {
    s.lines()
        .filter(|line| !is_non_latin_noise(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_non_latin_noise(line: &str) -> bool {
    let mut total = 0usize;
    let mut noise = 0usize;
    for c in line.chars() {
        if c.is_whitespace() {
            continue;
        }
        total += 1;
        if !is_latin_or_punctuation(c) {
            noise += 1;
        }
    }
    total > 0 && noise * 2 > total
}

fn is_latin_or_punctuation(c: char) -> bool {
    c.is_ascii_alphanumeric() || "<>/:.,-_()'\"*#|=+&%!?;".contains(c)
}

/// Unescape the small set of HTML entities docling actually emits. Not a
/// general-purpose HTML unescaper — just enough to undo docling's markdown
/// rendering of `<`/`>`/`&` etc. in OCR text.
fn unescape_html(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&amp;", "&") // must be last: undoes double-escaping of the above
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescapes_mrz_chevrons() {
        assert_eq!(unescape_html("P&lt;UTOA1234567&lt;&lt;"), "P<UTOA1234567<<");
    }

    #[test]
    fn ampersand_unescaped_last_avoids_double_unescape() {
        // "&amp;lt;" should become "&lt;", not "<".
        assert_eq!(unescape_html("&amp;lt;"), "&lt;");
    }

    #[test]
    fn prompt_contains_chatml_markers_and_fields() {
        let p = build_prompt("some markdown", None);
        assert!(p.starts_with("<|im_start|>system\n"));
        assert!(p.ends_with("<|im_start|>assistant"));
        assert!(p.contains("document_number"));
        assert!(p.contains("some markdown"));
    }

    /// Pinned byte-for-byte against the pre-hint (v1) template shape: adding
    /// the `{hint}` slot to `TEMPLATE` must not change a single byte of
    /// output when no hint is supplied.
    #[test]
    fn build_prompt_with_no_hint_is_byte_identical_to_v1() {
        let p = build_prompt("some markdown", None);
        assert_eq!(
            p,
            "<|im_start|>system\n\
             You are an expert, highly accurate identity document parser. Your task is \
             to extract specific fields from the provided OCR Markdown text into a \
             strict, valid JSON object. If a field is not found or is illegible, use \
             null. Do not invent data. Some documents are bilingual or print a non-Latin \
             script (Hebrew, Arabic, Chinese, etc.) alongside a Latin transliteration — \
             for every field, extract the Latin/romanized rendering exactly as printed. \
             Never invent or guess a Latin spelling from non-Latin text you cannot read; \
             if a field's only legible rendering is in a non-Latin script, use null for \
             that field instead of guessing.\n\
             <|im_end|>\n\
             <|im_start|>user\nExtract these fields: document_type, issuing_country, \
             document_number, surname, given_names, nationality, date_of_birth, sex, \
             date_of_expiry, mrz_line.\n\n\
             OCR Markdown Text:\nsome markdown\n\n\
             Output ONLY valid JSON. No markdown formatting, no explanations, no \
             code blocks.\n<|im_end|>\n<|im_start|>assistant"
        );
    }

    #[test]
    fn a_hint_is_inserted_between_the_field_list_and_the_ocr_text() {
        let p = build_prompt("some markdown", Some("document_number=L898902C3"));
        assert!(p.contains(
            "Verified from the machine-readable zone (trust these over the OCR text): \
             document_number=L898902C3. Confirm or correct the remaining fields.\n\n\
             OCR Markdown Text:\nsome markdown"
        ));
    }

    #[test]
    fn system_prompt_instructs_latin_transliteration() {
        assert!(SYSTEM.contains("Latin"));
        assert!(SYSTEM.contains("null"));
    }

    #[test]
    fn drops_non_latin_noise_lines_keeps_latin_lines() {
        let md = "surname/NAME\nZHENGJIAN YANGBEN\n证件样本\nP<CHNZHENGJIAN<<YANGBEN<<<<<<<<<<<<<<<<<<<<\nE0000000008CHN8310291F2202059NGKELMPONBPJB972";
        let filtered = drop_non_latin_noise(md);
        assert!(filtered.contains("ZHENGJIAN YANGBEN"));
        assert!(filtered.contains("P<CHNZHENGJIAN<<YANGBEN"));
        assert!(!filtered.contains("证件样本"));
    }

    #[test]
    fn keeps_ordinary_latin_lines_with_light_punctuation() {
        let line = "Date of birth: 29 OCT 1983 / Place: BEIJING";
        assert!(!is_non_latin_noise(line));
    }

    #[test]
    fn prompt_ref_reports_id_and_version() {
        let r = prompt_ref();
        assert_eq!(r.id, PROMPT_ID);
        assert_eq!(r.version, PROMPT_VERSION);
        assert_eq!(r.digest.len(), 8);
        assert!(r.digest.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn prompt_digest_is_pinned() {
        // If this fails, SYSTEM, FIELDS, or TEMPLATE changed. Bump
        // PROMPT_VERSION and update the expected digest below in the same
        // commit — don't just paste in the new value.
        assert_eq!(prompt_digest(), "086095b6");
    }
}
