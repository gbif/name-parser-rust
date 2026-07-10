// SPDX-License-Identifier: Apache-2.0
//! Java `org.gbif.nameparser.pipeline.Pipeline` — orchestrates the staged parsing
//! pipeline. Each stage mutates the shared [`ParseContext`].

pub(crate) mod authorship_parser;
pub(crate) mod authorship_split;
pub(crate) mod context;
pub(crate) mod name_tokens;
pub(crate) mod preflight;
pub(crate) mod rank_markers;
pub(crate) mod stripandstash;

pub(crate) use context::ParseContext;

use std::sync::LazyLock;

use regex::Regex;

use crate::model::{warnings, NameType, NomCode, ParseError, ParsedName, Rank};
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
/// build [`ParseContext`] → split-glued-phrase → Preflight → StripAndStash (skeleton
/// only so far — see the `stripandstash` module) → … Downstream stages (Tokenizer,
/// AuthorshipSplit, NameTokens, AuthorshipParser, Assemble) are later slices — see the
/// `TODO` below.
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

    // Java `Pipeline.run`: `StripAndStash.run(ctx); if (!hasLetter(ctx.working)) throw new
    // UnparsableNameException(NameType.OTHER, scientificName);` (Pipeline.java:70-73) — a
    // 4th inline guard, distinct from Preflight and from the 3 guards at the top of this
    // function, sitting between StripAndStash and the Tokenizer.
    stripandstash::run(&mut ctx);

    // StripAndStash's skeleton is wired in above, in Java's exact position (Phase 1 Slice 2
    // Task 2), but every one of its 55 steps is still a `// TODO batch N` no-op stub —
    // later Slice 2 batches replace them — so `ctx.working` here is still byte-for-byte
    // what Preflight left it. That already makes this guard sound in the direction that
    // matters: StripAndStash only ever strips characters out of `ctx.working` (stashing
    // annotation spans elsewhere on `ctx`), it never adds any, so "no letters left after
    // Preflight" already guarantees "no letters left after a fully-ported StripAndStash"
    // too — every input this guard rejects, Java would also reject post-strip. Known
    // deferred gap, not yet reachable by the corpus, that resolves itself automatically as
    // the batches land (no further change needed at this call site): an input WITH a
    // letter now, all of whose letters a *completed* StripAndStash would strip away (e.g. a
    // bracketed "[sic]"-only comment), isn't caught here yet. (Originally found via the
    // Task 6 golden corpus: `-,.#` — Java `Err(OTHER)` — before this guard existed at all.)
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

    // TODO: AuthorshipParser → Assemble (later slices) — including the autonym mid-author
    // application and the trailing-authorship parse when `boundary < ctx.tokens.len()`
    // (`Pipeline.java:78-95`, the AuthorshipParser slice).

    Ok(ctx.name)
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
        // rejects); the stubbed Preflight lets it through this slice.
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
        // Tokenizer + AuthorshipSplit + NameTokens are wired in as of Phase 1 Slice 3 Task
        // 3, so a clean binomial is now fully name-part-classified; only the authorship
        // fields (AuthorshipParser/Assemble, still `// TODO`) stay at their
        // ParseContext-seeded defaults.
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
        // "Abies " (6 chars) * 42 = 252 chars, still all-letters/space so Preflight's
        // no-op stub and the length guard (1000) both let it through.
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
}
