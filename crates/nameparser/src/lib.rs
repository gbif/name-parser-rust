// SPDX-License-Identifier: Apache-2.0

//! A faithful Rust port of the GBIF scientific name parser. [`parse_name`] turns a scientific name into
//! its structured atoms — genus, epithets, authorship, rank, nomenclatural code, notes, warnings —
//! as a [`model::ParsedName`]. Byte-for-byte cross-validated against the Java `name-parser`
//! (`NameParserImpl`) over 11,302 + 6.4M names (0 diffs). The same engine also ships as a native
//! CLI and as Java/Python/R bindings — see the repository README.

pub mod format;
pub mod model;
pub mod pipeline;
pub mod regexes;
pub mod token;
pub mod unicode;
pub mod viral;

use model::{Informal, NameType, NomCode, ParseError, ParsedName, Rank};

/// The lower-level raw parse: a scientific name — optionally alongside a separately supplied
/// authorship string, a requested [`Rank`] and a requested [`NomCode`] — into a [`ParsedName`], or
/// `Err(`[`ParseError`]`)` when the input cannot be parsed into a meaningful name. Delegates to
/// [`pipeline::run`].
///
/// This is the raw `ParsedName`/error path — informal names come back as `Ok(ParsedName)` with
/// `type == INFORMAL`, NOT split off. It backs the golden snapshot, the FFI/CLI/R encoders (which
/// need the full `ParsedName` and derive the three-way at their own boundary), and any caller that
/// wants the structured atoms directly. **Most callers want [`parse`] instead** — the primary 5.0.0
/// exceptionless entry point that returns the three-way [`ParseResult`].
pub fn parse_name(
    name: &str,
    authorship: Option<&str>,
    rank: Option<Rank>,
    code: Option<NomCode>,
) -> Result<ParsedName, ParseError> {
    pipeline::run(name, authorship, rank, code)
}

/// The 5.0.0 exceptionless result of [`parse`], mirroring Java
/// `org.gbif.nameparser.api.ParseResult`: a structurally [`Parsed`](ParseResult::Parsed) name, an
/// [`Informal`](ParseResult::Informal) semistructured name (a taxon anchor carrying a provisional
/// designation, e.g. `Rhizobium sp. RMCC TR1811`), or an [`Unparsable`](ParseResult::Unparsable)
/// classification (virus, hybrid formula, placeholder, BOLD BIN, ...). The 5.5%-of-corpus informal
/// band is split off from `Parsed` at the [`parse`] boundary — see the verbatim-corpus study
/// in `docs/superpowers/findings/`.
///
/// `Parsed` is far larger than the other two variants (it wraps the ~1 KB [`ParsedName`] vs ~56 B
/// for [`Informal`]/[`ParseError`]), so `clippy::large_enum_variant` fires — but `Parsed` is the
/// dominant outcome (≈89% of the corpus) and the value is decomposed immediately at the call
/// boundary, so boxing it would add a heap allocation to the common path for no real gain. This
/// mirrors [`parse_name`]'s own `Result<ParsedName, ParseError>`, which carries the identical size
/// profile unboxed.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseResult {
    /// A structurally parsed name carrying the full [`ParsedName`] atoms.
    Parsed(ParsedName),
    /// An informal / semistructured name: a supraspecific anchor + a provisional, non-code
    /// designation, with no species epithet. See [`Informal`].
    Informal(Informal),
    /// A string that is not a parsable scientific name (virus, hybrid formula, placeholder, ...).
    Unparsable(ParseError),
}

impl ParseResult {
    /// The name type, available on every variant (Java `ParseResult.type()`).
    pub fn type_(&self) -> NameType {
        match self {
            ParseResult::Parsed(pn) => pn.type_,
            ParseResult::Informal(_) => NameType::Informal,
            ParseResult::Unparsable(e) => e.type_,
        }
    }

    /// The nomenclatural code when known, else `None` — available on every variant (Java `code()`).
    pub fn code(&self) -> Option<NomCode> {
        match self {
            ParseResult::Parsed(pn) => pn.code,
            ParseResult::Informal(i) => i.code,
            ParseResult::Unparsable(e) => e.code,
        }
    }

    /// The parsed name if this is [`ParseResult::Parsed`], else `None` (Java `parsed()`).
    /// [`ParseResult::Informal`] and [`ParseResult::Unparsable`] carry no [`ParsedName`].
    pub fn parsed(&self) -> Option<&ParsedName> {
        match self {
            ParseResult::Parsed(pn) => Some(pn),
            _ => None,
        }
    }

    /// True iff this carries a structured [`ParsedName`] — i.e. is a [`ParseResult::Parsed`]
    /// (Java `isParsable()`).
    pub fn is_parsable(&self) -> bool {
        matches!(self, ParseResult::Parsed(_))
    }
}

/// The primary 5.0.0 exceptionless entry point (mirrors Java `NameParser.parse()`): classifies a
/// scientific name into the three-way [`ParseResult`] (`Parsed | Informal | Unparsable`) that every
/// binding exposes, never throwing. The informal split is applied here — a supraspecific taxon
/// carrying a provisional designation with NO species epithet becomes [`ParseResult::Informal`]; a
/// name with a species epithet (including cf./aff. and infraspecific-indeterminate binomials) stays
/// [`ParseResult::Parsed`], so its `specific_authorship` — which a flat anchor could not represent —
/// is preserved. For the raw `ParsedName`/error shape (no informal split), use [`parse_name`].
pub fn parse(
    name: &str,
    authorship: Option<&str>,
    rank: Option<Rank>,
    code: Option<NomCode>,
) -> ParseResult {
    match pipeline::run(name, authorship, rank, code) {
        Ok(pn) if is_informal(&pn) => ParseResult::Informal(to_informal(pn)),
        Ok(pn) => ParseResult::Parsed(pn),
        // `Unparsable` may only carry a non-parsable type (mirrors Java); the core's error path
        // can still tag an informal-but-unrepresentable grouping as INFORMAL — clamp it to OTHER.
        Err(e) => ParseResult::Unparsable(e.clamped_to_unparsable()),
    }
}

/// The informal discriminator (settled via the 67.5M verbatim-corpus study): an `INFORMAL`-typed
/// name with a real supraspecific anchor but NO species epithet is an [`ParseResult::Informal`];
/// everything else parsable stays `Parsed`. Keying on `specific_epithet` routes cf./aff. and
/// infraspecific-indeterminate binomials to `Parsed` automatically (they keep a species epithet),
/// so their `epithet_qualifier` annotation and `specific_authorship` survive. The anchor guard keeps
/// a degenerate anchor-less `INFORMAL` (should not occur on the `Ok` path) out of `Informal`.
fn is_informal(pn: &ParsedName) -> bool {
    pn.type_ == NameType::Informal
        && pn.specific_epithet.is_none()
        && (pn.genus.is_some() || pn.uninomial.is_some() || pn.infrageneric_epithet.is_some())
}

/// Derive the flat [`Informal`] anchor from an informal [`ParsedName`]. The anchor sits in the
/// `genus` slot for the overwhelming `Genus sp. <tag>` majority (→ `taxon_rank = GENUS`, even when
/// the word is really a family the unvalidated parser cannot know that); a bare supraspecific
/// monomial falls back to `uninomial` (then `infrageneric`) at the name's own rank. Precondition:
/// [`is_informal`] held, so at least one anchor slot is populated.
fn to_informal(pn: ParsedName) -> Informal {
    let (taxon, taxon_rank) = if let Some(g) = pn.genus.as_deref() {
        (g.to_string(), Rank::Genus)
    } else if let Some(u) = pn.uninomial.as_deref() {
        (u.to_string(), pn.rank)
    } else {
        (pn.infrageneric_epithet.clone().unwrap_or_default(), pn.rank)
    };
    Informal {
        taxon,
        taxon_rank,
        rank: pn.rank,
        phrase: pn.phrase,
        code: pn.code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::NameType;

    #[test]
    fn parse_delegates_to_pipeline_run() {
        let pn = parse_name("Abies alba", None, None, None).expect("should parse");
        assert_eq!(pn.type_, NameType::Scientific);
    }

    #[test]
    fn parse_rejects_empty_input() {
        let err = parse_name("", None, None, None).unwrap_err();
        assert_eq!(err.type_, NameType::Other);
    }

    /// The three-way [`parse`] classifier + the flat [`Informal`] derivation, on cases whose
    /// outcome is independent of the (separate) trailing-tag-capture parser change: an already-captured
    /// phrase (`Serratia sp. RE1-2a`), a bare `Genus sp.`, and a numbered placeholder route to
    /// `Informal`; a binomial core (species epithet present) and a bare determined genus stay `Parsed`;
    /// empty input is `Unparsable`.
    #[test]
    fn parse_classifies_three_way() {
        use model::Rank;

        // --- Informal: supraspecific anchor + provisional designation, no species epithet ---
        match parse("Serratia sp. RE1-2a", None, None, None) {
            ParseResult::Informal(i) => {
                assert_eq!(i.taxon, "Serratia");
                assert_eq!(i.taxon_rank, Rank::Genus);
                assert_eq!(i.rank, Rank::Species);
                assert_eq!(i.phrase.as_deref(), Some("RE1-2a"));
            }
            other => panic!("expected Informal, got {other:?}"),
        }
        // bare "Genus sp." — no tag to capture
        match parse("Rhizobium sp.", None, None, None) {
            ParseResult::Informal(i) => {
                assert_eq!(i.taxon, "Rhizobium");
                assert_eq!(i.taxon_rank, Rank::Genus);
                assert_eq!(i.rank, Rank::Species);
                assert_eq!(i.phrase, None);
            }
            other => panic!("expected Informal, got {other:?}"),
        }
        // numbered placeholder
        match parse("Allium sp. 1", None, None, None) {
            ParseResult::Informal(i) => {
                assert_eq!(i.taxon, "Allium");
                assert_eq!(i.phrase.as_deref(), Some("1"));
            }
            other => panic!("expected Informal, got {other:?}"),
        }

        // --- Parsed: a species epithet is present (binomial core), so it is NOT Informal ---
        // infraspecific-indeterminate — authorship would land in specific_authorship, unrepresentable flat
        assert!(
            matches!(
                parse("Salix alba subsp. B", None, None, None),
                ParseResult::Parsed(_)
            ),
            "infraspecific-indet binomial must stay Parsed"
        );
        // cf. qualifier on a complete binomial — the qualifier is an annotation, not a reclassification
        match parse("Salicornia cf. patula", None, None, None) {
            ParseResult::Parsed(pn) => assert_eq!(pn.specific_epithet.as_deref(), Some("patula")),
            other => panic!("expected Parsed, got {other:?}"),
        }
        // a plain determined binomial and a bare determined genus
        assert!(parse("Abies alba", None, None, None).is_parsable());
        assert!(parse("Rhizobium", None, None, None).is_parsable());

        // --- Unparsable ---
        match parse("", None, None, None) {
            ParseResult::Unparsable(e) => assert_eq!(e.type_, NameType::Other),
            other => panic!("expected Unparsable, got {other:?}"),
        }
    }

    /// The 5.0.0 boundary clamp — a defensive invariant that `Unparsable` may only carry a
    /// non-parsable type. Since the preflight informal-group rescue landed, NO real input produces an
    /// error:INFORMAL anymore (anchored groupings are rescued to `Informal`; the rest emit OTHER —
    /// see `pipeline::preflight`), so this exercises the clamp DIRECTLY on a synthetic `ParseError`,
    /// then confirms an anchorless clade label is `Unparsable(OTHER)` end to end.
    #[test]
    fn clamped_to_unparsable_forces_a_nonparsable_type() {
        use model::{NomCode, ParseError};

        // a parsable error type is forced to OTHER; code preserved, message rebuilt for the new type
        let clamped = ParseError::new(NameType::Informal, Some(NomCode::Bacterial), "X group")
            .clamped_to_unparsable();
        assert_eq!(clamped.type_, NameType::Other);
        assert!(!clamped.type_.is_parsable());
        assert_eq!(clamped.code, Some(NomCode::Bacterial));
        assert_eq!(clamped.message, "Unparsable OTHER name: X group");

        // a non-parsable error is left untouched
        let other = ParseError::new(NameType::Other, None, "junk");
        assert_eq!(other.clone().clamped_to_unparsable(), other);

        // end to end: an anchorless clade label is Unparsable(OTHER)
        match parse("Amauropeltoid clade", None, None, None) {
            ParseResult::Unparsable(e) => assert_eq!(e.type_, NameType::Other),
            other => panic!("expected Unparsable(OTHER), got {other:?}"),
        }
    }
}
