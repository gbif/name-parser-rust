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
// referenced by Preflight/ViralSuffix/the skeleton + Unranked default, Species and
// Subspecies (needed for the Task 2 wire-format golden-reference tests to be byte-exact —
// the Java CLI oracle over "Abies alba Mill." / "Vulpes vulpes silaceus Miller, 1907"
// reports rank SPECIES / SUBSPECIES respectively), plus Genus/Family/Variety/Form (set by
// `StripAndStash::strip_nom_note`'s NOM_NOTE_RANK_HINT switch, StripAndStash.java:1184-1190)
// and Cultivar/CultivarGroup/Grex (set by `strip_quoted_cultivar`/`strip_cultivar_group_grex`,
// StripAndStash.java:998/1018/1029/1061 — Phase 1 Slice 2 batch 2b), exist for now. `rank` is
// not itself one of this slice's gated downstream-independent fields (it's a shared field
// also written by later, not-yet-ported stages), but these steps set it as a faithful,
// observable side effect, so the variants they need are added rather than stubbed out.
//
// Phase 1 Slice 2 batch 2e (steps 53-55, `StripAndStash.java` line numbers) adds 18 more:
//   - Subfamily/Tribe/Subtribe/Supertribe/Infratribe: `SUPRA_RANK_MARKERS` (1693-1701),
//     step 53 `stripSupraRankPrefix`.
//   - Subgenus/SectionBotany/SubsectionBotany/SupersectionBotany/SeriesBotany/
//     SubseriesBotany: the `RankUtils.RANK_MARKER_MAP_INFRAGENERIC` (RankUtils.java:90-106)
//     SUBSET reachable via `LEADING_INFRAGEN_MARKER`'s 13-word alternation (1716-1719),
//     step 54 `stripLeadingInfragenericMarker`.
//   - SectionZoology/SubsectionZoology/SupersectionZoology/SeriesZoology/SubseriesZoology/
//     SuperseriesZoology/SuperseriesBotany: `BOT_TO_ZOOL` (1706-1712), ported as the
//     complete 6-pair map (step 54's own literal), one pair (Superseries) unreachable via
//     the current regex but kept for faithfulness — see `bot_to_zool`'s own doc comment.
// Step 55 (`stashPhraseName`) needs no new variants: SPECIES/SUBSPECIES/VARIETY/FORM
// already exist above.
/// Java `org.gbif.nameparser.api.Rank` (STUB — see note above).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Rank {
    Unranked,
    Family,
    Genus,
    Species,
    Grex,
    Subspecies,
    CultivarGroup,
    Variety,
    Form,
    Cultivar,
    // ---- Phase 1 Slice 2 batch 2e additions (StripAndStash steps 53-55) ----
    Subfamily,
    Tribe,
    Subtribe,
    Supertribe,
    Infratribe,
    Subgenus,
    SectionBotany,
    SubsectionBotany,
    SupersectionBotany,
    SeriesBotany,
    SubseriesBotany,
    SuperseriesBotany,
    SectionZoology,
    SubsectionZoology,
    SupersectionZoology,
    SeriesZoology,
    SubseriesZoology,
    SuperseriesZoology,
    // ---- Phase 1 Slice 3 Task 1 additions (RankMarkers, `pipeline::rank_markers`) ----
    // Every `Rank` value the two `RankMarkers` maps (`INFRASPECIFIC`/`INFRAGENERIC`,
    // RankMarkers.java:23-90) reference that isn't already present above. Cross-checked
    // one-for-one against each map value in `RankMarkers.java` and against `Rank.java`'s own
    // constructor argument for each (which nomenclatural code, if any, it anchors to — see
    // `code()` below).
    //   - `RankMarkers.INFRASPECIFIC` values not already present: Other (the
    //     `LETTER_SUBDIVISION` synthetic marker's unmappable rank), Subvariety, Subform,
    //     Pathovar, Biovar, Chemoform, Serovar, Morph, Morphovar, Phagovar, Natio, Mutatio,
    //     Convariety, Proles, Aberration, Strain, InfraspecificName.
    //   - `RankMarkers.INFRAGENERIC` values not already present: DivisionBotany (the
    //     botanical "div."/"divisio" marker — distinct from the zoological division rank,
    //     which this stub doesn't carry at all yet).
    Other,
    Subvariety,
    Subform,
    Pathovar,
    Biovar,
    Chemoform,
    Serovar,
    Morph,
    Morphovar,
    Phagovar,
    Natio,
    Mutatio,
    Convariety,
    Proles,
    Aberration,
    Strain,
    InfraspecificName,
    DivisionBotany,
    // ---- Phase 1 Slice 3 Task 3 additions (NameTokens) ----
    // Three more `Rank` constants `NameTokens.classify` references by name that weren't
    // needed by Task 1's `RankMarkers` maps (those two ARE marker-table values; these
    // three are used as bare `Rank.XXX` literals in `NameTokens.java`'s own control flow):
    //   - FormaSpecialis (`Rank.FORMA_SPECIALIS`, code BOTANICAL, marker "f.sp.") — the
    //     microbial "f. sp." -> forma specialis special-case (NameTokens.java:348).
    //   - InfrasubspecificName (`Rank.INFRASUBSPECIFIC_NAME`, no code, marker
    //     "infrasubsp.") — assigned when 3+ lower epithets exist with no rank marker
    //     (NameTokens.java:466); NOT the same constant as `InfraspecificName` above
    //     (`Rank.INFRASPECIFIC_NAME`, "used for any unspecific rank below species") — Java
    //     keeps these as two distinct enum constants for "below species" vs "below
    //     subspecies".
    //   - InfragenericName (`Rank.INFRAGENERIC_NAME`, no code, marker "infragen.") — the
    //     default rank for a paren-based subgenus with no explicit rank marker
    //     (NameTokens.java:501); also the rank this crate's `Rank` stub was still missing
    //     for `authorship_split::rank_is_infrageneric_strictly`'s own documented gap (see
    //     that function's doc comment, which explicitly flagged `InfragenericName` as one
    //     of the two not-yet-added ranks in its Java-range list).
    FormaSpecialis,
    InfrasubspecificName,
    InfragenericName,
}

impl Rank {
    /// Java `Rank.getCode()` (`Rank.java:472-474`): the single nomenclatural code a rank
    /// constant is anchored to (set via that constant's own constructor argument), or
    /// `None` for a code-agnostic rank — most of them (`Rank`'s own class doc: "The ranks
    /// listed are code agnostic" unless stated otherwise). Exhaustive `match`, not a
    /// wildcard fallback, over every variant this STUB enum currently has: a future slice
    /// adding more `Rank` variants will get a compile error here until it decides each new
    /// variant's code explicitly, rather than silently defaulting it to `None`.
    ///
    /// Used by `StripAndStash::strip_leading_infrageneric_marker` (step 54, batch 2e) to
    /// backfill `ParsedName::code` from a recognised infrageneric rank marker when the
    /// caller supplied none (Java: `if (r.getCode() != null && ctx.name.getCode() == null)
    /// ctx.name.setCode(r.getCode());`).
    pub fn code(&self) -> Option<NomCode> {
        match self {
            Rank::Grex | Rank::CultivarGroup | Rank::Cultivar | Rank::Convariety => {
                Some(NomCode::Cultivars)
            }
            Rank::SectionBotany
            | Rank::SubsectionBotany
            | Rank::SupersectionBotany
            | Rank::SeriesBotany
            | Rank::SubseriesBotany
            | Rank::SuperseriesBotany
            | Rank::DivisionBotany
            | Rank::Proles => Some(NomCode::Botanical),
            Rank::SectionZoology
            | Rank::SubsectionZoology
            | Rank::SupersectionZoology
            | Rank::SeriesZoology
            | Rank::SubseriesZoology
            | Rank::SuperseriesZoology
            | Rank::Morph
            | Rank::Natio
            | Rank::Mutatio
            | Rank::Aberration => Some(NomCode::Zoological),
            // New Phase 1 Slice 3 Task 1 group: the microbial (bacteriological-code)
            // infrasubspecific ranks — `Rank.java`'s own `NomCode.BACTERIAL` constructor arg.
            Rank::Pathovar
            | Rank::Biovar
            | Rank::Chemoform
            | Rank::Serovar
            | Rank::Morphovar
            | Rank::Phagovar => Some(NomCode::Bacterial),
            // Phase 1 Slice 3 Task 3: `Rank.FORMA_SPECIALIS(NomCode.BOTANICAL, "f.sp.")` —
            // joins the `*Botany` section-series group above.
            Rank::FormaSpecialis => Some(NomCode::Botanical),
            Rank::Unranked
            | Rank::Family
            | Rank::Genus
            | Rank::Species
            | Rank::Subspecies
            | Rank::Variety
            | Rank::Form
            | Rank::Subfamily
            | Rank::Tribe
            | Rank::Subtribe
            | Rank::Supertribe
            | Rank::Infratribe
            | Rank::Subgenus
            | Rank::Other
            | Rank::Subvariety
            | Rank::Subform
            | Rank::Strain
            | Rank::InfraspecificName
            // Phase 1 Slice 3 Task 3: `INFRASUBSPECIFIC_NAME`/`INFRAGENERIC_NAME` both use
            // Java's no-code, marker-only constructor (`Rank(String marker)`) — code-agnostic,
            // same as their `InfraspecificName`/`Other` siblings just above.
            | Rank::InfrasubspecificName
            | Rank::InfragenericName => None,
        }
    }
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

    /// Phase 1 Slice 2 batch 2b additions: `StripAndStash::strip_nom_note`'s rank-hint
    /// switch (Genus/Family/Variety/Form) and `strip_cultivar_group_grex`/
    /// `strip_quoted_cultivar` (Cultivar/CultivarGroup/Grex) — verifies the
    /// `SCREAMING_SNAKE_CASE` rename produces the exact Java enum constant name for each,
    /// in particular that `CultivarGroup`'s two-word boundary renders as the
    /// underscore-joined `CULTIVAR_GROUP` Java uses (`Rank.CULTIVAR_GROUP`).
    #[test]
    fn rank_stub_batch_2b_variants() {
        assert_eq!(serde_json::to_string(&Rank::Genus).unwrap(), "\"GENUS\"");
        assert_eq!(serde_json::to_string(&Rank::Family).unwrap(), "\"FAMILY\"");
        assert_eq!(
            serde_json::to_string(&Rank::Variety).unwrap(),
            "\"VARIETY\""
        );
        assert_eq!(serde_json::to_string(&Rank::Form).unwrap(), "\"FORM\"");
        assert_eq!(
            serde_json::to_string(&Rank::Cultivar).unwrap(),
            "\"CULTIVAR\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::CultivarGroup).unwrap(),
            "\"CULTIVAR_GROUP\""
        );
        assert_eq!(serde_json::to_string(&Rank::Grex).unwrap(), "\"GREX\"");
    }

    /// Phase 1 Slice 2 batch 2e additions: `StripAndStash` steps 53-55's suprageneric
    /// family-group ranks (step 53) and the infrageneric botany/zoology section-series
    /// pairs (step 54's `RANK_MARKER_MAP_INFRAGENERIC` subset + `BOT_TO_ZOOL`) — verifies
    /// the `SCREAMING_SNAKE_CASE` rename produces the exact Java enum constant name for
    /// each, in particular that the two-word `*Botany`/`*Zoology` variants render with the
    /// underscore Java uses (`Rank.SECTION_BOTANY`, `Rank.SUPERSERIES_ZOOLOGY`).
    #[test]
    fn rank_stub_batch_2e_variants() {
        assert_eq!(
            serde_json::to_string(&Rank::Subfamily).unwrap(),
            "\"SUBFAMILY\""
        );
        assert_eq!(serde_json::to_string(&Rank::Tribe).unwrap(), "\"TRIBE\"");
        assert_eq!(
            serde_json::to_string(&Rank::Subtribe).unwrap(),
            "\"SUBTRIBE\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::Supertribe).unwrap(),
            "\"SUPERTRIBE\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::Infratribe).unwrap(),
            "\"INFRATRIBE\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::Subgenus).unwrap(),
            "\"SUBGENUS\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::SectionBotany).unwrap(),
            "\"SECTION_BOTANY\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::SubsectionBotany).unwrap(),
            "\"SUBSECTION_BOTANY\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::SupersectionBotany).unwrap(),
            "\"SUPERSECTION_BOTANY\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::SeriesBotany).unwrap(),
            "\"SERIES_BOTANY\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::SubseriesBotany).unwrap(),
            "\"SUBSERIES_BOTANY\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::SuperseriesBotany).unwrap(),
            "\"SUPERSERIES_BOTANY\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::SectionZoology).unwrap(),
            "\"SECTION_ZOOLOGY\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::SubsectionZoology).unwrap(),
            "\"SUBSECTION_ZOOLOGY\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::SupersectionZoology).unwrap(),
            "\"SUPERSECTION_ZOOLOGY\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::SeriesZoology).unwrap(),
            "\"SERIES_ZOOLOGY\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::SubseriesZoology).unwrap(),
            "\"SUBSERIES_ZOOLOGY\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::SuperseriesZoology).unwrap(),
            "\"SUPERSERIES_ZOOLOGY\""
        );
    }

    /// Phase 1 Slice 3 Task 1 additions: every `Rank` value the two `RankMarkers` maps
    /// (`pipeline::rank_markers`) reference that wasn't already present — verifies the
    /// `SCREAMING_SNAKE_CASE` rename produces the exact Java enum constant name for each,
    /// in particular the two-word `InfraspecificName`/`DivisionBotany` (`INFRASPECIFIC_NAME`
    /// / `DIVISION_BOTANY`, matching Java's own multi-word constants).
    #[test]
    fn rank_stub_slice3_task1_variants() {
        assert_eq!(serde_json::to_string(&Rank::Other).unwrap(), "\"OTHER\"");
        assert_eq!(
            serde_json::to_string(&Rank::Subvariety).unwrap(),
            "\"SUBVARIETY\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::Subform).unwrap(),
            "\"SUBFORM\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::Pathovar).unwrap(),
            "\"PATHOVAR\""
        );
        assert_eq!(serde_json::to_string(&Rank::Biovar).unwrap(), "\"BIOVAR\"");
        assert_eq!(
            serde_json::to_string(&Rank::Chemoform).unwrap(),
            "\"CHEMOFORM\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::Serovar).unwrap(),
            "\"SEROVAR\""
        );
        assert_eq!(serde_json::to_string(&Rank::Morph).unwrap(), "\"MORPH\"");
        assert_eq!(
            serde_json::to_string(&Rank::Morphovar).unwrap(),
            "\"MORPHOVAR\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::Phagovar).unwrap(),
            "\"PHAGOVAR\""
        );
        assert_eq!(serde_json::to_string(&Rank::Natio).unwrap(), "\"NATIO\"");
        assert_eq!(
            serde_json::to_string(&Rank::Mutatio).unwrap(),
            "\"MUTATIO\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::Convariety).unwrap(),
            "\"CONVARIETY\""
        );
        assert_eq!(serde_json::to_string(&Rank::Proles).unwrap(), "\"PROLES\"");
        assert_eq!(
            serde_json::to_string(&Rank::Aberration).unwrap(),
            "\"ABERRATION\""
        );
        assert_eq!(serde_json::to_string(&Rank::Strain).unwrap(), "\"STRAIN\"");
        assert_eq!(
            serde_json::to_string(&Rank::InfraspecificName).unwrap(),
            "\"INFRASPECIFIC_NAME\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::DivisionBotany).unwrap(),
            "\"DIVISION_BOTANY\""
        );
    }

    /// Phase 1 Slice 3 Task 3 additions (`NameTokens`): the three bare `Rank.XXX` literals
    /// `NameTokens.classify` references that Task 1's `RankMarkers` maps didn't already pull
    /// in — verifies the `SCREAMING_SNAKE_CASE` rename produces the exact Java enum constant
    /// name for each, in particular that `FormaSpecialis`'s two-word boundary renders as the
    /// underscore-joined `FORMA_SPECIALIS` Java uses.
    #[test]
    fn rank_stub_slice3_task3_variants() {
        assert_eq!(
            serde_json::to_string(&Rank::FormaSpecialis).unwrap(),
            "\"FORMA_SPECIALIS\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::InfrasubspecificName).unwrap(),
            "\"INFRASUBSPECIFIC_NAME\""
        );
        assert_eq!(
            serde_json::to_string(&Rank::InfragenericName).unwrap(),
            "\"INFRAGENERIC_NAME\""
        );
    }

    /// `Rank::code()` — spot-checks against `Rank.java`'s own constructor arguments (see
    /// `code()`'s doc comment for the exact source lines): most ranks are code-agnostic
    /// (`None`); `Grex`/`CultivarGroup`/`Cultivar` anchor to `CULTIVARS`; the `*Botany` /
    /// `*Zoology` section-series pairs anchor to `BOTANICAL` / `ZOOLOGICAL` respectively;
    /// `Subgenus` (unlike its `*Botany` siblings) carries no code at all in Java.
    #[test]
    fn rank_code_matches_java_get_code() {
        assert_eq!(Rank::Unranked.code(), None);
        assert_eq!(Rank::Species.code(), None);
        assert_eq!(Rank::Subgenus.code(), None);
        assert_eq!(Rank::Tribe.code(), None);
        assert_eq!(Rank::Grex.code(), Some(NomCode::Cultivars));
        assert_eq!(Rank::CultivarGroup.code(), Some(NomCode::Cultivars));
        assert_eq!(Rank::Cultivar.code(), Some(NomCode::Cultivars));
        assert_eq!(Rank::SectionBotany.code(), Some(NomCode::Botanical));
        assert_eq!(Rank::SubsectionBotany.code(), Some(NomCode::Botanical));
        assert_eq!(Rank::SupersectionBotany.code(), Some(NomCode::Botanical));
        assert_eq!(Rank::SeriesBotany.code(), Some(NomCode::Botanical));
        assert_eq!(Rank::SubseriesBotany.code(), Some(NomCode::Botanical));
        assert_eq!(Rank::SuperseriesBotany.code(), Some(NomCode::Botanical));
        assert_eq!(Rank::SectionZoology.code(), Some(NomCode::Zoological));
        assert_eq!(Rank::SubsectionZoology.code(), Some(NomCode::Zoological));
        assert_eq!(Rank::SupersectionZoology.code(), Some(NomCode::Zoological));
        assert_eq!(Rank::SeriesZoology.code(), Some(NomCode::Zoological));
        assert_eq!(Rank::SubseriesZoology.code(), Some(NomCode::Zoological));
        assert_eq!(Rank::SuperseriesZoology.code(), Some(NomCode::Zoological));
    }

    /// `Rank::code()` — Phase 1 Slice 3 Task 1 additions: the new `BACTERIAL` group
    /// (`Pathovar`/`Biovar`/`Chemoform`/`Serovar`/`Morphovar`/`Phagovar`, all microbial
    /// ranks under the Bacteriological Code), the legacy `ZOOLOGICAL` ranks
    /// (`Morph`/`Natio`/`Mutatio`/`Aberration`), `Convariety` (`CULTIVARS`, joining
    /// `Grex`/`CultivarGroup`/`Cultivar`), `Proles`/`DivisionBotany` (`BOTANICAL`, joining
    /// the `*Botany` section-series group), and the code-agnostic stragglers
    /// (`Other`/`Subvariety`/`Subform`/`Strain`/`InfraspecificName`) — each cross-checked
    /// against `Rank.java`'s own constructor argument (see `RankMarkers.java`'s ported
    /// values in `pipeline::rank_markers` for where each of these is actually looked up).
    #[test]
    fn rank_code_matches_java_get_code_slice3_task1_variants() {
        assert_eq!(Rank::Pathovar.code(), Some(NomCode::Bacterial));
        assert_eq!(Rank::Biovar.code(), Some(NomCode::Bacterial));
        assert_eq!(Rank::Chemoform.code(), Some(NomCode::Bacterial));
        assert_eq!(Rank::Serovar.code(), Some(NomCode::Bacterial));
        assert_eq!(Rank::Morphovar.code(), Some(NomCode::Bacterial));
        assert_eq!(Rank::Phagovar.code(), Some(NomCode::Bacterial));
        assert_eq!(Rank::Morph.code(), Some(NomCode::Zoological));
        assert_eq!(Rank::Natio.code(), Some(NomCode::Zoological));
        assert_eq!(Rank::Mutatio.code(), Some(NomCode::Zoological));
        assert_eq!(Rank::Aberration.code(), Some(NomCode::Zoological));
        assert_eq!(Rank::Convariety.code(), Some(NomCode::Cultivars));
        assert_eq!(Rank::Proles.code(), Some(NomCode::Botanical));
        assert_eq!(Rank::DivisionBotany.code(), Some(NomCode::Botanical));
        assert_eq!(Rank::Other.code(), None);
        assert_eq!(Rank::Subvariety.code(), None);
        assert_eq!(Rank::Subform.code(), None);
        assert_eq!(Rank::Strain.code(), None);
        assert_eq!(Rank::InfraspecificName.code(), None);
    }

    /// `Rank::code()` — Phase 1 Slice 3 Task 3 additions: `FormaSpecialis` joins the
    /// `BOTANICAL` group (`Rank.java`'s own `FORMA_SPECIALIS(NomCode.BOTANICAL, "f.sp.")`);
    /// `InfrasubspecificName`/`InfragenericName` are code-agnostic, like their
    /// `InfraspecificName`/`Other` siblings (Java's marker-only, no-code constructor).
    #[test]
    fn rank_code_matches_java_get_code_slice3_task3_variants() {
        assert_eq!(Rank::FormaSpecialis.code(), Some(NomCode::Botanical));
        assert_eq!(Rank::InfrasubspecificName.code(), None);
        assert_eq!(Rank::InfragenericName.code(), None);
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
