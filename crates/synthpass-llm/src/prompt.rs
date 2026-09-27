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
    let md_content = clean_markdown(md_content);
    fill_template(&md_content, &hint_text(hint))
}

/// Build the ChatML prompt exactly as [`build_prompt`] would, then, if it
/// does not leave room for `reserved_output_tokens` more tokens inside
/// `n_ctx`, drop content lines deterministically, never an MRZ-shaped one,
/// until it does. See [`fit_content`] for the drop policy.
///
/// `count_tokens` counts a string exactly as the model would (production
/// passes `LlamaModel::str_to_token`); this function calls it once for the
/// unmodified content and, only when that does not fit, once per candidate
/// while searching for one that does. Unit tests pass a fake counter so the
/// policy is verifiable without a model.
///
/// Returns `Err` — naming the token counts involved — when even every
/// MRZ-shaped line together still does not fit; this must never be papered
/// over by dropping an MRZ line, since that is the one content a Tier-2
/// extraction cannot recover from at all.
pub fn build_prompt_within_budget(
    md_content: &str,
    hint: Option<&str>,
    n_ctx: u32,
    reserved_output_tokens: u32,
    mut count_tokens: impl FnMut(&str) -> usize,
) -> Result<String, String> {
    let md_content = clean_markdown(md_content);
    let hint_text = hint_text(hint);
    // Compared as `usize`, the counter's own type: a count narrowed to `u32`
    // could wrap and read as fitting.
    let budget = n_ctx.saturating_sub(reserved_output_tokens) as usize;

    let full_prompt = fill_template(&md_content, &hint_text);
    if count_tokens(&full_prompt) <= budget {
        return Ok(full_prompt);
    }

    let fitted =
        fit_content(&md_content, budget, &hint_text, &mut count_tokens).ok_or_else(|| {
            let mrz_only: String = md_content
                .lines()
                .filter(|l| looks_like_mrz(l))
                .collect::<Vec<_>>()
                .join("\n");
            let tokens = count_tokens(&fill_template(&mrz_only, &hint_text));
            format!(
                "prompt ({tokens} tokens) plus the {reserved_output_tokens}-token output budget \
                 does not fit in context window ({n_ctx}) even keeping only MRZ-shaped lines"
            )
        })?;
    Ok(fill_template(&fitted, &hint_text))
}

/// Drop content lines from `content` until `fill_template(_, hint)` counts at
/// or under `budget` tokens, or return `None` if even the MRZ-shaped lines
/// alone do not fit.
///
/// Policy, in this order: keep every MRZ-shaped line ([`looks_like_mrz`]) —
/// this is the one content a Tier-2 extraction cannot recover from at all if
/// it is lost, so it is never traded away for anything else. Then add back
/// the remaining lines one at a time, in their original document order,
/// keeping each only while the result still fits — so nothing behind the cap
/// is reordered, and what survives is whatever the document put first.
///
/// A microprint fragment short enough to survive [`drop_long_run_noise`] can
/// also look MRZ-shaped, and is then kept like an MRZ line. That costs budget,
/// never correctness: the result still fits, or the caller gets an explicit
/// error.
fn fit_content(
    content: &str,
    budget: usize,
    hint_text: &str,
    count_tokens: &mut impl FnMut(&str) -> usize,
) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut kept: Vec<bool> = lines.iter().map(|l| looks_like_mrz(l)).collect();

    let assemble = |kept: &[bool]| -> String {
        lines
            .iter()
            .zip(kept)
            .filter(|(_, k)| **k)
            .map(|(l, _)| *l)
            .collect::<Vec<_>>()
            .join("\n")
    };

    let mrz_only_tokens = count_tokens(&fill_template(&assemble(&kept), hint_text));
    if mrz_only_tokens > budget {
        return None;
    }

    for i in 0..lines.len() {
        if kept[i] {
            continue;
        }
        kept[i] = true;
        let candidate_tokens = count_tokens(&fill_template(&assemble(&kept), hint_text));
        if candidate_tokens > budget {
            kept[i] = false;
        }
    }

    Some(assemble(&kept))
}

/// The `hint` slot's rendered text, or the empty string when `hint` is
/// `None` — shared by [`build_prompt`] and [`build_prompt_within_budget`] so
/// the two can never format it differently.
fn hint_text(hint: Option<&str>) -> String {
    hint.map(|h| format!("{HINT_PREFIX}{h}. Confirm or correct the remaining fields.\n\n"))
        .unwrap_or_default()
}

/// Every deterministic content cleanup [`build_prompt`] and
/// [`build_prompt_within_budget`] apply before the OCR text reaches the
/// template, in order: undo docling's HTML-escaping, then drop the two kinds
/// of line noise this crate knows how to recognise without a model. Shared
/// so the two entry points can never clean the input differently.
fn clean_markdown(md_content: &str) -> String {
    // docling renders MRZ filler chevrons as HTML entities (`<` -> `&lt;`) in
    // its Markdown output. Left escaped, the model echoes `&lt;` verbatim into
    // fields like mrz_line instead of the literal `<` printed on the
    // document, so unescape before the model ever sees the text.
    let md_content = unescape_html(md_content);
    let md_content = drop_non_latin_noise(&md_content);
    drop_long_run_noise(&md_content)
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
/// `HINT_PREFIX` — those change `prompt_digest_is_pinned`'s expected digest
/// and must bump this in the same commit — **and** on any change to the
/// content preprocessing `build_prompt`/`build_prompt_within_budget` apply
/// (the noise filters, the token-budget cap), even though that leaves the
/// digest unchanged: `prompt_digest` is taken over the static skeleton with
/// `{content}` empty, so it cannot see a change in what happens to
/// `{content}` before it lands there, and this version is the only auditable
/// record that such a change shipped.
///
/// Bumped 1 -> 2 for the MRZ-hint slot (`{hint}`/`HINT_PREFIX`) added to
/// the template — see issue #102. The hint is gated off by default
/// (`SYNTHPASS_LLM_MRZ_HINT`), but the template text itself changed, so the
/// version and digest change regardless of the flag's runtime value.
///
/// Bumped 2 -> 3 for [`drop_long_run_noise`] and [`build_prompt_within_budget`]
/// (issue #506): one fixture's guilloche microprint OCR'd into a prompt too
/// large for the default 2048-token context once the 500-token output budget
/// is accounted for. Neither `SYSTEM`, `FIELDS`, `TEMPLATE` nor `HINT_PREFIX`
/// changed, so the digest below is unchanged too.
pub const PROMPT_VERSION: u32 = 3;

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

/// Longest unbroken (whitespace-free) run measured, across every fixture
/// under `samples/ocr_fixtures/` and its `derived/` subdirectory, in a line
/// that verbatim carries a ground-truth MRZ line or field value: 47
/// characters
/// (`Korea_Republic_of_Korea_Passport_Specimen_PM_KOR_2022_mrz.md`'s
/// `PMKORPARK<<JUNSU<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<`) — one character more
/// than ICAO 9303's longest defined MRZ line (TD3, 44 characters) glued to an
/// adjacent character with no separating whitespace. OCR corruption can push
/// that further still: the same corpus's
/// `Afghanistan_Passport_Specimen_P0_AFG_2016_mrz.md` reads its second MRZ
/// line as a 48-character run behind a garbled two-character prefix.
///
/// [`LONG_RUN_NOISE_THRESHOLD`] sits at 64 — comfortably above both, with
/// margin for corruption this corpus hasn't shown yet — while every run this
/// rule exists to catch in
/// `Romania_Passport_Specimen_PE_ROU_2024_mrz.md`'s guilloche microprint
/// (`MANOMANIAROMANIAROMANIA…`) measures 69 characters or more. High
/// precision matters more than catching every noisy line here: whatever
/// survives this filter still has to fit the context window, and
/// [`fit_content`] is what guarantees that.
const LONG_RUN_NOISE_THRESHOLD: usize = 64;

/// Drop OCR Markdown lines dominated by an unbroken character run far longer
/// than any real line on an identity document produces — guilloche/microprint
/// background text that OCR misreads as one long string of pseudo-Latin
/// noise. [`drop_non_latin_noise`] cannot catch this: every character in that
/// noise is ordinary Latin, just glued into a run no genuine line has.
///
/// Never drops a line on its total length — a lone `F`/`M` sex value, or any
/// other short line, is untouched regardless of how the rest of the document
/// reads, because this only ever looks at the longest whitespace-free run
/// inside a line.
fn drop_long_run_noise(s: &str) -> String {
    s.lines()
        .filter(|line| !is_long_run_noise(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_long_run_noise(line: &str) -> bool {
    line.split_whitespace()
        .any(|run| run.chars().count() > LONG_RUN_NOISE_THRESHOLD)
}

/// The ICAO 9303 MRZ character set, expanded with `?` — OCR reads the filler
/// chevron `<` as `?` about as often as it reads it correctly (this crate
/// deliberately has no `mrz` dependency to import the canonical charset
/// from; letters OCR might substitute for `<`, such as `K` or `L`, are
/// already in the base A-Z set below and need no separate allowance).
const MRZ_SHAPE_CHARSET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789<?";

/// Minimum non-whitespace length before a line counts as MRZ-shaped. TD1 is
/// ICAO 9303's shortest format at 30 characters per line; OCR routinely
/// truncates a few characters off either end, so this sits below that with
/// the same margin `tests/parity.rs`'s `HOLDOUT_MIN_LEN` uses for the
/// opposite purpose (identifying MRZ lines to strip for the holdout corpus,
/// rather than to keep here).
const MRZ_SHAPE_MIN_LEN: usize = 25;

/// Minimum fraction of a line's non-whitespace characters that must fall in
/// [`MRZ_SHAPE_CHARSET`] for the line to count as MRZ-shaped. Matches
/// `tests/parity.rs`'s `HOLDOUT_MIN_CHARSET_FRACTION`, measured over the same
/// corpus for the same reason: below this a line reads as ordinary VIZ prose,
/// not an OCR reading of an MRZ.
const MRZ_SHAPE_MIN_CHARSET_FRACTION: f64 = 0.85;

/// Whether `line` reads as an OCR transcription of an MRZ line, rather than
/// ordinary visual-zone text — used by [`fit_content`] to decide what must
/// never be dropped when the prompt has to be cut down to fit the context
/// window. See [`MRZ_SHAPE_CHARSET`]'s doc comment for why this crate keeps
/// its own copy of this predicate instead of importing one.
fn looks_like_mrz(line: &str) -> bool {
    let compact: Vec<char> = line.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.len() < MRZ_SHAPE_MIN_LEN {
        return false;
    }
    let hits = compact
        .iter()
        .filter(|c| MRZ_SHAPE_CHARSET.contains(**c))
        .count();
    hits as f64 / compact.len() as f64 >= MRZ_SHAPE_MIN_CHARSET_FRACTION
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

    #[test]
    fn drops_long_run_noise_but_keeps_short_and_spaced_lines() {
        // A representative slice of Romania_Passport_Specimen_PE_ROU_2024_mrz.md's
        // guilloche microprint (issue #506).
        let noise = "MANOMANIAROMANIAROMANIAROMAMIAROMANIAROMANAROMANIAROMANIAOMAMLAROMANIAROMANIAROMANIAROMANIAROMANILBOMANIA";
        let label = "Date of birth: 29 OCT 1983 / Place: BUCURESTI";
        let sex_line = "F";
        let md = format!("{noise}\n{label}\n{sex_line}");
        let filtered = drop_long_run_noise(&md);
        assert!(
            !filtered.contains(noise),
            "the microprint run must go: {filtered}"
        );
        assert!(
            filtered.contains(label),
            "ordinary spaced prose must survive: {filtered}"
        );
        assert!(
            filtered.contains(sex_line),
            "a lone sex-value line must never be dropped on length alone: {filtered}"
        );
    }

    #[test]
    fn long_run_noise_rule_keeps_a_genuine_td3_mrz_line() {
        // 44 characters, ICAO 9303's longest defined MRZ line.
        let mrz = "PEROUPOPESCU<<MIHAELA<STEFANIA<<<<<<<<<<<<<<";
        assert_eq!(mrz.chars().count(), 44);
        assert!(!is_long_run_noise(mrz));
    }

    #[test]
    fn looks_like_mrz_accepts_ocr_corrupted_chevrons() {
        // OCR reading `<` as `?`, as `HOLDOUT_MRZ_CHARSET`'s sibling in
        // tests/parity.rs documents happening in this corpus.
        let corrupted = "PEROUPOPESCU??MIHAELA?STEFANIA????????????????";
        assert!(looks_like_mrz(corrupted));
    }

    #[test]
    fn looks_like_mrz_rejects_short_or_prose_lines() {
        assert!(!looks_like_mrz("F"));
        assert!(!looks_like_mrz(
            "Date of birth: 29 OCT 1983 / Place: BUCURESTI"
        ));
    }

    /// Fake token counter for the tests below: one "token" per byte. Any
    /// deterministic, injective-enough function of the string would do —
    /// this one makes budgets easy to reason about without a model.
    fn byte_counter(s: &str) -> usize {
        s.len()
    }

    #[test]
    fn build_prompt_within_budget_returns_the_unmodified_prompt_when_it_fits() {
        let prompt = build_prompt_within_budget("small ocr text", None, 10_000, 500, byte_counter)
            .expect("small content always fits a generous budget");
        assert_eq!(prompt, build_prompt("small ocr text", None));
    }

    #[test]
    fn build_prompt_within_budget_keeps_mrz_lines_and_fills_remaining_budget_in_document_order() {
        let mrz_line = "PEROUPOPESCU<<MIHAELA<STEFANIA<<<<<<<<<<<<<<";
        let filler_before = "ORDER-BEFORE-MARKER";
        let filler_after = "ORDER-AFTER-MARKER-THAT-IS-MUCH-LONGER-THAN-THE-FIRST-FILLER";
        let content = format!("{filler_before}\n{mrz_line}\n{filler_after}");

        // Sized off the actual template (not a hardcoded number) so this
        // test does not rot if SYSTEM/TEMPLATE grows: wide enough for the
        // MRZ-only prompt plus the shorter filler, not the longer one.
        let mrz_only = build_prompt_within_budget(mrz_line, None, u32::MAX, 0, byte_counter)
            .expect("mrz-only prompt fits an unbounded budget");
        let n_ctx = (mrz_only.len() + filler_before.len() + 1) as u32;

        let out = build_prompt_within_budget(&content, None, n_ctx, 0, byte_counter)
            .expect("fits once the longer filler is dropped");
        assert!(
            out.contains(mrz_line),
            "must keep the MRZ-shaped line: {out}"
        );
        assert!(
            out.contains(filler_before),
            "the earlier, cheaper filler line should survive: {out}"
        );
        assert!(
            !out.contains(filler_after),
            "the later, more expensive filler line should be dropped: {out}"
        );
    }

    #[test]
    fn build_prompt_within_budget_errors_when_even_the_mrz_lines_do_not_fit() {
        let mrz_line = "PEROUPOPESCU<<MIHAELA<STEFANIA<<<<<<<<<<<<<<";
        let err = build_prompt_within_budget(mrz_line, None, 10, 0, byte_counter)
            .expect_err("a 10-token budget cannot hold this prompt even alone");
        assert!(err.contains("MRZ-shaped"), "{err}");
    }

    #[test]
    fn hint_text_counts_against_the_budget_and_is_never_the_thing_dropped() {
        let content = "small ocr text";
        let hint = "document_number=L898902C3";
        let unhinted = build_prompt(content, None);
        // Exactly enough for the un-hinted prompt — content has no
        // MRZ-shaped line to protect, so once the hint is added the only
        // lever is dropping the one content line, and that alone cannot
        // make room because the hint itself, which is never dropped, is
        // what pushed the prompt over budget.
        let n_ctx = unhinted.len() as u32;

        let err = build_prompt_within_budget(content, Some(hint), n_ctx, 0, byte_counter)
            .expect_err(
                "the hint has no content line to trade away, so adding it must not fit for free",
            );
        assert!(err.contains("MRZ-shaped"), "{err}");
    }
}
