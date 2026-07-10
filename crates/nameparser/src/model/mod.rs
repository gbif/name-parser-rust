// SPDX-License-Identifier: Apache-2.0
pub mod enums;
pub use enums::*;

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
}
