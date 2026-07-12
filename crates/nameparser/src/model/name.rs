// SPDX-License-Identifier: Apache-2.0

//! Java `org.gbif.nameparser.api.{Authorship, CombinedAuthorship, ParsedAuthorship,
//! ParsedName}`, ported as Gson-wire-faithful Rust structs.
//!
//! Java models a class hierarchy `ParsedName extends ParsedAuthorship extends
//! CombinedAuthorship`. Gson's default reflective serialization walks that hierarchy
//! **most-derived class first**, emitting each class's own declared fields (in
//! declaration order) before moving up to the superclass. To reproduce that exact key
//! order with a plain `#[derive(Serialize)]` struct (whose fields always serialize in
//! declaration order), `ParsedAuthorship` is not given its own Rust type: its 11 fields,
//! and `CombinedAuthorship`'s 3, are flattened directly onto a single [`ParsedName`]
//! struct, in the order: ParsedName's own 16 fields, then ParsedAuthorship's 11, then
//! CombinedAuthorship's 3. `CombinedAuthorship` itself still exists as a Rust type,
//! because Java also uses it standalone as the type of `ParsedName::generic_authorship`
//! / `specific_authorship`.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::model::enums::{NamePart, NameType, NomCode, Rank, State};

/// Java `org.gbif.nameparser.api.Authorship`. Authorship of a name (recombination) or
/// basionym: authors, ex-authors and the year, but no "in" authors (those are part of
/// the `publishedIn` citation).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Authorship {
    /// Eagerly-initialized `List<String>` in Java (`= new ArrayList<>()`) — never
    /// omitted, serializes as `[]` when empty.
    pub authors: Vec<String>,
    /// Eagerly-initialized `List<String>` in Java — never omitted, serializes as `[]`
    /// when empty.
    #[serde(rename = "exAuthors")]
    pub ex_authors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    #[serde(rename = "imprintYear", skip_serializing_if = "Option::is_none")]
    pub imprint_year: Option<String>,
}

impl Authorship {
    /// Java `Authorship.exists()` = `!isEmpty()`, and Java `isEmpty()` checks ONLY
    /// authors and year (NOT exAuthors) — see Authorship.java:145-151.
    pub fn exists(&self) -> bool {
        !self.authors.is_empty() || self.year.is_some()
    }

    /// Java `Authorship.hasAuthors()`: true if `authors` is non-empty. (Java's own
    /// null-guard, `authors != null && …`, is unreachable here — `Vec<String>` is eagerly
    /// initialized and never absent, matching the field's Java default `= new
    /// ArrayList<>()`.) Used by `pipeline::code_inference`'s vote tally, distinct from
    /// [`Self::exists`] (which additionally counts a bare year with no authors).
    pub fn has_authors(&self) -> bool {
        !self.authors.is_empty()
    }
}

/// Java `org.gbif.nameparser.api.CombinedAuthorship`. Bundles the combination
/// authorship, the basionym authorship, and the (fungal) sanctioning author.
///
/// Used standalone as the type of [`ParsedName::generic_authorship`] /
/// [`ParsedName::specific_authorship`]. Its own three fields are, in addition,
/// flattened directly onto `ParsedName` (see the module doc) to reproduce Java's Gson
/// class-hierarchy field order for the outermost `ParsedName` object itself.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CombinedAuthorship {
    /// Eagerly-initialized `Authorship` in Java (`= new Authorship()`) — never omitted.
    #[serde(rename = "combinationAuthorship")]
    pub combination_authorship: Authorship,
    /// Eagerly-initialized `Authorship` in Java — never omitted.
    #[serde(rename = "basionymAuthorship")]
    pub basionym_authorship: Authorship,
    #[serde(rename = "sanctioningAuthor", skip_serializing_if = "Option::is_none")]
    pub sanctioning_author: Option<String>,
}

impl CombinedAuthorship {
    /// Java `CombinedAuthorship.hasAuthorship()` — true if either the combination or the
    /// basionym authorship carries any actual value.
    pub fn has_authorship(&self) -> bool {
        self.combination_authorship.exists() || self.basionym_authorship.exists()
    }
}

/// Java `org.gbif.nameparser.api.ParsedName` (flattened with its `ParsedAuthorship` and
/// `CombinedAuthorship` superclasses — see the module doc for why).
///
/// **Field order below is the wire contract, not incidental.** Gson serializes in
/// declaration order, most-derived class first:
///   1. `ParsedName`'s own 16 fields: `rank` .. `type_` (JSON `type`).
///   2. `ParsedAuthorship`'s own 11 fields: `extinct` .. `warnings`.
///   3. `CombinedAuthorship`'s own 3 fields: `combination_authorship` (JSON
///      `combinationAuthorship`) .. `sanctioning_author` (JSON `sanctioningAuthor`).
///
/// Do not reorder these fields without re-checking the plan's Reference section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParsedName {
    // ---- ParsedName's own 16 fields ----
    /// `@Nonnull` in Java, defaulting to `Rank.UNRANKED` — never omitted.
    pub rank: Rank,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<NomCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uninomial: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genus: Option<String>,
    #[serde(rename = "genericAuthorship", skip_serializing_if = "Option::is_none")]
    pub generic_authorship: Option<CombinedAuthorship>,
    #[serde(
        rename = "infragenericEpithet",
        skip_serializing_if = "Option::is_none"
    )]
    pub infrageneric_epithet: Option<String>,
    #[serde(rename = "specificEpithet", skip_serializing_if = "Option::is_none")]
    pub specific_epithet: Option<String>,
    #[serde(rename = "specificAuthorship", skip_serializing_if = "Option::is_none")]
    pub specific_authorship: Option<CombinedAuthorship>,
    #[serde(
        rename = "infraspecificEpithet",
        skip_serializing_if = "Option::is_none"
    )]
    pub infraspecific_epithet: Option<String>,
    #[serde(rename = "cultivarEpithet", skip_serializing_if = "Option::is_none")]
    pub cultivar_epithet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phrase: Option<String>,
    /// Primitive `boolean` in Java — always serializes, including `false`.
    pub candidatus: bool,
    /// `EnumSet<NamePart>` in Java (null when unset) — omitted when `None`, else an
    /// array in ordinal order, e.g. `["SPECIFIC"]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notho: Option<Vec<NamePart>>,
    /// Boxed `Boolean` in Java — omitted when `None`, unlike the primitive `bool`
    /// fields above/below.
    #[serde(rename = "originalSpelling", skip_serializing_if = "Option::is_none")]
    pub original_spelling: Option<bool>,
    /// `Map<NamePart, String>` in Java (an `EnumMap`, iterating in ordinal order) — a
    /// `BTreeMap` reproduces that order on the wire since `NamePart` derives `Ord` in
    /// declaration (== ordinal) order. `serde_json` renders enum map keys as their
    /// `.name()` via `serialize_unit_variant`, matching Gson's `.name()` enum keys.
    #[serde(rename = "epithetQualifier", skip_serializing_if = "Option::is_none")]
    pub epithet_qualifier: Option<BTreeMap<NamePart, String>>,
    /// Java field name is `type`, a Rust keyword — renamed on the wire, not renamed on
    /// the Rust struct (which uses the trailing-underscore convention, matching
    /// `ParseError::type_` elsewhere in this crate).
    #[serde(rename = "type")]
    pub type_: NameType,

    // ---- ParsedAuthorship's own 11 fields ----
    /// Primitive `boolean` in Java — always serializes, including `false`.
    pub extinct: bool,
    #[serde(rename = "taxonomicNote", skip_serializing_if = "Option::is_none")]
    pub taxonomic_note: Option<String>,
    #[serde(rename = "nomenclaturalNote", skip_serializing_if = "Option::is_none")]
    pub nomenclatural_note: Option<String>,
    #[serde(rename = "publishedIn", skip_serializing_if = "Option::is_none")]
    pub published_in: Option<String>,
    /// Boxed `Integer` in Java — a bare JSON number when present, omitted when `None`.
    #[serde(rename = "publishedInYear", skip_serializing_if = "Option::is_none")]
    pub published_in_year: Option<i32>,
    #[serde(rename = "publishedInPage", skip_serializing_if = "Option::is_none")]
    pub published_in_page: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unparsed: Option<String>,
    /// Primitive `boolean` in Java — always serializes, including `false`.
    pub doubtful: bool,
    /// Primitive `boolean` in Java — always serializes, including `false`.
    pub manuscript: bool,
    /// Never null in Java: the field initializer defaults it to `NONE`, and
    /// `ParseContext` unconditionally promotes it to `COMPLETE` before any stage runs —
    /// always serializes.
    pub state: State,
    /// Java `Set<String>` (a `HashSet`, eagerly initialized to `new HashSet<>()`) —
    /// never omitted, serializes as `[]` when empty. Do NOT add
    /// `skip_serializing_if` here. Java's `HashSet` iteration order is not insertion
    /// order; diff this field as a set (sort both sides), never positionally.
    pub warnings: Vec<String>,

    // ---- CombinedAuthorship's own 3 fields, flattened ----
    /// Eagerly-initialized `Authorship` in Java (`= new Authorship()`) — never omitted.
    #[serde(rename = "combinationAuthorship")]
    pub combination_authorship: Authorship,
    /// Eagerly-initialized `Authorship` in Java — never omitted.
    #[serde(rename = "basionymAuthorship")]
    pub basionym_authorship: Authorship,
    #[serde(rename = "sanctioningAuthor", skip_serializing_if = "Option::is_none")]
    pub sanctioning_author: Option<String>,
}

/// Java `ParsedAuthorship`'s private `PUBLISHED_IN_YEAR` pattern (`ParsedAuthorship.java`):
/// `\b(1[5-9]\d{2}|20\d{2}|2100)\b`, compiled with no `Pattern.UNICODE_CHARACTER_CLASS`
/// flag — Java's `\b` is therefore ASCII-only there, ported here as `(?-u:\b)` on both
/// sides rather than the crate's Unicode-default `\b` (this port's per-pattern flag rule;
/// see `pipeline::preflight`'s module doc for the rule spelled out in full).
static PUBLISHED_IN_YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\b(1[5-9]\d{2}|20\d{2}|2100)\b)").unwrap());

impl ParsedName {
    /// Add a warning if not already present — mirrors Java's `warnings` HashSet (deduping).
    /// (The golden harness sorts warnings before diffing, so order is irrelevant; dedup is what matters.)
    pub fn add_warning(&mut self, w: &str) {
        if !self.warnings.iter().any(|x| x == w) {
            self.warnings.push(w.to_string());
        }
    }

    /// Java `ParsedAuthorship.setPublishedIn(String)`. Sets `published_in` verbatim and, as
    /// a side effect, derives `published_in_year` from it: the LAST year-shaped
    /// (1500-2100) match anywhere in the string, replicating `ParsedAuthorship.extractYear`'s
    /// `while (m.find()) year = …;` loop — publication references often list page numbers
    /// (which can themselves look like years, e.g. "1658") before the trailing publication
    /// year, so the last match wins, not the first. No match clears `published_in_year` to
    /// `None`; the field is always recomputed from scratch, never left stale from a
    /// previous call, matching Java's unconditional `this.publishedInYear =
    /// extractYear(publishedIn);`.
    pub fn set_published_in(&mut self, s: &str) {
        self.published_in = Some(s.to_string());
        self.published_in_year = PUBLISHED_IN_YEAR
            .captures_iter(s)
            .last()
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse::<i32>().ok());
    }

    /// Java's inline `publishedIn` APPEND idiom, repeated verbatim at both `StripAndStash`
    /// `stripInAuthorInParens` (step 47) and `stripInAuthorCitation` (step 48): `existing ==
    /// null ? ref : existing + " " + ref` immediately followed by `setPublishedIn(combined)`.
    /// Unlike [`Self::add_nomenclatural_note`]/[`Self::add_taxonomic_note`] (plain
    /// concatenation with no further side effect), appending to `publishedIn` and re-deriving
    /// `publishedInYear` are the SAME Java call, not two — Java never appends to `publishedIn`
    /// without immediately re-running its year extraction over the full combined string, so
    /// this delegates to [`Self::set_published_in`] rather than touching the field directly.
    /// Steps that overwrite instead of appending (`stripIpniCitation`,
    /// `stripPeriodSeparatedReference`, `stripCommaPrefixedReference`) call
    /// [`Self::set_published_in`] directly rather than this.
    pub fn add_published_in(&mut self, reference: &str) {
        let combined = match self.published_in.take() {
            None => reference.to_string(),
            Some(existing) => format!("{existing} {reference}"),
        };
        self.set_published_in(&combined);
    }

    /// Java's inline nomenclatural-note APPEND pattern, repeated verbatim at every
    /// `StripAndStash` step that adds to (rather than replaces) the note: `existing == null
    /// ? note : existing + " " + note` (e.g. `StripAndStash.stripNomNote`,
    /// `stripManuscriptMarker`, `stripInPress`). Deliberately NOT the same as the
    /// `ParsedAuthorship.addNomenclaturalNote(String)` Java *method*, which additionally
    /// blank-checks and trims — those guards live at each Java call site instead (e.g.
    /// `if (!raw.isEmpty()) …`), so callers here are likewise expected to pre-filter blank
    /// notes themselves. Steps that overwrite instead of appending (`stripBracketedNomNote`,
    /// `stripTaxNote`, …) assign the plain `nomenclatural_note`/`taxonomic_note` field
    /// directly rather than calling this.
    pub fn add_nomenclatural_note(&mut self, note: &str) {
        self.nomenclatural_note = Some(match self.nomenclatural_note.take() {
            None => note.to_string(),
            Some(existing) => format!("{existing} {note}"),
        });
    }

    /// Same append semantics as [`Self::add_nomenclatural_note`] (see its doc comment for
    /// the full rationale), for `taxonomic_note` — e.g. `StripAndStash.stripParenTaxNote`,
    /// `stripBracketedTaxNote`, `stripColonConceptReference`.
    pub fn add_taxonomic_note(&mut self, note: &str) {
        self.taxonomic_note = Some(match self.taxonomic_note.take() {
            None => note.to_string(),
            Some(existing) => format!("{existing} {note}"),
        });
    }

    /// Java `ParsedName.addNotho(NamePart)` (`ParsedName.java:306-315`): adds `part` to the
    /// notho set, a no-op if already present (`EnumSet` dedup semantics). Java's
    /// `EnumSet` always iterates in ordinal order regardless of insertion order (unlike a
    /// general `HashSet`) — reproduced here by keeping the backing `Vec` sorted by
    /// `NamePart`'s `Ord` (declaration order, matching Java's ordinal order — see
    /// `NamePart`'s own doc comment) after every insert, rather than merely appending in
    /// call order.
    pub fn add_notho(&mut self, part: NamePart) {
        match &mut self.notho {
            None => self.notho = Some(vec![part]),
            Some(set) => {
                if !set.contains(&part) {
                    set.push(part);
                    set.sort();
                }
            }
        }
    }

    /// Java `ParsedName.setNotho(NamePart)` (`ParsedName.java:302-304`): REPLACES the
    /// whole notho set with a single-element set containing just `part` — an overwrite,
    /// not an insert like [`Self::add_notho`]. This asymmetry is load-bearing:
    /// `NameTokens`'s post-loop `if (inlineRankNotho) setNotho(INFRASPECIFIC)` erases any
    /// earlier `addNotho(GENERIC)` recorded from a `HYBRID_MARK` token — reproduced
    /// verbatim, not "fixed" into an add. (Java's signature takes a nullable `NamePart`
    /// and folds a `null` argument to clearing the field entirely; every real call site —
    /// including `NameTokens`'s own `setNotho(NamePart.INFRASPECIFIC)` — passes a
    /// non-null literal, so this port's signature takes `part: NamePart` directly.)
    pub fn set_notho(&mut self, part: NamePart) {
        self.notho = Some(vec![part]);
    }

    /// Java `ParsedName.setEpithetQualifier(NamePart, String)` (`ParsedName.java:355-362`):
    /// inserts into the qualifier map, creating it on first use. (Java guards both
    /// arguments against `null` before inserting; every real call site passes non-null
    /// literals, so this port's signature takes `part: NamePart, q: &str` directly,
    /// matching [`Self::set_notho`]'s same non-nullable-signature rationale.)
    pub fn set_epithet_qualifier(&mut self, part: NamePart, q: &str) {
        self.epithet_qualifier
            .get_or_insert_with(BTreeMap::new)
            .insert(part, q.to_string());
    }

    /// Java `ParsedName.isAutonym()` (`ParsedName.java:397-399`): true when the specific
    /// and infraspecific epithets are both set and identical (ICN Art. 26.1's autonym rule
    /// — e.g. "Rosa gallica var. gallica"). The `Option` equality below already folds
    /// Java's three-part `specificEpithet != null && infraspecificEpithet != null &&
    /// specificEpithet.equals(infraspecificEpithet)`: the `is_some()` guard rules out the
    /// vacuous `None == None` case, and `Some(x) == Some(y)` iff `x == y`.
    pub fn is_autonym(&self) -> bool {
        self.specific_epithet.is_some() && self.specific_epithet == self.infraspecific_epithet
    }

    /// Java `CombinedAuthorshipIF.hasAuthorship()`/`CombinedAuthorship.hasAuthorship()` —
    /// inherited by `ParsedName` through Java's `ParsedName extends ParsedAuthorship
    /// extends CombinedAuthorship` chain. Ported directly onto `ParsedName` here since this
    /// port flattens `CombinedAuthorship`'s fields onto `ParsedName` (see the module doc)
    /// rather than modeling the inheritance hierarchy. See [`CombinedAuthorship::has_authorship`]
    /// for the identical logic used by the *nested* `generic_authorship`/
    /// `specific_authorship` slots.
    pub fn has_authorship(&self) -> bool {
        self.combination_authorship.exists() || self.basionym_authorship.exists()
    }
}

impl Default for ParsedName {
    /// Matches Java `ParseContext`'s seeding of a fresh `ParsedName` before any pipeline
    /// stage runs: `rank = Rank.UNRANKED`, `type = NameType.SCIENTIFIC`,
    /// `state = ParsedName.State.COMPLETE`. Every other field takes Rust's natural zero
    /// value (`None` / empty `Vec` / `false`), matching Java's `null` fields and
    /// eagerly-initialized-empty-collection fields respectively.
    fn default() -> Self {
        ParsedName {
            rank: Rank::Unranked,
            code: None,
            uninomial: None,
            genus: None,
            generic_authorship: None,
            infrageneric_epithet: None,
            specific_epithet: None,
            specific_authorship: None,
            infraspecific_epithet: None,
            cultivar_epithet: None,
            phrase: None,
            candidatus: false,
            notho: None,
            original_spelling: None,
            epithet_qualifier: None,
            type_: NameType::Scientific,
            extinct: false,
            taxonomic_note: None,
            nomenclatural_note: None,
            published_in: None,
            published_in_year: None,
            published_in_page: None,
            unparsed: None,
            doubtful: false,
            manuscript: false,
            state: State::Complete,
            warnings: Vec::new(),
            combination_authorship: Authorship::default(),
            basionym_authorship: Authorship::default(),
            sanctioning_author: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wire-format golden test — reference row 1 (plan's Reference section, copied
    /// verbatim): the Java CLI oracle's `parsed` object for
    /// `Vulpes vulpes silaceus Miller, 1907`. This is the wire-format contract: if any
    /// field's presence, order, or omission differs, this assertion fails byte-exactly
    /// rather than approximately.
    #[test]
    fn wire_matches_reference_row_1_vulpes_vulpes_silaceus() {
        let pn = ParsedName {
            rank: Rank::Subspecies,
            code: Some(NomCode::Zoological),
            genus: Some("Vulpes".to_string()),
            specific_epithet: Some("vulpes".to_string()),
            infraspecific_epithet: Some("silaceus".to_string()),
            combination_authorship: Authorship {
                authors: vec!["Miller".to_string()],
                year: Some("1907".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let expected = r#"{"rank":"SUBSPECIES","code":"ZOOLOGICAL","genus":"Vulpes","specificEpithet":"vulpes","infraspecificEpithet":"silaceus","candidatus":false,"type":"SCIENTIFIC","extinct":false,"doubtful":false,"manuscript":false,"state":"COMPLETE","warnings":[],"combinationAuthorship":{"authors":["Miller"],"exAuthors":[],"year":"1907"},"basionymAuthorship":{"authors":[],"exAuthors":[]}}"#;
        assert_eq!(serde_json::to_string(&pn).unwrap(), expected);
    }

    /// Wire-format golden test — reference row 2: the Java CLI oracle's `parsed` object
    /// for `Abies alba Mill.`. Not printed verbatim in the plan's Reference section
    /// text, so generated directly from the Java oracle (the shaded CLI jar) to stay
    /// authoritative: `java -jar name-parser-cli-4.2.0-SNAPSHOT-shaded.jar parse
    /// --input=<2-line file with row 1 then this row> --output=- --format=jsonl`, whose
    /// row-1 output was first confirmed byte-identical to the plan's printed reference
    /// before trusting this row's freshly-generated output as ground truth.
    #[test]
    fn wire_matches_reference_row_2_abies_alba() {
        let pn = ParsedName {
            rank: Rank::Species,
            genus: Some("Abies".to_string()),
            specific_epithet: Some("alba".to_string()),
            combination_authorship: Authorship {
                authors: vec!["Mill.".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let expected = r#"{"rank":"SPECIES","genus":"Abies","specificEpithet":"alba","candidatus":false,"type":"SCIENTIFIC","extinct":false,"doubtful":false,"manuscript":false,"state":"COMPLETE","warnings":[],"combinationAuthorship":{"authors":["Mill."],"exAuthors":[]},"basionymAuthorship":{"authors":[],"exAuthors":[]}}"#;
        assert_eq!(serde_json::to_string(&pn).unwrap(), expected);
    }

    #[test]
    fn parsed_name_default_seeds_match_parse_context() {
        let pn = ParsedName::default();
        assert_eq!(pn.rank, Rank::Unranked);
        assert_eq!(pn.type_, NameType::Scientific);
        assert_eq!(pn.state, State::Complete);
        // Every remaining field is Rust's natural zero value.
        assert_eq!(pn.code, None);
        assert!(!pn.candidatus);
        assert!(!pn.extinct);
        assert!(!pn.doubtful);
        assert!(!pn.manuscript);
        assert!(pn.warnings.is_empty());
        assert_eq!(pn.combination_authorship, Authorship::default());
        assert_eq!(pn.basionym_authorship, Authorship::default());
    }

    #[test]
    fn default_parsed_name_serializes_with_all_optionals_omitted() {
        // Sanity check independent of the two golden rows above: a bare `default()`
        // must omit every Option field and every nested-struct Option, while still
        // emitting the always-on primitives/collections/enums.
        let expected = r#"{"rank":"UNRANKED","candidatus":false,"type":"SCIENTIFIC","extinct":false,"doubtful":false,"manuscript":false,"state":"COMPLETE","warnings":[],"combinationAuthorship":{"authors":[],"exAuthors":[]},"basionymAuthorship":{"authors":[],"exAuthors":[]}}"#;
        assert_eq!(
            serde_json::to_string(&ParsedName::default()).unwrap(),
            expected
        );
    }

    #[test]
    fn epithet_qualifier_btreemap_key_renders_as_name_and_sorts_ordinally() {
        // Not one of the two golden rows (both leave epithetQualifier unset) — this
        // only defines and exercises the *shape* of the populated case; full parity
        // with Java's populated output is validated separately, by the full-corpus
        // golden harness (`tests/parse_golden.rs`).
        let mut map = BTreeMap::new();
        map.insert(NamePart::Specific, "cf.".to_string());
        map.insert(NamePart::Generic, "aff.".to_string());
        let pn = ParsedName {
            epithet_qualifier: Some(map),
            ..Default::default()
        };
        let json = serde_json::to_string(&pn).unwrap();
        assert!(json.contains(r#""epithetQualifier":{"GENERIC":"aff.","SPECIFIC":"cf."}"#));
    }

    #[test]
    fn notho_serializes_as_array_when_present_and_is_omitted_when_none() {
        assert!(!serde_json::to_string(&ParsedName::default())
            .unwrap()
            .contains("notho"));
        let pn = ParsedName {
            notho: Some(vec![NamePart::Specific]),
            ..Default::default()
        };
        assert!(serde_json::to_string(&pn)
            .unwrap()
            .contains(r#""notho":["SPECIFIC"]"#));
    }

    #[test]
    fn add_warning_dedups_like_javas_hashset() {
        // Java's `warnings` field is a `HashSet<String>`, so adding the same warning
        // constant from multiple pipeline stages must still yield exactly one entry on
        // the wire — a plain unconditional `Vec::push` would duplicate it instead.
        let mut pn = ParsedName::default();
        pn.add_warning("some warning");
        pn.add_warning("some warning");
        assert_eq!(pn.warnings, vec!["some warning".to_string()]);
    }

    #[test]
    fn set_published_in_stores_the_reference_verbatim_and_derives_the_year() {
        let mut pn = ParsedName::default();
        pn.set_published_in("Annals and Magazine of Natural History 1988");
        assert_eq!(
            pn.published_in,
            Some("Annals and Magazine of Natural History 1988".to_string())
        );
        assert_eq!(pn.published_in_year, Some(1988));
    }

    #[test]
    fn set_published_in_with_no_year_leaves_published_in_year_none() {
        let mut pn = ParsedName::default();
        pn.set_published_in("no year here");
        assert_eq!(pn.published_in, Some("no year here".to_string()));
        assert_eq!(pn.published_in_year, None);
    }

    #[test]
    fn set_published_in_takes_the_last_year_shaped_match_not_a_page_number() {
        // "1658" and "1662" are a page range, but both happen to be shaped like years
        // (1500-1999) too. Java's extractYear keeps overwriting through every match of
        // its `while (m.find())` loop, so the true trailing publication year (1988) must
        // win over the earlier page-range numbers — this is the whole reason "last match"
        // rather than "first match" is the correct semantics.
        let mut pn = ParsedName::default();
        pn.set_published_in("75: 1658-1662 fig. 1988");
        assert_eq!(pn.published_in_year, Some(1988));
    }

    #[test]
    fn set_published_in_recomputes_the_year_on_every_call_not_just_the_first() {
        // Regression guard: a naive "only set if currently None" implementation would
        // leave a stale year from an earlier call. Java always recomputes from scratch.
        let mut pn = ParsedName::default();
        pn.set_published_in("Author, 1988");
        assert_eq!(pn.published_in_year, Some(1988));
        pn.set_published_in("Author, no year this time");
        assert_eq!(
            pn.published_in,
            Some("Author, no year this time".to_string())
        );
        assert_eq!(
            pn.published_in_year, None,
            "must be cleared, not left stale from the previous call"
        );
    }

    #[test]
    fn add_published_in_stores_verbatim_and_derives_the_year_on_first_call() {
        let mut pn = ParsedName::default();
        pn.add_published_in("Fourcroy, 1785");
        assert_eq!(pn.published_in, Some("Fourcroy, 1785".to_string()));
        assert_eq!(pn.published_in_year, Some(1785));
    }

    #[test]
    fn add_published_in_appends_with_a_space_separator_and_rederives_the_year() {
        // Regression guard for the "append IS setPublishedIn on the combined string, not two
        // separate operations" semantics: the year must come from the LAST year-shaped match
        // in the COMBINED string, not be left over from the first call or derived from only
        // the newly-appended part.
        let mut pn = ParsedName::default();
        pn.add_published_in("Fourcroy, 1785");
        pn.add_published_in("Smith, 1900");
        assert_eq!(
            pn.published_in,
            Some("Fourcroy, 1785 Smith, 1900".to_string())
        );
        assert_eq!(pn.published_in_year, Some(1900));
    }

    #[test]
    fn add_published_in_on_a_reference_with_no_year_clears_a_stale_year() {
        let mut pn = ParsedName::default();
        pn.set_published_in("Author, 1988");
        assert_eq!(pn.published_in_year, Some(1988));
        // A fresh ParsedName (no prior publishedIn) appending a year-free reference: the
        // combined string is just the new reference, so the year must be None, not stale.
        let mut pn2 = ParsedName::default();
        pn2.add_published_in("Fleisch.");
        assert_eq!(pn2.published_in, Some("Fleisch.".to_string()));
        assert_eq!(pn2.published_in_year, None);
    }

    #[test]
    fn add_nomenclatural_note_appends_with_a_space_separator() {
        let mut pn = ParsedName::default();
        assert_eq!(pn.nomenclatural_note, None);
        pn.add_nomenclatural_note("a");
        assert_eq!(pn.nomenclatural_note, Some("a".to_string()));
        pn.add_nomenclatural_note("b");
        assert_eq!(pn.nomenclatural_note, Some("a b".to_string()));
    }

    #[test]
    fn add_taxonomic_note_appends_with_a_space_separator() {
        let mut pn = ParsedName::default();
        assert_eq!(pn.taxonomic_note, None);
        pn.add_taxonomic_note("a");
        assert_eq!(pn.taxonomic_note, Some("a".to_string()));
        pn.add_taxonomic_note("b");
        assert_eq!(pn.taxonomic_note, Some("a b".to_string()));
    }

    #[test]
    fn authorship_exists_ignores_ex_authors_like_java() {
        let a = Authorship {
            authors: vec![],
            ex_authors: vec!["hort.".into()],
            year: None,
            imprint_year: None,
        };
        assert!(
            !a.exists(),
            "only ex-authors present must be exists()==false, matching Java isEmpty()"
        );
        let b = Authorship {
            authors: vec!["L.".into()],
            ..Default::default()
        };
        assert!(b.exists());
    }

    #[test]
    fn published_in_year_ignores_non_ascii_digits_like_java() {
        // ASCII \d only (Java parity): a stray non-ASCII digit run must not shadow the real year.
        let mut pn = ParsedName::default();
        pn.set_published_in("Author, 1900, republished ref. 19\u{0668}8 variant");
        assert_eq!(pn.published_in_year, Some(1900));
    }

    #[test]
    fn add_notho_inserts_and_dedups_like_javas_enumset() {
        let mut pn = ParsedName::default();
        pn.add_notho(NamePart::Specific);
        assert_eq!(pn.notho, Some(vec![NamePart::Specific]));
        pn.add_notho(NamePart::Specific);
        assert_eq!(
            pn.notho,
            Some(vec![NamePart::Specific]),
            "adding the same part twice must not duplicate it"
        );
    }

    #[test]
    fn add_notho_keeps_ordinal_order_regardless_of_insertion_order() {
        let mut pn = ParsedName::default();
        pn.add_notho(NamePart::Infraspecific);
        pn.add_notho(NamePart::Generic);
        assert_eq!(
            pn.notho,
            Some(vec![NamePart::Generic, NamePart::Infraspecific]),
            "Java's EnumSet always iterates in ordinal order regardless of insertion order"
        );
    }

    #[test]
    fn set_notho_overwrites_rather_than_adds() {
        let mut pn = ParsedName::default();
        pn.add_notho(NamePart::Generic);
        pn.set_notho(NamePart::Infraspecific);
        assert_eq!(
            pn.notho,
            Some(vec![NamePart::Infraspecific]),
            "set_notho must REPLACE the whole set, erasing the earlier add_notho result — \
             the load-bearing overwrite asymmetry"
        );
    }

    #[test]
    fn set_epithet_qualifier_creates_the_map_on_first_use() {
        let mut pn = ParsedName::default();
        assert_eq!(pn.epithet_qualifier, None);
        pn.set_epithet_qualifier(NamePart::Specific, "cf.");
        let mut expected = BTreeMap::new();
        expected.insert(NamePart::Specific, "cf.".to_string());
        assert_eq!(pn.epithet_qualifier, Some(expected));
    }

    #[test]
    fn set_epithet_qualifier_inserts_additional_parts_into_the_existing_map() {
        let mut pn = ParsedName::default();
        pn.set_epithet_qualifier(NamePart::Specific, "cf.");
        pn.set_epithet_qualifier(NamePart::Infraspecific, "aff.");
        let mut expected = BTreeMap::new();
        expected.insert(NamePart::Specific, "cf.".to_string());
        expected.insert(NamePart::Infraspecific, "aff.".to_string());
        assert_eq!(pn.epithet_qualifier, Some(expected));
    }

    #[test]
    fn set_epithet_qualifier_overwrites_an_existing_part() {
        let mut pn = ParsedName::default();
        pn.set_epithet_qualifier(NamePart::Specific, "cf.");
        pn.set_epithet_qualifier(NamePart::Specific, "aff.");
        let mut expected = BTreeMap::new();
        expected.insert(NamePart::Specific, "aff.".to_string());
        assert_eq!(pn.epithet_qualifier, Some(expected));
    }
}
