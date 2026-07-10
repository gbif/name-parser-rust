// SPDX-License-Identifier: Apache-2.0

//! Java `org.gbif.nameparser.pipeline.CodeInference` (161 lines) — infers a name's
//! nomenclatural [`NomCode`] from the shape of its authorship.
//!
//! Inference is a **vote tally**, not a priority cascade: each independent signal casts a
//! vote for a code, and a code is assigned only when every vote agrees. Contradicting
//! signals — or no signal at all — leave the code unset rather than guessing.
//! [`crate::pipeline::assemble::finish`] calls [`infer`] once, only when the name has no code
//! yet, and [`apply_rank_code_mismatch`] unconditionally afterwards.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use crate::model::{warnings, NomCode};
use crate::pipeline::authorship_parser::AuthState;
use crate::pipeline::ParseContext;

/// Java `CodeInference.ICN_STATUS` (`CodeInference.java:141-142`), compiled with no flags —
/// Java's `\b` is therefore ASCII-only, ported here as `(?-u:\b…)` (this port's per-pattern
/// flag rule; see `pipeline::preflight`'s module doc for the rule spelled out in full).
/// Botanical-only (ICN) nomenclatural statuses: nom. illeg. / inval. / superfl. (super.) /
/// cons. / rej. (rejic.) / ambig. / confus.
static ICN_STATUS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?-u:\b(?:illeg|inval|superfl|super|cons|rej|rejic|ambig|confus))").unwrap()
});

/// Java `CodeInference.ICZN_STATUS` (`CodeInference.java:143-144`), same ASCII-`\b` rationale
/// as [`ICN_STATUS`]. Zoological-only (ICZN) nomenclatural statuses: nomen oblitum / nomen
/// protectum.
static ICZN_STATUS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\b(?:oblitum|protectum)\b)").unwrap());

/// Java `CodeInference.infer(ParseContext, AuthorshipParser.AuthState)`
/// (`CodeInference.java:47-111`). Tallies the authorship signals onto a name whose code is
/// not yet set. Called by [`crate::pipeline::assemble::finish`] only when
/// `ctx.name.code.is_none()`.
///
/// Two decisive signals are checked first and pin the code outright:
///   1. a code-exclusive rank ([`crate::model::Rank::code`] != `None`) — the cultivar, viral
///      and bacterial `*-var` ranks, forma specialis, botanical section/series, etc. The
///      generic markers subsp./var./f. carry *no* code (they are used across codes, mostly
///      on old zoological synonyms) and so are deliberately not a signal.
///   2. a code-exclusive nomenclatural status ([`code_from_nom_note`]) — several ICN and
///      ICZN statuses exist only under their own code.
///
/// Otherwise a **vote tally** over authorship shape decides:
///   - BOTANICAL — a sanctioning author; a `(Basionym) Recombination` two-author citation; a
///     filius suffix with no year.
///   - ZOOLOGICAL — a basionym-only `(Author, year)` citation; a year on an authored
///     basionym or combination.
///   - BACTERIAL — a `Candidatus` name.
///
/// A single distinct vote wins; zero or contradicting votes leave the code `None`.
pub(crate) fn infer(ctx: &mut ParseContext, auth_state: Option<&AuthState>) {
    // 1. A code-exclusive rank pins the code outright — no vote needed.
    if let Some(rank_code) = ctx.name.rank.code() {
        ctx.name.code = Some(rank_code);
        return;
    }

    // 2. A code-exclusive nomenclatural status pins the code. An ICN-only or ICZN-only
    // status is decisive even against a contradicting authorship year (e.g. the year on
    // "Polygala vulgaris L., 1753, nom. cons." doesn't make it zoological).
    if let Some(note_code) = code_from_nom_note(ctx.name.nomenclatural_note.as_deref()) {
        ctx.name.code = Some(note_code);
        return;
    }

    let mut votes: HashSet<NomCode> = HashSet::new();

    // Bacterial: a Candidatus name is a provisional prokaryote name.
    if ctx.name.candidatus {
        votes.insert(NomCode::Bacterial);
    }

    if let Some(auth_state) = auth_state {
        let bas_year = auth_state.basionym_present
            && auth_state.basionym.year.is_some()
            && auth_state.basionym.has_authors();
        let comb_year =
            auth_state.combination.year.is_some() && auth_state.combination.has_authors();
        let any_author_year = bas_year || comb_year;

        // --- botanical votes ---
        // Sanctioning author (": Fr." / ": Pers.").
        if auth_state.sanctioning_author.is_some() || ctx.name.sanctioning_author.is_some() {
            votes.insert(NomCode::Botanical);
        }
        // "(Basionym) Recombination" — a parenthesised basionym plus a recombination author.
        if auth_state.basionym_present && auth_state.combination.has_authors() {
            votes.insert(NomCode::Botanical);
        }
        // Filius ("f." / "fil.") without any year.
        if auth_state.has_filius && !any_author_year {
            votes.insert(NomCode::Botanical);
        }

        // --- zoological votes ---
        // Basionym-only parenthesised recombination with no recombination author, "(Author)"
        // or "(Author, year)" — the year is optional. Fires on a species recombination
        // ("Abies alba (Smith)") and on a genus basionym with the year inside the parens
        // ("Heptacyclus (Vasileyev, 1939)"). A trailing "(Subgenus) Author, year" is split
        // into a subgenus + combination author by AuthorshipSplit, so its parens are not a
        // basionym here.
        if auth_state.basionym_present && !auth_state.combination.has_authors() {
            votes.insert(NomCode::Zoological);
        }
        // A year on an authored basionym or combination.
        if any_author_year {
            votes.insert(NomCode::Zoological);
        }
    }

    if votes.len() == 1 {
        ctx.name.code = votes.into_iter().next();
    }
}

/// Java `CodeInference.codeFromNomNote(String)` (`CodeInference.java:125-139`). Maps a
/// nomenclatural status note to the code it is exclusive to, or `None` when the status is
/// shared between codes and therefore carries no code signal — or when `note` is `None`
/// (Java: `if (note == null) return null;`, ported here as the param itself being an
/// `Option<&str>` rather than a caller-side guard, since the null-check is the very first
/// line of Java's own method body).
///
/// Botanical-only (ICN): nom. inval. / illeg. / superfl. (super.) / cons. / rej. (rejic.) /
/// ambig. / confus. Zoological-only (ICZN): nomen oblitum / nomen protectum. Shared and thus
/// neutral: nom. nud. / dub. / nov., comb. nov., stat. nov., orth., correct, transf.
///
/// Word-boundary matching (not bare substring search) so a status token is only recognised
/// as a whole word — "cons" matches "nom. cons." but not an unrelated word that merely
/// contains the letters (e.g. "reconsider").
pub(crate) fn code_from_nom_note(note: Option<&str>) -> Option<NomCode> {
    let note = note?;
    let s = note.to_lowercase();
    if ICN_STATUS.is_match(&s) {
        return Some(NomCode::Botanical);
    }
    if ICZN_STATUS.is_match(&s) {
        return Some(NomCode::Zoological);
    }
    None
}

/// Java `CodeInference.applyRankCodeMismatch(ParseContext)` (`CodeInference.java:151-160`).
/// Rank-restricted code mismatch with the caller-supplied code → override the code to what
/// the rank requires and warn about it. E.g. `supersect.` is only valid in botany; if the
/// caller asked for ZOOLOGICAL, surface a `CODE_MISMATCH` and pin BOTANICAL.
pub(crate) fn apply_rank_code_mismatch(ctx: &mut ParseContext) {
    let pinned = ctx.name.rank.code();
    if let (Some(pinned), Some(existing), Some(requested)) =
        (pinned, ctx.name.code, ctx.requested_code)
    {
        if existing != pinned && requested != pinned {
            ctx.name.code = Some(pinned);
            ctx.name.add_warning(warnings::CODE_MISMATCH);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Rank;
    use crate::pipeline::authorship_parser::parse as parse_auth;
    use crate::token::tokenize;

    fn auth_state(input: &str) -> AuthState {
        let tokens = tokenize(input);
        parse_auth(&tokens, 0)
    }

    fn ctx_with_rank(rank: Rank) -> ParseContext {
        let mut ctx = ParseContext::new("x".to_string(), None, None, None);
        ctx.name.rank = rank;
        ctx
    }

    // ---- Pin 1: code-exclusive rank ----

    #[test]
    fn pin1_code_exclusive_rank_wins_outright_even_against_contradicting_votes() {
        let mut ctx = ctx_with_rank(Rank::Cultivar); // CULTIVARS-restricted rank
                                                     // "(DC.) L., 1901" alone would be a CONTRADICTION at the vote-tally stage (see
                                                     // `contradicting_votes_leave_code_unset` below) — proving pin 1 truly returns early
                                                     // rather than merely "also happening to win a tally".
        let auth = auth_state("(DC.) L., 1901");
        infer(&mut ctx, Some(&auth));
        assert_eq!(ctx.name.code, Some(NomCode::Cultivars));
    }

    // ---- Pin 2: code-exclusive nomenclatural status ----

    #[test]
    fn pin2_nom_cons_pins_botanical() {
        let mut ctx = ctx_with_rank(Rank::Unranked);
        ctx.name.nomenclatural_note = Some("nom. cons.".to_string());
        infer(&mut ctx, None);
        assert_eq!(ctx.name.code, Some(NomCode::Botanical));
    }

    #[test]
    fn pin2_nomen_oblitum_pins_zoological() {
        let mut ctx = ctx_with_rank(Rank::Unranked);
        ctx.name.nomenclatural_note = Some("nomen oblitum".to_string());
        infer(&mut ctx, None);
        assert_eq!(ctx.name.code, Some(NomCode::Zoological));
    }

    #[test]
    fn code_from_nom_note_returns_none_for_a_shared_status() {
        assert_eq!(code_from_nom_note(Some("nom. nud.")), None);
    }

    #[test]
    fn code_from_nom_note_returns_none_for_none_input() {
        assert_eq!(code_from_nom_note(None), None);
    }

    #[test]
    fn code_from_nom_note_does_not_match_a_word_that_merely_contains_the_token() {
        // "reconsider" contains "cons" but not at a word boundary (preceded by "e", not a
        // boundary) — must not match.
        assert_eq!(code_from_nom_note(Some("reconsider this")), None);
    }

    // ---- vote tally ----

    #[test]
    fn basionym_recombination_votes_botanical() {
        let mut ctx = ctx_with_rank(Rank::Unranked);
        let auth = auth_state("(DC.) L."); // basionym + authored combination, no year anywhere
        infer(&mut ctx, Some(&auth));
        assert_eq!(ctx.name.code, Some(NomCode::Botanical));
    }

    #[test]
    fn basionym_only_author_year_votes_zoological() {
        let mut ctx = ctx_with_rank(Rank::Unranked);
        let auth = auth_state("(Vasileyev, 1939)"); // basionym-only, no combination author
        infer(&mut ctx, Some(&auth));
        assert_eq!(ctx.name.code, Some(NomCode::Zoological));
    }

    #[test]
    fn candidatus_votes_bacterial() {
        let mut ctx = ctx_with_rank(Rank::Unranked);
        ctx.name.candidatus = true;
        infer(&mut ctx, None);
        assert_eq!(ctx.name.code, Some(NomCode::Bacterial));
    }

    #[test]
    fn contradicting_votes_leave_code_unset() {
        let mut ctx = ctx_with_rank(Rank::Unranked);
        // "(DC.) L., 1901": basionym + authored combination (BOTANICAL "(Basionym)
        // Recombination" vote) AND a year on that same authored combination (ZOOLOGICAL "a
        // year on an authored … combination" vote) — two distinct votes, no winner.
        let auth = auth_state("(DC.) L., 1901");
        infer(&mut ctx, Some(&auth));
        assert_eq!(ctx.name.code, None);
    }

    #[test]
    fn no_signal_at_all_leaves_code_unset() {
        let mut ctx = ctx_with_rank(Rank::Unranked);
        infer(&mut ctx, None);
        assert_eq!(ctx.name.code, None);
    }

    // ---- apply_rank_code_mismatch ----

    #[test]
    fn rank_code_mismatch_overrides_and_warns() {
        let mut ctx = ctx_with_rank(Rank::SupersectionBotany); // BOTANICAL-restricted
        ctx.name.code = Some(NomCode::Zoological);
        ctx.requested_code = Some(NomCode::Zoological);
        apply_rank_code_mismatch(&mut ctx);
        assert_eq!(ctx.name.code, Some(NomCode::Botanical));
        assert!(ctx
            .name
            .warnings
            .contains(&warnings::CODE_MISMATCH.to_string()));
    }

    #[test]
    fn rank_code_mismatch_skips_when_requested_code_matches_pinned() {
        let mut ctx = ctx_with_rank(Rank::SupersectionBotany);
        ctx.name.code = Some(NomCode::Zoological);
        ctx.requested_code = Some(NomCode::Botanical); // matches the pinned code
        apply_rank_code_mismatch(&mut ctx);
        assert_eq!(
            ctx.name.code,
            Some(NomCode::Zoological),
            "no override: requested == pinned"
        );
        assert!(ctx.name.warnings.is_empty());
    }

    #[test]
    fn rank_code_mismatch_skips_when_requested_code_is_none() {
        let mut ctx = ctx_with_rank(Rank::SupersectionBotany);
        ctx.name.code = Some(NomCode::Zoological);
        ctx.requested_code = None;
        apply_rank_code_mismatch(&mut ctx);
        assert_eq!(ctx.name.code, Some(NomCode::Zoological));
        assert!(ctx.name.warnings.is_empty());
    }

    #[test]
    fn rank_code_mismatch_skips_when_rank_has_no_code() {
        let mut ctx = ctx_with_rank(Rank::Genus); // code-agnostic
        ctx.name.code = Some(NomCode::Zoological);
        ctx.requested_code = Some(NomCode::Botanical);
        apply_rank_code_mismatch(&mut ctx);
        assert_eq!(ctx.name.code, Some(NomCode::Zoological));
        assert!(ctx.name.warnings.is_empty());
    }

    #[test]
    fn rank_code_mismatch_skips_when_code_already_matches_pinned() {
        let mut ctx = ctx_with_rank(Rank::SupersectionBotany);
        ctx.name.code = Some(NomCode::Botanical); // already matches
        ctx.requested_code = Some(NomCode::Zoological);
        apply_rank_code_mismatch(&mut ctx);
        assert_eq!(ctx.name.code, Some(NomCode::Botanical));
        assert!(ctx.name.warnings.is_empty());
    }
}
