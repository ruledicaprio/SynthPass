//! Exact check-digit blind-spot arithmetic for ICAO 9303 MRZ fields: the
//! reproducer for
//! `knowledge/benchmarks/checkdigit-blindspots-exact-2026-09-26.md`.
//!
//! Every rate is a ratio of integers, counted by enumeration or by an exact
//! dynamic programme over residues mod 10. Nothing is sampled, and no floating
//! point enters a count; section F's closed-form sample sizes are the only
//! logarithms. The confusion table, the residue classes and the country codes
//! are read from the crate (`mrz::CONFUSABLES`, `mrz::CLASSES`, `mrz::codes()`),
//! so the output follows the parser's own tables. The only hand-entered input
//! is `OBSERVED`, the 126 aligned substitution events of
//! `knowledge/benchmarks/observed-ocrb-confusions-2026-09-21.md` §2.
//!
//! Sections:
//!
//! - A: single-field k-substitution pass rates under three Δ models
//! - B: same-Δ cancellation, enumerated for every Δ and k
//! - C: the composite check digit, alignment and joint detection per format
//! - D: issuer and nationality codes, substitutions that land on a listed code
//! - E: exhaustive-enumeration budgets under a bounded confusion model
//! - F: zero-failure sample sizes for a lower bound, one- and two-sided
//!
//! The laws behind A–C are pinned against the real parser in
//! `tests/checkdigit_algebra.rs`.
//!
//! Run: `cargo run -p mrz --example checkdigit_blindspots_exact`

use std::collections::{BTreeMap, BTreeSet};

use mrz::{blindspot, codes, CLASSES, CONFUSABLES, MRZ_ALPHABET};

const DIGITS: &str = "0123456789";
/// What a line-1 code cell may hold: a letter or the filler.
const CODE_CELL: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ<";
const WEIGHTS: [u32; 3] = [7, 3, 1];
/// −1 (mod 10). A field's own check-digit cell enters its own comparison with
/// this weight, and the composite check-digit cell enters the composite's.
const MINUS_ONE: u32 = 9;

/// `(printed, returned, count)`: the 126 aligned substitution events of the
/// 2026-09-21 confusion note, §2. Miss-conditioned and pooled across fields,
/// so it is used as a marginal Δ distribution, never as per-field truth.
const OBSERVED: [(char, char, u128); 35] = [
    ('<', 'C', 29),
    ('0', 'O', 26),
    ('<', 'E', 10),
    ('<', 'Z', 9),
    ('<', 'S', 8),
    ('<', '3', 7),
    ('0', 'D', 4),
    ('0', '<', 3),
    ('7', '<', 2),
    ('M', 'N', 2),
    ('<', 'I', 2),
    ('2', '0', 1),
    ('0', '2', 1),
    ('9', '4', 1),
    ('9', '<', 1),
    ('8', '<', 1),
    ('4', '<', 1),
    ('9', 'O', 1),
    ('2', '<', 1),
    ('1', '<', 1),
    ('O', '0', 1),
    ('A', '0', 1),
    ('M', '0', 1),
    ('4', '0', 1),
    ('7', '4', 1),
    ('9', '2', 1),
    ('Z', '7', 1),
    ('W', 'M', 1),
    ('N', 'R', 1),
    ('0', 'G', 1),
    ('H', '4', 1),
    ('F', '1', 1),
    ('5', 'S', 1),
    ('Z', '2', 1),
    ('<', '4', 1),
];

/// A character's ICAO value mod 10: the index of its row in `mrz::CLASSES`.
fn residue(c: char) -> usize {
    CLASSES
        .iter()
        .position(|class| class.contains(&c))
        .expect("MRZ alphabet character")
}

/// How far a misread moves the value, `returned − printed` (mod 10).
fn delta(printed: char, returned: char) -> usize {
    (residue(returned) + 10 - residue(printed)) % 10
}

fn gcd(a: u128, b: u128) -> u128 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

fn lcm(a: u128, b: u128) -> u128 {
    a / gcd(a, b) * b
}

fn choose(n: usize, k: usize) -> u128 {
    if k > n {
        return 0;
    }
    (0..k).fold(1u128, |acc, i| acc * (n - i) as u128 / (i + 1) as u128)
}

/// `num / den` as a percentage, rounded half up to `places` decimals.
fn pct(num: u128, den: u128, places: u32) -> String {
    let unit = 10u128.pow(places);
    let scaled = (num * 100 * unit * 2 + den) / (den * 2);
    if places == 0 {
        format!("{scaled}%")
    } else {
        format!(
            "{}.{:0width$}%",
            scaled / unit,
            scaled % unit,
            width = places as usize
        )
    }
}

/// Every way to choose `k` of `n` positions, in lexicographic order.
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    fn extend(start: usize, n: usize, k: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if cur.len() == k {
            out.push(cur.clone());
            return;
        }
        for i in start..n {
            cur.push(i);
            extend(i + 1, n, k, cur, out);
            cur.pop();
        }
    }
    let mut out = Vec::new();
    extend(0, n, k, &mut Vec::with_capacity(k), &mut out);
    out
}

/// `mrz::CONFUSABLES`, walked both ways as the crate's repair search walks it.
fn confusable_rows() -> BTreeMap<char, BTreeSet<char>> {
    let mut rows: BTreeMap<char, BTreeSet<char>> = BTreeMap::new();
    for &(key, group) in CONFUSABLES {
        for alt in group.chars() {
            rows.entry(key).or_default().insert(alt);
            rows.entry(alt).or_default().insert(key);
        }
    }
    rows
}

/// A Δ model: how many of `total` weighted misreads move a cell's value by
/// each residue mod 10.
struct Dist {
    counts: [u128; 10],
    total: u128,
}

impl Dist {
    fn from_events(events: &[(char, char, u128)]) -> Dist {
        let mut counts = [0u128; 10];
        for &(printed, returned, n) in events {
            counts[delta(printed, returned)] += n;
        }
        Dist {
            counts,
            total: events.iter().map(|e| e.2).sum(),
        }
    }

    /// The same distribution over `denominator`, a multiple of `total`.
    fn scaled_to(&self, denominator: u128) -> [u128; 10] {
        let factor = denominator / self.total;
        self.counts.map(|c| c * factor)
    }
}

#[derive(Clone, Copy)]
enum Model {
    /// Printed glyph uniform over the cell's alphabet, misread uniform over
    /// the other symbols. Content-free, and the closed-form baseline.
    Uniform,
    /// Printed glyph uniform over the glyphs with a `CONFUSABLES` row,
    /// misread uniform over that row: what the repair search believes.
    Confusables,
    /// The 126 events of `OBSERVED`.
    Observed,
}

impl Model {
    fn label(self) -> &'static str {
        match self {
            Model::Uniform => "uniform: every symbol for every other",
            Model::Confusables => "confusables: mrz::CONFUSABLES, both directions",
            Model::Observed => "observed: 126 aligned events, 2026-09-21 note",
        }
    }

    fn dist(self, alphabet: &str) -> Dist {
        let events: Vec<(char, char, u128)> = match self {
            Model::Uniform => alphabet
                .chars()
                .flat_map(|a| {
                    alphabet
                        .chars()
                        .filter(move |&b| b != a)
                        .map(move |b| (a, b, 1))
                })
                .collect(),
            Model::Confusables => {
                let rows = confusable_rows();
                alphabet
                    .chars()
                    .flat_map(|a| {
                        rows.get(&a)
                            .into_iter()
                            .flatten()
                            .filter(|b| alphabet.contains(**b))
                            .map(move |&b| (a, b, 1))
                            .collect::<Vec<_>>()
                    })
                    .collect()
            }
            Model::Observed => OBSERVED
                .iter()
                .filter(|(a, b, _)| alphabet.contains(*a) && alphabet.contains(*b))
                .copied()
                .collect(),
        };
        Dist::from_events(&events)
    }
}

/// P(the field's own check digit still agrees | exactly `k` of its `n + 1`
/// cells, data plus check digit, were misread at uniformly chosen cells),
/// as `(numerator, denominator)`.
fn single_field_pass(n: usize, k: usize, data: &Dist, check: &Dist) -> (u128, u128) {
    let scale = lcm(data.total, check.total);
    let cells: Vec<(usize, [u128; 10])> = (0..n)
        .map(|i| (WEIGHTS[i % 3] as usize, data.scaled_to(scale)))
        .chain(std::iter::once((
            MINUS_ONE as usize,
            check.scaled_to(scale),
        )))
        .collect();
    // dp[j][r]: weighted count of j-cell misread patterns, over the cells seen
    // so far, that move the check sum by r (mod 10).
    let mut dp = vec![[0u128; 10]; k + 1];
    dp[0][0] = 1;
    for (weight, shifts) in &cells {
        for j in (1..=k).rev() {
            for r in 0..10 {
                let base = dp[j - 1][r];
                if base == 0 {
                    continue;
                }
                for (d, &count) in shifts.iter().enumerate() {
                    if count != 0 {
                        dp[j][(r + weight * d) % 10] += base * count;
                    }
                }
            }
        }
    }
    (dp[k][0], choose(cells.len(), k) * scale.pow(k as u32))
}

fn section_a() {
    println!("{}", "=".repeat(78));
    println!("A. Single field: share of k-substitution patterns the check digit passes");
    println!("{}", "=".repeat(78));
    let fields = [
        ("document number (9, alnum)", 9, MRZ_ALPHABET),
        ("date (6, digits)", 6, DIGITS),
        ("personal number (14, alnum)", 14, MRZ_ALPHABET),
    ];
    for model in [Model::Uniform, Model::Confusables, Model::Observed] {
        println!("\n[{}]", model.label());
        println!(
            "{:32}{}",
            "field",
            (1..=5)
                .map(|k| format!("{:>10}", format!("k={k}")))
                .collect::<String>()
        );
        let check = model.dist(DIGITS);
        for (label, n, alphabet) in fields {
            let data = model.dist(alphabet);
            let row: String = (1..=5)
                .map(|k| {
                    let (num, den) = single_field_pass(n, k, &data, &check);
                    format!("{:>10}", pct(num, den, 2))
                })
                .collect();
            println!("{label:32}{row}");
        }
        let alnum = model.dist(MRZ_ALPHABET);
        let digits = model.dist(DIGITS);
        println!(
            "    Δ≡0 share (single-error blind, per data cell): {}/{} alnum, {}/{} digits",
            alnum.counts[0], alnum.total, digits.counts[0], digits.total
        );
        println!(
            "    digit-only Δ counts: {:?} of {}",
            digits.counts, digits.total
        );
    }

    let alphabet: Vec<char> = MRZ_ALPHABET.chars().collect();
    let ordered = alphabet.len() * (alphabet.len() - 1);
    let blind = alphabet
        .iter()
        .flat_map(|&a| alphabet.iter().map(move |&b| (a, b)))
        .filter(|&(a, b)| a != b && blindspot(a, b).is_blind())
        .count();
    println!(
        "\nk=1 document number: {blind}/{ordered} blind ordered pairs = {} per data cell, × 9/10 = {}",
        pct(blind as u128, ordered as u128, 2),
        pct(9 * blind as u128, 10 * ordered as u128, 2)
    );
    let blind_events: Vec<_> = OBSERVED
        .iter()
        .filter(|(a, b, _)| blindspot(*a, *b).is_blind())
        .collect();
    let blind_count: u128 = blind_events.iter().map(|e| e.2).sum();
    println!(
        "observed events that are single-error blind: {blind_count}/126 = {} → {}",
        pct(blind_count, 126, 2),
        blind_events
            .iter()
            .map(|(a, b, n)| format!("{a}→{b} ×{n}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let rows = confusable_rows();
    let blind_pairs: BTreeSet<(char, char)> = rows
        .iter()
        .flat_map(|(&a, alts)| alts.iter().map(move |&b| (a.min(b), a.max(b))))
        .filter(|&(a, b)| blindspot(a, b).is_blind())
        .collect();
    println!("CONFUSABLES pairs a check digit cannot separate: {blind_pairs:?}");
}

/// Share of `k`-subsets of an `n`-cell field's data cells at which `k` misreads
/// that each move the value by the same `delta` leave the check sum unchanged.
fn same_delta_pass(n: usize, k: usize, delta: usize) -> (u128, u128) {
    let subsets = combinations(n, k);
    let hits = subsets
        .iter()
        .filter(|s| delta * s.iter().map(|&i| WEIGHTS[i % 3] as usize).sum::<usize>() % 10 == 0)
        .count();
    (hits as u128, subsets.len() as u128)
}

fn section_b() {
    println!("\n{}", "=".repeat(78));
    println!("B. Same-Δ cancellation: k identical shifts at k data cells");
    println!("{}", "=".repeat(78));
    for (label, n) in [
        ("document number (9)", 9),
        ("date (6)", 6),
        ("personal number (14)", 14),
    ] {
        println!(
            "\n{label}:   Δ mod 10 →{}",
            (0..10).map(|d| format!("{d:>8}")).collect::<String>()
        );
        for k in 1..=4 {
            let row: String = (0..10)
                .map(|d| {
                    let (num, den) = same_delta_pass(n, k, d);
                    format!("{:>8}", pct(num, den, 1))
                })
                .collect();
            println!("  k={k}          {row}");
        }
    }
    let (num, den) = same_delta_pass(9, 2, 4);
    println!("\ndocument number, k=2, Δ≡4 (two 0→O): {num}/{den} position pairs pass");

    println!("\nΔ of the most frequent observed confusions (returned − printed, mod 10):");
    let mut frequent: Vec<_> = OBSERVED.iter().collect();
    frequent.sort_by(|a, b| b.2.cmp(&a.2));
    for (a, b, n) in frequent.into_iter().take(8) {
        println!("  {a}→{b}: Δ≡{}  (n={n})", delta(*a, *b));
    }
    println!("Δ of every CONFUSABLES pair, both directions:");
    for (a, alts) in confusable_rows() {
        for b in alts.into_iter().filter(|&b| a < b) {
            println!("  {a}→{b}: Δ≡{}   {b}→{a}: Δ≡{}", delta(a, b), delta(b, a));
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Layout {
    Td3,
    Td2,
    Td1,
}

/// One cell the composite check digit covers.
struct Cell {
    field: &'static str,
    /// Weight under the field's own check digit: `None` for a field without
    /// one and for the composite cell, `MINUS_ONE` for the own check-digit cell.
    own: Option<u32>,
    /// Weight under the composite; `MINUS_ONE` for the composite cell itself.
    composite: u32,
    alphabet: &'static str,
}

/// The composite's input string, cell by cell (ICAO 9303 Part 4 §4.2.2.2 for
/// TD3, Part 5 for TD1, Part 6 for TD2). The composite restarts its 7-3-1
/// cycle at its own first cell.
fn composite_layout(layout: Layout) -> Vec<Cell> {
    let fields: &[(&'static str, usize, &'static str, bool)] = match layout {
        Layout::Td3 => &[
            ("document number", 9, MRZ_ALPHABET, true),
            ("date of birth", 6, DIGITS, true),
            ("date of expiry", 6, DIGITS, true),
            ("personal number", 14, MRZ_ALPHABET, true),
        ],
        Layout::Td2 => &[
            ("document number", 9, MRZ_ALPHABET, true),
            ("date of birth", 6, DIGITS, true),
            ("date of expiry", 6, DIGITS, true),
            ("optional data", 7, MRZ_ALPHABET, false),
        ],
        Layout::Td1 => &[
            ("document number", 9, MRZ_ALPHABET, true),
            ("optional data 1", 15, MRZ_ALPHABET, false),
            ("date of birth", 6, DIGITS, true),
            ("date of expiry", 6, DIGITS, true),
            ("optional data 2", 11, MRZ_ALPHABET, false),
        ],
    };
    let mut cells: Vec<Cell> = Vec::new();
    for &(field, n, alphabet, own_digit) in fields {
        for i in 0..n {
            cells.push(Cell {
                field,
                own: own_digit.then_some(WEIGHTS[i % 3]),
                composite: WEIGHTS[cells.len() % 3],
                alphabet,
            });
        }
        if own_digit {
            cells.push(Cell {
                field,
                own: Some(MINUS_ONE),
                composite: WEIGHTS[cells.len() % 3],
                alphabet: DIGITS,
            });
        }
    }
    cells.push(Cell {
        field: "composite",
        own: None,
        composite: MINUS_ONE,
        alphabet: DIGITS,
    });
    cells
}

/// Exact pass counts for `k` misreads at uniformly chosen check-covered cells
/// of `layout` (only `field`'s cells when given), uniform Δ model:
/// `(every own digit passes, every own digit and the composite pass, denominator)`.
fn joint_pass(layout: Layout, k: usize, field: Option<&str>) -> (u128, u128, u128) {
    let cells: Vec<Cell> = composite_layout(layout)
        .into_iter()
        .filter(|c| field.is_none_or(|f| c.field == f))
        .collect();
    let alnum = Model::Uniform.dist(MRZ_ALPHABET);
    let digits = Model::Uniform.dist(DIGITS);
    let scale = lcm(alnum.total, digits.total);
    let (alnum, digits) = (alnum.scaled_to(scale), digits.scaled_to(scale));

    let mut groups: Vec<(&str, Vec<&Cell>)> = Vec::new();
    for cell in &cells {
        match groups.iter_mut().find(|(f, _)| *f == cell.field) {
            Some((_, group)) => group.push(cell),
            None => groups.push((cell.field, vec![cell])),
        }
    }

    // total[j][r]: weighted count of j-cell patterns over the fields combined so
    // far in which every own digit agrees and the composite sum moved by r.
    // Own digits are per field and fields are disjoint, so fields factor.
    let mut total = vec![[0u128; 10]; k + 1];
    total[0][0] = 1;
    for (name, group) in &groups {
        let has_own = *name != "composite" && group.iter().any(|c| c.own.is_some());
        // local[j][own residue][composite residue]
        let mut local = vec![[[0u128; 10]; 10]; k + 1];
        local[0][0][0] = 1;
        for cell in group {
            let shifts = if cell.alphabet == DIGITS {
                &digits
            } else {
                &alnum
            };
            for j in (1..=k).rev() {
                for o in 0..10 {
                    for c in 0..10 {
                        let base = local[j - 1][o][c];
                        if base == 0 {
                            continue;
                        }
                        for (d, &count) in shifts.iter().enumerate() {
                            if count == 0 {
                                continue;
                            }
                            let own = match (has_own, cell.own) {
                                (true, Some(w)) => (o + w as usize * d) % 10,
                                _ => 0,
                            };
                            let comp = (c + cell.composite as usize * d) % 10;
                            local[j][own][comp] += base * count;
                        }
                    }
                }
            }
        }
        let mut next = vec![[0u128; 10]; k + 1];
        for j1 in 0..=k {
            for r1 in 0..10 {
                let a = total[j1][r1];
                if a == 0 {
                    continue;
                }
                for j2 in 0..=(k - j1) {
                    for r2 in 0..10 {
                        let b = local[j2][0][r2];
                        if b != 0 {
                            next[j1 + j2][(r1 + r2) % 10] += a * b;
                        }
                    }
                }
            }
        }
        total = next;
    }
    (
        total[k].iter().sum(),
        total[k][0],
        choose(cells.len(), k) * scale.pow(k as u32),
    )
}

fn section_c() {
    println!("\n{}", "=".repeat(78));
    println!("C. The composite check digit");
    println!("{}", "=".repeat(78));
    for layout in [Layout::Td3, Layout::Td2, Layout::Td1] {
        println!("\n{layout:?}: composite weights equal the field's own weights?");
        let cells = composite_layout(layout);
        let mut seen: Vec<&str> = Vec::new();
        for cell in &cells {
            if cell.own.is_none() || seen.contains(&cell.field) {
                continue;
            }
            seen.push(cell.field);
            let start = cells
                .iter()
                .position(|c| c.field == cell.field)
                .unwrap_or(0);
            let aligned = cells
                .iter()
                .filter(|c| c.field == cell.field && c.own != Some(MINUS_ONE))
                .all(|c| c.own == Some(c.composite));
            println!(
                "  {:16} composite index {start:>2}: {}",
                cell.field,
                if aligned {
                    "ALIGNED, the same equation twice for errors in its data"
                } else {
                    "misaligned, an independent second equation"
                }
            );
        }
        let composite_only: Vec<&str> = cells
            .iter()
            .filter(|c| c.own.is_none() && c.field != "composite")
            .map(|c| c.field)
            .fold(Vec::new(), |mut acc, f| {
                if !acc.contains(&f) {
                    acc.push(f);
                }
                acc
            });
        if !composite_only.is_empty() {
            println!(
                "  composite only (no own digit): {}",
                composite_only.join(", ")
            );
        }
    }
    println!(
        "\nEvery weight is odd, so every check sum ≡ Σ Δᵢ (mod 2) wherever the cells sit: a\n\
         second digit over the same cells can add information only mod 5, and the floor for\n\
         two digits on one field is 1/2 · 1/25 = 2%, not 1%."
    );

    for layout in [Layout::Td3, Layout::Td2, Layout::Td1] {
        println!("\n{layout:?}, uniform model, k misreads anywhere in the covered cells:");
        println!(
            "  {:22}{}",
            "",
            (1..=6)
                .map(|k| format!("{:>10}", format!("k={k}")))
                .collect::<String>()
        );
        let rows: Vec<(u128, u128, u128)> = (1..=6).map(|k| joint_pass(layout, k, None)).collect();
        println!(
            "  {:22}{}",
            "own digits only",
            rows.iter()
                .map(|&(own, _, den)| format!("{:>10}", pct(own, den, 2)))
                .collect::<String>()
        );
        println!(
            "  {:22}{}",
            "own + composite",
            rows.iter()
                .map(|&(_, both, den)| format!("{:>10}", pct(both, den, 2)))
                .collect::<String>()
        );
    }

    println!("\nMisreads confined to one field (its data + own check-digit cell), uniform model, k=1..4:");
    for (layout, field) in [
        (Layout::Td3, "document number"),
        (Layout::Td3, "date of birth"),
        (Layout::Td3, "date of expiry"),
        (Layout::Td3, "personal number"),
        (Layout::Td1, "document number"),
        (Layout::Td1, "date of birth"),
        (Layout::Td1, "date of expiry"),
        (Layout::Td2, "optional data"),
        (Layout::Td1, "optional data 1"),
        (Layout::Td1, "optional data 2"),
    ] {
        let rows: Vec<(u128, u128, u128)> = (1..=4)
            .map(|k| joint_pass(layout, k, Some(field)))
            .collect();
        println!(
            "  {:?} {:16} own:{}   own+comp:{}",
            layout,
            field,
            rows.iter()
                .map(|&(own, _, den)| format!("{:>9}", pct(own, den, 2)))
                .collect::<String>(),
            rows.iter()
                .map(|&(_, both, den)| format!("{:>9}", pct(both, den, 2)))
                .collect::<String>()
        );
    }

    let td3 = composite_layout(Layout::Td3);
    let pair_counts = |field: &str| {
        let data: Vec<&Cell> = td3
            .iter()
            .filter(|c| c.field == field && c.own != Some(MINUS_ONE))
            .collect();
        let pairs = combinations(data.len(), 2);
        let passes = |weight: fn(&Cell) -> u32, s: &Vec<usize>| {
            4 * s.iter().map(|&i| weight(data[i]) as usize).sum::<usize>() % 10 == 0
        };
        let own: Vec<&Vec<usize>> = pairs
            .iter()
            .filter(|s| passes(|c| c.own.unwrap_or(0), s))
            .collect();
        let both = own.iter().filter(|s| passes(|c| c.composite, s)).count();
        (own.len(), both, pairs.len())
    };
    println!("\nTwo misreads with Δ≡4 (two 0→O) at two data cells of a TD3 field:");
    for field in ["date of birth", "document number"] {
        let (own, both, all) = pair_counts(field);
        println!("  {field:16} own digit passes {own}/{all}, own and composite pass {both}/{all}");
    }
}

fn section_d() {
    println!("\n{}", "=".repeat(78));
    println!("D. Line-1 codes: substitutions that land on another listed code");
    println!("{}", "=".repeat(78));
    let listed: BTreeSet<String> = codes()
        .iter()
        .map(|(code, _)| {
            let mut padded = code.to_string();
            while padded.len() < 3 {
                padded.push('<');
            }
            padded
        })
        .collect();
    let cells = 27u128.pow(3);
    println!(
        "codes listed: {} of {cells} possible 3-cell strings ({} density)",
        listed.len(),
        pct(listed.len() as u128, cells, 2)
    );
    let swap = |code: &str, at: &[(usize, char)]| {
        let mut chars: Vec<char> = code.chars().collect();
        for &(i, x) in at {
            chars[i] = x;
        }
        listed.contains(&chars.into_iter().collect::<String>())
    };
    let (mut single, mut single_hits) = (0u128, 0u128);
    let (mut double, mut double_hits) = (0u128, 0u128);
    let (mut confusable, mut confusable_hits) = (0u128, 0u128);
    let rows = confusable_rows();
    for code in &listed {
        let chars: Vec<char> = code.chars().collect();
        for (i, &listed_char) in chars.iter().enumerate() {
            for x in CODE_CELL.chars().filter(|&x| x != listed_char) {
                single += 1;
                single_hits += u128::from(swap(code, &[(i, x)]));
            }
            for &x in rows.get(&listed_char).into_iter().flatten() {
                if CODE_CELL.contains(x) {
                    confusable += 1;
                    confusable_hits += u128::from(swap(code, &[(i, x)]));
                }
            }
        }
        for pair in combinations(3, 2) {
            let (i, j) = (pair[0], pair[1]);
            for x in CODE_CELL.chars().filter(|&x| x != chars[i]) {
                for y in CODE_CELL.chars().filter(|&y| y != chars[j]) {
                    double += 1;
                    double_hits += u128::from(swap(code, &[(i, x), (j, y)]));
                }
            }
        }
    }
    println!(
        "one substitution lands on another listed code: {single_hits}/{single} = {}",
        pct(single_hits, single, 2)
    );
    println!(
        "two substitutions: {double_hits}/{double} = {}",
        pct(double_hits, double, 2)
    );
    println!(
        "one CONFUSABLES letter substitution: {confusable_hits}/{confusable} = {}",
        pct(confusable_hits, confusable, 2)
    );
}

fn section_e() {
    println!("\n{}", "=".repeat(78));
    println!(
        "E. Exhaustive-enumeration budget: patterns with ≤ k misreads, ≤ 4 alternatives per cell"
    );
    println!("{}", "=".repeat(78));
    let rows = confusable_rows();
    let entries: usize = rows.values().map(BTreeSet::len).sum();
    let widest = rows.values().map(BTreeSet::len).max().unwrap_or(0);
    println!(
        "CONFUSABLES: {} glyphs have a row, widest row {widest}, mean {entries}/{} = {:.2}; {} glyphs have none.",
        rows.len(),
        rows.len(),
        entries as f64 / rows.len() as f64,
        MRZ_ALPHABET.len() - rows.len()
    );
    for (label, n) in [
        ("document number + check digit (10)", 10),
        ("date + check digit (7)", 7),
        ("personal number + check digit (15)", 15),
        ("TD3 line 2 (44)", 44),
        ("TD1 zone (90)", 90),
    ] {
        let budgets: Vec<String> = (1..=4)
            .map(|k| {
                let total: u128 = (1..=k).map(|j| choose(n, j) * 4u128.pow(j as u32)).sum();
                format!("k≤{k}: {total}")
            })
            .collect();
        println!("  {label:36} {}", budgets.join("; "));
    }
}

fn section_f() {
    println!("\n{}", "=".repeat(78));
    println!("F. Consecutive clean reads needed for a lower bound at 95% confidence");
    println!("{}", "=".repeat(78));
    println!("  n = ⌈ln α / ln p⌉: α = 0.05 one-sided, α = 0.025 for a two-sided 95% Clopper–Pearson interval");
    for p in [0.99f64, 0.995, 0.999] {
        let one_sided = (0.05f64.ln() / p.ln()).ceil();
        let two_sided = (0.025f64.ln() / p.ln()).ceil();
        let rule_of_three = (3.0 / (1.0 - p)).round();
        println!(
            "  lower bound ≥ {:.1}%: one-sided n = {one_sided}, two-sided n = {two_sided} (rule of three: {rule_of_three})",
            p * 100.0
        );
    }
}

fn main() {
    assert_eq!(
        OBSERVED.iter().map(|e| e.2).sum::<u128>(),
        126,
        "OBSERVED must hold the 126 events of the 2026-09-21 note"
    );
    section_a();
    section_b();
    section_c();
    section_d();
    section_e();
    section_f();
}
