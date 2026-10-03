//! The per-document benchmark archive (ADR-0024, build step 3): one JSON Lines file per run
//! and track, written beside the run and read by nothing in it.
//!
//! A file is a run header (line 1) and then one [`DocRecord`] per document per provider.
//! Records hold the provider's OCR text, so a file is document content and never leaves the
//! machine: nothing here reaches stdout, stderr, `--out`, a dump, the outcome ledger or a
//! tracked file, and only a path and a count are ever printed.
//!
//! **The archive is a side output.** Every method returns `()`. An archive problem (a root
//! that is refused, a file that cannot be created, a write, a flush, a rename) prints one
//! `warning: archive: ...` line on stderr and turns the archive off for the rest of the run.
//! It never changes an exit code, a report, a ledger, a dump or a manifest (Decision 1).
//!
//! **Files** (Decision 3): `<root>/{public,local}/<YYYYMMDDTHHMMSSZ>-<binary>-<run>.jsonl`, with
//! `<binary>` the run header's `binary_name` (`provider-bench` or `synthpass-bench`).
//! A file is created, as `<name>.jsonl.partial` with `create_new`, at its track's first record
//! and renamed by [`Archive::finish`] only. A run that is killed leaves its `.partial`, which
//! readers ignore; an existing file is never opened, appended to, renamed over or rewritten.
//! **The private track** (Decision 7) is written only as text-free records, to
//! `<root>/private/`, a file of its own with the same stem as the run's other files. The type
//! is the guarantee: `PrivateDocRecord` has no field that can hold free text, and
//! `Archive::record_private` is the only way into that directory. The generic
//! [`Archive::record`] drops an [`ArchiveTrack::Private`] record, whatever the caller does, so no
//! other type can reach it. The private file's header is the run's header with `argv` set to
//! `null`: a file name on the command line can name a person.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::ocr_passes::PassObject;
use crate::provider_bench::TruthComparison;
use crate::report::{ModelPathsReport, OutcomeRow};
use crate::synthetic_ledger::LedgerRow;
use crate::CorpusTrack;

/// The record schema. A change to a key's name, order or meaning bumps it (schema 2 adds the
/// accepted pass's lines before repair, ADR-0024 Decision 5).
pub const SCHEMA: u32 = 1;

/// The environment variable that names the archive root, or turns the archive `off`.
pub const ARCHIVE_ENV: &str = "SYNTHPASS_BENCH_ARCHIVE";

/// The directory inside the git common directory that holds the archive by default.
pub const ARCHIVE_DIR_NAME: &str = "synthpass-bench-archive";

/// The measurement variables a header records, by exact name. **Never a prefix rule:**
/// `SYNTHPASS_KEY`, `_TOKEN`, `_TLS_KEY` and `_LICENSE_PRIVKEY` exist beside these.
pub const ARCHIVE_ENV_ALLOWLIST: &[&str] = &[
    "SYNTHPASS_OCR_TEXTURE",
    "SYNTHPASS_OCR_ORDER",
    "SYNTHPASS_OCR_ROTATE",
    "SYNTHPASS_OCR_SKEW",
    "SYNTHPASS_OCR_CHARGRID",
    "SYNTHPASS_OCR_MAX_PASSES",
    "SYNTHPASS_OCR_MAX_SECONDS",
    "SYNTHPASS_OCR_THREADS",
    "SYNTHPASS_OCR_VERBOSE",
    // Removed variables: the code only warns that they have no effect, and a header that
    // records one says the run was set up with it.
    "SYNTHPASS_OCR_STOP",
    "SYNTHPASS_OCR_CONFIRM_PASSES",
    "SYNTHPASS_MRZ_CLASS_SWEEP",
    "SYNTHPASS_MRZ_LINE1_SELECT",
    "SYNTHPASS_MRZ_REFUSE_REPEATED_LINE",
    "SYNTHPASS_MRZ_DATE_DIGITS",
    "SYNTHPASS_LLM_MRZ_HINT",
    "SYNTHPASS_LLM_GRAMMAR",
    "SYNTHPASS_LLM_CONTEXTS",
    "SYNTHPASS_MODEL_N_CTX",
    "RTEN_NUM_THREADS",
];

// ---------------------------------------------------------------- tracks

/// Which track a record belongs to. The directory it is written under is [`Self::dir`]:
/// `public/` holds the public corpus, covers and synthetic runs; `local/` holds
/// `samples/local/`; the private track has no directory here: [`DocRecord`] never reaches it,
/// and its text-free `PrivateDocRecord`s go to `private/` through `Archive::record_private`
/// alone (Decision 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveTrack {
    Synthetic,
    Public,
    Covers,
    Local,
    Private,
}

impl ArchiveTrack {
    /// The corpus walk's own track ([`crate::asset_track`]) for a real specimen.
    pub fn from_corpus(track: CorpusTrack) -> Self {
        match track {
            CorpusTrack::Public => Self::Public,
            CorpusTrack::Covers => Self::Covers,
            CorpusTrack::Local => Self::Local,
            CorpusTrack::Private => Self::Private,
        }
    }

    /// The `track` key of a record.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Synthetic => "synthetic",
            Self::Public => "public",
            Self::Covers => "covers",
            Self::Local => "local",
            Self::Private => "private",
        }
    }

    /// The directory under the root this track's [`DocRecord`]s go to; `None` for the private
    /// track, which [`Archive::record`] never writes.
    pub fn dir(self) -> Option<&'static str> {
        match self {
            Self::Synthetic | Self::Public | Self::Covers => Some("public"),
            Self::Local => Some("local"),
            Self::Private => None,
        }
    }
}

// ---------------------------------------------------------------- records

/// Line 1 of every file. Key set and order are pinned by tests (Decision 4).
#[derive(Debug, Clone, Serialize)]
pub struct RunHeader {
    pub kind: &'static str,
    pub schema: u32,
    /// The hex SHA-256 of this header serialized with `run_id` empty ([`Self::with_run_id`]).
    pub run_id: String,
    pub binary_name: String,
    pub started_unix_ms: u64,
    pub pid: u32,
    /// `ci` if and only if `GITHUB_ACTIONS=true`, else `local`.
    pub source: &'static str,
    pub machine: Machine,
    pub binary: String,
    pub binary_sha256: Option<String>,
    pub git_commit: Option<String>,
    pub working_tree_dirty: Option<bool>,
    /// The command line; `null` in the private file's header ([`Self::without_argv`]).
    pub argv: Option<Vec<String>>,
    pub scope: Scope,
    pub tracks: TrackFlags,
    /// `SAMPLES_DATA_SHA` when it is 40 hex characters, else `null`.
    pub samples_data_sha: Option<String>,
    pub corpus_manifest_sha256: Option<String>,
    pub documents_loaded: usize,
    pub labelled_loaded: usize,
    pub providers: Vec<String>,
    /// The run manifest's `ocr_arms`, from the same code.
    pub ocr_arms: BTreeMap<String, String>,
    /// This process's budget, `null` for a replay: its OCR ran in the capture, whose
    /// manifest does not carry the budget it ran under (ADR-0024, amendment 3, Decision 6).
    pub retry_budget: Option<RetryBudget>,
    /// The run manifest's `mrz_arms`, from the same code.
    pub mrz_arms: BTreeMap<String, String>,
    pub pivot_yy: u32,
    pub model_paths: ModelPathsReport,
    /// The SHA-256 of each OCR model file's bytes, from the same two paths `model_paths` names:
    /// a model replaced under one path changes it. `null` for a replay, which loads no model
    /// (its OCR came from the capture `replay_of` names). Always serialized.
    pub model_sha256: Option<ModelSha256>,
    /// A replay's capture: its run-manifest file name and that file's SHA-256.
    pub replay_of: Option<ReplayOfRecord>,
    /// [`allowlisted_env`]: the allowlisted variables that are set, and only those.
    pub env: BTreeMap<String, Option<String>>,
}

impl RunHeader {
    /// Sets `run_id` from the header's own bytes: the hex SHA-256 of the header serialized
    /// with `run_id` empty. Two runs that differ in any recorded fact (the start time and the
    /// process id included) get different ids. A header that cannot be serialized (it holds
    /// no floats or maps with non-string keys, so it is not expected) gets the id of no
    /// bytes, and [`Archive::start`] then refuses it with a warning, so no file carries it.
    #[must_use]
    pub fn with_run_id(mut self) -> Self {
        self.run_id = String::new();
        let bytes = serde_json::to_vec(&self).unwrap_or_default();
        self.run_id = sha256_hex(&bytes);
        self
    }

    /// This header with `argv` set to `null`, which is line 1 of the private file (Decision 7):
    /// a file name on the command line can name a person, and private records are keyed by hash
    /// for the same reason. Every other key, the `run_id` included, is unchanged, so the pinned
    /// key set holds and the run's files are joined by their id.
    #[must_use]
    pub fn without_argv(&self) -> Self {
        Self {
            argv: None,
            ..self.clone()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Machine {
    pub label: String,
    pub os: &'static str,
    pub arch: &'static str,
    pub cpus: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Scope {
    /// `synthetic-corpus` or `real-specimens`.
    pub corpus: &'static str,
    pub format: Option<String>,
    pub limit: Option<usize>,
    pub document_type: Option<String>,
    pub profile: Option<String>,
    pub seed_start: Option<u64>,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackFlags {
    pub private: bool,
    pub local: bool,
    pub covers: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetryBudget {
    pub max_passes: usize,
    pub max_seconds: u64,
}

/// The SHA-256 of the two OCR model files a run loaded, in the shape the nightly's run header
/// records (`bench_nightly_rows.py`'s `model_sha256`). A file that cannot be read is `null` for
/// its key, never an error that stops the run (the rule `binary_sha256` follows).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelSha256 {
    pub detection: Option<String>,
    pub recognition: Option<String>,
}

impl ModelSha256 {
    /// The hashes of the files at the two paths the engine was loaded from.
    pub fn of_files(detection: &Path, recognition: &Path) -> Self {
        let hash = |path: &Path| std::fs::read(path).ok().map(|bytes| sha256_hex(&bytes));
        Self {
            detection: hash(detection),
            recognition: hash(recognition),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplayOfRecord {
    pub run_manifest: String,
    pub sha256: String,
}

/// One document as one provider read it (Decision 5). Key set and order are pinned by tests.
///
/// It holds the provider-input OCR text verbatim, so it is document content. It never holds
/// the fixture's text: a labelled specimen carries mismatch counts, cell positions and the
/// classes of the printed document code's two cells only.
#[derive(Debug, Serialize)]
pub(crate) struct DocRecord<'a> {
    pub kind: &'static str,
    pub run_id: String,
    pub provider: String,
    pub track: &'static str,
    pub name: &'a str,
    pub asset_id: Option<&'a str>,
    pub source_sha256: Option<&'a str>,
    /// Exactly `OutcomeRow::from(&DocumentDetail)`.
    pub ledger_row: OutcomeRow,
    pub read_ok: bool,
    pub read_us: u128,
    /// As the report writes it: field name to `exact`, `wrong` or `unread`, never a value.
    pub field_correctness: Option<BTreeMap<&'static str, &'static str>>,
    pub ocr: OcrRecord<'a>,
    /// From the Tier-1 parse; `null` when nothing parsed.
    pub tier1_read: Option<Tier1ReadRecord>,
    /// The provider's value for every field, keyed by `CoreField::as_str`; `null` on a reader
    /// error.
    pub fields: Option<BTreeMap<&'static str, Option<String>>>,
    /// For a labelled specimen only, else `null`.
    pub truth: Option<&'a TruthComparison>,
}

#[derive(Debug, Serialize)]
pub(crate) struct OcrRecord<'a> {
    pub text: &'a str,
    pub rotation: u16,
    pub mrz_band_score: Option<f64>,
    pub chargrid: Option<&'a str>,
    /// Filled only when the run already traces (`--dump-ocr-passes`, or a replay's rows);
    /// `null` otherwise. The archive adds no tracing of its own: under the retry loop's 52 s
    /// budget it would change timing.
    pub ocr_passes: Option<&'a [PassObject]>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Tier1ReadRecord {
    /// The lines after repair.
    pub lines: Vec<String>,
    pub damaged_recovery: bool,
    pub valid: bool,
}

// ---------------------------------------------------------------- the private record

/// A lowercase hex SHA-256: 64 characters from `0-9a-f`, checked when it is made, so a value of
/// this type cannot be anything else (a file name, say).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Sha256Hex(String);

impl Sha256Hex {
    /// `None` unless `text` is exactly a lowercase hex SHA-256.
    pub(crate) fn new(text: &str) -> Option<Self> {
        let is_hex =
            text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        is_hex.then(|| Self(text.to_string()))
    }
}

/// Why the native retry loop stopped: the five values `synthpass-ocr` writes, a closed set.
/// `DocumentDetail::retry_stop` is a `String`, so a replayed capture could carry any text there;
/// [`Self::parse`] admits the five names and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RetryStop {
    GeneralValid,
    VariantValid,
    PassCap,
    Budget,
    Exhausted,
}

impl RetryStop {
    pub(crate) fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "general_valid" => Self::GeneralValid,
            "variant_valid" => Self::VariantValid,
            "pass_cap" => Self::PassCap,
            "budget" => Self::Budget,
            "exhausted" => Self::Exhausted,
            _ => return None,
        })
    }
}

/// The native retry pass a read came from: `general`, or `pass-NN` with `NN` the pass's number
/// (`synthpass-ocr` writes `pass-{i:02}`). It holds a number, never the text it was parsed
/// from, so a value that is neither form is not representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetryVariant {
    General,
    Pass(u16),
}

impl RetryVariant {
    pub(crate) fn parse(text: &str) -> Option<Self> {
        if text == "general" {
            return Some(Self::General);
        }
        let digits = text.strip_prefix("pass-")?;
        let plain = (2..=4).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit());
        plain.then(|| digits.parse().ok()).flatten().map(Self::Pass)
    }
}

impl Serialize for RetryVariant {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::General => serializer.serialize_str("general"),
            Self::Pass(n) => serializer.collect_str(&format_args!("pass-{n:02}")),
        }
    }
}

/// What one cell of a recovered zone is, and nothing more: the partition `classify_char` in
/// `tools/archive_query.py` uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CellClass {
    /// An ASCII letter, either case.
    Letter,
    /// An ASCII digit.
    Digit,
    /// The filler `<`.
    Filler,
    /// Anything else, a non-ASCII character included.
    Other,
}

impl CellClass {
    pub(crate) fn of(c: char) -> Self {
        if c == '<' {
            Self::Filler
        } else if c.is_ascii_alphabetic() {
            Self::Letter
        } else if c.is_ascii_digit() {
            Self::Digit
        } else {
            Self::Other
        }
    }

    /// The character a class is written as: `A`, `9`, `<` or `?`.
    pub(crate) fn symbol(self) -> char {
        match self {
            Self::Letter => 'A',
            Self::Digit => '9',
            Self::Filler => '<',
            Self::Other => '?',
        }
    }
}

/// A recovered zone as classes, one entry per line, one class per cell. The zone's characters
/// are not kept: [`Self::of_zone`] maps each to its [`CellClass`] and drops it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZoneClasses(Vec<Vec<CellClass>>);

impl ZoneClasses {
    pub(crate) fn of_zone(zone: &str) -> Self {
        Self(
            zone.lines()
                .map(|line| line.chars().map(CellClass::of).collect())
                .collect(),
        )
    }
}

/// The classes of a zone's document code, the first two cells of its first line, as the symbols
/// [`CellClass::symbol`] writes (`P<` is `A<`, `PS` is `AA`): at most two symbols, fewer for a
/// shorter line, none for an empty zone. The characters are mapped and dropped, so the result
/// can go on every track (Decision 7 already admits [`ZoneClasses`]).
pub(crate) fn document_code_classes(zone: &str) -> String {
    zone.lines()
        .next()
        .unwrap_or_default()
        .chars()
        .take(2)
        .map(|c| CellClass::of(c).symbol())
        .collect()
}

impl Serialize for ZoneClasses {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(
            self.0
                .iter()
                .map(|line| line.iter().map(|class| class.symbol()).collect::<String>()),
        )
    }
}

/// One private-track document as one provider read it (Decision 7): a type with **no field that
/// can hold free text**. Every string in it is a `&'static str` from a closed set (an enum's
/// name), a checked hex digest ([`Sha256Hex`]), or class symbols ([`ZoneClasses`]). The record
/// has no `name`, no `asset_id`, no OCR text, no zone line and no field value, and it is keyed by
/// the SHA-256 of the image's bytes. Key set and order are pinned by a test.
///
/// A field outside this list, even a boolean, needs the owner's yes and an ADR-0024 amendment.
#[derive(Debug, Serialize)]
pub(crate) struct PrivateDocRecord<'a> {
    /// `private_doc`, so a reader that keeps `kind == "doc"` skips it.
    pub kind: &'static str,
    pub run_id: Sha256Hex,
    /// A provider id from the catalog (`ProviderId` wraps a `&'static str`).
    pub provider: &'static str,
    /// `private`.
    pub track: &'static str,
    pub source_sha256: Sha256Hex,
    /// The miss kind, `hit` or a `miss_kind` name.
    pub outcome: &'static str,
    pub mrz_format: Option<&'static str>,
    pub mrz_found: bool,
    pub mrz_checksums_valid: bool,
    /// Check name to `true`, `false` or `null`; `null` when no MRZ parsed.
    pub check_states: Option<BTreeMap<&'static str, Option<bool>>>,
    pub retry_variant_id: Option<RetryVariant>,
    pub retry_budget_hit: bool,
    pub retry_stop: Option<RetryStop>,
    pub retry_damaged_recovery: Option<bool>,
    pub tier1_damaged_recovery: Option<bool>,
    pub read_us: u128,
    pub ocr_ms: u128,
    pub mrz_band_score: Option<f64>,
    pub rotation: u16,
    /// Counts and cell positions against the fixture, and the classes of its document code's
    /// two cells (`code_cells`), never a character; `null` for an unlabelled document.
    pub truth: Option<&'a TruthComparison>,
    /// The Tier-1 zone after repair; `null` when nothing parsed.
    pub zone_classes: Option<ZoneClasses>,
}

// ---------------------------------------------------------------- header parts

/// Lowercase hex SHA-256.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for byte in Sha256::digest(bytes) {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// `ci` if and only if the value is exactly `true` (`GITHUB_ACTIONS`), else `local`.
pub fn source_from(github_actions: Option<&str>) -> &'static str {
    if github_actions == Some("true") {
        "ci"
    } else {
        "local"
    }
}

/// `SAMPLES_DATA_SHA` when it is 40 hexadecimal characters, else `None`.
pub fn samples_data_sha_from(value: Option<&str>) -> Option<String> {
    value
        .filter(|v| v.len() == 40 && v.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_string)
}

/// The machine label: the operating system and the processor's designation, such as
/// `win11-i5-4570`, `win11-i7-1255U` or `linux-xeon-8272CL`. It names the hardware a number was
/// measured on, never the host: a host name can be a person's. `windows_build` is the Windows
/// build number (11 starts at 22000); `cpu_brand` is the processor's brand string. Both are
/// injected so every rule is testable.
pub fn machine_label_from(os: &str, windows_build: Option<u32>, cpu_brand: Option<&str>) -> String {
    let os = match (os, windows_build) {
        ("windows", Some(build)) if build >= 22_000 => "win11",
        ("windows", Some(_)) => "win10",
        (other, _) => other,
    };
    format!("{os}-{}", cpu_designation(cpu_brand))
}

/// The longest processor designation recorded. The longest real one is about twenty
/// characters (`xeon-E5-2673-v4`, `ultra7-155H`); the cap only keeps a garbled brand string
/// from becoming a very long label in every record's header.
const DESIGNATION_MAX: usize = 48;

/// The processor's designation from its brand string, in the form people write it:
/// `Intel(R) Core(TM) i5-4570 CPU @ 3.20GHz` is `i5-4570`; `12th Gen Intel(R) Core(TM)
/// i7-1255U` is `i7-1255U`; a Xeon is `xeon-` and its model (`xeon-8272CL`, `xeon-E5-2673-v4`);
/// `AMD EPYC 7763 64-Core Processor` is `epyc-7763`; `AMD Ryzen 7 5800X` is `ryzen7-5800X`;
/// `Core(TM) Ultra 7 155H` is `ultra7-155H`. Any other brand string is lower-cased, its
/// vendor words dropped and its runs of other characters turned into `-`. Nothing recognisable
/// is `unknown`. The result is only `[A-Za-z0-9._-]`.
pub fn cpu_designation(brand: Option<&str>) -> String {
    let cleaned = brand
        .unwrap_or("")
        .replace("(R)", " ")
        .replace("(TM)", " ")
        .replace(['®', '™'], " ");
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    let after = |word: &str| {
        tokens
            .iter()
            .position(|t| t.eq_ignore_ascii_case(word))
            .map(|i| &tokens[i + 1..])
    };
    let is_model = |t: &str| t.bytes().any(|b| b.is_ascii_digit()) && !t.ends_with("GHz");
    let is_core_i = |t: &str| {
        let bytes = t.as_bytes();
        bytes.len() > 3
            && bytes[0] == b'i'
            && matches!(bytes[1], b'3' | b'5' | b'7' | b'9')
            && bytes[2] == b'-'
    };
    let designation = if let Some(token) = tokens.iter().find(|t| is_core_i(t)) {
        Some((*token).to_string())
    } else if let Some(rest) = after("Ultra") {
        match rest {
            [tier, model, ..] if tier.parse::<u32>().is_ok() && is_model(model) => {
                Some(format!("ultra{tier}-{model}"))
            }
            _ => None,
        }
    } else if let Some(rest) = after("Xeon") {
        rest.iter().position(|t| is_model(t)).map(|i| {
            let model = rest[i];
            match rest.get(i + 1) {
                Some(version)
                    if version.len() > 1
                        && version.starts_with('v')
                        && version[1..].bytes().all(|b| b.is_ascii_digit()) =>
                {
                    format!("xeon-{model}-{version}")
                }
                _ => format!("xeon-{model}"),
            }
        })
    } else if let Some(rest) = after("EPYC") {
        rest.iter()
            .find(|t| is_model(t))
            .map(|model| format!("epyc-{model}"))
    } else if let Some(rest) = after("Ryzen") {
        match rest {
            [tier, model, ..] if tier.parse::<u32>().is_ok() && is_model(model) => {
                Some(format!("ryzen{tier}-{model}"))
            }
            _ => None,
        }
    } else {
        None
    };
    let designation = designation.unwrap_or_else(|| {
        let words: Vec<&str> = tokens
            .iter()
            .copied()
            .filter(|t| {
                let lower = t.to_ascii_lowercase();
                !([
                    "intel",
                    "amd",
                    "core",
                    "cpu",
                    "processor",
                    "genuineintel",
                    "@",
                ]
                .contains(&lower.as_str())
                    || lower.ends_with("ghz")
                    || lower.ends_with("mhz"))
            })
            .collect();
        words.join(" ").to_ascii_lowercase()
    });
    let mut plain = String::new();
    for ch in designation.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' {
            plain.push(ch);
        } else if !plain.ends_with('-') {
            plain.push('-');
        }
    }
    let plain = plain.trim_matches('-');
    if plain.is_empty() {
        "unknown".to_string()
    } else {
        plain.chars().take(DESIGNATION_MAX).collect()
    }
}

/// The value of `name` in `reg query` output: the text after the type column (`REG_SZ`,
/// `REG_DWORD`...) on the line that starts with `name`.
fn reg_value(output: &str, name: &str) -> Option<String> {
    output
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with(name))
        .and_then(|line| {
            line[name.len()..]
                .trim_start()
                .split_once(char::is_whitespace)
        })
        .map(|(_type, value)| value.trim().to_string())
}

/// The first `model name` of `/proc/cpuinfo`.
fn cpuinfo_model(text: &str) -> Option<String> {
    text.lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(key, _)| key.trim() == "model name")
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// What `command` prints on stdout, or `None` when it cannot run or fails.
fn command_stdout(command: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(command)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// This machine's processor brand string and, on Windows, its build number.
fn detect_cpu() -> (Option<String>, Option<u32>) {
    match std::env::consts::OS {
        "windows" => {
            let cpu = command_stdout(
                "reg",
                &[
                    "query",
                    r"HKLM\HARDWARE\DESCRIPTION\System\CentralProcessor\0",
                    "/v",
                    "ProcessorNameString",
                ],
            )
            .and_then(|out| reg_value(&out, "ProcessorNameString"));
            let build = command_stdout(
                "reg",
                &[
                    "query",
                    r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion",
                    "/v",
                    "CurrentBuildNumber",
                ],
            )
            .and_then(|out| reg_value(&out, "CurrentBuildNumber"))
            .and_then(|value| value.parse().ok());
            (cpu, build)
        }
        "macos" => (
            command_stdout("sysctl", &["-n", "machdep.cpu.brand_string"])
                .map(|out| out.trim().to_string()),
            None,
        ),
        _ => (
            std::fs::read_to_string("/proc/cpuinfo")
                .ok()
                .and_then(|text| cpuinfo_model(&text)),
            None,
        ),
    }
}

impl Machine {
    /// This process's machine.
    pub fn detect() -> Self {
        let (cpu_brand, windows_build) = detect_cpu();
        Self {
            label: machine_label_from(std::env::consts::OS, windows_build, cpu_brand.as_deref()),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            cpus: std::thread::available_parallelism().map_or(1, usize::from),
        }
    }
}

/// A path as a person reads it: Windows' canonical form carries a `\\?\` (or `\\?\UNC\`)
/// prefix that says nothing about where the file is.
pub fn plain_path(path: &Path) -> String {
    let text = path.display().to_string();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        text
    }
}

/// The allowlisted variables this process has, looked up **by name**.
///
/// Never `std::env::vars()`: it panics while it iterates if any variable's name or value
/// is not valid Unicode, and that would turn a side output into an exit code 101. A
/// non-Unicode value of an allowlisted name becomes `""`, which [`allowlisted_env`]'s value
/// gate records as `null`. On Windows the lookup is case-insensitive, as every reader's is.
pub fn process_env() -> BTreeMap<String, Option<String>> {
    env_from(|name| std::env::var_os(name))
}

/// [`process_env`] with the lookup injected, so a non-Unicode value is testable without
/// touching the process's environment.
fn env_from(get: impl Fn(&str) -> Option<std::ffi::OsString>) -> BTreeMap<String, Option<String>> {
    allowlisted_env(ARCHIVE_ENV_ALLOWLIST.iter().filter_map(|name| {
        get(name).map(|value| ((*name).to_string(), value.into_string().unwrap_or_default()))
    }))
}

/// The variables in [`ARCHIVE_ENV_ALLOWLIST`] that appear in `vars`, with their values.
///
/// A pure function over injected pairs, and an exact-name match: a prefix rule would record
/// `SYNTHPASS_KEY` and its neighbours. Only variables that are set are recorded. A value
/// outside `[A-Za-z0-9._-]{1,32}` is recorded as `null`, so a value that does not look like a
/// measurement setting is never copied.
pub fn allowlisted_env(
    vars: impl IntoIterator<Item = (String, String)>,
) -> BTreeMap<String, Option<String>> {
    vars.into_iter()
        .filter(|(name, _)| ARCHIVE_ENV_ALLOWLIST.contains(&name.as_str()))
        .map(|(name, value)| {
            let plain = (1..=32).contains(&value.len())
                && value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
            (name, plain.then_some(value))
        })
        .collect()
}

/// `HEAD` and whether the working tree is dirty: the two git commands the run manifest
/// records, in one place so the manifest and the archive cannot disagree about how they ask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitState {
    pub commit: String,
    pub dirty: bool,
}

/// [`GitState`] of `root`. The error strings are the run manifest's own.
pub fn git_state(root: &Path) -> Result<GitState, String> {
    let commit_output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("git rev-parse HEAD: {e}"))?;
    if !commit_output.status.success() {
        return Err("git rev-parse HEAD failed".to_string());
    }
    let commit = String::from_utf8(commit_output.stdout)
        .map_err(|e| format!("git rev-parse HEAD was not UTF-8: {e}"))?
        .trim()
        .to_string();
    let dirty_output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("git status --porcelain: {e}"))?;
    if !dirty_output.status.success() {
        return Err("git status --porcelain failed".to_string());
    }
    Ok(GitState {
        commit,
        dirty: !dirty_output.stdout.is_empty(),
    })
}

/// `git -C root rev-parse --path-format=absolute --git-common-dir`: the directory every
/// worktree of a clone shares. `Err` says why git could not answer.
pub fn git_common_dir(root: &Path) -> Result<PathBuf, String> {
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !output.status.success() {
        return Err("git rev-parse --git-common-dir failed".to_string());
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|_| "git rev-parse --git-common-dir was not UTF-8".to_string())?;
    parse_git_common_dir(&text)
}

/// The one absolute path `git rev-parse --path-format=absolute --git-common-dir` prints.
/// Git before 2.31 does not know the option: it echoes it back as an extra line and then
/// prints a relative directory, which must never be taken for the root.
fn parse_git_common_dir(text: &str) -> Result<PathBuf, String> {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let (Some(dir), None) = (lines.next(), lines.next()) else {
        return Err("git rev-parse --git-common-dir did not print exactly one path".to_string());
    };
    let dir = PathBuf::from(dir);
    if !dir.is_absolute() {
        return Err("git rev-parse --git-common-dir did not print an absolute path".to_string());
    }
    Ok(dir)
}

// ---------------------------------------------------------------- root

/// Whether the run asks for an archive at all: `--no-archive` and `SYNTHPASS_BENCH_ARCHIVE=off`
/// (trimmed, any case) say no, and the flag wins over the variable.
pub fn archive_requested(no_archive: bool, variable: Option<&str>) -> bool {
    !no_archive && !variable.is_some_and(|value| value.trim().eq_ignore_ascii_case("off"))
}

/// Which root a run asks for, before any check of the place itself (Decision 2).
///
/// - `no_archive` (`--no-archive`) turns the archive off, and wins over the variable.
/// - `variable` is `SYNTHPASS_BENCH_ARCHIVE`: `off` (trimmed, any case) turns it off, an empty
///   or blank value counts as unset, and any other value is the directory, relative to the
///   current directory like `--out`.
/// - Unset, the root is `synthpass-bench-archive/` in the git common directory. `Err` when
///   git cannot say where that is: the caller warns and runs without an archive.
pub fn archive_root_choice(
    no_archive: bool,
    variable: Option<&str>,
    git_common_dir: impl FnOnce() -> Result<PathBuf, String>,
) -> Result<Option<PathBuf>, String> {
    if !archive_requested(no_archive, variable) {
        return Ok(None);
    }
    match variable {
        Some(value) if !value.trim().is_empty() => Ok(Some(PathBuf::from(value))),
        _ => git_common_dir()
            .map(|dir| Some(dir.join(ARCHIVE_DIR_NAME)))
            .map_err(|e| format!("cannot find the git common directory ({e})")),
    }
}

/// [`archive_root_choice`] and the refusal of a root that git would stage (Decision 2), as
/// far as the bench binaries need it. `Ok(None)` is an archive that is off; `Err` is the text
/// of a warning, after which the archive is off too.
///
/// A root **inside the git common directory** is accepted first: git never tracks it, and
/// `git check-ignore` exits 1 for `.git/...` paths, so a plain reuse of the dump guard would
/// refuse the default root. Inside the working tree (`repo_root`), a root is accepted only
/// when `is_ignored` says git ignores a record path under it. Anywhere else it is accepted.
/// Both sides are canonicalized before they are compared (Windows returns `\\?\` paths). This
/// is decided before any model loads.
pub fn resolve_archive_root(
    no_archive: bool,
    variable: Option<&str>,
    repo_root: &Path,
    cwd: &Path,
    git_common_dir: impl FnOnce() -> Result<PathBuf, String>,
    is_ignored: impl Fn(&Path) -> Result<bool, String>,
) -> Result<Option<PathBuf>, String> {
    let git_dir = std::cell::RefCell::new(None);
    let Some(root) = archive_root_choice(no_archive, variable, || {
        let dir = git_common_dir();
        *git_dir.borrow_mut() = dir.as_ref().ok().cloned();
        dir
    })?
    else {
        return Ok(None);
    };
    // Git was asked for its directory only when the default root was wanted. A root named by
    // the variable never asks, so one inside `.git/` is judged like any in-tree root below.
    let git_dir = git_dir.into_inner();
    let probe = Path::new("public").join("probe.jsonl");
    let located = crate::ocr_passes::locate_in_tree(repo_root, cwd, &root, &probe)?;
    if let Some(git_dir) = git_dir
        .as_deref()
        .and_then(|d| std::fs::canonicalize(d).ok())
    {
        if located.resolved_dir.starts_with(&git_dir) {
            return Ok(Some(located.resolved_dir));
        }
    }
    match located.relative {
        None => Ok(Some(located.resolved_dir)),
        Some(relative) => match is_ignored(&relative) {
            Ok(true) => Ok(Some(located.resolved_dir)),
            Ok(false) => Err(format!(
                "{} is inside the working tree and not ignored by git; set {ARCHIVE_ENV} to a \
                 directory outside the checkout or one git ignores",
                relative
                    .parent()
                    .and_then(Path::parent)
                    .unwrap_or(&relative)
                    .display()
            )),
            Err(e) => Err(format!(
                "cannot tell whether git ignores {} ({e})",
                relative.display()
            )),
        },
    }
}

// ---------------------------------------------------------------- plan and header

/// What a run does about the archive (ADR-0024), decided before any model loads.
#[derive(Debug, PartialEq)]
pub enum ArchivePlan {
    /// No archive, and nothing to say: `--no-archive` or `SYNTHPASS_BENCH_ARCHIVE=off`.
    Off,
    /// No archive, and one `warning: archive: ...` line to print.
    Warn(String),
    /// Write the archive under this root, which has passed the tree check.
    Root(PathBuf),
}

/// The archive plan for a run, from the process's facts injected as arguments so every rule is
/// testable without the environment or git: `variable` is `SYNTHPASS_BENCH_ARCHIVE`, `repo` the
/// working tree, `cwd` the current directory.
///
/// A `--include-private` run archives like any other: its private documents are written as
/// text-free `PrivateDocRecord`s to `private/` (ADR-0024, Decision 7), so the flag does not
/// enter the plan. `--no-archive` and `SYNTHPASS_BENCH_ARCHIVE=off` still turn the archive off
/// for it, and every problem (a root git would stage, a git that cannot say where its directory
/// is) is a warning that never changes an exit code (Decision 1).
pub fn plan(
    no_archive: bool,
    variable: Option<&str>,
    repo: &Path,
    cwd: &Path,
    git_common_dir: impl FnOnce() -> Result<PathBuf, String>,
    is_ignored: impl Fn(&Path) -> Result<bool, String>,
) -> ArchivePlan {
    match resolve_archive_root(no_archive, variable, repo, cwd, git_common_dir, is_ignored) {
        Ok(None) => ArchivePlan::Off,
        Ok(Some(root)) => ArchivePlan::Root(root),
        Err(e) => ArchivePlan::Warn(format!("warning: archive: {e}")),
    }
}

/// [`plan`] for this process: `SYNTHPASS_BENCH_ARCHIVE`, the working directory, and git.
pub fn plan_for_process(no_archive: bool, repo: &Path) -> ArchivePlan {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let variable = match std::env::var_os(ARCHIVE_ENV).map(std::ffi::OsString::into_string) {
        None => None,
        Some(Ok(value)) => Some(value),
        Some(Err(_)) => {
            return ArchivePlan::Warn(format!(
                "warning: archive: {ARCHIVE_ENV} is not valid Unicode, so no archive is written"
            ));
        }
    };
    plan(
        no_archive,
        variable.as_deref(),
        repo,
        &cwd,
        || git_common_dir(repo),
        |relative| crate::ocr_passes::git_ignores(repo, relative),
    )
}

/// The `SYNTHPASS_MRZ_*` arms as the run manifest and the archive header spell them: each name
/// is what the binary resolved, never the variable's raw value (an unrecognised value falls
/// back to `off`).
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct MrzArms {
    pub class_sweep: &'static str,
    pub line1_select: &'static str,
    pub refuse_repeated_line: &'static str,
    pub date_digits: &'static str,
}

impl MrzArms {
    /// The arms this process resolved.
    pub fn from_env() -> Self {
        Self {
            class_sweep: synthpass_die::class_sweep_arm().0,
            line1_select: synthpass_die::line1_select_arm().0,
            refuse_repeated_line: synthpass_die::refuse_repeated_line_arm().0,
            date_digits: synthpass_die::date_digits_arm().0,
        }
    }
}

/// The `SYNTHPASS_MRZ_*` arms as a map, from [`MrzArms`] itself: the run manifest and the
/// archive header both spell them, and neither keeps its own list of names.
pub fn mrz_arms_map() -> BTreeMap<String, String> {
    serde_json::to_value(MrzArms::from_env())
        .ok()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

/// The five `SYNTHPASS_OCR_*` arms as the run manifest, the archive header and the replay's arm
/// check spell them.
pub fn ocr_arms_map(arms: &synthpass_ocr::OcrArms) -> BTreeMap<String, String> {
    [
        ("texture", arms.texture),
        ("order", arms.order),
        ("rotate", arms.rotate),
        ("skew", arms.skew),
        ("chargrid", arms.chargrid),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect()
}

/// What a binary tells the archive about its run, beyond the process's own facts.
pub struct HeaderInputs<'a> {
    /// `provider-bench` or `synthpass-bench`: the header's `binary_name` and the file name's.
    pub binary_name: &'a str,
    /// The working tree, for the git commit and dirty flag.
    pub repo: &'a Path,
    pub argv: &'a [String],
    pub scope: Scope,
    pub tracks: TrackFlags,
    pub corpus_manifest_sha256: Option<String>,
    pub documents_loaded: usize,
    pub labelled_loaded: usize,
    pub providers: Vec<String>,
    pub ocr_arms: BTreeMap<String, String>,
    pub model_paths: ModelPathsReport,
    /// `None` for a replay.
    pub model_sha256: Option<ModelSha256>,
    pub replay_of: Option<ReplayOfRecord>,
}

/// The header of a run's archive files (Decision 4): `inputs` plus this process's facts. The
/// arms come from [`ocr_arms_map`] and [`mrz_arms_map`] (the run manifest's own code), the retry
/// budget from `synthpass_ocr::effective_retry_budget` (the source the benchmarks' reports
/// use), and `env` from [`allowlisted_env`].
pub fn run_header(inputs: HeaderInputs<'_>) -> RunHeader {
    let git = git_state(inputs.repo).ok();
    let binary = std::env::current_exe().ok();
    let (max_passes, max_seconds) = synthpass_ocr::effective_retry_budget();
    RunHeader {
        kind: "run",
        schema: SCHEMA,
        run_id: String::new(),
        binary_name: inputs.binary_name.to_string(),
        started_unix_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64),
        pid: std::process::id(),
        source: source_from(std::env::var("GITHUB_ACTIONS").ok().as_deref()),
        machine: Machine::detect(),
        // The file name only: the path says whose machine this is, and the SHA-256 below
        // identifies the binary. `argv` and `model_paths` keep their paths, which the Decision 8
        // publisher is to refuse or redact.
        binary: binary.as_ref().and_then(|b| b.file_name()).map_or_else(
            || inputs.binary_name.to_string(),
            |name| name.to_string_lossy().into_owned(),
        ),
        binary_sha256: binary
            .as_ref()
            .and_then(|b| std::fs::read(b).ok())
            .map(|bytes| sha256_hex(&bytes)),
        git_commit: git.as_ref().map(|g| g.commit.clone()),
        working_tree_dirty: git.as_ref().map(|g| g.dirty),
        argv: Some(inputs.argv.to_vec()),
        scope: inputs.scope,
        tracks: inputs.tracks,
        samples_data_sha: samples_data_sha_from(std::env::var("SAMPLES_DATA_SHA").ok().as_deref()),
        corpus_manifest_sha256: inputs.corpus_manifest_sha256,
        documents_loaded: inputs.documents_loaded,
        labelled_loaded: inputs.labelled_loaded,
        providers: inputs.providers,
        ocr_arms: inputs.ocr_arms,
        // A replay's OCR ran in the capture, whose manifest does not carry its budget.
        retry_budget: inputs.replay_of.is_none().then_some(RetryBudget {
            max_passes,
            max_seconds,
        }),
        mrz_arms: mrz_arms_map(),
        pivot_yy: synthpass_die::mrz_parse_options().pivot_yy,
        model_paths: inputs.model_paths,
        model_sha256: inputs.model_sha256,
        replay_of: inputs.replay_of,
        env: process_env(),
    }
    .with_run_id()
}

// ---------------------------------------------------------------- the writer

/// The civil date (year, month, day) of Unix seconds, in UTC: Howard Hinnant's
/// civil-from-days algorithm, one copy for every date this crate writes (the archive's file
/// names and `provider-bench`'s report date). One date does not justify a `chrono`
/// dependency.
pub fn civil_date(unix_secs: u64) -> (i64, i64, i64) {
    let days = (unix_secs / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// `YYYYMMDDTHHMMSSZ` (UTC) from Unix seconds: sortable, and free of `:`, which Windows refuses
/// in a file name.
pub fn utc_stamp(unix_secs: u64) -> String {
    let secs = unix_secs % 86_400;
    let (y, m, d) = civil_date(unix_secs);
    format!(
        "{y:04}{m:02}{d:02}T{:02}{:02}{:02}Z",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

struct TrackFile {
    partial: PathBuf,
    path: PathBuf,
    out: BufWriter<File>,
    records: usize,
}

/// The three directories a file can go to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Dir {
    Public,
    Local,
    /// Text-free records only: reached by [`Archive::record_private`] alone.
    Private,
}

impl Dir {
    fn name(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Local => "local",
            Self::Private => "private",
        }
    }
}

struct Writer {
    root: PathBuf,
    run_id: String,
    header_line: String,
    /// Line 1 of the private file: the header with `argv` set to `null`.
    private_header_line: String,
    file_stem: String,
    public: Option<TrackFile>,
    local: Option<TrackFile>,
    private: Option<TrackFile>,
}

impl Writer {
    fn slot(&mut self, dir: Dir) -> &mut Option<TrackFile> {
        match dir {
            Dir::Public => &mut self.public,
            Dir::Local => &mut self.local,
            Dir::Private => &mut self.private,
        }
    }

    fn header_line(&self, dir: Dir) -> &str {
        match dir {
            Dir::Public | Dir::Local => &self.header_line,
            Dir::Private => &self.private_header_line,
        }
    }
}

enum State {
    Off,
    On(Box<Writer>),
}

struct Inner {
    state: State,
    warnings: Vec<String>,
}

impl Inner {
    /// One warning line on stderr, and the archive is off from here on.
    fn fail(&mut self, what: &str, path: &Path, error: &std::io::Error) {
        let line = format!(
            "warning: archive: {what} {}: {:?}",
            plain_path(path),
            error.kind()
        );
        eprintln!("{line}");
        self.warnings.push(line);
        self.state = State::Off;
    }
}

/// The archive of one run. Shared by reference: every method takes `&self`.
pub struct Archive {
    inner: Mutex<Inner>,
}

impl std::fmt::Debug for Archive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // No records, no paths: only whether it is still writing.
        f.debug_struct("Archive")
            .field("on", &self.is_on())
            .finish()
    }
}

impl Archive {
    /// An archive that is off.
    pub fn disabled() -> Self {
        Self {
            inner: Mutex::new(Inner {
                state: State::Off,
                warnings: Vec::new(),
            }),
        }
    }

    /// An archive under `root` for the run `header` describes. Nothing touches the disk until
    /// a track's first record: no record means no file.
    pub fn start(root: &Path, header: &RunHeader) -> Self {
        let lines = serde_json::to_string(header)
            .and_then(|line| Ok((line, serde_json::to_string(&header.without_argv())?)));
        let (line, private_line) = match lines {
            Ok(lines) => lines,
            Err(e) => {
                let warning = format!("warning: archive: cannot serialize the run header: {e}");
                eprintln!("{warning}");
                return Self {
                    inner: Mutex::new(Inner {
                        state: State::Off,
                        warnings: vec![warning],
                    }),
                };
            }
        };
        let stem = format!(
            "{}-{}-{}",
            utc_stamp(header.started_unix_ms / 1000),
            header.binary_name,
            header.run_id.get(..12).unwrap_or(&header.run_id)
        );
        Self {
            inner: Mutex::new(Inner {
                state: State::On(Box::new(Writer {
                    root: root.to_path_buf(),
                    run_id: header.run_id.clone(),
                    header_line: line,
                    private_header_line: private_line,
                    file_stem: stem,
                    public: None,
                    local: None,
                    private: None,
                })),
                warnings: Vec::new(),
            }),
        }
    }

    /// Whether records are still being written.
    pub fn is_on(&self) -> bool {
        self.inner
            .lock()
            .is_ok_and(|inner| matches!(inner.state, State::On(_)))
    }

    /// The run id records carry; empty when the archive is off.
    pub fn run_id(&self) -> String {
        match self.inner.lock() {
            Ok(inner) => match &inner.state {
                State::On(writer) => writer.run_id.clone(),
                State::Off => String::new(),
            },
            Err(_) => String::new(),
        }
    }

    /// The warnings this archive has printed, in order. At most one: the first problem turns
    /// it off.
    pub fn warnings(&self) -> Vec<String> {
        self.inner
            .lock()
            .map(|inner| inner.warnings.clone())
            .unwrap_or_default()
    }

    /// Appends one record to its track's file, creating the file (and its header line) first
    /// when it is the track's first. A private-track record is dropped: the private track has
    /// one type and one door, [`Self::record_private`].
    pub(crate) fn record<T: Serialize>(&self, track: ArchiveTrack, record: &T) {
        let dir = match track.dir() {
            Some("local") => Dir::Local,
            Some(_) => Dir::Public,
            None => return,
        };
        self.append(dir, record);
    }

    /// Appends one text-free record to the private file (`<root>/private/`), creating the file
    /// first when it is the run's first private document. The only way into that directory.
    pub(crate) fn record_private(&self, record: &PrivateDocRecord<'_>) {
        self.append(Dir::Private, record);
    }

    fn append<T: Serialize>(&self, dir: Dir, record: &T) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let inner = &mut *inner;
        let State::On(writer) = &mut inner.state else {
            return;
        };
        let line = match serde_json::to_string(record) {
            Ok(line) => line,
            Err(e) => {
                let error = std::io::Error::new(std::io::ErrorKind::InvalidData, e);
                let root = writer.root.clone();
                inner.fail("cannot serialize a record for", &root, &error);
                return;
            }
        };
        if writer.slot(dir).is_none() {
            let track_dir = writer.root.join(dir.name());
            let partial = track_dir.join(format!("{}.jsonl.partial", writer.file_stem));
            let path = track_dir.join(format!("{}.jsonl", writer.file_stem));
            let created = std::fs::create_dir_all(&track_dir).and_then(|()| {
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&partial)
            });
            match created {
                Ok(file) => {
                    let mut out = BufWriter::new(file);
                    if let Err(e) = writeln!(out, "{}", writer.header_line(dir)) {
                        inner.fail("cannot write", &partial, &e);
                        return;
                    }
                    *writer.slot(dir) = Some(TrackFile {
                        partial,
                        path,
                        out,
                        records: 0,
                    });
                }
                Err(e) => {
                    inner.fail("cannot create", &partial, &e);
                    return;
                }
            }
        }
        let Some(file) = writer.slot(dir).as_mut() else {
            return;
        };
        if let Err(e) = writeln!(file.out, "{line}") {
            let partial = file.partial.clone();
            inner.fail("cannot write", &partial, &e);
            return;
        }
        file.records += 1;
    }

    /// Flushes every open file to the operating system. Called at each provider's end, so a
    /// run killed during a later provider (the `llm` pass) still leaves the earlier
    /// provider's records on disk.
    pub fn flush(&self) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let inner = &mut *inner;
        let State::On(writer) = &mut inner.state else {
            return;
        };
        let mut failed = None;
        for file in [&mut writer.public, &mut writer.local, &mut writer.private]
            .into_iter()
            .flatten()
        {
            if let Err(e) = file.out.flush() {
                failed = Some((file.partial.clone(), e));
                break;
            }
        }
        if let Some((partial, error)) = failed {
            inner.fail("cannot flush", &partial, &error);
        }
    }

    /// Closes the run: for each file, flush, `sync_all`, close the handle (Windows cannot
    /// rename an open file) and rename `.partial` to `.jsonl`; then one stderr line per file
    /// with its path and record count. Called explicitly once the reader loops return, before
    /// anything that may `exit` (which skips `Drop`). The archive is off afterwards. A file
    /// that is not finished stays `.partial`; an existing `.jsonl` is never replaced.
    pub fn finish(&self) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let inner = &mut *inner;
        let State::On(writer) = std::mem::replace(&mut inner.state, State::Off) else {
            return;
        };
        let mut writer = *writer;
        for file in [
            writer.public.take(),
            writer.local.take(),
            writer.private.take(),
        ]
        .into_iter()
        .flatten()
        {
            let TrackFile {
                partial,
                path,
                out,
                records,
            } = file;
            let file = match out.into_inner() {
                Ok(file) => file,
                Err(e) => {
                    let error = std::io::Error::new(e.error().kind(), "flush");
                    inner.fail("cannot flush", &partial, &error);
                    return;
                }
            };
            if let Err(e) = file.sync_all() {
                inner.fail("cannot sync", &partial, &e);
                return;
            }
            drop(file);
            if path.exists() {
                let error = std::io::Error::from(std::io::ErrorKind::AlreadyExists);
                inner.fail("will not replace", &path, &error);
                return;
            }
            if let Err(e) = std::fs::rename(&partial, &path) {
                inner.fail("cannot rename to", &path, &e);
                return;
            }
            eprintln!("archive: {} ({records} records)", plain_path(&path));
        }
    }
}

// ---------------------------------------------------------------- the synthetic record

/// One synthetic seed as `synthpass-bench` archives it (ADR-0024, Decision 5), where the
/// synthetic path has the fact: a field it does not have is `null`, never invented. Key order is
/// pinned by a test. The `ocr`, `tier1_read`, `fields` and `truth` keys are the ones
/// [`DocRecord`] writes, under the same names.
///
/// The record holds the provider-input OCR text, so it is document content like a
/// [`DocRecord`] (synthetic text, but the same file format and the same rule: nothing here
/// reaches stdout, stderr, `--out`, a dump or the ledger). It never holds the generator's own
/// zone: `truth` carries mismatch counts, positions and the document code's two classes only.
#[derive(Debug, Serialize)]
struct SyntheticDocRecord<'a> {
    kind: &'static str,
    run_id: String,
    track: &'static str,
    format: &'a str,
    profile: &'a str,
    seed: u64,
    /// Exactly the row `--ledger` writes for this seed (it carries the check states).
    ledger_row: &'a LedgerRow,
    /// `null` when OCR itself failed.
    ocr: Option<OcrRecord<'a>>,
    /// `null` when nothing parsed.
    tier1_read: Option<Tier1ReadRecord>,
    /// The read's value for every scored field but the zone itself (that is `tier1_read`),
    /// keyed by `CoreField::as_str`; an absent optional field is `null`. `null` when OCR failed.
    fields: Option<BTreeMap<&'static str, Option<String>>>,
    /// Mismatch counts and positions of the recovered zone against the generator's labels.
    truth: Option<TruthComparison>,
}

impl Archive {
    /// Records one synthetic seed: `ledger_row` is the row [`LedgerRow`] `--ledger` writes for
    /// it, `hit` what `check_document` returned, `labels` the generator's labels, and `passes`
    /// the OCR pass objects when the run already traces (`--ocr-passes`), else `None`: the
    /// archive adds no tracing. Everything synthetic goes to `public/` (Decision 6).
    pub fn record_synthetic(
        &self,
        ledger_row: &LedgerRow,
        hit: &crate::HitResult,
        labels: &synthpass_gen::Labels,
        passes: Option<&[PassObject]>,
    ) {
        // The recovered zone is the `mrz_lines` row of the per-field comparison: present
        // exactly when an MRZ parsed.
        let zone = hit
            .fields
            .iter()
            .find(|outcome| outcome.field == "mrz_lines")
            .and_then(|outcome| outcome.got.as_deref());
        let record = SyntheticDocRecord {
            kind: "doc",
            run_id: self.run_id(),
            track: ArchiveTrack::Synthetic.as_str(),
            format: &ledger_row.format,
            profile: &ledger_row.profile,
            seed: ledger_row.seed,
            ledger_row,
            ocr: hit
                .raw_text
                .as_deref()
                .zip(hit.ocr.as_ref())
                .map(|(text, facts)| OcrRecord {
                    text,
                    rotation: facts.rotation,
                    mrz_band_score: facts.mrz_band_score,
                    // The page-level chargrid verdict is not on the synthetic path; a traced
                    // run's pass objects carry their own chargrid records.
                    chargrid: None,
                    ocr_passes: passes,
                }),
            tier1_read: zone.map(|zone| Tier1ReadRecord {
                lines: zone.lines().map(str::to_string).collect(),
                damaged_recovery: hit.tier1_damaged_recovery.unwrap_or(false),
                // A parsed zone is valid unless its check digits failed.
                valid: !matches!(hit.reason, Some(crate::MissReason::ChecksumFailed { .. })),
            }),
            fields: hit.ocr.as_ref().map(|_| {
                hit.fields
                    .iter()
                    .filter(|outcome| outcome.field != "mrz_lines")
                    .map(|outcome| {
                        (
                            outcome.field,
                            outcome.got.clone().filter(|value| !value.is_empty()),
                        )
                    })
                    .collect()
            }),
            truth: crate::provider_bench::truth_comparison_of_zone(
                zone,
                Some(&labels.mrz_lines.join("\n")),
                Some(&ledger_row.format),
            ),
        };
        self.record(ArchiveTrack::Synthetic, &record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// A scratch directory unique to the process and the test, like `ocr_passes`' tests: it is
    /// under the system temp directory and never inside the working tree.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("archive-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the scratch directory");
        dir
    }

    fn header(started_unix_ms: u64) -> RunHeader {
        RunHeader {
            kind: "run",
            schema: SCHEMA,
            run_id: String::new(),
            binary_name: "provider-bench".to_string(),
            started_unix_ms,
            pid: 4242,
            source: "local",
            machine: Machine {
                label: "test-machine".to_string(),
                os: "testos",
                arch: "testarch",
                cpus: 4,
            },
            binary: "provider-bench".to_string(),
            binary_sha256: None,
            git_commit: Some("0".repeat(40)),
            working_tree_dirty: Some(false),
            argv: Some(vec!["--count".to_string(), "5".to_string()]),
            scope: Scope {
                corpus: "synthetic-corpus",
                format: None,
                limit: None,
                document_type: Some("td3".to_string()),
                profile: Some("clean".to_string()),
                seed_start: Some(0),
                count: 5,
            },
            tracks: TrackFlags {
                private: false,
                local: false,
                covers: false,
            },
            samples_data_sha: None,
            corpus_manifest_sha256: None,
            documents_loaded: 5,
            labelled_loaded: 5,
            providers: vec!["mrz".to_string()],
            ocr_arms: BTreeMap::from([("texture".to_string(), "on".to_string())]),
            retry_budget: Some(RetryBudget {
                max_passes: 14,
                max_seconds: 52,
            }),
            mrz_arms: BTreeMap::from([("class_sweep".to_string(), "off".to_string())]),
            pivot_yy: 26,
            model_paths: ModelPathsReport::default(),
            model_sha256: Some(ModelSha256 {
                detection: Some("a".repeat(64)),
                recognition: Some("b".repeat(64)),
            }),
            replay_of: None,
            env: BTreeMap::new(),
        }
        .with_run_id()
    }

    fn ledger_row() -> OutcomeRow {
        OutcomeRow {
            asset_id: None,
            name: "0".to_string(),
            outcome: "hit".to_string(),
            miss_reason: None,
            mrz_format: Some("TD3".to_string()),
            mrz_found: true,
            mrz_checksums_valid: true,
            names_exact: None,
            name_error: None,
            ocr_ms: 0,
            retry_variant_id: None,
            retry_budget_hit: false,
            retry_stop: None,
            check_states: None,
            retry_damaged_recovery: None,
            tier1_damaged_recovery: None,
        }
    }

    fn record<'a>(
        run_id: &str,
        text: &'a str,
        truth: Option<&'a TruthComparison>,
    ) -> DocRecord<'a> {
        DocRecord {
            kind: "doc",
            run_id: run_id.to_string(),
            provider: "mrz".to_string(),
            track: "synthetic",
            name: "0",
            asset_id: None,
            source_sha256: None,
            ledger_row: ledger_row(),
            read_ok: true,
            read_us: 12,
            field_correctness: None,
            ocr: OcrRecord {
                text,
                rotation: 0,
                mrz_band_score: Some(0.5),
                chargrid: None,
                ocr_passes: None,
            },
            tier1_read: None,
            fields: None,
            truth,
        }
    }

    fn files_under(dir: &Path) -> Vec<String> {
        let mut names = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return names;
        };
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                names.extend(
                    files_under(&entry.path())
                        .into_iter()
                        .map(|n| format!("{}/{n}", entry.file_name().to_string_lossy())),
                );
            } else {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        names.sort();
        names
    }

    /// The top-level keys of a JSON object, in the order they were written. `serde_json`'s
    /// own object type sorts them, so this reads the map as a sequence of pairs.
    fn ordered_keys(json: &str) -> Vec<String> {
        struct Keys(Vec<String>);
        impl<'de> serde::Deserialize<'de> for Keys {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                struct V;
                impl<'de> serde::de::Visitor<'de> for V {
                    type Value = Keys;
                    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.write_str("a JSON object")
                    }
                    fn visit_map<A: serde::de::MapAccess<'de>>(
                        self,
                        mut map: A,
                    ) -> Result<Keys, A::Error> {
                        let mut keys = Vec::new();
                        while let Some(key) = map.next_key::<String>()? {
                            map.next_value::<serde::de::IgnoredAny>()?;
                            keys.push(key);
                        }
                        Ok(Keys(keys))
                    }
                }
                d.deserialize_map(V)
            }
        }
        serde_json::from_str::<Keys>(json).expect("an object").0
    }

    // ---- root ----

    #[test]
    fn no_archive_and_off_turn_it_off() {
        let never = || -> Result<PathBuf, String> { panic!("git must not be asked") };
        assert_eq!(
            archive_root_choice(true, None, || Ok(PathBuf::from("/g"))),
            Ok(None)
        );
        // The flag beats a variable that names a directory.
        assert_eq!(
            archive_root_choice(true, Some("/somewhere"), never),
            Ok(None)
        );
        for off in ["off", "OFF", "Off", "  off  "] {
            assert_eq!(
                archive_root_choice(false, Some(off), never),
                Ok(None),
                "{off:?}"
            );
        }
        // `off` is a word, not a prefix: a directory whose name starts with it is a directory.
        assert_eq!(
            archive_root_choice(false, Some("offline"), never),
            Ok(Some(PathBuf::from("offline")))
        );
    }

    #[test]
    fn archive_root_prefers_the_variable() {
        let asked = Cell::new(false);
        let root = archive_root_choice(false, Some("elsewhere/archive"), || {
            asked.set(true);
            Ok(PathBuf::from("/g"))
        });
        assert_eq!(root, Ok(Some(PathBuf::from("elsewhere/archive"))));
        assert!(!asked.get(), "a named root never asks git");
    }

    #[test]
    fn archive_root_defaults_to_the_git_common_dir() {
        for unset in [None, Some(""), Some("   ")] {
            assert_eq!(
                archive_root_choice(false, unset, || Ok(PathBuf::from("/clone/.git"))),
                Ok(Some(PathBuf::from("/clone/.git").join(ARCHIVE_DIR_NAME))),
                "{unset:?}"
            );
        }
    }

    #[test]
    fn an_unresolvable_git_dir_turns_the_archive_off() {
        let dir = scratch("nogit");
        let resolved = resolve_archive_root(
            false,
            None,
            &dir,
            &dir,
            || Err("git rev-parse --git-common-dir failed".to_string()),
            |_| panic!("no root, nothing to check"),
        );
        let warning = resolved.expect_err("no root can be guessed");
        assert!(warning.contains("git common directory"), "{warning}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_git_dir_root_is_accepted_without_check_ignore() {
        let repo = scratch("gitdir-root");
        let git = repo.join(".git");
        std::fs::create_dir_all(&git).expect("fake .git");
        let resolved = resolve_archive_root(
            false,
            None,
            &repo,
            &repo,
            || Ok(git.clone()),
            |_| panic!("git check-ignore exits 1 for .git paths: it must not be asked"),
        );
        let root = resolved.expect("accepted").expect("on");
        assert!(root.ends_with(ARCHIVE_DIR_NAME), "{}", root.display());
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn an_in_tree_root_git_does_not_ignore_is_refused() {
        let repo = scratch("intree-refused");
        let asked = std::cell::RefCell::new(Vec::new());
        let resolved = resolve_archive_root(
            false,
            Some("out/archive"),
            &repo,
            &repo,
            || panic!("a named root never asks git"),
            |relative| {
                asked.borrow_mut().push(relative.to_path_buf());
                Ok(false)
            },
        );
        let warning = resolved.expect_err("one `git add -A` from a commit");
        assert!(warning.contains("not ignored by git"), "{warning}");
        assert_eq!(
            asked.borrow().as_slice(),
            [Path::new("out")
                .join("archive")
                .join("public")
                .join("probe.jsonl")],
            "git is asked about a record path under the root"
        );
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn an_in_tree_root_git_ignores_is_accepted() {
        let repo = scratch("intree-accepted");
        let resolved = resolve_archive_root(
            false,
            Some("artifacts/archive"),
            &repo,
            &repo,
            || panic!("a named root never asks git"),
            |_| Ok(true),
        );
        assert!(resolved.expect("accepted").is_some());
        // Outside the tree, git is not asked at all.
        let elsewhere = scratch("outside-tree");
        let resolved = resolve_archive_root(
            false,
            Some(elsewhere.to_str().expect("utf-8 temp path")),
            &repo,
            &repo,
            || panic!("a named root never asks git"),
            |_| panic!("outside the tree there is nothing for git to stage"),
        );
        assert!(resolved.expect("accepted").is_some());
        let _ = std::fs::remove_dir_all(&repo);
        let _ = std::fs::remove_dir_all(&elsewhere);
    }

    // ---- files ----

    #[test]
    fn a_run_is_partial_until_finished_then_renamed() {
        let root = scratch("partial-then-final");
        let h = header(1_000_000_000_000);
        let archive = Archive::start(&root, &h);
        assert!(files_under(&root).is_empty(), "no record, no file");
        archive.record(ArchiveTrack::Synthetic, &record(&h.run_id, "text", None));
        let names = files_under(&root);
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(names[0].ends_with(".jsonl.partial"), "{names:?}");
        archive.finish();
        let names = files_under(&root);
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(
            names[0].ends_with(".jsonl") && !names[0].contains("partial"),
            "{names:?}"
        );
        let body = std::fs::read_to_string(
            root.join("public")
                .join(names[0].rsplit('/').next().unwrap()),
        )
        .expect("the finished file");
        assert_eq!(body.lines().count(), 2, "one header and one record");
        assert!(!archive.is_on(), "finish closes the archive");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_unfinished_run_leaves_only_the_partial() {
        let root = scratch("unfinished");
        let h = header(1_000_000_000_000);
        {
            let archive = Archive::start(&root, &h);
            archive.record(ArchiveTrack::Public, &record(&h.run_id, "text", None));
            // Dropped without `finish`: the run was killed or exited early.
        }
        let names = files_under(&root);
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(
            names[0].ends_with(".jsonl.partial"),
            "a dropped archive must not look complete: {names:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn records_reach_disk_at_each_provider_boundary() {
        let root = scratch("flush");
        let h = header(1_000_000_000_000);
        let archive = Archive::start(&root, &h);
        archive.record(
            ArchiveTrack::Public,
            &record(&h.run_id, "first provider", None),
        );
        archive.flush();
        // Killed here, in the second provider: the first provider's records are on disk.
        let partial = root
            .join("public")
            .join(files_under(&root.join("public")).remove(0));
        let body = std::fs::read_to_string(&partial).expect("readable while open");
        assert_eq!(
            body.lines().count(),
            2,
            "header and the flushed record: {body}"
        );
        assert!(body.contains("first provider"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_existing_file_is_never_overwritten() {
        let root = scratch("no-overwrite");
        let h = header(1_000_000_000_000);
        let stem = format!(
            "{}-provider-bench-{}",
            utc_stamp(1_000_000_000),
            &h.run_id[..12]
        );
        let public = root.join("public");
        std::fs::create_dir_all(&public).expect("track dir");

        // A `.partial` of the same name: creation fails, the file is untouched, one warning.
        let partial = public.join(format!("{stem}.jsonl.partial"));
        std::fs::write(&partial, b"earlier run").expect("seed the partial");
        let archive = Archive::start(&root, &h);
        archive.record(ArchiveTrack::Public, &record(&h.run_id, "text", None));
        assert_eq!(
            std::fs::read(&partial).expect("still there"),
            b"earlier run"
        );
        assert_eq!(archive.warnings().len(), 1);
        assert!(!archive.is_on());
        std::fs::remove_file(&partial).expect("clear");

        // A finished `.jsonl` of the same name: finishing does not replace it.
        let finished = public.join(format!("{stem}.jsonl"));
        std::fs::write(&finished, b"finished earlier").expect("seed the finished file");
        let archive = Archive::start(&root, &h);
        archive.record(ArchiveTrack::Public, &record(&h.run_id, "text", None));
        archive.finish();
        assert_eq!(
            std::fs::read(&finished).expect("still there"),
            b"finished earlier"
        );
        assert_eq!(archive.warnings().len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn file_names_sort_by_start_and_hold_no_colon() {
        assert_eq!(utc_stamp(0), "19700101T000000Z");
        assert_eq!(utc_stamp(1_000_000_000), "20010909T014640Z");
        assert_eq!(utc_stamp(1_759_190_400), "20250930T000000Z");
        // A leap day, the century that is a leap year, and the century that is not.
        assert_eq!(utc_stamp(1_709_164_800), "20240229T000000Z");
        assert_eq!(utc_stamp(951_782_400), "20000229T000000Z");
        assert_eq!(utc_stamp(4_102_444_800), "21000101T000000Z");
        let times = [
            0_u64,
            59,
            60,
            3_599,
            3_600,
            86_399,
            86_400,
            1_000_000_000,
            4_102_444_800,
        ];
        let stamps: Vec<String> = times.iter().map(|t| utc_stamp(*t)).collect();
        let mut sorted = stamps.clone();
        sorted.sort();
        assert_eq!(stamps, sorted, "later starts sort later");
        assert!(stamps.iter().all(|s| !s.contains(':') && s.len() == 16));

        let root = scratch("names");
        let early = header(1_000_000_000_000);
        let late = header(1_000_000_060_000);
        for h in [&late, &early] {
            let archive = Archive::start(&root, h);
            archive.record(ArchiveTrack::Public, &record(&h.run_id, "t", None));
            archive.finish();
        }
        let names = files_under(&root.join("public"));
        assert_eq!(names.len(), 2);
        assert!(
            names[0].starts_with("20010909T014640Z-provider-bench-"),
            "{names:?}"
        );
        assert!(
            names[1].starts_with("20010909T014740Z-provider-bench-"),
            "{names:?}"
        );
        assert!(names.iter().all(|n| !n.contains(':')));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_private_track_record_is_never_written() {
        let root = scratch("private");
        let h = header(1_000_000_000_000);
        let archive = Archive::start(&root, &h);
        archive.record(
            ArchiveTrack::Private,
            &record(&h.run_id, "private text", None),
        );
        archive.flush();
        archive.finish();
        assert!(
            files_under(&root).is_empty(),
            "no file for a private record"
        );
        assert!(!root.join("private").exists(), "no private/ directory");
        assert!(archive.warnings().is_empty(), "dropping is not a failure");
        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- the private track: text-free records (Decision 7) ----

    /// A private record built from fabricated values only.
    fn private_record<'a>(
        run_id: &str,
        truth: Option<&'a TruthComparison>,
    ) -> PrivateDocRecord<'a> {
        PrivateDocRecord {
            kind: "private_doc",
            run_id: Sha256Hex::new(run_id).expect("a run id is a SHA-256"),
            provider: "mrz",
            track: ArchiveTrack::Private.as_str(),
            source_sha256: Sha256Hex::new(&"c".repeat(64)).expect("hex"),
            outcome: "hit",
            mrz_format: Some("TD3"),
            mrz_found: true,
            mrz_checksums_valid: true,
            check_states: Some(BTreeMap::from([
                ("composite", Some(true)),
                ("personal_number", None),
            ])),
            retry_variant_id: Some(RetryVariant::Pass(3)),
            retry_budget_hit: false,
            retry_stop: Some(RetryStop::VariantValid),
            retry_damaged_recovery: Some(false),
            tier1_damaged_recovery: None,
            read_us: 12,
            ocr_ms: 34,
            mrz_band_score: Some(0.5),
            rotation: 0,
            truth,
            zone_classes: Some(ZoneClasses::of_zone("AB1<\nZ9")),
        }
    }

    #[test]
    fn a_private_record_has_the_pinned_keys_in_order() {
        let h = header(1_000_000_000_000);
        let json = serde_json::to_string(&private_record(&h.run_id, None)).expect("serialize");
        assert_eq!(
            ordered_keys(&json),
            [
                "kind",
                "run_id",
                "provider",
                "track",
                "source_sha256",
                "outcome",
                "mrz_format",
                "mrz_found",
                "mrz_checksums_valid",
                "check_states",
                "retry_variant_id",
                "retry_budget_hit",
                "retry_stop",
                "retry_damaged_recovery",
                "tier1_damaged_recovery",
                "read_us",
                "ocr_ms",
                "mrz_band_score",
                "rotation",
                "truth",
                "zone_classes",
            ]
        );
        let value: serde_json::Value = serde_json::from_str(&json).expect("JSON");
        assert_eq!(value["kind"], "private_doc");
        assert_eq!(value["track"], "private");
        assert_eq!(value["retry_variant_id"], "pass-03");
        assert_eq!(value["retry_stop"], "variant_valid");
        assert_eq!(
            value["check_states"],
            serde_json::json!({"composite": true, "personal_number": null})
        );
        assert_eq!(value["zone_classes"], serde_json::json!(["AA9<", "A9"]));
        assert!(value["truth"].is_null());
    }

    #[test]
    fn a_private_records_truth_carries_the_code_as_classes_only() {
        let h = header(1_000_000_000_000);
        let truth = TruthComparison {
            zone_mismatch: Some(2),
            compared_cells: Some(88),
            field_mismatch: None,
            code_cells: Some(document_code_classes(
                "PSUTOSECRETSURNAME<<GIVEN\nL898902C36",
            )),
        };
        let json = serde_json::to_string(&private_record(&h.run_id, Some(&truth))).expect("JSON");
        let value: serde_json::Value = serde_json::from_str(&json).expect("JSON");
        assert_eq!(
            value["truth"],
            serde_json::json!({
                "zone_mismatch": 2,
                "compared_cells": 88,
                "field_mismatch": null,
                "code_cells": "AA"
            })
        );
        assert!(!json.contains("PS") && !json.contains("SECRET"), "{json}");
    }

    #[test]
    fn a_document_code_is_written_as_at_most_two_class_symbols() {
        for (zone, expected) in [
            ("P<UTOX\nY", "A<"),
            ("PSUTOX", "AA"),
            ("I<UTO", "A<"),
            ("1<", "9<"),
            ("\u{00e9}Z", "?A"),
            ("P", "A"),
            ("", ""),
            ("\nPS", ""),
        ] {
            assert_eq!(document_code_classes(zone), expected, "{zone:?}");
        }
    }

    #[test]
    fn a_zone_is_written_as_classes_and_never_as_characters() {
        // Letter, digit, filler, a lowercase letter (still a letter), a non-ASCII letter and a
        // symbol (both "other"), and a last line shorter than the first.
        let classes = ZoneClasses::of_zone("AZ09<<az\u{00e9}\u{0416}-?\nB3<");
        assert_eq!(
            serde_json::to_value(&classes).expect("serialize"),
            serde_json::json!(["AA99<<AA????", "A9<"])
        );
        // The same partition as `classify_char` in tools/archive_query.py.
        for (c, class) in [
            ('Q', CellClass::Letter),
            ('q', CellClass::Letter),
            ('0', CellClass::Digit),
            ('<', CellClass::Filler),
            (' ', CellClass::Other),
            ('\u{00e9}', CellClass::Other),
            ('\u{0664}', CellClass::Other),
        ] {
            assert_eq!(CellClass::of(c), class, "{c:?}");
        }
        assert_eq!(
            serde_json::to_value(ZoneClasses::of_zone("")).expect("serialize"),
            serde_json::json!([])
        );
    }

    #[test]
    fn the_private_key_types_admit_only_closed_values() {
        assert!(Sha256Hex::new(&"a".repeat(64)).is_some());
        for bad in [
            "a".repeat(63),
            "a".repeat(65),
            "A".repeat(64),
            "g".repeat(64),
            "passport scan.png".to_string(),
            String::new(),
        ] {
            assert!(Sha256Hex::new(&bad).is_none(), "{bad:?}");
        }
        for stop in [
            "general_valid",
            "variant_valid",
            "pass_cap",
            "budget",
            "exhausted",
        ] {
            let parsed = RetryStop::parse(stop).expect(stop);
            assert_eq!(serde_json::to_value(parsed).expect("serialize"), stop);
        }
        assert!(RetryStop::parse("stopped at a name").is_none());
        assert!(RetryStop::parse("").is_none());
        for (text, variant) in [
            ("general", RetryVariant::General),
            ("pass-01", RetryVariant::Pass(1)),
            ("pass-13", RetryVariant::Pass(13)),
            ("pass-120", RetryVariant::Pass(120)),
        ] {
            assert_eq!(RetryVariant::parse(text), Some(variant), "{text}");
            assert_eq!(serde_json::to_value(variant).expect("serialize"), text);
        }
        for bad in [
            "pass-1",
            "pass-",
            "pass-ab",
            "pass-123456",
            "Pass-01",
            "a name",
        ] {
            assert!(RetryVariant::parse(bad).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn record_private_writes_only_to_private_and_record_still_drops_the_track() {
        let root = scratch("private-door");
        let h = header(1_000_000_000_000);
        let archive = Archive::start(&root, &h);
        // The generic door stays shut, even with the type a private record would carry.
        archive.record(ArchiveTrack::Private, &private_record(&h.run_id, None));
        archive.flush();
        assert!(
            files_under(&root).is_empty(),
            "record() never writes private"
        );
        archive.record_private(&private_record(&h.run_id, None));
        archive.record_private(&private_record(&h.run_id, None));
        archive.finish();
        let names = files_under(&root);
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(names[0].starts_with("private/"), "{names:?}");
        assert!(!root.join("public").exists() && !root.join("local").exists());
        let body = std::fs::read_to_string(root.join(&names[0])).expect("the private file");
        assert_eq!(body.lines().count(), 3, "one header and two records");
        assert!(archive.warnings().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_private_header_drops_argv_and_nothing_else() {
        let root = scratch("private-header");
        let h = header(1_000_000_000_000);
        let archive = Archive::start(&root, &h);
        archive.record(ArchiveTrack::Public, &record(&h.run_id, "t", None));
        archive.record_private(&private_record(&h.run_id, None));
        archive.finish();
        let first_line = |dir: &str| {
            let name = files_under(&root.join(dir)).remove(0);
            std::fs::read_to_string(root.join(dir).join(name))
                .expect("read")
                .lines()
                .next()
                .expect("a header")
                .to_string()
        };
        let (public, private) = (first_line("public"), first_line("private"));
        let public_json: serde_json::Value = serde_json::from_str(&public).expect("JSON");
        let private_json: serde_json::Value = serde_json::from_str(&private).expect("JSON");
        assert_eq!(public_json["argv"], serde_json::json!(["--count", "5"]));
        assert!(private_json["argv"].is_null(), "{private}");
        // The same run: the same id, and every other key equal, in the same order.
        assert_eq!(private_json["run_id"], public_json["run_id"]);
        assert_eq!(private_json["model_sha256"], public_json["model_sha256"]);
        assert!(private_json["model_sha256"]["detection"].is_string());
        assert_eq!(ordered_keys(&private), ordered_keys(&public));
        let mut without_argv = public_json.clone();
        without_argv["argv"] = serde_json::Value::Null;
        assert_eq!(private_json, without_argv);
        // The id still hashes the whole header, argv included: the files are joined by it.
        assert_eq!(public_json["run_id"], h.run_id.as_str());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn finish_renames_the_private_partial_with_the_others() {
        let root = scratch("private-finish");
        let h = header(1_000_000_000_000);
        let archive = Archive::start(&root, &h);
        archive.record(ArchiveTrack::Public, &record(&h.run_id, "t", None));
        archive.record_private(&private_record(&h.run_id, None));
        let names = files_under(&root);
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(
            names.iter().all(|n| n.ends_with(".jsonl.partial")),
            "{names:?}"
        );
        archive.finish();
        let names = files_under(&root);
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(names.iter().all(|n| n.ends_with(".jsonl")), "{names:?}");
        // The same stem in each directory.
        let stem = |n: &String| n.rsplit('/').next().expect("a name").to_string();
        assert_eq!(stem(&names[0]), stem(&names[1]), "{names:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn private_records_reach_disk_at_each_provider_boundary() {
        let root = scratch("private-flush");
        let h = header(1_000_000_000_000);
        let archive = Archive::start(&root, &h);
        archive.record_private(&private_record(&h.run_id, None));
        archive.flush();
        // Killed here, in the next provider: the flushed record is in the `.partial`.
        let dir = root.join("private");
        let partial = dir.join(files_under(&dir).remove(0));
        assert!(partial.to_string_lossy().ends_with(".jsonl.partial"));
        let body = std::fs::read_to_string(&partial).expect("readable while open");
        assert_eq!(body.lines().count(), 2, "header and the flushed record");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_run_with_no_private_document_makes_no_private_directory() {
        let root = scratch("no-private-dir");
        let h = header(1_000_000_000_000);
        let archive = Archive::start(&root, &h);
        archive.record(ArchiveTrack::Public, &record(&h.run_id, "t", None));
        archive.record(ArchiveTrack::Local, &record(&h.run_id, "t", None));
        archive.flush();
        archive.finish();
        assert!(!root.join("private").exists(), "{:?}", files_under(&root));
        assert!(root.join("public").exists() && root.join("local").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn tracks_go_to_their_own_directories() {
        let root = scratch("tracks");
        let h = header(1_000_000_000_000);
        let archive = Archive::start(&root, &h);
        for track in [
            ArchiveTrack::Synthetic,
            ArchiveTrack::Public,
            ArchiveTrack::Covers,
            ArchiveTrack::Local,
        ] {
            archive.record(track, &record(&h.run_id, "t", None));
        }
        archive.finish();
        let public = files_under(&root.join("public"));
        let local = files_under(&root.join("local"));
        assert_eq!((public.len(), local.len()), (1, 1), "{public:?} {local:?}");
        let count = |dir: &str, name: &str| {
            std::fs::read_to_string(root.join(dir).join(name))
                .expect("read")
                .lines()
                .count()
        };
        assert_eq!(
            count("public", &public[0]),
            1 + 3,
            "header, synthetic, public, covers"
        );
        assert_eq!(
            count("local", &local[0]),
            1 + 1,
            "header and the local record"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_write_failure_is_one_warning_and_turns_the_archive_off() {
        let dir = scratch("write-failure");
        let root_is_a_file = dir.join("not-a-directory");
        std::fs::write(&root_is_a_file, b"x").expect("a file where the root should be");
        let h = header(1_000_000_000_000);
        let archive = Archive::start(&root_is_a_file, &h);
        // Nothing here can return an error or panic: every method is `()`.
        archive.record(ArchiveTrack::Public, &record(&h.run_id, "t", None));
        archive.record(ArchiveTrack::Public, &record(&h.run_id, "t", None));
        archive.flush();
        archive.finish();
        let warnings = archive.warnings();
        assert_eq!(
            warnings.len(),
            1,
            "one warning, then the archive is off: {warnings:?}"
        );
        assert!(
            warnings[0].starts_with("warning: archive: "),
            "{}",
            warnings[0]
        );
        assert!(!archive.is_on());
        assert_eq!(std::fs::read(&root_is_a_file).expect("untouched"), b"x");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- header and record shapes ----

    #[test]
    fn header_keys_are_pinned() {
        let h = header(1_000_000_000_000);
        let json = serde_json::to_string(&h).expect("serialize");
        assert_eq!(
            ordered_keys(&json),
            [
                "kind",
                "schema",
                "run_id",
                "binary_name",
                "started_unix_ms",
                "pid",
                "source",
                "machine",
                "binary",
                "binary_sha256",
                "git_commit",
                "working_tree_dirty",
                "argv",
                "scope",
                "tracks",
                "samples_data_sha",
                "corpus_manifest_sha256",
                "documents_loaded",
                "labelled_loaded",
                "providers",
                "ocr_arms",
                "retry_budget",
                "mrz_arms",
                "pivot_yy",
                "model_paths",
                "model_sha256",
                "replay_of",
                "env",
            ]
        );
        assert!(json.starts_with(r#"{"kind":"run","schema":1,"run_id":""#));
        // The run id is the SHA-256 of the header serialized with the id empty.
        let mut blank = h.clone();
        blank.run_id = String::new();
        assert_eq!(
            h.run_id,
            sha256_hex(&serde_json::to_vec(&blank).expect("serialize"))
        );
        // A different start is a different run.
        assert_ne!(h.run_id, header(1_000_000_000_001).run_id);
    }

    /// The SHA-256 of `abc`, a published test vector.
    const SHA256_OF_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn model_hashes_are_the_sha256_of_each_files_bytes() {
        let dir = scratch("model-hashes");
        let (detection, recognition) = (dir.join("det.rten"), dir.join("rec.rten"));
        std::fs::write(&detection, b"abc").expect("write");
        std::fs::write(&recognition, b"").expect("write");
        let hashes = ModelSha256::of_files(&detection, &recognition);
        assert_eq!(hashes.detection.as_deref(), Some(SHA256_OF_ABC));
        // The SHA-256 of no bytes.
        assert_eq!(
            hashes.recognition.as_deref(),
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unreadable_model_file_is_a_null_for_its_key_and_never_an_error() {
        let dir = scratch("model-hashes-missing");
        let present = dir.join("det.rten");
        std::fs::write(&present, b"abc").expect("write");
        let missing = dir.join("nothing.rten");
        let hashes = ModelSha256::of_files(&present, &missing);
        assert_eq!(hashes.detection.as_deref(), Some(SHA256_OF_ABC));
        assert_eq!(hashes.recognition, None);
        let json = serde_json::to_value(&hashes).expect("serialize");
        assert!(json["recognition"].is_null(), "the key is present and null");
        // A directory where a file should be is unreadable too.
        assert_eq!(ModelSha256::of_files(&dir, &dir).detection, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_header_carries_the_hashes_and_a_replay_carries_null() {
        let live = serde_json::to_value(header(1_000_000_000_000)).expect("serialize");
        assert_eq!(live["model_sha256"]["detection"], "a".repeat(64));
        assert_eq!(live["model_sha256"]["recognition"], "b".repeat(64));
        let mut replay = header(1_000_000_000_000);
        replay.model_sha256 = None;
        let json = serde_json::to_string(&replay.with_run_id()).expect("serialize");
        assert!(
            json.contains(r#""model_sha256":null"#),
            "always serialized: {json}"
        );
    }

    #[test]
    fn two_headers_that_differ_only_in_one_model_hash_have_different_run_ids() {
        let a = header(1_000_000_000_000);
        let mut b = header(1_000_000_000_000);
        assert_eq!(a.run_id, b.run_id, "the fixture is deterministic");
        b.model_sha256 = Some(ModelSha256 {
            detection: Some("c".repeat(64)),
            recognition: Some("b".repeat(64)),
        });
        assert_ne!(a.run_id, b.with_run_id().run_id);
    }

    #[test]
    fn doc_record_keys_are_pinned() {
        let rec = record("abc", "some text", None);
        let json = serde_json::to_string(&rec).expect("serialize");
        assert_eq!(
            ordered_keys(&json),
            [
                "kind",
                "run_id",
                "provider",
                "track",
                "name",
                "asset_id",
                "source_sha256",
                "ledger_row",
                "read_ok",
                "read_us",
                "field_correctness",
                "ocr",
                "tier1_read",
                "fields",
                "truth",
            ]
        );
        let ocr = serde_json::to_string(&rec.ocr).expect("serialize");
        assert_eq!(
            ordered_keys(&ocr),
            [
                "text",
                "rotation",
                "mrz_band_score",
                "chargrid",
                "ocr_passes"
            ]
        );
        let tier1 = Tier1ReadRecord {
            lines: vec!["A".to_string()],
            damaged_recovery: false,
            valid: true,
        };
        assert_eq!(
            ordered_keys(&serde_json::to_string(&tier1).expect("serialize")),
            ["lines", "damaged_recovery", "valid"]
        );
    }

    #[test]
    fn header_env_is_the_allowlist_only() {
        let sentinel = |name: &str| format!("SENTINEL-{name}-DO-NOT-RECORD");
        let mut vars: Vec<(String, String)> = [
            "SYNTHPASS_KEY",
            "SYNTHPASS_TOKEN",
            "SYNTHPASS_TLS_KEY",
            "SYNTHPASS_LICENSE_PRIVKEY",
            "GITHUB_TOKEN",
            "SYNTHPASS_OCR_TEXTURE_EXTRA",
            "PATH",
        ]
        .iter()
        .map(|n| ((*n).to_string(), sentinel(n)))
        .collect();
        vars.push(("SYNTHPASS_OCR_TEXTURE".to_string(), "off".to_string()));
        vars.push(("SYNTHPASS_OCR_MAX_SECONDS".to_string(), "600".to_string()));
        vars.push(("RTEN_NUM_THREADS".to_string(), "4".to_string()));
        // A set variable whose value is not a plain setting is recorded as null.
        vars.push((
            "SYNTHPASS_OCR_ORDER".to_string(),
            "/some/path with spaces".to_string(),
        ));
        vars.push(("SYNTHPASS_OCR_SKEW".to_string(), "x".repeat(33)));
        vars.push(("SYNTHPASS_OCR_ROTATE".to_string(), String::new()));

        let env = allowlisted_env(vars);
        let mut h = header(1_000_000_000_000);
        h.env = env.clone();
        let bytes = serde_json::to_string(&h).expect("serialize");
        assert!(
            !bytes.contains("SENTINEL"),
            "a credential-shaped value reached the header"
        );
        for name in [
            "SYNTHPASS_KEY",
            "SYNTHPASS_TOKEN",
            "SYNTHPASS_TLS_KEY",
            "SYNTHPASS_LICENSE_PRIVKEY",
            "GITHUB_TOKEN",
            "SYNTHPASS_OCR_TEXTURE_EXTRA",
            "PATH",
        ] {
            assert!(!bytes.contains(name), "{name} reached the header");
            assert!(!env.contains_key(name));
        }
        assert_eq!(env["SYNTHPASS_OCR_TEXTURE"].as_deref(), Some("off"));
        assert_eq!(env["SYNTHPASS_OCR_MAX_SECONDS"].as_deref(), Some("600"));
        assert_eq!(env["RTEN_NUM_THREADS"].as_deref(), Some("4"));
        assert_eq!(env["SYNTHPASS_OCR_ORDER"], None);
        assert_eq!(env["SYNTHPASS_OCR_SKEW"], None);
        assert_eq!(env["SYNTHPASS_OCR_ROTATE"], None);
        assert_eq!(
            env.len(),
            6,
            "only the allowlisted names that are set: {env:?}"
        );
    }

    #[test]
    fn the_allowlist_names_no_credential() {
        for name in ARCHIVE_ENV_ALLOWLIST {
            for word in [
                "KEY", "TOKEN", "SECRET", "PASSWORD", "TLS", "CERT", "LICENSE", "PRIV",
            ] {
                assert!(!name.contains(word), "{name} contains {word}");
            }
            assert!(
                name.starts_with("SYNTHPASS_") || *name == "RTEN_NUM_THREADS",
                "{name}"
            );
        }
        let mut sorted = ARCHIVE_ENV_ALLOWLIST.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            ARCHIVE_ENV_ALLOWLIST.len(),
            "a name is listed twice"
        );
    }

    #[test]
    fn source_label_and_sha_follow_their_rules() {
        assert_eq!(source_from(Some("true")), "ci");
        for other in [None, Some("false"), Some("TRUE"), Some("1"), Some("")] {
            assert_eq!(source_from(other), "local", "{other:?}");
        }
        let sha = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(samples_data_sha_from(Some(sha)).as_deref(), Some(sha));
        for bad in [
            None,
            Some(""),
            Some("main"),
            Some(&sha[..39]),
            Some("0123456789abcdef0123456789abcdef0123456789"),
            Some("g123456789abcdef0123456789abcdef01234567"),
        ] {
            assert_eq!(samples_data_sha_from(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn the_machine_label_is_the_os_and_the_processor_never_the_host() {
        assert_eq!(
            machine_label_from(
                "windows",
                Some(26_200),
                Some("Intel(R) Core(TM) i5-4570 CPU @ 3.20GHz")
            ),
            "win11-i5-4570"
        );
        assert_eq!(
            machine_label_from(
                "windows",
                Some(22_631),
                Some("12th Gen Intel(R) Core(TM) i7-1255U")
            ),
            "win11-i7-1255U"
        );
        // Windows 11 starts at build 22000; below it is Windows 10, unreadable is just `windows`.
        assert_eq!(
            machine_label_from(
                "windows",
                Some(21_999),
                Some("Intel(R) Core(TM) i5-4570 CPU")
            ),
            "win10-i5-4570"
        );
        assert_eq!(
            machine_label_from(
                "windows",
                Some(22_000),
                Some("Intel(R) Core(TM) i5-4570 CPU")
            ),
            "win11-i5-4570"
        );
        assert_eq!(
            machine_label_from("windows", None, Some("Intel(R) Core(TM) i5-4570 CPU")),
            "windows-i5-4570"
        );
        assert_eq!(
            machine_label_from(
                "linux",
                None,
                Some("Intel(R) Xeon(R) Platinum 8272CL CPU @ 2.60GHz")
            ),
            "linux-xeon-8272CL"
        );
        assert_eq!(machine_label_from("linux", None, None), "linux-unknown");
        assert_eq!(machine_label_from("macos", None, Some("")), "macos-unknown");
    }

    #[test]
    fn cpu_designation_follows_the_vendor_forms() {
        for (brand, expected) in [
            ("Intel(R) Core(TM) i5-4570 CPU @ 3.20GHz", "i5-4570"),
            ("12th Gen Intel(R) Core(TM) i7-1255U", "i7-1255U"),
            ("Intel(R) Core(TM) i9-14900K", "i9-14900K"),
            ("Intel(R) Core(TM) Ultra 7 155H", "ultra7-155H"),
            (
                "Intel(R) Xeon(R) CPU E5-2673 v4 @ 2.30GHz",
                "xeon-E5-2673-v4",
            ),
            (
                "Intel(R) Xeon(R) Platinum 8272CL CPU @ 2.60GHz",
                "xeon-8272CL",
            ),
            ("Intel(R) Xeon(R) W-2295 CPU @ 3.00GHz", "xeon-W-2295"),
            ("AMD EPYC 7763 64-Core Processor", "epyc-7763"),
            ("AMD Ryzen 7 5800X 8-Core Processor", "ryzen7-5800X"),
            // Unrecognised: vendor words and the clock dropped, the rest made plain.
            ("Apple M2 Pro", "apple-m2-pro"),
            ("Intel(R) Pentium(R) CPU G4560 @ 3.50GHz", "pentium-g4560"),
        ] {
            assert_eq!(cpu_designation(Some(brand)), expected, "{brand}");
        }
        for nothing in [None, Some(""), Some("   "), Some("Intel(R) CPU")] {
            assert_eq!(cpu_designation(nothing), "unknown", "{nothing:?}");
        }
        // Whatever the brand string holds, the label is plain: no path separator, space or `:`.
        let odd = cpu_designation(Some("Weird/CPU: name\\with spaces 9000"));
        assert!(
            odd.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-')),
            "{odd}"
        );
        assert!(odd.len() <= 48);
        assert!(cpu_designation(Some(&"x".repeat(200))).len() <= 48);
    }

    #[test]
    fn the_brand_string_is_read_from_registry_and_cpuinfo_output() {
        let cpu = "\r\nHKEY_LOCAL_MACHINE\\HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0\r\n    \
                   ProcessorNameString    REG_SZ    Intel(R) Core(TM) i5-4570 CPU @ 3.20GHz\r\n\r\n";
        assert_eq!(
            reg_value(cpu, "ProcessorNameString").as_deref(),
            Some("Intel(R) Core(TM) i5-4570 CPU @ 3.20GHz")
        );
        let build = "HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\r\n    \
                     CurrentBuildNumber    REG_SZ    26200\r\n";
        assert_eq!(
            reg_value(build, "CurrentBuildNumber").as_deref(),
            Some("26200")
        );
        assert_eq!(
            reg_value(
                "ProcessorNameString    REG_SZ    \r\n",
                "ProcessorNameString"
            ),
            None
        );
        assert_eq!(reg_value("nothing here", "ProcessorNameString"), None);

        let info = "processor\t: 0\nvendor_id\t: GenuineIntel\nmodel name\t: Intel(R) Xeon(R) \
                    Platinum 8272CL CPU @ 2.60GHz\nmodel name\t: second core\n";
        assert_eq!(
            cpuinfo_model(info).as_deref(),
            Some("Intel(R) Xeon(R) Platinum 8272CL CPU @ 2.60GHz")
        );
        assert_eq!(cpuinfo_model("processor\t: 0\nHardware\t: BCM2835\n"), None);
    }

    #[test]
    fn a_path_is_shown_without_the_extended_prefix() {
        assert_eq!(plain_path(Path::new(r"\\?\D:\a\b.jsonl")), r"D:\a\b.jsonl");
        assert_eq!(
            plain_path(Path::new(r"\\?\UNC\host\share\b")),
            r"\\host\share\b"
        );
        assert_eq!(plain_path(Path::new(r"D:\a\b")), r"D:\a\b");
        assert_eq!(plain_path(Path::new("plain/a")), "plain/a");
        // A prefix anywhere but the start is part of a name and stays.
        assert_eq!(plain_path(Path::new(r"x\\?\y")), r"x\\?\y");

        // The warning line uses it too.
        let mut inner = Inner {
            state: State::Off,
            warnings: Vec::new(),
        };
        inner.fail(
            "cannot create",
            Path::new(r"\\?\D:\archive\public\x.partial"),
            &std::io::Error::from(std::io::ErrorKind::NotFound),
        );
        assert_eq!(
            inner.warnings,
            [r"warning: archive: cannot create D:\archive\public\x.partial: NotFound"]
        );
    }

    // ---- the synthetic record (build step 4) ----

    #[test]
    fn the_file_name_carries_the_binary_name() {
        let root = scratch("binary-name");
        let mut h = header(1_000_000_000_000);
        h.binary_name = "synthpass-bench".to_string();
        let h = h.with_run_id();
        let archive = Archive::start(&root, &h);
        archive.record(ArchiveTrack::Synthetic, &record(&h.run_id, "t", None));
        archive.finish();
        let names = files_under(&root.join("public"));
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(
            names[0].starts_with("20010909T014640Z-synthpass-bench-"),
            "{names:?}"
        );
        assert!(names[0].ends_with(".jsonl"), "{names:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    fn synthetic_ledger_row() -> LedgerRow {
        LedgerRow {
            format: "TD3".to_string(),
            seed: 0,
            profile: "clean".to_string(),
            hit: true,
            miss_kind: None,
            wrong_accept: false,
            prefix_wrong_accept: false,
            wrong_fields: Vec::new(),
            check_states: None,
            names_exact: true,
            name_error: None,
            line1_flagged: false,
            retry_stop: None,
            retry_variant_id: None,
            retry_damaged_recovery: None,
            tier1_damaged_recovery: None,
            elapsed_ms: 0,
        }
    }

    fn bare_hit(raw_text: Option<&str>, ocr: bool) -> crate::HitResult {
        crate::HitResult {
            hit: false,
            reason: None,
            check_states: None,
            elapsed: std::time::Duration::ZERO,
            fields: Vec::new(),
            line1_integrity: None,
            names_exact: false,
            name_error: None,
            raw_text: raw_text.map(str::to_string),
            retry_stop: None,
            retry_variant_id: None,
            retry_damaged_recovery: None,
            tier1_damaged_recovery: None,
            ocr: ocr.then_some(crate::OcrFacts {
                rotation: 90,
                mrz_band_score: None,
            }),
        }
    }

    #[test]
    fn a_synthetic_record_has_the_pinned_keys() {
        let root = scratch("synthetic-keys");
        let h = header(1_000_000_000_000);
        let archive = Archive::start(&root, &h);
        let labels =
            synthpass_gen::generate_from_seed(&synthpass_gen::GeneratorConfig::with_document_type(
                0,
                synthpass_gen::DocumentType::TD3,
            ))
            .1;
        archive.record_synthetic(
            &synthetic_ledger_row(),
            &bare_hit(Some("text"), true),
            &labels,
            None,
        );
        archive.finish();
        let public = root.join("public");
        let name = files_under(&public).remove(0);
        let body = std::fs::read_to_string(public.join(name)).expect("the file");
        let line = body.lines().nth(1).expect("the record");
        assert_eq!(
            ordered_keys(line),
            [
                "kind",
                "run_id",
                "track",
                "format",
                "profile",
                "seed",
                "ledger_row",
                "ocr",
                "tier1_read",
                "fields",
                "truth",
            ]
        );
        // OCR succeeded but nothing parsed: no zone, so no read, and a truth comparison whose
        // parts are all null; the fields exist (the read found none of them).
        let value: serde_json::Value = serde_json::from_str(line).expect("JSON");
        assert_eq!(value["ocr"]["rotation"], 90);
        assert!(value["ocr"]["mrz_band_score"].is_null());
        assert!(value["tier1_read"].is_null());
        assert!(value["truth"]["zone_mismatch"].is_null());
        assert!(value["truth"]["field_mismatch"].is_null());
        // The generator's zone is never written: only counts and positions are.
        for zone_line in &labels.mrz_lines {
            assert!(!line.contains(zone_line.as_str()));
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A value that is not valid Unicode on this platform: a lone byte above 0x7f on Unix, an
    /// unpaired surrogate on Windows.
    fn non_unicode_os_string() -> std::ffi::OsString {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            std::ffi::OsString::from_vec(vec![0x66, 0xff, 0xfe])
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            std::ffi::OsString::from_wide(&[0x66, 0xD800])
        }
    }

    #[test]
    fn the_environment_is_looked_up_by_name_and_a_non_unicode_value_is_null() {
        let env = env_from(|name| match name {
            "SYNTHPASS_OCR_TEXTURE" => Some(std::ffi::OsString::from("off")),
            "SYNTHPASS_OCR_ORDER" => Some(non_unicode_os_string()),
            other => {
                // Only allowlisted names are ever asked for, so a credential's name is never
                // looked up and its value never read.
                assert!(
                    ARCHIVE_ENV_ALLOWLIST.contains(&other),
                    "{other} was asked for"
                );
                None
            }
        });
        assert_eq!(env["SYNTHPASS_OCR_TEXTURE"].as_deref(), Some("off"));
        assert_eq!(env["SYNTHPASS_OCR_ORDER"], None);
        assert_eq!(env.len(), 2, "{env:?}");
        // This process's own environment goes through the same function and cannot panic,
        // whatever it holds.
        assert!(process_env()
            .keys()
            .all(|name| ARCHIVE_ENV_ALLOWLIST.contains(&name.as_str())));
    }

    #[test]
    fn git_must_print_exactly_one_absolute_path() {
        let absolute = if cfg!(windows) {
            "D:/repo/.git"
        } else {
            "/repo/.git"
        };
        assert_eq!(
            parse_git_common_dir(&format!("{absolute}\n")),
            Ok(PathBuf::from(absolute))
        );
        // Git before 2.31 does not know the option: it echoes it, then prints a relative path.
        assert!(parse_git_common_dir("--path-format=absolute\n.git\n").is_err());
        assert!(parse_git_common_dir(".git\n").is_err());
        assert!(parse_git_common_dir("").is_err());
        assert!(parse_git_common_dir(&format!("{absolute}\n{absolute}\n")).is_err());
    }

    #[test]
    fn the_civil_date_is_the_utc_date_of_the_seconds() {
        assert_eq!(civil_date(0), (1970, 1, 1));
        assert_eq!(civil_date(86_399), (1970, 1, 1));
        assert_eq!(civil_date(86_400), (1970, 1, 2));
        assert_eq!(civil_date(951_782_400), (2000, 2, 29));
        assert_eq!(civil_date(1_709_164_800), (2024, 2, 29));
        assert_eq!(civil_date(4_102_444_800), (2100, 1, 1));
    }
}
