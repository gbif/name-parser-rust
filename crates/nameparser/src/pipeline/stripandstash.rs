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
//! **Phase 1 Slice 2 Task 3 (this batch): steps 1-19 ported.** [`run`] dispatches all 55
//! steps in Java's exact `StripAndStash.run(ParseContext)` order — that order is
//! load-bearing and was locked in by Task 2 — steps 1-19 (leading normalizers + flaggers)
//! now carry their faithful port; steps 20-55 (batches 2-5) remain `// TODO batch N` no-op
//! passthroughs (see `docs/superpowers/plans/2026-07-10-phase1-stripandstash.md` for the
//! batch breakdown). One step, `replace_homoglyphs` (step 13), is a DOCUMENTED stub — see
//! its own doc comment — since porting its backing table is a sizeable sub-project deferred
//! by design (mirrors `crate::unicode`'s own existing deferral of the same table).

use std::sync::LazyLock;

use fancy_regex::Regex as FancyRegex;
use regex::Regex;

use crate::model::{warnings, NameType, ParsedName};
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
// ---------------------------------------------------------------------------------

// TODO batch 2 — Java `stripCandidatus`: a leading "Candidatus "/"Ca. " prefix sets
// `candidatus = true` and `code = BACTERIAL`, and is stripped from the working string.
fn strip_candidatus(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `normaliseHortExPlaceholder`: "cv. ex" / "Hort. ex" /
// "hortus(a) ex" / "ht." horticultural placeholder variants are normalised to the
// canonical lower-case "hort.".
fn normalise_hort_ex_placeholder(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripCultivarGroupGrex`: a trailing "... CapWord(s) (Group|grex|gx)"
// sets `cultivarEpithet` + `code = CULTIVARS` + rank (CULTIVAR_GROUP or GREX).
fn strip_cultivar_group_grex(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripQuotedCultivar`: a quoted cultivar epithet, at end of
// string or mid-string before a trailing author span, sets `cultivarEpithet` +
// `code = CULTIVARS` + `rank = CULTIVAR` (splitting off a preceding species author into
// `ctx.pending_specific_author` for the mid-string form).
fn strip_quoted_cultivar(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripExtinctDagger`: "†"/"✝" anywhere in the string set
// `extinct = true` and are stripped.
fn strip_extinct_dagger(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripTinfrMarker`: the "t.infr." infraspecific abbreviation
// (Hieracium notation) is stripped so the trailing epithet parses as a normal
// infraspecific name.
fn strip_tinfr_marker(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripDoubtfulGenusBrackets`: a leading bracketed genus
// ("[Acontia] chia ...") has its brackets dropped, flagging doubtful + DOUBTFUL_GENUS.
fn strip_doubtful_genus_brackets(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripSicAndCorrig`: "[sic]"/"(sic)"/"[sic, comment]" set
// `originalSpelling = true` (the comment form also stashes the comment into
// `ctx.pending_unparsed`); "corrig."/"(corrig.)" sets `originalSpelling = false`.
fn strip_sic_and_corrig(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stashSynonymBracket`: a trailing "[= Grislea L. 1753]" synonymy
// reference is parked in `ctx.pending_unparsed`, flagging doubtful.
fn stash_synonym_bracket(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripBracketedNomNote`: a trailing bracketed/parenthesised
// nom-note ("[nom. et typ. cons.]", "(nom. nud.)") OVERWRITES `nomenclaturalNote`
// (not an append — this is the first step that can populate the field, so there is
// nothing to append to yet in Java's own call order either).
fn strip_bracketed_nom_note(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripNomNote`: a nom/comb/orth/sp.nov./pro-syn. keyword tail
// APPENDS to `nomenclaturalNote` (via `ctx.name.add_nomenclatural_note` once ported),
// plus manuscript-flag and rank-hint side effects.
fn strip_nom_note(_ctx: &mut ParseContext, s: String) -> String {
    s
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
    /// steps a pure passthrough. Batch 1 (steps 1-19, Task 3) now carries a faithful port,
    /// but a clean, unremarkable binomial should still round-trip untouched: none of
    /// steps 1-19's guard conditions (a trailing/glued "?", a quoted leading monomial, a
    /// "Missing "/lowercase-epithet prefix, Greek/star markers, a letter-subdivision
    /// marker, "str"/"strain", imprint-year brackets, "null" between epithets, Unicode
    /// hyphen/Win-1252/double-underscore artefacts, a trailing OTU code, serovar/serotype,
    /// an angle bracket, or HTML) fire on "Abies alba Mill.". Steps 20-55 remain no-op
    /// stubs regardless. This still locks the same invariant Task 2 established — a batch
    /// landing later can't silently leave a stub half-wired — just no longer via literally
    /// every step being a no-op.
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
}
