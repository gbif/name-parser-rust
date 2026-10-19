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
/// basionym: authors, ex-authors, in-authors and the year.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Authorship {
    /// Eagerly-initialized `List<String>` in Java (`= new ArrayList<>()`) — never
    /// omitted, serializes as `[]` when empty.
    pub authors: Vec<String>,
    /// Eagerly-initialized `List<String>` in Java — never omitted, serializes as `[]`
    /// when empty.
    #[serde(rename = "exAuthors")]
    pub ex_authors: Vec<String>,
    /// Java `Authorship.inAuthors` (name-parser-api 5.6): the authors of the work the name was
    /// published in, "Natterer in Kuhl, 1817" → `["Kuhl"]` (ICN Art. 46.2, ICZN Recommendation
    /// 51E). Set only when the `in` citation is an author team, not a reference ("Mill. in Gard.
    /// Dict. ed. 8" has none); the whole citation stays in [`ParsedName::published_in`] as well.
    /// Omitted when empty, as Java leaves the list null until set.
    #[serde(rename = "inAuthors", skip_serializing_if = "Vec::is_empty")]
    pub in_authors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    /// Java `Authorship.imprintYear`: the year printed on the work when it differs from the actual
    /// publication year, cited after it (`Storr, 1970 [1969]` → year 1970, imprint year 1969). A
    /// bracketed year on its own is the actual year, established from external evidence — the
    /// cataloguers' convention, `Cabanis [1851]` → year 1851, no imprint year. ICZN Recommendation
    /// 22A.2.3 brackets only an imprint year cited after the actual one.
    #[serde(rename = "imprintYear", skip_serializing_if = "Option::is_none")]
    pub imprint_year: Option<String>,
    /// Java `Authorship.bracketedYear` (name-parser-api 5.4): the [`Self::year`] is not given in the
    /// work itself but established from external evidence, cited in square brackets: `Westwood,
    /// [1851]` (ICZN Recommendation 22A.2.3). Always serialized, like Java's primitive boolean.
    #[serde(rename = "bracketedYear")]
    pub bracketed_year: bool,
    /// Java `Authorship.anonymous` (name-parser-api 5.1): the work was published anonymously,
    /// "Anon." / "anon." / "Anonymous". Any [`Self::authors`] are then attributed from external
    /// evidence, cited in square brackets: `[Denis & Schiffermüller], 1775` (ICZN Recommendation
    /// 51D). Always serialized, like Java's primitive boolean.
    pub anonymous: bool,
    /// Java `Authorship.sanctioningAuthor` (name-parser-api 5.2): the sanctioning author of a
    /// sanctioned fungal name (ICN Art. 15), Fries or Persoon, cited after a colon — for a basionym
    /// inside its brackets: `Merulius lacrimans (Wulfen : Fr.) Schum.`.
    #[serde(rename = "sanctioningAuthor", skip_serializing_if = "Option::is_none")]
    pub sanctioning_author: Option<String>,
}

impl Authorship {
    /// Java `Authorship.exists()` = `!isEmpty()`, and Java `isEmpty()` checks ONLY
    /// authors, year and the anonymous flag (NOT exAuthors) — see Authorship.java.
    pub fn exists(&self) -> bool {
        !self.authors.is_empty() || self.year.is_some() || self.anonymous
    }

    /// True if there are authors or the work is anonymous: an anonymous author takes an author's
    /// place ("(Fr.) anon." is a recombination).
    pub fn has_authors_or_anon(&self) -> bool {
        !self.authors.is_empty() || self.anonymous
    }

    /// Java `Authorship.hasAuthors()`: true if `authors` is non-empty. (Java's own
    /// null-guard, `authors != null && …`, is unreachable here — `Vec<String>` is eagerly
    /// initialized and never absent, matching the field's Java default `= new
    /// ArrayList<>()`.) Distinct from [`Self::exists`] (which additionally counts a bare year
    /// with no authors).
    pub fn has_authors(&self) -> bool {
        !self.authors.is_empty()
    }
}

/// Java `org.gbif.nameparser.api.CombinedAuthorship`. Bundles the combination
/// authorship, the basionym authorship, and the (fungal) sanctioning author.
///
/// Used standalone as the type of [`ParsedName::generic_authorship`] /
/// [`ParsedName::specific_authorship`]. Its own two fields are, in addition,
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
///   3. `CombinedAuthorship`'s own 2 fields: `combination_authorship` (JSON
///      `combinationAuthorship`) and `basionym_authorship` (JSON `basionymAuthorship`), each
///      with its own sanctioning author (name-parser-api 5.2 removed the combination's from
///      `CombinedAuthorship`).
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
    /// The provisional designation of an INFORMAL name, verbatim from its rank marker on
    /// (`"sp. RMCC TR1811"`, `"sp. 1"`, `"group"`). Nothing in it is interpreted, so it may hold
    /// what looks like an author citation (`"sp. Forster, 1968"`): the parser cannot reliably tell
    /// a citation from a specimen or culture code there (43% of the tails the pre-5.0.0 rule read
    /// as authorship were spurious — impossible years, collection acronyms like `ZRC`/`MNHN`), and
    /// on the genuinely authored minority the citation belongs to the anchor
    /// ([`Self::anchor`]) rather than to the undetermined taxon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phrase: Option<String>,
    /// Primitive `boolean` in Java — always serializes, including `false`.
    pub candidatus: bool,
    /// Java `ParsedName.graftChimaera` (name-parser-api 6.0): a graft-chimaera, tissue of two
    /// genera grown together at a graft, not a hybrid, named under the cultivated-plants code. Its
    /// genus is written with a plus sign (`+Crataegomespilus`, `+Laburnocytisus adamii`), and it
    /// carries no [`Self::notho`]. Primitive `boolean` in Java — always serializes, like
    /// `candidatus`.
    #[serde(rename = "graftChimaera")]
    pub graft_chimaera: bool,
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

    // ---- CombinedAuthorship's own 2 fields, flattened ----
    /// Eagerly-initialized `Authorship` in Java (`= new Authorship()`) — never omitted.
    #[serde(rename = "combinationAuthorship")]
    pub combination_authorship: Authorship,
    /// Eagerly-initialized `Authorship` in Java — never omitted.
    #[serde(rename = "basionymAuthorship")]
    pub basionym_authorship: Authorship,
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
    /// not an insert like [`Self::add_notho`]. The parser itself only adds: Java's
    /// `NameTokens` called this for a notho rank marker and so erased a hybrid sign before
    /// the genus or species, which this port does not reproduce. (Java's signature takes a
    /// nullable `NamePart` and folds a `null` argument to clearing the field entirely; this
    /// port's signature takes `part: NamePart` directly.)
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

    /// Java `ParsedName.isIndetermined()`: the name lacks an epithet its rank requires — a species
    /// without its specific epithet, an infraspecific name without its infraspecific one. A phrase
    /// does not stand in for the missing epithet: `Rhizobium sp.` and `Serratia sp. RE1-2a` are both
    /// indetermined species, as their [`Self::anchor`] is only a genus.
    pub fn is_indetermined(&self) -> bool {
        let r = self.rank;
        // a convariety's epithet is an infraspecific one, not a cultivar epithet
        let cultivar_rank =
            r.is_restricted_to_code() == Some(NomCode::Cultivars) && r != Rank::Convariety;
        r.is_infrageneric_strictly()
            && self.uninomial.is_none()
            && self.infrageneric_epithet.is_none()
            && self.specific_epithet.is_none()
            || r.is_species_or_below() && !cultivar_rank && self.specific_epithet.is_none()
            || r.is_infraspecific() && !cultivar_rank && self.infraspecific_epithet.is_none()
            || cultivar_rank && self.cultivar_epithet.is_none()
    }

    /// Java `ParsedName.anchor()`: the determined part of this name as a name of its own — the real
    /// taxon an informal name hangs off. `Rhizobium sp. RMCC TR1811` is anchored on the genus
    /// *Rhizobium*, `Lasioglossum (Dialictus) sp. 1` on the subgenus *Lasioglossum (Dialictus)*,
    /// `Grevillea brachystylis subsp. Busselton (G.J.Keighery s.n. 28/8/1985)` on the species
    /// *Grevillea brachystylis* and `Abies cf. alba` on the genus *Abies*.
    ///
    /// The anchor keeps the name parts down to the deepest one that is present and not qualified
    /// (cf., aff., ?, ...). A qualified genus is the exception: `?Sydonia alba`, `Ferganoconcha?
    /// oblonga` and `cf. Craticula` are anchored on that genus, as no part of the name is surer.
    /// The anchor takes the rank of its deepest part and the authorship that belongs to it: the
    /// specific authorship for a species anchor of an infraspecific name, the generic one for a
    /// genus, the name's own when nothing below the anchor is given. The phrase, qualifiers, notes,
    /// publication, warnings and flags other than `candidatus`, `graft_chimaera` and `extinct` are
    /// left out. A complete
    /// scientific name is its own anchor, without those details. `None` when no part of the name
    /// is determined or its type is not parsable. A placeholder is anchored on the real taxon it
    /// hangs off (`Chromista incertae sedis` on *Chromista*, #185), not on the `?` that stands in
    /// for a missing genus (`? alba`).
    pub fn anchor(&self) -> Option<ParsedName> {
        if !self.type_.is_parsable() && self.type_ != NameType::Placeholder {
            return None;
        }
        if self.genus.as_deref() == Some("?") || self.uninomial.as_deref() == Some("?") {
            return None;
        }
        let qualified = |p: NamePart| {
            self.epithet_qualifier
                .as_ref()
                .is_some_and(|q| q.contains_key(&p))
        };
        // qualified, the genus anchors the name all the same, but alone: `?Sydonia alba`
        let qualified_genus = qualified(NamePart::Generic);
        let mut a = ParsedName {
            type_: NameType::Scientific,
            code: self.code,
            candidatus: self.candidatus,
            graft_chimaera: self.graft_chimaera,
            extinct: self.extinct,
            ..ParsedName::default()
        };
        // the deepest name part given, phrase and cultivar epithet aside
        let deepest = if self.infraspecific_epithet.is_some() {
            NamePart::Infraspecific
        } else if self.specific_epithet.is_some() {
            NamePart::Specific
        } else if self.infrageneric_epithet.is_some() {
            NamePart::Infrageneric
        } else {
            NamePart::Generic
        };
        let own_authorship = |a: &mut ParsedName| {
            a.combination_authorship = self.combination_authorship.clone();
            a.basionym_authorship = self.basionym_authorship.clone();
        };

        let Some(genus) = self.genus.clone() else {
            if let Some(uninomial) = self.uninomial.clone() {
                a.uninomial = Some(uninomial);
                a.rank = if self.rank.not_other_or_unranked()
                    && !self.rank.is_species_or_below()
                    && !self.rank.is_infrageneric_strictly()
                {
                    self.rank
                } else {
                    Rank::Unranked
                };
                self.keep_notho(&mut a, NamePart::Generic);
                if deepest == NamePart::Generic {
                    own_authorship(&mut a);
                }
                return Some(a);
            }
            // an infrageneric name without its genus, "subgen. Trematostoma"
            let infrageneric = self.infrageneric_epithet.clone()?;
            if qualified(NamePart::Infrageneric) {
                return None;
            }
            a.infrageneric_epithet = Some(infrageneric);
            a.rank = if self.rank.is_infrageneric_strictly() {
                self.rank
            } else {
                Rank::InfragenericName
            };
            self.keep_notho(&mut a, NamePart::Infrageneric);
            if deepest == NamePart::Infrageneric {
                own_authorship(&mut a);
            }
            return Some(a);
        };

        let mut kept = NamePart::Generic;
        if !qualified_genus {
            if self.infrageneric_epithet.is_some() && !qualified(NamePart::Infrageneric) {
                kept = NamePart::Infrageneric;
            }
            let cut = self.infrageneric_epithet.is_some() && kept == NamePart::Generic;
            if !cut && self.specific_epithet.is_some() && !qualified(NamePart::Specific) {
                kept = NamePart::Specific;
                if self.infraspecific_epithet.is_some() && !qualified(NamePart::Infraspecific) {
                    kept = NamePart::Infraspecific;
                }
            }
        }
        // nothing cut and nothing missing: the name is its own anchor, at its own rank
        let complete = kept == deepest && !self.is_indetermined();

        match kept {
            NamePart::Generic => {
                a.uninomial = Some(genus);
                a.rank = Rank::Genus;
            }
            NamePart::Infrageneric => {
                a.genus = Some(genus);
                a.infrageneric_epithet = self.infrageneric_epithet.clone();
                // a bracketed name between genus and species is a subgenus; a botanical one, perhaps
                // a section, has no rank to tell (`Abies sect. Grandes sp. 1`, #176)
                a.rank = if self.rank.is_infrageneric_strictly() {
                    self.rank
                } else if self.code == Some(NomCode::Botanical) {
                    Rank::InfragenericName
                } else {
                    Rank::Subgenus
                };
            }
            NamePart::Specific | NamePart::Infraspecific => {
                a.genus = Some(genus);
                a.infrageneric_epithet = self.infrageneric_epithet.clone();
                a.specific_epithet = self.specific_epithet.clone();
                if kept == NamePart::Infraspecific {
                    a.infraspecific_epithet = self.infraspecific_epithet.clone();
                    a.rank = if self.rank.is_infraspecific() {
                        self.rank
                    } else {
                        Rank::InfraspecificName
                    };
                } else {
                    a.rank = if complete && self.rank.is_species_or_below() {
                        self.rank
                    } else {
                        Rank::Species
                    };
                }
            }
        }
        if complete {
            a.cultivar_epithet = self.cultivar_epithet.clone();
        }
        for p in [
            NamePart::Generic,
            NamePart::Infrageneric,
            NamePart::Specific,
            NamePart::Infraspecific,
        ] {
            if p <= kept {
                self.keep_notho(&mut a, p);
            }
        }
        if kept != NamePart::Generic {
            a.generic_authorship = self.generic_authorship.clone();
        }
        if kept == NamePart::Infraspecific {
            a.specific_authorship = self.specific_authorship.clone();
        }
        // the authorship of the anchor's own deepest part
        let authored =
            |ca: &Option<CombinedAuthorship>| ca.clone().filter(CombinedAuthorship::has_authorship);
        if let Some(sa) = authored(&self.specific_authorship).filter(|_| kept == NamePart::Specific)
        {
            a.combination_authorship = sa.combination_authorship;
            a.basionym_authorship = sa.basionym_authorship;
        } else if let Some(ga) =
            authored(&self.generic_authorship).filter(|_| kept == NamePart::Generic)
        {
            a.combination_authorship = ga.combination_authorship;
            a.basionym_authorship = ga.basionym_authorship;
        } else if kept == deepest {
            own_authorship(&mut a);
        }
        Some(a)
    }

    fn keep_notho(&self, anchor: &mut ParsedName, part: NamePart) {
        if self.notho.as_ref().is_some_and(|n| n.contains(&part)) {
            anchor.add_notho(part);
        }
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
            graft_chimaera: false,
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
        let expected = r#"{"rank":"SUBSPECIES","code":"ZOOLOGICAL","genus":"Vulpes","specificEpithet":"vulpes","infraspecificEpithet":"silaceus","candidatus":false,"graftChimaera":false,"type":"SCIENTIFIC","extinct":false,"doubtful":false,"manuscript":false,"state":"COMPLETE","warnings":[],"combinationAuthorship":{"authors":["Miller"],"exAuthors":[],"year":"1907","bracketedYear":false,"anonymous":false},"basionymAuthorship":{"authors":[],"exAuthors":[],"bracketedYear":false,"anonymous":false}}"#;
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
        let expected = r#"{"rank":"SPECIES","genus":"Abies","specificEpithet":"alba","candidatus":false,"graftChimaera":false,"type":"SCIENTIFIC","extinct":false,"doubtful":false,"manuscript":false,"state":"COMPLETE","warnings":[],"combinationAuthorship":{"authors":["Mill."],"exAuthors":[],"bracketedYear":false,"anonymous":false},"basionymAuthorship":{"authors":[],"exAuthors":[],"bracketedYear":false,"anonymous":false}}"#;
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
        let expected = r#"{"rank":"UNRANKED","candidatus":false,"graftChimaera":false,"type":"SCIENTIFIC","extinct":false,"doubtful":false,"manuscript":false,"state":"COMPLETE","warnings":[],"combinationAuthorship":{"authors":[],"exAuthors":[],"bracketedYear":false,"anonymous":false},"basionymAuthorship":{"authors":[],"exAuthors":[],"bracketedYear":false,"anonymous":false}}"#;
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
            in_authors: vec!["Kuhl".into()],
            year: None,
            imprint_year: None,
            bracketed_year: false,
            anonymous: false,
            sanctioning_author: None,
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
        // an anonymous authorship exists without authors or year, like Java's isEmpty()
        let c = Authorship {
            anonymous: true,
            ..Default::default()
        };
        assert!(c.exists());
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
            "set_notho must REPLACE the whole set, erasing the earlier add_notho result"
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
