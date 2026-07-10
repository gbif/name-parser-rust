// SPDX-License-Identifier: Apache-2.0

//! Java `org.gbif.nameparser.pipeline.RankMarkers` — the two rank-marker lookup tables
//! (`INFRASPECIFIC`, `INFRAGENERIC`: marker string -> [`Rank`]) and their notho-aware
//! matchers. Consumed by the not-yet-ported `AuthorshipSplit`/`NameTokens` stages (Phase 1
//! Slice 3 Tasks 2-3) to recognise rank-marker tokens ("subsp.", "var.", "sect.", …)
//! between name-part epithets.
//!
//! Recognised infraspecific and infrageneric rank markers (case-insensitive). Per Java's
//! own class doc, "trailing dot optional" — but that's a property of the SYSTEM, not this
//! module: every Java call site (`AuthorshipSplit`/`NameTokens`, both via their own private
//! `stripDot` helper) strips a trailing dot from the token text BEFORE calling in here, so
//! neither `INFRASPECIFIC`/`INFRAGENERIC` nor any function below ever sees or strips one
//! itself (verified against every call site in `AuthorshipSplit.java`/`NameTokens.java` —
//! all four pass `stripDot(t.text)`, never `t.text` directly). Tier 1 keeps the
//! high-coverage subset; rarer markers (cv., grex, microbial bv./ct./sv., agamosp.) are
//! already included below — nothing was trimmed from Java's map.
//!
//! `stripandstash.rs` has its own small pre-existing stand-in
//! (`KNOWN_INFRASPECIFIC_MARKERS`/`is_known_infraspecific_marker`, predating this module)
//! used only by its `normalise_letter_subdivision_marker` step; that file's own doc comment
//! already flags it as a placeholder for "the dedicated rank-handling slice" (this one) —
//! left as is here since consolidating it isn't in this task's scope, but a future cleanup
//! could replace it with [`match_infraspecific`].

// This module's maps/functions are only exercised by their own unit tests so far —
// `AuthorshipSplit`/`NameTokens` (Phase 1 Slice 3 Tasks 2-3) are the real call sites and
// land in the next two tasks. Until then rustc's `dead_code` lint fires on every item here
// (their effective visibility is capped at `pub(crate)` by the enclosing `pub(crate) mod
// rank_markers` in `pipeline/mod.rs`, same as `context::ParseContext`'s own
// `#[allow(dead_code)]`, which hit this identical situation — see that struct's doc
// comment). Drop this once Task 2 adds the first non-test caller.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::model::Rank;

/// Java `RankMarkers.LETTER_SUBDIVISION` (`RankMarkers.java:21`): the synthetic
/// single-token marker `StripAndStash` substitutes for informal letter-based species
/// subdivisions ("a.", "b.", "a.b." — old floras) so the normal rank-marker machinery
/// (this module) treats the following epithet as an infraspecific of the unmappable rank
/// [`Rank::Other`]. Never appears in user input. (`stripandstash.rs` carries its own
/// same-valued private copy of this literal, predating this module — see this module's
/// own doc comment.)
pub const LETTER_SUBDIVISION: &str = "infrasubdivision";

/// Java `RankMarkers.INFRASPECIFIC` (`RankMarkers.java:23,27-74`) — marker string ->
/// [`Rank`]. Every entry transcribed verbatim, including `LETTER_SUBDIVISION` itself
/// (which Java's own map also carries as a key) and the non-alphabetic `"*"` key (an old
/// infraspecific separator between two lowercase epithets).
static INFRASPECIFIC: LazyLock<HashMap<&'static str, Rank>> = LazyLock::new(|| {
    HashMap::from([
        (LETTER_SUBDIVISION, Rank::Other),
        ("subsp", Rank::Subspecies),
        ("ssp", Rank::Subspecies),
        ("var", Rank::Variety),
        ("subvar", Rank::Subvariety),
        ("subv", Rank::Subvariety),
        ("f", Rank::Form),
        ("forma", Rank::Form),
        ("form", Rank::Form),
        ("fo", Rank::Form),
        ("subf", Rank::Subform),
        ("subforma", Rank::Subform),
        ("pv", Rank::Pathovar),
        ("pathovar", Rank::Pathovar),
        ("bv", Rank::Biovar),
        ("biovar", Rank::Biovar),
        ("ct", Rank::Chemoform),
        ("chemoform", Rank::Chemoform),
        ("sv", Rank::Serovar),
        ("serovar", Rank::Serovar),
        ("morph", Rank::Morph),
        ("morphovar", Rank::Morphovar),
        ("phagovar", Rank::Phagovar),
        ("nat", Rank::Natio),
        ("natio", Rank::Natio),
        ("mut", Rank::Mutatio),
        ("mutatio", Rank::Mutatio),
        ("agamosp", Rank::Species),
        ("agamossp", Rank::Subspecies),
        ("agamovar", Rank::Variety),
        ("conv", Rank::Convariety),
        ("convar", Rank::Convariety),
        ("subspec", Rank::Subspecies),
        ("variety", Rank::Variety),
        ("fm", Rank::Form),
        ("fma", Rank::Form),
        ("prol", Rank::Proles),
        ("proles", Rank::Proles),
        ("ab", Rank::Aberration),
        ("aberration", Rank::Aberration),
        ("strain", Rank::Strain),
        ("str", Rank::Strain),
        // "st." used in some old fungal works as a generic infraspecific marker.
        ("st", Rank::InfraspecificName),
        // "*" between two lowercase epithets is an old infraspecific separator.
        ("*", Rank::InfraspecificName),
    ])
});

/// Java `RankMarkers.INFRAGENERIC` (`RankMarkers.java:24,79-89`) — marker string ->
/// [`Rank`].
static INFRAGENERIC: LazyLock<HashMap<&'static str, Rank>> = LazyLock::new(|| {
    HashMap::from([
        // "div." between a genus and an infrageneric epithet is the botanical divisio rank
        // (Lindley's "Rosa div. Caninae"), not the zoological suprageneric division.
        ("div", Rank::DivisionBotany),
        ("divisio", Rank::DivisionBotany),
        ("subg", Rank::Subgenus),
        ("subgen", Rank::Subgenus),
        ("sect", Rank::SectionBotany),
        ("subsect", Rank::SubsectionBotany),
        ("supersect", Rank::SupersectionBotany),
        // IPNI also writes "supersect." as "suprasect." — same rank.
        ("suprasect", Rank::SupersectionBotany),
        ("ser", Rank::SeriesBotany),
        ("subser", Rank::SubseriesBotany),
    ])
});

/// Java `RankMarkers.matchInfraspecific(String)` (`RankMarkers.java:132-134`): plain
/// (non-notho) lookup, case-insensitive.
pub fn match_infraspecific(word: &str) -> Option<Rank> {
    // Java's `.toLowerCase()` uses the JVM default locale; Rust's `.to_lowercase()` is the
    // full-Unicode locale-independent mapping — every marker string in `INFRASPECIFIC` is
    // pure ASCII, so the two can never disagree on whether a given input matches (same
    // reasoning as `token::is_particle`/`Preflight::run`'s `.to_lowercase()` calls).
    INFRASPECIFIC.get(word.to_lowercase().as_str()).copied()
}

/// Java `RankMarkers.matchInfrageneric(String)` (`RankMarkers.java:136-138`): plain
/// (non-notho) lookup, case-insensitive.
pub fn match_infrageneric(word: &str) -> Option<Rank> {
    INFRAGENERIC.get(word.to_lowercase().as_str()).copied()
}

/// Java `RankMarkers.matchInfragenericAllowNotho(String, boolean[])`
/// (`RankMarkers.java:93-104`). Recognises a "notho-" prefix variant (e.g. "nothosect" for
/// "sect") ahead of the plain lookup. Returns `Some((rank, true))` when a notho prefix was
/// present AND stripped down to a recognised marker, `Some((rank, false))` for a direct
/// (non-notho) match, `None` for no match at all — collapsing Java's out-parameter
/// `boolean[] notho` (meaningless when the method returns `null`) into the tupled
/// `Option`.
///
/// Faithful to Java's exact fallthrough: a word that STARTS WITH "notho" but whose
/// remainder isn't itself a recognised marker does NOT short-circuit to `None` — it falls
/// through to the final plain-lookup line and is retried as a whole word (unreachable in
/// practice since no map key itself starts with "notho", but the control flow is ported
/// branch-for-branch, not simplified/short-circuited).
pub fn match_infrageneric_allow_notho(word: &str) -> Option<(Rank, bool)> {
    let w = word.to_lowercase();
    if let Some(rest) = w.strip_prefix("notho") {
        if let Some(&r) = INFRAGENERIC.get(rest) {
            return Some((r, true));
        }
    }
    INFRAGENERIC.get(w.as_str()).map(|&r| (r, false))
}

/// Java `RankMarkers.matchInfraspecificAllowNotho(String, boolean[])`
/// (`RankMarkers.java:108-127`). Same shape as [`match_infrageneric_allow_notho`], plus a
/// second, short "n"-prefix variant used in some literature ("nvar." == "nothovar.", "nf."
/// == "nothof.", "nsubsp." == "nothosubsp.") — tried strictly AFTER the full "notho-"
/// prefix and strictly BEFORE the final plain lookup, matching Java's linear
/// if/if/fallback order exactly (not restructured into an if/else-if/else: Java's is a bare
/// sequence of independent `if`s that each fall through on a failed INNER map lookup, not
/// merely on a failed prefix test, so an if/else-if would change behaviour for a
/// hypothetical "notho"-prefixed-but-unmapped word whose 1-char-stripped remainder happens
/// to map — see the doc comment on [`match_infrageneric_allow_notho`] for the same point).
pub fn match_infraspecific_allow_notho(word: &str) -> Option<(Rank, bool)> {
    let w = word.to_lowercase();
    if let Some(rest) = w.strip_prefix("notho") {
        if let Some(&r) = INFRASPECIFIC.get(rest) {
            return Some((r, true));
        }
    }
    // Short "n" prefix. Byte-index slicing (`&w[1..]`) is safe here because the guard just
    // below confirms the first byte is the ASCII char 'n' (1 byte), so `1` is always a
    // valid char boundary regardless of what (possibly multi-byte) text follows. Java's
    // `w.length() > 1` (UTF-16 code units) and `w.charAt(0) == 'n'` are likewise equivalent
    // to `.len() > 1` (UTF-8 bytes) / `.starts_with('n')` for this ASCII-first-char case.
    if w.len() > 1 && w.starts_with('n') {
        if let Some(&r) = INFRASPECIFIC.get(&w[1..]) {
            return Some((r, true));
        }
    }
    INFRASPECIFIC.get(w.as_str()).map(|&r| (r, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subsp_matches_subspecies() {
        assert_eq!(match_infraspecific("subsp"), Some(Rank::Subspecies));
        assert_eq!(
            match_infraspecific_allow_notho("subsp"),
            Some((Rank::Subspecies, false))
        );
    }

    #[test]
    fn var_matches_variety() {
        assert_eq!(match_infraspecific("var"), Some(Rank::Variety));
    }

    #[test]
    fn f_and_forma_both_match_form_java_has_no_forma_rank_constant() {
        // Java has no `Rank.FORMA` constant — both the abbreviation "f" and the
        // spelled-out "forma" map to the very same `Rank.FORM` (RankMarkers.java:34-37).
        assert_eq!(match_infraspecific("f"), Some(Rank::Form));
        assert_eq!(match_infraspecific("forma"), Some(Rank::Form));
        assert_eq!(match_infraspecific("form"), Some(Rank::Form));
        assert_eq!(match_infraspecific("fo"), Some(Rank::Form));
    }

    #[test]
    fn sect_matches_section_botany() {
        assert_eq!(match_infrageneric("sect"), Some(Rank::SectionBotany));
    }

    #[test]
    fn subg_and_subgen_both_match_subgenus() {
        assert_eq!(match_infrageneric("subg"), Some(Rank::Subgenus));
        assert_eq!(match_infrageneric("subgen"), Some(Rank::Subgenus));
    }

    #[test]
    fn nothovar_strips_the_notho_prefix_and_flags_it() {
        assert_eq!(
            match_infraspecific_allow_notho("nothovar"),
            Some((Rank::Variety, true))
        );
    }

    #[test]
    fn notho_prefix_also_recognised_for_infrageneric_markers() {
        assert_eq!(
            match_infrageneric_allow_notho("nothosect"),
            Some((Rank::SectionBotany, true))
        );
        // No match at all when the remainder isn't a recognised infrageneric marker.
        assert_eq!(match_infrageneric_allow_notho("nothoxyz"), None);
    }

    #[test]
    fn short_n_prefix_is_infraspecific_only_and_notho_flagged() {
        // "nvar" (short "n" prefix, no full "notho") == "nothovar" per Java's own comment.
        assert_eq!(
            match_infraspecific_allow_notho("nvar"),
            Some((Rank::Variety, true))
        );
        assert_eq!(
            match_infraspecific_allow_notho("nf"),
            Some((Rank::Form, true))
        );
        // The infrageneric matcher has no such short-prefix fallback (Java only implements
        // it on `matchInfraspecificAllowNotho`) — "nsect" is simply not a marker at all.
        assert_eq!(match_infrageneric_allow_notho("nsect"), None);
    }

    #[test]
    fn case_insensitive_matching() {
        assert_eq!(match_infraspecific("SUBSP"), Some(Rank::Subspecies));
        assert_eq!(match_infrageneric("Sect"), Some(Rank::SectionBotany));
        assert_eq!(
            match_infraspecific_allow_notho("NOTHOVAR"),
            Some((Rank::Variety, true))
        );
    }

    #[test]
    fn unrecognised_marker_returns_none() {
        assert_eq!(match_infraspecific("xyz"), None);
        assert_eq!(match_infrageneric("xyz"), None);
        assert_eq!(match_infraspecific_allow_notho("xyz"), None);
        assert_eq!(match_infrageneric_allow_notho("xyz"), None);
    }

    #[test]
    fn trailing_dot_is_not_stripped_by_this_module_the_caller_must_do_it() {
        // Every real Java call site pre-strips the dot via its own `stripDot` helper
        // (ported in Tasks 2-3) before calling in here — RankMarkers itself never does, so
        // a dot-suffixed marker deliberately does NOT match in this module alone.
        assert_eq!(match_infraspecific("subsp."), None);
        assert_eq!(match_infraspecific_allow_notho("var."), None);
        assert_eq!(match_infrageneric("sect."), None);
    }

    #[test]
    fn letter_subdivision_synthetic_marker_maps_to_rank_other() {
        assert_eq!(match_infraspecific(LETTER_SUBDIVISION), Some(Rank::Other));
    }

    #[test]
    fn star_is_the_old_infraspecific_separator() {
        assert_eq!(match_infraspecific("*"), Some(Rank::InfraspecificName));
    }

    #[test]
    fn division_botany_marker_and_its_suprasect_alias() {
        assert_eq!(match_infrageneric("div"), Some(Rank::DivisionBotany));
        assert_eq!(match_infrageneric("divisio"), Some(Rank::DivisionBotany));
        // "supersect."/"suprasect." (IPNI spelling variant) are aliases for the same rank.
        assert_eq!(
            match_infrageneric("supersect"),
            Some(Rank::SupersectionBotany)
        );
        assert_eq!(
            match_infrageneric("suprasect"),
            Some(Rank::SupersectionBotany)
        );
    }

    #[test]
    fn microbial_and_legacy_infraspecific_markers() {
        assert_eq!(match_infraspecific("pv"), Some(Rank::Pathovar));
        assert_eq!(match_infraspecific("biovar"), Some(Rank::Biovar));
        assert_eq!(match_infraspecific("ct"), Some(Rank::Chemoform));
        assert_eq!(match_infraspecific("sv"), Some(Rank::Serovar));
        assert_eq!(match_infraspecific("agamosp"), Some(Rank::Species));
        assert_eq!(match_infraspecific("agamossp"), Some(Rank::Subspecies));
        assert_eq!(match_infraspecific("agamovar"), Some(Rank::Variety));
        assert_eq!(match_infraspecific("strain"), Some(Rank::Strain));
        assert_eq!(match_infraspecific("str"), Some(Rank::Strain));
        assert_eq!(match_infraspecific("st"), Some(Rank::InfraspecificName));
    }
}
