// SPDX-License-Identifier: Apache-2.0
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

/// Java `org.gbif.nameparser.api.NameType`. A short classification of scientific name strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NameType {
    Scientific,
    Formula,
    Informal,
    Placeholder,
    /// An anchorless, scheme-prefixed *machine identifier* — a UNITE species hypothesis
    /// (`SH1957732.10FU`), a BOLD BIN (`BOLD:AAA0001`), an OTU/ASV/… operational unit, or a
    /// standalone culture-collection accession (`DSM 10`). Not a name the parser can express as a
    /// [`crate::model::ParsedName`], so — like `Other` — it is **not parsable** and only ever
    /// appears in a `ParseResult::Unparsable`; it just carries a more specific classification than
    /// the catch-all `Other`. Placed before `Other` so the catch-all stays last; this makes it
    /// FFI-wire ordinal 4 and shifts `Other` to 5 (a lockstep change gated by the ABI-version bump).
    Identifier,
    Other,
}

impl NameType {
    /// Java `NameType.isParsable()` (`NameType.java`): true iff the parser can express such a name
    /// as a [`crate::model::ParsedName`] — `SCIENTIFIC` or `INFORMAL`. The 5.0.0
    /// `ParseResult.Unparsable` variant (Java + Rust) may only carry a non-parsable type, so this
    /// gates the [`crate::ParseError::clamped_to_unparsable`] normalization.
    pub fn is_parsable(&self) -> bool {
        matches!(self, NameType::Scientific | NameType::Informal)
    }
}

/// Java `org.gbif.nameparser.api.NomCode`. Nomenclatural codes governing biological taxonomic
/// nomenclature.
///
/// `Hash` (beyond the `Rank`-parity `Debug, Clone, Copy, PartialEq, Eq, Serialize` set) is
/// needed for `pipeline::code_inference::infer`'s vote tally, a `HashSet<NomCode>` mirroring
/// Java's `EnumSet<NomCode> votes` (`CodeInference.java:66`) — added here (Phase 1 Slice 4
/// Task 3) rather than duplicated as a local newtype, since `Rank` already sets this same
/// precedent (it derives `Hash` for its own `HashMap`/`HashSet` static tables above).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
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

// ================================================================================================
// Rank
// ================================================================================================

/// Java `org.gbif.nameparser.api.Rank` (`Rank.java`, 637 lines) — the full 117-constant
/// taxonomic rank enumeration, ported in EXACT Java declaration order (verified mechanically
/// against `Rank.java` line-by-line, `SUPERDOMAIN` first through `UNRANKED` last — see the
/// exhaustive round-trip test `rank_all_117_variants_in_java_declaration_order` below).
/// Declaration order is load-bearing: Rust numbers enum discriminants 0..117 in declaration
/// order exactly like Java's `Enum.ordinal()`, so [`Rank::ordinal`] and every ordinal-compare
/// predicate below (`is_infraspecific`, `higher_than`, `is_family_group`, …) reproduce Java's
/// behaviour by construction, not by coincidence.
///
/// Each constant carries the same `code`/`marker`/`plural` triple as its Java constructor call
/// — see [`Rank::code`], [`Rank::marker`], [`Rank::plural`]. 67 of the 117 are code-restricted
/// (44 `ZOOLOGICAL`, 10 `BOTANICAL`, 7 `BACTERIAL`, 4 `CULTIVARS`, 2 `VIRUS` — counted by
/// grepping `NomCode\.` inside `Rank.java`'s own constant-declaration block, lines 25-311); the
/// remaining 50 are code-agnostic (`None`).
///
/// `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` reproduces Java's `.name()` wire form for
/// every one of the 117, including multi-word constants like `SUPERSECTION_BOTANY`,
/// `INFRASPECIFIC_NAME` and `FORMA_SPECIALIS` — verified by the exhaustive round-trip test
/// below (`rank_all_117_variants_in_java_declaration_order`), so no `#[serde(rename = "…")]`
/// overrides were needed.
///
/// This is the crate's single source of truth for rank data and predicates: the ordinal
/// predicates here replace the former ad-hoc `rank_is_infrageneric_strictly`
/// (`pipeline::authorship_split`) and `rank_is_infraspecific` / `rank_marker`
/// (`pipeline::name_tokens`) — both were explicit stand-ins for a not-yet-full `Rank`; those
/// two files now call [`Rank::is_infrageneric_strictly`], [`Rank::is_infraspecific`] and
/// [`Rank::marker`] directly instead of carrying their own copies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Rank {
    Superdomain,
    Domain,
    Subdomain,
    Infradomain,
    Empire,
    Realm,
    Subrealm,
    Superkingdom,
    Kingdom,
    Subkingdom,
    Infrakingdom,
    Superphylum,
    Phylum,
    Subphylum,
    Infraphylum,
    Parvphylum,
    Microphylum,
    Nanophylum,
    Claudius,
    Gigaclass,
    Megaclass,
    Superclass,
    Class,
    Subclass,
    Infraclass,
    Subterclass,
    Parvclass,
    Superdivision,
    DivisionZoology,
    Subdivision,
    Infradivision,
    Superlegion,
    Legion,
    Sublegion,
    Infralegion,
    Megacohort,
    Supercohort,
    Cohort,
    Subcohort,
    Infracohort,
    Gigaorder,
    Magnorder,
    Grandorder,
    Mirorder,
    Superorder,
    Order,
    Nanorder,
    Hypoorder,
    Minorder,
    Suborder,
    Infraorder,
    Parvorder,
    SupersectionZoology,
    SectionZoology,
    SubsectionZoology,
    SuperseriesZoology,
    SeriesZoology,
    SubseriesZoology,
    Falanx,
    Gigafamily,
    Megafamily,
    Grandfamily,
    Superfamily,
    Epifamily,
    Family,
    Subfamily,
    Infrafamily,
    Supertribe,
    Tribe,
    Subtribe,
    Infratribe,
    SupragenericName,
    Supergenus,
    Genus,
    Subgenus,
    Infragenus,
    DivisionBotany,
    SupersectionBotany,
    SectionBotany,
    SubsectionBotany,
    SuperseriesBotany,
    SeriesBotany,
    SubseriesBotany,
    InfragenericName,
    SpeciesAggregate,
    Species,
    InfraspecificName,
    Grex,
    Klepton,
    Subspecies,
    CultivarGroup,
    Convariety,
    InfrasubspecificName,
    Proles,
    Natio,
    Aberration,
    Morph,
    Supervariety,
    Variety,
    Subvariety,
    Superform,
    Form,
    Subform,
    Pathovar,
    Biovar,
    Chemovar,
    Morphovar,
    Phagovar,
    Serovar,
    Chemoform,
    FormaSpecialis,
    Lusus,
    Cultivar,
    Mutatio,
    Strain,
    Other,
    Unranked,
}

/// Java `Rank.UNCOMPARABLE_RANKS` (`Rank.java:341-353`, `private`): ranks which represent a
/// "range" rather than a single comparable level, so [`Rank::is_uncomparable`] excludes them
/// from ordinal comparisons.
const UNCOMPARABLE_RANKS: [Rank; 6] = [
    Rank::SupragenericName,
    Rank::InfragenericName,
    Rank::InfraspecificName,
    Rank::InfrasubspecificName,
    Rank::Other,
    Rank::Unranked,
];

/// Java `Rank.LEGACY_RANKS` (`Rank.java:355-364`, `private`): ranks considered legacy, not
/// used in current nomenclature.
const LEGACY_RANKS: [Rank; 8] = [
    Rank::Morph,
    Rank::Aberration,
    Rank::Natio,
    Rank::Proles,
    Rank::Convariety,
    Rank::Klepton,
    Rank::Falanx,
    Rank::Lusus,
];

/// Java `Rank.MAJOR_RANKS`'s static-init prefix-stripping regex,
/// `^(SUPER|SUB(?:TER)?|INFRA|MICRO|NANO|GIGA|MAGN|GRAND|MIR|NAN|HYPO|MIN|PARV|MEGA|EPI)`
/// (`Rank.java:373`), transcribed as an ordered list of literal prefixes tried left-to-right
/// — reproducing Java regex alternation's ordered-choice semantics without pulling `regex`
/// into the model layer. `SUBTER` is listed before the plain `SUB` so a name like
/// `SUBTERCLASS` strips the full `SUBTER` (the `(?:TER)?` group is greedy and tried first),
/// not just `SUB` (which would leave the unmappable remainder `TERCLASS`). `NAN` (after
/// `MIR`) is unreachable in practice — every current rank name starting "NAN" also starts
/// "NANO", which is tried earlier in this same list — kept only because the source regex
/// still lists it (same "port the whole literal, faithfully, even the unreachable part"
/// precedent as `bot_to_zool` in `pipeline::stripandstash`).
const MAJOR_RANK_PREFIXES: [&str; 16] = [
    "SUPER", "SUBTER", "SUB", "INFRA", "MICRO", "NANO", "GIGA", "MAGN", "GRAND", "MIR", "NAN",
    "HYPO", "MIN", "PARV", "MEGA", "EPI",
];

/// Strips the first matching prefix from [`MAJOR_RANK_PREFIXES`] (tried in order), or
/// returns `None` if none match — the literal-prefix-list equivalent of applying Java's
/// `prefixes.matcher(r.name())` + `m.replaceFirst("")` (`Rank.java:388-390`).
fn strip_major_rank_prefix(name: &str) -> Option<&str> {
    for prefix in MAJOR_RANK_PREFIXES {
        if let Some(rest) = name.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    None
}

/// The exact Java `.name()` string for a rank (e.g. `Rank::DivisionZoology` ->
/// `"DIVISION_ZOOLOGY"`), reusing `Rank`'s own `Serialize` impl (rather than a second,
/// hand-duplicated name table) so this can never drift from the wire form. Used only by the
/// [`MAJOR_RANKS`] static-init below, mirroring how `Rank.java`'s own static block calls
/// `r.name()` (`Rank.java:388`).
fn rank_wire_name(rank: Rank) -> String {
    serde_json::to_string(&rank)
        .expect("Rank always serializes to a JSON string")
        .trim_matches('"')
        .to_string()
}

/// Java `Rank.MAJOR_RANKS` (`Rank.java:367-410`, `private`, built in a static initializer):
/// for every rank, the "major" rank it belongs to once its prefix (`SUPER-`/`SUB-`/…) is
/// stripped — infraspecific ranks all collapse to `INFRASPECIFIC_NAME` first, then the
/// prefix-strip + name lookup runs for everything else, then 6 manual fixes are layered on
/// top for the cases the automatic algorithm can't resolve (verified against Java's own
/// static block line-by-line; see [`Rank::get_major_rank`]'s tests for the specific cases).
static MAJOR_RANKS: LazyLock<HashMap<Rank, Rank>> = LazyLock::new(|| {
    let name_to_rank: HashMap<String, Rank> =
        Rank::ALL.iter().map(|&r| (rank_wire_name(r), r)).collect();

    let mut map = HashMap::with_capacity(Rank::ALL.len());
    for &r in Rank::ALL.iter() {
        let major = if r.is_infraspecific() {
            Rank::InfraspecificName
        } else {
            strip_major_rank_prefix(&rank_wire_name(r))
                .and_then(|rest| name_to_rank.get(rest))
                .copied()
                .unwrap_or(r)
        };
        map.insert(r, major);
    }
    // Manual fixes (`Rank.java:399-407`): `NANORDER` strips (via `NANO`, tried before the
    // unreachable `NAN`) to the unmapped "RDER"; `SPECIES_AGGREGATE`/`INFRAGENERIC_NAME`
    // match no prefix at all; and the zoological division ranks were renamed to
    // `*_ZOOLOGY`, so `SUPERDIVISION`/`SUBDIVISION`/`INFRADIVISION` can no longer strip to
    // a plain "DIVISION" (Java's own comment on this exact point, `Rank.java:403-404`).
    map.insert(Rank::Nanorder, Rank::Order);
    map.insert(Rank::SpeciesAggregate, Rank::Species);
    map.insert(Rank::InfragenericName, Rank::Genus);
    map.insert(Rank::Superdivision, Rank::DivisionZoology);
    map.insert(Rank::Subdivision, Rank::DivisionZoology);
    map.insert(Rank::Infradivision, Rank::DivisionZoology);
    map
});

/// Java `Rank.AMBIGUOUS_MARKER` (`Rank.java:366-382`, `private`, same static initializer as
/// `MAJOR_RANKS`): the set of ranks whose marker text is shared with at least one other
/// rank (the 7 zoology/botany pairs — `div.`, `sect.`, `subsect.`, `supersect.`, `ser.`,
/// `subser.`, `superser.` — 14 ranks total), computed by walking [`Rank::ALL`] in ordinal
/// order and flagging both ranks the first time a marker string repeats.
static AMBIGUOUS_MARKER: LazyLock<HashSet<Rank>> = LazyLock::new(|| {
    let mut ambiguous = HashSet::new();
    let mut seen: HashMap<&'static str, Rank> = HashMap::new();
    for &r in Rank::ALL.iter() {
        if let Some(marker) = r.marker() {
            if let Some(&earlier) = seen.get(marker) {
                ambiguous.insert(r);
                ambiguous.insert(earlier);
            } else {
                seen.insert(marker, r);
            }
        }
    }
    ambiguous
});

impl Rank {
    /// All 117 constants, in Java declaration/ordinal order — the Rust equivalent of the
    /// compiler-generated Java `Rank.values()`, needed since Rust has no built-in enum
    /// reflection. Used internally to build [`MAJOR_RANKS`]/[`AMBIGUOUS_MARKER`] and by
    /// this module's own tests.
    pub const ALL: [Rank; 117] = [
        Rank::Superdomain,
        Rank::Domain,
        Rank::Subdomain,
        Rank::Infradomain,
        Rank::Empire,
        Rank::Realm,
        Rank::Subrealm,
        Rank::Superkingdom,
        Rank::Kingdom,
        Rank::Subkingdom,
        Rank::Infrakingdom,
        Rank::Superphylum,
        Rank::Phylum,
        Rank::Subphylum,
        Rank::Infraphylum,
        Rank::Parvphylum,
        Rank::Microphylum,
        Rank::Nanophylum,
        Rank::Claudius,
        Rank::Gigaclass,
        Rank::Megaclass,
        Rank::Superclass,
        Rank::Class,
        Rank::Subclass,
        Rank::Infraclass,
        Rank::Subterclass,
        Rank::Parvclass,
        Rank::Superdivision,
        Rank::DivisionZoology,
        Rank::Subdivision,
        Rank::Infradivision,
        Rank::Superlegion,
        Rank::Legion,
        Rank::Sublegion,
        Rank::Infralegion,
        Rank::Megacohort,
        Rank::Supercohort,
        Rank::Cohort,
        Rank::Subcohort,
        Rank::Infracohort,
        Rank::Gigaorder,
        Rank::Magnorder,
        Rank::Grandorder,
        Rank::Mirorder,
        Rank::Superorder,
        Rank::Order,
        Rank::Nanorder,
        Rank::Hypoorder,
        Rank::Minorder,
        Rank::Suborder,
        Rank::Infraorder,
        Rank::Parvorder,
        Rank::SupersectionZoology,
        Rank::SectionZoology,
        Rank::SubsectionZoology,
        Rank::SuperseriesZoology,
        Rank::SeriesZoology,
        Rank::SubseriesZoology,
        Rank::Falanx,
        Rank::Gigafamily,
        Rank::Megafamily,
        Rank::Grandfamily,
        Rank::Superfamily,
        Rank::Epifamily,
        Rank::Family,
        Rank::Subfamily,
        Rank::Infrafamily,
        Rank::Supertribe,
        Rank::Tribe,
        Rank::Subtribe,
        Rank::Infratribe,
        Rank::SupragenericName,
        Rank::Supergenus,
        Rank::Genus,
        Rank::Subgenus,
        Rank::Infragenus,
        Rank::DivisionBotany,
        Rank::SupersectionBotany,
        Rank::SectionBotany,
        Rank::SubsectionBotany,
        Rank::SuperseriesBotany,
        Rank::SeriesBotany,
        Rank::SubseriesBotany,
        Rank::InfragenericName,
        Rank::SpeciesAggregate,
        Rank::Species,
        Rank::InfraspecificName,
        Rank::Grex,
        Rank::Klepton,
        Rank::Subspecies,
        Rank::CultivarGroup,
        Rank::Convariety,
        Rank::InfrasubspecificName,
        Rank::Proles,
        Rank::Natio,
        Rank::Aberration,
        Rank::Morph,
        Rank::Supervariety,
        Rank::Variety,
        Rank::Subvariety,
        Rank::Superform,
        Rank::Form,
        Rank::Subform,
        Rank::Pathovar,
        Rank::Biovar,
        Rank::Chemovar,
        Rank::Morphovar,
        Rank::Phagovar,
        Rank::Serovar,
        Rank::Chemoform,
        Rank::FormaSpecialis,
        Rank::Lusus,
        Rank::Cultivar,
        Rank::Mutatio,
        Rank::Strain,
        Rank::Other,
        Rank::Unranked,
    ];

    /// Java `Rank.LINNEAN_RANKS` (`Rank.java:313-324`, `public`): the 7 main Linnean ranks,
    /// ordered. Backs [`Rank::is_linnean`].
    pub const LINNEAN_RANKS: [Rank; 7] = [
        Rank::Kingdom,
        Rank::Phylum,
        Rank::Class,
        Rank::Order,
        Rank::Family,
        Rank::Genus,
        Rank::Species,
    ];

    /// Java `Enum.ordinal()` — this rank's 0-based position in declaration order, i.e. its
    /// index into [`Rank::ALL`]. Every ordinal-compare predicate below spells this out
    /// explicitly (`self.ordinal() > Rank::Species.ordinal()`, …) to mirror each Java method
    /// body line-for-line rather than relying on a derived `Ord`.
    pub fn ordinal(&self) -> usize {
        *self as usize
    }

    /// Java `Rank.getCode()` (`Rank.java:469-474`): the single nomenclatural code a rank
    /// constant is anchored to (set via that constant's own constructor argument), or
    /// `None` for a code-agnostic rank (most of them — `Rank`'s own class doc: "The ranks
    /// listed are code agnostic" unless stated otherwise). Exhaustive `match` grouped by
    /// code, not a wildcard fallback: a future variant addition gets a compile error here
    /// until it's assigned a code explicitly. Verified against every one of `Rank.java`'s
    /// 117 constructor calls (67 code-carrying: 44 `ZOOLOGICAL`, 10 `BOTANICAL`, 7
    /// `BACTERIAL`, 4 `CULTIVARS`, 2 `VIRUS`; 50 `None`).
    pub fn code(&self) -> Option<NomCode> {
        match self {
            // -> VIRUS (2)
            Rank::Realm | Rank::Subrealm => Some(NomCode::Virus),
            // -> ZOOLOGICAL (44)
            Rank::Parvphylum
            | Rank::Microphylum
            | Rank::Nanophylum
            | Rank::Claudius
            | Rank::Gigaclass
            | Rank::Megaclass
            | Rank::Subterclass
            | Rank::Parvclass
            | Rank::Superdivision
            | Rank::DivisionZoology
            | Rank::Subdivision
            | Rank::Infradivision
            | Rank::Superlegion
            | Rank::Legion
            | Rank::Sublegion
            | Rank::Infralegion
            | Rank::Megacohort
            | Rank::Supercohort
            | Rank::Cohort
            | Rank::Subcohort
            | Rank::Infracohort
            | Rank::Gigaorder
            | Rank::Magnorder
            | Rank::Grandorder
            | Rank::Mirorder
            | Rank::Nanorder
            | Rank::Hypoorder
            | Rank::Minorder
            | Rank::Parvorder
            | Rank::SupersectionZoology
            | Rank::SectionZoology
            | Rank::SubsectionZoology
            | Rank::SuperseriesZoology
            | Rank::SeriesZoology
            | Rank::SubseriesZoology
            | Rank::Gigafamily
            | Rank::Megafamily
            | Rank::Grandfamily
            | Rank::Epifamily
            | Rank::Klepton
            | Rank::Natio
            | Rank::Aberration
            | Rank::Morph
            | Rank::Mutatio => Some(NomCode::Zoological),
            // -> BOTANICAL (10)
            Rank::DivisionBotany
            | Rank::SupersectionBotany
            | Rank::SectionBotany
            | Rank::SubsectionBotany
            | Rank::SuperseriesBotany
            | Rank::SeriesBotany
            | Rank::SubseriesBotany
            | Rank::Proles
            | Rank::FormaSpecialis
            | Rank::Lusus => Some(NomCode::Botanical),
            // -> CULTIVARS (4)
            Rank::Grex | Rank::CultivarGroup | Rank::Convariety | Rank::Cultivar => {
                Some(NomCode::Cultivars)
            }
            // -> BACTERIAL (7)
            Rank::Pathovar
            | Rank::Biovar
            | Rank::Chemovar
            | Rank::Morphovar
            | Rank::Phagovar
            | Rank::Serovar
            | Rank::Chemoform => Some(NomCode::Bacterial),
            // -> None (50)
            Rank::Superdomain
            | Rank::Domain
            | Rank::Subdomain
            | Rank::Infradomain
            | Rank::Empire
            | Rank::Superkingdom
            | Rank::Kingdom
            | Rank::Subkingdom
            | Rank::Infrakingdom
            | Rank::Superphylum
            | Rank::Phylum
            | Rank::Subphylum
            | Rank::Infraphylum
            | Rank::Superclass
            | Rank::Class
            | Rank::Subclass
            | Rank::Infraclass
            | Rank::Superorder
            | Rank::Order
            | Rank::Suborder
            | Rank::Infraorder
            | Rank::Falanx
            | Rank::Superfamily
            | Rank::Family
            | Rank::Subfamily
            | Rank::Infrafamily
            | Rank::Supertribe
            | Rank::Tribe
            | Rank::Subtribe
            | Rank::Infratribe
            | Rank::SupragenericName
            | Rank::Supergenus
            | Rank::Genus
            | Rank::Subgenus
            | Rank::Infragenus
            | Rank::InfragenericName
            | Rank::SpeciesAggregate
            | Rank::Species
            | Rank::InfraspecificName
            | Rank::Subspecies
            | Rank::InfrasubspecificName
            | Rank::Supervariety
            | Rank::Variety
            | Rank::Subvariety
            | Rank::Superform
            | Rank::Form
            | Rank::Subform
            | Rank::Strain
            | Rank::Other
            | Rank::Unranked => None,
        }
    }

    /// Java `Rank.isRestrictedToCode()` (`Rank.java:590-597`, `@Deprecated` in favour of
    /// `getCode()`, kept as a plain alias) — ported for call-site parity with any future
    /// port that still names it this way.
    pub fn is_restricted_to_code(&self) -> Option<NomCode> {
        self.code()
    }

    /// Java `Rank.getMarker()` (`Rank.java:457-459`): this rank's own marker string literal
    /// (e.g. `SUBSPECIES` -> `"subsp."`), or `None` for the 3 constants with no marker at
    /// all (`CULTIVAR_GROUP`, `OTHER`, `UNRANKED`). Exhaustive `match`, grouped by identical
    /// marker text where Java's own constants share one (the 7 zoology/botany pairs, e.g.
    /// `SECTION_ZOOLOGY`/`SECTION_BOTANY` both `"sect."`) — verified against every one of
    /// `Rank.java`'s 117 constructor calls (114 marker-carrying, 3 `None`).
    pub fn marker(&self) -> Option<&'static str> {
        match self {
            Rank::Superdomain => Some("superdom."),
            Rank::Domain => Some("dom."),
            Rank::Subdomain => Some("subdom."),
            Rank::Infradomain => Some("infradom."),
            Rank::Empire => Some("imp."),
            Rank::Realm => Some("realm"),
            Rank::Subrealm => Some("subrealm"),
            Rank::Superkingdom => Some("superreg."),
            Rank::Kingdom => Some("regn."),
            Rank::Subkingdom => Some("subreg."),
            Rank::Infrakingdom => Some("infrareg."),
            Rank::Superphylum => Some("superphyl."),
            Rank::Phylum => Some("phyl."),
            Rank::Subphylum => Some("subphyl."),
            Rank::Infraphylum => Some("infraphyl."),
            Rank::Parvphylum => Some("parvphyl."),
            Rank::Microphylum => Some("microphyl."),
            Rank::Nanophylum => Some("nanophyl."),
            Rank::Claudius => Some("claud."),
            Rank::Gigaclass => Some("gigacl."),
            Rank::Megaclass => Some("megacl."),
            Rank::Superclass => Some("supercl."),
            Rank::Class => Some("cl."),
            Rank::Subclass => Some("subcl."),
            Rank::Infraclass => Some("infracl."),
            Rank::Subterclass => Some("subtercl."),
            Rank::Parvclass => Some("parvcl."),
            Rank::Superdivision => Some("superdiv."),
            Rank::DivisionZoology | Rank::DivisionBotany => Some("div."),
            Rank::Subdivision => Some("subdiv."),
            Rank::Infradivision => Some("infradiv."),
            Rank::Superlegion => Some("superleg."),
            Rank::Legion => Some("leg."),
            Rank::Sublegion => Some("subleg."),
            Rank::Infralegion => Some("infraleg."),
            Rank::Megacohort => Some("megacohort"),
            Rank::Supercohort => Some("supercohort"),
            Rank::Cohort => Some("cohort"),
            Rank::Subcohort => Some("subcohort"),
            Rank::Infracohort => Some("infracohort"),
            Rank::Gigaorder => Some("gigaord."),
            Rank::Magnorder => Some("magnord."),
            Rank::Grandorder => Some("grandord."),
            Rank::Mirorder => Some("mirord."),
            Rank::Superorder => Some("superord."),
            Rank::Order => Some("ord."),
            Rank::Nanorder => Some("nanord."),
            Rank::Hypoorder => Some("hypoord."),
            Rank::Minorder => Some("minord."),
            Rank::Suborder => Some("subord."),
            Rank::Infraorder => Some("infraord."),
            Rank::Parvorder => Some("parvord."),
            Rank::SupersectionZoology | Rank::SupersectionBotany => Some("supersect."),
            Rank::SectionZoology | Rank::SectionBotany => Some("sect."),
            Rank::SubsectionZoology | Rank::SubsectionBotany => Some("subsect."),
            Rank::SuperseriesZoology | Rank::SuperseriesBotany => Some("superser."),
            Rank::SeriesZoology | Rank::SeriesBotany => Some("ser."),
            Rank::SubseriesZoology | Rank::SubseriesBotany => Some("subser."),
            Rank::Falanx => Some("falanx"),
            Rank::Gigafamily => Some("gigafam."),
            Rank::Megafamily => Some("megafam."),
            Rank::Grandfamily => Some("grandfam."),
            Rank::Superfamily => Some("superfam."),
            Rank::Epifamily => Some("epifam."),
            Rank::Family => Some("fam."),
            Rank::Subfamily => Some("subfam."),
            Rank::Infrafamily => Some("infrafam."),
            Rank::Supertribe => Some("supertrib."),
            Rank::Tribe => Some("trib."),
            Rank::Subtribe => Some("subtrib."),
            Rank::Infratribe => Some("infratrib."),
            Rank::SupragenericName => Some("supragen."),
            Rank::Supergenus => Some("supergen."),
            Rank::Genus => Some("gen."),
            Rank::Subgenus => Some("subgen."),
            Rank::Infragenus => Some("infrag."),
            Rank::InfragenericName => Some("infragen."),
            Rank::SpeciesAggregate => Some("agg."),
            Rank::Species => Some("sp."),
            Rank::InfraspecificName => Some("infrasp."),
            Rank::Grex => Some("gx"),
            Rank::Klepton => Some("klepton"),
            Rank::Subspecies => Some("subsp."),
            Rank::Convariety => Some("convar."),
            Rank::InfrasubspecificName => Some("infrasubsp."),
            Rank::Proles => Some("prol."),
            Rank::Natio => Some("natio"),
            Rank::Aberration => Some("ab."),
            Rank::Morph => Some("morph"),
            Rank::Supervariety => Some("supervar."),
            Rank::Variety => Some("var."),
            Rank::Subvariety => Some("subvar."),
            Rank::Superform => Some("superf."),
            Rank::Form => Some("f."),
            Rank::Subform => Some("subf."),
            Rank::Pathovar => Some("pv."),
            Rank::Biovar => Some("biovar"),
            Rank::Chemovar => Some("chemovar"),
            Rank::Morphovar => Some("morphovar"),
            Rank::Phagovar => Some("phagovar"),
            Rank::Serovar => Some("serovar"),
            Rank::Chemoform => Some("chemoform"),
            Rank::FormaSpecialis => Some("f.sp."),
            Rank::Lusus => Some("lusus"),
            Rank::Cultivar => Some("cv."),
            Rank::Mutatio => Some("mut."),
            Rank::Strain => Some("strain"),
            Rank::CultivarGroup | Rank::Other | Rank::Unranked => None,
        }
    }

    /// Java `Rank.getPlural()` (`Rank.java:465-467`): this rank's plural word — either the
    /// explicit third constructor argument, or (when a rank has a marker but no explicit
    /// plural) Java's own default `rank.name().toLowerCase() + "s"` computed once and
    /// transcribed here literally. That default is applied VERBATIM even where it reads
    /// oddly (e.g. `DIVISION_ZOOLOGY` -> `"division_zoologys"`, `SECTION_BOTANY` ->
    /// `"section_botanys"` — the Java default never strips the constant's own `_` word
    /// separators before appending `"s"`) — faithful port, not a fix. `None` for the same 3
    /// markerless constants as [`Rank::marker`] (a rank's `plural` field is only ever set
    /// alongside a `marker` in every one of Java's 5 constructors).
    pub fn plural(&self) -> Option<&'static str> {
        match self {
            Rank::Superdomain => Some("superdomains"),
            Rank::Domain => Some("domains"),
            Rank::Subdomain => Some("subdomains"),
            Rank::Infradomain => Some("infradomains"),
            Rank::Empire => Some("empires"),
            Rank::Realm => Some("realms"),
            Rank::Subrealm => Some("subrealms"),
            Rank::Superkingdom => Some("superkingdoms"),
            Rank::Kingdom => Some("kingdoms"),
            Rank::Subkingdom => Some("subkingdoms"),
            Rank::Infrakingdom => Some("infrakingdoms"),
            Rank::Superphylum => Some("superphyla"),
            Rank::Phylum => Some("phyla"),
            Rank::Subphylum => Some("subphyla"),
            Rank::Infraphylum => Some("infraphyla"),
            Rank::Parvphylum => Some("parvphyla"),
            Rank::Microphylum => Some("microphyla"),
            Rank::Nanophylum => Some("nanophyla"),
            Rank::Claudius => Some("claudius"),
            Rank::Gigaclass => Some("gigaclasses"),
            Rank::Megaclass => Some("megaclasses"),
            Rank::Superclass => Some("superclasses"),
            Rank::Class => Some("classes"),
            Rank::Subclass => Some("subclasses"),
            Rank::Infraclass => Some("infraclasses"),
            Rank::Subterclass => Some("subterclasses"),
            Rank::Parvclass => Some("parvclasses"),
            Rank::Superdivision => Some("superdivisions"),
            Rank::DivisionZoology => Some("division_zoologys"),
            Rank::Subdivision => Some("subdivisions"),
            Rank::Infradivision => Some("infradivisions"),
            Rank::Superlegion => Some("superlegions"),
            Rank::Legion => Some("legions"),
            Rank::Sublegion => Some("sublegions"),
            Rank::Infralegion => Some("infralegions"),
            Rank::Megacohort => Some("megacohorts"),
            Rank::Supercohort => Some("supercohorts"),
            Rank::Cohort => Some("cohorts"),
            Rank::Subcohort => Some("subcohorts"),
            Rank::Infracohort => Some("infracohorts"),
            Rank::Gigaorder => Some("gigaorders"),
            Rank::Magnorder => Some("magnorders"),
            Rank::Grandorder => Some("grandorders"),
            Rank::Mirorder => Some("mirorders"),
            Rank::Superorder => Some("superorders"),
            Rank::Order => Some("orders"),
            Rank::Nanorder => Some("nanorders"),
            Rank::Hypoorder => Some("hypoorders"),
            Rank::Minorder => Some("minorders"),
            Rank::Suborder => Some("suborders"),
            Rank::Infraorder => Some("infraorders"),
            Rank::Parvorder => Some("parvorders"),
            Rank::SupersectionZoology => Some("supersection_zoologys"),
            Rank::SectionZoology => Some("section_zoologys"),
            Rank::SubsectionZoology => Some("subsection_zoologys"),
            Rank::SuperseriesZoology | Rank::SuperseriesBotany => Some("superseries"),
            Rank::SeriesZoology | Rank::SeriesBotany => Some("series"),
            Rank::SubseriesZoology | Rank::SubseriesBotany => Some("subseries"),
            Rank::Falanx => Some("falanges"),
            Rank::Gigafamily => Some("gigafamilies"),
            Rank::Megafamily => Some("megafamilies"),
            Rank::Grandfamily => Some("grandfamilies"),
            Rank::Superfamily => Some("superfamilies"),
            Rank::Epifamily => Some("epifamilies"),
            Rank::Family => Some("families"),
            Rank::Subfamily => Some("subfamilies"),
            Rank::Infrafamily => Some("infrafamilies"),
            Rank::Supertribe => Some("supertribes"),
            Rank::Tribe => Some("tribes"),
            Rank::Subtribe => Some("subtribes"),
            Rank::Infratribe => Some("infratribes"),
            Rank::SupragenericName => Some("suprageneric_names"),
            Rank::Supergenus => Some("supergenera"),
            Rank::Genus => Some("genera"),
            Rank::Subgenus => Some("subgenera"),
            Rank::Infragenus => Some("infragenera"),
            Rank::DivisionBotany => Some("divisions"),
            Rank::SupersectionBotany => Some("supersection_botanys"),
            Rank::SectionBotany => Some("section_botanys"),
            Rank::SubsectionBotany => Some("subsection_botanys"),
            Rank::InfragenericName => Some("infrageneric_names"),
            Rank::SpeciesAggregate => Some("species_aggregates"),
            Rank::Species => Some("species"),
            Rank::InfraspecificName => Some("infraspecific_names"),
            Rank::Grex => Some("grexs"),
            Rank::Klepton => Some("kleptons"),
            Rank::Subspecies => Some("subspecies"),
            Rank::Convariety => Some("convarieties"),
            Rank::InfrasubspecificName => Some("infrasubspecific_names"),
            Rank::Proles => Some("proles"),
            Rank::Natio => Some("natios"),
            Rank::Aberration => Some("aberrations"),
            Rank::Morph => Some("morphs"),
            Rank::Supervariety => Some("supervarieties"),
            Rank::Variety => Some("varieties"),
            Rank::Subvariety => Some("subvarieties"),
            Rank::Superform => Some("superforms"),
            Rank::Form => Some("forms"),
            Rank::Subform => Some("subforms"),
            Rank::Pathovar => Some("pathovars"),
            Rank::Biovar => Some("biovars"),
            Rank::Chemovar => Some("chemovars"),
            Rank::Morphovar => Some("morphovars"),
            Rank::Phagovar => Some("phagovars"),
            Rank::Serovar => Some("serovars"),
            Rank::Chemoform => Some("chemoforms"),
            Rank::FormaSpecialis => Some("forma_specialiss"),
            Rank::Lusus => Some("lusi"),
            Rank::Cultivar => Some("cultivars"),
            Rank::Mutatio => Some("mutatios"),
            Rank::Strain => Some("strains"),
            Rank::CultivarGroup | Rank::Other | Rank::Unranked => None,
        }
    }

    /// Java `Rank.hasAmbiguousMarker()` (`Rank.java:461-463`). See [`AMBIGUOUS_MARKER`].
    pub fn has_ambiguous_marker(&self) -> bool {
        AMBIGUOUS_MARKER.contains(self)
    }

    /// Java `Rank.notOtherOrUnranked()` (`Rank.java:529-531`).
    pub fn not_other_or_unranked(&self) -> bool {
        *self != Rank::Other && *self != Rank::Unranked
    }

    /// Java `Rank.otherOrUnranked()` (`Rank.java:533-535`).
    pub fn other_or_unranked(&self) -> bool {
        !self.not_other_or_unranked()
    }

    /// Java `Rank.isInfraspecific()` (`Rank.java:476-481`): true for infraspecific ranks,
    /// excluding `SPECIES` itself.
    pub fn is_infraspecific(&self) -> bool {
        self.ordinal() > Rank::Species.ordinal() && self.not_other_or_unranked()
    }

    /// Java `Rank.isInfrageneric()` (`Rank.java:490-495`): true for any rank below genus —
    /// also true for species and infraspecific ranks (see [`Rank::is_infrageneric_strictly`]
    /// for the narrower, "strictly infrageneric" version).
    pub fn is_infrageneric(&self) -> bool {
        self.ordinal() > Rank::Genus.ordinal() && self.not_other_or_unranked()
    }

    /// Java `Rank.isInfragenericStrictly()` (`Rank.java:497-502`): true for real
    /// infrageneric ranks (an infrageneric epithet, strictly below genus and above species
    /// aggregate) — e.g. `SUBGENUS`, `SECTION_BOTANY`. Replaces the former ad-hoc
    /// `authorship_split::rank_is_infrageneric_strictly`.
    pub fn is_infrageneric_strictly(&self) -> bool {
        self.is_infrageneric() && self.ordinal() < Rank::SpeciesAggregate.ordinal()
    }

    /// Java `Rank.isLinnean()` (`Rank.java:504-514`): true for the 7 main Linnean ranks.
    pub fn is_linnean(&self) -> bool {
        Self::LINNEAN_RANKS.contains(self)
    }

    /// Java `Rank.getMajorRank()` (`Rank.java:516-523`): the major rank (incl. all Linnean
    /// ranks) this rank belongs to, stripping its prefix (e.g. `PHYLUM` for `SUBPHYLUM`).
    /// Infraspecific ranks return `INFRASPECIFIC_NAME`. Ranks that can't be mapped to a
    /// major rank return themselves — never a sentinel/`None` (matches Java's own doc:
    /// "never null"). See [`MAJOR_RANKS`].
    pub fn get_major_rank(&self) -> Rank {
        *MAJOR_RANKS
            .get(self)
            .expect("MAJOR_RANKS is populated for every Rank in Rank::ALL")
    }

    /// Java `Rank.isSpeciesOrBelow()` (`Rank.java:525-527`).
    pub fn is_species_or_below(&self) -> bool {
        self.ordinal() >= Rank::SpeciesAggregate.ordinal() && self.not_other_or_unranked()
    }

    /// Java `Rank.isFamilyGroup()` (`Rank.java:537-542`): true for family-group ranks,
    /// between `GIGAFAMILY` (inclusive) and `GENUS`-side `SUPRAGENERIC_NAME` (exclusive).
    pub fn is_family_group(&self) -> bool {
        Rank::Gigafamily.ordinal() <= self.ordinal()
            && self.ordinal() < Rank::SupragenericName.ordinal()
    }

    /// Java `Rank.isGenusGroup()` (`Rank.java:544-549`): true for genus-group ranks,
    /// between `SUPERGENUS` (inclusive) and `SPECIES_AGGREGATE` (exclusive).
    pub fn is_genus_group(&self) -> bool {
        Rank::Supergenus.ordinal() <= self.ordinal()
            && self.ordinal() < Rank::SpeciesAggregate.ordinal()
    }

    /// Java `Rank.isSuprageneric()` (`Rank.java:551-556`): true for any rank above genus.
    pub fn is_suprageneric(&self) -> bool {
        self.ordinal() < Rank::Genus.ordinal()
    }

    /// Java `Rank.isUncomparable()` (`Rank.java:572-581`). See [`UNCOMPARABLE_RANKS`].
    pub fn is_uncomparable(&self) -> bool {
        UNCOMPARABLE_RANKS.contains(self)
    }

    /// Java `Rank.isLegacy()` (`Rank.java:583-588`). See [`LEGACY_RANKS`].
    pub fn is_legacy(&self) -> bool {
        LEGACY_RANKS.contains(self)
    }

    /// Java `Rank.higherThan(Rank)` (`Rank.java:606-613`): true if this rank is higher than
    /// `other`, excluding `OTHER`/`UNRANKED` from the comparison (checked on `other`, not
    /// `self` — this asymmetry with [`Rank::lower_than`] is exactly Java's own).
    pub fn higher_than(&self, other: Rank) -> bool {
        self.ordinal() < other.ordinal() && other.not_other_or_unranked()
    }

    /// Java `Rank.lowerThan(Rank)` (`Rank.java:615-622`): true if this rank is lower than
    /// `other`, excluding `OTHER`/`UNRANKED` from the comparison (checked on `self`, not
    /// `other` — see [`Rank::higher_than`]'s doc comment for the same asymmetry).
    pub fn lower_than(&self, other: Rank) -> bool {
        self.ordinal() > other.ordinal() && self.not_other_or_unranked()
    }
}

/// Phase 3 (FFM binding) additions — purely additive, nothing above this point is touched.
/// The FFI crate (`nameparser-ffi`) needs to turn a Java `Rank.name()` / `NomCode.name()`
/// string (received across the C ABI as a plain `&str`) back into the corresponding enum
/// variant. Neither `Rank` nor `NomCode` derives `serde::Deserialize` (only `Serialize`, for
/// the wire-JSON output direction), so this is a hand-written reverse lookup rather than a
/// `serde_json::from_value` round-trip.
impl Rank {
    /// Looks up a variant by its serialized SCREAMING_SNAKE name (Java `Rank.name()`), the
    /// exact inverse of [`rank_wire_name`] — which itself is built on this type's own
    /// `Serialize` impl, so `from_name` can never silently drift from the wire form it
    /// reverses. A linear scan over all 117 variants; `Rank::from_name` is only ever called
    /// once per FFI parse call, not in a hot inner loop. Returns `None` for an unrecognized
    /// name — mirrors Java's `Rank.valueOf(name)` throwing `IllegalArgumentException`, folded
    /// to `None`/absent here since the FFI caller treats an unrecognized rank hint the same
    /// as an absent one rather than propagating a Java exception type across the C ABI.
    pub fn from_name(name: &str) -> Option<Rank> {
        Rank::ALL
            .iter()
            .copied()
            .find(|&r| rank_wire_name(r) == name)
    }
}

impl NomCode {
    /// Looks up a variant by its serialized SCREAMING_SNAKE name (Java `NomCode.name()`).
    /// A hand-written match, unlike [`Rank::from_name`], since `NomCode` has no `ALL`-style
    /// constant to reverse-search over — but each arm's string literal is exactly this type's
    /// own `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` output for that variant, pinned
    /// against the real `Serialize` impl by
    /// `nomcode_from_name_round_trips_every_variant_via_its_own_serialize_impl` below, so the
    /// two representations cannot silently drift apart. Returns `None` for an unrecognized
    /// name, same rationale as `Rank::from_name`.
    pub fn from_name(name: &str) -> Option<NomCode> {
        match name {
            "BACTERIAL" => Some(NomCode::Bacterial),
            "BOTANICAL" => Some(NomCode::Botanical),
            "CULTIVARS" => Some(NomCode::Cultivars),
            "PHYTO" => Some(NomCode::Phyto),
            "VIRUS" => Some(NomCode::Virus),
            "ZOOLOGICAL" => Some(NomCode::Zoological),
            "PHYLO" => Some(NomCode::Phylo),
            _ => None,
        }
    }
}

/// Java `org.gbif.nameparser.util.RankUtils`'s rank-lookup statics not already covered by
/// `Rank`'s own instance methods — ported as a `pub mod` the same way `Warnings` (a separate
/// Java class) is ported as this file's own `pub mod warnings` below.
///
/// **Not ported: `RankUtils.GLOBAL_SUFFICES_RANK_MAP`** (`RankUtils.java:262-292`, the
/// cross-code-deduplicated, length-sorted derivative of `SUFFICES_RANK_MAP`). It backs only
/// `RankUtils.inferRank(LinneanName)`, a public utility method no Phase 1 pipeline stage
/// calls — confirmed by reading `Assemble.java` itself: its own `rankFromGlobalSuffix`
/// helper (the "no code known" fallback for step 14) hardcodes exactly the two globally
/// unambiguous suffixes ("aceae"/"oideae") rather than consulting the derived global map at
/// all. Porting `GLOBAL_SUFFICES_RANK_MAP`'s length-then-alphabetical custom `Comparator`
/// and cross-code non-unique-key removal would be pure dead weight for this crate's actual
/// call graph; left unported with this note rather than stubbed.
pub mod rank_utils {
    use super::{NomCode, Rank};

    /// Java `RankUtils.SUFFICES_RANK_MAP` (`RankUtils.java:204-260`): per-`NomCode`
    /// name-suffix -> `Rank` lookup, used by `Assemble.rankFromSuffix` (Phase 1 Slice 4
    /// Task 3's "step 14" — suffix-based rank inference for a monomial once its code is
    /// known, e.g. bacterial "...aceae" -> `FAMILY`). Each code's list is manually ordered
    /// longest/most-specific-suffix first in Java's own source (that field's own doc
    /// comment) — NOT re-sorted here; slice iteration order is exactly Java's
    /// `LinkedHashMap` insertion order, so a first-match-wins linear scan by the consuming
    /// task (`.iter().find(|(suffix, _)| name.ends_with(suffix))`) reproduces the same
    /// "most specific first" behaviour with no sort needed on either side. Only the 4 codes
    /// below appear at all in Java's map (`CULTIVARS`/`PHYTO`/`PHYLO` have no entry, matching
    /// `Map.of(...)`'s fixed 4-key literal) — `None` for those three, mirroring `Map.get`'s
    /// null for an absent key (`Assemble.rankFromSuffix`'s own `if (suffixes == null) return
    /// null;` guard).
    pub fn suffices_rank_map(code: NomCode) -> Option<&'static [(&'static str, Rank)]> {
        match code {
            NomCode::Bacterial => Some(&[
                ("oideae", Rank::Subfamily),
                ("aceae", Rank::Family),
                ("ineae", Rank::Suborder),
                ("ales", Rank::Order),
                ("idae", Rank::Subclass),
                ("inae", Rank::Subtribe),
                ("eae", Rank::Tribe),
                ("ia", Rank::Class),
            ]),
            NomCode::Botanical => Some(&[
                ("mycetidae", Rank::Subclass),
                ("phycidae", Rank::Subclass),
                ("mycotina", Rank::Subphylum),
                ("phytina", Rank::Subphylum),
                ("mycetes", Rank::Class),
                ("phyceae", Rank::Class),
                ("mycota", Rank::Phylum),
                ("opsida", Rank::Class),
                ("oideae", Rank::Subfamily),
                ("phyta", Rank::Phylum),
                ("ineae", Rank::Suborder),
                ("aceae", Rank::Family),
                ("idae", Rank::Subclass),
                ("anae", Rank::Superorder),
                ("acea", Rank::Superfamily),
                ("aria", Rank::Infraorder),
                ("ales", Rank::Order),
                ("inae", Rank::Subtribe),
                ("eae", Rank::Tribe),
            ]),
            NomCode::Zoological => Some(&[
                ("oidea", Rank::Superfamily),
                ("oidae", Rank::Epifamily),
                ("idae", Rank::Family),
                ("inae", Rank::Subfamily),
                ("ini", Rank::Tribe),
                ("ina", Rank::Subtribe),
            ]),
            NomCode::Virus => Some(&[
                ("viricetidae", Rank::Subclass),
                ("viricotina", Rank::Subphylum),
                ("viricetes", Rank::Class),
                ("viricota", Rank::Phylum),
                ("virineae", Rank::Suborder),
                ("virites", Rank::Subkingdom),
                ("virales", Rank::Order),
                ("viridae", Rank::Family),
                ("virinae", Rank::Subfamily),
                ("viriae", Rank::Kingdom),
                ("viria", Rank::Realm),
                ("vira", Rank::Subrealm),
            ]),
            NomCode::Cultivars | NomCode::Phyto | NomCode::Phylo => None,
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

    // ============================================================================
    // Phase 1 Slice 4 Task 1: full 117-constant `Rank` + ordinal predicates.
    // ============================================================================

    /// The exhaustive round-trip: every one of the 117 `Rank` variants, in exact Java
    /// declaration/ordinal order, serializes to its exact Java `.name()` string. This both
    /// locks in declaration order (a mismatch here means an ordinal-compare predicate would
    /// silently disagree with Java) and empirically confirms
    /// `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` needed no per-variant
    /// `#[serde(rename = "…")]` override for any multi-word constant (`SUPERSECTION_BOTANY`,
    /// `INFRASPECIFIC_NAME`, `FORMA_SPECIALIS`, …) — generated mechanically from `Rank.java`
    /// itself, not hand-transcribed.
    #[test]
    fn rank_all_117_variants_in_java_declaration_order() {
        let expected: [(Rank, &str); 117] = [
            (Rank::Superdomain, "SUPERDOMAIN"),
            (Rank::Domain, "DOMAIN"),
            (Rank::Subdomain, "SUBDOMAIN"),
            (Rank::Infradomain, "INFRADOMAIN"),
            (Rank::Empire, "EMPIRE"),
            (Rank::Realm, "REALM"),
            (Rank::Subrealm, "SUBREALM"),
            (Rank::Superkingdom, "SUPERKINGDOM"),
            (Rank::Kingdom, "KINGDOM"),
            (Rank::Subkingdom, "SUBKINGDOM"),
            (Rank::Infrakingdom, "INFRAKINGDOM"),
            (Rank::Superphylum, "SUPERPHYLUM"),
            (Rank::Phylum, "PHYLUM"),
            (Rank::Subphylum, "SUBPHYLUM"),
            (Rank::Infraphylum, "INFRAPHYLUM"),
            (Rank::Parvphylum, "PARVPHYLUM"),
            (Rank::Microphylum, "MICROPHYLUM"),
            (Rank::Nanophylum, "NANOPHYLUM"),
            (Rank::Claudius, "CLAUDIUS"),
            (Rank::Gigaclass, "GIGACLASS"),
            (Rank::Megaclass, "MEGACLASS"),
            (Rank::Superclass, "SUPERCLASS"),
            (Rank::Class, "CLASS"),
            (Rank::Subclass, "SUBCLASS"),
            (Rank::Infraclass, "INFRACLASS"),
            (Rank::Subterclass, "SUBTERCLASS"),
            (Rank::Parvclass, "PARVCLASS"),
            (Rank::Superdivision, "SUPERDIVISION"),
            (Rank::DivisionZoology, "DIVISION_ZOOLOGY"),
            (Rank::Subdivision, "SUBDIVISION"),
            (Rank::Infradivision, "INFRADIVISION"),
            (Rank::Superlegion, "SUPERLEGION"),
            (Rank::Legion, "LEGION"),
            (Rank::Sublegion, "SUBLEGION"),
            (Rank::Infralegion, "INFRALEGION"),
            (Rank::Megacohort, "MEGACOHORT"),
            (Rank::Supercohort, "SUPERCOHORT"),
            (Rank::Cohort, "COHORT"),
            (Rank::Subcohort, "SUBCOHORT"),
            (Rank::Infracohort, "INFRACOHORT"),
            (Rank::Gigaorder, "GIGAORDER"),
            (Rank::Magnorder, "MAGNORDER"),
            (Rank::Grandorder, "GRANDORDER"),
            (Rank::Mirorder, "MIRORDER"),
            (Rank::Superorder, "SUPERORDER"),
            (Rank::Order, "ORDER"),
            (Rank::Nanorder, "NANORDER"),
            (Rank::Hypoorder, "HYPOORDER"),
            (Rank::Minorder, "MINORDER"),
            (Rank::Suborder, "SUBORDER"),
            (Rank::Infraorder, "INFRAORDER"),
            (Rank::Parvorder, "PARVORDER"),
            (Rank::SupersectionZoology, "SUPERSECTION_ZOOLOGY"),
            (Rank::SectionZoology, "SECTION_ZOOLOGY"),
            (Rank::SubsectionZoology, "SUBSECTION_ZOOLOGY"),
            (Rank::SuperseriesZoology, "SUPERSERIES_ZOOLOGY"),
            (Rank::SeriesZoology, "SERIES_ZOOLOGY"),
            (Rank::SubseriesZoology, "SUBSERIES_ZOOLOGY"),
            (Rank::Falanx, "FALANX"),
            (Rank::Gigafamily, "GIGAFAMILY"),
            (Rank::Megafamily, "MEGAFAMILY"),
            (Rank::Grandfamily, "GRANDFAMILY"),
            (Rank::Superfamily, "SUPERFAMILY"),
            (Rank::Epifamily, "EPIFAMILY"),
            (Rank::Family, "FAMILY"),
            (Rank::Subfamily, "SUBFAMILY"),
            (Rank::Infrafamily, "INFRAFAMILY"),
            (Rank::Supertribe, "SUPERTRIBE"),
            (Rank::Tribe, "TRIBE"),
            (Rank::Subtribe, "SUBTRIBE"),
            (Rank::Infratribe, "INFRATRIBE"),
            (Rank::SupragenericName, "SUPRAGENERIC_NAME"),
            (Rank::Supergenus, "SUPERGENUS"),
            (Rank::Genus, "GENUS"),
            (Rank::Subgenus, "SUBGENUS"),
            (Rank::Infragenus, "INFRAGENUS"),
            (Rank::DivisionBotany, "DIVISION_BOTANY"),
            (Rank::SupersectionBotany, "SUPERSECTION_BOTANY"),
            (Rank::SectionBotany, "SECTION_BOTANY"),
            (Rank::SubsectionBotany, "SUBSECTION_BOTANY"),
            (Rank::SuperseriesBotany, "SUPERSERIES_BOTANY"),
            (Rank::SeriesBotany, "SERIES_BOTANY"),
            (Rank::SubseriesBotany, "SUBSERIES_BOTANY"),
            (Rank::InfragenericName, "INFRAGENERIC_NAME"),
            (Rank::SpeciesAggregate, "SPECIES_AGGREGATE"),
            (Rank::Species, "SPECIES"),
            (Rank::InfraspecificName, "INFRASPECIFIC_NAME"),
            (Rank::Grex, "GREX"),
            (Rank::Klepton, "KLEPTON"),
            (Rank::Subspecies, "SUBSPECIES"),
            (Rank::CultivarGroup, "CULTIVAR_GROUP"),
            (Rank::Convariety, "CONVARIETY"),
            (Rank::InfrasubspecificName, "INFRASUBSPECIFIC_NAME"),
            (Rank::Proles, "PROLES"),
            (Rank::Natio, "NATIO"),
            (Rank::Aberration, "ABERRATION"),
            (Rank::Morph, "MORPH"),
            (Rank::Supervariety, "SUPERVARIETY"),
            (Rank::Variety, "VARIETY"),
            (Rank::Subvariety, "SUBVARIETY"),
            (Rank::Superform, "SUPERFORM"),
            (Rank::Form, "FORM"),
            (Rank::Subform, "SUBFORM"),
            (Rank::Pathovar, "PATHOVAR"),
            (Rank::Biovar, "BIOVAR"),
            (Rank::Chemovar, "CHEMOVAR"),
            (Rank::Morphovar, "MORPHOVAR"),
            (Rank::Phagovar, "PHAGOVAR"),
            (Rank::Serovar, "SEROVAR"),
            (Rank::Chemoform, "CHEMOFORM"),
            (Rank::FormaSpecialis, "FORMA_SPECIALIS"),
            (Rank::Lusus, "LUSUS"),
            (Rank::Cultivar, "CULTIVAR"),
            (Rank::Mutatio, "MUTATIO"),
            (Rank::Strain, "STRAIN"),
            (Rank::Other, "OTHER"),
            (Rank::Unranked, "UNRANKED"),
        ];
        assert_eq!(expected.len(), 117);
        assert_eq!(Rank::ALL.len(), 117);
        for (i, (rank, name)) in expected.iter().enumerate() {
            assert_eq!(Rank::ALL[i], *rank, "Rank::ALL[{i}] should be {rank:?}");
            assert_eq!(rank.ordinal(), i, "{rank:?}.ordinal() should be {i}");
            assert_eq!(
                serde_json::to_string(rank).unwrap(),
                format!("\"{name}\""),
                "{rank:?} should serialize to \"{name}\""
            );
        }
    }

    /// TDD sample from the task brief: a handful of code-carrying ranks not already
    /// exercised above, including the new `REALM` (`VIRUS`) variant.
    #[test]
    fn rank_code_tdd_sample() {
        assert_eq!(Rank::Cultivar.code(), Some(NomCode::Cultivars));
        assert_eq!(Rank::Subgenus.code(), None);
        assert_eq!(Rank::Pathovar.code(), Some(NomCode::Bacterial));
        assert_eq!(Rank::SectionBotany.code(), Some(NomCode::Botanical));
        assert_eq!(Rank::Realm.code(), Some(NomCode::Virus));
        assert_eq!(Rank::Subrealm.code(), Some(NomCode::Virus));
    }

    /// TDD sample from the task brief: the ordinal predicates.
    #[test]
    fn rank_ordinal_predicates_tdd_sample() {
        assert!(Rank::Subspecies.is_infraspecific());
        assert!(!Rank::Species.is_infraspecific());
        assert!(Rank::Subgenus.is_infrageneric_strictly());
        assert!(!Rank::Genus.is_infrageneric_strictly());
        assert!(!Rank::Species.is_infrageneric_strictly());
        assert!(Rank::Family.is_suprageneric());
        assert!(!Rank::Genus.is_suprageneric());
        assert!(Rank::Genus.higher_than(Rank::Species));
        assert!(!Rank::Species.higher_than(Rank::Genus));
        assert!(Rank::Species.higher_than(Rank::Subspecies));
        assert!(Rank::Subspecies.lower_than(Rank::Species));
        assert!(!Rank::Genus.higher_than(Rank::Other));
        assert!(!Rank::Genus.higher_than(Rank::Unranked));
    }

    #[test]
    fn rank_markers_tdd_sample() {
        assert_eq!(Rank::Subspecies.marker(), Some("subsp."));
        assert_eq!(Rank::Species.marker(), Some("sp."));
        assert_eq!(Rank::CultivarGroup.marker(), None);
        assert_eq!(Rank::Other.marker(), None);
        assert_eq!(Rank::Unranked.marker(), None);
        // Zoology/botany pairs share their marker text verbatim.
        assert_eq!(Rank::SectionZoology.marker(), Rank::SectionBotany.marker());
        assert_eq!(
            Rank::DivisionZoology.marker(),
            Rank::DivisionBotany.marker()
        );
    }

    #[test]
    fn rank_plural_sample() {
        assert_eq!(Rank::Species.plural(), Some("species"));
        assert_eq!(Rank::Genus.plural(), Some("genera"));
        assert_eq!(Rank::Falanx.plural(), Some("falanges"));
        assert_eq!(Rank::Lusus.plural(), Some("lusi"));
        assert_eq!(Rank::CultivarGroup.plural(), None);
        assert_eq!(Rank::Other.plural(), None);
        assert_eq!(Rank::Unranked.plural(), None);
        // Java's un-de-underscored default plural (`name().toLowerCase() + "s"`),
        // transcribed verbatim rather than "fixed".
        assert_eq!(Rank::DivisionZoology.plural(), Some("division_zoologys"));
        assert_eq!(Rank::SectionBotany.plural(), Some("section_botanys"));
        // Explicit Java plural shared by a zoology/botany pair.
        assert_eq!(Rank::SeriesZoology.plural(), Some("series"));
        assert_eq!(Rank::SeriesBotany.plural(), Some("series"));
    }

    #[test]
    fn rank_is_family_group_and_genus_group() {
        assert!(Rank::Family.is_family_group());
        assert!(Rank::Gigafamily.is_family_group());
        assert!(!Rank::SupragenericName.is_family_group());
        assert!(!Rank::Genus.is_family_group());
        assert!(Rank::Genus.is_genus_group());
        assert!(Rank::Supergenus.is_genus_group());
        assert!(Rank::Subgenus.is_genus_group());
        assert!(!Rank::SpeciesAggregate.is_genus_group());
        assert!(!Rank::SupragenericName.is_genus_group());
    }

    #[test]
    fn rank_is_species_or_below() {
        assert!(Rank::SpeciesAggregate.is_species_or_below());
        assert!(Rank::Species.is_species_or_below());
        assert!(Rank::Subspecies.is_species_or_below());
        assert!(!Rank::Genus.is_species_or_below());
        assert!(!Rank::Other.is_species_or_below());
        assert!(!Rank::Unranked.is_species_or_below());
    }

    #[test]
    fn rank_is_uncomparable() {
        assert!(Rank::SupragenericName.is_uncomparable());
        assert!(Rank::InfragenericName.is_uncomparable());
        assert!(Rank::InfraspecificName.is_uncomparable());
        assert!(Rank::InfrasubspecificName.is_uncomparable());
        assert!(Rank::Other.is_uncomparable());
        assert!(Rank::Unranked.is_uncomparable());
        assert!(!Rank::Species.is_uncomparable());
        assert!(!Rank::Genus.is_uncomparable());
    }

    #[test]
    fn rank_is_legacy() {
        for r in [
            Rank::Morph,
            Rank::Aberration,
            Rank::Natio,
            Rank::Proles,
            Rank::Convariety,
            Rank::Klepton,
            Rank::Falanx,
            Rank::Lusus,
        ] {
            assert!(r.is_legacy(), "{r:?} should be legacy");
        }
        assert!(!Rank::Species.is_legacy());
        assert!(!Rank::Subspecies.is_legacy());
    }

    #[test]
    fn rank_is_linnean() {
        for r in Rank::LINNEAN_RANKS {
            assert!(r.is_linnean(), "{r:?} should be Linnean");
        }
        assert!(!Rank::Subgenus.is_linnean());
        assert!(!Rank::Subspecies.is_linnean());
        assert_eq!(Rank::LINNEAN_RANKS.len(), 7);
    }

    #[test]
    fn rank_is_restricted_to_code_matches_code() {
        assert_eq!(
            Rank::Cultivar.is_restricted_to_code(),
            Rank::Cultivar.code()
        );
        assert_eq!(Rank::Species.is_restricted_to_code(), Rank::Species.code());
    }

    #[test]
    fn rank_has_ambiguous_marker() {
        // The 7 zoology/botany pairs that share marker text are ambiguous...
        for r in [
            Rank::DivisionZoology,
            Rank::DivisionBotany,
            Rank::SectionZoology,
            Rank::SectionBotany,
            Rank::SubsectionZoology,
            Rank::SubsectionBotany,
            Rank::SupersectionZoology,
            Rank::SupersectionBotany,
            Rank::SeriesZoology,
            Rank::SeriesBotany,
            Rank::SubseriesZoology,
            Rank::SubseriesBotany,
            Rank::SuperseriesZoology,
            Rank::SuperseriesBotany,
        ] {
            assert!(r.has_ambiguous_marker(), "{r:?} should be ambiguous");
        }
        // ...but a rank with a unique marker is not.
        assert!(!Rank::Species.has_ambiguous_marker());
        assert!(!Rank::Genus.has_ambiguous_marker());
        assert!(!Rank::Cultivar.has_ambiguous_marker());
    }

    /// `Rank::get_major_rank()` — the automatic prefix-strip cases (no manual fix needed).
    #[test]
    fn rank_get_major_rank_automatic() {
        assert_eq!(Rank::Subphylum.get_major_rank(), Rank::Phylum);
        assert_eq!(Rank::Nanophylum.get_major_rank(), Rank::Phylum);
        assert_eq!(Rank::Subclass.get_major_rank(), Rank::Class);
        // Greedy `SUBTER` (tried before plain `SUB`) strips correctly to `CLASS`.
        assert_eq!(Rank::Subterclass.get_major_rank(), Rank::Class);
        assert_eq!(Rank::SubsectionBotany.get_major_rank(), Rank::SectionBotany);
        assert_eq!(
            Rank::SupersectionZoology.get_major_rank(),
            Rank::SectionZoology
        );
        // A rank with no prefix at all maps to itself (never `None`/a sentinel).
        assert_eq!(Rank::Species.get_major_rank(), Rank::Species);
        assert_eq!(Rank::Genus.get_major_rank(), Rank::Genus);
        assert_eq!(Rank::Other.get_major_rank(), Rank::Other);
        assert_eq!(Rank::Unranked.get_major_rank(), Rank::Unranked);
    }

    /// `Rank::get_major_rank()` — the 6 manual fixes (`Rank.java:399-407`), each a case the
    /// automatic algorithm alone cannot resolve (see [`MAJOR_RANKS`]'s own doc comment).
    #[test]
    fn rank_get_major_rank_manual_fixes() {
        assert_eq!(Rank::Nanorder.get_major_rank(), Rank::Order);
        assert_eq!(Rank::SpeciesAggregate.get_major_rank(), Rank::Species);
        assert_eq!(Rank::InfragenericName.get_major_rank(), Rank::Genus);
        assert_eq!(Rank::Superdivision.get_major_rank(), Rank::DivisionZoology);
        assert_eq!(Rank::Subdivision.get_major_rank(), Rank::DivisionZoology);
        assert_eq!(Rank::Infradivision.get_major_rank(), Rank::DivisionZoology);
    }

    /// `Rank::get_major_rank()` — every infraspecific rank (ordinal-wise, per
    /// `is_infraspecific()`) collapses to `INFRASPECIFIC_NAME`, including ones that aren't
    /// obviously "infraspecific" by name (`KLEPTON`, `CULTIVAR_GROUP`).
    #[test]
    fn rank_get_major_rank_infraspecific_collapse() {
        for r in [
            Rank::Grex,
            Rank::Klepton,
            Rank::Subspecies,
            Rank::CultivarGroup,
            Rank::Variety,
            Rank::Cultivar,
            Rank::Strain,
        ] {
            assert_eq!(
                r.get_major_rank(),
                Rank::InfraspecificName,
                "{r:?} should collapse to INFRASPECIFIC_NAME"
            );
        }
    }

    /// `rank_utils::suffices_rank_map` — Assemble step 14's suffix->rank data, per code.
    #[test]
    fn rank_utils_suffices_rank_map_sample() {
        let bot = rank_utils::suffices_rank_map(NomCode::Botanical).unwrap();
        assert!(bot.contains(&("aceae", Rank::Family)));
        assert!(bot.contains(&("oideae", Rank::Subfamily)));

        let zoo = rank_utils::suffices_rank_map(NomCode::Zoological).unwrap();
        assert!(zoo.contains(&("idae", Rank::Family)));
        assert!(zoo.contains(&("oidae", Rank::Epifamily)));
        // "idae" means FAMILY under zoology but SUBCLASS under botany/bacteriology —
        // confirms the map is genuinely per-code, not a single global lookup.
        let bact = rank_utils::suffices_rank_map(NomCode::Bacterial).unwrap();
        assert!(bact.contains(&("idae", Rank::Subclass)));

        let virus = rank_utils::suffices_rank_map(NomCode::Virus).unwrap();
        assert!(virus.contains(&("viridae", Rank::Family)));

        // Codes with no suffix map at all return None (mirrors Java's `Map.get` on a
        // missing key), not an empty slice.
        assert_eq!(rank_utils::suffices_rank_map(NomCode::Cultivars), None);
        assert_eq!(rank_utils::suffices_rank_map(NomCode::Phyto), None);
        assert_eq!(rank_utils::suffices_rank_map(NomCode::Phylo), None);
    }

    // ---- Phase 3 (FFM binding): Rank::from_name / NomCode::from_name ----

    /// Every one of the 117 `Rank` variants round-trips through its own wire name — the
    /// exhaustive version of the pin below, covering multi-word constants like
    /// `SUPERSECTION_BOTANY` and `FORMA_SPECIALIS` too, not just a hand-picked sample.
    #[test]
    fn rank_from_name_round_trips_every_variant_via_its_own_serialize_impl() {
        for &r in Rank::ALL.iter() {
            let name = rank_wire_name(r);
            assert_eq!(
                Rank::from_name(&name),
                Some(r),
                "round-trip failed for {r:?}"
            );
        }
    }

    #[test]
    fn rank_from_name_pins_a_multi_word_constant_and_rejects_nonsense() {
        assert_eq!(Rank::from_name("SPECIES"), Some(Rank::Species));
        assert_eq!(
            Rank::from_name("DIVISION_ZOOLOGY"),
            Some(Rank::DivisionZoology)
        );
        assert_eq!(Rank::from_name("NONSENSE"), None);
        assert_eq!(Rank::from_name(""), None);
    }

    /// Every one of the 7 `NomCode` variants round-trips through its own wire name —
    /// guards `NomCode::from_name`'s hand-written match against drifting from the real
    /// `#[derive(Serialize)]` output it's meant to reverse.
    #[test]
    fn nomcode_from_name_round_trips_every_variant_via_its_own_serialize_impl() {
        for &c in &[
            NomCode::Bacterial,
            NomCode::Botanical,
            NomCode::Cultivars,
            NomCode::Phyto,
            NomCode::Virus,
            NomCode::Zoological,
            NomCode::Phylo,
        ] {
            let name = serde_json::to_string(&c)
                .unwrap()
                .trim_matches('"')
                .to_string();
            assert_eq!(
                NomCode::from_name(&name),
                Some(c),
                "round-trip failed for {c:?}"
            );
        }
    }

    #[test]
    fn nomcode_from_name_pins_botanical_and_rejects_nonsense() {
        assert_eq!(NomCode::from_name("BOTANICAL"), Some(NomCode::Botanical));
        assert_eq!(NomCode::from_name("NONSENSE"), None);
    }
}
