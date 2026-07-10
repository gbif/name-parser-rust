// SPDX-License-Identifier: Apache-2.0

//! Java `org.gbif.nameparser.pipeline.Assemble` (262 lines) — the pipeline's final stage:
//! invariants on the produced [`ParsedName`] (rank defaulted, name-part residuals closed,
//! code inferred where possible, implausible data flagged).
//!
//! [`finish`] runs **19 ordered steps** against `ctx.name`, in the exact order of Java's own
//! `finish` method body (one step per top-level statement/`if`-block there). This order is
//! load-bearing — several steps depend on an earlier step's output (e.g. step 2 can pre-empt
//! step 6 by already changing `n.getRank()`; step 14's suffix-based rank inference only fires
//! when an earlier step left the rank `UNRANKED`). Each step below is numbered for
//! traceability against `Assemble.java`:
//!
//!  1. aggregate marker + specific epithet, no infraspecific epithet → `SPECIES_AGGREGATE`.
//!  2. caller-requested `SPECIES_AGGREGATE` forwarded onto a name with a specific epithet.
//!  3. `rank == null` → `UNRANKED` (unreachable in this port — see the step's own comment).
//!  4. monomial + caller rank at species-or-below → INDETERMINED placeholder (nulls both
//!     authorships).
//!  5. monomial whose (caller or parsed) rank is strictly infrageneric → uninomial moves to
//!     `infragenericEpithet`.
//!  6. binomial + caller rank higher than species → rank pinned to caller, flagged informal +
//!     doubtful + `RANK_MISMATCH`.
//!  7. an underscore in a scientific-type uninomial splits into genus+species or
//!     uninomial+phrase.
//!  8. a stashed pending-unparsed remainder → `PARTIAL` state.
//!  9. a detected year range → `YEAR_INTERPRETED` warning.
//! 10. code inference (only when `code` is still unset).
//! 11. viral shape with no other code signal → `VIRUS`.
//! 12. rank-restricted code/caller-code mismatch → override + `CODE_MISMATCH`.
//! 13. `INFRASPECIFIC_NAME` + `ZOOLOGICAL` → `SUBSPECIES`.
//! 14. suffix-based rank inference for a still-`UNRANKED` monomial.
//! 15. literal "null" / blacklisted epithet → doubtful + warning.
//! 16. implausible authorship year → doubtful + `UNLIKELY_YEAR`.
//! 17. a cultivar epithet clears `INFORMAL`/`INDETERMINED`.
//! 18. a bare phrase (no cultivar) → genus-promotion (below genus) + `INFORMAL`.
//! 19. a quoted leading monomial is re-wrapped in its quotes.

use std::sync::LazyLock;

use regex::Regex;

use crate::model::{rank_utils, warnings, Authorship, NameType, NomCode, ParsedName, Rank, State};
use crate::pipeline::authorship_parser::AuthState;
use crate::pipeline::blacklisted_epithets;
use crate::pipeline::code_inference;
use crate::pipeline::ParseContext;

/// Java `Assemble.finish(ParseContext, AuthorshipParser.AuthState)` (`Assemble.java:20-195`).
/// See the module doc comment for the 19-ordered-steps overview.
pub(crate) fn finish(ctx: &mut ParseContext, auth_state: Option<&AuthState>) {
    // Step 1: an aggregate marker (stripped upstream into `ctx.aggregate`) on a name with a
    // specific epithet but no infraspecific epithet is a SPECIES_AGGREGATE.
    if ctx.aggregate
        && ctx.name.specific_epithet.is_some()
        && ctx.name.infraspecific_epithet.is_none()
    {
        ctx.name.rank = Rank::SpeciesAggregate;
    }

    // Java captures `ctx.requestedRank` into a local once here and reuses it through several
    // of the steps below (2, 4, 5, 6) — mirrored with a single `let` here too.
    let requested = ctx.requested_rank;

    // Step 2: caller-supplied rank wins when explicit (and not just UNRANKED) and the parsed
    // structure is compatible — here, specifically forwarding an explicit SPECIES_AGGREGATE
    // request onto a name that has a specific epithet. NOTE the load-bearing order: this can
    // pre-empt step 6 below by already making `req == ctx.name.rank` true (see
    // `requested_species_aggregate_preempts_the_later_rank_mismatch_step` in the tests).
    if let Some(req) = requested {
        if req != Rank::Unranked
            && req != ctx.name.rank
            && req == Rank::SpeciesAggregate
            && ctx.name.specific_epithet.is_some()
        {
            ctx.name.rank = Rank::SpeciesAggregate;
        }
    }

    // Step 3: Java `if (n.getRank() == null) n.setRank(UNRANKED);` — unreachable in this
    // port. `ParsedName::rank` is a plain (non-`Option`) `Rank`, always a concrete value
    // (defaulting to `Rank::Unranked` itself already), so there is no "null rank" state to
    // fold here — this is in fact already dead code in Java too (the field is `@Nonnull`
    // with a `= Rank.UNRANKED` initializer, so it can never actually be null by the time
    // `finish` runs). Kept as a documented no-op step, not silently dropped, to preserve the
    // 19-step numbering against `Assemble.java`.

    // Step 4: monomial + caller-supplied rank at species level or below → indeterminate
    // placeholder (e.g. "Polygonum" + CULTIVAR → genus "Polygonum" with cv. marker;
    // "Lepidoptera Hooker" + SPECIES → genus parsed as genus-only, but caller says it's a
    // species).
    if let Some(req) = requested {
        if req != Rank::Unranked
            && (req == Rank::Species
                || req.is_infraspecific()
                || req == Rank::Cultivar
                || req == Rank::CultivarGroup
                || req == Rank::Grex)
            && ctx.name.uninomial.is_some()
            && ctx.name.specific_epithet.is_none()
            && ctx.name.type_ != NameType::Informal
        {
            ctx.name.genus = ctx.name.uninomial.take();
            ctx.name.rank = req;
            ctx.name.type_ = NameType::Informal;
            ctx.name.add_warning(warnings::INDETERMINED);
            // Java `n.setCombinationAuthorship(null)`/`setBasionymAuthorship(null)`: these
            // fields are eagerly-initialized (never actually `null`) `Authorship` objects on
            // this port's `ParsedName` (see `model::name`'s module doc — "never omitted" on
            // the wire), so Java's "null" is represented here as the empty
            // `Authorship::default()` rather than an absent key. Same authorship DATA (no
            // authors, no year) either way; the wire SHAPE differs (Gson would omit a null
            // field vs. this port always emitting `{"authors":[],"exAuthors":[]}`) — flagged
            // here for Task 4's harness extension to double-check against the Java oracle
            // once this path is reachable end-to-end.
            ctx.name.combination_authorship = Authorship::default();
            ctx.name.basionym_authorship = Authorship::default();
        }
    }

    // Step 5: monomial whose rank is strictly infrageneric (SUBGENUS, SECTION_BOTANY, …) →
    // move the uninomial into infragenericEpithet. Triggers both for caller-supplied ranks
    // ("Polygonum" + SUBGENUS) and for a leading rank marker stripped by StripAndStash
    // ("subgen. Trematostoma" → rank=SUBGENUS, uninomial=Trematostoma).
    {
        let r = match requested {
            Some(req) if req.is_infrageneric_strictly() => req,
            _ => ctx.name.rank,
        };
        if r.is_infrageneric_strictly()
            && ctx.name.uninomial.is_some()
            && ctx.name.genus.is_none()
            && ctx.name.infrageneric_epithet.is_none()
        {
            ctx.name.infrageneric_epithet = ctx.name.uninomial.take();
            ctx.name.rank = r;
        }
    }

    // Step 6: binomial (or richer) + caller-supplied higher-rank → keep the parsed structure
    // but pin the rank to what the caller asked, flag the mismatch as informal + doubtful
    // with a RANK_MISMATCH warning ("Polygonum alba" + GENUS).
    if let Some(req) = requested {
        if req != Rank::Unranked
            && ctx.name.specific_epithet.is_some()
            && req.higher_than(Rank::Species)
            && req != ctx.name.rank
        {
            ctx.name.rank = req;
            ctx.name.type_ = NameType::Informal;
            ctx.name.doubtful = true;
            ctx.name.add_warning(warnings::RANK_MISMATCH);
        }
    }

    // Step 7: a monomial with an underscore is either "Genus_species" (underscore as space:
    // genus + specific epithet, when the after-part starts lowercase) or a GTDB-style phrase
    // name (e.g. "Desulfobacterota_B": uninomial + phrase, when the after-part starts
    // uppercase).
    if ctx.name.type_ == NameType::Scientific {
        if let Some(idx) = ctx.name.uninomial.as_ref().and_then(|u| u.find('_')) {
            let uni = ctx.name.uninomial.clone().expect("just matched Some above");
            let before = uni[..idx].to_string();
            let after = uni[idx + 1..].to_string();
            if after.chars().next().is_some_and(|c| c.is_lowercase()) {
                // "Oxalis_barrelieri" → genus + specific epithet
                ctx.name.uninomial = None;
                ctx.name.genus = Some(before);
                ctx.name.specific_epithet = Some(after);
                ctx.name.rank = Rank::Species;
            } else {
                // "Desulfobacterota_B" → GTDB-style phrase name
                ctx.name.uninomial = Some(before);
                ctx.name.phrase = Some(after);
                ctx.name.type_ = NameType::Informal;
                ctx.name.rank = Rank::Unranked;
            }
        }
    }

    // Step 8: a pending unparsed remainder stashed upstream (and not already consumed)
    // demotes the name to a PARTIAL parse.
    if ctx.pending_unparsed.is_some() && ctx.name.unparsed.is_none() {
        ctx.name.state = State::Partial;
        ctx.name.unparsed = ctx.pending_unparsed.clone();
    }

    // Step 9: a year range in the authorship ("1845-1847") was interpreted down to just its
    // first year — flag it so callers know the year was reduced from a range.
    if auth_state.is_some_and(|s| s.year_range) {
        ctx.name.add_warning(warnings::YEAR_INTERPRETED);
    }

    // Step 10: indeterminate infraspecific names ("Nitzschia sinuata var. (Grunow)
    // Lange-Bert.", "Canis lupus subsp. Linnaeus, 1758") keep the authorship trailing the
    // rank marker — it belongs to the (unnamed) infraspecific taxon and is not a parsing
    // artefact. All code-setting heuristics live in `code_inference` (called only when the
    // name has no code yet).
    if ctx.name.code.is_none() {
        code_inference::infer(ctx, auth_state);
    }

    // Step 11: a virally-shaped name (ICTV rank suffix, Preflight-detected) with no other
    // code signal defaults to VIRUS.
    if ctx.viral_shape && ctx.name.code.is_none() {
        ctx.name.code = Some(NomCode::Virus);
    }

    // Step 12: rank-restricted code mismatch with the caller-supplied code → override the
    // code to what the rank requires and warn (e.g. supersect. is botany-only).
    code_inference::apply_rank_code_mismatch(ctx);

    // Step 13: zoological trinomials default to SUBSPECIES, not the generic
    // INFRASPECIFIC_NAME: ICZN doesn't use rank markers for subspecies, so a bare "Genus
    // species infra" with the zoological code (caller-supplied or inferred) is by convention
    // a subspecies.
    if ctx.name.rank == Rank::InfraspecificName
        && ctx.name.code == Some(NomCode::Zoological)
        && ctx.name.infraspecific_epithet.is_some()
    {
        ctx.name.rank = Rank::Subspecies;
    }

    // Step 14: suffix-based rank inference for monomials: use the explicitly-requested code
    // when provided, otherwise fall back to globally unambiguous suffixes only (-aceae,
    // -oideae). Viral code is inferred from a highly reliable suffix, so it is safe to drive
    // suffix-based rank inference (e.g. "Coronaviridae" → FAMILY). Never apply code-specific
    // suffix maps derived from authorship-inferred code — that would silently assign ranks to
    // names whose code we merely guessed.
    if ctx.name.rank == Rank::Unranked {
        if let Some(uninomial) = ctx.name.uninomial.clone() {
            let code_for_inference =
                ctx.requested_code
                    .or(if ctx.viral_shape { ctx.name.code } else { None });
            let r = match code_for_inference {
                Some(code) => rank_from_suffix(&uninomial, code),
                None => rank_from_global_suffix(&uninomial),
            };
            if let Some(r) = r {
                ctx.name.rank = r;
            }
        }
    }

    // Step 15: flag the literal "null" epithet and any blacklisted epithet as doubtful.
    flag_blacklisted_epithets(&mut ctx.name);

    // Step 16: flag implausible authorship years ("Wilcox, 137", "Hall, 0000", "Bromley,
    // 193k7" → "193").
    flag_unlikely_years(&mut ctx.name);

    // Step 17: a cultivar epithet pins the name as a valid scientific identification → clear
    // the INFORMAL flag and INDETERMINED warning that an "sp." indet marker may have left
    // behind ("Symphoricarpos sp. cv. 'mother of pearl'" is a complete cultivar name).
    if ctx.name.cultivar_epithet.is_some() {
        if ctx.name.type_ == NameType::Informal {
            ctx.name.type_ = NameType::Scientific;
        }
        ctx.name.warnings.retain(|w| w != warnings::INDETERMINED);
    }

    // Step 18: a phrase epithet without a cultivar always denotes an INFORMAL name → promote
    // a monomial uninomial to a genus so callers see "Baeckea ssp. <phrase>" as a phrase name
    // on a genus, not a uninomial scientific name. Skip promotion for suprageneric phrase
    // forms (e.g. GTDB "Desulfobacterota_B") where the uninomial really is at family/order
    // level.
    if ctx.name.phrase.as_ref().is_some_and(|p| !p.is_empty())
        && ctx.name.cultivar_epithet.is_none()
    {
        let r = ctx.name.rank;
        let below_genus = r.is_infrageneric() || r == Rank::Species || r.is_infraspecific();
        if below_genus && ctx.name.uninomial.is_some() && ctx.name.genus.is_none() {
            ctx.name.genus = ctx.name.uninomial.take();
        }
        ctx.name.type_ = NameType::Informal;
    }

    // Step 19: re-wrap a quoted leading monomial ("'Prosthète'") so the output keeps the
    // quotes that mark it as an unavailable name; the quotes were stripped for parsing in
    // StripAndStash.
    if let Some(q) = ctx.quoted_monomial.clone() {
        if let Some(uninomial) = ctx.name.uninomial.take() {
            ctx.name.uninomial = Some(format!("{q}{uninomial}{q}"));
        } else if let Some(genus) = ctx.name.genus.take() {
            ctx.name.genus = Some(format!("{q}{genus}{q}"));
        }
    }
}

/// Java `Assemble.rankFromSuffix(String, NomCode)` (`Assemble.java:243-253`): per-code
/// suffix → rank, first-match-wins over [`rank_utils::suffices_rank_map`]'s ordered
/// (longest/most-specific-first) list — a plain linear scan, mirroring Java's own
/// `for (Map.Entry<...> e : suffixes.entrySet())` loop with no sort (the map is already
/// ordered on insertion).
fn rank_from_suffix(name: &str, code: NomCode) -> Option<Rank> {
    let suffixes = rank_utils::suffices_rank_map(code)?;
    let s = name.to_lowercase();
    suffixes
        .iter()
        .find(|(suffix, _)| s.ends_with(suffix))
        .map(|(_, rank)| *rank)
}

/// Java `Assemble.rankFromGlobalSuffix(String)` (`Assemble.java:255-261`): the "no code
/// known" fallback — only the two globally unambiguous suffixes are safe to use blind.
fn rank_from_global_suffix(name: &str) -> Option<Rank> {
    let s = name.to_lowercase();
    if s.ends_with("aceae") {
        Some(Rank::Family)
    } else if s.ends_with("oideae") {
        Some(Rank::Subfamily)
    } else {
        None
    }
}

/// Plausible authorship years fall in this inclusive range; anything else is flagged. Java
/// `Assemble.MIN_YEAR`/`MAX_YEAR` (`Assemble.java:198-199`).
const MIN_YEAR: i32 = 1500;
const MAX_YEAR: i32 = 2100;

/// Java `Assemble.YEAR_4DIGIT` (`Assemble.java:200-201`), compiled with no flags — Java's
/// `\d` is therefore ASCII-only, ported as `(?-u:\d{4})` (this port's per-pattern flag rule).
/// Anchored on both ends (`^…$`) to reproduce Java `Matcher.matches()` (whole-string match)
/// rather than `Matcher.find()` (substring search) — the `regex` crate's `is_match` behaves
/// like `find()`, so the anchors are added explicitly here to get `matches()`'s semantics
/// (verified by the `is_unlikely_year_rejects_a_5_digit_run` regression test below, which
/// would wrongly pass without them).
static YEAR_4DIGIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?-u:\d{4})$").unwrap());

/// Java `Assemble.isUnlikelyYear(String)` (`Assemble.java:216-221`): a parsed authorship year
/// that isn't a clean 4-digit number in a plausible range is a data-quality artefact
/// ("Wilcox, 137", "Hall, 0000", the "193" truncated from "193k7"). An intentionally
/// uncertain year ("198?") is left alone. `year.is_none()` (Java: `year == null`) returns
/// `false` (not unlikely — there is simply no year to judge).
fn is_unlikely_year(year: Option<&str>) -> bool {
    let Some(year) = year else { return false };
    if year.ends_with('?') {
        return false;
    }
    if !YEAR_4DIGIT.is_match(year) {
        return true;
    }
    let v: i32 = year
        .parse()
        .expect("YEAR_4DIGIT just matched exactly 4 ASCII digits, always parseable as i32");
    // Java: `return v < MIN_YEAR || v > MAX_YEAR;` — same truth table, spelled with the
    // standard range helper per clippy::manual_range_contains.
    !(MIN_YEAR..=MAX_YEAR).contains(&v)
}

/// Java `Assemble.flagUnlikelyYears(ParsedName)` (`Assemble.java:208-214`).
fn flag_unlikely_years(n: &mut ParsedName) {
    if is_unlikely_year(n.combination_authorship.year.as_deref())
        || is_unlikely_year(n.basionym_authorship.year.as_deref())
    {
        n.doubtful = true;
        n.add_warning(warnings::UNLIKELY_YEAR);
    }
}

/// Java `Assemble.flagBlacklistedEpithets(ParsedName)` (`Assemble.java:223-241`). A literal
/// "null" is a data artefact in any name part — uninomial or genus just as much as an
/// epithet ("Null bactus", "Abies null Hood") — flagged doubtful in all cases; a blacklisted
/// epithet (common non-Latin stop words) is checked only on the two epithet fields. Values
/// are cloned up front (rather than borrowed) purely to keep the two mutations below
/// (`n.doubtful =`/`n.add_warning`) free of any overlapping borrow of `n`.
fn flag_blacklisted_epithets(n: &mut ParsedName) {
    let name_parts = [
        n.uninomial.clone(),
        n.genus.clone(),
        n.specific_epithet.clone(),
        n.infraspecific_epithet.clone(),
    ];
    for part in name_parts.into_iter().flatten() {
        if part.eq_ignore_ascii_case("null") {
            n.doubtful = true;
            n.add_warning(warnings::NULL_EPITHET);
        }
    }

    let epithets = [n.specific_epithet.clone(), n.infraspecific_epithet.clone()];
    for ep in epithets.into_iter().flatten() {
        if blacklisted_epithets::contains(&ep) {
            n.doubtful = true;
            n.add_warning(warnings::BLACKLISTED_EPITHET);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_ctx() -> ParseContext {
        ParseContext::new("x".to_string(), None, None, None)
    }

    // ---- Step 1: aggregate ----

    #[test]
    fn aggregate_with_specific_epithet_becomes_species_aggregate() {
        let mut ctx = base_ctx();
        ctx.aggregate = true;
        ctx.name.specific_epithet = Some("alba".to_string());
        finish(&mut ctx, None);
        assert_eq!(ctx.name.rank, Rank::SpeciesAggregate);
    }

    // ---- Steps 2 + 6: load-bearing order between the two rank-forwarding steps ----

    #[test]
    fn requested_species_aggregate_preempts_the_later_rank_mismatch_step() {
        // If step 6 (rank-mismatch demotion) ran using the ORIGINAL rank, it would ALSO
        // fire here (SPECIES_AGGREGATE.higherThan(SPECIES) is true) and flag
        // RANK_MISMATCH/INFORMAL/doubtful. Because step 2 runs first and already sets
        // `n.rank == req`, step 6's own `req != n.getRank()` guard is false by the time it
        // runs — proving the 19-step ORDER (not just each step's own logic) is load-bearing.
        let mut ctx = base_ctx();
        ctx.requested_rank = Some(Rank::SpeciesAggregate);
        ctx.name.rank = Rank::Species;
        ctx.name.specific_epithet = Some("alba".to_string());
        finish(&mut ctx, None);
        assert_eq!(ctx.name.rank, Rank::SpeciesAggregate);
        assert_eq!(
            ctx.name.type_,
            NameType::Scientific,
            "step 6 must NOT also have fired"
        );
        assert!(!ctx.name.doubtful);
        assert!(ctx.name.warnings.is_empty());
    }

    // ---- Step 4: INDETERMINED placeholder nulls both authorships ----

    #[test]
    fn indetermined_placeholder_promotes_uninomial_and_nulls_authorship() {
        let mut ctx = base_ctx();
        ctx.requested_rank = Some(Rank::Species);
        ctx.name.uninomial = Some("Lepidoptera".to_string());
        ctx.name.combination_authorship = Authorship {
            authors: vec!["Hooker".to_string()],
            ..Default::default()
        };
        ctx.name.basionym_authorship = Authorship {
            authors: vec!["Someone".to_string()],
            ..Default::default()
        };
        finish(&mut ctx, None);
        assert_eq!(ctx.name.genus, Some("Lepidoptera".to_string()));
        assert_eq!(ctx.name.uninomial, None);
        assert_eq!(ctx.name.rank, Rank::Species);
        assert_eq!(ctx.name.type_, NameType::Informal);
        assert!(ctx
            .name
            .warnings
            .contains(&warnings::INDETERMINED.to_string()));
        assert_eq!(ctx.name.combination_authorship, Authorship::default());
        assert_eq!(ctx.name.basionym_authorship, Authorship::default());
    }

    #[test]
    fn indetermined_placeholder_does_not_fire_when_specific_epithet_already_present() {
        let mut ctx = base_ctx();
        ctx.requested_rank = Some(Rank::Species);
        ctx.name.uninomial = Some("Polygonum".to_string());
        ctx.name.specific_epithet = Some("alba".to_string());
        finish(&mut ctx, None);
        assert_eq!(ctx.name.uninomial, Some("Polygonum".to_string()));
        assert!(!ctx
            .name
            .warnings
            .contains(&warnings::INDETERMINED.to_string()));
    }

    // ---- Step 5: infrageneric-strict uninomial -> infragenericEpithet ----

    #[test]
    fn infrageneric_strict_uninomial_moves_to_infrageneric_epithet() {
        let mut ctx = base_ctx();
        ctx.requested_rank = Some(Rank::Subgenus);
        ctx.name.uninomial = Some("Trematostoma".to_string());
        finish(&mut ctx, None);
        assert_eq!(
            ctx.name.infrageneric_epithet,
            Some("Trematostoma".to_string())
        );
        assert_eq!(ctx.name.uninomial, None);
        assert_eq!(ctx.name.rank, Rank::Subgenus);
    }

    // ---- Step 6: rank-mismatch demotion ----

    #[test]
    fn binomial_with_higher_caller_rank_is_demoted_and_flagged() {
        let mut ctx = base_ctx();
        ctx.requested_rank = Some(Rank::Genus);
        ctx.name.genus = Some("Polygonum".to_string());
        ctx.name.specific_epithet = Some("alba".to_string());
        ctx.name.rank = Rank::Species;
        finish(&mut ctx, None);
        assert_eq!(ctx.name.rank, Rank::Genus);
        assert_eq!(ctx.name.type_, NameType::Informal);
        assert!(ctx.name.doubtful);
        assert!(ctx
            .name
            .warnings
            .contains(&warnings::RANK_MISMATCH.to_string()));
    }

    // ---- Step 7: underscore-split ----

    #[test]
    fn underscore_split_lowercase_after_becomes_genus_species() {
        let mut ctx = base_ctx();
        ctx.name.uninomial = Some("Oxalis_barrelieri".to_string());
        finish(&mut ctx, None);
        assert_eq!(ctx.name.uninomial, None);
        assert_eq!(ctx.name.genus, Some("Oxalis".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("barrelieri".to_string()));
        assert_eq!(ctx.name.rank, Rank::Species);
    }

    #[test]
    fn underscore_split_uppercase_after_becomes_gtdb_phrase_name() {
        let mut ctx = base_ctx();
        ctx.name.uninomial = Some("Desulfobacterota_B".to_string());
        finish(&mut ctx, None);
        assert_eq!(ctx.name.uninomial, Some("Desulfobacterota".to_string()));
        assert_eq!(ctx.name.phrase, Some("B".to_string()));
        assert_eq!(ctx.name.type_, NameType::Informal);
        assert_eq!(ctx.name.rank, Rank::Unranked);
        assert_eq!(
            ctx.name.genus, None,
            "step 18 must not promote a suprageneric GTDB phrase name to genus"
        );
    }

    #[test]
    fn underscore_split_is_skipped_for_a_non_scientific_type() {
        let mut ctx = base_ctx();
        ctx.name.uninomial = Some("Oxalis_barrelieri".to_string());
        ctx.name.type_ = NameType::Informal;
        finish(&mut ctx, None);
        assert_eq!(ctx.name.uninomial, Some("Oxalis_barrelieri".to_string()));
    }

    // ---- Step 8: pendingUnparsed -> PARTIAL ----

    #[test]
    fn pending_unparsed_demotes_to_partial_state() {
        let mut ctx = base_ctx();
        ctx.pending_unparsed = Some("extra junk".to_string());
        finish(&mut ctx, None);
        assert_eq!(ctx.name.state, State::Partial);
        assert_eq!(ctx.name.unparsed, Some("extra junk".to_string()));
    }

    // ---- Step 9: yearRange -> YEAR_INTERPRETED ----

    #[test]
    fn year_range_authorship_flags_year_interpreted() {
        let mut ctx = base_ctx();
        let auth = AuthState {
            year_range: true,
            ..AuthState::default()
        };
        finish(&mut ctx, Some(&auth));
        assert!(ctx
            .name
            .warnings
            .contains(&warnings::YEAR_INTERPRETED.to_string()));
    }

    // ---- Steps 10-12: code inference / viral / rank-code-mismatch wiring ----

    #[test]
    fn candidatus_name_gets_bacterial_code_via_code_inference_wiring() {
        let mut ctx = base_ctx();
        ctx.name.candidatus = true;
        finish(&mut ctx, None);
        assert_eq!(ctx.name.code, Some(NomCode::Bacterial));
    }

    #[test]
    fn viral_shape_with_no_other_signal_defaults_to_virus_code() {
        let mut ctx = base_ctx();
        ctx.viral_shape = true;
        finish(&mut ctx, None);
        assert_eq!(ctx.name.code, Some(NomCode::Virus));
    }

    #[test]
    fn rank_code_mismatch_wired_through_finish_overrides_and_warns() {
        let mut ctx = base_ctx();
        ctx.name.rank = Rank::SupersectionBotany; // BOTANICAL-restricted
        ctx.name.code = Some(NomCode::Zoological); // pre-set: step 10 must skip
        ctx.requested_code = Some(NomCode::Zoological);
        finish(&mut ctx, None);
        assert_eq!(ctx.name.code, Some(NomCode::Botanical));
        assert!(ctx
            .name
            .warnings
            .contains(&warnings::CODE_MISMATCH.to_string()));
    }

    // ---- Step 13: INFRASPECIFIC_NAME + ZOOLOGICAL -> SUBSPECIES ----

    #[test]
    fn infraspecific_name_with_zoological_code_becomes_subspecies() {
        let mut ctx = base_ctx();
        ctx.name.genus = Some("Vulpes".to_string());
        ctx.name.specific_epithet = Some("vulpes".to_string());
        ctx.name.infraspecific_epithet = Some("silaceus".to_string());
        ctx.name.rank = Rank::InfraspecificName;
        ctx.name.code = Some(NomCode::Zoological);
        finish(&mut ctx, None);
        assert_eq!(ctx.name.rank, Rank::Subspecies);
    }

    #[test]
    fn infraspecific_name_without_zoological_code_stays_unchanged() {
        let mut ctx = base_ctx();
        ctx.name.infraspecific_epithet = Some("silaceus".to_string());
        ctx.name.rank = Rank::InfraspecificName;
        ctx.name.code = Some(NomCode::Botanical);
        finish(&mut ctx, None);
        assert_eq!(ctx.name.rank, Rank::InfraspecificName);
    }

    // ---- Step 14: suffix-based rank inference ----

    #[test]
    fn suffix_based_rank_via_requested_code() {
        let mut ctx = base_ctx();
        ctx.requested_code = Some(NomCode::Bacterial);
        ctx.name.uninomial = Some("Enterobacteriaceae".to_string());
        finish(&mut ctx, None);
        assert_eq!(ctx.name.rank, Rank::Family);
    }

    #[test]
    fn suffix_based_rank_global_fallback_when_no_code_known() {
        let mut ctx = base_ctx();
        ctx.name.uninomial = Some("Rosaceae".to_string());
        finish(&mut ctx, None);
        assert_eq!(ctx.name.rank, Rank::Family);
    }

    // ---- Step 15: blacklisted epithet ----

    #[test]
    fn blacklisted_epithet_flags_doubtful() {
        let mut ctx = base_ctx();
        ctx.name.genus = Some("Abies".to_string());
        ctx.name.specific_epithet = Some("about".to_string()); // a real blacklist entry
        finish(&mut ctx, None);
        assert!(ctx.name.doubtful);
        assert!(ctx
            .name
            .warnings
            .contains(&warnings::BLACKLISTED_EPITHET.to_string()));
    }

    #[test]
    fn literal_null_epithet_flags_doubtful() {
        let mut ctx = base_ctx();
        ctx.name.genus = Some("Null".to_string());
        finish(&mut ctx, None);
        assert!(ctx.name.doubtful);
        assert!(ctx
            .name
            .warnings
            .contains(&warnings::NULL_EPITHET.to_string()));
    }

    // ---- Step 16: unlikely year ----

    #[test]
    fn unlikely_year_flags_doubtful() {
        let mut ctx = base_ctx();
        ctx.name.combination_authorship = Authorship {
            authors: vec!["Wilcox".to_string()],
            year: Some("137".to_string()),
            ..Default::default()
        };
        finish(&mut ctx, None);
        assert!(ctx.name.doubtful);
        assert!(ctx
            .name
            .warnings
            .contains(&warnings::UNLIKELY_YEAR.to_string()));
    }

    #[test]
    fn plausible_year_does_not_flag_unlikely_year() {
        let mut ctx = base_ctx();
        ctx.name.combination_authorship = Authorship {
            authors: vec!["Mill.".to_string()],
            year: Some("1987".to_string()),
            ..Default::default()
        };
        finish(&mut ctx, None);
        assert!(!ctx
            .name
            .warnings
            .contains(&warnings::UNLIKELY_YEAR.to_string()));
    }

    // ---- Step 17: cultivar clears INFORMAL/INDETERMINED ----

    #[test]
    fn cultivar_epithet_clears_informal_type_and_indetermined_warning() {
        let mut ctx = base_ctx();
        ctx.name.cultivar_epithet = Some("mother of pearl".to_string());
        ctx.name.type_ = NameType::Informal;
        ctx.name.add_warning(warnings::INDETERMINED);
        finish(&mut ctx, None);
        assert_eq!(ctx.name.type_, NameType::Scientific);
        assert!(!ctx
            .name
            .warnings
            .contains(&warnings::INDETERMINED.to_string()));
    }

    // ---- Step 18: phrase promotion ----

    #[test]
    fn phrase_without_cultivar_promotes_uninomial_to_genus_below_genus_rank() {
        let mut ctx = base_ctx();
        ctx.name.rank = Rank::Species;
        ctx.name.uninomial = Some("Baeckea".to_string());
        ctx.name.phrase = Some("sp1".to_string());
        finish(&mut ctx, None);
        assert_eq!(ctx.name.genus, Some("Baeckea".to_string()));
        assert_eq!(ctx.name.uninomial, None);
        assert_eq!(ctx.name.type_, NameType::Informal);
    }

    // ---- Step 19: quoted-monomial re-wrap ----

    #[test]
    fn quoted_monomial_is_rewrapped_around_the_uninomial() {
        let mut ctx = base_ctx();
        ctx.quoted_monomial = Some("'".to_string());
        ctx.name.uninomial = Some("Prosthete".to_string());
        finish(&mut ctx, None);
        assert_eq!(ctx.name.uninomial, Some("'Prosthete'".to_string()));
    }

    #[test]
    fn quoted_monomial_is_rewrapped_around_the_genus_when_no_uninomial() {
        let mut ctx = base_ctx();
        ctx.quoted_monomial = Some("\"".to_string());
        ctx.name.genus = Some("Prosthete".to_string());
        ctx.name.specific_epithet = Some("alba".to_string());
        finish(&mut ctx, None);
        assert_eq!(ctx.name.genus, Some("\"Prosthete\"".to_string()));
    }

    // ---- direct helper-function tests ----

    #[test]
    fn is_unlikely_year_boundaries() {
        assert!(!is_unlikely_year(Some("1500")));
        assert!(!is_unlikely_year(Some("2100")));
        assert!(is_unlikely_year(Some("1499")));
        assert!(is_unlikely_year(Some("2101")));
        assert!(is_unlikely_year(Some("137")));
        assert!(is_unlikely_year(Some("0000")));
        assert!(!is_unlikely_year(Some("198?")));
        assert!(!is_unlikely_year(None));
    }

    #[test]
    fn is_unlikely_year_rejects_a_5_digit_run() {
        // Regression guard for the `^…$` anchoring: without it, `\d{4}` would still find a
        // 4-digit substring inside a 5-digit run and wrongly call this "likely".
        assert!(is_unlikely_year(Some("12345")));
    }

    #[test]
    fn rank_from_suffix_direct() {
        assert_eq!(
            rank_from_suffix("Enterobacteriaceae", NomCode::Bacterial),
            Some(Rank::Family)
        );
        assert_eq!(rank_from_suffix("Xyz", NomCode::Bacterial), None);
        assert_eq!(rank_from_suffix("Xyz", NomCode::Cultivars), None);
    }

    #[test]
    fn rank_from_global_suffix_direct() {
        assert_eq!(rank_from_global_suffix("Rosaceae"), Some(Rank::Family));
        assert_eq!(
            rank_from_global_suffix("Bacilloideae"),
            Some(Rank::Subfamily)
        );
        assert_eq!(rank_from_global_suffix("Xyz"), None);
    }

    #[test]
    fn flag_blacklisted_epithets_flags_literal_null_on_a_non_epithet_field() {
        let mut n = ParsedName {
            genus: Some("Null".to_string()),
            ..Default::default()
        };
        flag_blacklisted_epithets(&mut n);
        assert!(n.doubtful);
        assert!(n.warnings.contains(&warnings::NULL_EPITHET.to_string()));
    }
}
