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

use model::{NameType, NomCode, ParseError, ParsedName, Rank};

/// The lower-level raw parse: a scientific name — optionally alongside a separately supplied
/// authorship string, a requested [`Rank`] and a requested [`NomCode`] — into a [`ParsedName`], or
/// `Err(`[`ParseError`]`)` when the input cannot be parsed into a meaningful name. Delegates to
/// [`pipeline::run`].
///
/// This is the raw `ParsedName`/error path. It backs the golden snapshot, the FFI/CLI/R encoders
/// and any caller that wants the structured atoms of every input, placeholders without a genus
/// included. **Most callers want [`parse`] instead** — the exceptionless entry point that returns a
/// [`ParseResult`].
pub fn parse_name(
    name: &str,
    authorship: Option<&str>,
    rank: Option<Rank>,
    code: Option<NomCode>,
) -> Result<ParsedName, ParseError> {
    pipeline::run(name, authorship, rank, code)
}

/// The exceptionless result of [`parse`], mirroring Java `org.gbif.nameparser.api.ParseResult`: a
/// structurally [`Parsed`](ParseResult::Parsed) name or an [`Unparsable`](ParseResult::Unparsable)
/// classification (virus, hybrid formula, placeholder, BOLD BIN, ...). Informal names — provisional,
/// open-nomenclature and indetermined ones like `Rhizobium sp. RMCC TR1811` or `Abies cf. alba` —
/// are `Parsed` names of type [`NameType::Informal`]; [`ParsedName::anchor`] gives the real taxon
/// they hang off.
///
/// `Parsed` is far larger than `Unparsable` (it wraps the ~1 KB [`ParsedName`] vs ~56 B for a
/// [`ParseError`]), so `clippy::large_enum_variant` fires — but `Parsed` is the dominant outcome
/// (≈95% of the corpus) and the value is decomposed immediately at the call boundary, so boxing it
/// would add a heap allocation to the common path for no real gain. This mirrors [`parse_name`]'s
/// own `Result<ParsedName, ParseError>`, which carries the identical size profile unboxed.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseResult {
    /// A structurally parsed name carrying the full [`ParsedName`] atoms, informal names included.
    Parsed(ParsedName),
    /// A string that is not a parsable scientific name (virus, hybrid formula, placeholder, ...).
    Unparsable(ParseError),
}

impl ParseResult {
    /// The name type, available on both variants (Java `ParseResult.type()`).
    pub fn type_(&self) -> NameType {
        match self {
            ParseResult::Parsed(pn) => pn.type_,
            ParseResult::Unparsable(e) => e.type_,
        }
    }

    /// The nomenclatural code when known, else `None` — available on both variants (Java `code()`).
    pub fn code(&self) -> Option<NomCode> {
        match self {
            ParseResult::Parsed(pn) => pn.code,
            ParseResult::Unparsable(e) => e.code,
        }
    }

    /// The parsed name if this is [`ParseResult::Parsed`], else `None` (Java `parsed()`).
    /// [`ParseResult::Unparsable`] carries no [`ParsedName`].
    pub fn parsed(&self) -> Option<&ParsedName> {
        match self {
            ParseResult::Parsed(pn) => Some(pn),
            ParseResult::Unparsable(_) => None,
        }
    }

    /// True iff this carries a structured [`ParsedName`] — i.e. is a [`ParseResult::Parsed`]
    /// (Java `isParsable()`).
    pub fn is_parsable(&self) -> bool {
        matches!(self, ParseResult::Parsed(_))
    }
}

/// The primary exceptionless entry point (mirrors Java `NameParser.parse()`): classifies a
/// scientific name into the [`ParseResult`] (`Parsed | Unparsable`) that every binding exposes,
/// never throwing. Informal names are `Parsed`, typed [`NameType::Informal`]. For the raw
/// `ParsedName`/error shape, use [`parse_name`].
pub fn parse(
    name: &str,
    authorship: Option<&str>,
    rank: Option<Rank>,
    code: Option<NomCode>,
) -> ParseResult {
    match pipeline::run(name, authorship, rank, code) {
        // `Parsed` may only carry a parsable type: a placeholder with its genus missing ("? alba
        // Smith", "Missing penchinati Bourguignat, 1870") is reported unparsable, like any other
        // placeholder; `parse_name` still returns its parts.
        Ok(pn) if !pn.type_.is_parsable() => {
            ParseResult::Unparsable(ParseError::new(pn.type_, pn.code, name))
        }
        Ok(pn) => ParseResult::Parsed(pn),
        // `Unparsable` may only carry a non-parsable type (mirrors Java); the core's error path
        // can still tag an informal-but-unrepresentable grouping as INFORMAL — clamp it to OTHER.
        Err(e) => ParseResult::Unparsable(e.clamped_to_unparsable()),
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

    /// [`parse`] returns informal names as `Parsed` names of type INFORMAL, their anchor derived on
    /// demand; empty input is `Unparsable`.
    #[test]
    fn parse_returns_informal_names_parsed() {
        use model::Rank;

        for (input, anchor, anchor_rank, phrase) in [
            (
                "Serratia sp. RE1-2a",
                "Serratia",
                Rank::Genus,
                Some("sp. RE1-2a"),
            ),
            // bare "Genus sp." — no distinguishing tag, but the verbatim marker is still the phrase
            ("Rhizobium sp.", "Rhizobium", Rank::Genus, Some("sp.")),
            ("Allium sp. 1", "Allium", Rank::Genus, Some("sp. 1")),
            // a qualified epithet is no part of the anchor
            ("Salicornia cf. patula", "Salicornia", Rank::Genus, None),
            (
                "Salix alba subsp. B",
                "Salix alba subsp. B",
                Rank::Subspecies,
                None,
            ),
        ] {
            match parse(input, None, None, None) {
                ParseResult::Parsed(pn) => {
                    assert_eq!(pn.type_, NameType::Informal, "{input}");
                    assert_eq!(pn.phrase.as_deref(), phrase, "{input}");
                    let a = pn.anchor().expect("an anchor");
                    assert_eq!(
                        a.canonical_name_without_authorship().as_deref(),
                        Some(anchor),
                        "{input}"
                    );
                    assert_eq!(a.rank, anchor_rank, "{input}");
                }
                other => panic!("expected Parsed, got {other:?}"),
            }
        }
        // a plain determined binomial and a bare determined genus
        assert!(parse("Abies alba", None, None, None).is_parsable());
        assert!(parse("Rhizobium", None, None, None).is_parsable());

        match parse("", None, None, None) {
            ParseResult::Unparsable(e) => assert_eq!(e.type_, NameType::Other),
            other => panic!("expected Unparsable, got {other:?}"),
        }
    }

    /// The boundary clamp — a defensive invariant that `Unparsable` may only carry a non-parsable
    /// type. Since the preflight informal-group rescue landed, NO real input produces an
    /// error:INFORMAL anymore (anchored groupings are rescued to an INFORMAL name; the rest emit
    /// OTHER — see `pipeline::preflight`), so this exercises the clamp DIRECTLY on a synthetic
    /// `ParseError`, then confirms an anchorless clade label is `Unparsable(OTHER)` end to end.
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
