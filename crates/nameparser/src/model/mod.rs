// SPDX-License-Identifier: Apache-2.0
pub mod enums;
pub use enums::*;
pub mod name;
pub use name::*;

/// Java `Enum.name()` — the exact SCREAMING_SNAKE_CASE identifier (e.g. `OTHER`), as opposed to
/// Rust's `{:?}` (e.g. `Other`). Reuses each wire enum's own `Serialize` impl — which already
/// carries `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` — rather than a hand-maintained match,
/// so the two representations cannot drift apart.
fn java_name<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .expect("wire enums always serialize to a JSON string")
        .trim_matches('"')
        .to_string()
}

/// Rust equivalent of Java UnparsableNameException (type/code/name/message).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub type_: NameType,
    pub code: Option<NomCode>,
    pub name: String,
    pub message: String,
}
impl ParseError {
    pub fn new(type_: NameType, code: Option<NomCode>, name: impl Into<String>) -> Self {
        let name = name.into();
        // Java: `"Unparsable " + type + " name: " + name`, where `type` is a NameType and string
        // concatenation calls its (un-overridden) `Enum.toString()`, i.e. `.name()` — UPPERCASE.
        let message = format!("Unparsable {} name: {name}", java_name(&type_));
        Self {
            type_,
            code,
            name,
            message,
        }
    }

    /// Clamp a parsable [`NameType`] to [`NameType::Other`] so this error is legal as a 5.0.0
    /// `Unparsable` result. The core's error path can still tag an unrepresentable-but-informal
    /// grouping (`"Bartonella group"`, `"Amauropeltoid clade"`) as `INFORMAL`, but the 5.0.0
    /// `ParseResult::Unparsable` variant — mirroring Java's, whose record rejects `isParsable()`
    /// types — may only carry a non-parsable type. Applied ONLY at the 5.0.0 boundaries
    /// ([`crate::parse`] and the FFI); [`crate::parse_name`] (the raw path, cross-validated
    /// byte-for-byte against Java 4.2.0) keeps the original `INFORMAL`, so the golden oracles are
    /// untouched. Rebuilds the message via [`Self::new`] so it names the clamped type.
    pub fn clamped_to_unparsable(self) -> ParseError {
        if self.type_.is_parsable() {
            ParseError::new(NameType::Other, self.code, self.name)
        } else {
            self
        }
    }
}

/// An informal / semistructured name — the payload of [`crate::ParseResult::Informal`], mirroring
/// Java `org.gbif.nameparser.api.ParseResult.Informal`.
///
/// A real supraspecific taxon ([`Self::taxon`], a genus or higher uninomial) carrying a provisional,
/// non-code designation instead of a determined species epithet: a molecular provisional species
/// (`Rhizobium sp. RMCC TR1811`), a numbered placeholder (`Allium sp. 1`), or an informal group.
/// The 67.5M verbatim-corpus study found these are 5.5% of all real
/// names — 1 in 18 — hence a first-class representation.
///
/// Deliberately a FLAT type, not a reused [`ParsedName`]: the anchor lives in one place
/// ([`Self::taxon`] + [`Self::taxon_rank`]) and is never mislabelled as a backbone-validated
/// "genus". Derived from an informal `ParsedName` at the [`crate::parse`] boundary. Only
/// names with NO species epithet land here; a binomial core (incl. cf./aff. and infraspecific-indet)
/// stays [`crate::ParseResult::Parsed`] so its `specific_authorship` is preserved.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Informal {
    /// The supraspecific taxon it hangs off (`"Rhizobium"`, `"Ichneumonidae"`) — the parser's best
    /// guess, NOT validated against a taxonomic backbone.
    pub taxon: String,
    /// That taxon's rank — `GENUS` for the overwhelming `Genus sp.` majority (the anchor sits in the
    /// genus slot); a bare supraspecific monomial carries its own rank. The parser's best guess.
    pub taxon_rank: Rank,
    /// The rank the informal name purports to be — `SPECIES` for `"sp."`, `UNRANKED` for a group.
    pub rank: Rank,
    /// The distinguishing designator (`"RMCC TR1811"`, `"1"`); `None` for a bare `"Genus sp."`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phrase: Option<String>,
    /// The nomenclatural code when known, else `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<NomCode>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_message_matches_java_default() {
        let err = ParseError::new(NameType::Other, None, "BOLD:ACW2100");
        assert_eq!(err.message, "Unparsable OTHER name: BOLD:ACW2100");
    }

    /// `model::{NameType, NomCode, NamePart, State, Rank}` and `model::warnings` are the
    /// documented public interface — reached via the `pub use enums::*;` re-export above, not
    /// only via `model::enums::…`. Exercised here through fully crate-qualified paths so a future
    /// refactor of the re-export can't silently narrow the interface without a test failure.
    #[test]
    fn wire_enums_and_warnings_are_reachable_at_model_top_level() {
        assert_eq!(crate::model::warnings::LONG_NAME, "unusually long name");
        let _: crate::model::NameType = crate::model::NameType::Scientific;
        let _: crate::model::NomCode = crate::model::NomCode::Zoological;
        let _: crate::model::NamePart = crate::model::NamePart::Generic;
        let _: crate::model::State = crate::model::State::Complete;
        let _: crate::model::Rank = crate::model::Rank::Unranked;
    }

    /// `model::{Authorship, CombinedAuthorship, ParsedName}` are likewise reached directly
    /// under `model::`, via the `pub use name::*;` re-export above, not only via
    /// `model::name::…`.
    #[test]
    fn wire_name_structs_are_reachable_at_model_top_level() {
        let _: crate::model::Authorship = crate::model::Authorship::default();
        let _: crate::model::CombinedAuthorship = crate::model::CombinedAuthorship::default();
        let _: crate::model::ParsedName = crate::model::ParsedName::default();
    }
}
