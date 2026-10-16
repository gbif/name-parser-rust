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
