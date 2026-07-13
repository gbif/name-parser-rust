// SPDX-License-Identifier: Apache-2.0

//! Port of Java `org.gbif.nameparser.util.NameFormatter` — renders a [`ParsedName`] back
//! into a formatted scientific-name string.
//!
//! The Java class exposes a family of convenience methods (`canonical`,
//! `canonicalWithoutAuthorship`, `canonicalMinimal`, `canonicalComplete`,
//! `canonicalCompleteHtml`, `authorshipComplete`, `authorString`) that all delegate to one
//! private `buildName(ParsedName, 16 booleans)`. This port keeps that exact shape: the 16
//! positional booleans become a named [`Flags`] struct (safer to read than 16 positional
//! `bool`s while preserving the same semantics), each public method fills the same flag
//! values Java passes, and [`build_name`] mirrors the Java control flow statement for
//! statement. The public entry points hang off [`ParsedName`] as methods so bindings can
//! call e.g. `pn.canonical_name()`.
//!
//! Cross-validated byte-for-byte against Java `NameFormatter` over the full corpus by
//! `tests/format_golden.rs` (the Java oracle emits the same five renderings per name).

use std::sync::LazyLock;

use regex::Regex;
use unicode_normalization::UnicodeNormalization;

use crate::model::enums::{NamePart, NameType, NomCode, Rank};
use crate::model::name::{Authorship, CombinedAuthorship, ParsedName};
use crate::model::Informal;
use crate::unicode::java_trim;

const HYBRID_MARKER: char = '×';
const NOTHO_PREFIX: &str = "notho";
const ET_AL: &str = "et al.";
const ITALICS_OPEN: &str = "<i>";
const ITALICS_CLOSE: &str = "</i>";

/// Java `NameFormatter.AL` = `^al\.?$`, tested with `Matcher.find()`. Detects a trailing
/// "al"/"al." author so `X, al.` renders with " et al." rather than " & al.". The `^…$`
/// anchors make find() a whole-string test for every real author token; the one Java/Rust
/// `$` difference (Java `$` also matches just before a trailing `\n`, Rust `$` = `\z` does
/// not) is unreachable — author tokens never carry a trailing line terminator.
static AL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^al\.?$").unwrap());

/// Java `NameFormatter.PHRASE_SPECIES_MARKER` = `^(?:species|spec|sp)\b.*`
/// (CASE_INSENSITIVE, tested with `Matcher.matches()`). Java's `\b` here is ASCII (no
/// UNICODE_CHARACTER_CLASS flag on this pattern), so it is pinned to `(?-u:\b)`. Detects a
/// phrase that already spells out the species marker as its leading word ("species 1"), so
/// the formatter must not also synthesise an "sp." marker.
static PHRASE_SPECIES_MARKER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:species|spec|sp)(?-u:\b).*$").unwrap());

/// Java's inline author-tail pattern `(?U)[\p{Lu}](?:\.[\p{Lu}])*\..+`, tested with
/// `String.matches()` (full match → anchored `^…$` here). Recognises an author-shaped tail
/// after a phrase's parenthesised collector reference (e.g. "…(D.Murfet 3190) R.J.Bates"),
/// which canonical rendering drops. `\p{Lu}` is Unicode by default in the Rust `regex`
/// crate, matching Java's `(?U)` flag. The `.`/`$` line-terminator set differs subtly from
/// Java's (Rust `.` excludes only `\n`; Java's also excludes `\r`/U+0085/U+2028/U+2029), but
/// the tail is `java_trim`'d and realistic author tails contain no embedded line terminators,
/// so it is not observable. The sibling `PHRASE_SPECIES_MARKER` shares this caveat.
static PHRASE_AUTHOR_TAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{Lu}](?:\.[\p{Lu}])*\..+$").unwrap());

/// The 16 boolean knobs of Java `NameFormatter.buildName`, one named field per positional
/// argument (declaration order matches Java's parameter order). See the Java Javadoc for
/// each flag's meaning; the field docs below repeat the load-bearing ones.
struct Flags {
    /// include the hybrid marker (×) with the name if existing
    hybrid_marker: bool,
    /// include the infraspecific or infrageneric rank marker with the name if existing
    rank_marker: bool,
    /// include the name's authorship (author team and year)
    authorship: bool,
    /// show the genus for infrageneric names
    genus_for_infrageneric: bool,
    /// include the infrageneric name in brackets for species or infraspecies
    infrageneric: bool,
    /// decompose unicode ligatures into their ascii ones, e.g. æ becomes ae
    decomposition: bool,
    /// transform unicode letters into their ascii ones, e.g. ø becomes o and ü u
    ascii_only: bool,
    /// include the epithet qualifiers
    show_qualifier: bool,
    /// include the rank marker for incomplete determinations, e.g. Puma spec.
    show_indet: bool,
    /// include nomenclatural notes
    nom_note: bool,
    /// include the sensu/sec taxonomic note
    show_sensu: bool,
    /// include the cultivar epithet
    show_cultivar: bool,
    /// include the phrase name
    show_phrase: bool,
    /// include the strain (a non-phrase `phrase` value)
    show_strain: bool,
    /// render the genus author of an infrageneric name and the species author of a
    /// below-species name
    show_extra_authorship: bool,
    /// add html `<i>` markup around the italicised parts
    html: bool,
}

impl Informal {
    /// The canonical string form of this informal name — e.g. `"Rhizobium sp. RMCC TR1811"`,
    /// `"Ichneumonidae sp."`, `"Bartonella group"`. Mirrors Java `NameFormatter.canonical(Informal)`:
    /// it rebuilds the equivalent `INFORMAL` [`ParsedName`] — the [`Informal::taxon`] in the genus
    /// slot, or the uninomial slot for a non-genus anchor — and renders it through
    /// [`ParsedName::canonical_name_without_authorship`], so the synthetic `sp.` marker and the
    /// phrase land exactly where the parser would place them. Never empty: falls back to the bare
    /// taxon should the rebuilt name render nothing. Named to parallel
    /// [`ParsedName::canonical_name`] so bindings expose one method across both result variants.
    pub fn canonical_name(&self) -> String {
        let pn = ParsedName {
            type_: NameType::Informal,
            rank: self.rank,
            code: self.code,
            phrase: self.phrase.clone(),
            genus: (self.taxon_rank == Rank::Genus).then(|| self.taxon.clone()),
            uninomial: (self.taxon_rank != Rank::Genus).then(|| self.taxon.clone()),
            ..ParsedName::default()
        };
        pn.canonical_name_without_authorship()
            .unwrap_or_else(|| self.taxon.clone())
    }
}

impl ParsedName {
    /// Java `NameFormatter.canonical(ParsedName)`. A full scientific name with authorship in
    /// its canonical form. Autonym authorship is placed per the nomenclatural code (after the
    /// species epithet for botany, at the very end for zoology); subspecies use the `subsp.`
    /// rank marker unless the name is zoological.
    pub fn canonical_name(&self) -> Option<String> {
        build_name(
            self,
            &Flags {
                hybrid_marker: true,
                rank_marker: true,
                authorship: true,
                genus_for_infrageneric: true,
                infrageneric: false,
                decomposition: false,
                ascii_only: false,
                show_qualifier: true,
                show_indet: true,
                nom_note: false,
                show_sensu: false,
                show_cultivar: true,
                show_phrase: true,
                show_strain: true,
                show_extra_authorship: false,
                html: false,
            },
        )
    }

    /// Java `NameFormatter.canonicalWithoutAuthorship(ParsedName)`. As [`Self::canonical_name`]
    /// but omitting any authorship.
    pub fn canonical_name_without_authorship(&self) -> Option<String> {
        build_name(
            self,
            &Flags {
                hybrid_marker: true,
                rank_marker: true,
                authorship: false,
                genus_for_infrageneric: true,
                infrageneric: false,
                decomposition: false,
                ascii_only: false,
                show_qualifier: true,
                show_indet: true,
                nom_note: false,
                show_sensu: false,
                show_cultivar: true,
                show_phrase: true,
                show_strain: true,
                show_extra_authorship: false,
                html: false,
            },
        )
    }

    /// Java `NameFormatter.canonicalMinimal(ParsedName)`. Just the three main name parts
    /// (genus, species, infraspecific) — no rank or hybrid markers, no authorship, cultivar
    /// or strain. Infrageneric names are shown without a leading genus. Unicode characters
    /// are folded to their ascii equivalents.
    pub fn canonical_name_minimal(&self) -> Option<String> {
        build_name(
            self,
            &Flags {
                hybrid_marker: false,
                rank_marker: false,
                authorship: false,
                genus_for_infrageneric: false,
                infrageneric: false,
                decomposition: true,
                ascii_only: true,
                show_qualifier: false,
                show_indet: false,
                nom_note: false,
                show_sensu: false,
                show_cultivar: false,
                show_phrase: false,
                show_strain: false,
                show_extra_authorship: false,
                html: false,
            },
        )
    }

    /// Java `NameFormatter.canonicalComplete(ParsedName)`. A full name with all details,
    /// including non-code-compliant informal remarks (subgenus in brackets, strain, sensu,
    /// nom. note, extra authorship). Unicode ligatures are decomposed.
    pub fn canonical_name_complete(&self) -> Option<String> {
        build_name(self, &complete_flags(false))
    }

    /// Java `NameFormatter.canonicalCompleteHtml(ParsedName)`. As
    /// [`Self::canonical_name_complete`] but with `<i>…</i>` markup around the italicised
    /// name parts.
    pub fn canonical_name_complete_html(&self) -> Option<String> {
        build_name(self, &complete_flags(true))
    }

    /// Java `NameFormatter.authorshipComplete(ParsedName)`. The full concatenated authorship
    /// (basionym in brackets, combination, sanctioning author, years) using the name's own
    /// nomenclatural code, or `None` when the name carries no authorship.
    pub fn authorship_complete(&self) -> Option<String> {
        let mut sb = String::new();
        append_authorship_parts(
            &mut sb,
            &self.basionym_authorship,
            &self.combination_authorship,
            self.sanctioning_author.as_deref(),
            true,
            self.code,
        );
        if sb.is_empty() {
            None
        } else {
            Some(sb)
        }
    }

    // ---- ParsedName predicates the formatter relies on (Java `ParsedName` API) ----

    /// Java `ParsedName.hasGenericAuthorship()`: the infrageneric genus author slot carries
    /// an actual authorship.
    pub fn has_generic_authorship(&self) -> bool {
        self.generic_authorship
            .as_ref()
            .is_some_and(CombinedAuthorship::has_authorship)
    }

    /// Java `ParsedName.hasSpecificAuthorship()`: the below-species species-author slot
    /// carries an actual authorship.
    pub fn has_specific_authorship(&self) -> bool {
        self.specific_authorship
            .as_ref()
            .is_some_and(CombinedAuthorship::has_authorship)
    }

    /// Java `ParsedName.hasEpithetQualifier(NamePart)`: a qualifier (cf./aff./…) is recorded
    /// for the given name part.
    pub fn has_epithet_qualifier(&self, part: NamePart) -> bool {
        self.epithet_qualifier
            .as_ref()
            .is_some_and(|m| m.contains_key(&part))
    }

    /// Java `ParsedName.isHybridName()`: at least one part is marked notho- (a named hybrid).
    pub fn is_hybrid_name(&self) -> bool {
        self.notho.as_ref().is_some_and(|s| !s.is_empty())
    }

    /// Java `ParsedName.isPhraseName()`: the `phrase` field is set and non-empty.
    pub fn is_phrase_name(&self) -> bool {
        self.phrase.as_ref().is_some_and(|p| !p.is_empty())
    }
}

/// Shared flag set for `canonicalComplete` / `canonicalCompleteHtml` — identical except for
/// the trailing `html` knob.
fn complete_flags(html: bool) -> Flags {
    Flags {
        hybrid_marker: true,
        rank_marker: true,
        authorship: true,
        genus_for_infrageneric: true,
        infrageneric: true,
        decomposition: true,
        ascii_only: false,
        show_qualifier: true,
        show_indet: true,
        nom_note: true,
        show_sensu: true,
        show_cultivar: true,
        show_phrase: true,
        show_strain: true,
        show_extra_authorship: true,
        html,
    }
}

/// Java `NameFormatter.hasNotho(ParsedName, NamePart)`.
fn has_notho(n: &ParsedName, part: NamePart) -> bool {
    n.notho.as_ref().is_some_and(|s| s.contains(&part))
}

/// Java `NameFormatter.isUnknown(Rank)` — `r == null || r.otherOrUnranked()`. Rank is never
/// null in this port (defaults to `Unranked`), so it reduces to `other_or_unranked()`.
fn is_unknown(r: Rank) -> bool {
    r.other_or_unranked()
}

/// Java `NameFormatter.isInfraspecificMarker(Rank)`.
fn is_infraspecific_marker(r: Rank) -> bool {
    r.is_infraspecific() && !r.is_uncomparable()
}

fn append_in_italics(sb: &mut String, x: &str, html: bool) {
    if html {
        sb.push_str(ITALICS_OPEN);
        sb.push_str(x);
        sb.push_str(ITALICS_CLOSE);
    } else {
        sb.push_str(x);
    }
}

/// Java `NameFormatter.appendIfNotEmpty(StringBuilder, String)` — append only when `sb` is
/// already non-empty.
fn append_if_not_empty(sb: &mut String, to_append: &str) {
    if !sb.is_empty() {
        sb.push_str(to_append);
    }
}

/// Java `NameFormatter.phraseLeadsWithSpeciesMarker(ParsedName)`.
fn phrase_leads_with_species_marker(n: &ParsedName) -> bool {
    n.is_phrase_name()
        && PHRASE_SPECIES_MARKER.is_match(java_trim(n.phrase.as_deref().unwrap_or("")))
}

/// Java `NameFormatter.appendRankMarker(sb, rank, ifRank, nothoPrefix)` (both the 3- and
/// 4-arg overloads; the 3-arg one passes `if_rank = None`). Returns true if a marker was
/// appended.
fn append_rank_marker(
    sb: &mut String,
    rank: Rank,
    if_rank: Option<fn(Rank) -> bool>,
    notho_prefix: bool,
) -> bool {
    if let Some(marker) = rank.marker() {
        if if_rank.is_none_or(|f| f(rank)) {
            if notho_prefix {
                sb.push_str(NOTHO_PREFIX);
            }
            sb.push_str(marker);
            return true;
        }
    }
    false
}

/// Java `NameFormatter.appendGenus(sb, n, hybridMarker, showQualifier, html)`.
fn append_genus(
    sb: &mut String,
    n: &ParsedName,
    hybrid_marker: bool,
    show_qualifier: bool,
    html: bool,
) {
    if show_qualifier && n.has_epithet_qualifier(NamePart::Generic) {
        sb.push_str(qualifier(n, NamePart::Generic));
        sb.push(' ');
    }
    if hybrid_marker && has_notho(n, NamePart::Generic) {
        sb.push(HYBRID_MARKER);
        sb.push(' ');
    }
    // In every call site the genus is Some (guarded by the caller), matching Java's
    // unconditional n.getGenus() dereference here.
    append_in_italics(sb, n.genus.as_deref().unwrap_or(""), html);
}

/// Java `NameFormatter.appendInfraspecific(...)`.
fn append_infraspecific(
    sb: &mut String,
    n: &ParsedName,
    hybrid_marker: bool,
    show_qualifier: bool,
    rank_marker: bool,
    force_rank_marker: bool,
    html: bool,
) {
    sb.push(' ');
    if show_qualifier && n.has_epithet_qualifier(NamePart::Infraspecific) {
        sb.push_str(qualifier(n, NamePart::Infraspecific));
        sb.push(' ');
    }
    if hybrid_marker && has_notho(n, NamePart::Infraspecific) {
        if rank_marker && is_infraspecific_marker(n.rank) {
            sb.push_str("notho");
        } else {
            sb.push(HYBRID_MARKER);
            sb.push(' ');
        }
    }
    // Render the infraspecific rank marker. Other ranks always show it; the subspecies
    // marker is hidden only for zoological names (ICZN trinomials carry no marker).
    // Kept as Java's two nested `if`s (not collapsed) because the inner condition's
    // `append_rank_marker` has a side effect (it appends the marker) — folding it into the
    // outer guard would bury that write inside a compound boolean.
    #[allow(clippy::collapsible_if)]
    if force_rank_marker
        || (rank_marker
            && (n.code != Some(NomCode::Zoological)
                || n.rank != Rank::Subspecies
                || n.is_hybrid_name()))
    {
        if append_rank_marker(sb, n.rank, Some(is_infraspecific_marker), false)
            && n.infraspecific_epithet.is_some()
        {
            sb.push(' ');
        }
    }
    if let Some(ie) = &n.infraspecific_epithet {
        append_in_italics(sb, ie, html);
    }
}

/// Fetch an epithet qualifier known to exist (caller guards with `has_epithet_qualifier`).
fn qualifier(n: &ParsedName, part: NamePart) -> &str {
    n.epithet_qualifier
        .as_ref()
        .and_then(|m| m.get(&part))
        .map(String::as_str)
        .unwrap_or("")
}

/// Java `NameFormatter.joinAuthors(List<String>, Integer maxAuthors)`.
fn join_authors(authors: &[String], max_authors: Option<usize>) -> String {
    if let Some(max) = max_authors {
        if authors.len() > max {
            return format!("{} {}", authors[0], ET_AL);
        }
    }
    if authors.len() > 1 {
        let last = &authors[authors.len() - 1];
        let end = if AL.is_match(last) {
            format!(" {ET_AL}")
        } else {
            format!(" & {last}")
        };
        return format!("{}{}", authors[..authors.len() - 1].join(", "), end);
    }
    authors.join(", ")
}

/// Java `NameFormatter.appendAuthorship(StringBuilder, Authorship, boolean, NomCode)`.
fn append_authorship(
    sb: &mut String,
    auth: &Authorship,
    include_year: bool,
    code: Option<NomCode>,
) {
    if !auth.exists() {
        return;
    }
    let mut authors_appended = false;
    let max = if code == Some(NomCode::Bacterial) {
        Some(2)
    } else {
        None
    };
    if !auth.ex_authors.is_empty() {
        sb.push_str(&join_authors(&auth.ex_authors, max));
        sb.push_str(" ex ");
        authors_appended = true;
    }
    if !auth.authors.is_empty() {
        sb.push_str(&join_authors(&auth.authors, max));
        authors_appended = true;
    }
    if include_year {
        if let Some(year) = &auth.year {
            if authors_appended {
                if code != Some(NomCode::Bacterial) {
                    sb.push(',');
                }
                sb.push(' ');
            }
            sb.push_str(year);
        }
        if let Some(iy) = &auth.imprint_year {
            sb.push_str(" [");
            sb.push_str(iy);
            sb.push(']');
        }
    }
}

/// Java `NameFormatter.appendAuthorship(StringBuilder, CombinedAuthorshipIF, boolean,
/// NomCode)` — used both for a name's own authorship (its flattened
/// combination/basionym/sanctioning fields) and for the nested
/// generic/specific `CombinedAuthorship` slots.
fn append_authorship_parts(
    sb: &mut String,
    basionym: &Authorship,
    combination: &Authorship,
    sanctioning: Option<&str>,
    include_year: bool,
    code: Option<NomCode>,
) {
    let orig_len = sb.len();
    if basionym.exists() {
        sb.push('(');
        append_authorship(sb, basionym, include_year, code);
        sb.push(')');
    }
    if combination.exists() {
        if orig_len < sb.len() {
            sb.push(' ');
        }
        append_authorship(sb, combination, include_year, code);
        if let Some(sanct) = sanctioning {
            sb.push_str(" : ");
            sb.push_str(sanct);
        }
    }
}

/// Append the name's OWN authorship — Java `appendAuthorship(sb, n, includeYear, code)`
/// where `n` (a `ParsedName`) is the `CombinedAuthorshipIF`.
fn append_name_authorship(sb: &mut String, n: &ParsedName, include_year: bool) {
    append_authorship_parts(
        sb,
        &n.basionym_authorship,
        &n.combination_authorship,
        n.sanctioning_author.as_deref(),
        include_year,
        n.code,
    );
}

/// Append a nested `CombinedAuthorship` slot (generic or specific authorship).
fn append_combined_authorship(
    sb: &mut String,
    c: &CombinedAuthorship,
    include_year: bool,
    code: Option<NomCode>,
) {
    append_authorship_parts(
        sb,
        &c.basionym_authorship,
        &c.combination_authorship,
        c.sanctioning_author.as_deref(),
        include_year,
        code,
    );
}

/// Java `UnicodeUtils.decompose(String)` — expands the fixed set of two-letter ligatures.
/// Every search key is a single unicode scalar and no replacement contains a search key, so
/// a single char-by-char pass reproduces `StringUtils.replaceEach` exactly.
fn decompose(x: &str) -> String {
    let mut out = String::with_capacity(x.len());
    for c in x.chars() {
        match c {
            'æ' => out.push_str("ae"),
            'Æ' => out.push_str("Ae"),
            'œ' => out.push_str("oe"),
            'Œ' => out.push_str("Oe"),
            'Ĳ' => out.push_str("Ij"),
            'ĳ' => out.push_str("ij"),
            'ǈ' => out.push_str("Lj"),
            'ǉ' => out.push_str("lj"),
            'ȸ' => out.push_str("db"),
            'ȹ' => out.push_str("qp"),
            'ß' => out.push_str("ss"),
            'ﬆ' => out.push_str("st"),
            'ﬅ' => out.push_str("ft"),
            'ﬀ' => out.push_str("ff"),
            'ﬁ' => out.push_str("fi"),
            'ﬂ' => out.push_str("fl"),
            'ﬃ' => out.push_str("ffi"),
            'ﬄ' => out.push_str("ffl"),
            other => out.push(other),
        }
    }
    out
}

/// Java `UnicodeUtils.replaceSpecialCases(String)` — cases the java Normalizer misses.
fn replace_special_cases(x: &str) -> String {
    let mut out = String::with_capacity(x.len());
    for c in x.chars() {
        match c {
            'ß' => out.push_str("ss"),
            'ſ' => out.push('s'),
            'Æ' => out.push_str("AE"),
            'æ' => out.push_str("ae"),
            'Ð' => out.push('D'),
            'đ' => out.push('d'),
            'ð' => out.push('d'),
            'Ø' => out.push('O'),
            'ø' => out.push('o'),
            'Œ' => out.push_str("OE"),
            'œ' => out.push_str("oe"),
            'Ŧ' => out.push('T'),
            'ŧ' => out.push('t'),
            'Ł' => out.push('L'),
            'ł' => out.push('l'),
            other => out.push(other),
        }
    }
    out
}

/// Java `UnicodeUtils.MARKER` = `Pattern.compile("\\p{M}")` — the Unicode general category
/// Mark (Mn/Mc/Me). The `regex` crate's `\p{M}` is the same category, so this reproduces
/// Java's `MARKER.matcher(x).replaceAll("")` byte-for-byte (no hand-rolled range set).
static MARK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\p{M}").unwrap());

/// Java `UnicodeUtils.foldToAscii(String)` — `replaceSpecialCases`, then Unicode NFD (which
/// splits accented letters into base + combining mark), then drop all `\p{M}` marks, leaving
/// the ascii base letters.
fn fold_to_ascii(x: &str) -> String {
    let x = replace_special_cases(x);
    let nfd: String = x.nfd().collect();
    MARK.replace_all(&nfd, "").into_owned()
}

/// Java `NameFormatter.buildName(ParsedName, 16 booleans)` — the core renderer every public
/// method delegates to. Ported statement-for-statement; see the field docs on [`Flags`] for
/// each knob.
fn build_name(n: &ParsedName, f: &Flags) -> Option<String> {
    let mut sb = String::new();

    // `html`/`authorship` are reassigned below (Candidatus turns html off; several indet
    // branches turn authorship off), so shadow them as locals like Java's mutable params.
    let mut html = f.html;
    let mut authorship = f.authorship;

    let mut candidate_italics = false;
    if n.candidatus {
        sb.push('"');
        if html {
            sb.push_str(ITALICS_OPEN);
            candidate_italics = true;
            // the entire name goes in italics, so turn off per-part html
            html = false;
        }
        sb.push_str("Candidatus ");
    }

    if let Some(uninomial) = &n.uninomial {
        // higher rank names being just a uninomial!
        if f.hybrid_marker && has_notho(n, NamePart::Generic) {
            sb.push(HYBRID_MARKER);
            sb.push(' ');
        }
        append_in_italics(&mut sb, uninomial, html);
    } else {
        // bi- or trinomials or infrageneric names
        if let Some(infrageneric_epithet) = &n.infrageneric_epithet {
            if (is_unknown(n.rank) && n.specific_epithet.is_none())
                || n.rank.is_infrageneric_strictly()
            {
                let mut show_infra_gen = true;
                // the infrageneric is the terminal rank. Always show it and wrap it with its
                // genus if requested
                if n.genus.is_some() && f.genus_for_infrageneric {
                    append_genus(&mut sb, n, f.hybrid_marker, f.show_qualifier, html);
                    // The genus author of an infrageneric name sits between the genus and the
                    // rank marker ("Cordia (Adans.) Kuntze sect. Salimori").
                    if f.show_extra_authorship && n.has_generic_authorship() {
                        sb.push(' ');
                        append_combined_authorship(
                            &mut sb,
                            n.generic_authorship.as_ref().unwrap(),
                            true,
                            n.code,
                        );
                    }
                    sb.push(' ');
                    // we show zoological infragenerics in brackets, but use rank markers for
                    // botanical names (unless its no defined rank)
                    if n.code == Some(NomCode::Zoological) {
                        sb.push('(');
                        if f.hybrid_marker && has_notho(n, NamePart::Infrageneric) {
                            sb.push(HYBRID_MARKER);
                            sb.push(' ');
                        }
                        append_in_italics(&mut sb, infrageneric_epithet, html);
                        sb.push(')');
                        show_infra_gen = false;
                    }
                }
                if show_infra_gen {
                    if f.rank_marker {
                        // If we know the rank we use explicit rank markers — this is how
                        // botanical infrageneric names are formed, see
                        // http://www.iapt-taxon.org/nomen/main.php?page=art21
                        if append_rank_marker(
                            &mut sb,
                            n.rank,
                            None,
                            f.hybrid_marker && has_notho(n, NamePart::Infrageneric),
                        ) {
                            sb.push(' ');
                        }
                    }
                    append_in_italics(&mut sb, infrageneric_epithet, html);
                }
            } else {
                if n.genus.is_some() {
                    append_genus(&mut sb, n, f.hybrid_marker, f.show_qualifier, html);
                }
                if f.infrageneric {
                    // additional subgenus shown for binomial. Always shown in brackets
                    sb.push_str(" (");
                    append_in_italics(&mut sb, infrageneric_epithet, html);
                    sb.push(')');
                }
            }
        } else if n.genus.is_some() {
            append_genus(&mut sb, n, f.hybrid_marker, f.show_qualifier, html);
        }

        if n.specific_epithet.is_none() {
            if (f.show_indet && n.genus.is_some() && n.cultivar_epithet.is_none())
                || (f.show_phrase && n.is_phrase_name())
            {
                if n.rank.is_species_or_below() {
                    // no species epithet given, indetermined!
                    if n.rank.is_infraspecific() {
                        // maybe we have an infraspecific epithet? force to show the rank marker
                        append_infraspecific(
                            &mut sb,
                            n,
                            f.hybrid_marker,
                            f.show_qualifier,
                            f.rank_marker,
                            true,
                            html,
                        );
                    } else if !phrase_leads_with_species_marker(n) {
                        // Skip the synthetic "sp." when an informal phrase already spells out
                        // the species marker verbatim ("Allium species 1") — the phrase
                        // carries it.
                        sb.push(' ');
                        // Java `sb.append(n.getRank().getMarker())`. Here the rank is
                        // species-or-below-but-not-infraspecific (SPECIES / SPECIES_AGGREGATE),
                        // both of which always have a marker, so the default is never taken; unlike
                        // line 757 there is no null-marker rank to mirror Java's "null" for.
                        sb.push_str(n.rank.marker().unwrap_or(""));
                    }
                    authorship = false;
                }
            } else if n.infraspecific_epithet.is_some() {
                append_infraspecific(
                    &mut sb,
                    n,
                    f.hybrid_marker,
                    f.show_qualifier,
                    f.rank_marker,
                    false,
                    html,
                );
            }
        } else {
            // species part
            sb.push(' ');
            if f.show_qualifier && n.has_epithet_qualifier(NamePart::Specific) {
                sb.push_str(qualifier(n, NamePart::Specific));
                sb.push(' ');
            }
            if f.hybrid_marker && has_notho(n, NamePart::Specific) {
                sb.push(HYBRID_MARKER);
                sb.push(' ');
            }
            append_in_italics(&mut sb, n.specific_epithet.as_deref().unwrap_or(""), html);
            // The species author of a below-species name (cultivar / trinomial) sits right
            // after the species epithet ("Acer campestre L. 'Elsrijk' Broerse").
            if f.show_extra_authorship && n.has_specific_authorship() {
                sb.push(' ');
                append_combined_authorship(
                    &mut sb,
                    n.specific_authorship.as_ref().unwrap(),
                    true,
                    n.code,
                );
            }

            if n.infraspecific_epithet.is_none() {
                // Indetermined infraspecies? Only show indet cultivar marker if no cultivar
                // epithet exists
                if f.show_indet
                    && n.rank.is_infraspecific()
                    && (n.rank.is_restricted_to_code() != Some(NomCode::Cultivars)
                        || n.cultivar_epithet.is_none())
                {
                    // no infraspecific epitheton given, but rank below species. Indetermined!
                    // use ssp. for subspecies in case of indetermined names
                    if n.rank == Rank::Subspecies {
                        sb.push_str(" ssp.");
                    } else {
                        sb.push(' ');
                        // Faithful to Java `sb.append(n.getRank().getMarker())`: when the marker
                        // is null Java's `StringBuilder.append((String) null)` appends the literal
                        // "null". The only infraspecific rank with a null marker is CULTIVAR_GROUP
                        // (which reaches here only for a specific-but-no-infraspecific, no-cultivar
                        // name — not producible by `parse()`, but the formatter is public API over
                        // any hand-built ParsedName), so mirror the "null" rather than dropping it.
                        sb.push_str(n.rank.marker().unwrap_or("null"));
                    }
                    authorship = false;
                }
            } else {
                // Autonym authorship placement follows the codes. The autonym's final epithet
                // never carries an author of its own; the author shown is the species author.
                //   ICN Art. 22.1/26.1 (botany): cite it right after the species epithet.
                //   ICZN (zoology): cite it at the very end of the trinomen.
                // Only an explicit botanical code triggers the after-species placement;
                // zoological and unknown-code autonyms keep the author in its default end
                // position below.
                if n.is_autonym() && n.code == Some(NomCode::Botanical) {
                    if authorship && n.has_authorship() {
                        sb.push(' ');
                        append_name_authorship(&mut sb, n, true);
                    }
                    authorship = false;
                }
                // infraspecific part
                append_infraspecific(
                    &mut sb,
                    n,
                    f.hybrid_marker,
                    f.show_qualifier,
                    f.rank_marker,
                    false,
                    html,
                );
            }
        }
    }

    // closing quotes for Candidatus names
    if n.candidatus {
        if candidate_italics {
            sb.push_str(ITALICS_CLOSE);
        }
        sb.push('"');
    }

    // uninomial, genus, infragen, species or infraspecies authorship. For a cultivar the
    // name's authorship IS the cultivar author, rendered AFTER the cultivar epithet below,
    // so it is suppressed here.
    let cultivar_shown = f.show_cultivar && n.cultivar_epithet.is_some();
    if authorship && n.has_authorship() && !cultivar_shown {
        sb.push(' ');
        append_name_authorship(&mut sb, n, true);
    }

    // add strain name (phrase names get special treatment)
    if f.show_strain && n.phrase.is_some() && !n.is_phrase_name() {
        sb.push(' ');
        sb.push_str(n.phrase.as_deref().unwrap_or(""));
    }

    // add cultivar name
    if f.show_cultivar {
        if let Some(cultivar) = &n.cultivar_epithet {
            if n.rank == Rank::CultivarGroup {
                sb.push(' ');
                sb.push_str(cultivar);
                sb.push_str(" Group");
            } else if n.rank == Rank::Grex {
                sb.push(' ');
                sb.push_str(cultivar);
                sb.push_str(" gx");
            } else {
                sb.push_str(" '");
                sb.push_str(cultivar);
                sb.push('\'');
            }
            // The cultivar author follows the cultivar epithet ("Acer campestre 'Elsrijk'
            // Broerse").
            if authorship && n.has_authorship() {
                sb.push(' ');
                append_name_authorship(&mut sb, n, true);
            }
        }
    }

    // Add phrase name. Phrase values may include a trailing author span after the collector
    // parenthesised reference ("Sandheath (D.Murfet 3190) R.J.Bates"); for canonical
    // rendering we drop that author-shaped tail so the output stays clean while the stored
    // phrase keeps the full annotation. A non-author suffix (e.g. "NT Herbarium") is kept.
    if f.show_phrase && n.is_phrase_name() {
        let phrase_full = n.phrase.as_deref().unwrap_or("");
        let mut phrase = phrase_full;
        if let Some(last_close) = phrase_full.rfind(')') {
            // ')' is ASCII (1 byte), so the byte comparison matches Java's char check for
            // "is there content after the last ')'".
            if last_close < phrase_full.len() - 1 {
                let tail = java_trim(&phrase_full[last_close + 1..]);
                // Author-shaped tail: initials with dots (e.g. "R.J.Bates").
                if PHRASE_AUTHOR_TAIL.is_match(tail) {
                    phrase = &phrase_full[..last_close + 1];
                }
            }
        }
        append_if_not_empty(&mut sb, " ");
        sb.push_str(phrase);
    }

    // add sensu/sec reference
    if f.show_sensu {
        if let Some(tn) = &n.taxonomic_note {
            append_if_not_empty(&mut sb, " ");
            sb.push_str(tn);
        }
    }

    // add nom status
    if f.nom_note {
        if let Some(nn) = &n.nomenclatural_note {
            append_if_not_empty(&mut sb, ", ");
            sb.push_str(nn);
        }
    }

    // final char transformations
    let mut name = java_trim(&sb).to_string();
    if f.decomposition {
        name = decompose(&name);
    }
    if f.ascii_only {
        name = fold_to_ascii(&name);
    }
    // Java StringUtils.trimToNull: trim again, null if empty.
    let name = java_trim(&name);
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    //! Always-on formatter regression guards. Every `expect_*` value here is
    //! **Java-authoritative** — produced by running the input through the real Java
    //! `NameFormatter` (via the `FormatOracle`, same tool `tests/format_golden.rs` uses), not
    //! hand-derived — so these pin the exact Java behaviour for the structural rendering
    //! paths (autonym placement, infrageneric markers, notho/hybrid spacing, Candidatus
    //! quoting, sanctioning author, cultivar, ascii folding, html) without needing the oracle
    //! present at test time.

    use crate::parse_name as parse;

    /// Parse with the default (no hints) entry point; panics if the name is unparsable — every
    /// input below is a parseable name chosen precisely to exercise a formatter path.
    fn p(input: &str) -> crate::model::ParsedName {
        parse(input, None, None, None).unwrap_or_else(|_| panic!("should parse: {input:?}"))
    }

    #[test]
    fn informal_canonical_rebuilds_the_parser_rendering() {
        use crate::model::enums::Rank;
        use crate::model::Informal;
        let inf = |taxon: &str, taxon_rank, rank, phrase: Option<&str>| Informal {
            taxon: taxon.into(),
            taxon_rank,
            rank,
            phrase: phrase.map(str::to_string),
            code: None,
        };
        // genus-anchored provisional species with a captured phrase tag
        assert_eq!(
            inf("Rhizobium", Rank::Genus, Rank::Species, Some("RMCC TR1811")).canonical_name(),
            "Rhizobium sp. RMCC TR1811"
        );
        // numbered placeholder
        assert_eq!(
            inf("Allium", Rank::Genus, Rank::Species, Some("1")).canonical_name(),
            "Allium sp. 1"
        );
        // bare "Genus sp." — no phrase (a higher-taxon anchor still sits in the genus slot)
        assert_eq!(
            inf("Ichneumonidae", Rank::Genus, Rank::Species, None).canonical_name(),
            "Ichneumonidae sp."
        );
        // informal group — UNRANKED, the phrase carries the designation (no synthetic sp.)
        assert_eq!(
            inf("Bartonella", Rank::Genus, Rank::Unranked, Some("group")).canonical_name(),
            "Bartonella group"
        );
    }

    #[test]
    fn basic_binomial_with_author() {
        let n = p("Abies alba Mill.");
        assert_eq!(n.canonical_name().as_deref(), Some("Abies alba Mill."));
        assert_eq!(
            n.canonical_name_without_authorship().as_deref(),
            Some("Abies alba")
        );
        assert_eq!(n.canonical_name_minimal().as_deref(), Some("Abies alba"));
        assert_eq!(
            n.canonical_name_complete().as_deref(),
            Some("Abies alba Mill.")
        );
        assert_eq!(n.authorship_complete().as_deref(), Some("Mill."));
    }

    #[test]
    fn authorless_name_has_no_authorship_string() {
        let n = p("Abies alba");
        assert_eq!(n.canonical_name().as_deref(), Some("Abies alba"));
        assert_eq!(n.authorship_complete(), None);
    }

    #[test]
    fn zoological_subspecies_shows_no_rank_marker() {
        // ICZN trinomials carry no infraspecific marker.
        let n = p("Vulpes vulpes silaceus Miller, 1907");
        assert_eq!(
            n.canonical_name().as_deref(),
            Some("Vulpes vulpes silaceus Miller, 1907")
        );
        assert_eq!(
            n.canonical_name_without_authorship().as_deref(),
            Some("Vulpes vulpes silaceus")
        );
        assert_eq!(n.authorship_complete().as_deref(), Some("Miller, 1907"));
    }

    #[test]
    fn botanical_autonym_keeps_marker_but_minimal_drops_it() {
        let n = p("Acer rubrum var. rubrum");
        assert_eq!(
            n.canonical_name().as_deref(),
            Some("Acer rubrum var. rubrum")
        );
        // canonicalMinimal drops the rank marker: three bare epithets.
        assert_eq!(
            n.canonical_name_minimal().as_deref(),
            Some("Acer rubrum rubrum")
        );
    }

    #[test]
    fn botanical_autonym_places_author_after_species_epithet() {
        // ICN Art. 22.1/26.1: the species author sits before the (repeated) rank marker.
        let n = p("Trimezia spathata (Klatt) Baker subsp. spathata");
        assert_eq!(
            n.canonical_name().as_deref(),
            Some("Trimezia spathata (Klatt) Baker subsp. spathata")
        );
        assert_eq!(
            n.canonical_name_without_authorship().as_deref(),
            Some("Trimezia spathata subsp. spathata")
        );
        assert_eq!(n.authorship_complete().as_deref(), Some("(Klatt) Baker"));
    }

    #[test]
    fn infrageneric_botanical_uses_rank_marker_and_minimal_drops_genus() {
        // "subg." normalises to the "subgen." marker; minimal shows only the terminal epithet.
        let n = p("Astragalus subg. Cercidothrix");
        assert_eq!(
            n.canonical_name().as_deref(),
            Some("Astragalus subgen. Cercidothrix")
        );
        assert_eq!(n.canonical_name_minimal().as_deref(), Some("Cercidothrix"));
    }

    #[test]
    fn basionym_and_combination_authorship() {
        let n = p("Baeomyces rufus (Huds.) Rebent.");
        assert_eq!(
            n.canonical_name().as_deref(),
            Some("Baeomyces rufus (Huds.) Rebent.")
        );
        assert_eq!(
            n.canonical_name_without_authorship().as_deref(),
            Some("Baeomyces rufus")
        );
        assert_eq!(n.authorship_complete().as_deref(), Some("(Huds.) Rebent."));
    }

    #[test]
    fn basionym_parens_with_year() {
        let n = p("Oenanthe oenanthe (Linnaeus, 1758)");
        assert_eq!(
            n.canonical_name().as_deref(),
            Some("Oenanthe oenanthe (Linnaeus, 1758)")
        );
        assert_eq!(n.authorship_complete().as_deref(), Some("(Linnaeus, 1758)"));
    }

    #[test]
    fn sanctioning_author_rendered_with_colon() {
        // http://www.iapt-taxon.org/nomen/main.php?page=r50E — sanctioning author after " : ".
        let n = p("Agaricus campestris L. : Fr.");
        assert_eq!(
            n.canonical_name().as_deref(),
            Some("Agaricus campestris L. : Fr.")
        );
        assert_eq!(n.authorship_complete().as_deref(), Some("L. : Fr."));
    }

    #[test]
    fn notho_genus_hybrid_marker_with_space() {
        let n = p("×Agropogon littoralis");
        assert_eq!(
            n.canonical_name().as_deref(),
            Some("× Agropogon littoralis")
        );
        // minimal drops the hybrid marker.
        assert_eq!(
            n.canonical_name_minimal().as_deref(),
            Some("Agropogon littoralis")
        );
    }

    #[test]
    fn notho_species_hybrid_marker() {
        let n = p("Salix ×capreola Andersson");
        assert_eq!(
            n.canonical_name().as_deref(),
            Some("Salix × capreola Andersson")
        );
        assert_eq!(
            n.canonical_name_minimal().as_deref(),
            Some("Salix capreola")
        );
    }

    #[test]
    fn cultivar_epithet_in_single_quotes_and_minimal_drops_it() {
        let n = p("Acer campestre 'Elsrijk'");
        assert_eq!(
            n.canonical_name().as_deref(),
            Some("Acer campestre 'Elsrijk'")
        );
        assert_eq!(
            n.canonical_name_minimal().as_deref(),
            Some("Acer campestre")
        );
    }

    #[test]
    fn candidatus_wraps_in_quotes_with_bacterial_comma_less_year() {
        // Candidatus implies the bacterial code: the year carries no comma, and the whole
        // name is wrapped in double quotes.
        let n = p("Candidatus Nanopelagicus abundans Ghai, 2013");
        assert_eq!(
            n.canonical_name().as_deref(),
            Some("\"Candidatus Nanopelagicus abundans\" Ghai 2013")
        );
        assert_eq!(n.authorship_complete().as_deref(), Some("Ghai 2013"));
    }

    #[test]
    fn minimal_folds_ligatures_and_diacritics_to_ascii() {
        // decompose expands the "æ" ligature; foldToAscii's NFD strips the "ü" umlaut.
        assert_eq!(
            p("Læptura").canonical_name_minimal().as_deref(),
            Some("Laeptura")
        );
        assert_eq!(
            p("Rübsaamenia excelsa").canonical_name_minimal().as_deref(),
            Some("Rubsaamenia excelsa")
        );
        // ...but the non-minimal renderings keep the original unicode.
        assert_eq!(p("Læptura").canonical_name().as_deref(), Some("Læptura"));
    }

    #[test]
    fn cultivar_group_indet_mirrors_javas_null_marker_append() {
        // Latent faithfulness case flagged by review, unreachable via parse(): a CULTIVAR_GROUP
        // name with a specific but no infraspecific/cultivar epithet. CULTIVAR_GROUP is
        // infraspecific yet has a null rank marker, so Java's
        // `sb.append(n.getRank().getMarker())` appends the literal "null"
        // (`StringBuilder.append((String) null)`). Java-verified: `canonical(pn)` == "Aaa bbb
        // null". The formatter is public API over any hand-built ParsedName, so we mirror it.
        use crate::model::{ParsedName, Rank};
        let pn = ParsedName {
            rank: Rank::CultivarGroup,
            genus: Some("Aaa".to_string()),
            specific_epithet: Some("bbb".to_string()),
            ..Default::default()
        };
        assert_eq!(pn.canonical_name().as_deref(), Some("Aaa bbb null"));
        assert_eq!(
            pn.canonical_name_complete().as_deref(),
            Some("Aaa bbb null")
        );
    }

    #[test]
    fn complete_html_italicises_name_parts() {
        assert_eq!(
            p("Abies alba Mill.")
                .canonical_name_complete_html()
                .as_deref(),
            Some("<i>Abies</i> <i>alba</i> Mill.")
        );
        // notho marker sits (un-italicised) between the italic parts.
        assert_eq!(
            p("Salix ×capreola Andersson")
                .canonical_name_complete_html()
                .as_deref(),
            Some("<i>Salix</i> × <i>capreola</i> Andersson")
        );
        assert_eq!(
            p("Astragalus subg. Cercidothrix")
                .canonical_name_complete_html()
                .as_deref(),
            Some("<i>Astragalus</i> subgen. <i>Cercidothrix</i>")
        );
    }
}
