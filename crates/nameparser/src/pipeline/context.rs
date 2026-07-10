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
///
/// Several fields (`tokens`, `pending_unparsed`, `aggregate`, `pending_year` and friends —
/// see each field's own doc comment) are written by `new`/Preflight for pipeline stages
/// (StripAndStash, NameTokens, AuthorshipParser, Assemble, …) that aren't ported yet, so
/// nothing reads them within the crate for now. While this struct was `pub`, rustc
/// exempted them from `dead_code` as public API surface; now that it's `pub(crate)`
/// (Phase 1 foundation review — closes a `MAX_LENGTH`-guard bypass), that exemption no
/// longer applies. Allowed at the struct level rather than per-field since the reason is
/// uniform; drop once every field has a real reader.
#[allow(dead_code)]
pub(crate) struct ParseContext {
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
    pub(crate) fn new(
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

    /// Java `ParseContext.setPendingImprintYear(String)`: records a stripped imprint year
    /// on a first-writer-wins basis (Java: `if (pendingImprintYear == null && year != null)
    /// pendingImprintYear = year;` — the `year != null` half is unreachable here since Rust
    /// callers pass a non-nullable `&str`). Used by `StripAndStash::strip_imprint_years`.
    pub(crate) fn set_pending_imprint_year(&mut self, year: &str) {
        if self.pending_imprint_year.is_none() {
            self.pending_imprint_year = Some(year.to_string());
        }
    }

    /// Java `ParseContext.setPendingUnparsed(String)`: records an unparsed remainder on a
    /// first-writer-wins basis; a blank (empty or all-whitespace) remainder is ignored.
    /// `pendingUnparsed` is a single slot written by several `StripAndStash` steps — routing
    /// every write through here makes precedence deterministic (the earliest strip step in
    /// `StripAndStash::run` wins) instead of depending on each writer to null-check itself.
    /// Java's `String.isBlank()` treats a string as blank when every char is
    /// `Character.isWhitespace` (or the string is empty) — the same predicate as
    /// `token::is_whitespace_java`, reused here rather than `java_trim`'s narrower
    /// (`<= U+0020`-only) notion of trimmable whitespace.
    pub(crate) fn set_pending_unparsed(&mut self, remainder: &str) {
        if self.pending_unparsed.is_none()
            && !remainder.chars().all(crate::token::is_whitespace_java)
        {
            self.pending_unparsed = Some(remainder.to_string());
        }
    }

    /// Java `ParseContext.setPendingPublicationYear(String)`: records a publication-derived
    /// year on a first-writer-wins basis and marks it code-neutral (see
    /// `pending_year_from_publication`'s own doc comment). Every current writer of
    /// `pending_year` (`StripAndStash::strip_in_author_citation`, `strip_ipni_citation`,
    /// `strip_period_separated_reference`) sets it from a stripped `publishedIn` reference, so
    /// the two fields are always set together — bundling them here keeps them in lock-step,
    /// matching Java's `if (pendingYear == null && year != null) { pendingYear = year;
    /// pendingYearFromPublication = true; }` (the `year != null` half is unreachable here
    /// since Rust callers pass a non-nullable `&str`, same as `set_pending_imprint_year`).
    pub(crate) fn set_pending_publication_year(&mut self, year: &str) {
        if self.pending_year.is_none() {
            self.pending_year = Some(year.to_string());
            self.pending_year_from_publication = true;
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

    #[test]
    fn set_pending_imprint_year_is_first_writer_wins() {
        let mut ctx = ParseContext::new("Abies alba".to_string(), None, None, None);
        ctx.set_pending_imprint_year("1969");
        assert_eq!(ctx.pending_imprint_year, Some("1969".to_string()));
        ctx.set_pending_imprint_year("1875");
        assert_eq!(
            ctx.pending_imprint_year,
            Some("1969".to_string()),
            "the first-written year must win, matching Java's null-guarded setter"
        );
    }

    #[test]
    fn set_pending_unparsed_is_first_writer_wins_and_ignores_blank() {
        let mut ctx = ParseContext::new("Abies alba".to_string(), None, None, None);
        ctx.set_pending_unparsed("   ");
        assert_eq!(
            ctx.pending_unparsed, None,
            "a blank remainder must be ignored, matching Java's isBlank() guard"
        );
        ctx.set_pending_unparsed("XXZ_21243");
        assert_eq!(ctx.pending_unparsed, Some("XXZ_21243".to_string()));
        ctx.set_pending_unparsed("(sic,foo)");
        assert_eq!(
            ctx.pending_unparsed,
            Some("XXZ_21243".to_string()),
            "the first non-blank write must win"
        );
    }

    #[test]
    fn set_pending_publication_year_is_first_writer_wins_and_marks_code_neutral() {
        let mut ctx = ParseContext::new("Abies alba".to_string(), None, None, None);
        assert_eq!(ctx.pending_year, None);
        assert!(!ctx.pending_year_from_publication);
        ctx.set_pending_publication_year("1988");
        assert_eq!(ctx.pending_year, Some("1988".to_string()));
        assert!(
            ctx.pending_year_from_publication,
            "a publication-derived year must be marked code-neutral"
        );
        ctx.set_pending_publication_year("1900");
        assert_eq!(
            ctx.pending_year,
            Some("1988".to_string()),
            "the first-written year must win, matching Java's null-guarded setter"
        );
    }
}
