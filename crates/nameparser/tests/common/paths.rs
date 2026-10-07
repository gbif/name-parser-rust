// SPDX-License-Identifier: Apache-2.0
//! Every expectation on both authorship paths.
//!
//! A name reaches the parser in three shapes: with its authorship inside the name string
//! (`Abies alba Mill.`), with the authorship passed separately (`Abies alba` + `Mill.`, how
//! ChecklistBank parses almost every record), or with the authorship in both (`Abies alba Mill.` +
//! `Mill.`, sources that repeat it in their authorship column). All three must parse alike.
//!
//! [`check_paths`] runs inside every classifying DSL entry point, before the test's own chain: it
//! derives the other shapes of the call and asserts their raw `ParsedName` equals the call's own,
//! field by field (on the JSON wire shape). The chain then only needs to pin one of them.
//!
//! * A full name string is split where the authorship starts: at the first whitespace or `(`
//!   whose prefix alone parses to the same name parts ([`find_split`]). It is checked as
//!   **separate** (prefix + the rest as authorship) and **redundant** (the whole string + the rest).
//!   A name with an authorship but no such split (an autonym's species author, a cultivar's, a
//!   mid-name `†`) is a `nosplit`.
//! * A name with a separate authorship is checked **embedded** (`"{name} {authorship}"`) and
//!   **redundant** (that string + the authorship). If the name already carries that authorship
//!   (sources repeat it in both columns), the embedded shape is the name alone.
//! * A bare authorship (`assert_authorship`, parsed as `Abies alba` + it) is checked as
//!   **auth-embedded** and **auth-redundant** the same way.
//!
//! Known divergences are listed in `path_divergences.tsv` with the fields they differ on. A listed
//! divergence is skipped; one that changed, disappeared or is new fails — the list can only shrink
//! deliberately. `NAMEPARSER_DUAL_PATH_REPORT=<file>` appends the would-be list lines to `<file>`
//! instead of failing, to (re)generate the list.

use std::io::Write;
use std::sync::LazyLock;

use nameparser::model::{NomCode, ParseError, ParsedName, Rank};

use super::parsed_name_diff;

/// The name a bare authorship is attached to, as Java's `parseAuthorship` does.
const AUTHORSHIP_HOST: &str = "Abies alba";

/// One call of a DSL entry point: the parser's four arguments.
#[derive(Clone, Copy, Debug)]
pub struct Call<'a> {
    pub input: &'a str,
    pub authorship: Option<&'a str>,
    pub rank: Option<Rank>,
    pub code: Option<NomCode>,
}

impl<'a> Call<'a> {
    /// "`input` + authorship `…` (rank …, code …)", for failure messages.
    pub fn label(&self) -> String {
        let mut s = format!("`{}`", self.input);
        if let Some(a) = self.authorship {
            s.push_str(&format!(" + authorship `{a}`"));
        }
        if let Some(r) = self.rank {
            s.push_str(&format!(" (rank {r:?})"));
        }
        if let Some(c) = self.code {
            s.push_str(&format!(" (code {c:?})"));
        }
        s
    }

    pub fn name(input: &'a str) -> Self {
        Call {
            input,
            authorship: None,
            rank: None,
            code: None,
        }
    }
}

/// What the call's input is: a name (with or without a separate authorship), or a bare authorship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Name,
    Authorship,
}

/// One derived shape of a call that does not parse like the call itself.
#[derive(Debug)]
pub struct Divergence {
    /// `separate`, `redundant`, `embedded`, `nosplit`, `auth-embedded` or `auth-redundant`.
    pub variant: &'static str,
    /// The derived parser arguments, for the failure message.
    pub derived: String,
    /// The differing fields, comma-separated (`classification` for parsed vs unparsable, `error`
    /// for two different errors, `-` for a nosplit) — the list's `fields` column.
    pub fields: String,
    /// One `field: own | derived` line per difference.
    pub details: Vec<String>,
}

type Raw = Result<ParsedName, ParseError>;

fn raw(name: &str, authorship: Option<&str>, rank: Option<Rank>, code: Option<NomCode>) -> Raw {
    nameparser::parse_name(name, authorship, rank, code)
}

/// True if the parse carries anything a source could have put in its authorship column instead:
/// authors, a year, ex-authors, a sanctioning author, a note, a reference or a manuscript flag.
fn carries_authorship(p: &ParsedName) -> bool {
    let a = |x: &nameparser::model::Authorship| x.exists() || !x.ex_authors.is_empty();
    a(&p.combination_authorship)
        || a(&p.basionym_authorship)
        || p.sanctioning_author.is_some()
        || p.taxonomic_note.is_some()
        || p.nomenclatural_note.is_some()
        || p.published_in.is_some()
        || p.manuscript
}

/// `s` reduced to its letters and digits, to compare authorships regardless of punctuation.
fn letters(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect()
}

/// Where the authorship starts in a full name string: the first whitespace or `(` whose trimmed
/// prefix alone parses to the same name parts and rank, with no authorship of its own (else the cut
/// fell inside the authorship: `Lampona spec Platnick,` + `2000`). The code the authorship implied is tried
/// as a hint first (a zoological trinomial is a subspecies only under that code), then without it
/// (a code hint, unlike an inferred code, also ranks a `-idae` uninomial a family). `None` if no
/// prefix does.
pub fn find_split(
    input: &str,
    rank: Option<Rank>,
    code: Option<NomCode>,
    own: &ParsedName,
) -> Option<(String, String)> {
    let key = |p: &ParsedName| {
        (
            p.uninomial.clone(),
            p.genus.clone(),
            p.infrageneric_epithet.clone(),
            p.specific_epithet.clone(),
            p.infraspecific_epithet.clone(),
            p.cultivar_epithet.clone(),
            p.notho.clone(),
            p.rank,
            p.phrase.clone(),
        )
    };
    let want = key(own);
    for (i, c) in input.char_indices().skip(1) {
        if !(c.is_whitespace() || c == '(') {
            continue;
        }
        let (prefix, rest) = (input[..i].trim(), input[i..].trim());
        if prefix.is_empty() || rest.is_empty() {
            continue;
        }
        let hints = [code.or(own.code), code];
        let hints = if hints[0] == hints[1] {
            &hints[..1]
        } else {
            &hints[..]
        };
        if hints.iter().any(|&c| {
            raw(prefix, None, rank, c).is_ok_and(|p| key(&p) == want && !carries_authorship(&p))
        }) {
            return Some((prefix.to_string(), rest.to_string()));
        }
    }
    None
}

/// How `derived` differs from `own`, or `None` if it parses alike.
fn compare(own: &Raw, derived: &Raw) -> Option<(String, Vec<String>)> {
    match (own, derived) {
        (Ok(a), Ok(b)) => {
            let diff = parsed_name_diff(a, b);
            (!diff.is_empty()).then(|| {
                let fields = diff
                    .iter()
                    .map(|d| d.0.as_str())
                    .collect::<Vec<_>>()
                    .join(",");
                let details = diff
                    .iter()
                    .map(|(k, a, b)| format!("{k}: {a} | {b}"))
                    .collect();
                (fields, details)
            })
        }
        (Err(a), Err(b)) => (a.type_ != b.type_ || a.code != b.code).then(|| {
            let detail = format!("{:?}/{:?} | {:?}/{:?}", a.type_, a.code, b.type_, b.code);
            ("error".to_string(), vec![detail])
        }),
        (Ok(_), Err(e)) => Some((
            "classification".to_string(),
            vec![format!("parsed | unparsable {:?}", e.type_)],
        )),
        (Err(e), Ok(_)) => Some((
            "classification".to_string(),
            vec![format!("unparsable {:?} | parsed", e.type_)],
        )),
    }
}

/// Every derived shape of `call` that does not parse like `call` itself.
pub fn path_divergences(call: &Call, shape: Shape) -> Vec<Divergence> {
    let Call {
        input,
        authorship,
        rank,
        code,
    } = *call;
    let describe = |n: &str, a: Option<&str>| match a {
        Some(a) => format!("`{n}` + authorship `{a}`"),
        None => format!("`{n}`"),
    };
    let mut derived: Vec<(&'static str, String, Raw)> = Vec::new();
    let own = match shape {
        Shape::Authorship => {
            let own = raw(AUTHORSHIP_HOST, Some(input), Some(Rank::Species), None);
            let full = format!("{AUTHORSHIP_HOST} {input}");
            for (variant, a) in [("auth-embedded", None), ("auth-redundant", Some(input))] {
                derived.push((
                    variant,
                    describe(&full, a),
                    raw(&full, a, Some(Rank::Species), None),
                ));
            }
            own
        }
        Shape::Name => match authorship.filter(|a| !a.trim().is_empty()) {
            None => {
                let own = raw(input, None, rank, code);
                let Ok(p) = &own else { return vec![] };
                if !carries_authorship(p) {
                    return vec![];
                }
                let Some((prefix, rest)) = find_split(input, rank, code, p) else {
                    return vec![Divergence {
                        variant: "nosplit",
                        derived: describe(input, None),
                        fields: "-".to_string(),
                        details: vec!["no prefix parses to the same name parts".to_string()],
                    }];
                };
                derived.push((
                    "separate",
                    describe(&prefix, Some(&rest)),
                    raw(&prefix, Some(&rest), rank, code),
                ));
                derived.push((
                    "redundant",
                    describe(input, Some(&rest)),
                    raw(input, Some(&rest), rank, code),
                ));
                own
            }
            Some(a) => {
                let own = raw(input, Some(a), rank, code);
                let alone = raw(input, None, rank, code);
                // the name already carries this authorship (sources repeat it, not always
                // identically) — as authors, or swallowed into the phrase of a provisional name
                // (`Cantuaria sp. Forster, 1968`); a different one, like a cultivar's species
                // author before the cultivar author, does not count
                let repeated = letters(input).contains(&letters(a))
                    && alone
                        .as_ref()
                        .is_ok_and(|p| carries_authorship(p) || p.phrase.is_some());
                if repeated {
                    derived.push(("embedded", describe(input, None), alone));
                } else {
                    let full = format!("{input} {a}");
                    derived.push((
                        "embedded",
                        describe(&full, None),
                        raw(&full, None, rank, code),
                    ));
                    derived.push((
                        "redundant",
                        describe(&full, Some(a)),
                        raw(&full, Some(a), rank, code),
                    ));
                }
                own
            }
        },
    };
    derived
        .into_iter()
        .filter_map(|(variant, derived, result)| {
            compare(&own, &result).map(|(fields, details)| Divergence {
                variant,
                derived,
                fields,
                details,
            })
        })
        .collect()
}

// ---- the list of known divergences ------------------------------------------------------------

/// One row of `path_divergences.tsv`.
#[derive(Debug)]
pub struct Known {
    pub line: usize,
    pub variant: String,
    pub input: String,
    pub authorship: Option<String>,
    pub rank: Option<Rank>,
    pub code: Option<NomCode>,
    pub fields: String,
}

impl Known {
    pub fn call(&self) -> Call<'_> {
        Call {
            input: &self.input,
            authorship: self.authorship.as_deref(),
            rank: self.rank,
            code: self.code,
        }
    }

    pub fn shape(&self) -> Shape {
        if self.variant.starts_with("auth-") {
            Shape::Authorship
        } else {
            Shape::Name
        }
    }

    fn is_for(&self, call: &Call, shape: Shape) -> bool {
        self.shape() == shape
            && self.input == call.input
            && self.authorship.as_deref() == call.authorship
            && self.rank == call.rank
            && self.code == call.code
    }
}

pub const KNOWN_FILE: &str = "crates/nameparser/tests/common/path_divergences.tsv";

/// The rows of `path_divergences.tsv`: tab-separated `variant`, `input`, `authorship`, `rank`,
/// `code`, `fields`, then an optional free-text note. Empty cells are absent; `#` starts a comment.
pub static KNOWN: LazyLock<Vec<Known>> = LazyLock::new(|| {
    include_str!("path_divergences.tsv")
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|(i, l)| {
            let cols: Vec<&str> = l.split('\t').collect();
            assert!(
                cols.len() >= 6,
                "{KNOWN_FILE}:{}: expected 6+ columns",
                i + 1
            );
            let opt = |s: &str| (!s.is_empty()).then(|| s.to_string());
            Known {
                line: i + 1,
                variant: cols[0].to_string(),
                input: cols[1].to_string(),
                authorship: opt(cols[2]),
                rank: opt(cols[3])
                    .map(|r| Rank::from_name(&r).unwrap_or_else(|| panic!("bad rank {r}"))),
                code: opt(cols[4])
                    .map(|c| NomCode::from_name(&c).unwrap_or_else(|| panic!("bad code {c}"))),
                fields: cols[5].to_string(),
            }
        })
        .collect()
});

/// The list line for a divergence of `call`.
pub fn tsv_line(call: &Call, d: &Divergence) -> String {
    let wire = |v: serde_json::Value| v.as_str().unwrap_or_default().to_string();
    let rank = call.rank.map(|r| wire(serde_json::to_value(r).unwrap()));
    let code = call.code.map(|c| wire(serde_json::to_value(c).unwrap()));
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        d.variant,
        call.input,
        call.authorship.unwrap_or_default(),
        rank.unwrap_or_default(),
        code.unwrap_or_default(),
        d.fields
    )
}

/// Assert every derived shape of `call` parses like `call` itself, except the divergences listed
/// in `path_divergences.tsv` with exactly these fields. See the module doc.
pub fn check_paths(call: Call, shape: Shape) {
    let divergences = path_divergences(&call, shape);
    let listed: Vec<&Known> = KNOWN.iter().filter(|k| k.is_for(&call, shape)).collect();
    let mut problems: Vec<String> = Vec::new();
    let mut report: Vec<String> = Vec::new();
    for d in &divergences {
        let line = tsv_line(&call, d);
        let how = format!(
            "{} {}:\n      {}\n    list line: {line}",
            d.variant,
            d.derived,
            d.details.join("\n      ")
        );
        match listed.iter().find(|k| k.variant == d.variant) {
            Some(k) if k.fields == d.fields => {}
            Some(k) => {
                problems.push(format!(
                    "{KNOWN_FILE}:{} lists `{}`, now {how}",
                    k.line, k.fields
                ));
                report.push(line);
            }
            None => {
                problems.push(format!("new divergence, {how}"));
                report.push(line);
            }
        }
    }
    for k in &listed {
        if !divergences.iter().any(|d| d.variant == k.variant) {
            problems.push(format!(
                "{KNOWN_FILE}:{}: {} now parses alike — remove the line",
                k.line, k.variant
            ));
            report.push(format!("STALE\t{}", k.line));
        }
    }
    if problems.is_empty() {
        return;
    }
    if let Ok(path) = std::env::var("NAMEPARSER_DUAL_PATH_REPORT") {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .unwrap_or_else(|e| panic!("cannot open {path}: {e}"));
        f.write_all((report.join("\n") + "\n").as_bytes())
            .unwrap_or_else(|e| panic!("cannot write {path}: {e}"));
        return;
    }
    panic!(
        "`{}`{} does not parse alike on every authorship path:\n  {}",
        call.input,
        call.authorship
            .map(|a| format!(" + authorship `{a}`"))
            .unwrap_or_default(),
        problems.join("\n  ")
    );
}
