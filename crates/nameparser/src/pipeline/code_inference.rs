// SPDX-License-Identifier: Apache-2.0

//! Java `org.gbif.nameparser.pipeline.CodeInference` (161 lines) — infers a name's
//! nomenclatural [`NomCode`] from evidence that exists under one code only.
//!
//! Only code-specific evidence sets a code: a rank, citation, status or hybrid notation that a
//! single code knows (#60). The shape of an authorship is no such evidence. A year, a comma, a
//! bracketed basionym, an `ex` or a filius is citation style, which sources follow across codes:
//! TAXREF cites its plants `Coode & Cullen, 1965` and LPSN its bacteria `Lee et al. 2004`, while
//! bacteria share the botanists' `(Basionym) Combination` and `ex`. Java voted on that shape; of
//! the ChecklistBank names that declare a code and carry a year, a year voted ZOOLOGICAL for about
//! 98% of the botanical ones and all bacterial ones. Without such evidence the code stays unset.
//! [`crate::pipeline::assemble::finish`] calls [`infer`] once, only when the name has no code
//! yet, and [`apply_rank_code_mismatch`] unconditionally afterwards.

use std::sync::LazyLock;

use regex::Regex;

use crate::model::{warnings, NomCode};
use crate::pipeline::authorship_parser::{is_sanctioning_author, AuthState};
use crate::pipeline::ParseContext;

/// Java `CodeInference.ICZN_STATUS` (`CodeInference.java:143-144`), compiled with no flags —
/// Java's `\b` is therefore ASCII-only, ported here as `(?-u:\b…)` (this port's per-pattern
/// flag rule; see `pipeline::preflight`'s module doc for the rule spelled out in full).
/// Zoological-only (ICZN) nomenclatural statuses: nomen oblitum / nomen protectum.
static ICZN_STATUS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\b(?:oblitum|protectum)\b)").unwrap());

/// Java `CodeInference.infer(ParseContext, AuthorshipParser.AuthState)`
/// (`CodeInference.java:47-111`), less its vote tally over the authorship's shape (#60). Called
/// by [`crate::pipeline::assemble::finish`] only when `ctx.name.code.is_none()`. The first
/// evidence found sets the code:
///   1. a code-exclusive rank ([`crate::model::Rank::code`] != `None`), from its marker or the
///      caller — the cultivar, viral and bacterial `*-var` ranks, forma specialis, botanical
///      section/series, etc. The generic markers subsp./var./f. carry *no* code (they are used
///      across codes) and so are no evidence.
///   2. a named hybrid (BOTANICAL), sanctioning by Fries or Persoon (BOTANICAL), an Approved
///      Lists citation or a revived name (BACTERIAL), a Candidatus name (BACTERIAL).
///   3. a zoological-only nomenclatural status ([`code_from_nom_note`]).
pub(crate) fn infer(ctx: &mut ParseContext, auth_state: Option<&AuthState>) {
    // 1. A code-exclusive rank.
    if let Some(rank_code) = ctx.name.rank.code() {
        ctx.name.code = Some(rank_code);
        return;
    }

    // A named hybrid is botanical: zoology and bacteriology name no hybrids. Not with a bracketed
    // subgenus, rarer in botany than in zoology: the water flea "Daphnia (Daphnia) x krausi
    // Flossner 1993" is no plant. A cultivar hybrid's rank pinned the cultivated-plant code above.
    let hybrid = ctx.name.notho.as_ref().is_some_and(|n| !n.is_empty());
    if hybrid && ctx.name.infrageneric_epithet.is_none() {
        ctx.name.code = Some(NomCode::Botanical);
        return;
    }
    // Sanctioning by Fries or Persoon exists under the botanical code only ("Link:Fr., 1809"), the
    // basionym's too ("(Wulfen : Fr.)"). Any other author after a colon is a misread colon.
    let sanctioned = [
        auth_state.and_then(|st| st.combination.sanctioning_author.as_deref()),
        auth_state.and_then(|st| st.basionym.sanctioning_author.as_deref()),
        ctx.name
            .combination_authorship
            .sanctioning_author
            .as_deref(),
        ctx.name.basionym_authorship.sanctioning_author.as_deref(),
    ];
    if sanctioned.into_iter().flatten().any(is_sanctioning_author) {
        ctx.name.code = Some(NomCode::Botanical);
        return;
    }
    // The 1980 Approved Lists of Bacterial Names exist under the prokaryote code only, and so does
    // the citation of a revived name, "(ex Choukévitch 1911) Nakamura 1984" (ICNP Rule 28a), and a
    // Candidatus name.
    if ctx.approved_lists || ctx.name.candidatus || auth_state.is_some_and(|st| st.revived_name) {
        ctx.name.code = Some(NomCode::Bacterial);
        return;
    }

    // 3. A zoological-only nomenclatural status.
    if let Some(note_code) = code_from_nom_note(ctx.name.nomenclatural_note.as_deref()) {
        ctx.name.code = Some(note_code);
    }
}

/// Java `CodeInference.codeFromNomNote(String)` (`CodeInference.java:125-139`). Maps a
/// nomenclatural status note to the code it is exclusive to, or `None` when the status is
/// shared between codes and therefore carries no code signal — or when `note` is `None`.
///
/// Zoological-only (ICZN): nomen oblitum / nomen protectum. Java also took nom. inval. / illeg. /
/// superfl. / cons. / rej. / ambig. / confus. for botanical-only, but prokaryote names carry them
/// too (cyanobacteria `Lyngbya erecta Setchell & Gardner, nom. illeg.`, the ICNP's `nom. cons.`),
/// so they tell no code (#60).
///
/// Word-boundary matching (not bare substring search) so a status token is only recognised
/// as a whole word.
pub(crate) fn code_from_nom_note(note: Option<&str>) -> Option<NomCode> {
    let note = note?;
    ICZN_STATUS
        .is_match(&note.to_lowercase())
        .then_some(NomCode::Zoological)
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

    // ---- code-exclusive rank ----

    #[test]
    fn code_exclusive_rank_wins_outright_over_the_authorship() {
        let mut ctx = ctx_with_rank(Rank::Cultivar); // CULTIVARS-restricted rank
        let auth = auth_state("(DC.) L., 1901");
        infer(&mut ctx, Some(&auth));
        assert_eq!(ctx.name.code, Some(NomCode::Cultivars));
    }

    // ---- code-exclusive nomenclatural status ----

    #[test]
    fn nom_cons_tells_no_code() {
        let mut ctx = ctx_with_rank(Rank::Unranked);
        ctx.name.nomenclatural_note = Some("nom. cons.".to_string());
        infer(&mut ctx, None);
        assert_eq!(ctx.name.code, None);
    }

    #[test]
    fn nomen_oblitum_pins_zoological() {
        let mut ctx = ctx_with_rank(Rank::Unranked);
        ctx.name.nomenclatural_note = Some("nomen oblitum".to_string());
        infer(&mut ctx, None);
        assert_eq!(ctx.name.code, Some(NomCode::Zoological));
    }

    #[test]
    fn code_from_nom_note_returns_none_for_a_shared_status() {
        assert_eq!(code_from_nom_note(Some("nom. nud.")), None);
        assert_eq!(code_from_nom_note(Some("nom. illeg.")), None);
    }

    #[test]
    fn code_from_nom_note_returns_none_for_none_input() {
        assert_eq!(code_from_nom_note(None), None);
    }

    // ---- authorship shape is no evidence (#60) ----

    #[test]
    fn authorship_shape_tells_no_code() {
        for a in [
            "(DC.) L.",
            "(Vasileyev, 1939)",
            "Coode & Cullen, 1965",
            "Lee et al. 2004",
            "Mart. ex DC.",
            "Hook. f.",
        ] {
            let mut ctx = ctx_with_rank(Rank::Unranked);
            infer(&mut ctx, Some(&auth_state(a)));
            assert_eq!(ctx.name.code, None, "{a}");
        }
    }

    #[test]
    fn fries_sanctioning_pins_botanical() {
        let mut ctx = ctx_with_rank(Rank::Unranked);
        infer(&mut ctx, Some(&auth_state("Bull. : Fr.")));
        assert_eq!(ctx.name.code, Some(NomCode::Botanical));
    }

    #[test]
    fn revived_name_pins_bacterial() {
        let mut ctx = ctx_with_rank(Rank::Unranked);
        infer(
            &mut ctx,
            Some(&auth_state("(ex Choukévitch 1911) Nakamura 1984")),
        );
        assert_eq!(ctx.name.code, Some(NomCode::Bacterial));
    }

    #[test]
    fn candidatus_pins_bacterial() {
        let mut ctx = ctx_with_rank(Rank::Unranked);
        ctx.name.candidatus = true;
        infer(&mut ctx, None);
        assert_eq!(ctx.name.code, Some(NomCode::Bacterial));
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
