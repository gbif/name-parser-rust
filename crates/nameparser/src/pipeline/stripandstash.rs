// SPDX-License-Identifier: Apache-2.0
//! Java `org.gbif.nameparser.pipeline.StripAndStash` — the pre-tokenisation stripper.
//! Removes annotations from the working string and stashes them onto
//! `ParseContext`/`ParsedName`. Per Java's own class doc: order matters — markers are
//! stripped from the most specific to the most general so that, for instance, a
//! "[sic, porphyria]" comment doesn't leak through the plainer "[sic]" path. [`run`] is
//! the explicit, ordered list of 55 strip steps; each step is a self-contained function
//! that takes the working string, mutates the [`ParseContext`] as needed, and returns
//! the (possibly shortened/rewritten) working string.
//!
//! **Phase 1 Slice 2, batches 1 + 2b (this batch): steps 1-30 ported.** [`run`] dispatches
//! all 55 steps in Java's exact `StripAndStash.run(ParseContext)` order — that order is
//! load-bearing and was locked in by Task 2 — steps 1-19 (leading normalizers + flaggers,
//! batch 1) and 20-30 (candidatus, cultivar Group/grex/quoted, extinct dagger, t.infr.,
//! doubtful-genus brackets, sic/corrig, synonym bracket, bracketed + bare nom-note, batch
//! 2b) now carry their faithful port; steps 31-55 (batches 2c-2e) remain `// TODO batch N`
//! no-op passthroughs (see `docs/superpowers/plans/2026-07-10-phase1-stripandstash.md`
//! for the batch breakdown). One step, `replace_homoglyphs` (step 13), is a DOCUMENTED
//! stub — see its own doc comment — since porting its backing table is a sizeable
//! sub-project deferred by design (mirrors `crate::unicode`'s own existing deferral of the
//! same table).

use std::sync::LazyLock;

use fancy_regex::Regex as FancyRegex;
use regex::Regex;

use crate::model::{warnings, NameType, NomCode, ParsedName, Rank};
use crate::pipeline::ParseContext;
use crate::token;
use crate::unicode::java_trim;

/// Java `StripAndStash.run(ParseContext ctx)`. Ordered dispatcher: threads the working
/// string through all 55 steps in Java's exact order (`StripAndStash.java:568-626`),
/// each step consuming the string by value and returning the (possibly
/// shortened/rewritten) result; steps also mutate `ctx` (`ctx.name`, `ctx.pending*`, …)
/// as a side effect for the ones that stash annotations rather than just discard them.
/// Steps 1-19 (batch 1) are ported; steps 20-55 (batches 2-5) are still `// TODO batch N`
/// no-op stubs — see the module doc.
pub(crate) fn run(ctx: &mut ParseContext) {
    let mut s = ctx.working.clone();
    s = flag_uncertain_authorship(ctx, s);
    s = extract_generic_author(ctx, s);
    s = strip_quoted_monomial(ctx, s);
    s = apply_missing_genus_placeholder(ctx, s);
    s = strip_infra_rank_letters(ctx, s);
    s = normalise_letter_subdivision_marker(ctx, s);
    s = repair_question_mark_in_word(ctx, s);
    s = strip_strain_designation(ctx, s);
    s = stash_trailing_strain_code(ctx, s);
    s = strip_imprint_years(ctx, s);
    s = strip_null_between_epithets(ctx, s);
    s = normalise_hyphens(ctx, s);
    s = replace_homoglyphs(ctx, s);
    s = repair_win1252_artefacts(ctx, s);
    s = normalise_double_underscores(ctx, s);
    s = stash_trailing_otu_code(ctx, s);
    s = strip_serovar_serotype(ctx, s);
    s = strip_angle_bracket_authorship(ctx, s);
    s = strip_html(ctx, s);
    s = strip_candidatus(ctx, s);
    s = normalise_hort_ex_placeholder(ctx, s);
    s = strip_cultivar_group_grex(ctx, s);
    s = strip_quoted_cultivar(ctx, s);
    s = strip_extinct_dagger(ctx, s);
    s = strip_tinfr_marker(ctx, s);
    s = strip_doubtful_genus_brackets(ctx, s);
    s = strip_sic_and_corrig(ctx, s);
    s = stash_synonym_bracket(ctx, s);
    s = strip_bracketed_nom_note(ctx, s);
    s = strip_nom_note(ctx, s);
    s = strip_authorship_placeholders(ctx, s);
    s = strip_trailing_species_word(ctx, s);
    s = strip_pro_parte(ctx, s);
    s = strip_pro_sp_annotation(ctx, s);
    s = strip_approved_lists(ctx, s);
    s = strip_mihi(ctx, s);
    s = normalise_anon(ctx, s);
    s = strip_colon_concept_reference(ctx, s);
    s = strip_bracketed_tax_note(ctx, s);
    s = strip_paren_tax_note(ctx, s);
    s = strip_sensu_lato_remainder(ctx, s);
    s = strip_sensu_stricto_ss(ctx, s);
    s = strip_tax_note(ctx, s);
    s = strip_aggregate_suffix(ctx, s);
    s = strip_published_page(ctx, s);
    s = strip_in_press(ctx, s);
    s = strip_in_author_in_parens(ctx, s);
    s = strip_in_author_citation(ctx, s);
    s = strip_ipni_citation(ctx, s);
    s = strip_period_separated_reference(ctx, s);
    s = strip_comma_prefixed_reference(ctx, s);
    s = strip_manuscript_marker(ctx, s);
    s = strip_supra_rank_prefix(ctx, s);
    s = strip_leading_infrageneric_marker(ctx, s);
    s = stash_phrase_name(ctx, s);
    ctx.working = s;
}

// ===================================================================================
// Batch 1 (steps 1-19): leading normalizers + flaggers. Java StripAndStash.java's
// `flagUncertainAuthorship` through `stripHtml`. Ported (Phase 1 Slice 2, Task 3).
//
// Per-pattern flag rule applied throughout (see `pipeline::preflight`'s module doc for the
// rule spelled out in full, and `regexes.rs`'s module doc for the possessive-quantifier
// note): Java `Pattern.UNICODE_CHARACTER_CLASS` -> keep the crate's default Unicode
// `\d\s\w\b`; otherwise ASCII-scope every `\s`/`\d`/`\w`/`\b` atom via `(?-u:…)` (the WHOLE
// alternation when there's no `\p{…}` class AND no unescaped wildcard `.` anywhere in the
// pattern; only the individual atoms when either is present); `\p{Lu}`/`\p{Ll}`/`\p{L}`
// always Unicode; `CASE_INSENSITIVE` -> `(?i)`. A handful of patterns carry lookaround or a
// backreference the linear `regex` crate can't express at all — those use `fancy_regex`
// verbatim (each one's own doc comment explains why a restructuring was rejected, following
// `regexes.rs`'s `CORRIG` precedent) except where the ONLY call site is a boolean
// existence check (`.find()`/`.matches()`, never `.replaceAll()`), in which case dropping
// the lookahead is a sound restructuring (same class as Preflight's `PURE_ALPHANUM`).
//
// Every worked example below was spot-checked against the real Java CLI oracle
// (`name-parser-cli-4.2.0-SNAPSHOT-shaded.jar`) on adjunct test input — not just traced by
// hand — since the corpus golden harness alone can't validate step-level behaviour for
// fields the (not yet ported) tokenizer/authorship-parser/Assemble stages would otherwise
// surface.
// ===================================================================================

/// Java `hasEarlierYear(String, int)` (StripAndStash.java:362-365): true if a 4-digit year
/// appears anywhere in `s[0, end)`. `end` is a byte offset from a prior match's `.start()`,
/// always a valid UTF-8 char boundary.
///
/// Java YEAR_4DIGIT (StripAndStash.java:161): `\b\d{4}\b`, no flags. Has `\b`/`\d`, no
/// `\p{...}`, no unescaped wildcard -> whole pattern ASCII-scoped.
static YEAR_4DIGIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?-u:\b\d{4}\b)").unwrap());

fn has_earlier_year(s: &str, end: usize) -> bool {
    YEAR_4DIGIT.is_match(&s[..end])
}

/// Generalised non-overlapping `.replace_all` for a `fancy_regex::Regex`, tolerant of a
/// match error (breaks rather than propagating/panicking — this pipeline must never panic
/// on arbitrary, possibly-malformed input; `fancy_regex::Regex::replace_all` itself
/// `.unwrap()`s internally on any match error, so it is deliberately NOT used here). Mirrors
/// `regexes::strip_corrig_fancy`'s manual find_iter+splice loop, generalised with a
/// per-match closure so a caller needing to reinsert a captured group (`GREEK_MARKER`'s
/// "$1 " replacement) and a caller needing only a fixed replacement (`STAR_MARKER`,
/// `NULL_EPITHET_MID`) can share one implementation.
fn fancy_replace_all(
    re: &FancyRegex,
    s: &str,
    replacement: impl Fn(&fancy_regex::Captures) -> String,
) -> String {
    let mut result = String::with_capacity(s.len());
    let mut last = 0usize;
    for m in re.captures_iter(s) {
        let caps = match m {
            Ok(c) => c,
            Err(_) => break,
        };
        let whole = caps.get(0).expect("group 0 is always present in a match");
        result.push_str(&s[last..whole.start()]);
        result.push_str(&replacement(&caps));
        last = whole.end();
    }
    result.push_str(&s[last..]);
    result
}

// ---- Step 1: flagUncertainAuthorship ----

/// Java TRAILING_QMARK (StripAndStash.java:298): `\s\?\s*$`, no flags. Has `\s`, no
/// `\p{...}`, no unescaped wildcard -> whole pattern ASCII-scoped (trailing `$` left
/// outside the wrap, per convention — anchors aren't `u`-sensitive either way).
static TRAILING_QMARK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?-u:\s\?\s*)$").unwrap());

/// Java UNCERTAIN_AUTHOR_QMARK (StripAndStash.java:302-303):
/// `\p{L}\?(?=\s*(?:$|[&,]))`, `Pattern.UNICODE_CHARACTER_CLASS` (keep default Unicode
/// `\s`). RESTRUCTURED: the trailing lookahead is dropped. The `regex` crate has no
/// lookaround at all, but the ONLY call site (`flag_uncertain_authorship` below) uses this
/// pattern solely as a boolean `.find()` gate — never `.replaceAll()` — so consuming the
/// lookahead's content instead of merely peeking at it is unobservable: existence of a
/// match is identical either way. (Same restructuring class as Preflight's
/// `PURE_ALPHANUM`.)
static UNCERTAIN_AUTHOR_QMARK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\p{L}\?\s*(?:$|[&,])").unwrap());

/// Java UNCERTAIN_AUTHOR_OR (StripAndStash.java:304-305): `\p{Lu}\p{L}*\s+or\s+\p{Lu}`,
/// `Pattern.UNICODE_CHARACTER_CLASS` -> keep default Unicode, ported verbatim.
static UNCERTAIN_AUTHOR_OR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\p{Lu}\p{L}*\s+or\s+\p{Lu}").unwrap());

/// Java UNCERTAIN_AUTHOR_SLASH (StripAndStash.java:306-307): `\p{L}\s*/\s*\p{L}`,
/// `Pattern.UNICODE_CHARACTER_CLASS` -> keep default Unicode, ported verbatim.
static UNCERTAIN_AUTHOR_SLASH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\p{L}\s*/\s*\p{L}").unwrap());

/// Java `StripAndStash.flagUncertainAuthorship` (StripAndStash.java:538-551). Flags
/// open-nomenclature uncertainty before any of it is normalised away. A trailing standalone
/// "?" ("… (Author, 1886) ?") is dropped but marks the name doubtful. A "?" glued to an
/// author word ("Sess?", "Smith?") or alternative authors joined by "or" / "/" ("Jarocki or
/// Schinz", "Smith/Jones") mark the authorship uncertain WITHOUT stripping anything — the
/// "?"/"or"/"/" stay in the working string (spot-checked against the Java CLI oracle: "Aus
/// bus Sess?" keeps the literal "Sess?" through this step — the "?" is only later dropped by
/// the not-yet-ported tokenizer; "Aus bus Jarocki or Schinz, 1900" keeps the literal " or "
/// in the author text all the way to the final parse). Internal "letter?letter"
/// transcription artefacts ("Istv?nffi") are left to `repair_question_mark_in_word` (step 7).
fn flag_uncertain_authorship(ctx: &mut ParseContext, mut s: String) -> String {
    if TRAILING_QMARK.is_match(&s) {
        ctx.name.doubtful = true;
        ctx.name.add_warning(warnings::QUESTION_MARKS_REMOVED);
        s = java_trim(&TRAILING_QMARK.replace_all(&s, "")).to_string();
    }
    if UNCERTAIN_AUTHOR_QMARK.is_match(&s)
        || UNCERTAIN_AUTHOR_OR.is_match(&s)
        || UNCERTAIN_AUTHOR_SLASH.is_match(&s)
    {
        ctx.name.doubtful = true;
        ctx.name.add_warning(warnings::UNCERTAIN_AUTHORSHIP);
    }
    s
}

// ---- Step 2: extractGenericAuthor ----

/// Java INFRAGEN_AUTHOR_BEFORE_MARKER (StripAndStash.java:211-216),
/// `Pattern.UNICODE_CHARACTER_CLASS` -> keep default Unicode, ported verbatim. "Cordia
/// (Adans.) Kuntze sect. Salimori" — authorship placed BEFORE an infrageneric rank marker.
/// group(1)=genus, group(2)=author span (optional parenthesised basionym + combination
/// author words), group(3)=marker + sectional epithet.
static INFRAGEN_AUTHOR_BEFORE_MARKER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(\p{Lu}[\p{Ll}]+)\s+((?:\(\s*[^()]*\)\s*)?\p{Lu}[\p{L}.'\-]*(?:\s+\p{Lu}[\p{L}.'\-]*)*)\s+((?:subg|subgen|subgenus|sect|subsect|supersect|ser|subser|superser|divisio|div)\.?\s+\p{Lu}[\p{Ll}]+)$",
    )
    .unwrap()
});

/// Java `StripAndStash.extractGenericAuthor` (StripAndStash.java:559-566). Splits off an
/// authorship placed before an infrageneric rank marker as the genus author: "Cordia
/// (Adans.) Kuntze sect. Salimori" leaves "Cordia sect. Salimori" for the pipeline (the
/// sectional epithet is not read as an author) and stashes "(Adans.) Kuntze" in
/// `ctx.pending_generic_author` for `Pipeline` (not yet ported) to apply as the generic
/// authorship — the section itself is unauthored.
fn extract_generic_author(ctx: &mut ParseContext, s: String) -> String {
    // Java calls `.matches()` (full-string match); the pattern's own `^…$` anchors make
    // `.captures()` here equivalent (a match can only ever span the whole string).
    if let Some(caps) = INFRAGEN_AUTHOR_BEFORE_MARKER.captures(&s) {
        let author = caps.get(2).unwrap().as_str();
        ctx.pending_generic_author = Some(java_trim(author).to_string());
        return format!(
            "{} {}",
            caps.get(1).unwrap().as_str(),
            caps.get(3).unwrap().as_str()
        );
    }
    s
}

// ---- Step 3: stripQuotedMonomial ----

/// Java QUOTED_MONOMIAL (StripAndStash.java:162-164):
/// `^(['"])\s*([\p{Lu}][\p{L}-]+)\s*\1(\s+.+)?$`, `Pattern.UNICODE_CHARACTER_CLASS` -> keep
/// default Unicode (fancy_regex's own default, so no scoping needed either way). Contains a
/// BACKREFERENCE (`\1`, matching whichever quote char group 1 captured) -> needs
/// `fancy_regex` (the `regex` crate has no backreferences at all). Contains a literal `"`
/// -> `r#"…"#` raw string.
static QUOTED_MONOMIAL: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r#"^(['"])\s*([\p{Lu}][\p{L}-]+)\s*\1(\s+.+)?$"#).unwrap());

/// Java `StripAndStash.stripQuotedMonomial` (StripAndStash.java:655-666). A leading
/// monomial wrapped in quotes ("'Prosthète' Hesse, 1861" / "\"Foo\" Bar, 2000") marks a
/// word that is not an available scientific name. Strips the quotes for parsing, remembers
/// the quote char in `ctx.quoted_monomial` (so `Assemble`, not yet ported, can re-wrap the
/// parsed uninomial — confirmed against the Java CLI oracle: the final `uninomial` comes
/// back as `'Prosthète'`, quotes and all), and flags doubtful.
fn strip_quoted_monomial(ctx: &mut ParseContext, s: String) -> String {
    if let Ok(Some(caps)) = QUOTED_MONOMIAL.captures(&s) {
        ctx.quoted_monomial = Some(caps.get(1).unwrap().as_str().to_string());
        ctx.name.doubtful = true;
        let tail = caps.get(3).map(|m| m.as_str()).unwrap_or("");
        return java_trim(&format!("{}{}", caps.get(2).unwrap().as_str(), tail)).to_string();
    }
    s
}

// ---- Step 4: applyMissingGenusPlaceholder ----

/// Java MISSING_GENUS_EPITHET (StripAndStash.java:341-342): `^[a-z][a-z\-]+\s+\p{Lu}.*`, no
/// flags. Has `\s` and `\p{Lu}` -> only `\s` ASCII-scoped. Called via `.matches()` in Java
/// -> trailing `$` added (the pattern already opens with `^`).
static MISSING_GENUS_EPITHET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z\-]+(?-u:\s+)\p{Lu}.*$").unwrap());

/// Java MISSING_GENUS_NOTE_KEYWORD (StripAndStash.java:343-344):
/// `^(?:non|nec|not|sensu|sec|auct|auctt|fide|emend|ss|s|cf|aff|hort)\b.*`, no flags. Has
/// `\b` and an unescaped wildcard `.*` -> only `\b` ASCII-scoped (never whole-wrapped,
/// since that would also flip `.`'s Unicode-scalar-vs-byte meaning). Called via
/// `.matches()` -> trailing `$` added.
static MISSING_GENUS_NOTE_KEYWORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:non|nec|not|sensu|sec|auct|auctt|fide|emend|ss|s|cf|aff|hort)(?-u:\b).*$")
        .unwrap()
});

/// Java `StripAndStash.firstWord` (StripAndStash.java:649-653): the leading non-whitespace
/// run, per Java's own `Character.isWhitespace` semantics (not Rust's broader default) —
/// reuses `token::is_whitespace_java`.
fn first_word(s: &str) -> &str {
    match s.find(token::is_whitespace_java) {
        Some(idx) => &s[..idx],
        None => s,
    }
}

/// Java `StripAndStash.applyMissingGenusPlaceholder` (StripAndStash.java:668-701).
/// Missing-genus placeholder forms — the user-facing genus is replaced by "?":
///   `"denheyeri Eghbalian, …, 2017"`         -> `"? denheyeri Eghbalian, …, 2017"` (+ warning)
///   `"Missing penchinati Bourguignat, 1870"` -> `"? penchinati Bourguignat, 1870"` (no warning)
///   `"\"? gryphoidis"`                       -> `"? gryphoidis"` (no warning)
/// Emits `NameType::Placeholder` for all three forms; `Warnings::MISSING_GENUS` only for
/// the inferred (third) form, since the other two carry an explicit "?"/"Missing" marker
/// the user wrote on purpose — all three spot-checked against the Java CLI oracle. Skips
/// the inferred form when the first word is a known taxonomic-note keyword that's NOT a
/// real epithet, or a surname particle ("van Berg") — a particle-led input is an author
/// name, not an epithet whose genus went missing.
fn apply_missing_genus_placeholder(ctx: &mut ParseContext, s: String) -> String {
    let mut missing: Option<String> = None;
    let mut emit_warning = false;
    if s.starts_with("\"? ") || s.starts_with("\"?\t") {
        let rest: String = s.chars().skip(3).collect();
        missing = Some(format!("? {}", java_trim(&rest)));
    } else if s.starts_with("Missing ") {
        let rest: String = s.chars().skip(8).collect();
        if rest.chars().next().is_some_and(|c| c.is_lowercase()) {
            missing = Some(format!("? {rest}"));
        }
    } else if s.chars().count() > 1
        && s.chars().next().is_some_and(|c| c.is_lowercase())
        && MISSING_GENUS_EPITHET.is_match(&s)
        && !MISSING_GENUS_NOTE_KEYWORD.is_match(&s)
        && !token::is_particle(first_word(&s))
    {
        missing = Some(format!("? {s}"));
        emit_warning = true;
    }
    if let Some(missing) = missing {
        ctx.name.type_ = NameType::Placeholder;
        if emit_warning {
            ctx.name.add_warning(warnings::MISSING_GENUS);
        }
        return missing;
    }
    s
}

// ---- Step 5: stripInfraRankLetters ----

/// Java GREEK_MARKER_TEST (StripAndStash.java:316-317):
/// `.*[\p{Ll}.]\s*[α-ω⍺](?:\s+|\.\s*)\p{Ll}.*`, no flags. Has `\s` (x2) and
/// `\p{Ll}` -> only `\s` ASCII-scoped. Called via `.matches()` on a `.*CORE.*` pattern —
/// equivalent to an unanchored `is_match` on CORE alone (dropping the `.*` bookends) since
/// `.` matches any non-newline char and no name string embeds a newline — restructured to
/// the core-only form.
static GREEK_MARKER_TEST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[\p{Ll}.](?-u:\s*)[\x{03B1}-\x{03C9}\x{237A}](?:(?-u:\s+)|\.(?-u:\s*))\p{Ll}")
        .unwrap()
});

/// Java STAR_MARKER_TEST (StripAndStash.java:318-319): `.*\p{Ll}\s+\*+\s+\p{Ll}.*`, no
/// flags. Has `\s` (x2) and `\p{Ll}` -> only `\s` ASCII-scoped. Same `.*CORE.*` ->
/// core-only restructuring as `GREEK_MARKER_TEST`.
static STAR_MARKER_TEST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\p{Ll}(?-u:\s+)\*+(?-u:\s+)\p{Ll}").unwrap());

/// Java GREEK_MARKER (StripAndStash.java:320-321):
/// `([\p{Ll}.])\s*[α-ω⍺](?:\s+|\.\s*)(?=[\p{Ll}])`, no flags (so Java's own `\s` here is
/// ASCII-only). Has a trailing LOOKAHEAD (not consumed by the match, so the replacement's
/// "$1 " leaves the following lowercase letter untouched right after it) -> needs
/// `fancy_regex` (the `regex` crate has no lookaround at all). NOT restructured to a
/// captured-and-reinserted group (unlike the `_TEST` sibling above): this pattern is used
/// for `.replaceAll`, and Java's zero-width lookahead lets a SECOND marker reuse the same
/// letter as ITS trailing-lookahead target when markers are adjacent — a capturing/consuming
/// rewrite would swallow that letter on the first match and miss the second (the same
/// adjacency asymmetry `regexes::CORRIG`'s doc comment documents at length). Greek-letter
/// rank markers are rare enough that the corpus gate is unlikely to exercise (and so could
/// not catch) such a subtle divergence, so the safer verbatim port is used instead.
/// `fancy_regex` has NO ASCII/non-Unicode mode at all — `(?-u:…)` is a hard parse error in
/// it, unconditionally (`fancy_regex::parse`'s flag parser rejects `-u` outright, unlike the
/// `regex` crate) — so ASCII-only `\s` is spelled out here as the literal Java ASCII
/// whitespace set `[ \t\n\x0B\f\r]` (space/tab/LF/VT/FF/CR) instead of the `\s` shorthand,
/// rather than attempting to scope it.
static GREEK_MARKER: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(
        r"([\p{Ll}.])[ \t\n\x0B\f\r]*[\x{03B1}-\x{03C9}\x{237A}](?:[ \t\n\x0B\f\r]+|\.[ \t\n\x0B\f\r]*)(?=[\p{Ll}])",
    )
    .unwrap()
});

/// Java STAR_MARKER (StripAndStash.java:322-323): `(?<=\p{Ll})\s+\*+\s+(?=\p{Ll})`, no
/// flags (ASCII-only `\s` in Java). Both boundaries are lookaround (lookbehind AND
/// lookahead) -> `fancy_regex`, same adjacency reasoning as `GREEK_MARKER`. Same
/// `fancy_regex`-has-no-ASCII-mode constraint as `GREEK_MARKER` -> `\s` spelled out as the
/// literal ASCII whitespace set rather than `(?-u:…)`-scoped.
static STAR_MARKER: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(r"(?<=\p{Ll})[ \t\n\x0B\f\r]+\*+[ \t\n\x0B\f\r]+(?=\p{Ll})").unwrap()
});

/// Java `StripAndStash.stripInfraRankLetters` (StripAndStash.java:703-717). Strips
/// Greek-like single-letter rank markers (α, β, …, and the APL-alpha lookalike U+237A) and
/// informal "***" markers sitting between two lowercase epithets — fungal rank markers
/// that must not be converted to ASCII letters or taken as authorship by downstream passes.
/// E.g. "Foo bar α baz" -> "Foo bar baz" and "Foo bar *** baz" -> "Foo bar baz" (both
/// spot-checked against the Java CLI oracle, which then parses the collapsed three-word
/// form as a normal trinomial).
fn strip_infra_rank_letters(_ctx: &mut ParseContext, s: String) -> String {
    if s.contains('\u{237A}') || GREEK_MARKER_TEST.is_match(&s) || STAR_MARKER_TEST.is_match(&s) {
        let s = fancy_replace_all(&GREEK_MARKER, &s, |caps| format!("{} ", &caps[1]));
        return fancy_replace_all(&STAR_MARKER, &s, |_| " ".to_string());
    }
    s
}

// ---- Step 6: normaliseLetterSubdivisionMarker ----

/// Java LETTER_SUBDIVISION_MARKER (StripAndStash.java:131-135),
/// `Pattern.UNICODE_CHARACTER_CLASS` -> keep default Unicode, ported verbatim. Old floras
/// subdivide a species informally with letters ("a.", "b.", "a.b.") between the species
/// name and a trailing epithet.
static LETTER_SUBDIVISION_MARKER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(\p{Lu}[\p{Ll}-]+\s+[\p{Ll}][\p{Ll}-]+(?:\s+\p{Lu}[\p{Ll}]*\.?)*)\s+((?:[a-z]\.){1,3}[a-z]?)\s+([\p{Ll}][\p{Ll}-]{2,})\s*$",
    )
    .unwrap()
});

/// Java `RankMarkers.LETTER_SUBDIVISION` (RankMarkers.java:21): the synthetic single-token
/// marker `StripAndStash` substitutes for informal letter-based species subdivisions, so
/// the (not yet ported) rank-marker machinery treats the trailing epithet as an
/// infraspecific of the unmappable rank `Rank::Other`. Never appears in user input.
const LETTER_SUBDIVISION: &str = "infrasubdivision";

/// Java `RankMarkers.matchInfraspecific(String) != null` — MINIMAL existence-only port.
/// `Rank` is still a stub in this Phase 1 slice (only `Unranked`/`Species`/`Subspecies`
/// exist), so the real `RankMarkers.INFRASPECIFIC` map (word -> `Rank`) can't be
/// represented yet; only the boolean null-check this one call site
/// (`normalise_letter_subdivision_marker` below) needs is ported here — the full map (with
/// real `Rank` values) lands in the dedicated rank-handling slice. Key set transcribed
/// verbatim from `RankMarkers.java`'s `INFRASPECIFIC` map (including `LETTER_SUBDIVISION`
/// itself, which Java's own map also carries as a key, and the non-letter `"*"` key, which
/// this call site can never actually produce). In practice the calling regex can only ever
/// split a SINGLE ASCII lowercase letter into `segments[0]` (see the doc comment below), so
/// only the `"f"` entry is reachable here — the full set is still ported (not hand-trimmed
/// down to just "f") so this stays an honest stand-in for the real map, not a
/// call-site-specific shortcut.
const KNOWN_INFRASPECIFIC_MARKERS: &[&str] = &[
    LETTER_SUBDIVISION,
    "subsp",
    "ssp",
    "var",
    "subvar",
    "subv",
    "f",
    "forma",
    "form",
    "fo",
    "subf",
    "subforma",
    "pv",
    "pathovar",
    "bv",
    "biovar",
    "ct",
    "chemoform",
    "sv",
    "serovar",
    "morph",
    "morphovar",
    "phagovar",
    "nat",
    "natio",
    "mut",
    "mutatio",
    "agamosp",
    "agamossp",
    "agamovar",
    "conv",
    "convar",
    "subspec",
    "variety",
    "fm",
    "fma",
    "prol",
    "proles",
    "ab",
    "aberration",
    "strain",
    "str",
    "st",
    "*",
];

fn is_known_infraspecific_marker(word: &str) -> bool {
    KNOWN_INFRASPECIFIC_MARKERS.contains(&word.to_lowercase().as_str())
}

/// Java `StripAndStash.normaliseLetterSubdivisionMarker` (StripAndStash.java:719-737).
/// Rewrites an informal letter-based subdivision marker ("a.", "b.", "a.b.") to the
/// synthetic `LETTER_SUBDIVISION` token, UNLESS the "letter" is itself a real rank marker
/// ("f." = forma), in which case the input is left alone for the normal rank-marker path.
/// Any abbreviated author before the marker is left in place (dropped by the mid-name-author
/// logic downstream — not this step's concern). Both branches spot-checked against the Java
/// CLI oracle: "Graphis scripta L. a.b pulverulenta" comes back `rank=OTHER` (rewritten),
/// "Graphis scripta L. f. pulverulenta" comes back `rank=FORM` (left alone).
fn normalise_letter_subdivision_marker(_ctx: &mut ParseContext, s: String) -> String {
    if let Some(caps) = LETTER_SUBDIVISION_MARKER.captures(&s) {
        let marker = caps.get(2).unwrap().as_str();
        // Java: `m.group(2).split("[^a-z]+")`. The only non-"[a-z]" character `marker` can
        // ever contain is '.' (its own capturing group is `(?:[a-z]\.){1,3}[a-z]?`), so
        // splitting on '.' and dropping empty pieces reproduces Java's regex-split +
        // trailing-empty-discard exactly for this constrained shape (consecutive dots can't
        // occur either, so no internal empty pieces are possible).
        let segments: Vec<&str> = marker.split('.').filter(|seg| !seg.is_empty()).collect();
        let real_marker = segments.len() == 1 && is_known_infraspecific_marker(segments[0]);
        if !real_marker {
            let g1 = caps.get(1).unwrap().as_str();
            let g3 = caps.get(3).unwrap().as_str();
            return format!("{g1} {LETTER_SUBDIVISION} {g3}");
        }
    }
    s
}

// ---- Step 7: repairQuestionMarkInWord ----

/// Java LETTER_QMARK_LETTER (StripAndStash.java:295): `.*\p{L}\?\p{L}.*`, no flags. Has
/// `\p{L}` and an unescaped wildcard `.*` bookends, no `\s`/`\d`/`\w`/`\b` atoms at all ->
/// nothing to ASCII-scope. Called via `.matches()` on a `.*CORE.*` shape -> restructured to
/// the core-only `is_match` form (same reasoning as `GREEK_MARKER_TEST`). Shared with the
/// not-yet-ported `stripAuthorshipMarkers` (StripAndStash.java:440) — defining it once here
/// makes it available for that future call site too.
static LETTER_QMARK_LETTER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\p{L}\?\p{L}").unwrap());

/// Java QMARK_BETWEEN_LETTERS (StripAndStash.java:296): `(\p{L})\?(\p{L})`, no flags. No
/// `\s`/`\d`/`\w`/`\b` atoms -> nothing to scope, ported verbatim. Shared with
/// `stripAuthorshipMarkers` (not yet ported), same as its sibling above.
static QMARK_BETWEEN_LETTERS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\p{L})\?(\p{L})").unwrap());

/// Java `StripAndStash.repairQuestionMarkInWord` (StripAndStash.java:739-748). A "?" inside
/// a word is a transcription artefact for a missing letter ("Istv?nffi") — strips the "?"
/// and glues the surrounding word parts directly together (no placeholder letter is
/// guessed: "Istv?nffi" -> "Istvnffi"), flagging doubtful + `QUESTION_MARKS_REMOVED`. (The
/// identical inline logic also opens the separate, not-yet-ported `stripAuthorshipMarkers`
/// auxiliary-authorship path — `StripAndStash.java` duplicates it rather than sharing a
/// helper, so this port does too, reusing the same static patterns.)
fn repair_question_mark_in_word(ctx: &mut ParseContext, s: String) -> String {
    if s.contains('?') && LETTER_QMARK_LETTER.is_match(&s) {
        let s = QMARK_BETWEEN_LETTERS.replace_all(&s, "$1$2").into_owned();
        ctx.name.doubtful = true;
        ctx.name.add_warning(warnings::QUESTION_MARKS_REMOVED);
        return s;
    }
    s
}

// ---- Step 8: stripStrainDesignation ----

/// Java STRAIN_DESIGNATION (StripAndStash.java:175-179):
/// `\s+(?:str|strain)\b\s*\.?(?:\s+(?:str|strain)\b\s*\.?)*\s*(['"])(.+?)\1\s*$`,
/// `Pattern.CASE_INSENSITIVE` (so Java's `\s`/`\b` here are ASCII-only). Has a BACKREFERENCE
/// (`\1`, matching whichever quote char group 1 captured) -> needs `fancy_regex` (the
/// `regex` crate has no backreferences at all). Contains a literal `"` -> `r#"…"#` raw
/// string. `fancy_regex` has no ASCII/non-Unicode mode at all (`(?-u:…)` is a hard parse
/// error, unconditionally — see `GREEK_MARKER`'s doc comment for the same constraint), so
/// `\s` is spelled out as the literal ASCII whitespace set `[ \t\n\x0B\f\r]` rather than
/// scoped; `\b` is left as `fancy_regex`'s only option (Unicode word-boundary) — a divergence
/// from Java's ASCII `\b` only for a "str"/"strain" token directly abutting a non-ASCII
/// word character with no space, vanishingly rare in practice and forced by this crate's
/// architecture rather than a faithfulness lapse.
static STRAIN_DESIGNATION: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(
        r#"(?i)[ \t\n\x0B\f\r]+(?:str|strain)\b[ \t\n\x0B\f\r]*\.?(?:[ \t\n\x0B\f\r]+(?:str|strain)\b[ \t\n\x0B\f\r]*\.?)*[ \t\n\x0B\f\r]*(['"])(.+?)\1[ \t\n\x0B\f\r]*$"#,
    )
    .unwrap()
});

/// Java `StripAndStash.stripStrainDesignation` (StripAndStash.java:628-647). A quoted
/// strain designation after a "str"/"strain" marker ("Aphanizomenon flos-aquae
/// str .'Aph K2'") is kept intact as `ctx.name.phrase` (type=INFORMAL) rather than let leak
/// into the authorship parser or get reinterpreted as a cultivar epithet. The remaining
/// "Genus species str." is left for NameTokens (not yet ported), which resolves the
/// trailing "str" marker to `Rank::Strain` — spot-checked against the Java CLI oracle:
/// `rank=STRAIN, phrase="Aph K2", type=INFORMAL`. Only fires when a plausible name (a
/// capitalised genus) precedes the marker, never on junk.
fn strip_strain_designation(ctx: &mut ParseContext, s: String) -> String {
    if let Ok(Some(caps)) = STRAIN_DESIGNATION.captures(&s) {
        let whole = caps.get(0).unwrap();
        let prefix = java_trim(&s[..whole.start()]).to_string();
        if !prefix.is_empty() && prefix.chars().next().is_some_and(|c| c.is_uppercase()) {
            ctx.name.phrase = Some(java_trim(&caps[2]).to_string());
            ctx.name.type_ = NameType::Informal;
            return format!("{prefix} str.");
        }
    }
    s
}

// ---- Step 9: stashTrailingStrainCode ----

/// Java TRAILING_STRAIN_CODE (StripAndStash.java:165-169),
/// `Pattern.UNICODE_CHARACTER_CLASS` -> keep default Unicode, ported verbatim.
static TRAILING_STRAIN_CODE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^([\p{Lu}][\p{Ll}]+\s+[\p{Ll}]+)\s+([dr]?RNA[a-zA-Z0-9_\-]*|[\p{Lu}][\p{L}\d]*\d[\p{L}\d_\-]*)\s*$",
    )
    .unwrap()
});

/// Java DIGITS_ONLY (StripAndStash.java:357): `\d+`, no flags — UNLIKE its sibling
/// `TRAILING_STRAIN_CODE` above, no `Pattern.UNICODE_CHARACTER_CLASS` here -> `\d`
/// ASCII-scoped. Called via `.matches()` -> `^…$` added.
static DIGITS_ONLY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?-u:\d+)$").unwrap());

/// Java `StripAndStash.stashTrailingStrainCode` (StripAndStash.java:750-768). A trailing
/// strain-code suffix on a binomial ("Candida albicans RNA_CTR0-3", "Armillaria ostoyae
/// RNA1") becomes `ctx.name.phrase` (type=INFORMAL), reducing the working string to "Genus
/// species" — spot-checked against the Java CLI oracle. The `DIGITS_ONLY` guard (never
/// consume a single trailing year) is, on inspection, unreachable via this regex: the
/// second alternative always starts with a captured `\p{Lu}` letter, so `code` can never be
/// digits-only — ported verbatim anyway (a faithful port preserves apparently-dead Java
/// guards, as already noted for `Preflight`'s `PR2_LIKE`).
fn stash_trailing_strain_code(ctx: &mut ParseContext, s: String) -> String {
    if let Some(caps) = TRAILING_STRAIN_CODE.captures(&s) {
        let code = caps.get(2).unwrap().as_str();
        if !DIGITS_ONLY.is_match(code) {
            ctx.name.phrase = Some(code.to_string());
            ctx.name.type_ = NameType::Informal;
            return caps.get(1).unwrap().as_str().to_string();
        }
    }
    s
}

// ---- Step 10: stripImprintYears ----

/// Java IMPRINT_YEAR_QUOTED (StripAndStash.java:180-181):
/// `\s*[\[\(]\s*"(\d{4}(?:[-–]\d{4})?)"\s*[\]\)]\s*\.?\s*$`, no flags. Has `\s`/`\d`, no
/// `\p{...}`, no unescaped wildcard `.` (only an escaped literal `\.?`) — but it DOES embed
/// a non-ASCII literal (the en dash `\x{2013}`) inside the custom class `[-\x{2013}]`. Java's
/// `UNICODE_CHARACTER_CLASS` flag only governs PREDEFINED shorthand classes (`\d`/`\s`/`\w`/
/// `\b`/POSIX `\p{Alpha}` etc.) — a literal/custom class like `[-\x{2013}]` always matches
/// exactly those code points in Java regardless of the flag, so it must stay OUTSIDE any
/// `(?-u:…)` scoping here too (`regex-syntax` rejects a non-ASCII literal inside a
/// `(?-u:…)` group outright: "Unicode not allowed here" — discovered when this pattern was
/// first run, not anticipated from the flag rule alone). Only the individual `\s`/`\d`
/// atoms are ASCII-scoped; the whole-pattern-wrap this port's rule would otherwise prefer
/// (no `\p{...}`, no unescaped wildcard) does not apply once a non-ASCII custom-class
/// literal is present. Contains a literal `"` -> `r#"…"#` raw string.
static IMPRINT_YEAR_QUOTED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?-u:\s*)[\[\(](?-u:\s*)"((?-u:\d{4})(?:[-\x{2013}](?-u:\d{4}))?)"(?-u:\s*)[\]\)](?-u:\s*)\.?(?-u:\s*)$"#).unwrap()
});

/// Java IMPRINT_YEAR_KEYWORD (StripAndStash.java:182-184):
/// `\s*\(\s*(?:imprint|not)\s+(\d{4}(?:[-–]\d{4})?)\s*\)\s*\.?\s*$`,
/// `Pattern.CASE_INSENSITIVE`. Same embedded-non-ASCII-literal situation as
/// `IMPRINT_YEAR_QUOTED` above (the `[-\x{2013}]` en-dash alternative) -> atom-only `\s`/`\d`
/// scoping, `[-\x{2013}]` left outside any `(?-u:…)` group.
static IMPRINT_YEAR_KEYWORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?-u:\s*)\((?-u:\s*)(?:imprint|not)(?-u:\s+)((?-u:\d{4})(?:[-\x{2013}](?-u:\d{4}))?)(?-u:\s*)\)(?-u:\s*)\.?(?-u:\s*)$")
        .unwrap()
});

/// Java IMPRINT_YEAR_ALT (StripAndStash.java:185-186): `\s+&\s+(\d{4})\s*\.?\s*$`, no
/// flags. Has `\s`/`\d`, no `\p{...}`, no unescaped wildcard -> whole-wrap.
static IMPRINT_YEAR_ALT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\s+&\s+(\d{4})\s*\.?\s*)$").unwrap());

/// Java `StripAndStash.stripImprintYears` (StripAndStash.java:770-808). By definition the
/// imprint year is a SECONDARY year cited alongside the publication year:
///   `"Storr, 1970 [\"1969\"]"`      -> `pending_imprint_year = "1969"` (quoted, always strips)
///   `"Storr, 1970 (imprint 1969)"`  -> `pending_imprint_year = "1969"` (explicit keyword, always strips)
///   `"Wagener, 1959 & 1961"`        -> `pending_imprint_year = "1961"` (bare "& YYYY", only
///                                      when another 4-digit year appears earlier)
/// All three spot-checked against the Java CLI oracle's `combinationAuthorship.imprintYear`.
/// The three checks run SEQUENTIALLY, each against the (possibly already-shortened) result
/// of the previous one — not all three against the original string.
fn strip_imprint_years(ctx: &mut ParseContext, mut s: String) -> String {
    if let Some(caps) = IMPRINT_YEAR_QUOTED.captures(&s) {
        let whole = caps.get(0).unwrap();
        ctx.set_pending_imprint_year(&caps[1]);
        s = java_trim(&s[..whole.start()]).to_string();
    }
    if let Some(caps) = IMPRINT_YEAR_KEYWORD.captures(&s) {
        let whole = caps.get(0).unwrap();
        ctx.set_pending_imprint_year(&caps[1]);
        s = java_trim(&s[..whole.start()]).to_string();
    }
    if let Some(caps) = IMPRINT_YEAR_ALT.captures(&s) {
        let whole = caps.get(0).unwrap();
        if has_earlier_year(&s, whole.start()) {
            ctx.set_pending_imprint_year(&caps[1]);
            s = java_trim(&s[..whole.start()]).to_string();
        }
    }
    s
}

// ---- Step 11: stripNullBetweenEpithets ----

/// Java NULL_EPITHET_TEST (StripAndStash.java:324-325): `.*[a-z]\s+null\s+[a-z]+.*`, no
/// flags. Has `\s` (x2), no `\p{...}`, unescaped wildcard `.*` bookends -> only `\s`
/// ASCII-scoped. `.matches()` on `.*CORE.*` -> restructured to core-only `is_match`.
static NULL_EPITHET_TEST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[a-z](?-u:\s+)null(?-u:\s+)[a-z]+").unwrap());

/// Java NULL_EPITHET_MID (StripAndStash.java:326-327): `(?<=[a-z])\s+null\s+(?=[a-z])`, no
/// flags (ASCII-only `\s` in Java). Lookbehind AND lookahead (both flanking letters are
/// zero-width, not consumed) -> `fancy_regex`, same adjacency-preserving reasoning as
/// `GREEK_MARKER`/`STAR_MARKER` above (a chain like "a null b null c" needs the shared
/// letter 'b' available to both matches). Same `fancy_regex`-has-no-ASCII-mode constraint
/// as `GREEK_MARKER` -> `\s` spelled out as the literal ASCII whitespace set rather than
/// `(?-u:…)`-scoped.
static NULL_EPITHET_MID: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(r"(?<=[a-z])[ \t\n\x0B\f\r]+null[ \t\n\x0B\f\r]+(?=[a-z])").unwrap()
});

/// Java `StripAndStash.stripNullBetweenEpithets` (StripAndStash.java:810-821). A bare
/// "null" between two lowercase epithets ("Austrorhynchus pectatus null pectatus") is a
/// data-quality artefact — dropped, flagging doubtful + `NULL_EPITHET` (spot-checked
/// against the Java CLI oracle). A single "null" epithet followed by an author span
/// ("Abies null Hood") is NOT touched here — left for `Assemble::flagBlacklistedEpithets`
/// (not yet ported).
fn strip_null_between_epithets(ctx: &mut ParseContext, s: String) -> String {
    if NULL_EPITHET_TEST.is_match(&s) {
        let s = fancy_replace_all(&NULL_EPITHET_MID, &s, |_| " ".to_string());
        ctx.name.doubtful = true;
        ctx.name.add_warning(warnings::NULL_EPITHET);
        return s;
    }
    s
}

// ---- Step 12: normaliseHyphens (structural — no Pattern) ----

/// Java `StripAndStash.normaliseHyphens` (StripAndStash.java:823-837). Normalises Unicode
/// hyphen variants (U+2010 HYPHEN, U+2011 NON-BREAKING HYPHEN, U+2012 FIGURE DASH, U+2013 EN
/// DASH, U+2014 EM DASH) to ASCII '-' so downstream tokenisation and canonical output use a
/// consistent character, flagging `HOMOGLYHPS` (spot-checked against the Java CLI oracle:
/// "Aus bus\u{2010}fer Mill." -> `specificEpithet="bus-fer"`, warning "homoglyphs
/// replaced") when anything actually changed.
fn normalise_hyphens(ctx: &mut ParseContext, s: String) -> String {
    let before = s.clone();
    let s = s.replace(
        ['\u{2010}', '\u{2011}', '\u{2012}', '\u{2013}', '\u{2014}'],
        "-",
    );
    if s != before {
        ctx.name.add_warning(warnings::HOMOGLYHPS);
    }
    s
}

// ---- Step 13: replaceHomoglyphs — STUBBED ----

/// Java `StripAndStash.replaceHomoglyphs` (StripAndStash.java:839-854) delegates to
/// `UnicodeUtils.containsHomoglyphs`/`replaceHomoglyphs`, which load a ~175-line
/// `unicode/homoglyphs.txt` resource at static-init time into a codepoint -> canonical-char
/// map (`UnicodeUtils.java:58-139`). Porting that table is a sizeable sub-project of its
/// own — `crate::unicode`'s own module doc already documents this exact deferral ("Only
/// `normalizeQuotes` is ported here; the homoglyph-replacement table … is intentionally NOT
/// ported"). Per this task's brief, STUBBED as a documented no-op: this step only ever sets
/// the (non-gated) `HOMOGLYHPS` warning and normalises the working string for RARE
/// non-Latin-lookalike input — it does not touch any of the 10 downstream-independent gate
/// fields, so leaving it a no-op does not affect this slice's gate. Port the real table in
/// a later slice, alongside `unicode.rs`'s existing deferral note.
fn replace_homoglyphs(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// ---- Step 14: repairWin1252Artefacts (structural — no Pattern; shared w/ stripAuthorshipMarkers) ----

/// Java `StripAndStash.repairWin1252Artefacts(ParsedName, String)`
/// (StripAndStash.java:856-876). Win-1252 -> UTF-8 transcription artefacts the homoglyph
/// table doesn't cover (e.g. "Plesn¡k" should read as "Plesnik") — maps a small set of
/// high-bit punctuation characters to their Latin look-alikes, flagging `HOMOGLYHPS` when
/// anything changed (spot-checked against the Java CLI oracle: "Aus bus Plesn¡k, 1900" ->
/// `authors=["Plesnik"]`, warning "homoglyphs replaced"). Java's real signature takes
/// `ParsedName` directly (shared with the not-yet-ported `stripAuthorshipMarkers`
/// auxiliary-authorship path) — ported with that same signature here (`_name` suffix) so
/// that future call site can reuse it directly; `repair_win1252_artefacts` below is the
/// uniform `(ctx, s)`-shaped wrapper `run`'s dispatcher needs.
fn repair_win1252_artefacts_name(name: &mut ParsedName, s: String) -> String {
    if s.contains('\u{00A1}')
        || s.contains('\u{00A2}')
        || s.contains('\u{00A3}')
        || s.contains('\u{201A}')
        || s.contains('\u{201E}')
        || s.contains('\u{2030}')
    {
        let before = s.clone();
        let s = s
            .replace('\u{00A1}', "i")
            .replace('\u{00A2}', "c")
            .replace('\u{00A3}', "L")
            .replace('\u{201A}', "e")
            .replace('\u{201E}', "a")
            .replace('\u{2030}', "e");
        if s != before {
            name.add_warning(warnings::HOMOGLYHPS);
        }
        s
    } else {
        s
    }
}

fn repair_win1252_artefacts(ctx: &mut ParseContext, s: String) -> String {
    repair_win1252_artefacts_name(&mut ctx.name, s)
}

// ---- Step 15: normaliseDoubleUnderscores ----

/// Java DOUBLE_UNDERSCORE (StripAndStash.java:328): `_{2,}`, no flags, no shorthand atoms
/// at all -> nothing to scope, ported verbatim.
static DOUBLE_UNDERSCORE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"_{2,}").unwrap());

/// Java `StripAndStash.normaliseDoubleUnderscores` (StripAndStash.java:878-885). Runs of 2+
/// underscores between letters ("Pseudocercospora__dendrobii") collapse to a single space
/// (spot-checked against the Java CLI oracle: parses clean as genus=Pseudocercospora,
/// specificEpithet=dendrobii, no warning — Java's method itself never calls `addWarning`).
fn normalise_double_underscores(_ctx: &mut ParseContext, s: String) -> String {
    if s.contains("__") {
        java_trim(&DOUBLE_UNDERSCORE.replace_all(&s, " ")).to_string()
    } else {
        s
    }
}

// ---- Step 16: stashTrailingOtuCode ----

/// Java TRAILING_OTU_CODE (StripAndStash.java:187-188): `\s+([A-Z0-9]{3,}_\d{3,})$`, no
/// flags. Has `\s`/`\d` (the `[A-Z0-9]` range is explicit, unaffected either way) -> whole
/// pattern ASCII-scoped (`$` left outside the wrap, per convention).
static TRAILING_OTU_CODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\s+([A-Z0-9]{3,}_\d{3,}))$").unwrap());

/// Java `StripAndStash.stashTrailingOtuCode` (StripAndStash.java:887-898). Strips a
/// trailing OTU-code identifier ("Oxalis barrelieri XXZ_21243") into
/// `ctx.pending_unparsed` (first-writer-wins) so the name portion still parses normally —
/// spot-checked against the Java CLI oracle (`unparsed="XXZ_21243"`).
fn stash_trailing_otu_code(ctx: &mut ParseContext, s: String) -> String {
    if s.contains(' ') && ctx.pending_unparsed.is_none() {
        if let Some(caps) = TRAILING_OTU_CODE.captures(&s) {
            let code = caps.get(1).unwrap().as_str().to_string();
            let whole = caps.get(0).unwrap();
            let prefix = java_trim(&s[..whole.start()]).to_string();
            ctx.set_pending_unparsed(&code);
            return prefix;
        }
    }
    s
}

// ---- Step 17: stripSerovarSerotype ----

/// Java SEROVAR_TEST (StripAndStash.java:349-350): `.*\b(?:serotype|serovar)\b.*`,
/// `Pattern.CASE_INSENSITIVE`. Has `\b` (x2), unescaped wildcard `.*` bookends -> only `\b`
/// ASCII-scoped. `.matches()` on `.*CORE.*` -> restructured to core-only `is_match`.
static SEROVAR_TEST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?-u:\b)(?:serotype|serovar)(?-u:\b)").unwrap());

/// Java SEROVAR_PAREN (StripAndStash.java:189-191):
/// `\s*\(\s*(?:serotype|serovar)\s+[^)]+\)\s*\.?\s*$`, `Pattern.CASE_INSENSITIVE`. Has `\s`
/// (ASCII-scoped individually) and a custom negated class `[^)]` — NOT a predefined
/// shorthand class, so Java's `UNICODE_CHARACTER_CLASS`/ASCII distinction never applies to
/// it at all (it always matches "any code point but `)`" in Java, flag or no flag) and it
/// must stay OUTSIDE any `(?-u:…)` scoping. (`[^)]` inside `(?-u:…)` also happens to be
/// independently rejected by `regex-syntax` — "pattern can match invalid UTF-8", since a
/// negated class in byte/ASCII mode could match a lone continuation byte of a multi-byte
/// UTF-8 sequence — discovered when this pattern was first run, not anticipated from the
/// flag rule alone; leaving `[^)]` unscoped, as Java semantics require anyway, sidesteps
/// that too.) So: atom-only `\s` scoping, not the whole-pattern-wrap this port's rule would
/// otherwise prefer for a `\p{...}`-free, wildcard-free pattern.
static SEROVAR_PAREN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?-u:\s*)\((?-u:\s*)(?:serotype|serovar)(?-u:\s+)[^)]+\)(?-u:\s*)\.?(?-u:\s*)$",
    )
    .unwrap()
});

/// Java SEROVAR_BARE (StripAndStash.java:192-194):
/// `\s+(?:serotype|serovar)\s+\S+(?:\s+(?:str\.?|strain)\s+\S+)?\s*\.?\s*$`,
/// `Pattern.CASE_INSENSITIVE`. Has `\s` (ASCII-scoped individually) and `\S`, the NEGATION
/// of the predefined `\s` shorthand. Unlike positive `\s`/`\d`/`\w`, a negated predefined
/// class (`\S`/`\D`/`\W`) genuinely can't be `(?-u:…)`-scoped in the `regex` crate at all —
/// same "matches invalid UTF-8" rejection as a negated custom class (see `SEROVAR_PAREN`
/// above). Fix: spell out Java's ASCII `\S` as its literal definition, NEGATED as an
/// ordinary (Unicode-default-scope) custom class — `[^\t\n\x0B\f\r ]` — which is exactly
/// "any code point that isn't one of these 6 ASCII whitespace chars", identical to what
/// Java's ASCII `\S` matches over full Unicode text; no `(?-u:…)` needed or wanted for it.
static SEROVAR_BARE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?-u:\s+)(?:serotype|serovar)(?-u:\s+)[^\t\n\x0B\f\r ]+(?:(?-u:\s+)(?:str\.?|strain)(?-u:\s+)[^\t\n\x0B\f\r ]+)?(?-u:\s*)\.?(?-u:\s*)$",
    )
    .unwrap()
});

/// Java `StripAndStash.stripSerovarSerotype` (StripAndStash.java:900-921). Bacterial
/// serovar/serotype/strain annotations on a binomial are sub-species-level epidemiological
/// designators, not formal taxonomic ranks — stripped silently so the underlying binomial
/// parses cleanly, e.g. "Leptospira interrogans serovar Fugis" -> "Leptospira interrogans",
/// "Aggregatibacter actinomycetemcomitans serotype d str. SA508" ->
/// "Aggregatibacter actinomycetemcomitans" (both spot-checked against the Java CLI oracle:
/// clean binomials, no warnings). `SEROVAR_PAREN` and `SEROVAR_BARE` both run in sequence
/// (not else-if), the second against the (possibly already-shortened) result of the first.
fn strip_serovar_serotype(_ctx: &mut ParseContext, mut s: String) -> String {
    if SEROVAR_TEST.is_match(&s) {
        if let Some(m) = SEROVAR_PAREN.find(&s) {
            s = java_trim(&s[..m.start()]).to_string();
        }
        if let Some(m) = SEROVAR_BARE.find(&s) {
            s = java_trim(&s[..m.start()]).to_string();
        }
    }
    s
}

// ---- Step 18: stripAngleBracketAuthorship ----

/// Java ANGLE_BRACKET_AUTHORSHIP (StripAndStash.java:195-197):
/// `\s+<\s*(\p{Lu}[^>]*\s[^>]*)>\s*$`, `Pattern.UNICODE_CHARACTER_CLASS` -> keep default
/// Unicode, ported verbatim.
static ANGLE_BRACKET_AUTHORSHIP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s+<\s*(\p{Lu}[^>]*\s[^>]*)>\s*$").unwrap());

/// Java `StripAndStash.stripAngleBracketAuthorship` (StripAndStash.java:923-938). An
/// angle-bracketed authorship placeholder ("Doradidae <Unspecified Agent>") — the
/// bracketed text starts with a capital and contains spaces (i.e. isn't an HTML tag) — is
/// stripped, flagging `AUTHORSHIP_REMOVED` + `UNUSUAL_CHARACTERS` and marking doubtful so
/// callers know the authorship couldn't be parsed (spot-checked against the Java CLI
/// oracle: `uninomial="Doradidae"`, `doubtful=true`,
/// `warnings=["authorship placeholder removed","unusual characters"]`).
fn strip_angle_bracket_authorship(ctx: &mut ParseContext, s: String) -> String {
    if s.contains('<') {
        if let Some(m) = ANGLE_BRACKET_AUTHORSHIP.find(&s) {
            ctx.name.add_warning(warnings::AUTHORSHIP_REMOVED);
            ctx.name.add_warning(warnings::UNUSUAL_CHARACTERS);
            ctx.name.doubtful = true;
            return java_trim(&s[..m.start()]).to_string();
        }
    }
    s
}

// ---- Step 19: stripHtml ----

/// Java HTML_TAG (StripAndStash.java:329): `<[^>]+>`, no flags, no shorthand atoms at all
/// -> nothing to scope, ported verbatim.
static HTML_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());

/// Java MULTI_SPACE (StripAndStash.java:289): `\s{2,}`, no flags. Has `\s`, no `\p{...}`
/// -> whole-wrap.
static MULTI_SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?-u:\s{2,})").unwrap());

/// Java `StripAndStash.stripHtml` (StripAndStash.java:940-961). Strips HTML tags (keeping
/// their text content, so "<i>sensu</i> Fabricius, 1780" becomes "sensu Fabricius, 1780"
/// and is picked up as a taxonomic note by the normal note handling downstream) and decodes
/// 4 basic HTML entities, flagging `XML_TAGS`/`HTML_ENTITIES` respectively when either
/// actually fired (both spot-checked against the Java CLI oracle). The final
/// whitespace-cleanup (collapse runs to one space + trim) always runs whenever the outer
/// `<`/`&` guard is true, even when neither replacement changed anything.
fn strip_html(ctx: &mut ParseContext, s: String) -> String {
    if s.contains('<') || s.contains('&') {
        let before_tags = s.clone();
        let mut s = HTML_TAG.replace_all(&s, "").into_owned();
        if s != before_tags {
            ctx.name.add_warning(warnings::XML_TAGS);
        }
        let before_entities = s.clone();
        s = s
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&nbsp;", " ");
        if s != before_entities {
            ctx.name.add_warning(warnings::HTML_ENTITIES);
        }
        return java_trim(&MULTI_SPACE.replace_all(&s, " ")).to_string();
    }
    s
}

// ---------------------------------------------------------------------------------
// Batch 2 (steps 20-30): candidatus, cultivar Group/grex/quoted, extinct, t.infr.,
// doubtful-genus brackets, sic/corrig, synonym bracket, bracketed + bare nom-note.
// Ported (Phase 1 Slice 2, batch 2b). This batch introduces the first `Rank` variants
// beyond the Unranked/Species/Subspecies trio (`model::enums::Rank`'s own STUB doc
// comment has the details) — `rank`/`code` are shared (B) fields also written by later,
// not-yet-ported stages, so they're not part of THIS slice's gate, but are set here
// faithfully anyway since later slices consume them.
// ---------------------------------------------------------------------------------

/// Java WHITESPACE (StripAndStash.java:288): `\s+`, no flags. Has `\s`, no `\p{...}` ->
/// whole-wrap. Distinct from batch 1's `MULTI_SPACE` (`\s{2,}`, only 2+ runs collapse) —
/// this pattern ALSO normalises a single non-space whitespace char (tab, newline, …) to a
/// plain space, which `MULTI_SPACE` alone would leave untouched. Shared by
/// `strip_extinct_dagger` (step 24), `strip_sic_and_corrig` (step 27, both the
/// SIC_WITH_COMMENT inner-squish — replacement `""`, not `" "` — and the CORRIG collapse),
/// and `normalise_nom_note` (steps 29/30).
static WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?-u:\s+)").unwrap());

/// Java's repeated `WHITESPACE.matcher(x).replaceAll(" ").trim()` idiom: collapse every
/// run of (ASCII) whitespace to a single space, then `java_trim`.
fn collapse_whitespace(s: &str) -> String {
    java_trim(&WHITESPACE.replace_all(s, " ")).to_string()
}

// ---- Step 20: stripCandidatus ----

/// Java CANDIDATUS_PREFIX (StripAndStash.java:198-199): `^["']?(?:Candidatus|Ca\.)\s+`,
/// `Pattern.CASE_INSENSITIVE`. Has `\s`, no `\p{...}`, no unescaped wildcard (only an
/// escaped `\.`) -> whole pattern ASCII-scoped (leading `^` left outside the wrap, same
/// convention as leaving a trailing `$` outside — anchors aren't `u`-sensitive either way).
static CANDIDATUS_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)^(?-u:["']?(?:Candidatus|Ca\.)\s+)"#).unwrap());

/// Java `StripAndStash.stripCandidatus` (StripAndStash.java:963-973). A leading
/// "Candidatus "/"Ca. " prefix (optionally quoted, e.g. `"Candidatus Something`) marks a
/// provisional bacterial taxon name under the *Candidatus* category: stashes
/// `candidatus = true` and `code = BACTERIAL`, and strips the prefix — plus a trailing
/// quote character, if one is present, regardless of whether it matches the leading quote
/// (Java doesn't check for a match, just presence; ported literally) — from the working
/// string.
fn strip_candidatus(ctx: &mut ParseContext, s: String) -> String {
    if let Some(m) = CANDIDATUS_PREFIX.find(&s) {
        ctx.name.candidatus = true;
        ctx.name.code = Some(NomCode::Bacterial);
        let mut rest = s[m.end()..].to_string();
        if rest.ends_with('"') || rest.ends_with('\'') {
            rest.pop();
        }
        return rest;
    }
    s
}

// ---- Step 21: normaliseHortExPlaceholder ----

/// Java CV_EX (StripAndStash.java:332): `\bcv\.(?=\s+ex\s+)`, no flags (ASCII `\s`/`\b` in
/// Java). Trailing lookahead (not consumed, so the following " ex " text survives
/// untouched for a possible later match too) -> `fancy_regex`. `fancy_regex` has no ASCII
/// mode at all -> `\s` spelled out as the literal ASCII whitespace set; `\b` is left as
/// `fancy_regex`'s only option (Unicode word-boundary), the same forced, vanishingly-rare
/// divergence documented at `STRAIN_DESIGNATION` in batch 1.
static CV_EX: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"\bcv\.(?=[ \t\n\x0B\f\r]+ex[ \t\n\x0B\f\r]+)").unwrap());

/// Java HORT_EX (StripAndStash.java:308): `\bHort\.(?=\s+ex\s+)`, no flags — note the
/// literal capital "Hort" (NOT case-insensitive). Same fancy_regex/ASCII-whitespace
/// reasoning as `CV_EX`. Shared with the not-yet-ported `stripAuthorshipMarkers`.
static HORT_EX: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"\bHort\.(?=[ \t\n\x0B\f\r]+ex[ \t\n\x0B\f\r]+)").unwrap());

/// Java HORTUS_EX (StripAndStash.java:309): `\bhortus[a]?\b(?=\s+ex\s+)`, no flags. Same
/// fancy_regex/ASCII-whitespace reasoning as `CV_EX`. Shared with the not-yet-ported
/// `stripAuthorshipMarkers`.
static HORTUS_EX: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(r"\bhortus[a]?\b(?=[ \t\n\x0B\f\r]+ex[ \t\n\x0B\f\r]+)").unwrap()
});

/// Java HT_MARKER (StripAndStash.java:333): `\bht\.`, no flags. No lookaround or
/// backreference -> plain `regex` crate. Has `\b`, no `\p{...}`, no unescaped wildcard ->
/// whole-wrap.
static HT_MARKER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?-u:\bht\.)").unwrap());

/// Java `StripAndStash.normaliseHortExPlaceholder` (StripAndStash.java:975-988).
/// Working-string-only (no `ctx.name` mutation, no warnings): normalises every spelling
/// of the horticultural "ex" placeholder author — "cv. ex", "Hort. ex", "hortus(a) ex",
/// and the bare "ht." abbreviation — to the canonical lower-case "hort.", so the
/// downstream authorship parser recognises one consistent marker regardless of which
/// variant the source data used. Four SEQUENTIAL replacements, each against the
/// (possibly already-rewritten) result of the previous one.
fn normalise_hort_ex_placeholder(_ctx: &mut ParseContext, s: String) -> String {
    let s = fancy_replace_all(&CV_EX, &s, |_| "hort.".to_string());
    let s = fancy_replace_all(&HORT_EX, &s, |_| "hort.".to_string());
    let s = fancy_replace_all(&HORTUS_EX, &s, |_| "hort.".to_string());
    HT_MARKER.replace_all(&s, "hort.").into_owned()
}

// ---- Step 22: stripCultivarGroupGrex ----

/// Java CULTIVAR_GROUP_GREX (StripAndStash.java:200-202),
/// `Pattern.UNICODE_CHARACTER_CLASS` -> keep default Unicode, ported verbatim.
static CULTIVAR_GROUP_GREX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s+([\p{Lu}][\p{L}]+(?:\s+[\p{Lu}][\p{L}]+)*)\s+(Group|grex|gx)\s*$").unwrap()
});

/// Java `StripAndStash.stripCultivarGroupGrex` (StripAndStash.java:990-1002). A trailing
/// "... CapWord(s) (Group|grex|gx)" names a Cultivar Group or grex rather than a single
/// cultivar: the capitalised epithet sequence becomes `cultivarEpithet`, `code` is pinned
/// to CULTIVARS, and `rank` to CULTIVAR_GROUP for the exact (case-sensitive) literal
/// "Group" or GREX for either "grex" or "gx" — matching Java's `"Group".equals(...)`
/// ternary, which folds every OTHER alternative the pattern can capture to GREX.
fn strip_cultivar_group_grex(ctx: &mut ParseContext, s: String) -> String {
    if let Some(caps) = CULTIVAR_GROUP_GREX.captures(&s) {
        ctx.name.cultivar_epithet = Some(java_trim(caps.get(1).unwrap().as_str()).to_string());
        ctx.name.code = Some(NomCode::Cultivars);
        ctx.name.rank = if caps.get(2).unwrap().as_str() == "Group" {
            Rank::CultivarGroup
        } else {
            Rank::Grex
        };
        let whole = caps.get(0).unwrap();
        return java_trim(&s[..whole.start()]).to_string();
    }
    s
}

// ---- Step 23: stripQuotedCultivar ----

/// Java QUOTED_CULTIVAR_END (StripAndStash.java:203-204):
/// `\s+(cv\.?\s+)?(['"])([^'"]+)\2\s*$`, no flags (ASCII `\s` in Java). BACKREFERENCE
/// (`\2`, the closing quote must match the opening one) -> `fancy_regex`. `[^'"]` is a
/// NEGATED custom class; Java's `UNICODE_CHARACTER_CLASS` flag never governs a
/// literal/custom class anyway, and `fancy_regex` (always Unicode text, no byte/ASCII
/// mode) has no analogous "matches invalid UTF-8" restriction to worry about here either
/// way. `fancy_regex` has no ASCII mode for `\s` -> spelled out as the literal ASCII
/// whitespace set. Group 1 = optional "cv. " prefix, group 2 = quote char, group 3 =
/// cultivar epithet content.
static QUOTED_CULTIVAR_END: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(r#"[ \t\n\x0B\f\r]+(cv\.?[ \t\n\x0B\f\r]+)?(['"])([^'"]+)\2[ \t\n\x0B\f\r]*$"#)
        .unwrap()
});

/// Java RANK_MARKER_SUFFIX (StripAndStash.java:336-337):
/// `.*\b(?:sp|spec|subsp|ssp|var|form|f)\.?$`, no flags, called via `.matches()`.
/// RESTRUCTURED: under `.matches()` the leading `.*` imposes no real constraint (it can
/// always stretch to cover whatever precedes the `$`-anchored suffix), so an unanchored
/// `is_match` on the suffix alone is existence-equivalent — the same class of
/// restructuring as the `.*CORE.*` -> `CORE` cases elsewhere in this file, just one-sided.
/// Has `\b`, no `\p{...}`, and (after dropping the leading `.*`) no remaining unescaped
/// wildcard -> whole-wrap (`$` left outside per convention).
static RANK_MARKER_SUFFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\b(?:sp|spec|subsp|ssp|var|form|f)\.?)$").unwrap());

/// Java QUOTED_CULTIVAR_MID (StripAndStash.java:205-207):
/// `\s+(?:cv\.?\s+)?(['"])([^'"]+)\1(\s+[\p{Lu}].*)$`, `Pattern.UNICODE_CHARACTER_CLASS`
/// -> keep default Unicode `\s` (fancy_regex's own default, so no ASCII spelling-out
/// needed here, unlike `QUOTED_CULTIVAR_END`). BACKREFERENCE (`\1`) -> `fancy_regex`.
/// Group 1 = quote char, group 2 = cultivar epithet content, group 3 = the trailing
/// author span (kept verbatim for splicing back onto the name part).
static QUOTED_CULTIVAR_MID: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(r#"\s+(?:cv\.?\s+)?(['"])([^'"]+)\1(\s+[\p{Lu}].*)$"#).unwrap()
});

/// Java AUTHOR_START (StripAndStash.java:283-285):
/// `^([\p{Lu}][\p{Ll}]+(?:\s+[\p{Ll}]+)?)\s+([\p{Lu}][\p{L}.]+.*)$`,
/// `Pattern.UNICODE_CHARACTER_CLASS` -> keep default Unicode, ported verbatim (no
/// backreference, no lookaround -> plain `regex` crate).
static AUTHOR_START: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([\p{Lu}][\p{Ll}]+(?:\s+[\p{Ll}]+)?)\s+([\p{Lu}][\p{L}.]+.*)$").unwrap()
});

/// Java `StripAndStash.findAuthorStart` (StripAndStash.java:1650-1657): the byte offset of
/// group 2 (the author span) within a "Genus[ species] Author..." prefix, or `None` when
/// the prefix doesn't have that shape (Java: `int`, -1 sentinel). The pattern's own `^…$`
/// anchors make `.captures()` here equivalent to Java's `.matches()`.
fn find_author_start(prefix: &str) -> Option<usize> {
    AUTHOR_START
        .captures(prefix)
        .map(|caps| caps.get(2).unwrap().start())
}

/// Java QUOTED_CULTIVAR_OPEN (StripAndStash.java:217-219):
/// `\s+(cv\.?\s+)?(['"])(\p{Ll}[\p{Ll} ]*)\s*$`, `Pattern.UNICODE_CHARACTER_CLASS` -> keep
/// default Unicode. No backreference (this is the UNCLOSED-quote fallback: content runs to
/// end of string, no closing quote required) and no lookaround -> plain `regex` crate.
/// Group 1 = optional "cv. " prefix, group 2 = quote char, group 3 = epithet content
/// (lowercase letters/spaces only, so this never swallows a capitalised or
/// punctuation-carrying apostrophe-author).
static QUOTED_CULTIVAR_OPEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\s+(cv\.?\s+)?(['"])(\p{Ll}[\p{Ll} ]*)\s*$"#).unwrap());

/// Java TRAILING_CV (StripAndStash.java:334): `\s+cv\.?\s*$`, no flags. Has `\s` (x2), no
/// `\p{...}`, no unescaped wildcard -> whole-wrap (`$` left outside).
static TRAILING_CV: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?-u:\s+cv\.?\s*)$").unwrap());

/// Java CV_MARKER (StripAndStash.java:335): `\s+cv\.?(?=\s|$)`, no flags (ASCII `\s`).
/// Trailing lookahead (boundary not consumed) -> `fancy_regex`, ASCII whitespace spelled
/// out (no ASCII mode in `fancy_regex`).
static CV_MARKER: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"[ \t\n\x0B\f\r]+cv\.?(?=[ \t\n\x0B\f\r]|$)").unwrap());

/// Java `StripAndStash.stripQuotedCultivar` (StripAndStash.java:1004-1068). A quoted
/// cultivar epithet — " 'Name'" / " \"Name\"", optionally preceded by an explicit "cv."
/// marker — becomes `cultivarEpithet` (`code = CULTIVARS`, `rank = CULTIVAR`). Three
/// shapes, tried in order (an `if`/else-if/else-if chain in Java; ported here as
/// sequential checks with early returns for the first two):
///   1. **At end of input** (`QUOTED_CULTIVAR_END`): "Acer campestre 'Elsrijk'" ->
///      cultivarEpithet "Elsrijk". Skipped when the quote is immediately preceded by a
///      bare rank marker with no explicit "cv." (`RANK_MARKER_SUFFIX` on the trimmed
///      preceding text) — that shape is a phrase name, not a cultivar.
///   2. **Mid-string, followed by an author span** (`QUOTED_CULTIVAR_MID`, tried only
///      when shape 1 didn't apply): "Acer campestre L. cv. 'Elsrijk' Broerse" ->
///      cultivarEpithet "Elsrijk", author "Broerse". Any species author preceding the
///      epithet is split off via `find_author_start` and stashed as
///      `ctx.pending_specific_author` (e.g. the "L." above) rather than left attached to
///      the binomial.
///   3. **Unclosed trailing quote** (`QUOTED_CULTIVAR_OPEN`, tried only when shapes 1 and
///      2 didn't apply): " 'albino" (opening quote, no closing one — common in
///      aquarium/horticultural trade lists), same rank-marker-prefix guard as shape 1.
///
/// All three spot-checked against the Java CLI oracle's `cultivarEpithet`/`rank`/`code`/
/// `specificAuthorship` output shape.
fn strip_quoted_cultivar(ctx: &mut ParseContext, mut s: String) -> String {
    let cm = QUOTED_CULTIVAR_END.captures(&s).ok().flatten();
    let cm_found = cm.is_some();
    let has_cv_marker = cm.as_ref().is_some_and(|c| c.get(1).is_some());
    let preceding = cm
        .as_ref()
        .map(|c| java_trim(&s[..c.get(0).unwrap().start()]).to_string());
    let is_rank_marker_prefix = !has_cv_marker
        && preceding
            .as_deref()
            .is_some_and(|p| RANK_MARKER_SUFFIX.is_match(p));
    if cm_found && !is_rank_marker_prefix {
        let caps = cm.unwrap();
        let match_start = caps.get(0).unwrap().start();
        let epithet = java_trim(caps.get(3).unwrap().as_str()).to_string();
        ctx.name.cultivar_epithet = Some(epithet);
        ctx.name.code = Some(NomCode::Cultivars);
        ctx.name.rank = Rank::Cultivar;
        s = java_trim(&s[..match_start]).to_string();
        s = java_trim(&TRAILING_CV.replace_all(&s, "")).to_string();
        return s;
    }

    if let Some(caps) = QUOTED_CULTIVAR_MID.captures(&s).ok().flatten() {
        let match_start = caps.get(0).unwrap().start();
        let epithet = java_trim(caps.get(2).unwrap().as_str()).to_string();
        let tail = caps.get(3).unwrap().as_str().to_string();
        let prefix = java_trim(&s[..match_start]).to_string();
        ctx.name.cultivar_epithet = Some(epithet);
        ctx.name.code = Some(NomCode::Cultivars);
        ctx.name.rank = Rank::Cultivar;
        let name_part = match find_author_start(&prefix) {
            Some(author_start) if author_start > 0 => {
                ctx.pending_specific_author = Some(java_trim(&prefix[author_start..]).to_string());
                java_trim(&prefix[..author_start]).to_string()
            }
            _ => prefix,
        };
        s = java_trim(&format!("{name_part}{tail}")).to_string();
        s = java_trim(&fancy_replace_all(&CV_MARKER, &s, |_| String::new())).to_string();
        return s;
    }

    if let Some(caps) = QUOTED_CULTIVAR_OPEN.captures(&s) {
        let has_cv = caps.get(1).is_some();
        let match_start = caps.get(0).unwrap().start();
        let epithet = java_trim(caps.get(3).unwrap().as_str()).to_string();
        let preceding_open = java_trim(&s[..match_start]).to_string();
        let is_rank_prefix = !has_cv && RANK_MARKER_SUFFIX.is_match(&preceding_open);
        if !is_rank_prefix {
            ctx.name.cultivar_epithet = Some(epithet);
            ctx.name.code = Some(NomCode::Cultivars);
            ctx.name.rank = Rank::Cultivar;
            s = java_trim(&s[..match_start]).to_string();
            s = java_trim(&TRAILING_CV.replace_all(&s, "")).to_string();
        }
    }
    s
}

// ---- Step 24: stripExtinctDagger ----

/// Java DAGGER (StripAndStash.java:330): `[†✝]`, no flags. A custom class holding
/// non-ASCII literals — Java's `UNICODE_CHARACTER_CLASS` flag never governs a
/// literal/custom class anyway (only predefined shorthand classes), and a non-ASCII
/// literal inside a `regex` crate `(?-u:…)` group is a hard parse error regardless (see
/// `IMPRINT_YEAR_QUOTED` in batch 1) — no shorthand atoms to scope here either, so this
/// ports verbatim with no scoping at all.
static DAGGER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[†✝]").unwrap());

/// Java `StripAndStash.stripExtinctDagger` (StripAndStash.java:1070-1077). An extinction
/// dagger ("†" or "✝") anywhere in the string — possibly more than one — marks the taxon
/// extinct; ALL occurrences are stripped (replaced by a space, then whitespace
/// collapsed+trimmed, so e.g. "Foo †bar" doesn't leave a glued "Foobar").
fn strip_extinct_dagger(ctx: &mut ParseContext, s: String) -> String {
    if s.contains('†') || s.contains('✝') {
        ctx.name.extinct = true;
        let no_dagger = DAGGER.replace_all(&s, " ");
        return collapse_whitespace(&no_dagger);
    }
    s
}

// ---- Step 25: stripTinfrMarker ----

/// Java TINFR_MARKER (StripAndStash.java:331): `\b[tT]\.?\s*infr\.?\s+`, no flags. Has
/// `\b`/`\s` (x2), no `\p{...}`, no unescaped wildcard (dots are escaped) -> whole-wrap.
static TINFR_MARKER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\b[tT]\.?\s*infr\.?\s+)").unwrap());

/// Java `StripAndStash.stripTinfrMarker` (StripAndStash.java:1079-1087). The Hieracium
/// "t.infr." infraspecific-epithet notation (e.g. "Hieracium alpinum t.infr. foobarum")
/// is stripped so downstream tokenisation sees a plain space-separated word run instead
/// of the marker — spot-checked against the Java CLI oracle: with a genus+species BEFORE
/// the marker, the trailing word is later read as a rankless `INFRASPECIFIC_NAME`
/// epithet ("Hieracium alpinum t.infr. foobarum" -> infraspecificEpithet "foobarum");
/// with only a bare genus before it, the trailing word instead reads as an ordinary
/// specific epithet ("Hieracium t.infr. foobarum" -> specificEpithet "foobarum") — this
/// step itself does neither classification, it only removes the marker text.
fn strip_tinfr_marker(_ctx: &mut ParseContext, s: String) -> String {
    if s.contains("infr") {
        return TINFR_MARKER.replace_all(&s, "").into_owned();
    }
    s
}

// ---- Step 26: stripDoubtfulGenusBrackets ----

/// Java DOUBTFUL_GENUS_BRACKET (StripAndStash.java:220-222):
/// `^\[\s*([\p{Lu}][\p{L}\-]+)\s*\](\s|$)`, `Pattern.UNICODE_CHARACTER_CLASS` -> keep
/// default Unicode, ported verbatim.
static DOUBTFUL_GENUS_BRACKET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[\s*([\p{Lu}][\p{L}\-]+)\s*\](\s|$)").unwrap());

/// Java `StripAndStash.stripDoubtfulGenusBrackets` (StripAndStash.java:1089-1099). A
/// leading bracketed genus ("[Acontia] chia ..." or just "[Dexia]") marks the genus
/// assignment as doubtful — the source questions whether the specimen truly belongs to
/// that genus. Drops the brackets (keeping the genus text and the single separator
/// char/end-of-string that followed them), flags doubtful + `DOUBTFUL_GENUS`.
fn strip_doubtful_genus_brackets(ctx: &mut ParseContext, s: String) -> String {
    if let Some(caps) = DOUBTFUL_GENUS_BRACKET.captures(&s) {
        ctx.name.doubtful = true;
        ctx.name.add_warning(warnings::DOUBTFUL_GENUS);
        let whole = caps.get(0).unwrap();
        let g1 = caps.get(1).unwrap().as_str();
        let g2 = caps.get(2).unwrap().as_str();
        return java_trim(&format!("{g1}{g2}{}", &s[whole.end()..])).to_string();
    }
    s
}

// ---- Step 27: stripSicAndCorrig ----

/// Java SIC_WITH_COMMENT (StripAndStash.java:32-33): `\s*[\(\[]\s*sic\s*,([^)\]]+)[\)\]]`,
/// no flags. `[^)\]]` is a NEGATED custom class -> must stay OUTSIDE any `(?-u:…)` (a
/// negated class in byte/ASCII mode could match a stray continuation byte of a multi-byte
/// UTF-8 sequence, rejected by `regex-syntax` as "pattern can match invalid UTF-8" — the
/// `SEROVAR_PAREN` precedent from batch 1) -> atom-only `\s` scoping, not the whole-wrap
/// this port's rule would otherwise prefer. No backreference, no lookaround -> plain
/// `regex` crate.
static SIC_WITH_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\s*)[(\[](?-u:\s*)sic(?-u:\s*),([^)\]]+)[)\]]").unwrap());

/// Java SIC (StripAndStash.java:29-30): `\s*[\(\[]\s*sic\s*!?\s*[)\]]`, no flags. Has `\s`
/// (x4), no `\p{...}`, no unescaped wildcard, and the custom classes `[(\[]`/`[)\]]` are
/// POSITIVE (list specific ASCII literals, not negated) so they're safe inside a
/// `(?-u:…)` group (unlike `SIC_WITH_COMMENT`'s negated class above) -> whole-pattern
/// ASCII-scope.
static SIC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\s*[(\[]\s*sic\s*!?\s*[)\]])").unwrap());

/// Java CORRIG (StripAndStash.java:35-36):
/// `\s*[\(\[]\s*corrig\.?\s*[\)\]]|(?<=\s)corrig\.?(?=\s|$)`, no flags. The bracketed
/// alternative alone needs no lookaround, but it's OR'd with a bare alternative using
/// BOTH a lookbehind and a lookahead -> the whole pattern needs `fancy_regex`. No ASCII
/// mode in `fancy_regex` -> `\s` spelled out as the literal ASCII whitespace set. Same
/// pattern text as the Phase 0 spike (`regexes::CORRIG_FANCY`) that first identified the
/// call-site harness below — re-defined here (rather than imported) to stay
/// self-contained like every other pattern in this file, and because `regexes::SIC`/
/// `CORRIG` are NOT ASCII-scoped per this port's flag rule (they predate it), so reusing
/// them directly would introduce a `\s`-scope divergence from Java.
static CORRIG: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(
        r"[ \t\n\x0B\f\r]*[(\[][ \t\n\x0B\f\r]*corrig\.?[ \t\n\x0B\f\r]*[)\]]|(?<=[ \t\n\x0B\f\r])corrig\.?(?=[ \t\n\x0B\f\r]|$)",
    )
    .unwrap()
});

/// Java `StripAndStash.stripSicAndCorrig` (StripAndStash.java:1101-1124). Three
/// SEQUENTIAL checks (not else-if — each runs against the possibly-already-updated `s`,
/// and a later one can overwrite an earlier one's flag if the input somehow carries both):
///   1. `[sic, comment]` / `(sic, comment)` -> `originalSpelling = true`; the inner
///      comment (trimmed, then had ALL internal whitespace removed — not just collapsed —
///      matching Java's `WHITESPACE.replaceAll("")`, replacement `""` not `" "`) is
///      parked in `ctx.pending_unparsed` as `"(sic,<comment>)"`.
///   2. Plain `[sic]` / `(sic)` / `[sic!]` (no inner comma) -> `originalSpelling = true`.
///   3. `corrig.` / `(corrig.)` / `[corrig.]` -> `originalSpelling = false`. Applied with
///      the Java call-site harness (StripAndStash.java:1117-1121, and its twin
///      `stripAuthorshipMarkers` call at line 432): a LEADING-SPACE PAD before matching
///      (so a leading/standalone "corrig." is stripped too — `CORRIG`'s bare alternative
///      needs an actual preceding whitespace char, which the pad supplies at position 0),
///      then a whitespace-collapse + trim after removing every match — the exact lesson
///      `regexes::strip_corrig_fancy` documents at length (Phase 0).
fn strip_sic_and_corrig(ctx: &mut ParseContext, mut s: String) -> String {
    if let Some(caps) = SIC_WITH_COMMENT.captures(&s) {
        ctx.name.original_spelling = Some(true);
        let inner = java_trim(caps.get(1).unwrap().as_str()).to_string();
        let squished = WHITESPACE.replace_all(&inner, "").into_owned();
        ctx.set_pending_unparsed(&format!("(sic,{squished})"));
        let whole = caps.get(0).unwrap();
        let (start, end) = (whole.start(), whole.end());
        s = format!("{}{}", &s[..start], &s[end..]);
    }
    if let Some(m) = SIC.find(&s) {
        ctx.name.original_spelling = Some(true);
        let (start, end) = (m.start(), m.end());
        s = format!("{}{}", &s[..start], &s[end..]);
    }
    let padded = format!(" {s}");
    if let Ok(Some(_)) = CORRIG.find(&padded) {
        ctx.name.original_spelling = Some(false);
        let stripped = fancy_replace_all(&CORRIG, &padded, |_| String::new());
        s = collapse_whitespace(&stripped);
    }
    s
}

// ---- Step 28: stashSynonymBracket ----

/// Java SYNONYM_BRACKET (StripAndStash.java:117-118): `\s*\[\s*=\s*[^\]]+\]\s*\.?\s*$`, no
/// flags. `[^\]]` is a negated custom class -> must stay outside `(?-u:…)` (same reasoning
/// as `SIC_WITH_COMMENT`) -> atom-only `\s` scoping.
static SYNONYM_BRACKET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?-u:\s*)\[(?-u:\s*)=(?-u:\s*)[^\]]+\](?-u:\s*)\.?(?-u:\s*)$").unwrap()
});

/// Java `StripAndStash.stashSynonymBracket` (StripAndStash.java:1126-1138). A trailing
/// synonymy reference in square brackets ("[= Grislea L. 1753]") is parked verbatim (with
/// surrounding whitespace trimmed) as `ctx.pending_unparsed` — first-writer-wins, so an
/// earlier strip step's stash (e.g. a trailing OTU code) takes precedence — flags
/// doubtful, and strips both the bracket AND any now-trailing comma(s) left dangling
/// before it (a `while` loop: Java strips one comma, re-trims, and repeats, UNLIKE
/// `strip_bracketed_nom_note`'s single `if`).
fn stash_synonym_bracket(ctx: &mut ParseContext, s: String) -> String {
    if let Some(m) = SYNONYM_BRACKET.find(&s) {
        let tail = java_trim(&s[m.start()..]).to_string();
        ctx.set_pending_unparsed(&tail);
        ctx.name.doubtful = true;
        let mut kept = java_trim(&s[..m.start()]).to_string();
        while kept.ends_with(',') {
            kept.pop();
            kept = java_trim(&kept).to_string();
        }
        return kept;
    }
    s
}

// ---- Shared helper: normaliseNomNote (used by steps 29 and 30) ----

/// Java ET_WORD (StripAndStash.java:290): `\bet\b`, no flags. Whole-wrap (has `\b`, no
/// `\p{...}`, no unescaped wildcard).
static ET_WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?-u:\bet\b)").unwrap());

/// Java AND_WORD (StripAndStash.java:291): `\band\b`, no flags. Whole-wrap.
static AND_WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?-u:\band\b)").unwrap());

/// Java SPACE_AROUND_DOT (StripAndStash.java:292): `\s*\.\s*`, no flags. Whole-wrap (the
/// `\.` is an escaped literal, not an unescaped wildcard).
static SPACE_AROUND_DOT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?-u:\s*\.\s*)").unwrap());

/// Java DOT_BEFORE_ALNUM (StripAndStash.java:293): `\.(?=[\p{L}\d])`, no flags. Trailing
/// lookahead (not consumed, so the letter/digit that follows the period survives
/// untouched, allowing a following period elsewhere to independently reuse the same
/// adjacency reasoning as `GREEK_MARKER` in batch 1) -> `fancy_regex`. `\d` is ASCII-only
/// in Java here (no `UNICODE_CHARACTER_CLASS`) -> spelled out as `[0-9]` (`fancy_regex`
/// has no ASCII mode); `\p{L}` always Unicode.
static DOT_BEFORE_ALNUM: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"\.(?=[\p{L}0-9])").unwrap());

/// Java SPACE_AROUND_AMP (StripAndStash.java:294): `\s*&\s*`, no flags. Whole-wrap.
static SPACE_AROUND_AMP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?-u:\s*&\s*)").unwrap());

/// Java `FULL_WORD_NOM_NOTES` (StripAndStash.java:406-407): `Set.of("correct", "error")`
/// — suffix words that are complete English/Latin forms; a nomenclatural note ending in
/// one of these (lower-cased) does NOT get a trailing dot appended.
const FULL_WORD_NOM_NOTES: &[&str] = &["correct", "error"];

/// Java `StripAndStash.normaliseNomNote` (StripAndStash.java:373-402). Canonical form for
/// nomenclatural notes, shared by `strip_bracketed_nom_note` (step 29) and
/// `strip_nom_note` (step 30): collapse whitespace, normalise "et"/"and" connectives to
/// "&", ensure a single space follows every interior period, add spaces around a bare
/// "&". Abbreviated notes ("nom. nud.", "Spec nov") get a closing dot if missing;
/// spelled-out "nomen …" forms have any trailing dot(s) stripped instead (sentence
/// punctuation, not part of the abbreviation); a note ending in a complete word
/// (`FULL_WORD_NOM_NOTES`) is left without an added dot either way.
fn normalise_nom_note(raw: &str) -> String {
    let mut s = collapse_whitespace(raw);
    let et_replaced = ET_WORD.replace_all(&s, "&");
    s = AND_WORD.replace_all(&et_replaced, "&").into_owned();
    s = SPACE_AROUND_DOT.replace_all(&s, ".").into_owned();
    s = fancy_replace_all(&DOT_BEFORE_ALNUM, &s, |_| ". ".to_string());
    s = SPACE_AROUND_AMP.replace_all(&s, " & ").into_owned();
    s = java_trim(&MULTI_SPACE.replace_all(&s, " ")).to_string();
    if s.len() >= 5 && s.as_bytes()[..5].eq_ignore_ascii_case(b"nomen") {
        while s.ends_with('.') {
            s.pop();
        }
        return java_trim(&s).to_string();
    }
    if !s.is_empty() && !s.ends_with('.') {
        let last_word = match s.rfind(' ') {
            Some(idx) => &s[idx + 1..],
            None => s.as_str(),
        };
        if !FULL_WORD_NOM_NOTES.contains(&last_word.to_lowercase().as_str()) {
            s.push('.');
        }
    }
    s
}

// ---- Step 29: stripBracketedNomNote ----

/// Java BRACKETED_NOM_NOTE (StripAndStash.java:61-63):
/// `\s*[\[\(]\s*((?:nom|comb|orth|typ)\b[^\]\)]*)[\]\)]\s*$`, `Pattern.CASE_INSENSITIVE`.
/// `[^\]\)]` is a negated custom class -> stays outside `(?-u:…)` -> atom-only `\s`/`\b`
/// scoping (not whole-wrap).
static BRACKETED_NOM_NOTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?-u:\s*)[\[\(](?-u:\s*)((?:nom|comb|orth|typ)(?-u:\b)[^\]\)]*)[\]\)](?-u:\s*)$",
    )
    .unwrap()
});

/// Java `StripAndStash.stripBracketedNomNote` (StripAndStash.java:1140-1150). A trailing
/// bracketed/parenthesised nom-note ("[nom. et typ. cons.]", "(nom. nud.)",
/// "[orth. error]") OVERWRITES `nomenclaturalNote` — Java calls the plain setter here
/// (`ctx.name.setNomenclaturalNote(...)`, no null-check/append), UNLIKE `strip_nom_note`
/// (step 30, later in run-order) which appends. Also strips at most ONE trailing comma
/// left dangling before the bracket — a single `if`, not the `while` loop
/// `stash_synonym_bracket` uses.
fn strip_bracketed_nom_note(ctx: &mut ParseContext, s: String) -> String {
    if let Some(caps) = BRACKETED_NOM_NOTE.captures(&s) {
        let raw = java_trim(caps.get(1).unwrap().as_str()).to_string();
        ctx.name.nomenclatural_note = Some(normalise_nom_note(&raw));
        let match_start = caps.get(0).unwrap().start();
        let mut kept = java_trim(&s[..match_start]).to_string();
        if kept.ends_with(',') {
            kept = java_trim(&kept[..kept.len() - 1]).to_string();
        }
        return kept;
    }
    s
}

// ---- Step 30: stripNomNote ----

// RULE: patterns on fancy_regex (backtracking) MUST keep Java's possessive/atomic
// quantifiers — only DROP possessives for patterns on the linear `regex` crate.
/// Java NOM_NOTE (StripAndStash.java:45-57), `Pattern.UNICODE_CHARACTER_CLASS` -> keep
/// default Unicode `\s`/`\b`/`\p{...}` throughout, ported near-verbatim. Internal
/// NEGATIVE LOOKAHEAD (`(?!\s+in\s+\p{Lu})`) in the first alternative AND a trailing
/// LOOKAHEAD boundary (`(?=$|,\s*non…|…)`) -> needs `fancy_regex` (the `regex` crate has
/// no lookaround at all). The one POSSESSIVE quantifier (`*+` on the first alternative's
/// inner repetition) is KEPT here as `*+` — UNLIKE every other possessive quantifier in
/// this port (see the RULE above, and `regexes.rs`'s module doc: those are all on the
/// linear `regex` crate, a linear-time automaton with no backtracking, where
/// possessive-vs-greedy can never change whether an overall match exists). `fancy_regex`
/// IS a backtracking engine, so on THIS pattern the possessive is load-bearing: without
/// it, an input with a long run of dotted lowercase words after "nomen"/"nom"/"comb"/
/// "orth" and no valid terminator (e.g. `" nomen " + "a.".repeat(20) + "#"`) makes the
/// greedy `*` re-partition the run exponentially before giving up, hitting
/// `fancy_regex`'s backtrack limit (proven: ~11ms and a `BacktrackLimitExceeded` no-match
/// with plain `*`, microseconds with `*+`). `fancy_regex` 0.14 parses `*+` natively
/// (compiling it to an atomic group), so restoring it is a straight verbatim port of
/// Java's own quantifier, not a restructuring. Java's own comment (StripAndStash.java:
/// 47-51) explains the captured run only ever consumes lowercase abbreviation words that
/// the trailing lookahead's own required content (whitespace/uppercase/comma) never
/// overlaps with, so the possessive changes nothing about WHICH strings match (the same
/// reasoning that lets Java itself use `*+` safely there) — it only forbids the
/// backtracking search that would otherwise explore every equivalent partition of the
/// run. Built via `concat!` (compile-time string concatenation) mirroring the Java
/// source's own `"..." + "..."` layout, one alternative per line, so it stays directly
/// diffable against StripAndStash.java line-by-line.
static NOM_NOTE: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(concat!(
        r"\s+(",
        r"(?i:nom|comb|orth|nomen)\b\.?(?:(?!\s+in\s+\p{Lu})[\s.&]*[a-z][a-z.]*)*+",
        r"|(?i:sp|spec|gen|fam|var|form)\b\.?\s*(?i:nov)\b\.?(?:\s+ined\b\.?)?(?:\s+(?i:sp|spec|gen|fam|var|form)\b\.?\s*(?i:nov)\b\.?(?:\s+ined\b\.?)?)*",
        r"|(?i:nov)\b\.?\s+(?i:sp|spec|gen|fam|var|form)\b\.?",
        r"|(?:in\s+obs\b\.?,?\s*)?pro\s+syn\b\.?",
        r")\s*(?=$|,\s*non(?:n\.?)?\b|,\s*nec\b|,\s*emend\b|,\s*sensu\b|,\s*auctt?\b|,\s*fide\b|\s+in\s+\p{Lu}|\s+\(.*\)\s*\.?\s*$|\s+\p{Lu})",
    ))
    .unwrap()
});

/// Java SP_NOV_PREFIX (StripAndStash.java:338-339): `(?i)^(?:sp|spec)\b\.?\s+nov.*`, no
/// `UNICODE_CHARACTER_CLASS`. Has `\b`/`\s` and an unescaped wildcard `.*` -> atom-only
/// scoping (not whole-wrap). Called via `.matches()` -> trailing `$` added.
static SP_NOV_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:sp|spec)(?-u:\b)\.?(?-u:\s+)nov.*$").unwrap());

/// Java SINGLE_TITLE_WORD (StripAndStash.java:340): `^[\p{Lu}][\p{Ll}]+$`, no flags.
/// `\p{Lu}`/`\p{Ll}` always Unicode, no ASCII atoms at all -> nothing to scope. Its own
/// `^…$` anchors make `.is_match()` here equivalent to Java's `.matches()`.
static SINGLE_TITLE_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{Lu}][\p{Ll}]+$").unwrap());

/// Java MANUSCRIPT_KEYWORD (StripAndStash.java:312-313):
/// `(?i).*\b(?:ined|ms|msc|unpublished)\b.*`, no `UNICODE_CHARACTER_CLASS`. RESTRUCTURED:
/// called via `.matches()` on a `.*CORE.*` shape -> equivalent to an unanchored `is_match`
/// on CORE alone (same restructuring as `GREEK_MARKER_TEST`/`SEROVAR_TEST` in batch 1).
/// `\b` (x2) ASCII-scoped. Shared with the not-yet-ported `stripAuthorshipMarkers` and
/// `stripManuscriptMarker` (batch 4) — defined once here, reusable file-wide.
static MANUSCRIPT_KEYWORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?-u:\b)(?:ined|ms|msc|unpublished)(?-u:\b)").unwrap());

/// Java NOM_NOTE_RANK_HINT (StripAndStash.java:223-225): `^(gen|fam|var|form|sp|spec)\b\.?`,
/// `Pattern.CASE_INSENSITIVE`. Has `\b`, no `\p{...}`, no unescaped wildcard -> whole
/// pattern ASCII-scoped (leading `^` left outside the wrap).
static NOM_NOTE_RANK_HINT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?-u:(gen|fam|var|form|sp|spec)\b\.?)").unwrap());

/// Java `StripAndStash.stripNomNote` (StripAndStash.java:1152-1195). The general
/// nom/comb/orth/nomen/sp.nov./pro-syn. keyword tail — the LAST-resort, most general
/// nomenclatural-note anchor (everything more specific was already peeled off by earlier
/// steps, e.g. `strip_bracketed_nom_note`). Unlike step 29, this APPENDS to
/// `nomenclaturalNote` (`ctx.name.add_nomenclatural_note`, matching Java's inline
/// `existing == null ? norm : existing + " " + norm`). Three further side effects, all
/// conditioned on the SAME captured `raw` (group 1, trimmed) and/or `before` (the
/// pre-match text, trimmed) — independent `if`s, not else-if:
///   - a bare "sp. nov."/"spec. nov." tail on an otherwise-single-Title-Word monomial
///     REPLACES the reconstructed working string with "`before` sp." (re-adding a bare
///     rank marker so the not-yet-ported indet/INFORMAL handling still recognises a
///     species-indet name, rather than leaving a bare uninomial with the marker fully
///     gone);
///   - a captured note containing "ined"/"ms"/"msc"/"unpublished" sets `manuscript = true`
///     (the note text itself already consumed the keyword, so the separate standalone
///     manuscript-marker step, batch 4, won't see it there anymore);
///   - when the name doesn't already carry a rank (`Rank::Unranked`), a gen/fam/var/form
///     prefix on `raw` pins the rank accordingly ("sp"/"spec" is left alone — SPECIES is
///     assigned later, by the not-yet-ported Assemble, for binomials).
fn strip_nom_note(ctx: &mut ParseContext, s: String) -> String {
    let caps = match NOM_NOTE.captures(&s) {
        Ok(Some(c)) => c,
        _ => return s,
    };
    let match_start = caps.get(0).unwrap().start();
    let match_end = caps.get(0).unwrap().end();
    let raw = java_trim(caps.get(1).unwrap().as_str()).to_string();
    let norm = normalise_nom_note(&raw);
    ctx.name.add_nomenclatural_note(&norm);

    let before = java_trim(&s[..match_start]).to_string();
    let after = java_trim(&s[match_end..]).to_string();
    let mut result = if after.is_empty() {
        before.clone()
    } else {
        format!("{before} {after}")
    };
    result = java_trim(&result).to_string();
    while result.ends_with(',') {
        result.pop();
        result = java_trim(&result).to_string();
    }

    if SP_NOV_PREFIX.is_match(&raw) && SINGLE_TITLE_WORD.is_match(&before) {
        result = format!("{before} sp.");
    }
    if MANUSCRIPT_KEYWORD.is_match(&raw) {
        ctx.name.manuscript = true;
    }
    if ctx.name.rank == Rank::Unranked {
        if let Some(hint) = NOM_NOTE_RANK_HINT.captures(&raw) {
            match hint.get(1).unwrap().as_str().to_lowercase().as_str() {
                "gen" => ctx.name.rank = Rank::Genus,
                "fam" => ctx.name.rank = Rank::Family,
                "var" => ctx.name.rank = Rank::Variety,
                "form" => ctx.name.rank = Rank::Form,
                _ => {}
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------------
// Batch 3 (steps 31-44): authorship placeholders, trailing-species, pro parte / pro
// sp. / approved-lists, mihi, anon, the taxonomic-note family, aggregate suffix.
// ---------------------------------------------------------------------------------

// TODO batch 3 — Java `stripAuthorshipPlaceholders`: "Not applicable"/"Not given"/
// "Not known"/… is stripped silently, flagging AUTHORSHIP_REMOVED.
fn strip_authorship_placeholders(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripTrailingSpeciesWord`: a trailing " species" on a bare
// Title-cased uninomial is dropped, producing a plain monomial.
fn strip_trailing_species_word(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripProParte`: ", pro parte" / ", p.p." is stripped silently,
// flagging doubtful.
fn strip_pro_parte(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripProSpAnnotation`: " (pro sp./spec./syn./hyb.)" is stripped
// silently.
fn strip_pro_sp_annotation(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripApprovedLists`: " (Approved Lists YYYY)" is stripped
// silently.
fn strip_approved_lists(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripMihi`: "mihi"/"Mihi" self-attribution (wherever it occurs)
// is stripped, flagging AUTHORSHIP_REMOVED when it actually fired.
fn strip_mihi(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `normaliseAnon`: "Anon."/"Anon"/"anon" is normalised to the
// canonical lower-case "anon." so it parses as a real (anonymous) authorship.
fn normalise_anon(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripColonConceptReference`: a trailing ": Author, YYYY"
// botanical taxonomic-concept citation APPENDS to `taxonomicNote`.
fn strip_colon_concept_reference(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripBracketedTaxNote`: a trailing "[auctt./sensu/sec/non/nec/
// misspelling/misapplied/misident ...]" bracket APPENDS to `taxonomicNote`.
fn strip_bracketed_tax_note(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripParenTaxNote`: a trailing "(nec/non/not …, YYYY)" homonym
// citation APPENDS to `taxonomicNote`.
fn strip_paren_tax_note(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripSensuLatoRemainder`: mid-string "s.l."/"s.str."/"s.lat."/
// "s.ampl." followed by trailing junk APPENDS the (lower-cased, whitespace-collapsed)
// marker to `taxonomicNote` and parks the junk in `ctx.pending_unparsed`.
fn strip_sensu_lato_remainder(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripSensuStrictoSS`: a trailing "s.s." (optionally followed by
// junk) APPENDS "s.s." to `taxonomicNote` and parks any junk in `ctx.pending_unparsed`.
fn strip_sensu_stricto_ss(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripTaxNote`: the general end-anchored taxonomic-note anchor
// (auct./sensu/sec./nec/emend./fide/according to/excl./ss/s.l./s.str./…) OVERWRITES
// `taxonomicNote` (the last of the taxonomic-note family to run, so anything already
// captured by the more specific steps above it never reaches this one).
fn strip_tax_note(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripAggregateSuffix`: " agg."/"aggregate"/"species group"/
// "species complex"/"group"/"complex"/"-group"/"-aggregate" sets `ctx.aggregate = true`.
fn strip_aggregate_suffix(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// ---------------------------------------------------------------------------------
// Batch 4 (steps 45-52): published-in family (page, in-press, in-author variants,
// IPNI, period-/comma-separated references) + manuscript marker.
// ---------------------------------------------------------------------------------

// TODO batch 4 — Java `stripPublishedPage`: a trailing ": 377" / ": 12-18" page
// reference is pulled into `publishedInPage`.
fn strip_published_page(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 4 — Java `stripInPress`: " in press" sets `manuscript = true` and APPENDS
// "in press" to `nomenclaturalNote`.
fn strip_in_press(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 4 — Java `stripInAuthorInParens`: an "in <publication>" citation INSIDE a
// parenthesised basionym ("(Geoffroy in Fourcroy, 1785)") rewrites the parens to just
// the basionym author (+ year, moved over when the basionym itself had none) and
// APPENDS the publication reference to `publishedIn`.
fn strip_in_author_in_parens(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 4 — Java `stripInAuthorCitation`: a trailing " in <Author>" / " apud
// <Author>" tail APPENDS to `publishedIn` and records a code-neutral
// `ctx.pending_year`/`pending_year_from_publication`.
fn strip_in_author_citation(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 4 — Java `stripIpniCitation`: an IPNI-style "Author., Title (Year)."
// citation OVERWRITES `publishedIn` (extracting any embedded nom-note first, which
// APPENDS to `nomenclaturalNote`) and records the pending publication year.
fn strip_ipni_citation(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 4 — Java `stripPeriodSeparatedReference`: a "Surname. <Reference Title>
// ... year ..." citation OVERWRITES `publishedIn`; a page-range ref flags
// NOMENCLATURAL_REFERENCE instead of propagating the year, a clean one propagates the
// pending publication year.
fn strip_period_separated_reference(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 4 — Java `stripCommaPrefixedReference`: an "Author(s), <Reference Title>
// …" citation OVERWRITES `publishedIn` and flags NOMENCLATURAL_REFERENCE (the year is
// never propagated for this form).
fn strip_comma_prefixed_reference(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 4 — Java `stripManuscriptMarker`: a trailing "ined."/"ms."/"msc."/
// "unpublished" sets `manuscript = true` and APPENDS the (lower-cased) tag to
// `nomenclaturalNote`.
fn strip_manuscript_marker(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// ---------------------------------------------------------------------------------
// Batch 5 (steps 53-55): suprarank prefix, leading infrageneric marker, phrase name.
// (Plus the separate `stripAuthorshipMarkers` auxiliary-authorship reimplementation,
// not part of this ordered dispatch — see the investigation §4.)
// ---------------------------------------------------------------------------------

// TODO batch 5 — Java `stripSupraRankPrefix`: a leading "<Family> <suprageneric-rank-
// marker>" (or a bare suprageneric marker with no family) strips the prefix/marker and
// pins the rank.
fn strip_supra_rank_prefix(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 5 — Java `stripLeadingInfragenericMarker`: a leading infrageneric rank
// marker with no genus prefix ("subgen. Trematostoma Sacc.") strips the marker and pins
// the rank (swapped to its zoological counterpart under a caller-supplied ZOOLOGICAL
// code).
fn strip_leading_infrageneric_marker(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 5 — Java `stashPhraseName`: BOLD/specimen-style phrase-name forms
// ("Prostanthera sp. Somersbey (B.J.Conn 4024)") set `ctx.name.phrase` and rewrite the
// working string to "Genus[ species] marker. [Author]" so NameTokens sees an indet name.
fn stash_phrase_name(_ctx: &mut ParseContext, s: String) -> String {
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::NameType;

    fn ctx(s: &str) -> ParseContext {
        ParseContext::new(s.to_string(), None, None, None)
    }

    /// Phase 1 Slice 2 Task 2 locked the dispatcher order down with every one of the 55
    /// steps a pure passthrough. Batches 1 and 2b (steps 1-30) now carry a faithful port,
    /// but a clean, unremarkable binomial should still round-trip untouched: none of
    /// steps 1-19's guard conditions (a trailing/glued "?", a quoted leading monomial, a
    /// "Missing "/lowercase-epithet prefix, Greek/star markers, a letter-subdivision
    /// marker, "str"/"strain", imprint-year brackets, "null" between epithets, Unicode
    /// hyphen/Win-1252/double-underscore artefacts, a trailing OTU code, serovar/serotype,
    /// an angle bracket, or HTML) nor steps 20-30's (a "Candidatus"/"Ca." prefix, a
    /// horticultural "ex" placeholder, a cultivar Group/grex/quoted epithet, an extinction
    /// dagger, a "t.infr." marker, a bracketed genus, "sic"/"corrig.", a synonymy bracket,
    /// or a nom/comb/orth/nomen/sp.nov./pro-syn. keyword) fire on "Abies alba Mill.".
    /// Steps 31-55 remain no-op stubs regardless. This still locks the same invariant
    /// Task 2 established — a batch landing later can't silently leave a stub half-wired
    /// — just no longer via literally every step being a no-op.
    #[test]
    fn run_is_a_complete_noop_until_the_batches_land() {
        let mut c = ParseContext::new("Abies alba Mill.".to_string(), None, None, None);
        let before_working = c.working.clone();
        let before_name = c.name.clone();
        run(&mut c);
        assert_eq!(c.working, before_working);
        assert_eq!(c.name, before_name);
    }

    // ---- Step 1: flagUncertainAuthorship ----

    #[test]
    fn trailing_qmark_is_stripped_and_flags_doubtful() {
        let mut c = ctx("x");
        let out = flag_uncertain_authorship(&mut c, "Aus bus Smith ?".to_string());
        assert_eq!(out, "Aus bus Smith");
        assert!(c.name.doubtful);
        assert!(c
            .name
            .warnings
            .contains(&warnings::QUESTION_MARKS_REMOVED.to_string()));
    }

    #[test]
    fn qmark_glued_to_author_word_flags_uncertain_without_stripping() {
        // Spot-checked against the Java CLI oracle: "Aus bus Sess?" keeps the literal
        // "Sess?" through StripAndStash — the "?" is dropped later by the (not yet
        // ported) tokenizer, not here.
        let mut c = ctx("x");
        let out = flag_uncertain_authorship(&mut c, "Aus bus Sess?".to_string());
        assert_eq!(out, "Aus bus Sess?");
        assert!(c.name.doubtful);
        assert!(c
            .name
            .warnings
            .contains(&warnings::UNCERTAIN_AUTHORSHIP.to_string()));
    }

    #[test]
    fn or_joined_alternative_authors_flag_uncertain_without_stripping() {
        let mut c = ctx("x");
        let out = flag_uncertain_authorship(&mut c, "Aus bus Jarocki or Schinz, 1900".to_string());
        assert_eq!(out, "Aus bus Jarocki or Schinz, 1900");
        assert!(c.name.doubtful);
        assert!(c
            .name
            .warnings
            .contains(&warnings::UNCERTAIN_AUTHORSHIP.to_string()));
    }

    #[test]
    fn slash_joined_alternative_authors_flag_uncertain_without_stripping() {
        let mut c = ctx("x");
        let out = flag_uncertain_authorship(&mut c, "Aus bus Smith/Jones, 1900".to_string());
        assert_eq!(out, "Aus bus Smith/Jones, 1900");
        assert!(c.name.doubtful);
    }

    #[test]
    fn clean_name_is_not_flagged() {
        let mut c = ctx("x");
        let out = flag_uncertain_authorship(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(!c.name.doubtful);
        assert!(c.name.warnings.is_empty());
    }

    // ---- Step 2: extractGenericAuthor ----

    #[test]
    fn infrageneric_author_before_marker_is_extracted() {
        let mut c = ctx("x");
        let out =
            extract_generic_author(&mut c, "Cordia (Adans.) Kuntze sect. Salimori".to_string());
        assert_eq!(out, "Cordia sect. Salimori");
        assert_eq!(
            c.pending_generic_author,
            Some("(Adans.) Kuntze".to_string())
        );
    }

    #[test]
    fn no_infrageneric_marker_leaves_string_untouched() {
        let mut c = ctx("x");
        let out = extract_generic_author(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.pending_generic_author, None);
    }

    // ---- Step 3: stripQuotedMonomial ----

    #[test]
    fn single_quoted_leading_monomial_is_unquoted_and_flagged_doubtful() {
        let mut c = ctx("x");
        let out = strip_quoted_monomial(&mut c, "'Prosthète' Hesse, 1861".to_string());
        assert_eq!(out, "Prosthète Hesse, 1861");
        assert_eq!(c.quoted_monomial, Some("'".to_string()));
        assert!(c.name.doubtful);
    }

    #[test]
    fn double_quoted_monomial_with_no_trailing_text() {
        let mut c = ctx("x");
        let out = strip_quoted_monomial(&mut c, "\"Foo\"".to_string());
        assert_eq!(out, "Foo");
        assert_eq!(c.quoted_monomial, Some("\"".to_string()));
    }

    #[test]
    fn unquoted_name_is_untouched() {
        let mut c = ctx("x");
        let out = strip_quoted_monomial(&mut c, "Abies alba".to_string());
        assert_eq!(out, "Abies alba");
        assert_eq!(c.quoted_monomial, None);
    }

    // ---- Step 4: applyMissingGenusPlaceholder ----

    #[test]
    fn inferred_missing_genus_gets_placeholder_type_and_warning() {
        let mut c = ctx("x");
        let out =
            apply_missing_genus_placeholder(&mut c, "denheyeri Eghbalian, Path, 2017".to_string());
        assert_eq!(out, "? denheyeri Eghbalian, Path, 2017");
        assert_eq!(c.name.type_, NameType::Placeholder);
        assert!(c
            .name
            .warnings
            .contains(&warnings::MISSING_GENUS.to_string()));
    }

    #[test]
    fn explicit_missing_prefix_gets_placeholder_type_but_no_warning() {
        let mut c = ctx("x");
        let out = apply_missing_genus_placeholder(
            &mut c,
            "Missing penchinati Bourguignat, 1870".to_string(),
        );
        assert_eq!(out, "? penchinati Bourguignat, 1870");
        assert_eq!(c.name.type_, NameType::Placeholder);
        assert!(c.name.warnings.is_empty());
    }

    #[test]
    fn quoted_question_prefix_gets_placeholder_type_but_no_warning() {
        let mut c = ctx("x");
        let out = apply_missing_genus_placeholder(&mut c, "\"? gryphoidis".to_string());
        assert_eq!(out, "? gryphoidis");
        assert_eq!(c.name.type_, NameType::Placeholder);
        assert!(c.name.warnings.is_empty());
    }

    #[test]
    fn particle_led_epithet_shape_is_not_treated_as_missing_genus() {
        // "van" is a surname particle (AuthorParticles) — a particle-led input is an
        // author name, not an epithet whose genus went missing, even though it matches
        // MISSING_GENUS_EPITHET's shape (lowercase word + capitalised word).
        let mut c = ctx("x");
        let out = apply_missing_genus_placeholder(&mut c, "van Berg".to_string());
        assert_eq!(out, "van Berg");
        assert_eq!(c.name.type_, NameType::Scientific);
    }

    #[test]
    fn note_keyword_led_epithet_shape_is_not_treated_as_missing_genus() {
        let mut c = ctx("x");
        let out = apply_missing_genus_placeholder(&mut c, "sensu Author".to_string());
        assert_eq!(out, "sensu Author");
        assert_eq!(c.name.type_, NameType::Scientific);
    }

    #[test]
    fn clean_capitalised_name_is_untouched() {
        let mut c = ctx("x");
        let out = apply_missing_genus_placeholder(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.type_, NameType::Scientific);
    }

    // ---- Step 5: stripInfraRankLetters ----

    #[test]
    fn greek_letter_marker_between_epithets_is_collapsed_to_a_space() {
        let mut c = ctx("x");
        let out = strip_infra_rank_letters(&mut c, "Foo bar \u{03B1} baz".to_string());
        assert_eq!(out, "Foo bar baz");
    }

    #[test]
    fn apl_alpha_lookalike_marker_is_also_collapsed() {
        let mut c = ctx("x");
        let out = strip_infra_rank_letters(&mut c, "Foo bar \u{237A} baz".to_string());
        assert_eq!(out, "Foo bar baz");
    }

    #[test]
    fn star_marker_between_epithets_is_collapsed_to_a_space() {
        let mut c = ctx("x");
        let out = strip_infra_rank_letters(&mut c, "Foo bar *** baz".to_string());
        assert_eq!(out, "Foo bar baz");
    }

    #[test]
    fn no_infra_rank_letter_marker_is_untouched() {
        let mut c = ctx("x");
        let out = strip_infra_rank_letters(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
    }

    // ---- Step 6: normaliseLetterSubdivisionMarker ----

    #[test]
    fn letter_subdivision_marker_is_rewritten_to_the_synthetic_token() {
        let mut c = ctx("x");
        let out = normalise_letter_subdivision_marker(
            &mut c,
            "Graphis scripta L. a.b pulverulenta".to_string(),
        );
        assert_eq!(out, "Graphis scripta L. infrasubdivision pulverulenta");
    }

    #[test]
    fn real_rank_marker_shaped_like_a_single_letter_is_left_alone() {
        // "f." is itself a real rank marker (forma) — must be left for the normal
        // rank-marker path, not rewritten to the synthetic subdivision token (confirmed
        // against the Java CLI oracle: this input parses with rank=FORM, not OTHER).
        let mut c = ctx("x");
        let out = normalise_letter_subdivision_marker(
            &mut c,
            "Graphis scripta L. f. pulverulenta".to_string(),
        );
        assert_eq!(out, "Graphis scripta L. f. pulverulenta");
    }

    #[test]
    fn non_matching_shape_is_untouched() {
        let mut c = ctx("x");
        let out = normalise_letter_subdivision_marker(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
    }

    // ---- Step 7: repairQuestionMarkInWord ----

    #[test]
    fn qmark_inside_a_word_is_removed_and_flags_doubtful() {
        let mut c = ctx("x");
        let out = repair_question_mark_in_word(&mut c, "Aus bus Istv?nffi".to_string());
        assert_eq!(out, "Aus bus Istvnffi");
        assert!(c.name.doubtful);
        assert!(c
            .name
            .warnings
            .contains(&warnings::QUESTION_MARKS_REMOVED.to_string()));
    }

    #[test]
    fn no_internal_qmark_is_untouched() {
        let mut c = ctx("x");
        let out = repair_question_mark_in_word(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(!c.name.doubtful);
    }

    // ---- Step 8: stripStrainDesignation ----

    #[test]
    fn quoted_strain_designation_becomes_a_phrase() {
        let mut c = ctx("x");
        let out =
            strip_strain_designation(&mut c, "Aphanizomenon flos-aquae str .'Aph K2'".to_string());
        assert_eq!(out, "Aphanizomenon flos-aquae str.");
        assert_eq!(c.name.phrase, Some("Aph K2".to_string()));
        assert_eq!(c.name.type_, NameType::Informal);
    }

    #[test]
    fn strain_designation_with_lowercase_prefix_is_untouched() {
        // The prefix before the marker must start with an uppercase letter (a plausible
        // genus); a lowercase-led prefix is junk, never touched.
        let mut c = ctx("x");
        let out = strip_strain_designation(&mut c, "foo str .'Aph K2'".to_string());
        assert_eq!(out, "foo str .'Aph K2'");
        assert_eq!(c.name.phrase, None);
    }

    #[test]
    fn no_strain_marker_is_untouched() {
        let mut c = ctx("x");
        let out = strip_strain_designation(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
    }

    // ---- Step 9: stashTrailingStrainCode ----

    #[test]
    fn trailing_rna_code_becomes_a_phrase() {
        let mut c = ctx("x");
        let out = stash_trailing_strain_code(&mut c, "Candida albicans RNA_CTR0-3".to_string());
        assert_eq!(out, "Candida albicans");
        assert_eq!(c.name.phrase, Some("RNA_CTR0-3".to_string()));
        assert_eq!(c.name.type_, NameType::Informal);
    }

    #[test]
    fn trailing_short_rna_code_becomes_a_phrase() {
        let mut c = ctx("x");
        let out = stash_trailing_strain_code(&mut c, "Armillaria ostoyae RNA1".to_string());
        assert_eq!(out, "Armillaria ostoyae");
        assert_eq!(c.name.phrase, Some("RNA1".to_string()));
    }

    #[test]
    fn clean_binomial_with_no_trailing_code_is_untouched() {
        let mut c = ctx("x");
        let out = stash_trailing_strain_code(&mut c, "Abies alba".to_string());
        assert_eq!(out, "Abies alba");
        assert_eq!(c.name.phrase, None);
    }

    // ---- Step 10: stripImprintYears ----

    #[test]
    fn quoted_bracketed_imprint_year_is_stashed() {
        let mut c = ctx("x");
        let out = strip_imprint_years(&mut c, "Aus bus Storr, 1970 [\"1969\"]".to_string());
        assert_eq!(out, "Aus bus Storr, 1970");
        assert_eq!(c.pending_imprint_year, Some("1969".to_string()));
    }

    #[test]
    fn keyword_imprint_year_is_stashed() {
        let mut c = ctx("x");
        let out = strip_imprint_years(&mut c, "Aus bus Storr, 1970 (imprint 1969)".to_string());
        assert_eq!(out, "Aus bus Storr, 1970");
        assert_eq!(c.pending_imprint_year, Some("1969".to_string()));
    }

    #[test]
    fn alt_ampersand_year_is_stashed_when_an_earlier_year_exists() {
        let mut c = ctx("x");
        let out = strip_imprint_years(&mut c, "Aus bus Wagener, 1959 & 1961".to_string());
        assert_eq!(out, "Aus bus Wagener, 1959");
        assert_eq!(c.pending_imprint_year, Some("1961".to_string()));
    }

    #[test]
    fn alt_ampersand_year_is_not_stripped_without_an_earlier_year() {
        let mut c = ctx("x");
        let out = strip_imprint_years(&mut c, "Aus bus & 1961".to_string());
        assert_eq!(out, "Aus bus & 1961");
        assert_eq!(c.pending_imprint_year, None);
    }

    #[test]
    fn no_imprint_year_annotation_is_untouched() {
        let mut c = ctx("x");
        let out = strip_imprint_years(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.pending_imprint_year, None);
    }

    // ---- Step 11: stripNullBetweenEpithets ----

    #[test]
    fn bare_null_between_epithets_is_dropped_and_flags_doubtful() {
        let mut c = ctx("x");
        let out = strip_null_between_epithets(
            &mut c,
            "Austrorhynchus pectatus null pectatus".to_string(),
        );
        assert_eq!(out, "Austrorhynchus pectatus pectatus");
        assert!(c.name.doubtful);
        assert!(c
            .name
            .warnings
            .contains(&warnings::NULL_EPITHET.to_string()));
    }

    #[test]
    fn no_null_epithet_is_untouched() {
        let mut c = ctx("x");
        let out = strip_null_between_epithets(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(!c.name.doubtful);
    }

    // ---- Step 12: normaliseHyphens ----

    #[test]
    fn en_dash_is_normalised_to_ascii_hyphen_and_flags_homoglyphs() {
        let mut c = ctx("x");
        let out = normalise_hyphens(&mut c, "Aus bus\u{2013}fer Mill.".to_string());
        assert_eq!(out, "Aus bus-fer Mill.");
        assert!(c.name.warnings.contains(&warnings::HOMOGLYHPS.to_string()));
    }

    #[test]
    fn all_five_unicode_hyphen_variants_are_normalised() {
        let mut c = ctx("x");
        let input = "a\u{2010}b\u{2011}c\u{2012}d\u{2013}e\u{2014}f";
        let out = normalise_hyphens(&mut c, input.to_string());
        assert_eq!(out, "a-b-c-d-e-f");
    }

    #[test]
    fn ascii_hyphen_alone_is_untouched_with_no_warning() {
        let mut c = ctx("x");
        let out = normalise_hyphens(&mut c, "Aus bus-fer Mill.".to_string());
        assert_eq!(out, "Aus bus-fer Mill.");
        assert!(c.name.warnings.is_empty());
    }

    // ---- Step 13: replaceHomoglyphs (documented stub) ----

    #[test]
    fn replace_homoglyphs_is_a_documented_noop_this_slice() {
        let mut c = ctx("x");
        let input = "Aus \u{0430}bus".to_string(); // Cyrillic 'а' look-alike
        let out = replace_homoglyphs(&mut c, input.clone());
        assert_eq!(
            out, input,
            "stubbed this slice — see the step's doc comment"
        );
        assert!(c.name.warnings.is_empty());
    }

    // ---- Step 14: repairWin1252Artefacts ----

    #[test]
    fn inverted_exclamation_artefact_is_repaired_and_flags_homoglyphs() {
        let mut c = ctx("x");
        let out = repair_win1252_artefacts(&mut c, "Aus bus Plesn\u{00A1}k, 1900".to_string());
        assert_eq!(out, "Aus bus Plesnik, 1900");
        assert!(c.name.warnings.contains(&warnings::HOMOGLYHPS.to_string()));
    }

    #[test]
    fn all_six_win1252_artefacts_are_repaired() {
        let mut c = ctx("x");
        let input = "\u{00A1}\u{00A2}\u{00A3}\u{201A}\u{201E}\u{2030}";
        let out = repair_win1252_artefacts(&mut c, input.to_string());
        assert_eq!(out, "icLeae");
    }

    #[test]
    fn no_win1252_artefact_is_untouched_with_no_warning() {
        let mut c = ctx("x");
        let out = repair_win1252_artefacts(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(c.name.warnings.is_empty());
    }

    // ---- Step 15: normaliseDoubleUnderscores ----

    #[test]
    fn double_underscore_collapses_to_a_single_space() {
        let mut c = ctx("x");
        let out = normalise_double_underscores(&mut c, "Pseudocercospora__dendrobii".to_string());
        assert_eq!(out, "Pseudocercospora dendrobii");
    }

    #[test]
    fn triple_underscore_also_collapses_to_a_single_space() {
        let mut c = ctx("x");
        let out = normalise_double_underscores(&mut c, "Foo___bar".to_string());
        assert_eq!(out, "Foo bar");
    }

    #[test]
    fn single_underscore_is_untouched() {
        let mut c = ctx("x");
        let out = normalise_double_underscores(&mut c, "Foo_bar".to_string());
        assert_eq!(out, "Foo_bar");
    }

    // ---- Step 16: stashTrailingOtuCode ----

    #[test]
    fn trailing_otu_code_is_stashed_as_pending_unparsed() {
        let mut c = ctx("x");
        let out = stash_trailing_otu_code(&mut c, "Oxalis barrelieri XXZ_21243".to_string());
        assert_eq!(out, "Oxalis barrelieri");
        assert_eq!(c.pending_unparsed, Some("XXZ_21243".to_string()));
    }

    #[test]
    fn otu_code_extraction_is_skipped_when_pending_unparsed_already_set() {
        let mut c = ctx("x");
        c.pending_unparsed = Some("existing".to_string());
        let out = stash_trailing_otu_code(&mut c, "Oxalis barrelieri XXZ_21243".to_string());
        assert_eq!(out, "Oxalis barrelieri XXZ_21243");
        assert_eq!(c.pending_unparsed, Some("existing".to_string()));
    }

    #[test]
    fn single_word_input_is_untouched() {
        let mut c = ctx("x");
        let out = stash_trailing_otu_code(&mut c, "Abies".to_string());
        assert_eq!(out, "Abies");
        assert_eq!(c.pending_unparsed, None);
    }

    // ---- Step 17: stripSerovarSerotype ----

    #[test]
    fn bare_serovar_annotation_is_stripped_silently() {
        let mut c = ctx("x");
        let out =
            strip_serovar_serotype(&mut c, "Leptospira interrogans serovar Fugis".to_string());
        assert_eq!(out, "Leptospira interrogans");
        assert!(c.name.warnings.is_empty());
    }

    #[test]
    fn serotype_with_trailing_strain_suffix_is_stripped() {
        let mut c = ctx("x");
        let out = strip_serovar_serotype(
            &mut c,
            "Aggregatibacter actinomycetemcomitans serotype d str. SA508".to_string(),
        );
        assert_eq!(out, "Aggregatibacter actinomycetemcomitans");
    }

    #[test]
    fn parenthesised_serotype_annotation_is_stripped() {
        let mut c = ctx("x");
        let out =
            strip_serovar_serotype(&mut c, "Streptococcus pyogenes (serotype M18)".to_string());
        assert_eq!(out, "Streptococcus pyogenes");
    }

    #[test]
    fn no_serovar_annotation_is_untouched() {
        let mut c = ctx("x");
        let out = strip_serovar_serotype(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
    }

    // ---- Step 18: stripAngleBracketAuthorship ----

    #[test]
    fn angle_bracketed_authorship_placeholder_is_stripped() {
        let mut c = ctx("x");
        let out =
            strip_angle_bracket_authorship(&mut c, "Doradidae <Unspecified Agent>".to_string());
        assert_eq!(out, "Doradidae");
        assert!(c.name.doubtful);
        assert!(c
            .name
            .warnings
            .contains(&warnings::AUTHORSHIP_REMOVED.to_string()));
        assert!(c
            .name
            .warnings
            .contains(&warnings::UNUSUAL_CHARACTERS.to_string()));
    }

    #[test]
    fn no_angle_bracket_is_untouched() {
        let mut c = ctx("x");
        let out = strip_angle_bracket_authorship(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(!c.name.doubtful);
    }

    // ---- Step 19: stripHtml ----

    #[test]
    fn html_tags_are_stripped_keeping_text_content() {
        let mut c = ctx("x");
        let out = strip_html(&mut c, "<i>Aus bus</i> Smith".to_string());
        assert_eq!(out, "Aus bus Smith");
        assert!(c.name.warnings.contains(&warnings::XML_TAGS.to_string()));
    }

    #[test]
    fn html_entities_are_decoded() {
        let mut c = ctx("x");
        let out = strip_html(&mut c, "Aus &amp; bus".to_string());
        assert_eq!(out, "Aus & bus");
        assert!(c
            .name
            .warnings
            .contains(&warnings::HTML_ENTITIES.to_string()));
    }

    #[test]
    fn whitespace_left_by_tag_removal_is_collapsed() {
        let mut c = ctx("x");
        let out = strip_html(&mut c, "Aus <br/> bus".to_string());
        assert_eq!(out, "Aus bus");
    }

    #[test]
    fn no_html_markup_is_untouched() {
        let mut c = ctx("x");
        let out = strip_html(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(c.name.warnings.is_empty());
    }

    // ---- Step 20: stripCandidatus ----
    // All step-20/21/22/23/24/25/26/27/28/29/30 examples below spot-checked against the
    // Java CLI oracle (`name-parser-cli-4.2.0-SNAPSHOT-shaded.jar`), not just traced by
    // hand — same rigor batch 1's module doc describes.

    #[test]
    fn candidatus_prefix_is_stripped_and_flags_bacterial() {
        let mut c = ctx("x");
        let out = strip_candidatus(&mut c, "Candidatus Amesbacteria bacterium".to_string());
        assert_eq!(out, "Amesbacteria bacterium");
        assert!(c.name.candidatus);
        assert_eq!(c.name.code, Some(NomCode::Bacterial));
    }

    #[test]
    fn ca_abbreviation_prefix_is_stripped() {
        let mut c = ctx("x");
        let out = strip_candidatus(&mut c, "Ca. Halobonum".to_string());
        assert_eq!(out, "Halobonum");
        assert!(c.name.candidatus);
        assert_eq!(c.name.code, Some(NomCode::Bacterial));
    }

    #[test]
    fn quoted_candidatus_prefix_strips_leading_and_trailing_quote() {
        let mut c = ctx("x");
        let out = strip_candidatus(&mut c, "\"Candidatus Foo bar\"".to_string());
        assert_eq!(out, "Foo bar");
        assert!(c.name.candidatus);
    }

    #[test]
    fn no_candidatus_prefix_is_untouched() {
        let mut c = ctx("x");
        let out = strip_candidatus(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(!c.name.candidatus);
        assert_eq!(c.name.code, None);
    }

    // ---- Step 21: normaliseHortExPlaceholder ----

    #[test]
    fn cv_ex_placeholder_is_normalised() {
        let mut c = ctx("x");
        let out = normalise_hort_ex_placeholder(&mut c, "Rosa foo cv. ex Smith".to_string());
        assert_eq!(out, "Rosa foo hort. ex Smith");
    }

    #[test]
    fn hort_ex_placeholder_is_normalised() {
        let mut c = ctx("x");
        let out = normalise_hort_ex_placeholder(&mut c, "Rosa foo Hort. ex Smith".to_string());
        assert_eq!(out, "Rosa foo hort. ex Smith");
    }

    #[test]
    fn hortus_ex_placeholder_is_normalised() {
        let mut c = ctx("x");
        let out = normalise_hort_ex_placeholder(&mut c, "Rosa foo hortus ex Smith".to_string());
        assert_eq!(out, "Rosa foo hort. ex Smith");
    }

    #[test]
    fn ht_marker_is_normalised() {
        let mut c = ctx("x");
        let out =
            normalise_hort_ex_placeholder(&mut c, "Gymnogramma alstoni ht.Birkenh.".to_string());
        assert_eq!(out, "Gymnogramma alstoni hort.Birkenh.");
    }

    #[test]
    fn no_hort_ex_placeholder_is_untouched() {
        let mut c = ctx("x");
        let out = normalise_hort_ex_placeholder(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
    }

    // ---- Step 22: stripCultivarGroupGrex ----

    #[test]
    fn trailing_group_marker_sets_cultivar_group_rank() {
        let mut c = ctx("x");
        let out = strip_cultivar_group_grex(&mut c, "Brassica oleracea Capitata Group".to_string());
        assert_eq!(out, "Brassica oleracea");
        assert_eq!(c.name.cultivar_epithet, Some("Capitata".to_string()));
        assert_eq!(c.name.code, Some(NomCode::Cultivars));
        assert_eq!(c.name.rank, Rank::CultivarGroup);
    }

    #[test]
    fn trailing_grex_marker_sets_grex_rank() {
        let mut c = ctx("x");
        let out = strip_cultivar_group_grex(&mut c, "Paphiopedilum Maudiae grex".to_string());
        assert_eq!(out, "Paphiopedilum");
        assert_eq!(c.name.cultivar_epithet, Some("Maudiae".to_string()));
        assert_eq!(c.name.code, Some(NomCode::Cultivars));
        assert_eq!(c.name.rank, Rank::Grex);
    }

    #[test]
    fn trailing_gx_marker_also_sets_grex_rank() {
        let mut c = ctx("x");
        let out = strip_cultivar_group_grex(&mut c, "Paphiopedilum Maudiae gx".to_string());
        assert_eq!(out, "Paphiopedilum");
        assert_eq!(c.name.rank, Rank::Grex);
    }

    #[test]
    fn no_group_grex_marker_is_untouched() {
        let mut c = ctx("x");
        let out = strip_cultivar_group_grex(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.cultivar_epithet, None);
    }

    // ---- Step 23: stripQuotedCultivar ----

    #[test]
    fn quoted_cultivar_at_end_with_explicit_cv_marker_is_captured() {
        let mut c = ctx("x");
        let out = strip_quoted_cultivar(&mut c, "Acer campestre L. cv. 'nanum'".to_string());
        assert_eq!(out, "Acer campestre L.");
        assert_eq!(c.name.cultivar_epithet, Some("nanum".to_string()));
        assert_eq!(c.name.code, Some(NomCode::Cultivars));
        assert_eq!(c.name.rank, Rank::Cultivar);
        // Shape 1 never splits off a specific author (only shape 2/MID does) — the
        // Java oracle keeps "L." in the main combinationAuthorship for this shape.
        assert_eq!(c.pending_specific_author, None);
    }

    #[test]
    fn quoted_cultivar_at_end_without_cv_marker_is_captured() {
        let mut c = ctx("x");
        let out = strip_quoted_cultivar(&mut c, "Acer pseudoplatanus L. 'Negenia'".to_string());
        assert_eq!(out, "Acer pseudoplatanus L.");
        assert_eq!(c.name.cultivar_epithet, Some("Negenia".to_string()));
    }

    #[test]
    fn quoted_cultivar_mid_string_splits_off_preceding_specific_author() {
        let mut c = ctx("x");
        let out = strip_quoted_cultivar(
            &mut c,
            "Acer campestre L. cv. 'Elsrijk' Broerse".to_string(),
        );
        // Spot-checked against the Java CLI oracle: genus=Acer, specificEpithet=campestre,
        // specificAuthorship.combinationAuthorship=["L."], cultivarEpithet=Elsrijk,
        // combinationAuthorship=["Broerse"] — i.e. the reduced working string must read as
        // "Genus species CombinationAuthor" with "L." split off separately.
        assert_eq!(out, "Acer campestre Broerse");
        assert_eq!(c.name.cultivar_epithet, Some("Elsrijk".to_string()));
        assert_eq!(c.name.code, Some(NomCode::Cultivars));
        assert_eq!(c.name.rank, Rank::Cultivar);
        assert_eq!(c.pending_specific_author, Some("L.".to_string()));
    }

    #[test]
    fn quoted_cultivar_mid_string_without_cv_marker_and_without_preceding_author() {
        // No "cv." marker required for shape 2 — MID's own prefix is optional. No author
        // precedes the epithet here (just "Genus species"), so `find_author_start` finds
        // nothing to split off. Spot-checked: combinationAuthorship=["Pils."], no
        // specificAuthorship.
        let mut c = ctx("x");
        let out = strip_quoted_cultivar(&mut c, "Verpericola megasoma \"Dall\" Pils.".to_string());
        assert_eq!(out, "Verpericola megasoma Pils.");
        assert_eq!(c.name.cultivar_epithet, Some("Dall".to_string()));
        assert_eq!(c.pending_specific_author, None);
    }

    #[test]
    fn unclosed_trailing_cultivar_quote_is_captured() {
        let mut c = ctx("x");
        let out = strip_quoted_cultivar(&mut c, "Genus species 'albino".to_string());
        assert_eq!(out, "Genus species");
        assert_eq!(c.name.cultivar_epithet, Some("albino".to_string()));
        assert_eq!(c.name.code, Some(NomCode::Cultivars));
        assert_eq!(c.name.rank, Rank::Cultivar);
    }

    #[test]
    fn quote_preceded_by_bare_rank_marker_is_not_a_cultivar() {
        // "var." right before the quote with no explicit "cv." is a phrase-name shape,
        // not a cultivar — RANK_MARKER_SUFFIX guard must suppress the strip.
        let mut c = ctx("x");
        let out = strip_quoted_cultivar(&mut c, "Genus species var. 'foo'".to_string());
        assert_eq!(out, "Genus species var. 'foo'");
        assert_eq!(c.name.cultivar_epithet, None);
    }

    #[test]
    fn no_quoted_cultivar_is_untouched() {
        let mut c = ctx("x");
        let out = strip_quoted_cultivar(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.cultivar_epithet, None);
    }

    // ---- Step 24: stripExtinctDagger ----

    #[test]
    fn dagger_glued_to_genus_is_stripped_and_flags_extinct() {
        let mut c = ctx("x");
        let out = strip_extinct_dagger(
            &mut c,
            "Henriksenopterix\u{2020} paucistriata (Henriksen, 1922)".to_string(),
        );
        assert_eq!(out, "Henriksenopterix paucistriata (Henriksen, 1922)");
        assert!(c.name.extinct);
    }

    #[test]
    fn alternate_dagger_glyph_is_also_stripped() {
        let mut c = ctx("x");
        let out = strip_extinct_dagger(&mut c, "Foo \u{271D}bar".to_string());
        assert_eq!(out, "Foo bar");
        assert!(c.name.extinct);
    }

    #[test]
    fn no_dagger_is_untouched() {
        let mut c = ctx("x");
        let out = strip_extinct_dagger(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(!c.name.extinct);
    }

    // ---- Step 25: stripTinfrMarker ----

    #[test]
    fn tinfr_marker_between_species_and_infraspecific_epithet_is_stripped() {
        let mut c = ctx("x");
        let out = strip_tinfr_marker(&mut c, "Hieracium alpinum t.infr. foobarum".to_string());
        assert_eq!(out, "Hieracium alpinum foobarum");
    }

    #[test]
    fn tinfr_marker_directly_after_a_bare_genus_is_also_stripped() {
        let mut c = ctx("x");
        let out = strip_tinfr_marker(&mut c, "Hieracium t.infr. foobarum".to_string());
        assert_eq!(out, "Hieracium foobarum");
    }

    #[test]
    fn no_tinfr_marker_is_untouched() {
        let mut c = ctx("x");
        let out = strip_tinfr_marker(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
    }

    // ---- Step 26: stripDoubtfulGenusBrackets ----

    #[test]
    fn bracketed_genus_followed_by_epithet_is_unbracketed_and_flags_doubtful() {
        let mut c = ctx("x");
        let out = strip_doubtful_genus_brackets(&mut c, "[Acontia] chia".to_string());
        assert_eq!(out, "Acontia chia");
        assert!(c.name.doubtful);
        assert!(c
            .name
            .warnings
            .contains(&warnings::DOUBTFUL_GENUS.to_string()));
    }

    #[test]
    fn bracketed_genus_alone_is_unbracketed() {
        let mut c = ctx("x");
        let out = strip_doubtful_genus_brackets(&mut c, "[Dexia]".to_string());
        assert_eq!(out, "Dexia");
        assert!(c.name.doubtful);
    }

    #[test]
    fn no_leading_bracket_is_untouched() {
        let mut c = ctx("x");
        let out = strip_doubtful_genus_brackets(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(!c.name.doubtful);
    }

    // ---- Step 27: stripSicAndCorrig ----

    #[test]
    fn bracketed_sic_with_comment_flags_true_and_stashes_squished_comment() {
        let mut c = ctx("x");
        let out = strip_sic_and_corrig(&mut c, "Aus bus Storr [sic, porphyria]".to_string());
        assert_eq!(out, "Aus bus Storr");
        assert_eq!(c.name.original_spelling, Some(true));
        assert_eq!(c.pending_unparsed, Some("(sic,porphyria)".to_string()));
    }

    #[test]
    fn multi_word_sic_comment_has_all_internal_whitespace_removed_not_just_collapsed() {
        // Spot-checked against the Java CLI oracle: "multiple words here" squishes to
        // "multiplewordshere" — Java's WHITESPACE.replaceAll("") here, replacement "",
        // NOT collapsed to single spaces.
        let mut c = ctx("x");
        let out = strip_sic_and_corrig(&mut c, "Foo bar [sic, multiple words here]".to_string());
        assert_eq!(out, "Foo bar");
        assert_eq!(
            c.pending_unparsed,
            Some("(sic,multiplewordshere)".to_string())
        );
    }

    #[test]
    fn plain_sic_without_comment_flags_true() {
        let mut c = ctx("x");
        let out = strip_sic_and_corrig(
            &mut c,
            "Ameiva plei (sic) Duméril & Bibron, 1839".to_string(),
        );
        assert_eq!(out, "Ameiva plei Duméril & Bibron, 1839");
        assert_eq!(c.name.original_spelling, Some(true));
        assert_eq!(c.pending_unparsed, None);
    }

    #[test]
    fn bare_corrig_marker_flags_false() {
        let mut c = ctx("x");
        let out = strip_sic_and_corrig(&mut c, "Aus bus corrig. Peters, 1878".to_string());
        assert_eq!(out, "Aus bus Peters, 1878");
        assert_eq!(c.name.original_spelling, Some(false));
    }

    #[test]
    fn bracketed_corrig_marker_flags_false() {
        let mut c = ctx("x");
        let out = strip_sic_and_corrig(&mut c, "Aus bus (corrig.) Smith".to_string());
        assert_eq!(out, "Aus bus Smith");
        assert_eq!(c.name.original_spelling, Some(false));
    }

    #[test]
    fn leading_standalone_corrig_is_stripped_via_the_pad_harness() {
        // The exact Phase 0 lesson: CORRIG's bare alternative needs an actual preceding
        // whitespace char, supplied by the call site's leading-space pad — a leading
        // "corrig." with nothing before it must still strip.
        let mut c = ctx("x");
        let out = strip_sic_and_corrig(&mut c, "corrig. Peters, 1878".to_string());
        assert_eq!(out, "Peters, 1878");
        assert_eq!(c.name.original_spelling, Some(false));
    }

    #[test]
    fn no_sic_or_corrig_marker_is_untouched() {
        let mut c = ctx("x");
        let out = strip_sic_and_corrig(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.original_spelling, None);
    }

    // ---- Step 28: stashSynonymBracket ----

    #[test]
    fn trailing_synonym_bracket_is_stashed_verbatim_with_brackets() {
        let mut c = ctx("x");
        let out = stash_synonym_bracket(&mut c, "Aus bus [= Grislea L. 1753]".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.pending_unparsed, Some("[= Grislea L. 1753]".to_string()));
        assert!(c.name.doubtful);
    }

    #[test]
    fn dangling_commas_before_the_bracket_are_all_stripped() {
        let mut c = ctx("x");
        let out = stash_synonym_bracket(&mut c, "Aus bus,, [= Grislea L. 1753]".to_string());
        assert_eq!(out, "Aus bus");
    }

    #[test]
    fn no_synonym_bracket_is_untouched() {
        let mut c = ctx("x");
        let out = stash_synonym_bracket(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.pending_unparsed, None);
        assert!(!c.name.doubtful);
    }

    // ---- Shared helper: normaliseNomNote ----

    #[test]
    fn normalise_nom_note_adds_a_closing_dot_to_an_abbreviated_note() {
        assert_eq!(normalise_nom_note("nom nud"), "nom nud.");
    }

    #[test]
    fn normalise_nom_note_leaves_a_full_word_suffix_without_an_added_dot() {
        assert_eq!(normalise_nom_note("orth error"), "orth error");
    }

    #[test]
    fn normalise_nom_note_strips_trailing_dots_from_spelled_out_nomen_forms() {
        assert_eq!(normalise_nom_note("nomen nudum..."), "nomen nudum");
    }

    #[test]
    fn normalise_nom_note_normalises_et_and_and_to_ampersand() {
        assert_eq!(
            normalise_nom_note("nom. cons. et typ. cons"),
            "nom. cons. & typ. cons."
        );
    }

    // ---- Step 29: stripBracketedNomNote ----

    #[test]
    fn bracketed_nom_nud_note_overwrites_nomenclatural_note() {
        let mut c = ctx("x");
        let out = strip_bracketed_nom_note(
            &mut c,
            "Baccharis microphylla var. rhomboidea (nom. nud.)".to_string(),
        );
        assert_eq!(out, "Baccharis microphylla var. rhomboidea");
        assert_eq!(c.name.nomenclatural_note, Some("nom. nud.".to_string()));
    }

    #[test]
    fn bracketed_nom_illeg_note_overwrites_nomenclatural_note() {
        let mut c = ctx("x");
        let out =
            strip_bracketed_nom_note(&mut c, "Barbula obscura Sull. (nom. illeg.)".to_string());
        assert_eq!(out, "Barbula obscura Sull.");
        assert_eq!(c.name.nomenclatural_note, Some("nom. illeg.".to_string()));
    }

    #[test]
    fn preexisting_note_is_overwritten_not_appended() {
        // Java calls the plain setter here (no null-check) — an already-populated note
        // (as could happen via a directly-constructed ParsedName, or a future reordering)
        // must be REPLACED, not appended to, unlike `strip_nom_note` (step 30).
        let mut c = ctx("x");
        c.name.nomenclatural_note = Some("existing".to_string());
        let out = strip_bracketed_nom_note(&mut c, "Foo bar (nom. nud.)".to_string());
        assert_eq!(out, "Foo bar");
        assert_eq!(c.name.nomenclatural_note, Some("nom. nud.".to_string()));
    }

    #[test]
    fn no_bracketed_nom_note_is_untouched() {
        let mut c = ctx("x");
        let out = strip_bracketed_nom_note(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.nomenclatural_note, None);
    }

    // ---- Step 30: stripNomNote ----

    #[test]
    fn sp_nov_on_a_bare_monomial_appends_note_and_keeps_a_bare_sp_marker() {
        // Spot-checked against the Java CLI oracle: genus=Abies (no specificEpithet),
        // type=INFORMAL, nomenclaturalNote="sp. nov." — the SP_NOV_PREFIX +
        // SINGLE_TITLE_WORD override reduces the working string to "Abies sp." so the
        // (not yet ported) indet-species handling still recognises it downstream.
        let mut c = ctx("x");
        let out = strip_nom_note(&mut c, "Abies sp. nov.".to_string());
        assert_eq!(out, "Abies sp.");
        assert_eq!(c.name.nomenclatural_note, Some("sp. nov.".to_string()));
        assert!(!c.name.manuscript);
    }

    #[test]
    fn sp_nov_ined_also_sets_manuscript_true() {
        let mut c = ctx("x");
        let out = strip_nom_note(&mut c, "Abies sp. nov. ined.".to_string());
        assert_eq!(out, "Abies sp.");
        assert_eq!(
            c.name.nomenclatural_note,
            Some("sp. nov. ined.".to_string())
        );
        assert!(c.name.manuscript);
    }

    #[test]
    fn sp_nov_on_a_full_binomial_does_not_trigger_the_bare_sp_override() {
        // `before` is "Abies alba" (two words) — SINGLE_TITLE_WORD must fail, so the
        // override does not fire; the note is still appended normally.
        let mut c = ctx("x");
        let out = strip_nom_note(&mut c, "Abies alba Sp. nov.".to_string());
        assert_eq!(out, "Abies alba");
        assert_eq!(c.name.nomenclatural_note, Some("Sp. nov.".to_string()));
    }

    #[test]
    fn gen_nov_pins_genus_rank_when_unranked() {
        let mut c = ctx("x");
        let out = strip_nom_note(&mut c, "Fooxus gen. nov.".to_string());
        assert_eq!(out, "Fooxus");
        assert_eq!(c.name.nomenclatural_note, Some("gen. nov.".to_string()));
        assert_eq!(c.name.rank, Rank::Genus);
    }

    #[test]
    fn var_nov_pins_variety_rank() {
        let mut c = ctx("x");
        let out = strip_nom_note(&mut c, "Foovar var. nov.".to_string());
        assert_eq!(out, "Foovar");
        assert_eq!(c.name.rank, Rank::Variety);
    }

    #[test]
    fn fam_nov_pins_family_rank() {
        let mut c = ctx("x");
        let out = strip_nom_note(&mut c, "Foofam fam. nov.".to_string());
        assert_eq!(out, "Foofam");
        assert_eq!(c.name.rank, Rank::Family);
    }

    #[test]
    fn form_nov_pins_form_rank() {
        let mut c = ctx("x");
        let out = strip_nom_note(&mut c, "Fooform form. nov.".to_string());
        assert_eq!(out, "Fooform");
        assert_eq!(c.name.rank, Rank::Form);
    }

    #[test]
    fn rank_hint_does_not_override_an_already_set_rank() {
        let mut c = ctx("x");
        c.name.rank = Rank::Species;
        let out = strip_nom_note(&mut c, "Fooxus gen. nov.".to_string());
        assert_eq!(out, "Fooxus");
        assert_eq!(
            c.name.rank,
            Rank::Species,
            "an already-set rank must not be clobbered by the hint"
        );
    }

    #[test]
    fn nom_note_appends_to_an_existing_nomenclatural_note() {
        let mut c = ctx("x");
        c.name.nomenclatural_note = Some("existing".to_string());
        let out = strip_nom_note(&mut c, "Foo bar comb. nov.".to_string());
        assert_eq!(out, "Foo bar");
        assert_eq!(
            c.name.nomenclatural_note,
            Some("existing comb. nov.".to_string())
        );
    }

    #[test]
    fn no_nom_note_keyword_is_untouched() {
        let mut c = ctx("x");
        let out = strip_nom_note(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.nomenclatural_note, None);
        assert!(!c.name.manuscript);
    }

    #[test]
    fn nom_note_requires_a_leading_whitespace_and_does_not_fire_at_string_start() {
        // NOM_NOTE's leading `\s+` means a marker glued to the very start of the string
        // (no preceding name text at all) can never match — spot-checked: "Gen. nov.
        // Foobarus" parses "Gen." as an abbreviated genus, "nov" as a specific epithet,
        // NOT as a nom-note (rank stays SPECIES, not GENUS, on the Java oracle).
        let mut c = ctx("x");
        let out = strip_nom_note(&mut c, "Gen. nov. Foobarus".to_string());
        assert_eq!(out, "Gen. nov. Foobarus");
        assert_eq!(c.name.nomenclatural_note, None);
        assert_eq!(c.name.rank, Rank::Unranked);
    }

    #[test]
    fn nom_note_does_not_redos_on_dotted_abbrev_run() {
        // NOM_NOTE runs on fancy_regex (backtracking), so Java's possessive quantifier
        // (restored above, see the RULE comment) is load-bearing. A long run of dotted
        // lowercase "words" after "nomen" with no valid terminator in sight (no comma, no
        // uppercase, no closing paren, no end-of-string right after) used to force the
        // greedy `*` to re-partition the run exponentially before finally giving up —
        // reintroducing the exact catastrophic-backtracking shape Java's own `*+` exists
        // to prevent (proven: ~11ms and a backtrack-limit no-match with plain `*` on just
        // 20 repeats; microseconds with `*+` restored). This must complete promptly
        // regardless of how long the dotted run is.
        let mut c = ctx("x");
        let input = "Aus bus nomen ".to_string() + &"a.".repeat(30) + "#";
        let start = std::time::Instant::now();
        let out = strip_nom_note(&mut c, input.clone());
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 200,
            "strip_nom_note took {elapsed:?} on a dotted-abbrev run — possessive quantifier regressed"
        );
        // The trailing "#" satisfies none of NOM_NOTE's terminator lookaheads, so the
        // whole pattern never matches here (same as any other non-matching input) — the
        // string comes back untouched, no note stashed.
        assert_eq!(out, input);
        assert_eq!(c.name.nomenclatural_note, None);
    }
}
