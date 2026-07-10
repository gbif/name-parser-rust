// SPDX-License-Identifier: Apache-2.0
use serde::Serialize;

/// Java `org.gbif.nameparser.api.NameType`. A short classification of scientific name strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NameType {
    Scientific,
    Formula,
    Informal,
    Placeholder,
    Other,
}

/// Java `org.gbif.nameparser.api.NomCode`. Nomenclatural codes governing biological taxonomic
/// nomenclature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NomCode {
    Bacterial,
    Botanical,
    Cultivars,
    Phyto,
    Virus,
    Zoological,
    Phylo,
}

/// Java `org.gbif.nameparser.api.NamePart`. Indicates a part of a canonical scientific name.
///
/// Declaration order below matches Java's ordinal order (verified against
/// `NamePart.java`), and carries `PartialOrd, Ord` so it can be used as a `BTreeMap` key
/// for `ParsedName::epithet_qualifier` — Java's own `EnumMap<NamePart, String>` there
/// iterates in the same ordinal order, so a `BTreeMap` reproduces it on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NamePart {
    Generic,
    Infrageneric,
    Specific,
    Infraspecific,
}

/// Java `org.gbif.nameparser.api.ParsedName.State`. Degree of parsing a `ParsedName` reflects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum State {
    Complete,
    Partial,
    None,
}

// STUB: full 117-constant port deferred to the rank-handling slice. Only variants
// referenced by Preflight/ViralSuffix/the skeleton + Unranked default, plus Species and
// Subspecies (needed for the Task 2 wire-format golden-reference tests to be byte-exact —
// the Java CLI oracle over "Abies alba Mill." / "Vulpes vulpes silaceus Miller, 1907"
// reports rank SPECIES / SUBSPECIES respectively), exist for now.
/// Java `org.gbif.nameparser.api.Rank` (STUB — see note above).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Rank {
    Unranked,
    Species,
    Subspecies,
}

/// String constants from `org.gbif.nameparser.api.Warnings`, transcribed verbatim (values, not
/// the Java constant names, though names are kept identical — including `HOMOGLYHPS`, which is
/// a typo in the Java constant name itself, not in its value — so call sites ported later match
/// the Java source 1:1).
pub mod warnings {
    pub const NULL_EPITHET: &str = "epithet with literal value null";
    /// NB: `HOMOGLYHPS` is the Java constant name's own typo (for "homoglyphs"); kept verbatim.
    pub const HOMOGLYHPS: &str = "homoglyphs replaced";
    pub const UNUSUAL_CHARACTERS: &str = "unusual characters";
    pub const SUBSPECIES_ASSIGNED: &str =
        "Name was considered species but contains infraspecific epithet";
    pub const LC_MONOMIAL: &str = "lower case monomial match";
    pub const INDETERMINED: &str = "indetermined name missing its terminal epithet";
    pub const HIGHER_RANK_BINOMIAL: &str = "binomial with rank higher than species aggregate";
    pub const QUESTION_MARKS_REMOVED: &str = "question marks removed";
    pub const REPL_ENCLOSING_QUOTE: &str = "removed enclosing quotes";
    pub const MISSING_GENUS: &str = "epithet without genus";
    pub const DOUBTFUL_GENUS: &str = "genus quoted or in square brackets";
    pub const RANK_MISMATCH: &str = "rank does not fit the parsed name";
    pub const CODE_MISMATCH: &str = "nomenclatural code does not fit the parsed name";
    pub const HTML_ENTITIES: &str = "html entities unescaped";
    pub const XML_TAGS: &str = "xml tags removed";
    pub const BLACKLISTED_EPITHET: &str = "blacklisted epithet used";
    pub const NOMENCLATURAL_REFERENCE: &str = "nomenclatural reference removed";
    pub const AUTHORSHIP_REMOVED: &str = "authorship placeholder removed";
    /// NB: "was extract" (not "was extracted") is verbatim from the Java source.
    pub const YEAR_INTERPRETED: &str =
        "authorship year was extract but originally was a year range or other form of year";
    pub const UNLIKELY_YEAR: &str = "unlikely authorship year";
    pub const UNCERTAIN_AUTHORSHIP: &str = "authorship marked as uncertain";
    pub const QUADRINOMIAL: &str = "name was quadrinomial";
    pub const ABBREVIATED_GENUS: &str = "abbreviated genus name";
    pub const ABBREVIATED_SUBGENUS: &str = "abbreviated subgenus name";
    /// NB: trailing space before the closing quote is verbatim from the Java source.
    pub const REMOVED_PREFIX: &str = "Removed: ";
    pub const LONG_NAME: &str = "unusually long name";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_type_scientific_serializes_uppercase() {
        assert_eq!(
            serde_json::to_string(&NameType::Scientific).unwrap(),
            "\"SCIENTIFIC\""
        );
    }

    #[test]
    fn name_type_first_and_last_variant() {
        assert_eq!(
            serde_json::to_string(&NameType::Scientific).unwrap(),
            "\"SCIENTIFIC\""
        );
        assert_eq!(
            serde_json::to_string(&NameType::Other).unwrap(),
            "\"OTHER\""
        );
    }

    #[test]
    fn nom_code_first_last_and_zoological() {
        assert_eq!(
            serde_json::to_string(&NomCode::Bacterial).unwrap(),
            "\"BACTERIAL\""
        );
        assert_eq!(serde_json::to_string(&NomCode::Phylo).unwrap(), "\"PHYLO\"");
        assert_eq!(
            serde_json::to_string(&NomCode::Zoological).unwrap(),
            "\"ZOOLOGICAL\""
        );
    }

    #[test]
    fn name_part_first_and_last_variant() {
        assert_eq!(
            serde_json::to_string(&NamePart::Generic).unwrap(),
            "\"GENERIC\""
        );
        assert_eq!(
            serde_json::to_string(&NamePart::Infraspecific).unwrap(),
            "\"INFRASPECIFIC\""
        );
    }

    #[test]
    fn state_first_and_last_variant() {
        assert_eq!(
            serde_json::to_string(&State::Complete).unwrap(),
            "\"COMPLETE\""
        );
        assert_eq!(serde_json::to_string(&State::None).unwrap(), "\"NONE\"");
    }

    #[test]
    fn rank_stub_unranked() {
        assert_eq!(
            serde_json::to_string(&Rank::Unranked).unwrap(),
            "\"UNRANKED\""
        );
    }

    #[test]
    fn rank_stub_species_and_subspecies() {
        assert_eq!(
            serde_json::to_string(&Rank::Species).unwrap(),
            "\"SPECIES\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::Subspecies).unwrap(),
            "\"SUBSPECIES\""
        );
    }

    #[test]
    fn name_part_ord_matches_java_ordinal_order() {
        let mut parts = vec![
            NamePart::Infraspecific,
            NamePart::Generic,
            NamePart::Specific,
            NamePart::Infrageneric,
        ];
        parts.sort();
        assert_eq!(
            parts,
            vec![
                NamePart::Generic,
                NamePart::Infrageneric,
                NamePart::Specific,
                NamePart::Infraspecific,
            ]
        );
    }
}
