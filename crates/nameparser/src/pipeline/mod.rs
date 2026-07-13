// SPDX-License-Identifier: Apache-2.0
//! Java `org.gbif.nameparser.pipeline.Pipeline` — orchestrates the staged parsing
//! pipeline. Each stage mutates the shared [`ParseContext`].

pub(crate) mod assemble;
pub(crate) mod authorship_parser;
pub(crate) mod authorship_split;
pub(crate) mod blacklisted_epithets;
pub(crate) mod code_inference;
pub(crate) mod context;
pub(crate) mod name_tokens;
pub(crate) mod preflight;
pub(crate) mod rank_markers;
pub(crate) mod stripandstash;

pub(crate) use context::ParseContext;

use std::sync::LazyLock;

use regex::Regex;

use crate::model::{
    warnings, CombinedAuthorship, NameType, NomCode, ParseError, ParsedName, Rank, State,
};
use crate::pipeline::authorship_parser::AuthState;
use crate::token::tokenize;
use crate::unicode::{java_trim, normalize_quotes};

/// Java `Pipeline.MAX_LENGTH`. Hard upper bound on the input length. Beyond this the
/// input is rejected as unparsable rather than parsed: real scientific names — even with
/// very large authorships — stay well under this (the longest known valid name is ~860
/// chars), and the regex-heavy pipeline has no execution timeout, so an unbounded input
/// is a denial-of-service risk (deep regex recursion can overflow the stack on the
/// caller's thread).
const MAX_LENGTH: usize = 1000;

/// Java `Pipeline.LONG_NAME_LENGTH`. Inputs longer than this still parse but carry a
/// [`warnings::LONG_NAME`] flag so callers can spot the unusual (but legitimate)
/// very-long names.
const LONG_NAME_LENGTH: usize = 250;

/// Java `Pipeline.GLUED_PHRASE`, compiled with `Pattern.UNICODE_CHARACTER_CLASS` — kept
/// as the `regex` crate's default Unicode-aware shorthand classes (per-pattern flag
/// rule: Unicode-flagged Java patterns keep Rust's default Unicode classes, not
/// `(?-u:…)` ASCII-scoped). `\p{Lu}`/`\p{Ll}` are already Unicode in both engines
/// regardless of that flag.
///
/// Pattern: Latin-style prefix glued to an all-caps / alphanumeric phrase suffix
/// ("OdontellidaeGEN", "GenusANIC_3"). Underscored prefixes ("Blattellinae_SB") are
/// handled later in Assemble; this doesn't match those.
static GLUED_PHRASE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([\p{Lu}][\p{Ll}]+)([\p{Lu}]{2,}[\p{Lu}\d_]*)$").unwrap());

/// Java `Pipeline.run`. Orchestrates the staged parsing pipeline: guards → normalize →
/// build [`ParseContext`] → split-glued-phrase → Preflight → StripAndStash → Tokenizer →
/// AuthorshipSplit → NameTokens → AuthorshipParser (embedded / autonym mid-author /
/// separately-supplied) → CodeInference (via `Assemble::finish`) → Assemble → pending
/// year/imprint-year/specific-author/generic-author application. Every stage is now
/// wired in (Phase 1 Slice 4 Task 4).
pub fn run(
    name: &str,
    authorship: Option<&str>,
    rank: Option<Rank>,
    code: Option<NomCode>,
) -> Result<ParsedName, ParseError> {
    // Java also null-checks `scientificName` here (`throw new
    // UnparsableNameException(NameType.OTHER, null)`); unreachable in Rust since `&str`
    // can never be null — only the empty-after-trim case below can actually occur.
    let trimmed = java_trim(name);
    if trimmed.is_empty() {
        return Err(ParseError::new(NameType::Other, None, name));
    }
    // Java measures length in UTF-16 code units. `str::len()` counts UTF-8 bytes (which
    // would over-count non-ASCII input relative to Java) so it's not used here;
    // `.chars().count()` (Unicode scalar count) is the closest faithful proxy available
    // without a UTF-16 dependency, and matches Java exactly outside the astral planes
    // (where it undercounts relative to Java's 2-code-units-per-codepoint, making Rust
    // marginally more permissive there, never a source of false rejections). The corpus
    // has no name anywhere near this bound, so the choice isn't exercised by the gate.
    if trimmed.chars().count() > MAX_LENGTH {
        return Err(ParseError::new(NameType::Other, None, name));
    }
    // The separately supplied authorship is tokenised and run through the same
    // regex-heavy authorship parser, so it carries the same DoS exposure — cap it too.
    // NB matches Java: the thrown error's `name` field is the scientific `name`, not the
    // (overlong) authorship string — Java's guard throws
    // `new UnparsableNameException(NameType.OTHER, scientificName)` here too.
    if let Some(a) = authorship {
        if java_trim(a).chars().count() > MAX_LENGTH {
            return Err(ParseError::new(NameType::Other, None, name));
        }
    }

    // Normalise the many unicode apostrophe / quote variants to ASCII (' and ") up front
    // so every parsed field (genus, epithets, authorship, unparsed) and both the name
    // and the separately supplied authorship come out with consistent ASCII
    // punctuation. `name` itself is kept raw/untouched (not even trimmed) for faithful
    // echo into `preflight::run` below, matching Java's `Preflight.run(scientificName,
    // ctx)` — that call passes `Pipeline.run`'s own original parameter, not the
    // trimmed+normalized local.
    let trimmed = normalize_quotes(trimmed);
    let authorship = authorship.map(normalize_quotes);

    let mut ctx = ParseContext::new(trimmed.clone(), authorship, rank, code);
    if trimmed.chars().count() > LONG_NAME_LENGTH {
        ctx.name.add_warning(warnings::LONG_NAME);
    }
    split_glued_phrase_name(&mut ctx);

    preflight::run(name, &mut ctx)?;

    // 5.0.0: Preflight may RESCUE an anchored informal grouping ("Bartonella group",
    // "Vermistella-lineage") into a complete Informal-shaped ParsedName instead of erroring — return
    // it as-is, skipping the tokenizer/classifier/assembler (there is nothing left to parse).
    if ctx.preflight_complete {
        return Ok(ctx.name);
    }

    // Java `Pipeline.run`: `StripAndStash.run(ctx); if (!hasLetter(ctx.working)) throw new
    // UnparsableNameException(NameType.OTHER, scientificName);` (Pipeline.java:70-73) — a
    // 4th inline guard, distinct from Preflight and from the 3 guards at the top of this
    // function, sitting between StripAndStash and the Tokenizer.
    stripandstash::run(&mut ctx);

    // The guard rejects any input left with no letters after Preflight + StripAndStash,
    // matching Java `Pipeline.java:70-73`. (Originally found via the Task 6 golden corpus:
    // `-,.#` — Java `Err(OTHER)` — before this guard existed at all.)
    if !has_letter(&ctx.working) {
        return Err(ParseError::new(NameType::Other, None, name));
    }

    // Java `Pipeline.run`: `ctx.tokens = Tokenizer.tokenize(ctx.working); int boundary =
    // AuthorshipSplit.findBoundary(ctx.tokens, ctx); NameTokens.classify(ctx, boundary);`
    // (`Pipeline.java:73-77`). `find_boundary` is a pure function of `ctx.tokens` +
    // `ctx.requested_rank` (no side effects); `classify` is the one that mutates `ctx.name`
    // + `ctx.mid_author_from`/`ctx.mid_author_to` + `ctx.aggregate` with the name-part
    // fields (Phase 1 Slice 3 Tasks 2-3).
    ctx.tokens = tokenize(&ctx.working);
    let boundary = authorship_split::find_boundary(&ctx.tokens, &ctx);
    name_tokens::classify(&mut ctx, boundary);

    // Java `Pipeline.run`, `Pipeline.java:79-185` (the AuthorshipParser → Assemble back
    // end). Each of the three embedded/mid-author/aux authorship spans is parsed
    // independently into its own `AuthState`, applied onto `ctx.name` as it's produced,
    // and also kept around (as `Option<&AuthState>`) so the `codeState` fallback logic
    // below can pick whichever one actually carries a code signal.

    // Embedded trailing authorship: whatever AuthorshipSplit left after the name-part
    // tokens. `authState.unparsedFrom >= 0` records a remainder AuthorshipParser itself
    // couldn't place (Phase A's `hasUpperWord` guard on a malformed leading "(...)") —
    // specific to this path, since the separately-supplied authorship has no leftover
    // name material of its own to park.
    let mut auth_state: Option<AuthState> = None;
    if boundary < ctx.tokens.len() {
        let st = authorship_parser::parse(&ctx.tokens, boundary);
        apply_authorship(&mut ctx.name, &st);
        if st.unparsed_from >= 0 {
            ctx.name.state = State::Partial;
            ctx.name.unparsed = st.unparsed_text.clone();
        }
        auth_state = Some(st);
    }

    // Autonym species author: a "(Bas) Comb" or plain author span recorded mid-name by
    // NameTokens, sitting between the species epithet and the infraspecific marker. The
    // autonym's final epithet carries no author of its own (ICN Art. 22.1/26.1), so this
    // span IS the species author and becomes the name's authorship. Only applied when the
    // name is an autonym and no trailing authorship was already parsed.
    let mut autonym_state: Option<AuthState> = None;
    if ctx.mid_author_from >= 0 && ctx.name.is_autonym() && !ctx.name.has_authorship() {
        let from = ctx.mid_author_from as usize;
        let to = ctx.mid_author_to as usize;
        let st = authorship_parser::parse(&ctx.tokens[from..to], 0);
        apply_authorship(&mut ctx.name, &st);
        autonym_state = Some(st);
    }

    // Separately-supplied authorship: run the same annotation strippers (sic / corrig /
    // extinct dagger / brackets etc.) on the auxiliary string via `strip_authorship_markers`
    // so its tokens are clean before parsing, then re-tokenise and parse it independently.
    // A sanctioning author found here is applied immediately (the embedded path's own
    // sanctioning author, applied further below, overwrites it — last-write-wins).
    let mut extra_state: Option<AuthState> = None;
    if let Some(authorship) = ctx.authorship_input.clone() {
        if !authorship.chars().all(crate::token::is_whitespace_java) {
            let auth_clean = stripandstash::strip_authorship_markers(&authorship, &mut ctx.name);
            let aux = tokenize(&auth_clean);
            let st = authorship_parser::parse(&aux, 0);
            apply_authorship(&mut ctx.name, &st);
            if st.sanctioning_author.is_some() {
                ctx.name.sanctioning_author = st.sanctioning_author.clone();
            }
            extra_state = Some(st);
        }
    }
    if let Some(st) = auth_state.as_ref() {
        if st.sanctioning_author.is_some() {
            ctx.name.sanctioning_author = st.sanctioning_author.clone();
        }
    }

    // Code inference uses the main scientific name's authState by default. When the main
    // name had no authorship of its own, fall back to the auxiliary authorship state when
    // it carries a basionym citation (parens) that tips the code, or (failing that) to the
    // autonym's species-author state — see `code_state_needs_fallback`'s doc comment.
    let mut code_state: Option<&AuthState> = auth_state.as_ref();
    if code_state_needs_fallback(code_state)
        && extra_state.as_ref().is_some_and(|st| {
            st.basionym_present && (st.basionym.year.is_some() || st.combination.exists())
        })
    {
        code_state = extra_state.as_ref();
    }
    if code_state_needs_fallback(code_state) && autonym_state.is_some() {
        code_state = autonym_state.as_ref();
    }

    // Year that came directly off the author span (e.g. "Linnaeus, 1771") is applied
    // BEFORE code inference because it IS the zoological author-year citation we want to
    // detect. A year extracted from a stripped publishedIn reference is just the
    // publication year — code-neutral — so it's applied AFTER inference instead (below),
    // so the same year on a botanical or bacterial name doesn't get misread as a
    // zoological author-year.
    if ctx.pending_year.is_some()
        && !ctx.pending_year_from_publication
        && ctx.name.combination_authorship.exists()
        && ctx.name.combination_authorship.year.is_none()
    {
        ctx.name.combination_authorship.year = ctx.pending_year.clone();
    }

    assemble::finish(&mut ctx, code_state);

    if ctx.pending_year.is_some()
        && ctx.pending_year_from_publication
        && ctx.name.combination_authorship.exists()
        && ctx.name.combination_authorship.year.is_none()
    {
        ctx.name.combination_authorship.year = ctx.pending_year.clone();
    }

    // An imprint year stripped before authorship parsing belongs to the name's combination
    // authorship (sitting next to its publication year).
    if ctx.pending_imprint_year.is_some() && ctx.name.combination_authorship.imprint_year.is_none()
    {
        ctx.name.combination_authorship.imprint_year = ctx.pending_imprint_year.clone();
    }

    // Irregular authorships split off during stripping: the species author of a
    // below-species name ("…L. cv. 'Elsrijk' Broerse") and the genus author of an
    // infrageneric name ("Cordia (Adans.) Kuntze sect. …"). Parse and attach to their
    // dedicated slots.
    if let Some(specific_author) = ctx.pending_specific_author.clone() {
        let already = ctx
            .name
            .specific_authorship
            .as_ref()
            .is_some_and(CombinedAuthorship::has_authorship);
        if !already {
            if let Some(ca) = parse_combined_authorship(&specific_author) {
                ctx.name.specific_authorship = Some(ca);
            }
        }
    }
    if let Some(generic_author) = ctx.pending_generic_author.clone() {
        let already = ctx
            .name
            .generic_authorship
            .as_ref()
            .is_some_and(CombinedAuthorship::has_authorship);
        if !already {
            if let Some(ca) = parse_combined_authorship(&generic_author) {
                ctx.name.generic_authorship = Some(ca);
            }
        }
    }

    Ok(ctx.name)
}

/// Java `Pipeline.applyAuthorship(ParsedName, AuthorshipParser.AuthState)`. Applies the
/// combination + basionym authorship from a parsed [`AuthState`] onto the name. Shared by
/// the embedded-authorship, autonym-mid-author and separately-supplied-authorship paths.
/// The sanctioning author and the unparsed remainder are applied by the callers, since
/// those differ between the paths. (Imprint years travel on the `Authorship` objects
/// themselves, so setting the authorship above already carries them along — nothing extra
/// to apply here, matching Java's own comment at this exact spot.)
fn apply_authorship(name: &mut ParsedName, st: &AuthState) {
    if st.combination.exists() {
        name.combination_authorship = st.combination.clone();
    }
    if st.basionym.exists() {
        name.basionym_authorship = st.basionym.clone();
    }
}

/// Java's repeated `codeState == null || (!codeState.combination.exists() &&
/// !codeState.basionymPresent)` guard (`Pipeline.java:132, 141`): true when `state` carries
/// no code-relevant signal yet (no authorship state picked at all, or one that has neither
/// a combination authorship nor a basionym) — i.e. the `codeState` fallback chain should
/// keep looking at the next candidate.
fn code_state_needs_fallback(state: Option<&AuthState>) -> bool {
    match state {
        None => true,
        Some(s) => !s.combination.exists() && !s.basionym_present,
    }
}

/// Java `Pipeline.parseCombinedAuthorship(String)`. Parses a bare author string (e.g. "L."
/// or "(Adans.) Kuntze") into a [`CombinedAuthorship`] for the generic/specific authorship
/// slots. Returns `None` when nothing parsable was found.
fn parse_combined_authorship(authors: &str) -> Option<CombinedAuthorship> {
    let tokens = tokenize(authors);
    let st = authorship_parser::parse(&tokens, 0);
    let mut ca = CombinedAuthorship::default();
    if st.combination.exists() {
        ca.combination_authorship = st.combination;
    }
    if st.basionym.exists() {
        ca.basionym_authorship = st.basionym;
    }
    ca.has_authorship().then_some(ca)
}

/// Java `Pipeline.hasLetter(String)`: `Character.isLetter` scanned per Unicode code point.
/// Same `is_alphabetic` approximation used throughout this port for `Character.isLetter`
/// (see `token.rs::is_letter`, `preflight.rs::count_letters`) — slightly broader than
/// Java's L*-only category; divergences would be surfaced by the golden corpus diff.
fn has_letter(s: &str) -> bool {
    s.chars().any(|c| c.is_alphabetic())
}

/// Java `Pipeline.splitGluedPhraseName`. BOLD/specimen-style phrase names with no
/// whitespace between the Latin prefix and the phrase suffix ("OdontellidaeGEN",
/// "GenusANIC_3"). Splits the working string so Preflight doesn't reject the
/// alphanumeric form and the rest of the pipeline can treat the prefix as a normal
/// uninomial.
fn split_glued_phrase_name(ctx: &mut ParseContext) {
    // Java: `if (ctx.working == null || ctx.working.indexOf(' ') >= 0) return;` — the
    // null check is unreachable here (`ctx.working` is a non-nullable `String`, never
    // empty at this call site since the empty-after-trim guard above already rejected
    // that). `.contains(' ')` matches Java's `indexOf(' ')`: both look for the literal
    // ASCII space character only, not general whitespace.
    if ctx.working.contains(' ') {
        return;
    }
    let Some(caps) = GLUED_PHRASE.captures(&ctx.working) else {
        return;
    };
    let prefix = caps.get(1).unwrap().as_str().to_string();
    let suffix = caps.get(2).unwrap().as_str().to_string();
    ctx.name.phrase = Some(suffix);
    ctx.name.type_ = NameType::Informal;
    ctx.working = prefix;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_is_rejected_as_other() {
        let err = run("", None, None, None).unwrap_err();
        assert_eq!(err.type_, NameType::Other);
        assert_eq!(err.code, None);
        assert_eq!(err.name, "");
    }

    #[test]
    fn whitespace_only_input_is_rejected_as_other() {
        let err = run("   ", None, None, None).unwrap_err();
        assert_eq!(err.type_, NameType::Other);
    }

    #[test]
    fn name_over_max_length_is_rejected_as_other() {
        let long = "a".repeat(1001);
        let err = run(&long, None, None, None).unwrap_err();
        assert_eq!(err.type_, NameType::Other);
        assert_eq!(err.code, None);
        // Java's exception name field echoes the raw scientificName, not a trimmed copy.
        assert_eq!(err.name, long);
    }

    #[test]
    fn name_at_max_length_is_not_rejected_by_the_length_guard() {
        // Exactly MAX_LENGTH (1000) chars must pass the length guard (only `> 1000`
        // rejects); Preflight has no length check of its own, so a bare run of letters
        // like this passes it too.
        let at_limit = "a".repeat(1000);
        assert!(run(&at_limit, None, None, None).is_ok());
    }

    #[test]
    fn authorship_over_max_length_is_rejected_as_other_with_name_field() {
        let long_authorship = "a".repeat(1001);
        let err = run("Abies alba", Some(&long_authorship), None, None).unwrap_err();
        assert_eq!(err.type_, NameType::Other);
        // Matches Java: the error's `name` field is the *scientific name*, not the
        // overlong authorship string.
        assert_eq!(err.name, "Abies alba");
    }

    #[test]
    fn normal_binomial_returns_ok_with_a_seeded_name() {
        let pn = run("Abies alba", None, None, None).expect("should parse");
        // Tokenizer + AuthorshipSplit + NameTokens classify a clean binomial into its
        // name-part fields; this input carries no authorship, so the authorship fields
        // (AuthorshipParser/Assemble) stay at their ParseContext-seeded defaults here.
        assert_eq!(pn.genus, Some("Abies".to_string()));
        assert_eq!(pn.specific_epithet, Some("alba".to_string()));
        assert_eq!(pn.rank, Rank::Species);
        assert_eq!(pn.code, None);
        assert_eq!(pn.type_, NameType::Scientific);
        assert_eq!(pn.state, crate::model::State::Complete);
        assert!(pn.warnings.is_empty());
    }

    #[test]
    fn requested_rank_and_code_are_seeded_onto_the_returned_name() {
        let pn = run(
            "Abies alba",
            None,
            Some(Rank::Species),
            Some(NomCode::Botanical),
        )
        .expect("should parse");
        assert_eq!(pn.rank, Rank::Species);
        assert_eq!(pn.code, Some(NomCode::Botanical));
    }

    #[test]
    fn long_name_over_250_chars_gets_the_long_name_warning() {
        // "Abies " (6 chars) * 42 = 252 chars, still all-letters/space so Preflight and
        // the length guard (1000) both let it through.
        let long = "Abies ".repeat(42);
        let pn = run(long.trim(), None, None, None).expect("should parse");
        assert!(pn.warnings.contains(&warnings::LONG_NAME.to_string()));
    }

    #[test]
    fn short_name_does_not_get_the_long_name_warning() {
        let pn = run("Abies alba", None, None, None).expect("should parse");
        assert!(!pn.warnings.contains(&warnings::LONG_NAME.to_string()));
    }

    #[test]
    fn quotes_are_normalized_before_preflight_and_storage() {
        // Load-bearing regression check: normalize_quotes must run *before* Preflight, not
        // just before storage. "Ceylonesmus vector Cham\u{2019}s, 1941" (curly apostrophe,
        // U+2019) parses Ok only because normalize_quotes folds it to ASCII "'" first, which
        // lets ZOOLOGICAL_BINOMIAL match the author block "Cham's, 1941" and rescue the
        // VIRUS-triggering epithet "vector" (mirrors preflight's own
        // zoological_binomial_with_author_year_overrides_stray_viral_token test, same input
        // with a plain-ASCII surname). Left un-normalized, the curly apostrophe breaks that
        // regex match and Preflight rejects the input as OTHER+Virus instead — so, unlike
        // the old `type_ == Scientific` assertion (true even if normalization were silently
        // dropped, since that's just ParsedName::default()'s seed value), this assertion
        // actually fails the moment normalize_quotes stops running ahead of Preflight.
        assert!(run("Ceylonesmus vector Cham\u{2019}s, 1941", None, None, None).is_ok());
    }

    #[test]
    fn glued_phrase_name_is_split_into_prefix_and_phrase() {
        let pn = run("OdontellidaeGEN", None, None, None).expect("should parse");
        assert_eq!(pn.phrase, Some("GEN".to_string()));
        assert_eq!(pn.type_, NameType::Informal);
    }

    #[test]
    fn glued_phrase_pattern_does_not_fire_when_working_has_a_space() {
        let pn = run("Odontellidae GEN", None, None, None).expect("should parse");
        assert_eq!(pn.phrase, None);
        assert_eq!(pn.type_, NameType::Scientific);
    }

    #[test]
    fn input_with_no_letters_at_all_is_rejected_as_other() {
        // Task 6 golden-corpus find (line 5048 of the benchmark data): none of Preflight's
        // 33 patterns fire on pure punctuation, but Java's `Pipeline.run` rejects it via the
        // separate `hasLetter` guard that sits after Preflight (Pipeline.java:71-73).
        let err = run("-,.#", None, None, None).unwrap_err();
        assert_eq!(err.type_, NameType::Other);
        assert_eq!(err.code, None);
        assert_eq!(err.name, "-,.#");
    }

    #[test]
    fn single_letter_abbreviation_survives_the_has_letter_guard() {
        // "B." has exactly one letter, so `has_letter` must let it through — regression
        // guard against an over-eager rewrite of this check.
        assert!(run("B.", None, None, None).is_ok());
    }

    // =======================================================================================
    // Back-end wiring (Phase 1 Slice 4 Task 4): AuthorshipParser -> Assemble. The golden
    // corpus harness (`tests/parse_golden.rs`) always calls `parse(input, None, None, None)`
    // — it validates the EMBEDDED-authorship, autonym-mid-author, and pendingSpecific/
    // GenericAuthor paths at full corpus scale (all four are driven by the scientific-name
    // string alone), but never exercises the SEPARATELY-SUPPLIED-`authorship`-argument path
    // (`extra_state`/`stripandstash::strip_authorship_markers`) at all, since that parameter
    // is always `None` there. The tests below close that gap directly.
    // =======================================================================================

    #[test]
    fn embedded_authorship_is_applied_to_combination_authorship() {
        let pn = run("Abies alba Mill.", None, None, None).expect("should parse");
        assert_eq!(pn.combination_authorship.authors, vec!["Mill.".to_string()]);
    }

    #[test]
    fn separately_supplied_authorship_is_applied_to_combination_authorship() {
        // The separate-authorship counterpart of the test just above: same expected
        // authorship, but supplied via the `authorship` argument instead of embedded in
        // `name`, exercising `extra_state`/`apply_authorship` on the aux path.
        let pn = run("Abies alba", Some("Mill."), None, None).expect("should parse");
        assert_eq!(pn.combination_authorship.authors, vec!["Mill.".to_string()]);
        assert_eq!(pn.genus, Some("Abies".to_string()));
        assert_eq!(pn.specific_epithet, Some("alba".to_string()));
    }

    #[test]
    fn separately_supplied_authorship_overwrites_embedded_authorship_when_both_are_given() {
        // Java's `applyAuthorship(ctx.name, extraState)` on the aux path has NO guard
        // against `ctx.name` already carrying an embedded/autonym authorship (unlike the
        // autonym path's own `!ctx.name.hasAuthorship()` guard) — it runs unconditionally
        // whenever a non-blank `authorship` argument is given, so a separately-supplied
        // authorship always wins over an embedded one when a caller (unusually) supplies
        // both, since the aux path runs last among the three.
        let pn = run("Abies alba Mill.", Some("L."), None, None).expect("should parse");
        assert_eq!(pn.combination_authorship.authors, vec!["L.".to_string()]);
    }

    #[test]
    fn separately_supplied_authorship_runs_through_strip_authorship_markers_first() {
        // A nom-note tail on the separately-supplied authorship must be stripped (via
        // `strip_authorship_markers`) before `AuthorshipParser::parse` sees it, exactly as
        // it would be if it were embedded in `name` and stripped by `run()`'s own
        // `strip_nom_note` — proving the two strippers are wired to agree.
        let pn = run("Abies alba", Some("Mill. nom. illeg."), None, None).expect("should parse");
        assert_eq!(pn.combination_authorship.authors, vec!["Mill.".to_string()]);
        assert_eq!(pn.nomenclatural_note, Some("nom. illeg.".to_string()));
    }

    #[test]
    fn standalone_manuscript_marker_as_the_whole_separate_authorship_sets_manuscript() {
        // `strip_authorship_markers`'s STANDALONE_MS early return: the aux authorship is
        // consumed entirely (manuscript=true, no authors), leaving the embedded name's own
        // (absent) authorship untouched.
        let pn = run("Abies alba", Some("ined."), None, None).expect("should parse");
        assert!(pn.manuscript);
        assert!(!pn.combination_authorship.exists());
    }

    #[test]
    fn sanctioning_author_is_last_write_wins_embedded_over_separately_supplied() {
        // Global Constraint 2 / the codeState-selection doc comment: the aux authorship's
        // sanctioning author is applied first, the embedded name's own sanctioning author
        // applied after (and so wins) — both present here to prove the ORDER, not just
        // that either one alone works.
        let pn =
            run("Boletus versicolor L. : Fr.", Some("X. : Y."), None, None).expect("should parse");
        assert_eq!(pn.sanctioning_author, Some("Fr.".to_string()));
    }

    #[test]
    fn code_state_falls_back_to_separately_supplied_authorship_when_embedded_has_none() {
        // Pipeline.java's own worked example for the codeState fallback, split across the
        // two arguments instead of embedded in one string: a bare uninomial (no embedded
        // trailing authorship at all, so `auth_state` stays `None`) with a separately
        // supplied basionym-only citation with a year — `extraState.basionymPresent &&
        // extraState.basionym.getYear() != null` — must drive code inference to
        // ZOOLOGICAL exactly as it would if "Heptacyclus (Vasileyev, 1939)" were one name.
        let pn = run("Heptacyclus", Some("(Vasileyev, 1939)"), None, None).expect("should parse");
        assert_eq!(pn.code, Some(NomCode::Zoological));
        assert_eq!(
            pn.basionym_authorship.authors,
            vec!["Vasileyev".to_string()]
        );
        assert_eq!(pn.basionym_authorship.year, Some("1939".to_string()));
    }

    #[test]
    fn autonym_species_author_drives_code_inference_when_no_other_authorship() {
        // Pipeline.java's own worked example (Pipeline.java:138-140) for the autonym
        // codeState fallback: the mid-name "(Klatt) Baker" span IS the species author of
        // the autonym "spathata subsp. spathata" and infers BOTANICAL from its
        // basionym+combination authors, with no separately-supplied authorship involved.
        let pn = run(
            "Trimezia spathata (Klatt) Baker subsp. spathata",
            None,
            None,
            None,
        )
        .expect("should parse");
        assert!(pn.is_autonym());
        assert_eq!(pn.code, Some(NomCode::Botanical));
        assert_eq!(pn.combination_authorship.authors, vec!["Baker".to_string()]);
        assert_eq!(pn.basionym_authorship.authors, vec!["Klatt".to_string()]);
    }
}
