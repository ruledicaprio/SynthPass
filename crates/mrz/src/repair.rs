//! Candidate generation for MRZ lines damaged badly enough that characters are
//! *missing*, not merely misread.
//!
//! The check digits are the oracle everywhere in this crate; nothing here
//! decides what a character is. This module only widens the set of candidates
//! the oracle gets to rule on, and reports honestly when the oracle cannot
//! separate them.
//!
//! # The gap this closes
//!
//! [`crate::checksum`]'s existing length repair inserts missing characters in
//! exactly two places: inside the longest `<` filler run, or appended at the
//! end. That covers the common OCR failure — a truncated trailing filler run —
//! and nothing else. Two failures measured on real documents fall outside it:
//!
//! - A hole punched through an ID card's MRZ. `ocrs` **drops** the destroyed
//!   glyph rather than emitting a placeholder, so a TD1 line 2 arrives 29
//!   characters wide with the deficit in the middle of the expiry field.
//! - A finger over the start of a passport's line 2, arriving 42 characters
//!   wide with the deficit at the front.
//!
//! In both cases every candidate the old repair builds still has the data
//! shifted, so every check digit correctly rejects all of them, and a
//! recoverable document falls through to a non-deterministic fallback.
//! [`width_candidates`] restores the width by inserting [`UNKNOWN`] at *every*
//! position, and [`solve_field`] resolves those unknowns against the field's
//! own check digit.
//!
//! # What "resolved" is allowed to mean
//!
//! A check digit sees only the value of a field mod 10 (ICAO 9303 part 3
//! §4.9), so a single unknown position generally admits **four** characters —
//! one residue class from [`crate::CLASSES`]. On the punched ID card the four
//! are `3`, `D`, `N`, `X`. Three of them make the expiry field `D01230`,
//! `N01230`, `X01230`, which are not dates: [`FieldKind::Date`] prunes them
//! with [`crate::Date::is_well_formed`] and the recovery is unique.
//!
//! Where the arithmetic genuinely cannot separate the candidates —
//! the finger-occluded passport leaves 138 surviving pairs — the answer is
//! [`Resolution::Ambiguous`], never a pick. A guess that happens to be wrong
//! is worse than a refusal, because it is indistinguishable from a proof.
//!
//! # A third shape: correct width, one glyph wrong
//!
//! Neither of the above covers a line that arrived the *correct* width with
//! one glyph misread — no insertion or deletion, just OCR confusing e.g. `0`
//! and `O`. This looks like it should be *easier* than the missing-character
//! case, but the arithmetic makes it the more dangerous one to get sloppy
//! about: because the check-digit weights 7, 3, and 1 are each coprime to 10,
//! whether two characters are interchangeable at a position depends only on
//! their ICAO value mod 10, *never* on the weight — so a full residue class
//! (see [`crate::CLASSES`]) is check-digit-equivalent at every position, not
//! just some. `0`, `A`, `K`, and `U` all share value ≡ 0 (mod 10), so an
//! unrestricted single-position sweep of the whole alphabet would accept `A`
//! and `K` as readings of a misprinted `0` just as readily as the `O` a real
//! OCR engine would actually emit. [`substitution_candidates`] sweeps a
//! bounded, hand-built table of *visually* confusable characters instead of
//! the full alphabet, so [`solve_substitution`]'s answers stay restricted to
//! misreads OCR plausibly makes, not every character sharing a residue.

use crate::checksum::{fit_length, is_mrz_charset, verify};
use crate::dates::Date;

/// Marks a position the recognizer could not read. Deliberately **not** `<`:
/// the filler is a legitimate MRZ value (0), so reusing it would make "no
/// character here" indistinguishable from "the character here is a filler".
/// Outside the ICAO alphabet by design, so a string still carrying one can
/// never be mistaken for a parseable line.
///
/// ```
/// use mrz::{solve_field, width_candidates, FieldKind, UNKNOWN};
///
/// // A date of birth that lost a glyph to a punched hole arrives one short.
/// // Mark where the glyph could have been, then let the check digit decide.
/// let candidates = width_candidates("74082", 6);
/// assert!(candidates.contains(&format!("7408{UNKNOWN}2")));
/// assert_eq!(
///     solve_field(&format!("7408{UNKNOWN}2"), '2', FieldKind::Date).unique(),
///     Some("740812"),
/// );
///
/// // A string still carrying one is never a parseable MRZ field.
/// assert!(mrz::check_digit(&format!("7408{UNKNOWN}2")).is_err());
/// ```
pub const UNKNOWN: char = '?';

/// The 37-character ICAO 9303 MRZ alphabet, in the order the solver sweeps it.
///
/// ```
/// assert_eq!(mrz::MRZ_ALPHABET.len(), 37);
/// // Exactly the characters a check digit accepts — nothing more.
/// assert!(mrz::MRZ_ALPHABET.chars().all(|c| mrz::check_digit(&c.to_string()).is_ok()));
/// assert!(mrz::check_digit("a").is_err()); // lowercase is not MRZ
/// ```
pub const MRZ_ALPHABET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ<";

/// Most [`UNKNOWN`] positions [`solve_field`] will sweep in one field.
///
/// Two is 37² = 1369 check-digit verifications — trivial — and three would be
/// 50653 candidates of which a useful fraction survive, i.e. an `Ambiguous`
/// answer so wide it carries no information. The bound exists for the same
/// reason [`crate::checksum`]'s defiller has one: this runs inside an OCR
/// retry loop, and an unbounded sweep there is a latency bug waiting for a
/// bad photo.
const MAX_UNKNOWNS: usize = 2;

/// Most characters [`width_candidates`] will insert. Past this the position
/// sweep stops being a repair and starts being a search over lines that were
/// never read in the first place.
const MAX_WIDTH_DEFICIT: usize = 2;

/// Which ICAO field a [`solve_field`] call is resolving. Selects the
/// structural constraint applied *after* the check digit, never instead of it.
///
/// ```
/// use mrz::{solve_field, FieldKind};
///
/// // The same damaged field under two structural rules. Four glyphs satisfy
/// // the check digit at the lost position — `2`, `C`, `M` and `W` — but only
/// // a digit makes a date.
/// assert_eq!(solve_field("74081?", '2', FieldKind::Date).unique(), Some("740812"));
/// assert_eq!(solve_field("74081?", '2', FieldKind::Other).unique(), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FieldKind {
    /// Left-justified, `<`-padded (ICAO 9303 part 4 §4.2.2): a filler may only
    /// appear in the trailing run, so `B<98730<` is not a document number
    /// however well its check digit verifies.
    DocumentNumber,
    /// `YYMMDD`. Must be six digits naming a real calendar date.
    Date,
    /// Left-justified and `<`-padded like [`FieldKind::DocumentNumber`].
    PersonalNumber,
    /// No constraint beyond the check digit and the MRZ alphabet.
    Other,
}

/// What the check digit could prove about a field carrying [`UNKNOWN`]s.
///
/// ```
/// use mrz::{solve_field, FieldKind, Resolution};
///
/// // The ICAO specimen document number `L898902C3` with one glyph unread.
/// // Every character in the lost `0`'s residue class verifies, and a document
/// // number may legitimately hold letters — so nothing separates them.
/// match solve_field("L8989?2C3", '6', FieldKind::DocumentNumber) {
///     Resolution::Ambiguous { candidates } => {
///         assert_eq!(candidates, ["L898902C3", "L8989A2C3", "L8989K2C3", "L8989U2C3"]);
///     }
///     other => panic!("expected an ambiguous answer, got {other:?}"),
/// }
///
/// // A check digit that was itself unread proves nothing.
/// assert_eq!(solve_field("L898902C3", '?', FieldKind::DocumentNumber), Resolution::Unresolvable);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Resolution {
    /// Exactly one reading satisfies the check digit and the field's
    /// structural constraint. This is a proof, not a preference — of
    /// *uniqueness within the candidate set*, which is built from the
    /// [`UNKNOWN`] positions the caller marked. A character misread without
    /// being marked is not in that set, so the uniqueness is conditional on
    /// the damage model the caller supplied.
    Unique(String),
    /// Several readings satisfy both and nothing in the MRZ can separate them
    /// — the check digit's blindspot, made explicit. Sorted, so the output is
    /// stable across runs.
    Ambiguous {
        /// Every reading that satisfies both the check digit and the
        /// field's structural constraint, sorted for a stable order.
        candidates: Vec<String>,
    },
    /// No reading satisfies the check digit, or the input was outside what
    /// this solver will attempt (too many unknowns, an unreadable check digit,
    /// non-ASCII input).
    Unresolvable,
}

impl Resolution {
    /// The proven reading, if there is exactly one. `Ambiguous` deliberately
    /// yields `None` — a caller that wants "the first candidate" has to reach
    /// into the variant and own that decision explicitly.
    ///
    /// ```
    /// use mrz::{solve_field, FieldKind};
    ///
    /// // A date has one digit per residue class, so one lost glyph is provable.
    /// assert_eq!(solve_field("7?0812", '2', FieldKind::Date).unique(), Some("740812"));
    /// // A document number does not: four readings verify, none is "the" one.
    /// assert_eq!(solve_field("L8989?2C3", '6', FieldKind::DocumentNumber).unique(), None);
    /// ```
    pub fn unique(&self) -> Option<&str> {
        match self {
            Resolution::Unique(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// Every way `line` could be restored to exactly `target` characters.
///
/// Combines the checksum module's existing filler-run and end-padding
/// candidates with a sweep that inserts [`UNKNOWN`] at each of the `len + 1`
/// positions — the case a punched hole or an occluded line start produces.
/// Candidates containing [`UNKNOWN`] are not parseable as they stand; resolve
/// them field-by-field with [`solve_field`] first.
///
/// Bounded and deterministic: at most `target + 4` candidates, each exactly
/// `target` characters, in a stable order. Returns empty for non-ASCII input
/// or a deficit wider than the module's insertion bound.
///
/// ```
/// use mrz::{width_candidates, UNKNOWN};
///
/// let cands = width_candidates("ABCDE", 6);
/// // Every candidate is exactly the target width.
/// assert!(cands.iter().all(|c| c.chars().count() == 6));
/// // The `?` is tried at every insertion position, including both ends.
/// assert!(cands.contains(&format!("{UNKNOWN}ABCDE")));
/// assert!(cands.contains(&format!("ABCDE{UNKNOWN}")));
/// ```
pub fn width_candidates(line: &str, target: usize) -> Vec<String> {
    let n: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    if !n.is_ascii() || target == 0 {
        return Vec::new();
    }
    let mut out: Vec<String> = Vec::new();

    // The pre-existing behaviour first, so a line this module could already
    // handle keeps resolving through the same candidate it always did.
    for candidate in fit_length(&n, target) {
        push_unique(&mut out, candidate, target);
    }

    let len = n.len();
    if len < target {
        let deficit = target - len;
        if deficit <= MAX_WIDTH_DEFICIT {
            let bytes = n.as_bytes();
            for at in 0..=len {
                let mut s = String::with_capacity(target);
                s.push_str(&n[..at]);
                for _ in 0..deficit {
                    s.push(UNKNOWN);
                }
                s.push_str(&n[at..]);
                debug_assert_eq!(bytes.len() + deficit, s.len());
                push_unique(&mut out, s, target);
            }
        }
    }
    out
}

/// Append `candidate` if it is exactly `target` characters and not already
/// present — the de-duplication [`width_candidates`] needs because the
/// existing filler-run repair and the position sweep overlap whenever the
/// deficit happens to sit inside a filler run.
fn push_unique(out: &mut Vec<String>, candidate: String, target: usize) {
    if candidate.len() == target && !out.contains(&candidate) {
        out.push(candidate);
    }
}

/// Resolve the [`UNKNOWN`] positions in one check-digited field.
///
/// Sweeps [`MRZ_ALPHABET`] over every unknown position and keeps the readings
/// whose own check digit verifies *and* which satisfy `kind`'s structural
/// constraint. A field with no unknowns is simply verified.
///
/// Returns [`Resolution::Unresolvable`] rather than attempting a solve when
/// `check` is itself [`UNKNOWN`]: a check digit that was not read cannot prove
/// anything, and recomputing it from the candidate would only prove the
/// candidate agrees with itself.
///
/// ```
/// use mrz::{solve_field, FieldKind, Resolution, UNKNOWN};
///
/// // ICAO 9303 specimen date of birth (740812, check digit 2) with one
/// // glyph unread. The residue class for that position also contains two
/// // letters, but `FieldKind::Date` requires six ASCII digits, so only the
/// // digit reading survives.
/// let field = format!("740{UNKNOWN}12");
/// assert_eq!(
///     solve_field(&field, '2', FieldKind::Date),
///     Resolution::Unique("740812".to_string()),
/// );
/// ```
pub fn solve_field(field: &str, check: char, kind: FieldKind) -> Resolution {
    if !field.is_ascii() || check == UNKNOWN {
        return Resolution::Unresolvable;
    }
    let chars: Vec<char> = field.chars().collect();
    let unknowns: Vec<usize> = chars
        .iter()
        .enumerate()
        .filter(|(_, c)| **c == UNKNOWN)
        .map(|(i, _)| i)
        .collect();
    if unknowns.len() > MAX_UNKNOWNS {
        return Resolution::Unresolvable;
    }
    // Anything that is neither a known MRZ character nor an explicit unknown
    // is corruption this module has no business guessing around.
    if chars
        .iter()
        .any(|c| *c != UNKNOWN && !is_mrz_charset(&c.to_string()))
    {
        return Resolution::Unresolvable;
    }

    let alphabet: Vec<char> = MRZ_ALPHABET.chars().collect();
    let mut hits: Vec<String> = Vec::new();
    let total = alphabet.len().pow(unknowns.len() as u32);
    for combo in 0..total {
        let mut candidate = chars.clone();
        let mut rest = combo;
        for &pos in &unknowns {
            candidate[pos] = alphabet[rest % alphabet.len()];
            rest /= alphabet.len();
        }
        let s: String = candidate.into_iter().collect();
        if verify(&s, check) && satisfies(&s, kind) {
            hits.push(s);
        }
    }

    hits.sort();
    match hits.len() {
        0 => Resolution::Unresolvable,
        1 => Resolution::Unique(hits.remove(0)),
        _ => Resolution::Ambiguous { candidates: hits },
    }
}

/// Positions [`substitution_candidates`] will change in one candidate.
///
/// Fixed at one by design, not merely by default: [`CONFUSABLES`] already
/// bounds each position to a handful of visually plausible alternatives, but
/// letting two positions vary independently multiplies those small counts
/// together and starts re-admitting the coincidental agreement the table
/// exists to keep out. A field with two misread glyphs is better served by
/// two separate passes — or by [`solve_field`] once the still-wrong position
/// is known — than by a search that quietly widens what "one repair" means.
const MAX_SUBSTITUTIONS: usize = 1;

/// Most candidates [`substitution_candidates`] will return.
///
/// A sibling of [`MAX_UNKNOWNS`]/[`MAX_WIDTH_DEFICIT`]: this runs inside the
/// same OCR retry loop, so the sweep — at most `field.len() * 4` candidates
/// given [`CONFUSABLES`]'s widest row — needs a hard ceiling rather than trust
/// that no field ever gets long enough to matter.
const MAX_SUBSTITUTION_CANDIDATES: usize = 256;

/// Bounded table of OCR glyph confusions, `(character, confusable-with)`.
///
/// Each entry is bidirectional: `('0', "ODQ")` means `0` may be misread as
/// `O`, `D`, or `Q` *and* that any of `O`, `D`, `Q` may be misread as `0` —
/// the crate's internal `confusable_alternatives` walks the table both ways,
/// so the digit and
/// its lookalike letters do not each need their own row.
///
/// Deliberately small and shape-driven (round vs. round, vertical stroke vs.
/// vertical stroke), not derived from [`crate::CLASSES`]. A residue class is
/// check-digit-equivalent at *every* weight (see [`crate::Blindspot`]), but `K` and
/// `U` are not plausible misreads of `0` — restricting this table to glyphs
/// that actually look alike is what keeps [`solve_substitution`]'s answers
/// meaningfully unique instead of reproducing the whole residue class.
///
/// Most rows pair a digit with letters in its own residue class, so a swap
/// leaves the check digit unchanged and the *structural* constraint
/// ([`FieldKind`]) is what separates the readings. `2`/`7` and `M`/`N` are the
/// exceptions — they are genuine stroke-shape confusions (an `ocrs` measurement
/// on real specimens: `India_..._P0_IND_2013` reads a printed `2` as `7`;
/// `Ghana_..._P0_GHA_2019` reads the sex `M` as `N`) that *cross* residues, so
/// there the check digit itself rejects the wrong reading. Both directions are
/// still worth carrying: the point of the table is which glyph OCR plausibly
/// emitted, and the arithmetic is a separate gate downstream.
///
/// ```
/// use mrz::{solve_substitution, FieldKind, Resolution, CONFUSABLES};
///
/// // Rows are bidirectional: `0` may be read as `O`, `D` or `Q`, and back.
/// assert!(CONFUSABLES.contains(&('0', "ODQ")));
///
/// // `2` ↔ `7` crosses residue classes, so the check digit alone rejects
/// // the wrong reading: expiry 120415 (check digit 9) misread as 170415.
/// assert_eq!(
///     solve_substitution("170415", '9', FieldKind::Date),
///     Resolution::Unique("120415".to_string()),
/// );
/// ```
pub const CONFUSABLES: &[(char, &str)] = &[
    ('0', "ODQ"),
    ('1', "ILTU"),
    ('2', "Z7"),
    ('4', "A"),
    ('5', "S"),
    ('6', "G"),
    ('7', "T2"),
    ('8', "B"),
    ('9', "G"),
    ('M', "N"),
];

/// Every character [`CONFUSABLES`] lists as a plausible misread of `c`, in
/// both directions, excluding `c` itself.
fn confusable_alternatives(c: char) -> Vec<char> {
    let mut alts: Vec<char> = Vec::new();
    for &(key, group) in CONFUSABLES {
        if c == key {
            for g in group.chars() {
                if g != c && !alts.contains(&g) {
                    alts.push(g);
                }
            }
        } else if group.contains(c) && key != c && !alts.contains(&key) {
            alts.push(key);
        }
    }
    alts
}

/// Every single-glyph substitution of `field` under [`CONFUSABLES`].
///
/// Sweeps positions left to right; at each position, tries every plausible
/// confusable in place of the character actually there (an internal
/// `MAX_SUBSTITUTIONS` cap keeps this to one changed position per candidate —
/// never two at once).
/// Non-ASCII input or a field still carrying [`UNKNOWN`] yields nothing: a
/// position this module cannot read is [`solve_field`]'s problem, not this
/// one's. The identity reading (`field` itself, unchanged) is never included
/// — this function only proposes *different* readings for [`solve_substitution`]
/// to test. Bounded by an internal `MAX_SUBSTITUTION_CANDIDATES` cap, in a
/// stable order.
///
/// ```
/// // `CONFUSABLES` pairs `0` with the round letters O, D, Q.
/// assert_eq!(mrz::substitution_candidates("0"), ["O", "D", "Q"]);
/// // The identity reading is never proposed.
/// assert!(!mrz::substitution_candidates("0").contains(&"0".to_string()));
/// // A field still carrying an unknown position is not this function's job.
/// assert!(mrz::substitution_candidates("A?C").is_empty());
/// ```
pub fn substitution_candidates(field: &str) -> Vec<String> {
    if !field.is_ascii() || field.contains(UNKNOWN) {
        return Vec::new();
    }
    let chars: Vec<char> = field.chars().collect();
    let mut out: Vec<String> = Vec::new();
    'positions: for i in 0..chars.len() {
        for alt in confusable_alternatives(chars[i]) {
            let mut candidate = chars.clone();
            candidate[i] = alt;
            let s: String = candidate.into_iter().collect();
            debug_assert_eq!(
                s.chars().zip(field.chars()).filter(|(a, b)| a != b).count(),
                MAX_SUBSTITUTIONS,
                "substitution_candidates must change exactly one position"
            );
            if s != field && !out.contains(&s) {
                out.push(s);
                if out.len() >= MAX_SUBSTITUTION_CANDIDATES {
                    break 'positions;
                }
            }
        }
    }
    out
}

/// Resolve a field that is the *right width* but may carry one misread glyph.
///
/// A field that already verifies is returned unchanged — this never
/// "repairs" a field the check digit has already proven, only one it has
/// rejected. Otherwise sweeps [`substitution_candidates`], keeping readings
/// whose check digit verifies *and* which satisfy `kind`'s structural
/// constraint, exactly as [`solve_field`] does for [`UNKNOWN`] positions.
///
/// Returns [`Resolution::Unresolvable`] for non-ASCII input or an unreadable
/// check digit, for the same reason [`solve_field`] does: a check digit that
/// was not itself read faithfully cannot prove anything about the field next
/// to it.
///
/// ```
/// use mrz::{solve_substitution, FieldKind, Resolution};
///
/// // The ICAO specimen DOB (740812, check digit 2) misread `2` → `Z`.
/// // The only confusable swap that verifies restores the digit.
/// assert_eq!(
///     solve_substitution("74081Z", '2', FieldKind::Date),
///     Resolution::Unique("740812".to_string()),
/// );
/// // A field whose check digit already verifies is returned unchanged.
/// assert_eq!(
///     solve_substitution("740812", '2', FieldKind::Date),
///     Resolution::Unique("740812".to_string()),
/// );
/// ```
pub fn solve_substitution(field: &str, check: char, kind: FieldKind) -> Resolution {
    if !field.is_ascii() || check == UNKNOWN {
        return Resolution::Unresolvable;
    }
    if verify(field, check) && satisfies(field, kind) {
        return Resolution::Unique(field.to_string());
    }

    let mut hits: Vec<String> = Vec::new();
    for candidate in substitution_candidates(field) {
        if verify(&candidate, check) && satisfies(&candidate, kind) && !hits.contains(&candidate) {
            hits.push(candidate);
        }
    }

    hits.sort();
    match hits.len() {
        0 => Resolution::Unresolvable,
        1 => Resolution::Unique(hits.remove(0)),
        _ => Resolution::Ambiguous { candidates: hits },
    }
}

/// Every way one confusable class could be swept uniformly across `field`
/// *and* its own check-digit cell.
///
/// [`solve_substitution`] varies exactly one position at a time
/// (`MAX_SUBSTITUTIONS` stays fixed at one — see its doc comment for why).
/// This function instead varies *every* occurrence of exactly one character
/// at once, including the check-digit cell if it happens to carry the same
/// character. The two are additive by construction, never in competition: a
/// character occurring only once is [`solve_substitution`]'s case, so this
/// function requires **at least two** occurrences of a character across
/// `field` and `check_digit` combined before it will consider sweeping it.
/// A uniform sweep of one class is a single decision, not the independent
/// multi-position search `MAX_SUBSTITUTIONS` exists to forbid, which is why
/// it sits beside that cap rather than raising it.
///
/// Measured motivation: a TD1 card back whose document number is nine cells
/// of one digit plus a check-digit cell of the same digit, all read as one
/// *letter* from that digit's [`CONFUSABLES`] row. Line 2 of the same card
/// reads the identical glyph correctly 16 times elsewhere, so this is not
/// legibility — it is field alphabet: the misread letter is legal wherever
/// OCR emitted it, and nothing downstream rejects it until the check digit is
/// swept along with the rest of the run.
///
/// # Why this is field-scoped, never line-scoped
///
/// A TD1 line 1 carries both a document code and a document number, but the
/// document code carries **no check digit of its own**. Sweeping a whole
/// *line* would rewrite a letter that is correct there, and the result would
/// validate while being wrong — precisely what [`CONFUSABLES`]'s own doc
/// comment warns a residue-class sweep can do. This function only ever sees
/// one already-check-digited field, so it can never reach past it; do not
/// build a line-scoped version of this by extending
/// [`substitution_candidates`].
///
/// # What it refuses
///
/// - Non-ASCII input, or a `field` still carrying [`UNKNOWN`]: not this
///   module's problem to guess around.
/// - `check_digit == `[`UNKNOWN`]: an unread check digit proves nothing.
/// - A `field` whose printed check digit already verifies under `kind`: this
///   never "repairs" a reading the arithmetic has already proven, so it can
///   never be tempted by a class that would *coincidentally* also validate.
/// - A character occurring only once across `field` and `check_digit`: that
///   is [`solve_substitution`]'s job, and the two must never both propose an
///   answer for the same position.
/// - A swept reading whose own checksum fails to verify.
/// - A swept reading that fails `kind`'s structural check even though its
///   checksum verifies — a filler in the interior of a
///   [`FieldKind::DocumentNumber`]/[`FieldKind::PersonalNumber`], or six
///   digits naming no real calendar date under [`FieldKind::Date`].
/// - More than one surviving class: returns [`Resolution::Ambiguous`] rather
///   than choosing, exactly as [`solve_substitution`] does. A guess that
///   happens to be wrong is indistinguishable from a proof.
///
/// Bounded by `MAX_SUBSTITUTION_CANDIDATES`, the same ceiling
/// [`substitution_candidates`] uses.
///
/// ```
/// use mrz::{solve_class_sweep, FieldKind, Resolution};
///
/// // Synthetic TD1-style document number, built from shape rather than
/// // copied from any real document: nine identical digits plus a
/// // check-digit cell of the same digit, all misread by OCR as one
/// // confusable letter -- the failure measured on a real TD1 card back,
/// // where a field-alphabet gap let the letter through until the check
/// // digit was swept along with the rest of the run.
/// let misread = "OOOOOOOOO"; // nine letters where nine digits were printed
/// assert_eq!(
///     solve_class_sweep(misread, 'O', FieldKind::DocumentNumber),
///     Resolution::Unique("000000000".to_string()),
/// );
///
/// // A single occurrence is out of scope -- that is `solve_substitution`'s
/// // case, and the two never compete for the same position.
/// assert_eq!(
///     solve_class_sweep("A0B", '1', FieldKind::Other),
///     Resolution::Unresolvable,
/// );
/// ```
pub fn solve_class_sweep(field: &str, check_digit: char, kind: FieldKind) -> Resolution {
    if !field.is_ascii() || field.contains(UNKNOWN) || check_digit == UNKNOWN {
        return Resolution::Unresolvable;
    }
    if verify(field, check_digit) && satisfies(field, kind) {
        // Already a faithful read. Never sweep a field the check digit has
        // already proven correct, even if some class would coincidentally
        // also validate -- see the "field-scoped" section above.
        return Resolution::Unresolvable;
    }

    let chars: Vec<char> = field.chars().collect();
    let mut classes: Vec<char> = Vec::new();
    for &c in chars.iter().chain(std::iter::once(&check_digit)) {
        if !classes.contains(&c) {
            classes.push(c);
        }
    }

    let mut hits: Vec<String> = Vec::new();
    'classes: for x in classes {
        let occurrences = chars.iter().filter(|&&c| c == x).count() + usize::from(check_digit == x);
        if occurrences < 2 {
            continue;
        }
        for y in confusable_alternatives(x) {
            let swept_field: String = chars.iter().map(|&c| if c == x { y } else { c }).collect();
            let swept_check = if check_digit == x { y } else { check_digit };
            if verify(&swept_field, swept_check)
                && satisfies(&swept_field, kind)
                && !hits.contains(&swept_field)
            {
                hits.push(swept_field);
                if hits.len() >= MAX_SUBSTITUTION_CANDIDATES {
                    break 'classes;
                }
            }
        }
    }

    hits.sort();
    match hits.len() {
        0 => Resolution::Unresolvable,
        1 => Resolution::Unique(hits.remove(0)),
        _ => Resolution::Ambiguous { candidates: hits },
    }
}

/// Every concrete reading of a line still carrying [`UNKNOWN`]s, for callers
/// that would rather let a whole-record parse be the oracle than resolve field
/// by field.
///
/// Bounded by [`MAX_UNKNOWNS`] exactly as [`solve_field`] is — a line with more
/// unknowns than that yields nothing rather than a combinatorial expansion.
/// A line with no unknowns yields itself, so this is safe to call
/// unconditionally.
pub(crate) fn concrete_fillings(line: &str) -> Vec<String> {
    if !line.is_ascii() {
        return Vec::new();
    }
    let chars: Vec<char> = line.chars().collect();
    let unknowns: Vec<usize> = chars
        .iter()
        .enumerate()
        .filter(|(_, c)| **c == UNKNOWN)
        .map(|(i, _)| i)
        .collect();
    if unknowns.is_empty() {
        return vec![line.to_string()];
    }
    if unknowns.len() > MAX_UNKNOWNS {
        return Vec::new();
    }
    let alphabet: Vec<char> = MRZ_ALPHABET.chars().collect();
    let mut out = Vec::with_capacity(alphabet.len().pow(unknowns.len() as u32));
    for combo in 0..alphabet.len().pow(unknowns.len() as u32) {
        let mut candidate = chars.clone();
        let mut rest = combo;
        for &pos in &unknowns {
            candidate[pos] = alphabet[rest % alphabet.len()];
            rest /= alphabet.len();
        }
        out.push(candidate.into_iter().collect());
    }
    out
}

/// The structural constraint for `kind` — applied only to readings the check
/// digit has already accepted, so it can never rescue a value the arithmetic
/// rejected, only narrow a set the arithmetic could not separate.
fn satisfies(field: &str, kind: FieldKind) -> bool {
    match kind {
        FieldKind::Other => true,
        FieldKind::DocumentNumber | FieldKind::PersonalNumber => left_justified(field),
        FieldKind::Date => is_plausible_yymmdd(field),
    }
}

/// No `<` appears before a non-filler character: ICAO left-justifies these
/// fields and pads them on the right only.
fn left_justified(field: &str) -> bool {
    !field.trim_end_matches('<').contains('<')
}

/// Six digits naming a real calendar date.
///
/// The century is unknown at field level — the pivot lives in
/// [`crate::ParseOptions`] and belongs to the parser, not here — so a date is
/// accepted when it is well-formed under **either** century. That only matters
/// for `yy = 00` on 29 February (1900 is not a leap year, 2000 is), and
/// accepting the union is the conservative choice: this function's job is to
/// discard the impossible, never to narrow by guessing an era.
fn is_plausible_yymmdd(field: &str) -> bool {
    if field.len() != 6 || !field.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let yy: i32 = field[0..2].parse().expect("two ascii digits");
    let month: u32 = field[2..4].parse().expect("two ascii digits");
    let day: u32 = field[4..6].parse().expect("two ascii digits");
    Date::new(1900 + yy, month, day).is_well_formed()
        || Date::new(2000 + yy, month, day).is_well_formed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checksum::check_digit;

    #[test]
    fn width_candidates_are_all_target_width_and_deduplicated() {
        for target in [30usize, 36, 44] {
            let short: String = "A".repeat(target - 1);
            let candidates = width_candidates(&short, target);
            assert!(!candidates.is_empty());
            for c in &candidates {
                assert_eq!(c.len(), target, "candidate {c:?} is not {target} wide");
            }
            let mut sorted = candidates.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), candidates.len(), "duplicate candidates");
        }
    }

    #[test]
    fn width_candidates_sweep_every_insertion_point() {
        // A deficit of one in a line with no filler run at all: the old
        // filler-run repair has nothing to inflate, so every candidate here
        // comes from the position sweep.
        let candidates = width_candidates("ABCDE", 6);
        assert!(candidates.contains(&"?ABCDE".to_string()));
        assert!(candidates.contains(&"AB?CDE".to_string()));
        assert!(candidates.contains(&"ABCDE?".to_string()));
    }

    #[test]
    fn width_candidates_refuses_a_deficit_it_cannot_bound() {
        let candidates = width_candidates("AB", 30);
        assert!(
            candidates.iter().all(|c| !c.contains(UNKNOWN)),
            "a 28-character deficit must not be swept position by position"
        );
    }

    #[test]
    fn a_field_with_no_unknowns_is_just_verified() {
        // ICAO 9303 part 3 worked example.
        assert_eq!(
            solve_field("L898902C<", '3', FieldKind::DocumentNumber),
            Resolution::Unique("L898902C<".to_string())
        );
        assert_eq!(
            solve_field("L898902C<", '4', FieldKind::DocumentNumber),
            Resolution::Unresolvable
        );
    }

    #[test]
    fn an_unreadable_check_digit_proves_nothing() {
        assert_eq!(
            solve_field("6908?22", UNKNOWN, FieldKind::Other),
            Resolution::Unresolvable
        );
    }

    #[test]
    fn too_many_unknowns_is_refused_not_attempted() {
        assert_eq!(
            solve_field("???????", '0', FieldKind::Other),
            Resolution::Unresolvable
        );
    }

    #[test]
    fn left_justified_rejects_an_interior_filler() {
        assert!(left_justified("B1987309"));
        assert!(left_justified("B198730<"));
        assert!(!left_justified("B<987309"));
    }

    #[test]
    fn plausible_dates_reject_impossible_calendars() {
        assert!(!is_plausible_yymmdd("890229")); // 1989/2089 both non-leap
        assert!(is_plausible_yymmdd("000229")); // 2000 is a leap year
        assert!(is_plausible_yymmdd("301230"));
        assert!(!is_plausible_yymmdd("301332"));
        assert!(!is_plausible_yymmdd("D01230"));
        assert!(!is_plausible_yymmdd("30123"));
    }

    #[test]
    fn confusable_alternatives_are_bidirectional_and_exclude_identity() {
        // '0' is a key row: its listed confusables come back directly.
        let mut zero = confusable_alternatives('0');
        zero.sort();
        assert_eq!(zero, vec!['D', 'O', 'Q']);
        // 'O' only appears inside '0's row, so it must resolve back to '0'
        // via the reverse direction, without needing its own row.
        assert_eq!(confusable_alternatives('O'), vec!['0']);
        // A character absent from every row has no confusables at all.
        assert!(confusable_alternatives('Y').is_empty());
    }

    #[test]
    fn substitution_candidates_never_includes_the_identity_reading() {
        for field in ["13E1AE", "AB0234C7", "L898902C", "OOOOOO"] {
            let candidates = substitution_candidates(field);
            assert!(
                !candidates.contains(&field.to_string()),
                "{field:?} candidates must never include the unchanged input"
            );
        }
    }

    #[test]
    fn substitution_candidates_respects_the_cap_on_a_worst_case_input() {
        // '1' is CONFUSABLES's widest row (I, L, T, U — four alternatives),
        // so a long run of '1's is the worst case for the sweep: 100
        // positions * 4 alternatives = 400 raw candidates, well past the cap.
        let field = "1".repeat(100);
        let candidates = substitution_candidates(&field);
        assert_eq!(candidates.len(), MAX_SUBSTITUTION_CANDIDATES);
    }

    #[test]
    fn solve_substitution_leaves_an_already_verifying_field_unchanged() {
        // ICAO 9303 part 3 worked example, reused from `solve_field`'s test:
        // a field that already checksums must never be "repaired".
        assert_eq!(
            solve_substitution("L898902C<", '3', FieldKind::DocumentNumber),
            Resolution::Unique("L898902C<".to_string())
        );
    }

    #[test]
    fn solve_substitution_resolves_a_single_confusable_glyph() {
        // Synthetic field, not drawn from any real document: the printed
        // field is "13E1AE" (check digit computed via `checksum::check_digit`
        // the same way an issuer's MRZ would be), and OCR misreads the
        // leading '1' as the visually similar 'I' — a `CONFUSABLES` pair.
        let field = "13E1AE";
        let check = '1';
        assert_eq!(check_digit(field).unwrap(), 1, "fixture's own check digit");
        let corrupted = "I3E1AE";
        assert!(
            !verify(corrupted, check),
            "the OCR misread must not verify as-is"
        );
        assert_eq!(
            solve_substitution(corrupted, check, FieldKind::Other),
            Resolution::Unique(field.to_string())
        );
    }

    #[test]
    fn solve_class_sweep_resolves_a_uniform_class_across_field_and_check_digit() {
        // Shape of the motivating TD1 document number: nine cells of one
        // digit plus a check-digit cell of the same digit, all misread as
        // one confusable letter. Constructed from shape, not copied from any
        // real document.
        let misread = "OOOOOOOOO"; // 9 letters, all-zero field printed
        assert_eq!(
            check_digit("000000000").unwrap(),
            0,
            "sanity: 9 zeros checksum to 0"
        );
        assert_eq!(
            solve_class_sweep(misread, 'O', FieldKind::DocumentNumber),
            Resolution::Unique("000000000".to_string()),
        );
    }

    #[test]
    fn solve_class_sweep_refuses_an_already_valid_field_even_when_a_class_would_also_validate() {
        // Mirrors the reason this repair must stay field-scoped: a field
        // that already checksums must never be swept, even when sweeping its
        // repeated class would *coincidentally* also validate. Two letters
        // sitting at the weight-7 and weight-3 positions of a two-character
        // field sum to a multiple of ten, so swapping the whole class leaves
        // the checksum completely unchanged -- both readings verify by
        // construction, not by luck, which is exactly what makes this a real
        // test of the guard rather than an accident of the fixture.
        let field = "OO";
        assert!(
            verify(field, '0'),
            "fixture must already validate as printed"
        );
        assert!(
            verify("00", '0'),
            "the swept reading would *also* validate -- the trap this guards against"
        );
        assert_eq!(
            solve_class_sweep(field, '0', FieldKind::Other),
            Resolution::Unresolvable,
            "a field that already checksums must never be swept, however tempting the class"
        );
    }

    #[test]
    fn solve_class_sweep_ignores_a_single_occurrence() {
        // 'A', '0' and 'B' each occur exactly once across the field and the
        // check digit, so none qualifies for a class sweep -- that is
        // `solve_substitution`'s case, and the two must never compete for
        // the same position.
        assert!(
            !confusable_alternatives('A').is_empty(),
            "the table does cover 'A'"
        );
        assert!(!verify("A0B", '9'), "fixture must not already validate");
        assert_eq!(
            solve_class_sweep("A0B", '9', FieldKind::Other),
            Resolution::Unresolvable,
        );
    }

    #[test]
    fn solve_class_sweep_reports_ambiguity_instead_of_guessing() {
        // Two independent classes ('O' -> '0' and 'M' -> 'N') each occur
        // twice, at positions engineered so *both* single-class sweeps
        // checksum to the same digit while the unswept field does not.
        // Neither the checksum nor `FieldKind::Other` can separate them, so
        // the honest answer is ambiguity, not a pick.
        let field = "<<O<<O<<M<<M";
        assert!(!verify(field, '4'), "fixture must not already validate");
        assert!(verify("<<0<<0<<M<<M", '4'), "the O-class sweep validates");
        assert!(
            verify("<<O<<O<<N<<N", '4'),
            "the M-class sweep validates too"
        );
        assert_eq!(
            solve_class_sweep(field, '4', FieldKind::Other),
            Resolution::Ambiguous {
                candidates: vec!["<<0<<0<<M<<M".to_string(), "<<O<<O<<N<<N".to_string()],
            }
        );
    }

    #[test]
    fn solve_class_sweep_rejects_an_interior_filler_in_a_document_number() {
        // Sweeping the repeated 'O' to '0' produces "0<0B", whose checksum
        // verifies against '7' -- but the untouched '<' sits before a
        // non-filler character, which `FieldKind::DocumentNumber` must
        // reject however well the arithmetic checks out.
        let field = "O<OB";
        assert!(!verify(field, '7'), "fixture must not already validate");
        assert!(
            verify("0<0B", '7'),
            "the swept reading's checksum does verify"
        );
        assert!(
            "0<0B".trim_end_matches('<').contains('<'),
            "fixture sanity: the swept reading really does carry an interior filler"
        );
        assert_eq!(
            solve_class_sweep(field, '7', FieldKind::DocumentNumber),
            Resolution::Unresolvable,
            "an interior filler must be rejected even though the checksum verifies",
        );
    }

    #[test]
    fn solve_class_sweep_rejects_a_swept_reading_that_names_no_calendar_date() {
        // Sweeping the repeated 'O' to '0' produces "001340" -- checksum
        // verifies against '4', but month 13 is not a real calendar month,
        // so `FieldKind::Date` must reject it even though the arithmetic
        // checks out.
        let field = "OO1340";
        // The letters make the printed field itself already fail
        // `FieldKind::Date` (not six digits), so the short-circuit guard
        // must not fire even though its checksum happens to verify.
        assert!(
            !(verify(field, '4') && is_plausible_yymmdd(field)),
            "the already-valid guard must not fire for this fixture"
        );
        assert!(
            verify("001340", '4'),
            "the swept reading's checksum does verify"
        );
        assert!(
            !is_plausible_yymmdd("001340"),
            "fixture sanity: month 13 is not real"
        );
        assert_eq!(
            solve_class_sweep(field, '4', FieldKind::Date),
            Resolution::Unresolvable,
            "an impossible calendar date must be rejected even though the checksum verifies",
        );
    }

    #[test]
    fn solve_class_sweep_leaves_cross_residue_rejection_untouched() {
        // '2'/'7' crosses residue classes (unlike most `CONFUSABLES` rows),
        // so the check digit itself -- not the table -- must reject the
        // wrong reading rather than the table doing it. Two '2's (at the
        // weight-7 and weight-1 positions) plus a check-digit cell that is
        // also '2', all misread uniformly as '7': the sweep must resolve
        // back to the digit reading and never to the letter 'T' also listed
        // for '7' -- 'T' is not an ASCII digit or filler, so it can never be
        // a valid check-digit cell.
        let true_field = "2C2";
        assert_eq!(
            check_digit(true_field).unwrap(),
            2,
            "sanity: fixture's own check digit"
        );
        assert!(verify(true_field, '2'), "sanity: the true reading verifies");
        let misread = "7C7"; // every '2', including the check digit, read as '7'
        assert!(!verify(misread, '7'), "fixture must not already validate");
        assert_eq!(
            solve_class_sweep(misread, '7', FieldKind::Other),
            Resolution::Unique(true_field.to_string()),
        );
    }

    #[test]
    fn solve_class_sweep_never_touches_an_already_valid_field() {
        for field in ["0000000", "OOOOOOO", "TTTTTT", "OK<<<<"] {
            let check = char::from_digit(check_digit(field).unwrap(), 10).unwrap();
            assert!(
                verify(field, check),
                "{field:?} fixture must already validate"
            );
            assert_eq!(
                solve_class_sweep(field, check, FieldKind::Other),
                Resolution::Unresolvable,
                "{field:?} already validates; the sweep must not touch it"
            );
        }
    }

    #[test]
    fn solve_substitution_reports_ambiguity_instead_of_guessing() {
        // Synthetic field: two different single-glyph substitutions of this
        // corrupted reading both verify against the same check digit, and
        // neither the checksum nor `FieldKind::Other` can separate them. This
        // is exactly why `CONFUSABLES` must stay a bounded, shape-driven
        // table rather than a full residue-class sweep (see the module
        // doc): even restricted to plausible OCR confusions, ambiguity is
        // still possible — an unrestricted sweep over `CLASSES` would only
        // make it worse by admitting readings like 'K' or 'U' here too.
        let corrupted = "O0ZVEZ";
        let check = '9';
        assert!(!verify(corrupted, check));
        assert_eq!(
            solve_substitution(corrupted, check, FieldKind::Other),
            Resolution::Ambiguous {
                candidates: vec!["00ZVEZ".to_string(), "OOZVEZ".to_string()],
            }
        );
    }
}
