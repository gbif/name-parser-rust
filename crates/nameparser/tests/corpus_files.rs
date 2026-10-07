// SPDX-License-Identifier: Apache-2.0
//! The Java suite's resource-file tests (`NameParserImplTest`'s `virusesFile`, `hybridsFile`,
//! `placeholderFile`, `otherFile`, `namesWithAuthorFile`, plus the OTU list): every line of a
//! `testdata/*.txt` file must parse the way the file is named for. The golden snapshot only pins
//! the benchmark corpus, so these lists were otherwise unchecked.

use nameparser::model::{NameType, NomCode};
use nameparser::ParseResult;

/// Lines that are known not to parse as their file says, each with the reason. Every entry must
/// still fail — a fixed one is reported, so it can be removed.
const KNOWN: &[(&str, &str)] = &[(
    "Thermus thermophilus phagein in93",
    // FIXME(review): a regression of the anchored `phages?` virus gate — "phagein" no longer
    // marks a virus, so this comes back as an infrasubspecific name with the epithet "in93"
    "phagein is a virus marker",
)];

fn lines(file: &str) -> Option<Vec<String>> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata");
    if !std::path::Path::new(dir).is_dir() {
        return None; // a packaged crate, without the repo's testdata
    }
    let text = std::fs::read_to_string(format!("{dir}/{file}"))
        .unwrap_or_else(|e| panic!("testdata/{file}: {e}"));
    Some(
        text.lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .map(str::to_string)
            .collect(),
    )
}

/// Asserts `expected` for every line of `file`, except the [`KNOWN`] ones, which must still fail.
fn check(file: &str, expected: impl Fn(&ParseResult) -> bool) {
    let Some(names) = lines(file) else { return };
    let mut problems = Vec::new();
    for name in &names {
        let ok = expected(&nameparser::parse(name, None, None, None));
        let known = KNOWN.iter().any(|(k, _)| k == name);
        match (ok, known) {
            (false, false) => problems.push(format!(
                "{name:?} parses as {:?}",
                nameparser::parse(name, None, None, None)
            )),
            (true, true) => problems.push(format!("{name:?} is fixed — remove it from KNOWN")),
            _ => {}
        }
    }
    assert!(
        problems.is_empty(),
        "testdata/{file}:\n  {}",
        problems.join("\n  ")
    );
}

fn unparsable(result: &ParseResult, type_: NameType) -> bool {
    matches!(result, ParseResult::Unparsable(e) if e.type_ == type_)
}

#[test]
fn viruses_have_the_virus_code() {
    check("viruses.txt", |r| r.code() == Some(NomCode::Virus));
}

#[test]
fn hybrid_formulas_are_unparsable_formulas() {
    check("hybrids.txt", |r| unparsable(r, NameType::Formula));
}

#[test]
fn placeholders_are_unparsable_placeholders() {
    check("placeholder.txt", |r| unparsable(r, NameType::Placeholder));
}

#[test]
fn otu_codes_are_identifiers() {
    check("otu.txt", |r| unparsable(r, NameType::Identifier));
}

#[test]
fn other_strings_are_unparsable() {
    check("other.txt", |r| unparsable(r, NameType::Other));
}

#[test]
fn names_with_authors_keep_their_authors() {
    check("names-with-authors.txt", |r| {
        r.parsed().is_some_and(|pn| {
            pn.combination_authorship.has_authors() || pn.basionym_authorship.has_authors()
        })
    });
}

/// Benchmark names `parse` returns in breach of its three-way contract, each still expected to.
// FIXME(review): a missing-genus placeholder ("? epithet", "Missing epithet") comes back `Parsed`
// with type PLACEHOLDER, which `NameType::is_parsable` says only an `Unparsable` may carry — either
// it becomes `Unparsable(PLACEHOLDER)` or the contract admits it; an API decision.
const CONTRACT_EXCEPTIONS: &[&str] = &[
    "denheyeri Eghbalian, Khanjani and Ueckermann in Eghbalian, Khanjani & Ueckermann, 2017",
    "\"? gryphoidis",
    "\"? gryphoidis (Bourguignat 1870) Schoepf. 1909",
    "Missing penchinati Bourguignat, 1870",
    "ex DC.",
];

/// Over the benchmark corpus, `parse` keeps its three-way contract: a `Parsed` name has a parsable
/// type, an `Informal` one a taxon, an `Unparsable` one a non-parsable type.
#[test]
fn parse_keeps_its_three_way_contract_over_the_benchmark_corpus() {
    let Some(names) = lines("benchmark-data.txt") else {
        return;
    };
    let mut problems = Vec::new();
    for name in &names {
        let breach = match nameparser::parse(name, None, None, None) {
            ParseResult::Parsed(pn) => {
                (!pn.type_.is_parsable()).then(|| format!("Parsed with type {:?}", pn.type_))
            }
            ParseResult::Informal(inf) => inf
                .taxon
                .is_empty()
                .then(|| "Informal without a taxon".into()),
            ParseResult::Unparsable(e) => e
                .type_
                .is_parsable()
                .then(|| format!("Unparsable with type {:?}", e.type_)),
        };
        let known = CONTRACT_EXCEPTIONS.contains(&name.as_str());
        match (breach, known) {
            (Some(b), false) => problems.push(format!("{name:?}: {b}")),
            (None, true) => problems.push(format!("{name:?} keeps the contract now — remove it")),
            _ => {}
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
