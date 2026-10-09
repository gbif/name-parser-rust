// SPDX-License-Identifier: Apache-2.0
//! Java `org.gbif.nameparser.pipeline.Pipeline` — orchestrates the staged parsing
//! pipeline. Each stage mutates the shared [`ParseContext`].

pub(crate) mod assemble;
pub(crate) mod authorship_parser;
pub(crate) mod authorship_split;
pub(crate) mod blacklisted_epithets;
pub(crate) mod cjk_names;
pub(crate) mod code_inference;
pub(crate) mod context;
pub(crate) mod culture_collections;
pub(crate) mod double_surnames;
pub(crate) mod name_tokens;
pub(crate) mod preflight;
pub(crate) mod rank_markers;
pub(crate) mod stripandstash;

pub(crate) use context::ParseContext;

use std::sync::LazyLock;

use regex::Regex;

use crate::model::{
    warnings, Authorship, CombinedAuthorship, NameType, NomCode, ParseError, ParsedName, Rank,
    State,
};
use crate::pipeline::authorship_parser::AuthState;
use crate::token::tokenize;
use crate::unicode::{is_fullwidth, java_trim, normalize_input, normalize_quotes};

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

/// A provisional name's separately supplied authorship joins its phrase, exactly as the embedded
/// form does (`Cantuaria sp. Forster, 1968` → phrase `sp. Forster, 1968`): an informal name with no
/// species epithet has no authorship slot of its own, so parsing it there lost it outright
/// (`Cantuaria sp.` + `Forster, 1968` came back as phrase `sp.`). A bracketed `[of … et al., 2023]`
/// source citation joins any designation's phrase, epithet or not — it is no authorship at all
/// (`Farrea occa n_ssp_NIWA_SO254` + `[of Dohrmann et al., 2023]`). Sources often repeat the
/// authorship in both columns, not always identically, so it is not appended when the phrase
/// already contains it ([`contains_ignoring_punctuation`]). Returns whether it was consumed.
fn append_authorship_to_phrase(authorship: &str, name: &mut ParsedName) -> bool {
    let a = java_trim(authorship);
    let Some(phrase) = name.phrase.as_ref() else {
        return false;
    };
    let is_citation = a.starts_with("[of ") && a.ends_with(']');
    if name.type_ != NameType::Informal || (name.specific_epithet.is_some() && !is_citation) {
        return false;
    }
    if !contains_ignoring_punctuation(phrase, a) {
        name.phrase = Some(format!("{phrase} {a}"));
    }
    true
}

/// `haystack` contains `needle` once both are reduced to their letters and digits — so
/// `sp. Forster, 1968` contains `Forster 1968`, and `sp.` contains `sp.`.
fn contains_ignoring_punctuation(haystack: &str, needle: &str) -> bool {
    let needle = letters(needle);
    needle.is_empty() || letters(haystack).contains(&needle)
}

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
    // Unicode space separators (NBSP & co) become ASCII spaces first, so the trim and every
    // later stage treat them as the word breaks they are; invisible format characters go and
    // fullwidth forms become ASCII (`normalize_input`). `name` itself stays raw for echoes.
    let spaced = normalize_input(name);
    let trimmed = java_trim(&spaced);
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
    let fullwidth = name
        .chars()
        .chain(authorship.unwrap_or_default().chars())
        .any(is_fullwidth);
    let authorship = authorship.map(|a| normalize_quotes(&normalize_input(a)));

    // The length of the name as a whole, whichever column its authorship came in — counted once
    // when the name string already repeats it.
    let full_length = trimmed.chars().count()
        + authorship
            .as_deref()
            .map(java_trim)
            .filter(|a| !a.is_empty() && !contains_ignoring_punctuation(&trimmed, a))
            .map_or(0, |a| a.chars().count() + 1);
    let mut ctx = ParseContext::new(trimmed.clone(), authorship, rank, code);
    // folded up front (see `normalize_input`), but still flagged like the homoglyphs they are
    if fullwidth {
        ctx.name.add_warning(warnings::HOMOGLYHPS);
    }
    if full_length > LONG_NAME_LENGTH {
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
    // A qualified genus (`?Sydonia alba`, `cf. Platypeltis croftii`) makes a binomial open
    // nomenclature, like the in-name `Sydonia? alba` / `Abies cf. alba`.
    // A bare qualified uninomial (`?Monotremata`) keeps its type: as INFORMAL it would become a flat
    // Informal result, which has no slot for the qualifier or the doubtful flag.
    if ctx.qualified_genus && ctx.name.specific_epithet.is_some() {
        ctx.name.type_ = NameType::Informal;
    }

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
    // Any other name keeps that mid-name span as the species' authorship ("Festuca ovina L. subsp.
    // guestfalica …" — it used to be dropped). The authorship after the infraspecific epithet is
    // the name's own — on an autonym that carries one too ("Pilocarpus microphyllus Stapf ex
    // Wardlew. var. microphyllus Rizzini").
    let mut species_state: Option<AuthState> = None;
    if ctx.mid_author_from >= 0 && autonym_state.is_none() && ctx.name.specific_authorship.is_none()
    {
        let from = ctx.mid_author_from as usize;
        let to = ctx.mid_author_to as usize;
        let st = authorship_parser::parse(&ctx.tokens[from..to], 0);
        if st.combination.exists() || st.basionym.exists() {
            ctx.name.specific_authorship = Some(CombinedAuthorship {
                combination_authorship: st.combination.clone(),
                basionym_authorship: st.basionym.clone(),
            });
            species_state = Some(st);
        }
    }
    // So does a provisional infraspecific designation whose author stands before it ("Acacia
    // mutabilis Maslin subsp. Young River (G.F. Craig 2052)"): the phrase has no author of its own.
    let phrase_after_author = ctx.name.rank.is_infraspecific()
        && ctx.name.specific_epithet.is_some()
        && ctx.name.infraspecific_epithet.is_none()
        && ctx
            .name
            .combination_authorship
            .authors
            .first()
            .is_some_and(|author| {
                let phrase = ctx.name.phrase.as_deref().unwrap_or_default();
                let text = letters(&trimmed);
                match (text.find(&letters(author)), text.find(&letters(phrase))) {
                    (Some(a), Some(p)) => !phrase.is_empty() && a < p,
                    _ => false,
                }
            });
    if phrase_after_author && ctx.name.specific_authorship.is_none() {
        ctx.name.specific_authorship = Some(CombinedAuthorship {
            combination_authorship: std::mem::take(&mut ctx.name.combination_authorship),
            basionym_authorship: std::mem::take(&mut ctx.name.basionym_authorship),
        });
    }
    // A name string ending in its cultivar epithet ("Acer campestre L. cv. 'nanum'") can only
    // carry the species author, so it is the specific authorship too — the cultivar has none of
    // its own; given one ("… L. cv. 'Elsrijk' Broerse", or separately), that is the name's.
    let ends_in_cultivar = ctx
        .name
        .cultivar_epithet
        .as_deref()
        .is_some_and(|cv| letters(&trimmed).ends_with(&letters(cv)));
    if ends_in_cultivar
        && ctx.name.has_authorship()
        && ctx.name.specific_authorship.is_none()
        && ctx.pending_specific_author.is_none()
    {
        ctx.name.specific_authorship = Some(CombinedAuthorship {
            combination_authorship: std::mem::take(&mut ctx.name.combination_authorship),
            basionym_authorship: std::mem::take(&mut ctx.name.basionym_authorship),
        });
    }

    // Separately-supplied authorship: run the name string's annotation steps that apply to an
    // authorship on it too (uncertainty, imprint years, homoglyphs, HTML, the dagger, sic /
    // corrig, notes, pro parte, page, in press, references …) so its tokens are clean before
    // parsing, then re-tokenise and parse it independently — so an authorship parses alike in
    // either column.
    // An `in` / `apud` citation is split off exactly as on the name string (#20): the host
    // goes to `publishedIn` and its year becomes the pending, code-neutral publication year.
    // A reference the name string already gave is kept as it is: sources often repeat the
    // authorship in both columns, not always identically (`Hwass in Bruguiere, 1792` +
    // `Hwass in Bruguiere`), and appending the second copy would record the reference twice.
    // A sanctioning author found here is applied immediately (the embedded path's own
    // sanctioning author, applied further below, overwrites it — last-write-wins).
    let mut extra_state: Option<AuthState> = None;
    if let Some(authorship) = ctx.authorship_input.clone() {
        // The name string's own authorship: the trailing one, or an autonym's species author.
        let own = auth_state
            .as_ref()
            .or(autonym_state.as_ref())
            .filter(|st| st.combination.exists() || st.basionym.exists());
        let repeated = own.is_some()
            && !letters(&authorship).is_empty()
            && contains_ignoring_punctuation(&trimmed, &authorship);
        match own.filter(|_| repeated) {
            // The name string repeats this authorship — sources fill both columns, not always
            // alike — so keep the name string's parse, which may well carry more: the year of
            // `Germar, 1848` + `Germar`, the brackets of `(Bloch, 1792)` + `Bloch 1792`, the
            // variety author of `… P.Willemet var. roxburghianus Müll.Arg.` + `P.Willemet`. The
            // column is parsed on a copy and only lends its spelling when it says the same
            // (`Mill` + `Mill.`), so nothing it carries is recorded twice.
            Some(own) => {
                let mut column = ctx.clone();
                if let Some(st) = parse_separate_authorship(&mut column, authorship) {
                    if same_authorship(own, &st) {
                        apply_authorship(&mut ctx.name, &st);
                        extra_state = Some(st);
                    }
                }
            }
            None => {
                // A note the name string already gave is not recorded twice: sources repeat
                // notes in both columns too ("Abies keralia spec. nov." + "spec. nov.").
                let notes = (
                    ctx.name.nomenclatural_note.clone(),
                    ctx.name.taxonomic_note.clone(),
                );
                extra_state = parse_separate_authorship(&mut ctx, authorship);
                drop_repeated_note(&mut ctx.name.nomenclatural_note, notes.0);
                drop_repeated_note(&mut ctx.name.taxonomic_note, notes.1);
            }
        }
    }

    // Code inference reads the authorship the name ends up with: the separately supplied one
    // when it carries authors or a basionym (it is applied last, so it wins), else the name
    // string's own, else the autonym's species author. Java 4.2.0 consulted a separate
    // authorship only when the name string had none AND it carried a basionym with a year or a
    // combination author, so `Aus bus` + `L., 1758` got no code while `Aus bus L., 1758` was
    // zoological — a deliberate change: both paths now infer alike. A name whose only authorship
    // is the species author before its rank marker ("Orchis punctulata Steven ex Lindl. var.")
    // infers from that one.
    let has_signal = |st: &&AuthState| !code_state_needs_fallback(Some(st));
    let code_state: Option<&AuthState> = extra_state
        .as_ref()
        .filter(has_signal)
        .or_else(|| auth_state.as_ref().filter(has_signal))
        .or(autonym_state.as_ref())
        .or_else(|| species_state.as_ref().filter(has_signal))
        .or(auth_state.as_ref())
        .or(extra_state.as_ref());
    if !code_state.is_some_and(|c| species_state.as_ref().is_some_and(|s| std::ptr::eq(c, s))) {
        ctx.species_code_state = species_state.clone();
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

/// Runs a separately supplied authorship through the name string's annotation steps and the
/// authorship parser onto `ctx` (see the comment at the call site in [`run`]). Returns its parsed
/// state, or `None` when nothing was left to parse as an authorship: a placeholder, a bracketed
/// family-group authorship, or one an informal name's phrase took.
fn parse_separate_authorship(ctx: &mut ParseContext, authorship: String) -> Option<AuthState> {
    // a placeholder ("Missing", "Not specified") is dropped first, so it never reaches the
    // phrase of a provisional name or the authorship parser
    let authorship = stripandstash::strip_authorship_placeholder(&authorship, &mut ctx.name);
    if !authorship.chars().all(crate::token::is_whitespace_java)
        && !stripandstash::stash_bracketed_family_group_authorship(&authorship, &mut ctx.name)
        && !append_authorship_to_phrase(&authorship, &mut ctx.name)
    {
        let auth_clean = stripandstash::strip_authorship_leading_steps(ctx, authorship);
        let auth_clean = stripandstash::strip_authorship_markers(&auth_clean, &mut ctx.name);
        let auth_clean = stripandstash::strip_authorship_trailing_steps(ctx, auth_clean);
        let embedded_reference = ctx
            .name
            .published_in
            .clone()
            .map(|r| (r, ctx.name.published_in_year));
        let auth_clean =
            stripandstash::strip_bracketed_manuscript_marker(&auth_clean, &mut ctx.name);
        let auth_clean = stripandstash::strip_in_author_citations(ctx, auth_clean);
        let auth_clean = stripandstash::strip_authorship_reference_steps(ctx, auth_clean);
        // after the in-citation, as on the name string: "Busk ms in Chimonides, 1987"
        let auth_clean =
            stripandstash::strip_trailing_manuscript_marker(&auth_clean, &mut ctx.name);
        if let Some((reference, year)) = embedded_reference {
            ctx.name.published_in = Some(reference);
            ctx.name.published_in_year = year;
        }
        let aux = tokenize(&auth_clean);
        let st = authorship_parser::parse(&aux, 0);
        apply_authorship(&mut ctx.name, &st);
        if st.unparsed_from >= 0 {
            ctx.name.state = State::Partial;
            // added to what the name string left unparsed, not in its place
            if let Some(rest) = st.unparsed_text.clone() {
                ctx.name.unparsed = Some(match ctx.name.unparsed.take() {
                    Some(own) if contains_ignoring_punctuation(&own, &rest) => own,
                    Some(own) => format!("{own} {rest}"),
                    None => rest,
                });
            }
        }
        return Some(st);
    }
    None
}

/// Undoes the separate authorship's addition to a note when the name string's note already said
/// the same: the note was `before`, and everything appended since is already in it.
fn drop_repeated_note(note: &mut Option<String>, before: Option<String>) {
    if let (Some(now), Some(before)) = (note.as_deref(), before) {
        if let Some(added) = now.strip_prefix(before.as_str()) {
            if contains_ignoring_punctuation(&before, added) {
                *note = Some(before);
            }
        }
    }
}

/// `a` and `b` name the same combination and basionym authors, ex-authors and years, regardless of
/// punctuation and spacing (`Mill` and `Mill.`).
fn same_authorship(a: &AuthState, b: &AuthState) -> bool {
    let key = |x: &Authorship| {
        (
            letters(&x.authors.concat()),
            letters(&x.ex_authors.concat()),
            x.year.clone(),
            x.anonymous,
        )
    };
    key(&a.combination) == key(&b.combination) && key(&a.basionym) == key(&b.basionym)
}

/// `s` reduced to its letters and digits.
fn letters(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect()
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
    fn sanctioning_author_goes_with_the_authorship_that_wins() {
        // The sanctioning author is part of its authorship: the separately supplied one, applied
        // last, wins with it.
        let pn =
            run("Boletus versicolor L. : Fr.", Some("X. : Y."), None, None).expect("should parse");
        assert_eq!(pn.combination_authorship.authors, vec!["X.".to_string()]);
        assert_eq!(
            pn.combination_authorship.sanctioning_author,
            Some("Y.".to_string())
        );
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
