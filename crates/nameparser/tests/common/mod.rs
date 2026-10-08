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
//! * **`NameType` has 6 variants** in the 5.x api (`Scientific`, `Formula`, `Informal`,
//!   `Placeholder`, `Identifier`, `Other`). Java `HYBRID_FORMULA` → `Formula`; a standalone
//!   SH/BOLD/MOTU code or culture accession → `Identifier`; `VIRUS`/`OTU`/`NO_NAME`/
//!   `BLACKLISTED`/`DOUBTFUL`/`OTHER` → `Other` (the `NomCode`/`state`/`doubtful` fields carry the
//!   finer distinction). When a ported assertion's type is ambiguous, check the name's actual
//!   output in `testdata/golden/expected-parse.jsonl` (the regression snapshot of the corpus).
//! * Overloaded builder methods are disambiguated by suffix: `species(g,ig,e)` → `species_ig`,
//!   `infraGeneric(g,rank,ig)` → `infrageneric_at`, the `cultivar(...)` overloads → `cultivar` /
//!   `cultivar_rank` / `cultivar_sp` / `cultivar_sp_rank`. Java varargs → a `&[&str]` slice;
//!   Java `null` year → `None`, `"1955"` → `Some("1955")`.

#![allow(dead_code)] // the DSL surface is used across many test files; not every method in each

mod paths;
pub use paths::*;

use std::collections::BTreeMap;

use nameparser::model::{
    Authorship, CombinedAuthorship, Informal, NamePart, NameType, NomCode, ParsedName, Rank, State,
};
use nameparser::ParseResult;

// ---- entry points -----------------------------------------------------------------------------

/// Parse `input` (no hints) through the 5.0.0 [`nameparser::parse`] and assert the outcome is
/// a [`ParseResult::Parsed`] name, starting an assertion chain. Panics (loudly) if the name comes
/// back `Informal` or `Unparsable` — so a determined-name test that wrongly reclassifies as informal
/// fails here, not silently. (The informal band is [`assert_informal`]; the junk band is
/// [`assert_unparsable`].)
#[track_caller]
pub fn assert_name(input: &str) -> NameAssertion {
    assert_name_hinted(input, None, None, None)
}

/// `assertName(name, rawAuthorship, canonical)` — parse with a separately supplied authorship.
#[track_caller]
pub fn assert_name_auth(input: &str, authorship: &str) -> NameAssertion {
    assert_name_hinted(input, Some(authorship), None, None)
}

/// `assertName(name, rank, canonical)` — parse with a rank hint.
#[track_caller]
pub fn assert_name_rank(input: &str, rank: Rank) -> NameAssertion {
    assert_name_hinted(input, None, Some(rank), None)
}

/// `assertName(name, code, canonical)` — parse with a nomenclatural-code hint.
#[track_caller]
pub fn assert_name_code(input: &str, code: NomCode) -> NameAssertion {
    assert_name_hinted(input, None, None, Some(code))
}

/// The full `assertName(name, [authorship,] [rank,] [code], canonical)` variant.
#[track_caller]
pub fn assert_name_hinted(
    input: &str,
    authorship: Option<&str>,
    rank: Option<Rank>,
    code: Option<NomCode>,
) -> NameAssertion {
    let call = Call {
        input,
        authorship,
        rank,
        code,
    };
    check_paths(call, Shape::Name);
    match nameparser::parse(input, authorship, rank, code) {
        ParseResult::Parsed(pn) => NameAssertion::new(pn, &call.label()),
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
#[track_caller]
pub fn assert_informal(input: &str) -> InformalAssertion {
    assert_informal_hinted(input, None, None, None)
}

/// [`assert_informal`] with the optional authorship / rank / code hints.
#[track_caller]
pub fn assert_informal_hinted(
    input: &str,
    authorship: Option<&str>,
    rank: Option<Rank>,
    code: Option<NomCode>,
) -> InformalAssertion {
    let call = Call {
        input,
        authorship,
        rank,
        code,
    };
    check_paths(call, Shape::Name);
    match nameparser::parse(input, authorship, rank, code) {
        ParseResult::Informal(inf) => InformalAssertion::new(inf, &call.label()),
        ParseResult::Parsed(pn) => {
            panic!("expected `{input}` to be an Informal result, but it Parsed: {pn:?}")
        }
        ParseResult::Unparsable(e) => {
            panic!("expected `{input}` to be an Informal result, but it was Unparsable: {e:?}")
        }
    }
}

/// `assertNoName(name)` — the input must be unparsable. Java asserts `NameType.NO_NAME`, which the
/// 5.x `NameType` folds into `Other` (see the mapping note in the module doc).
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
    check_paths(Call::name(raw), Shape::Name);
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
#[track_caller]
pub fn assert_phrase_name(
    sciname: &str,
    canonical: &str,
    rank: Option<Rank>,
    phrase: &str,
) -> NameAssertion {
    check_paths(Call::name(sciname), Shape::Name);
    let n = nameparser::parse_name(sciname, None, None, None)
        .unwrap_or_else(|e| panic!("expected `{sciname}` to parse: {e:?}"));
    assert_eq!(
        n.canonical_name().as_deref(),
        Some(canonical),
        "canonical mismatch for `{sciname}`"
    );
    let na = NameAssertion::new(n, &format!("`{sciname}`")).phrase(phrase);
    let na = match rank {
        Some(r) => na.rank(r),
        None => na,
    };
    na.type_(NameType::Informal)
}

/// `assertNomNote(note, sciname)` — parse `sciname` and assert its nomenclatural note. Returns
/// the assertion for further chaining.
#[track_caller]
pub fn assert_nom_note(note: &str, sciname: &str) -> NameAssertion {
    check_paths(Call::name(sciname), Shape::Name);
    let n = nameparser::parse_name(sciname, None, None, None)
        .unwrap_or_else(|e| panic!("expected `{sciname}` to parse: {e:?}"));
    NameAssertion::new(n, &format!("`{sciname}`")).nom_note(note)
}

/// The raw [`nameparser::parse_name`] result, for a name [`nameparser::parse`] does not report as
/// `Parsed`: a placeholder with its genus missing ("? alba Smith") is unparsable there, but keeps its
/// parts here.
#[track_caller]
pub fn assert_raw_name(input: &str) -> NameAssertion {
    check_paths(Call::name(input), Shape::Name);
    let n = nameparser::parse_name(input, None, None, None)
        .unwrap_or_else(|e| panic!("expected `{input}` to parse: {e:?}"));
    NameAssertion::new(n, &format!("`{input}`"))
}

/// `assertCultivar(note)` — parse `"Abies alba <note>"` and assert its nomenclatural note
/// equals `note` (Java's helper is misnamed; it checks the nom-note). Returns the assertion.
#[track_caller]
pub fn assert_cultivar(note: &str) -> NameAssertion {
    let sciname = format!("Abies alba {note}");
    check_paths(Call::name(&sciname), Shape::Name);
    let n = nameparser::parse_name(&sciname, None, None, None)
        .unwrap_or_else(|e| panic!("expected `{sciname}` to parse: {e:?}"));
    NameAssertion::new(n, &format!("`{sciname}`")).nom_note(note)
}

/// `assertAuthorship(rawAuthorship, expectedAuthors...)` — parse a bare authorship string and
/// assert the combination authors. Java's `parseAuthorship(auth, code)` is
/// `parse("Abies alba", auth, SPECIES, code)` reading `combinationAuthorship`, reproduced here.
#[track_caller]
pub fn assert_authorship(raw: &str, expected_authors: &[&str]) -> NameAssertion {
    assert_ex_authorship(raw, None, expected_authors)
}

/// `assertSingleAuthor(raw)` — a bare authorship parsing to exactly the single author `raw`.
#[track_caller]
pub fn assert_single_author(raw: &str) -> NameAssertion {
    assert_ex_authorship(raw, None, &[raw])
}

/// `assertExAuthorship(rawAuthorship, exAuthor, expectedAuthors...)` — parse a bare authorship
/// and assert its ex-author (or none) and combination authors.
#[track_caller]
pub fn assert_ex_authorship(
    raw: &str,
    ex_author: Option<&str>,
    expected_authors: &[&str],
) -> NameAssertion {
    check_paths(Call::name(raw), Shape::Authorship);
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
        ..Default::default()
    };
    NameAssertion::new(pn, &format!("authorship `{raw}`"))
}

/// The name with its authorship embedded, the name with the authorship passed separately, and the
/// name with it in both must all parse alike — for checks that need no expectation of their own.
pub fn assert_paths_agree(name: &str, authorship: &str) {
    let call = Call {
        input: name,
        authorship: Some(authorship),
        rank: None,
        code: None,
    };
    check_paths(call, Shape::Name);
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
    /// The combination half of `generic_authorship`.
    GenericComb,
    /// The basionym half of `generic_authorship`.
    GenericBas,
    /// The combination half of `specific_authorship`.
    SpecificComb,
    /// The ex authors of `specific_authorship`'s combination half.
    SpecificEx,
    /// The basionym half of `specific_authorship`.
    SpecificBas,
    Sanct,
    /// The basionym authorship's sanctioning author.
    BasSanct,
    Rank,
    TaxNote,
    NomNote,
    PublishedIn,
    PublishedInYear,
    PublishedInPage,
    /// The combination authorship's imprint year.
    ImprintYear,
    /// The basionym authorship's imprint year.
    BasImprintYear,
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
    /// The parsed call, named in the failure message of [`Self::nothing_else`].
    input: String,
    tested: std::collections::HashSet<Np>,
    /// Where the chain starts in the test, and whether it ended in `nothing_else()` — an open
    /// chain reports what it leaves unpinned under `NAMEPARSER_DUMP_OPEN_CHAINS`.
    location: &'static std::panic::Location<'static>,
    closed: bool,
}

impl NameAssertion {
    #[track_caller]
    fn new(n: ParsedName, input: &str) -> Self {
        NameAssertion {
            n,
            input: input.to_string(),
            tested: std::collections::HashSet::new(),
            location: std::panic::Location::caller(),
            closed: false,
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

    /// A genus with no epithet — the name of a provisional designation (`Pultenaea sp. Olinda (R.Coveny
    /// 6616)`), with the rank it stands for.
    pub fn genus_rank(self, genus: &str, rank: Rank) -> Self {
        assert!(self.n.uninomial.is_none());
        assert_eq!(self.n.genus.as_deref(), Some(genus));
        assert!(self.n.infrageneric_epithet.is_none());
        assert!(self.n.specific_epithet.is_none());
        assert!(self.n.infraspecific_epithet.is_none());
        assert_eq!(self.n.rank, rank);
        self.mark(&[Np::Epithets, Np::Infragen, Np::Rank])
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
        let at = format!("{} {}", self.location, self.input);
        assert_eq!(
            Self::author_year(&self.n.combination_authorship),
            year,
            "{at}: combination year"
        );
        assert_eq!(
            self.n.combination_authorship.authors,
            str_vec(authors),
            "{at}: combination authors"
        );
        assert!(
            !self.n.combination_authorship.anonymous,
            "unexpected anonymous combination"
        );
        self.mark(&[Np::Auth])
    }

    /// An anonymous combination ("Anon., 1830"), with the authors attributed to it (`[Bennett]`).
    pub fn comb_anon(self, year: Option<&str>, authors: &[&str]) -> Self {
        assert!(
            self.n.combination_authorship.anonymous,
            "expected an anonymous combination"
        );
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
        assert!(
            !self.n.basionym_authorship.anonymous,
            "unexpected anonymous basionym"
        );
        self.mark(&[Np::Bas])
    }

    /// An anonymous basionym ("(Anon., 1830)"), with the authors attributed to it (`[Bennett]`).
    pub fn bas_anon(self, year: Option<&str>, authors: &[&str]) -> Self {
        assert!(
            self.n.basionym_authorship.anonymous,
            "expected an anonymous basionym"
        );
        assert_eq!(Self::author_year(&self.n.basionym_authorship), year);
        assert_eq!(self.n.basionym_authorship.authors, str_vec(authors));
        self.mark(&[Np::Bas])
    }

    /// Ex-authors of the basionym; `year` is the basionym's year, as in Java `basExAuthors`.
    pub fn bas_ex_authors(self, year: Option<&str>, authors: &[&str]) -> Self {
        assert_eq!(Self::author_year(&self.n.basionym_authorship), year);
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
        self.mark(&[Np::GenericComb])
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
        self.mark(&[Np::GenericBas])
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
        self.mark(&[Np::SpecificComb])
    }

    /// Ex authors of the species authorship ("Mucher" of "… Mucher ex Starm. nothosubsp. …").
    pub fn specific_ex_authors(self, authors: &[&str]) -> Self {
        let sa = self
            .n
            .specific_authorship
            .as_ref()
            .expect("specificAuthorship set");
        assert_eq!(sa.combination_authorship.ex_authors, str_vec(authors));
        self.mark(&[Np::SpecificEx])
    }

    /// Basionym authors of the species authorship ("(Aubl.)" of "… (Aubl.) Sw. var. robusta …").
    pub fn specific_bas_authors(self, year: Option<&str>, authors: &[&str]) -> Self {
        let sa = self
            .n
            .specific_authorship
            .as_ref()
            .expect("specificAuthorship set");
        assert_eq!(sa.basionym_authorship.year.as_deref(), year);
        assert_eq!(sa.basionym_authorship.authors, str_vec(authors));
        self.mark(&[Np::SpecificBas])
    }

    pub fn sanct_author(self, author: &str) -> Self {
        assert_eq!(
            self.n.combination_authorship.sanctioning_author.as_deref(),
            Some(author),
            "{} {}: combination sanctioning author",
            self.location,
            self.input
        );
        assert_eq!(self.n.code, Some(NomCode::Botanical));
        self.mark(&[Np::Sanct, Np::Code])
    }

    /// The basionym's sanctioning author: `(Wulfen : Fr.)`.
    pub fn bas_sanct_author(self, author: &str) -> Self {
        assert_eq!(
            self.n.basionym_authorship.sanctioning_author.as_deref(),
            Some(author),
            "{} {}: basionym sanctioning author",
            self.location,
            self.input
        );
        assert_eq!(self.n.code, Some(NomCode::Botanical));
        self.mark(&[Np::BasSanct, Np::Code])
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
        assert_eq!(
            self.n.code,
            Some(code),
            "{} {}: code {:?}, expected {code:?}",
            self.location,
            self.input,
            self.n.code
        );
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

    /// The year derived from `publishedIn` (or set by an in-citation).
    pub fn published_in_year(self, year: Option<i32>) -> Self {
        assert_eq!(self.n.published_in_year, year);
        self.mark(&[Np::PublishedInYear])
    }

    /// The combination authorship's imprint year ("Storr, 1970 [1969]").
    pub fn imprint_year(self, imprint_year: &str) -> Self {
        assert_eq!(
            self.n.combination_authorship.imprint_year.as_deref(),
            Some(imprint_year)
        );
        self.mark(&[Np::ImprintYear])
    }

    /// The basionym authorship's imprint year ("(Storr, 1970 [1969])").
    pub fn bas_imprint_year(self, imprint_year: &str) -> Self {
        assert_eq!(
            self.n.basionym_authorship.imprint_year.as_deref(),
            Some(imprint_year)
        );
        self.mark(&[Np::BasImprintYear])
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
    pub fn nothing_else(mut self) {
        self.closed = true;
        // Under NAMEPARSER_DUMP_OPEN_CHAINS a failing closed chain reports what it misses instead
        // of failing, so one run collects every chain an engine change touches.
        if std::env::var_os("NAMEPARSER_DUMP_OPEN_CHAINS").is_some()
            && std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.check_nothing_else()))
                .is_err()
        {
            dump_open_chain(self.location, self.missing_assertions());
            return;
        }
        let input = format!("{} {}", self.location, self.input);
        with_input(&input, || self.check_nothing_else());
    }

    fn check_nothing_else(&self) {
        let untested = |p: Np| !self.tested.contains(&p);
        // Exhaustive on purpose (no `..`): a new `ParsedName` field fails to compile here until
        // it is given a default check.
        let ParsedName {
            rank,
            code,
            uninomial,
            genus,
            generic_authorship,
            infrageneric_epithet,
            specific_epithet,
            specific_authorship,
            infraspecific_epithet,
            cultivar_epithet,
            phrase,
            candidatus,
            notho,
            original_spelling,
            epithet_qualifier,
            type_,
            extinct,
            taxonomic_note,
            nomenclatural_note,
            published_in,
            published_in_year,
            published_in_page,
            unparsed,
            doubtful,
            manuscript,
            state,
            warnings,
            combination_authorship,
            basionym_authorship,
        } = &self.n;

        if untested(Np::Epithets) {
            assert!(uninomial.is_none(), "unexpected uninomial: {uninomial:?}");
            assert!(genus.is_none(), "unexpected genus: {genus:?}");
            assert!(
                specific_epithet.is_none(),
                "unexpected specificEpithet: {specific_epithet:?}"
            );
            assert!(
                infraspecific_epithet.is_none(),
                "unexpected infraspecificEpithet: {infraspecific_epithet:?}"
            );
        }
        if untested(Np::Infragen) {
            assert!(
                infrageneric_epithet.is_none(),
                "unexpected infragenericEpithet: {infrageneric_epithet:?}"
            );
        }
        if untested(Np::Phrase) {
            assert!(phrase.is_none(), "unexpected phrase: {phrase:?}");
        }
        if untested(Np::Cultivar) {
            assert!(
                cultivar_epithet.is_none(),
                "unexpected cultivarEpithet: {cultivar_epithet:?}"
            );
        }
        if untested(Np::Candidate) {
            assert!(!candidatus, "unexpected candidatus");
        }
        if untested(Np::Extinct) {
            assert!(!extinct, "unexpected extinct");
        }
        if untested(Np::Notho) {
            assert!(
                notho.as_ref().is_none_or(|v| v.is_empty()),
                "unexpected notho: {notho:?}"
            );
        }
        if untested(Np::Sic) {
            assert!(
                original_spelling.is_none(),
                "unexpected originalSpelling: {original_spelling:?}"
            );
        }
        check_authorship_default(
            "combinationAuthorship",
            combination_authorship,
            untested(Np::Auth),
            untested(Np::ExAuth),
            untested(Np::ImprintYear),
            untested(Np::Sanct),
        );
        check_authorship_default(
            "basionymAuthorship",
            basionym_authorship,
            untested(Np::Bas),
            untested(Np::ExBas),
            untested(Np::BasImprintYear),
            untested(Np::BasSanct),
        );

        check_combined_authorship_default(
            "genericAuthorship",
            generic_authorship,
            untested(Np::GenericComb),
            true,
            untested(Np::GenericBas),
        );
        check_combined_authorship_default(
            "specificAuthorship",
            specific_authorship,
            untested(Np::SpecificComb),
            untested(Np::SpecificEx),
            untested(Np::SpecificBas),
        );
        if untested(Np::Rank) {
            assert_eq!(*rank, Rank::Unranked, "unexpected rank");
        }
        if untested(Np::TaxNote) {
            assert!(
                taxonomic_note.is_none(),
                "unexpected taxonomicNote: {taxonomic_note:?}"
            );
        }
        if untested(Np::NomNote) {
            assert!(
                nomenclatural_note.is_none(),
                "unexpected nomenclaturalNote: {nomenclatural_note:?}"
            );
        }
        if untested(Np::PublishedIn) {
            assert!(
                published_in.is_none(),
                "unexpected publishedIn: {published_in:?}"
            );
        }
        if untested(Np::PublishedInYear) {
            assert!(
                published_in_year.is_none(),
                "unexpected publishedInYear: {published_in_year:?}"
            );
        }
        if untested(Np::PublishedInPage) {
            assert!(
                published_in_page.is_none(),
                "unexpected publishedInPage: {published_in_page:?}"
            );
        }
        if untested(Np::Doubtful) {
            assert!(!doubtful, "unexpected doubtful");
        }
        if untested(Np::State) {
            assert_eq!(*state, State::Complete, "unexpected state");
        }
        if untested(Np::Type) {
            assert_eq!(*type_, NameType::Scientific, "unexpected type");
        }
        if untested(Np::Code) {
            assert!(code.is_none(), "unexpected code: {code:?}");
        }
        if untested(Np::Remains) {
            assert!(unparsed.is_none(), "unexpected unparsed: {unparsed:?}");
        }
        if untested(Np::Warning) {
            assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
        }
        if untested(Np::Manuscript) {
            assert!(!manuscript, "unexpected manuscript");
        }
        if untested(Np::Qualifiers) {
            assert!(
                epithet_qualifier.as_ref().is_none_or(|m| m.is_empty()),
                "unexpected epithetQualifier: {epithet_qualifier:?}"
            );
        }
    }
}

/// [`NameAssertion::nothing_else`] for one [`Authorship`]: the authors/year/anonymous flag, the
/// ex-authors, the imprint year and the sanctioning author are each at their default unless an
/// assertion covered them.
fn check_authorship_default(
    label: &str,
    a: &Authorship,
    main_untested: bool,
    ex_untested: bool,
    imprint_untested: bool,
    sanct_untested: bool,
) {
    let Authorship {
        authors,
        ex_authors,
        year,
        imprint_year,
        anonymous,
        sanctioning_author,
    } = a;
    if sanct_untested {
        assert!(
            sanctioning_author.is_none(),
            "unexpected {label} sanctioningAuthor: {sanctioning_author:?}"
        );
    }
    if main_untested {
        assert!(
            authors.is_empty(),
            "unexpected {label} authors: {authors:?}"
        );
        assert!(year.is_none(), "unexpected {label} year: {year:?}");
        assert!(!anonymous, "unexpected anonymous {label}");
    }
    if ex_untested {
        assert!(
            ex_authors.is_empty(),
            "unexpected {label} exAuthors: {ex_authors:?}"
        );
    }
    if imprint_untested {
        assert!(
            imprint_year.is_none(),
            "unexpected {label} imprintYear: {imprint_year:?}"
        );
    }
}

/// [`NameAssertion::nothing_else`] for the genus / species authorship slots: absent, or every part
/// not covered by an assertion at its default. Their ex-authors, imprint years, anonymous flags and
/// sanctioning author have no assertion of their own, so they must always be at the default.
fn check_combined_authorship_default(
    label: &str,
    ca: &Option<CombinedAuthorship>,
    comb_untested: bool,
    ex_untested: bool,
    bas_untested: bool,
) {
    let Some(CombinedAuthorship {
        combination_authorship,
        basionym_authorship,
    }) = ca
    else {
        assert!(
            comb_untested && ex_untested && bas_untested,
            "{label} asserted but absent"
        );
        return;
    };
    for (half, a, untested, ex_untested) in [
        (
            "combination",
            combination_authorship,
            comb_untested,
            ex_untested,
        ),
        ("basionym", basionym_authorship, bas_untested, true),
    ] {
        check_authorship_default(
            &format!("{label}.{half}"),
            a,
            untested,
            ex_untested,
            true,
            true,
        );
        assert!(!a.anonymous, "unexpected anonymous {label}.{half}");
    }
}

// ---- the Informal assertion builder -----------------------------------------------------------

/// The [`Informal`] fields, tracked so [`InformalAssertion::nothing_else`] can check the result is
/// pinned in full: `taxon`/`taxon_rank`/`rank` are always populated, so they must have been
/// asserted; the optional `phrase`/`code` must be asserted or absent.
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum InfProp {
    Taxon,
    TaxonRank,
    Rank,
    Phrase,
    Code,
}

/// Fluent assertion over a [`ParseResult::Informal`], mirroring [`NameAssertion`]'s chaining style.
/// Every method returns `self`; [`Self::nothing_else`] closes the chain by asserting the optional
/// fields you did NOT mention (`phrase`, `code`) are absent — so a test pins the whole informal
/// result, not just the parts it named.
pub struct InformalAssertion {
    inf: Informal,
    /// The parsed call, named in the failure message of [`Self::nothing_else`].
    input: String,
    tested: std::collections::HashSet<InfProp>,
    /// Where the chain starts in the test, and whether it ended in `nothing_else()` — an open
    /// chain reports what it leaves unpinned under `NAMEPARSER_DUMP_OPEN_CHAINS`.
    location: &'static std::panic::Location<'static>,
    closed: bool,
}

impl InformalAssertion {
    #[track_caller]
    fn new(inf: Informal, input: &str) -> Self {
        InformalAssertion {
            inf,
            input: input.to_string(),
            tested: std::collections::HashSet::new(),
            location: std::panic::Location::caller(),
            closed: false,
        }
    }

    /// Assert the supraspecific taxon anchor (`"Rhizobium"`, `"Ichneumonidae"`).
    pub fn taxon(mut self, taxon: &str) -> Self {
        assert_eq!(self.inf.taxon, taxon, "taxon mismatch");
        self.tested.insert(InfProp::Taxon);
        self
    }

    /// Assert the anchor's rank (usually `Genus`, since the anchor sits in the genus slot).
    pub fn taxon_rank(mut self, rank: Rank) -> Self {
        assert_eq!(
            self.inf.taxon_rank, rank,
            "taxonRank mismatch for {:?}",
            self.inf.taxon
        );
        self.tested.insert(InfProp::TaxonRank);
        self
    }

    /// Assert the informal name's own purported rank (`Species` for `"sp."`, `Unranked` for a group).
    pub fn rank(mut self, rank: Rank) -> Self {
        assert_eq!(
            self.inf.rank, rank,
            "rank mismatch for {:?}",
            self.inf.taxon
        );
        self.tested.insert(InfProp::Rank);
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

    /// Close the chain: `taxon`, `taxon_rank` and `rank` must have been asserted, and every optional
    /// field not mentioned above (`phrase`, `code`) must be absent.
    pub fn nothing_else(mut self) {
        self.closed = true;
        let input = format!("{} {}", self.location, self.input);
        with_input(&input, || self.check_nothing_else());
    }

    fn check_nothing_else(&self) {
        for (prop, name) in [
            (InfProp::Taxon, "taxon"),
            (InfProp::TaxonRank, "taxon_rank"),
            (InfProp::Rank, "rank"),
        ] {
            assert!(
                self.tested.contains(&prop),
                "{name} not asserted for {:?} — an informal result is only pinned with all three",
                self.inf
            );
        }
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

// ---- whole-parse comparison -------------------------------------------------------------------

/// Fields Java serialises from a `Set`/`Map`-like collection whose iteration order is not
/// an insertion-order guarantee: `warnings` (`HashSet<String>`), `notho`
/// (`EnumSet<NamePart>`), `epithetQualifier` (`EnumMap<NamePart, String>`). Routed through
/// [`json_eq_unordered`] rather than plain `serde_json::Value` equality — see that
/// function's own doc comment. Every other field (including the nested authorship
/// objects, whose `authors`/`exAuthors` arrays ARE genuinely ordered) uses plain equality.
pub const UNORDERED_FIELD_KEYS: [&str; 3] = ["warnings", "notho", "epithetQualifier"];

/// Order-insensitive equality for a JSON value Java serialises from a `Set`/`Map`-like
/// collection: `warnings`'s `HashSet<String>`, `notho`'s `EnumSet<NamePart>` (both -> a JSON
/// array) and `epithetQualifier`'s `EnumMap<NamePart, String>` (-> a JSON object). Java's
/// `HashSet`/`EnumSet` iteration order is not guaranteed to match insertion order, and
/// `serde_json::Value`'s own `PartialEq` for the `Array` variant IS positional — so these
/// fields need this explicit set-shaped comparison. `epithetQualifier` (a JSON *object*)
/// would in practice already compare order-insensitively via plain `==` given this crate's
/// `serde_json` dependency has no `preserve_order` feature enabled (its `Map` is
/// `BTreeMap`-backed, canonically key-ordered regardless of insertion order) — but it's
/// routed through here too rather than leaning on that feature-flag default, so the
/// comparison stays correct even if that default ever changes.
///
/// `None`/absent on both sides is equal; one side absent and the other present (even an
/// empty array/object) is a mismatch, matching plain `Option`/`Value` equality — only the
/// *internal* ordering of a doubly-present array/object is ignored.
pub fn json_eq_unordered(jv: Option<&serde_json::Value>, rv: Option<&serde_json::Value>) -> bool {
    match (jv, rv) {
        (None, None) => true,
        (Some(a), Some(b)) => match (a, b) {
            (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                let mut a_sorted: Vec<String> = a.iter().map(|v| v.to_string()).collect();
                let mut b_sorted: Vec<String> = b.iter().map(|v| v.to_string()).collect();
                a_sorted.sort();
                b_sorted.sort();
                a_sorted == b_sorted
            }
            (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                let mut a_pairs: Vec<(String, String)> =
                    a.iter().map(|(k, v)| (k.clone(), v.to_string())).collect();
                let mut b_pairs: Vec<(String, String)> =
                    b.iter().map(|(k, v)| (k.clone(), v.to_string())).collect();
                a_pairs.sort();
                b_pairs.sort();
                a_pairs == b_pairs
            }
            _ => a == b,
        },
        _ => false,
    }
}

/// Dispatches to [`json_eq_unordered`] for [`UNORDERED_FIELD_KEYS`], plain
/// `serde_json::Value` equality for every other field (including the nested authorship
/// objects — see the module doc for why plain equality is correct there too).
pub fn fields_equal(
    key: &str,
    jv: Option<&serde_json::Value>,
    rv: Option<&serde_json::Value>,
) -> bool {
    if UNORDERED_FIELD_KEYS.contains(&key) {
        json_eq_unordered(jv, rv)
    } else {
        jv == rv
    }
}

/// The `ParsedName` fields on which `a` and `b` differ, as `(field, a, b)` with the JSON wire names
/// and values (`-` for an absent field), compared with [`fields_equal`]. Empty when the two parses
/// are the same.
pub fn parsed_name_diff(a: &ParsedName, b: &ParsedName) -> Vec<(String, String, String)> {
    let (ja, jb) = (
        serde_json::to_value(a).expect("ParsedName serialises"),
        serde_json::to_value(b).expect("ParsedName serialises"),
    );
    let (ma, mb) = (ja.as_object().unwrap(), jb.as_object().unwrap());
    let mut keys: Vec<&String> = ma.keys().chain(mb.keys()).collect();
    keys.sort();
    keys.dedup();
    let show = |v: Option<&serde_json::Value>| v.map_or("-".to_string(), |v| v.to_string());
    keys.into_iter()
        .filter(|k| !fields_equal(k, ma.get(*k), mb.get(*k)))
        .map(|k| (k.clone(), show(ma.get(k)), show(mb.get(k))))
        .collect()
}

// ---- open chains ------------------------------------------------------------------------------

/// Appends one `file<TAB>line<TAB>column<TAB>call<US>call…` line per open chain to the file named
/// by `NAMEPARSER_DUMP_OPEN_CHAINS`: the DSL calls that would pin every field the chain left
/// unchecked, generated from the actual parse (a `// TODO` call marks a field no single DSL call
/// can pin). A tool for closing chains — every generated call still needs a review before it is
/// taken as the expectation.
fn dump_open_chain(location: &std::panic::Location<'static>, calls: Vec<String>) {
    if std::thread::panicking() {
        return;
    }
    let Ok(path) = std::env::var("NAMEPARSER_DUMP_OPEN_CHAINS") else {
        return;
    };
    use std::io::Write;
    let line = format!(
        "{}\t{}\t{}\t{}\n",
        location.file(),
        location.line(),
        location.column(),
        calls.join("\u{1f}")
    );
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(line.as_bytes()))
        .unwrap_or_else(|e| panic!("cannot write {path}: {e}"));
}

impl Drop for NameAssertion {
    fn drop(&mut self) {
        if !self.closed {
            dump_open_chain(self.location, self.missing_assertions());
        }
    }
}

impl Drop for InformalAssertion {
    fn drop(&mut self) {
        if !self.closed {
            dump_open_chain(self.location, self.missing_assertions());
        }
    }
}

/// A Rust string literal for `s`.
fn lit(s: &str) -> String {
    format!("{s:?}")
}

/// A Rust `&[…]` literal of string literals.
fn lits(v: &[String]) -> String {
    let items: Vec<String> = v.iter().map(|s| lit(s)).collect();
    format!("&[{}]", items.join(", "))
}

/// `Some("…")` / `None` for an optional year.
fn opt_lit(y: &Option<String>) -> String {
    y.as_deref()
        .map_or("None".to_string(), |y| format!("Some({})", lit(y)))
}

impl NameAssertion {
    /// The DSL calls pinning every field no assertion of this chain covered — see
    /// [`dump_open_chain`].
    fn missing_assertions(&self) -> Vec<String> {
        let n = &self.n;
        let mut out = Vec::new();
        let mut code_done = false;
        // the name parts first: their calls also pin the rank (and some the infrageneric epithet
        // and cultivar)
        let mut covered = self.tested.clone();
        if !covered.contains(&Np::Epithets) {
            let r = n.rank;
            let call = match (
                &n.uninomial,
                &n.genus,
                &n.infrageneric_epithet,
                &n.specific_epithet,
                &n.infraspecific_epithet,
                &n.cultivar_epithet,
            ) {
                (None, None, None, None, None, None) => None,
                (Some(u), None, None, None, None, None) => Some((
                    format!(".monomial_rank({}, Rank::{r:?})", lit(u)),
                    vec![Np::Epithets, Np::Rank, Np::Cultivar],
                )),
                (None, Some(g), None, Some(e), None, None) if r == Rank::Species => Some((
                    format!(".species({}, {})", lit(g), lit(e)),
                    vec![Np::Epithets, Np::Infragen, Np::Rank],
                )),
                (None, Some(g), ig, Some(e), None, None) => Some((
                    format!(
                        ".binomial({}, {}, {}, Rank::{r:?})",
                        lit(g),
                        ig.as_deref()
                            .map_or("None".to_string(), |ig| format!("Some({})", lit(ig))),
                        lit(e)
                    ),
                    vec![Np::Epithets, Np::Infragen, Np::Rank],
                )),
                (None, Some(g), None, Some(e), Some(i), None) => Some((
                    format!(
                        ".infra_species({}, {}, Rank::{r:?}, {})",
                        lit(g),
                        lit(e),
                        lit(i)
                    ),
                    vec![Np::Epithets, Np::Rank],
                )),
                (None, Some(g), None, None, None, None) => Some((
                    format!(".genus_rank({}, Rank::{r:?})", lit(g)),
                    vec![Np::Epithets, Np::Infragen, Np::Rank],
                )),
                (None, Some(g), Some(ig), None, None, None) => Some((
                    format!(".infrageneric_at({}, Rank::{r:?}, {})", lit(g), lit(ig)),
                    vec![Np::Epithets, Np::Infragen, Np::Rank, Np::Cultivar],
                )),
                (u, g, ig, e, i, cv) => {
                    out.push(format!(
                        "// TODO name parts: {u:?} {g:?} {ig:?} {e:?} {i:?} {cv:?}"
                    ));
                    None
                }
            };
            if let Some((call, marks)) = call {
                out.push(call);
                covered.extend(marks);
            }
        }
        let untested = |p: Np| !covered.contains(&p);
        if untested(Np::Infragen) {
            if let Some(x) = &n.infrageneric_epithet {
                out.push(format!(".infrageneric({})", lit(x)));
            }
        }
        if untested(Np::Cultivar) && n.cultivar_epithet.is_some() {
            out.push(format!("// TODO cultivar {:?}", n.cultivar_epithet));
        }
        if untested(Np::Phrase) {
            if let Some(x) = &n.phrase {
                out.push(format!(".phrase({})", lit(x)));
            }
        }
        if untested(Np::Rank) && n.rank != Rank::Unranked {
            out.push(format!(".rank(Rank::{:?})", n.rank));
        }
        let a = &n.combination_authorship;
        if untested(Np::Auth) && (a.anonymous || !a.authors.is_empty() || a.year.is_some()) {
            let m = if a.anonymous {
                "comb_anon"
            } else {
                "comb_authors"
            };
            out.push(format!(".{m}({}, {})", opt_lit(&a.year), lits(&a.authors)));
        }
        if untested(Np::ExAuth) && !a.ex_authors.is_empty() {
            out.push(format!(".comb_ex_authors({})", lits(&a.ex_authors)));
        }
        if untested(Np::ImprintYear) {
            if let Some(y) = &a.imprint_year {
                out.push(format!(".imprint_year({})", lit(y)));
            }
        }
        let b = &n.basionym_authorship;
        if untested(Np::Bas) && (b.anonymous || !b.authors.is_empty() || b.year.is_some()) {
            let m = if b.anonymous {
                "bas_anon"
            } else {
                "bas_authors"
            };
            out.push(format!(".{m}({}, {})", opt_lit(&b.year), lits(&b.authors)));
        }
        if untested(Np::ExBas) && !b.ex_authors.is_empty() {
            out.push(format!(
                ".bas_ex_authors({}, {})",
                opt_lit(&b.year),
                lits(&b.ex_authors)
            ));
        }
        if untested(Np::BasImprintYear) {
            if let Some(y) = &b.imprint_year {
                out.push(format!(".bas_imprint_year({})", lit(y)));
            }
        }
        for (slot, ca, comb_m, bas_m, comb_np, bas_np) in [
            (
                "generic",
                &n.generic_authorship,
                "generic_authors",
                Some("generic_bas_authors"),
                Np::GenericComb,
                Some(Np::GenericBas),
            ),
            (
                "specific",
                &n.specific_authorship,
                "specific_authors",
                Some("specific_bas_authors"),
                Np::SpecificComb,
                Some(Np::SpecificBas),
            ),
        ] {
            let Some(ca) = ca else { continue };
            let c = &ca.combination_authorship;
            if untested(comb_np) && (!c.authors.is_empty() || c.year.is_some()) {
                out.push(format!(
                    ".{comb_m}({}, {})",
                    opt_lit(&c.year),
                    lits(&c.authors)
                ));
            }
            let bb = &ca.basionym_authorship;
            if !bb.authors.is_empty() || bb.year.is_some() {
                match (bas_m, bas_np) {
                    (Some(m), Some(np)) if untested(np) => out.push(format!(
                        ".{m}({}, {})",
                        opt_lit(&bb.year),
                        lits(&bb.authors)
                    )),
                    (Some(_), _) => {}
                    _ => out.push(format!("// TODO {slot} basionym {bb:?}")),
                }
            }
        }
        for (np, method, sanct) in [
            (
                Np::Sanct,
                "sanct_author",
                &n.combination_authorship.sanctioning_author,
            ),
            (
                Np::BasSanct,
                "bas_sanct_author",
                &n.basionym_authorship.sanctioning_author,
            ),
        ] {
            if untested(np) {
                if let Some(s) = sanct {
                    if n.code == Some(NomCode::Botanical) {
                        out.push(format!(".{method}({})", lit(s)));
                        code_done = true;
                    } else {
                        out.push(format!("// TODO {method} {s:?} with code {:?}", n.code));
                    }
                }
            }
        }
        for (np, v, m) in [
            (Np::TaxNote, &n.taxonomic_note, "sensu"),
            (Np::NomNote, &n.nomenclatural_note, "nom_note"),
            (Np::PublishedIn, &n.published_in, "published_in"),
            (
                Np::PublishedInPage,
                &n.published_in_page,
                "published_in_page",
            ),
        ] {
            if let (true, Some(x)) = (untested(np), v) {
                out.push(format!(".{m}({})", lit(x)));
            }
        }
        if untested(Np::PublishedInYear) {
            if let Some(y) = n.published_in_year {
                out.push(format!(".published_in_year(Some({y}))"));
            }
        }
        for (np, set, m) in [
            (Np::Doubtful, n.doubtful, "doubtful"),
            (Np::Manuscript, n.manuscript, "manuscript"),
            (Np::Extinct, n.extinct, "extinct"),
        ] {
            if untested(np) && set {
                out.push(format!(".{m}()"));
            }
        }
        if untested(Np::Candidate) && n.candidatus {
            if n.code == Some(NomCode::Bacterial) {
                out.push(".candidatus()".to_string());
                code_done = true;
            } else {
                out.push(format!("// TODO candidatus with code {:?}", n.code));
            }
        }
        if untested(Np::Notho) {
            if let Some(parts) = n.notho.as_ref().filter(|v| !v.is_empty()) {
                let parts: Vec<String> = parts.iter().map(|p| format!("NamePart::{p:?}")).collect();
                out.push(format!(".notho(&[{}])", parts.join(", ")));
            }
        }
        if untested(Np::Sic) {
            match n.original_spelling {
                Some(true) => out.push(".sic()".to_string()),
                Some(false) => out.push(".corrig()".to_string()),
                None => {}
            }
        }
        if untested(Np::Qualifiers) {
            if let Some(map) = n.epithet_qualifier.as_ref().filter(|m| !m.is_empty()) {
                let pairs: Vec<String> = map
                    .iter()
                    .map(|(p, q)| format!("(NamePart::{p:?}, {})", lit(q)))
                    .collect();
                out.push(format!(".qualifiers(&[{}])", pairs.join(", ")));
            }
        }
        if untested(Np::Remains) || untested(Np::State) {
            match (&n.state, &n.unparsed) {
                (State::Partial, Some(u)) if untested(Np::Remains) && untested(Np::State) => {
                    out.push(format!(".partial({})", lit(u)))
                }
                (State::Complete, None) => {}
                (state, unparsed) => {
                    out.push(format!("// TODO state {state:?}, unparsed {unparsed:?}"))
                }
            }
        }
        if untested(Np::Warning) && !n.warnings.is_empty() {
            out.push(format!(".warning({})", lits(&n.warnings)));
        }
        if untested(Np::Type) && n.type_ != NameType::Scientific {
            out.push(format!(".type_(NameType::{:?})", n.type_));
        }
        if untested(Np::Code) && !code_done {
            if let Some(c) = n.code {
                out.push(format!(".code(NomCode::{c:?})"));
            }
        }
        out
    }
}

impl InformalAssertion {
    /// The DSL calls pinning every field no assertion of this chain covered — see
    /// [`dump_open_chain`].
    fn missing_assertions(&self) -> Vec<String> {
        let i = &self.inf;
        let untested = |p: InfProp| !self.tested.contains(&p);
        let mut out = Vec::new();
        if untested(InfProp::Taxon) {
            out.push(format!(".taxon({})", lit(&i.taxon)));
        }
        if untested(InfProp::TaxonRank) {
            out.push(format!(".taxon_rank(Rank::{:?})", i.taxon_rank));
        }
        if untested(InfProp::Rank) {
            out.push(format!(".rank(Rank::{:?})", i.rank));
        }
        if untested(InfProp::Phrase) {
            if let Some(p) = &i.phrase {
                out.push(format!(".phrase({})", lit(p)));
            }
        }
        if untested(InfProp::Code) {
            if let Some(c) = i.code {
                out.push(format!(".code(NomCode::{c:?})"));
            }
        }
        out
    }
}

/// Runs `check`, re-raising any assertion failure prefixed with the input it was about — the
/// field-level messages alone don't say which of a test's many names failed.
fn with_input(input: &str, check: impl FnOnce()) {
    if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check)) {
        let msg = e
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        panic!("{input}: {msg}");
    }
}

fn str_vec(s: &[&str]) -> Vec<String> {
    s.iter().map(|x| x.to_string()).collect()
}
