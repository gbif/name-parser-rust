// SPDX-License-Identifier: Apache-2.0
//! Java `org.gbif.nameparser.pipeline.ParseContext` — mutable per-parse state shared
//! across pipeline stages.

use crate::model::{NomCode, ParsedName, Rank};
use crate::token::Token;

/// Mutable per-parse state shared across pipeline stages. Faithful port of Java
/// `ParseContext`.
///
/// Java declares the first four fields `final` (set once by the constructor and never
/// reassigned); Rust has no per-field `final` short of splitting the type into two
/// structs, so they are plain `pub` fields here too — callers should treat them as
/// read-only after construction, matching the Java contract.
pub struct ParseContext {
    // ---- Java `final` (conceptually immutable after construction) ----
    pub original: String,
    pub authorship_input: Option<String>,
    pub requested_rank: Option<Rank>,
    pub requested_code: Option<NomCode>,

    // ---- mutable, written by later pipeline stages ----
    pub working: String,
    pub tokens: Vec<Token>,
    pub name: ParsedName,
    /// Set by StripAndStash when a parenthesised "[sic, …]" comment was removed.
    pub pending_unparsed: Option<String>,
    /// Set by StripAndStash when an aggregate marker was stripped from the input.
    pub aggregate: bool,
    /// Set by Preflight when the input is a clean uni/binomial whose genus (or
    /// monomial) carries an ICTV viral rank suffix. Assemble turns this into
    /// `NomCode::Virus` when the caller supplied no code.
    pub viral_shape: bool,
    /// Year extracted from a stripped publishedIn tail (e.g. "in Author, 1987").
    pub pending_year: Option<String>,
    /// True when `pending_year` came from a stripped publishedIn reference rather than
    /// from the author span itself. Such a year is the publication year of the work —
    /// code-neutral — and is propagated onto the combination authorship for output but
    /// must NOT be used as a signal for code inference. (A name's nomenclatural code is
    /// settled by other authorship cues; a publication year can attach to zoological,
    /// botanical, or bacteriological names alike.)
    pub pending_year_from_publication: bool,
    /// Quote char ("'" or "\"") a leading monomial was wrapped in (e.g. "'Prosthète'
    /// Hesse, 1861"). Such quotes mark a name that is not an available scientific name;
    /// the quotes are stripped for parsing and re-wrapped around the parsed uninomial in
    /// Assemble so the output keeps them, and the name is flagged doubtful.
    pub quoted_monomial: Option<String>,
    /// Token index range [`mid_author_from`, `mid_author_to`) of an author span that
    /// sits between the species epithet and a following infraspecific rank marker
    /// ("Cirsium creticum d'Urv. subsp. creticum", "Trimezia spathata (Klatt) Baker
    /// subsp. spathata"). Recorded by NameTokens. For an autonym this is the *species*
    /// author (ICN Art. 22.1/26.1) — the autonym's final epithet bears no author of its
    /// own — so Pipeline parses this span and applies it as the name's authorship. For
    /// non-autonym infraspecific names the model holds the terminal (infraspecific)
    /// author instead, so this span is left dropped.
    pub mid_author_from: i32,
    pub mid_author_to: i32,
    /// Imprint year stripped from the input before authorship parsing (e.g. the "1969"
    /// in `Storr, 1970 ["1969"]` / `(imprint 1969)`). Applied onto the name's combination
    /// authorship once it exists — imprint years live on `Authorship`.
    pub pending_imprint_year: Option<String>,
    /// Species author extracted from a below-species name where it sits before the
    /// terminal epithet (e.g. the "L." in "Acer campestre L. cv. 'Elsrijk' Broerse").
    /// Parsed and set as `ParsedName::specific_authorship` by Pipeline.
    pub pending_specific_author: Option<String>,
    /// Genus author of an infrageneric name where it sits before the rank marker (e.g.
    /// the "(Adans.) Kuntze" in "Cordia (Adans.) Kuntze sect. Salimori"). Parsed and set
    /// as `ParsedName::generic_authorship` by Pipeline.
    pub pending_generic_author: Option<String>,
}

impl ParseContext {
    /// Java `ParseContext(String scientificName, String authorship, Rank rank, NomCode
    /// code)`. Seeds `name.rank`/`name.code` from the args on top of
    /// `ParsedName::default()`'s `type = SCIENTIFIC`, `state = COMPLETE`: `rank` folds a
    /// `None` to `Rank::Unranked` (matching Java `ParsedName.setRank`'s null→UNRANKED
    /// fold), while `code` passes straight through (matching `setCode`, which has no such
    /// fold — `None`/`null` stays unset).
    pub fn new(
        original: String,
        authorship_input: Option<String>,
        requested_rank: Option<Rank>,
        requested_code: Option<NomCode>,
    ) -> Self {
        let name = ParsedName {
            rank: requested_rank.unwrap_or(Rank::Unranked),
            code: requested_code,
            ..ParsedName::default()
        };
        ParseContext {
            working: original.clone(),
            original,
            authorship_input,
            requested_rank,
            requested_code,
            tokens: Vec::new(),
            name,
            pending_unparsed: None,
            aggregate: false,
            viral_shape: false,
            pending_year: None,
            pending_year_from_publication: false,
            quoted_monomial: None,
            mid_author_from: -1,
            mid_author_to: -1,
            pending_imprint_year: None,
            pending_specific_author: None,
            pending_generic_author: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NameType, State};

    #[test]
    fn constructor_seeds_name_rank_code_type_and_state() {
        let ctx = ParseContext::new(
            "Abies alba".to_string(),
            None,
            Some(Rank::Species),
            Some(NomCode::Botanical),
        );
        assert_eq!(ctx.name.rank, Rank::Species);
        assert_eq!(ctx.name.code, Some(NomCode::Botanical));
        assert_eq!(ctx.name.type_, NameType::Scientific);
        assert_eq!(ctx.name.state, State::Complete);
        assert_eq!(ctx.working, "Abies alba");
        assert_eq!(ctx.original, "Abies alba");
    }

    #[test]
    fn constructor_folds_missing_rank_to_unranked_but_leaves_code_unset() {
        let ctx = ParseContext::new("Abies".to_string(), None, None, None);
        assert_eq!(ctx.name.rank, Rank::Unranked);
        assert_eq!(ctx.name.code, None);
    }

    #[test]
    fn mid_author_range_defaults_to_minus_one() {
        let ctx = ParseContext::new("Abies".to_string(), None, None, None);
        assert_eq!(ctx.mid_author_from, -1);
        assert_eq!(ctx.mid_author_to, -1);
    }

    #[test]
    fn authorship_input_is_threaded_through() {
        let ctx = ParseContext::new(
            "Abies alba".to_string(),
            Some("Mill.".to_string()),
            None,
            None,
        );
        assert_eq!(ctx.authorship_input, Some("Mill.".to_string()));
    }
}
