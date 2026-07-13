// SPDX-License-Identifier: Apache-2.0
//! Readable per-name test DSL — the Rust port of Java `NameAssertion` +
//! `NameParserImplTest`'s `assertName`/`assertUnparsable` helpers, so the Java test suites
//! translate 1:1 into native Rust tests. Shared across the `tests/*.rs` files via `mod common;`.
//!
//! ## How to use
//! ```ignore
//! use common::*;
//! assert_name("Ameiva plei (sic) Duméril & Bibron, 1839")
//!     .species("Ameiva", "plei")
//!     .comb_authors(Some("1839"), &["Duméril", "Bibron"])
//!     .sic()
//!     .code(NomCode::Zoological)
//!     .nothing_else();
//! ```
//! `nothing_else()` asserts every field you did NOT mention is at its default — so a test pins
//! the WHOLE parse, not just the parts you named.
//!
//! ## 5.0.0 three-way DSL (the authoritative spec — the golden is only a Rust regression snapshot)
//! The three classifying entry points go through the exceptionless [`nameparser::parse`] and
//! assert the actual 5.0.0 [`nameparser::ParseResult`] VARIANT, so a misclassification fails loudly:
//! * [`assert_name`] (+ `_hinted`/`_rank`/`_code`/`_auth`) — asserts a [`ParseResult::Parsed`] name.
//! * [`assert_informal`] (+ `_hinted`) — asserts a [`ParseResult::Informal`] (a supraspecific anchor
//!   + a provisional designation, no species epithet) via the fluent [`InformalAssertion`].
//! * [`assert_unparsable`] (+ `_code`/`_rank`/`_name`) — asserts a [`ParseResult::Unparsable`] with
//!   the CLAMPED type (an `Unparsable` may only carry a non-parsable `NameType`).
//!
//! The remaining helpers ([`assert_phrase_name`], [`assert_authorship`], [`assert_sensu`],
//! [`assert_nom_note`], [`is_viral_name`], …) deliberately stay on the lower-level raw
//! [`nameparser::parse_name`] path — they test the `ParsedName` atoms / `NameFormatter` renderings /
//! authorship directly, which is a distinct (still public) API from the three-way classification.
//!
//! ## Java → Rust mapping (for porting the suites)
//! * `assertName(name, canonical)` → `assert_name(name)` — the `expectedCanonicalWithoutAuthors`
//!   arg is dropped (Rust has no `canonicalName()`; the field assertions + `nothing_else()` pin
//!   the parse). `assertName(name, auth, canonical)` → `assert_name_auth(name, auth)`;
//!   `assertName(name, RANK, canonical)` → `assert_name_rank(name, RANK)`;
//!   `assertName(name, CODE, canonical)` → `assert_name_code(name, CODE)`; the fuller variants →
//!   `assert_name_hinted(name, auth, rank, code)`.
//! * `assertUnparsable(name, TYPE)` → `assert_unparsable(name, NameType::TYPE)`;
//!   `assertUnparsable(name, TYPE, CODE)` → `assert_unparsable_code(...)`;
//!   `assertNoName(name)` → `assert_no_name(name)`.
//! * **`NameType` is reduced to 5 variants** in the 4.2.0 api (`Scientific`, `Formula`,
//!   `Informal`, `Placeholder`, `Other`). Java `HYBRID_FORMULA` → `Formula`; `VIRUS`/`OTU`/
//!   `NO_NAME`/`BLACKLISTED`/`DOUBTFUL`/`OTHER` → `Other` (the `NomCode`/`state`/`doubtful`
//!   fields carry the finer distinction). When a ported assertion's type is ambiguous, check the
//!   name's actual output in `testdata/expected-parse.jsonl` (the golden the whole corpus is
//!   validated against).
//! * Overloaded builder methods are disambiguated by suffix: `species(g,ig,e)` → `species_ig`,
//!   `infraGeneric(g,rank,ig)` → `infrageneric_at`, the `cultivar(...)` overloads → `cultivar` /
//!   `cultivar_rank` / `cultivar_sp` / `cultivar_sp_rank`. Java varargs → a `&[&str]` slice;
//!   Java `null` year → `None`, `"1955"` → `Some("1955")`.

#![allow(dead_code)] // the DSL surface is used across many test files; not every method in each

use std::collections::BTreeMap;

use nameparser::model::{Informal, NamePart, NameType, NomCode, ParsedName, Rank, State};
use nameparser::ParseResult;

// ---- entry points -----------------------------------------------------------------------------

/// Parse `input` (no hints) through the 5.0.0 [`nameparser::parse`] and assert the outcome is
/// a [`ParseResult::Parsed`] name, starting an assertion chain. Panics (loudly) if the name comes
/// back `Informal` or `Unparsable` — so a determined-name test that wrongly reclassifies as informal
/// fails here, not silently. (The informal band is [`assert_informal`]; the junk band is
/// [`assert_unparsable`].)
pub fn assert_name(input: &str) -> NameAssertion {
    assert_name_hinted(input, None, None, None)
}

/// `assertName(name, rawAuthorship, canonical)` — parse with a separately supplied authorship.
pub fn assert_name_auth(input: &str, authorship: &str) -> NameAssertion {
    assert_name_hinted(input, Some(authorship), None, None)
}

/// `assertName(name, rank, canonical)` — parse with a rank hint.
pub fn assert_name_rank(input: &str, rank: Rank) -> NameAssertion {
    assert_name_hinted(input, None, Some(rank), None)
}

/// `assertName(name, code, canonical)` — parse with a nomenclatural-code hint.
pub fn assert_name_code(input: &str, code: NomCode) -> NameAssertion {
    assert_name_hinted(input, None, None, Some(code))
}

/// The full `assertName(name, [authorship,] [rank,] [code], canonical)` variant.
pub fn assert_name_hinted(
    input: &str,
    authorship: Option<&str>,
    rank: Option<Rank>,
    code: Option<NomCode>,
) -> NameAssertion {
    match nameparser::parse(input, authorship, rank, code) {
        ParseResult::Parsed(pn) => NameAssertion::new(pn),
        ParseResult::Informal(inf) => {
            panic!("expected `{input}` to be a Parsed name, but it was Informal: {inf:?}")
        }
        ParseResult::Unparsable(e) => {
            panic!("expected `{input}` to parse, but it was unparsable: {e:?}")
        }
    }
}

// ---- 5.0.0 three-way entry points (parse, not the raw parse_name()) ---------------------------

/// Parse `input` through the 5.0.0 [`nameparser::parse`] and assert the outcome is an
/// [`ParseResult::Informal`], starting a fluent [`InformalAssertion`] chain. Panics (loudly) if the
/// name comes back `Parsed` or `Unparsable` instead. Use for the informal / semistructured band —
/// a supraspecific taxon carrying a provisional designation with no species epithet.
pub fn assert_informal(input: &str) -> InformalAssertion {
    assert_informal_hinted(input, None, None, None)
}

/// [`assert_informal`] with the optional authorship / rank / code hints.
pub fn assert_informal_hinted(
    input: &str,
    authorship: Option<&str>,
    rank: Option<Rank>,
    code: Option<NomCode>,
) -> InformalAssertion {
    match nameparser::parse(input, authorship, rank, code) {
        ParseResult::Informal(inf) => InformalAssertion::new(inf),
        ParseResult::Parsed(pn) => {
            panic!("expected `{input}` to be an Informal result, but it Parsed: {pn:?}")
        }
        ParseResult::Unparsable(e) => {
            panic!("expected `{input}` to be an Informal result, but it was Unparsable: {e:?}")
        }
    }
}

/// `assertNoName(name)` — the input must be unparsable. Java asserts `NameType.NO_NAME`, but the
/// 4.2.0 `NameType` this port targets has only 5 variants (see the mapping note in the module
/// doc), so a definitively-not-a-name input classifies as `Other` here.
pub fn assert_no_name(input: &str) {
    assert_unparsable(input, NameType::Other);
}

/// `assertUnparsable(name, type)` — the input must be a 5.0.0 [`ParseResult::Unparsable`] with the
/// given `NameType` (the type is the CLAMPED one — `Unparsable` may only carry a non-parsable type).
pub fn assert_unparsable(input: &str, type_: NameType) {
    match nameparser::parse(input, None, None, None) {
        ParseResult::Unparsable(e) => assert_eq!(
            e.type_, type_,
            "`{input}` unparsable as expected but with type {:?}, expected {type_:?}",
            e.type_
        ),
        other => panic!("expected `{input}` to be unparsable ({type_:?}), got {other:?}"),
    }
}

/// `assertUnparsable(name, type, code)` — [`ParseResult::Unparsable`] with the given `NameType` AND
/// `NomCode`.
pub fn assert_unparsable_code(input: &str, type_: NameType, code: NomCode) {
    match nameparser::parse(input, None, None, None) {
        ParseResult::Unparsable(e) => {
            assert_eq!(e.type_, type_, "`{input}`: type {:?} != {type_:?}", e.type_);
            assert_eq!(
                e.code,
                Some(code),
                "`{input}`: code {:?} != {code:?}",
                e.code
            );
        }
        other => panic!("expected `{input}` unparsable ({type_:?}/{code:?}), got {other:?}"),
    }
}

/// `assertUnparsable(name, rank, type)` — unparsable with a rank hint; the echoed error name
/// equals the input. Delegates to [`assert_unparsable_name`].
pub fn assert_unparsable_rank(input: &str, rank: Rank, type_: NameType) {
    assert_unparsable_name(input, rank, type_, input);
}

/// `assertUnparsableName(name, rank, type, expectedName)` — unparsable (parsed with the rank
/// hint) with the given `NameType`, and the echoed error name equals `expected_name`.
pub fn assert_unparsable_name(input: &str, rank: Rank, type_: NameType, expected_name: &str) {
    match nameparser::parse(input, None, Some(rank), None) {
        ParseResult::Unparsable(e) => {
            assert_eq!(e.type_, type_, "`{input}`: type {:?} != {type_:?}", e.type_);
            assert_eq!(
                e.name, expected_name,
                "`{input}`: name {:?} != {expected_name:?}",
                e.name
            );
        }
        other => panic!("expected `{input}` to be unparsable ({type_:?}), got {other:?}"),
    }
}

/// `assertSensu(raw, sensu)` — parse `raw` and assert its taxonomic (sensu/sec) note.
pub fn assert_sensu(raw: &str, sensu: &str) {
    let n = nameparser::parse_name(raw, None, None, None)
        .unwrap_or_else(|e| panic!("expected `{raw}` to parse: {e:?}"));
    assert_eq!(
        n.taxonomic_note.as_deref(),
        Some(sensu),
        "sensu mismatch for `{raw}`"
    );
}

/// `assertPhraseName(sciname, canonicalName, rank, phrase)` — parse, assert the `phrase`, the
/// full canonical rendering (`NameFormatter.canonical`), the optional rank, and `type=INFORMAL`.
/// Returns the assertion for further chaining.
pub fn assert_phrase_name(
    sciname: &str,
    canonical: &str,
    rank: Option<Rank>,
    phrase: &str,
) -> NameAssertion {
    let n = nameparser::parse_name(sciname, None, None, None)
        .unwrap_or_else(|e| panic!("expected `{sciname}` to parse: {e:?}"));
    assert_eq!(
        n.canonical_name().as_deref(),
        Some(canonical),
        "canonical mismatch for `{sciname}`"
    );
    let na = NameAssertion::new(n).phrase(phrase);
    let na = match rank {
        Some(r) => na.rank(r),
        None => na,
    };
    na.type_(NameType::Informal)
}

/// `assertNomNote(note, sciname)` — parse `sciname` and assert its nomenclatural note. Returns
/// the assertion for further chaining.
pub fn assert_nom_note(note: &str, sciname: &str) -> NameAssertion {
    let n = nameparser::parse_name(sciname, None, None, None)
        .unwrap_or_else(|e| panic!("expected `{sciname}` to parse: {e:?}"));
    NameAssertion::new(n).nom_note(note)
}

/// `assertCultivar(note)` — parse `"Abies alba <note>"` and assert its nomenclatural note
/// equals `note` (Java's helper is misnamed; it checks the nom-note). Returns the assertion.
pub fn assert_cultivar(note: &str) -> NameAssertion {
    let sciname = format!("Abies alba {note}");
    let n = nameparser::parse_name(&sciname, None, None, None)
        .unwrap_or_else(|e| panic!("expected `{sciname}` to parse: {e:?}"));
    NameAssertion::new(n).nom_note(note)
}

/// `assertAuthorship(rawAuthorship, expectedAuthors...)` — parse a bare authorship string and
/// assert the combination authors. Java's `parseAuthorship(auth, code)` is
/// `parse("Abies alba", auth, SPECIES, code)` reading `combinationAuthorship`, reproduced here.
pub fn assert_authorship(raw: &str, expected_authors: &[&str]) -> NameAssertion {
    assert_ex_authorship(raw, None, expected_authors)
}

/// `assertSingleAuthor(raw)` — a bare authorship parsing to exactly the single author `raw`.
pub fn assert_single_author(raw: &str) -> NameAssertion {
    assert_ex_authorship(raw, None, &[raw])
}

/// `assertExAuthorship(rawAuthorship, exAuthor, expectedAuthors...)` — parse a bare authorship
/// and assert its ex-author (or none) and combination authors.
pub fn assert_ex_authorship(
    raw: &str,
    ex_author: Option<&str>,
    expected_authors: &[&str],
) -> NameAssertion {
    let full = nameparser::parse_name("Abies alba", Some(raw), Some(Rank::Species), None)
        .unwrap_or_else(|e| panic!("authorship `{raw}` should parse: {e:?}"));
    let auth = &full.combination_authorship;
    match ex_author {
        None => assert!(
            auth.ex_authors.is_empty(),
            "unexpected exAuthors for `{raw}`: {:?}",
            auth.ex_authors
        ),
        Some(ex) => {
            assert_eq!(
                auth.ex_authors.len(),
                1,
                "expected exactly 1 exAuthor for `{raw}`"
            );
            assert_eq!(auth.ex_authors[0], ex, "exAuthor mismatch for `{raw}`");
        }
    }
    if !expected_authors.is_empty() {
        assert_eq!(
            auth.authors,
            str_vec(expected_authors),
            "authors mismatch for `{raw}`"
        );
    }
    // Authorship-only assertion, mirroring Java's `NameAssertion(ParsedAuthorship)` =
    // `new ParsedName(); n.copy(pa)`: the 16 ParsedName-own fields reset to their defaults
    // (name parts, rank, code, type, notho, …), and the 11 ParsedAuthorship + 3
    // CombinedAuthorship fields carried over from the parse — so a chained `.sensu()`,
    // `.nom_note()`, `.doubtful()`, etc. sees the note/flag the authorship parse produced (e.g.
    // "auct. nec Zeller, 1877" lands in taxonomicNote, not in the author list).
    let pn = ParsedName {
        extinct: full.extinct,
        taxonomic_note: full.taxonomic_note,
        nomenclatural_note: full.nomenclatural_note,
        published_in: full.published_in,
        published_in_year: full.published_in_year,
        published_in_page: full.published_in_page,
        unparsed: full.unparsed,
        doubtful: full.doubtful,
        manuscript: full.manuscript,
        state: full.state,
        warnings: full.warnings,
        combination_authorship: full.combination_authorship,
        basionym_authorship: full.basionym_authorship,
        sanctioning_author: full.sanctioning_author,
        ..Default::default()
    };
    NameAssertion::new(pn)
}

/// `isViralName(name)` — Java's test helper: parse `name` and report whether the nomenclatural
/// code came out as `VIRUS` (on the parsed name, or on the unparsable error). NOT the core
/// `viral::is_viral` word-primitive.
pub fn is_viral_name(name: &str) -> bool {
    match nameparser::parse_name(name, None, None, None) {
        Ok(pn) => pn.code == Some(NomCode::Virus),
        Err(e) => e.type_ == NameType::Other && e.code == Some(NomCode::Virus),
    }
}

// ---- the assertion builder --------------------------------------------------------------------

/// One `ParsedName` field-category, tracked so `nothing_else()` can check the untouched ones are
/// at their default (mirrors Java `NameAssertion.NP`).
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum Np {
    Type,
    Epithets,
    Infragen,
    Phrase,
    Cultivar,
    Extinct,
    Candidate,
    Notho,
    Sic,
    Auth,
    ExAuth,
    Bas,
    ExBas,
    Generic,
    Specific,
    Sanct,
    Rank,
    TaxNote,
    NomNote,
    PublishedIn,
    PublishedInPage,
    ImprintYear,
    Doubtful,
    State,
    Code,
    Remains,
    Warning,
    Manuscript,
    Qualifiers,
}

pub struct NameAssertion {
    n: ParsedName,
    tested: std::collections::HashSet<Np>,
}

impl NameAssertion {
    fn new(n: ParsedName) -> Self {
        NameAssertion {
            n,
            tested: std::collections::HashSet::new(),
        }
    }

    fn mark(mut self, props: &[Np]) -> Self {
        for p in props {
            self.tested.insert(*p);
        }
        self
    }

    fn author_year(a: &nameparser::model::Authorship) -> Option<&str> {
        a.year.as_deref()
    }

    // ---- name parts ----

    pub fn monomial(self, monomial: &str) -> Self {
        self.monomial_rank(monomial, Rank::Unranked)
    }

    pub fn monomial_rank(self, monomial: &str, rank: Rank) -> Self {
        assert_eq!(self.n.uninomial.as_deref(), Some(monomial));
        assert_eq!(self.n.rank, rank);
        assert!(self.n.genus.is_none());
        assert!(self.n.infrageneric_epithet.is_none());
        assert!(self.n.specific_epithet.is_none());
        assert!(self.n.infraspecific_epithet.is_none());
        assert!(self.n.cultivar_epithet.is_none());
        self.mark(&[Np::Epithets, Np::Rank, Np::Cultivar])
    }

    pub fn infrageneric_at(self, genus: &str, rank: Rank, infrageneric: &str) -> Self {
        assert!(self.n.uninomial.is_none());
        assert_eq!(self.n.genus.as_deref(), Some(genus));
        assert_eq!(self.n.infrageneric_epithet.as_deref(), Some(infrageneric));
        assert!(self.n.specific_epithet.is_none());
        assert!(self.n.infraspecific_epithet.is_none());
        assert_eq!(self.n.rank, rank);
        assert!(self.n.cultivar_epithet.is_none());
        self.mark(&[Np::Epithets, Np::Infragen, Np::Rank, Np::Cultivar])
    }

    pub fn infrageneric(self, infrageneric: &str) -> Self {
        assert_eq!(self.n.infrageneric_epithet.as_deref(), Some(infrageneric));
        self.mark(&[Np::Infragen])
    }

    pub fn species(self, genus: &str, epithet: &str) -> Self {
        self.binomial(genus, None, epithet, Rank::Species)
    }

    pub fn species_ig(self, genus: &str, infrageneric: &str, epithet: &str) -> Self {
        self.binomial(genus, Some(infrageneric), epithet, Rank::Species)
    }

    pub fn binomial(
        self,
        genus: &str,
        infrageneric: Option<&str>,
        epithet: &str,
        rank: Rank,
    ) -> Self {
        assert!(self.n.uninomial.is_none());
        assert_eq!(self.n.genus.as_deref(), Some(genus));
        assert_eq!(self.n.infrageneric_epithet.as_deref(), infrageneric);
        assert_eq!(self.n.specific_epithet.as_deref(), Some(epithet));
        assert!(self.n.infraspecific_epithet.is_none());
        assert_eq!(self.n.rank, rank);
        self.mark(&[Np::Epithets, Np::Infragen, Np::Rank])
    }

    pub fn infra_species(
        self,
        genus: &str,
        epithet: &str,
        rank: Rank,
        infra_epithet: &str,
    ) -> Self {
        assert!(self.n.uninomial.is_none());
        assert_eq!(self.n.genus.as_deref(), Some(genus));
        assert_eq!(self.n.specific_epithet.as_deref(), Some(epithet));
        assert_eq!(self.n.infraspecific_epithet.as_deref(), Some(infra_epithet));
        assert_eq!(self.n.rank, rank);
        self.mark(&[Np::Epithets, Np::Rank])
    }

    pub fn indet(self, genus: &str, epithet: &str, rank: Rank) -> Self {
        assert!(self.n.uninomial.is_none());
        assert_eq!(self.n.genus.as_deref(), Some(genus));
        assert_eq!(self.n.specific_epithet.as_deref(), Some(epithet));
        assert!(self.n.infraspecific_epithet.is_none());
        assert_eq!(self.n.rank, rank);
        assert_eq!(self.n.type_, NameType::Informal);
        self.mark(&[Np::Epithets, Np::Rank, Np::Type])
    }

    // ---- authorship ----

    pub fn comb_authors(self, year: Option<&str>, authors: &[&str]) -> Self {
        assert_eq!(Self::author_year(&self.n.combination_authorship), year);
        assert_eq!(self.n.combination_authorship.authors, str_vec(authors));
        self.mark(&[Np::Auth])
    }

    pub fn comb_ex_authors(self, authors: &[&str]) -> Self {
        assert_eq!(self.n.combination_authorship.ex_authors, str_vec(authors));
        self.mark(&[Np::ExAuth])
    }

    pub fn bas_authors(self, year: Option<&str>, authors: &[&str]) -> Self {
        assert_eq!(Self::author_year(&self.n.basionym_authorship), year);
        assert_eq!(self.n.basionym_authorship.authors, str_vec(authors));
        self.mark(&[Np::Bas])
    }

    pub fn bas_ex_authors(self, _year: Option<&str>, authors: &[&str]) -> Self {
        assert_eq!(self.n.basionym_authorship.ex_authors, str_vec(authors));
        self.mark(&[Np::ExBas])
    }

    /// Combination authors of the genus authorship (infrageneric names, e.g. Kuntze of "(Adans.) Kuntze").
    pub fn generic_authors(self, year: Option<&str>, authors: &[&str]) -> Self {
        let ga = self
            .n
            .generic_authorship
            .as_ref()
            .expect("genericAuthorship set");
        assert_eq!(ga.combination_authorship.year.as_deref(), year);
        assert_eq!(ga.combination_authorship.authors, str_vec(authors));
        self.mark(&[Np::Generic])
    }

    /// Basionym authors of the genus authorship (e.g. Adans. of "(Adans.) Kuntze").
    pub fn generic_bas_authors(self, year: Option<&str>, authors: &[&str]) -> Self {
        let ga = self
            .n
            .generic_authorship
            .as_ref()
            .expect("genericAuthorship set");
        assert_eq!(ga.basionym_authorship.year.as_deref(), year);
        assert_eq!(ga.basionym_authorship.authors, str_vec(authors));
        self.mark(&[Np::Generic])
    }

    /// Combination authors of the species authorship (below-species names, e.g. L. before a cultivar).
    pub fn specific_authors(self, year: Option<&str>, authors: &[&str]) -> Self {
        let sa = self
            .n
            .specific_authorship
            .as_ref()
            .expect("specificAuthorship set");
        assert_eq!(sa.combination_authorship.year.as_deref(), year);
        assert_eq!(sa.combination_authorship.authors, str_vec(authors));
        self.mark(&[Np::Specific])
    }

    pub fn sanct_author(self, author: &str) -> Self {
        assert_eq!(self.n.sanctioning_author.as_deref(), Some(author));
        assert_eq!(self.n.code, Some(NomCode::Botanical));
        self.mark(&[Np::Sanct, Np::Code])
    }

    // ---- flags / notes / misc ----

    pub fn autonym(self) -> Self {
        assert!(self.n.is_autonym(), "expected an autonym");
        self
    }

    pub fn manuscript(self) -> Self {
        assert!(self.n.manuscript);
        self.mark(&[Np::Manuscript])
    }

    pub fn type_(self, type_: NameType) -> Self {
        assert_eq!(self.n.type_, type_);
        self.mark(&[Np::Type])
    }

    pub fn notho(self, notho: &[NamePart]) -> Self {
        let actual: Vec<NamePart> = self.n.notho.clone().unwrap_or_default();
        assert_eq!(actual, notho.to_vec());
        self.mark(&[Np::Notho])
    }

    pub fn sic(self) -> Self {
        assert_eq!(
            self.n.original_spelling,
            Some(true),
            "expected [sic] (originalSpelling=true)"
        );
        self.mark(&[Np::Sic])
    }

    pub fn corrig(self) -> Self {
        assert_eq!(
            self.n.original_spelling,
            Some(false),
            "expected corrig. (originalSpelling=false)"
        );
        self.mark(&[Np::Sic])
    }

    pub fn warning(self, warnings: &[&str]) -> Self {
        let mut got: Vec<String> = self.n.warnings.clone();
        got.sort();
        let mut want: Vec<String> = str_vec(warnings);
        want.sort();
        assert_eq!(got, want, "warnings mismatch");
        self.mark(&[Np::Warning])
    }

    pub fn partial(self, unparsed: &str) -> Self {
        assert_eq!(self.n.state, State::Partial);
        assert_eq!(self.n.unparsed.as_deref(), Some(unparsed));
        self.mark(&[Np::Remains, Np::State])
    }

    pub fn cultivar(self, genus: &str, cultivar: &str) -> Self {
        self.cultivar_full(genus, None, Rank::Cultivar, cultivar)
    }

    pub fn cultivar_rank(self, genus: &str, rank: Rank, cultivar: &str) -> Self {
        self.cultivar_full(genus, None, rank, cultivar)
    }

    pub fn cultivar_sp(self, genus: &str, species: &str, cultivar: &str) -> Self {
        self.cultivar_full(genus, Some(species), Rank::Cultivar, cultivar)
    }

    pub fn cultivar_sp_rank(self, genus: &str, species: &str, rank: Rank, cultivar: &str) -> Self {
        self.cultivar_full(genus, Some(species), rank, cultivar)
    }

    fn cultivar_full(self, genus: &str, species: Option<&str>, rank: Rank, cultivar: &str) -> Self {
        assert!(self.n.uninomial.is_none());
        assert_eq!(self.n.genus.as_deref(), Some(genus));
        assert_eq!(self.n.specific_epithet.as_deref(), species);
        assert!(self.n.infrageneric_epithet.is_none());
        assert!(self.n.infraspecific_epithet.is_none());
        assert_eq!(self.n.cultivar_epithet.as_deref(), Some(cultivar));
        assert_eq!(self.n.rank, rank);
        assert_eq!(self.n.code, Some(NomCode::Cultivars));
        self.mark(&[Np::Epithets, Np::Rank, Np::Cultivar, Np::Code])
    }

    pub fn code(self, code: NomCode) -> Self {
        assert_eq!(self.n.code, Some(code));
        self.mark(&[Np::Code])
    }

    pub fn extinct(self) -> Self {
        assert!(self.n.extinct);
        self.mark(&[Np::Extinct])
    }

    pub fn candidatus(self) -> Self {
        assert!(self.n.candidatus);
        assert_eq!(self.n.code, Some(NomCode::Bacterial));
        self.mark(&[Np::Candidate, Np::Code])
    }

    pub fn phrase(self, phrase: &str) -> Self {
        assert!(self.n.cultivar_epithet.is_none());
        assert_eq!(self.n.phrase.as_deref(), Some(phrase));
        self.mark(&[Np::Phrase])
    }

    /// Java `sensu(...)` — the taxonomic note.
    pub fn sensu(self, sensu: &str) -> Self {
        assert_eq!(self.n.taxonomic_note.as_deref(), Some(sensu));
        self.mark(&[Np::TaxNote])
    }

    pub fn published_in(self, published_in: &str) -> Self {
        assert_eq!(self.n.published_in.as_deref(), Some(published_in));
        self.mark(&[Np::PublishedIn])
    }

    pub fn published_in_page(self, page: &str) -> Self {
        assert_eq!(self.n.published_in_page.as_deref(), Some(page));
        self.mark(&[Np::PublishedInPage])
    }

    pub fn published_in_year(self, year: Option<i32>) -> Self {
        assert_eq!(self.n.published_in_year, year);
        self
    }

    pub fn imprint_year(self, imprint_year: &str) -> Self {
        assert_eq!(self.imprint_year_of().as_deref(), Some(imprint_year));
        self.mark(&[Np::ImprintYear])
    }

    fn imprint_year_of(&self) -> Option<String> {
        self.n
            .combination_authorship
            .imprint_year
            .clone()
            .or_else(|| self.n.basionym_authorship.imprint_year.clone())
    }

    pub fn nom_note(self, nom_note: &str) -> Self {
        assert_eq!(self.n.nomenclatural_note.as_deref(), Some(nom_note));
        self.mark(&[Np::NomNote])
    }

    pub fn doubtful(self) -> Self {
        assert!(self.n.doubtful);
        self.mark(&[Np::Doubtful])
    }

    pub fn rank(self, rank: Rank) -> Self {
        assert_eq!(self.n.rank, rank);
        self.mark(&[Np::Rank])
    }

    pub fn state(self, state: State) -> Self {
        assert_eq!(self.n.state, state);
        self.mark(&[Np::State])
    }

    /// Java `qualifiers(part, value, part, value, ...)` → pairs.
    pub fn qualifiers(self, pairs: &[(NamePart, &str)]) -> Self {
        let map: &BTreeMap<NamePart, String> = self
            .n
            .epithet_qualifier
            .as_ref()
            .expect("epithetQualifier set");
        for (part, qual) in pairs {
            assert_eq!(map.get(part).map(String::as_str), Some(*qual));
        }
        assert_eq!(map.len(), pairs.len());
        self.mark(&[Np::Qualifiers])
    }

    // ---- the closer: every untouched field must be at its default ----

    /// Assert that every field NOT covered by a previous method call is at its default value —
    /// so the whole parse is pinned. Mirrors Java `NameAssertion.nothingElse()`.
    pub fn nothing_else(self) {
        let n = &self.n;
        let untested = |p: Np| !self.tested.contains(&p);

        if untested(Np::Epithets) {
            assert!(
                n.uninomial.is_none(),
                "unexpected uninomial: {:?}",
                n.uninomial
            );
            assert!(n.genus.is_none(), "unexpected genus: {:?}", n.genus);
            assert!(
                n.specific_epithet.is_none(),
                "unexpected specificEpithet: {:?}",
                n.specific_epithet
            );
            assert!(
                n.infraspecific_epithet.is_none(),
                "unexpected infraspecificEpithet: {:?}",
                n.infraspecific_epithet
            );
        }
        if untested(Np::Infragen) {
            assert!(
                n.infrageneric_epithet.is_none(),
                "unexpected infragenericEpithet: {:?}",
                n.infrageneric_epithet
            );
        }
        if untested(Np::Phrase) {
            assert!(n.phrase.is_none(), "unexpected phrase: {:?}", n.phrase);
        }
        if untested(Np::Cultivar) {
            assert!(
                n.cultivar_epithet.is_none(),
                "unexpected cultivarEpithet: {:?}",
                n.cultivar_epithet
            );
        }
        if untested(Np::Candidate) {
            assert!(!n.candidatus, "unexpected candidatus");
        }
        if untested(Np::Extinct) {
            assert!(!n.extinct, "unexpected extinct");
        }
        if untested(Np::Notho) {
            assert!(
                n.notho.as_ref().is_none_or(|v| v.is_empty()),
                "unexpected notho: {:?}",
                n.notho
            );
        }
        if untested(Np::Sic) {
            assert!(
                n.original_spelling.is_none(),
                "unexpected originalSpelling: {:?}",
                n.original_spelling
            );
        }
        if untested(Np::Auth) {
            assert!(
                n.combination_authorship.year.is_none(),
                "unexpected comb year"
            );
            assert!(
                !n.combination_authorship.has_authors(),
                "unexpected comb authors"
            );
        }
        if untested(Np::ExAuth) {
            assert!(
                n.combination_authorship.ex_authors.is_empty(),
                "unexpected comb exAuthors"
            );
        }
        if untested(Np::Bas) {
            assert!(n.basionym_authorship.year.is_none(), "unexpected bas year");
            assert!(
                !n.basionym_authorship.has_authors(),
                "unexpected bas authors"
            );
        }
        if untested(Np::ExBas) {
            assert!(
                n.basionym_authorship.ex_authors.is_empty(),
                "unexpected bas exAuthors"
            );
        }
        if untested(Np::Generic) {
            assert!(
                n.generic_authorship
                    .as_ref()
                    .is_none_or(|a| !a.has_authorship()),
                "unexpected genericAuthorship"
            );
        }
        if untested(Np::Specific) {
            assert!(
                n.specific_authorship
                    .as_ref()
                    .is_none_or(|a| !a.has_authorship()),
                "unexpected specificAuthorship"
            );
        }
        if untested(Np::Sanct) {
            assert!(
                n.sanctioning_author.is_none(),
                "unexpected sanctioningAuthor: {:?}",
                n.sanctioning_author
            );
        }
        if untested(Np::Rank) {
            assert_eq!(n.rank, Rank::Unranked, "unexpected rank");
        }
        if untested(Np::TaxNote) {
            assert!(
                n.taxonomic_note.is_none(),
                "unexpected taxonomicNote: {:?}",
                n.taxonomic_note
            );
        }
        if untested(Np::NomNote) {
            assert!(
                n.nomenclatural_note.is_none(),
                "unexpected nomenclaturalNote: {:?}",
                n.nomenclatural_note
            );
        }
        if untested(Np::PublishedIn) {
            assert!(
                n.published_in.is_none(),
                "unexpected publishedIn: {:?}",
                n.published_in
            );
        }
        if untested(Np::PublishedInPage) {
            assert!(
                n.published_in_page.is_none(),
                "unexpected publishedInPage: {:?}",
                n.published_in_page
            );
        }
        if untested(Np::ImprintYear) {
            assert!(self.imprint_year_of().is_none(), "unexpected imprintYear");
        }
        if untested(Np::Doubtful) {
            assert!(!n.doubtful, "unexpected doubtful");
        }
        if untested(Np::State) {
            assert_eq!(n.state, State::Complete, "unexpected state");
        }
        if untested(Np::Type) {
            assert_eq!(n.type_, NameType::Scientific, "unexpected type");
        }
        if untested(Np::Code) {
            assert!(n.code.is_none(), "unexpected code: {:?}", n.code);
        }
        if untested(Np::Remains) {
            assert!(
                n.unparsed.is_none(),
                "unexpected unparsed: {:?}",
                n.unparsed
            );
        }
        if untested(Np::Warning) {
            assert!(
                n.warnings.is_empty(),
                "unexpected warnings: {:?}",
                n.warnings
            );
        }
        if untested(Np::Manuscript) {
            assert!(!n.manuscript, "unexpected manuscript");
        }
        if untested(Np::Qualifiers) {
            assert!(
                n.epithet_qualifier.as_ref().is_none_or(|m| m.is_empty()),
                "unexpected epithetQualifier: {:?}",
                n.epithet_qualifier
            );
        }
    }
}

// ---- the Informal assertion builder -----------------------------------------------------------

/// The two OPTIONAL [`Informal`] fields, tracked so [`InformalAssertion::nothing_else`] can check
/// the untouched ones are absent. `taxon`/`taxon_rank`/`rank` are always populated on a valid
/// informal result, so they have no "default" to check.
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum InfProp {
    Phrase,
    Code,
}

/// Fluent assertion over a [`ParseResult::Informal`], mirroring [`NameAssertion`]'s chaining style.
/// Every method returns `self`; [`Self::nothing_else`] closes the chain by asserting the optional
/// fields you did NOT mention (`phrase`, `code`) are absent — so a test pins the whole informal
/// result, not just the parts it named.
pub struct InformalAssertion {
    inf: Informal,
    tested: std::collections::HashSet<InfProp>,
}

impl InformalAssertion {
    fn new(inf: Informal) -> Self {
        InformalAssertion {
            inf,
            tested: std::collections::HashSet::new(),
        }
    }

    /// Assert the supraspecific taxon anchor (`"Rhizobium"`, `"Ichneumonidae"`).
    pub fn taxon(self, taxon: &str) -> Self {
        assert_eq!(self.inf.taxon, taxon, "taxon mismatch");
        self
    }

    /// Assert the anchor's rank (usually `Genus`, since the anchor sits in the genus slot).
    pub fn taxon_rank(self, rank: Rank) -> Self {
        assert_eq!(
            self.inf.taxon_rank, rank,
            "taxonRank mismatch for {:?}",
            self.inf.taxon
        );
        self
    }

    /// Assert the informal name's own purported rank (`Species` for `"sp."`, `Unranked` for a group).
    pub fn rank(self, rank: Rank) -> Self {
        assert_eq!(
            self.inf.rank, rank,
            "rank mismatch for {:?}",
            self.inf.taxon
        );
        self
    }

    /// Assert the distinguishing phrase (`"RMCC TR1811"`, `"1"`).
    pub fn phrase(mut self, phrase: &str) -> Self {
        assert_eq!(self.inf.phrase.as_deref(), Some(phrase), "phrase mismatch");
        self.tested.insert(InfProp::Phrase);
        self
    }

    /// Assert there is NO phrase — a bare `"Genus sp."`.
    pub fn no_phrase(mut self) -> Self {
        assert_eq!(
            self.inf.phrase, None,
            "expected no phrase, got {:?}",
            self.inf.phrase
        );
        self.tested.insert(InfProp::Phrase);
        self
    }

    /// Assert the nomenclatural code.
    pub fn code(mut self, code: NomCode) -> Self {
        assert_eq!(self.inf.code, Some(code), "code mismatch");
        self.tested.insert(InfProp::Code);
        self
    }

    /// Close the chain: every optional field not mentioned above (`phrase`, `code`) must be absent.
    pub fn nothing_else(self) {
        if !self.tested.contains(&InfProp::Phrase) {
            assert!(
                self.inf.phrase.is_none(),
                "unexpected phrase: {:?}",
                self.inf.phrase
            );
        }
        if !self.tested.contains(&InfProp::Code) {
            assert!(
                self.inf.code.is_none(),
                "unexpected code: {:?}",
                self.inf.code
            );
        }
    }
}

fn str_vec(s: &[&str]) -> Vec<String> {
    s.iter().map(|x| x.to_string()).collect()
}
