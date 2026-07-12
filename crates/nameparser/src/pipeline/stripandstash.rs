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
//! **Phase 1 Slice 2, batches 1 + 2b + 2c + 2d + 2e (this batch, FINAL): all 55 steps
//! ported.** [`run`] dispatches all 55 steps in Java's exact `StripAndStash.run(ParseContext)`
//! order — that order is load-bearing and was locked in by Task 2 — steps 1-19 (leading
//! normalizers + flaggers, batch 1), 20-30 (candidatus, cultivar Group/grex/quoted, extinct
//! dagger, t.infr., doubtful-genus brackets, sic/corrig, synonym bracket, bracketed + bare
//! nom-note, batch 2b), 31-44 (authorship placeholders, trailing-species, pro parte /
//! pro sp. / approved-lists, mihi, anon, the six-step taxonomic-note family, aggregate
//! suffix, batch 2c), 45-52 (published-in page/in-press/in-author-in-parens/
//! in-author-citation/IPNI/period-separated-reference/comma-prefixed-reference/
//! manuscript-marker, batch 2d), and 53-55 (suprarank prefix, leading infrageneric marker,
//! phrase name, batch 2e) all now carry their faithful port (see
//! `docs/superpowers/plans/2026-07-10-phase1-stripandstash.md` for the batch breakdown).
//! Landing batch 2e flips the golden harness (`tests/parse_golden.rs`) from a deferred
//! baseline print to an asserted-0 gate over the 10 downstream-independent fields — see
//! that file's own module doc.
//!
//! Step 13, `replace_homoglyphs`, delegates to `crate::unicode`'s full port of Java's
//! `UnicodeUtils` homoglyph table (Phase 1 Slice 4) — see that step's own doc comment.
//! Separately, Java's `stripAuthorshipMarkers` — an auxiliary-authorship
//! reimplementation that duplicates a handful of this file's own strip steps but is NOT
//! part of this ordered `run()` dispatch at all — is ported further down this same file
//! (`strip_authorship_markers`, Phase 1 Slice 4 Task 4) as its own, shorter, ordered step
//! list, invoked on the caller-supplied `authorship` string from `pipeline::mod`'s
//! aux-authorship path.

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
/// All 55 steps (batches 1, 2b, 2c, 2d, 2e) now carry a faithful port — see the module
/// doc.
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
// fields the tokenizer/authorship-parser/Assemble stages would otherwise surface.
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
/// the tokenizer; "Aus bus Jarocki or Schinz, 1900" keeps the literal " or "
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
/// `ctx.pending_generic_author` for `Pipeline` to apply as the generic
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
/// the quote char in `ctx.quoted_monomial` (so `Assemble` can re-wrap the
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
/// the rank-marker machinery treats the trailing epithet as an
/// infraspecific of the unmappable rank `Rank::Other`. Never appears in user input.
const LETTER_SUBDIVISION: &str = "infrasubdivision";

/// Java `RankMarkers.matchInfraspecific(String) != null` — MINIMAL existence-only port.
/// This is a small pre-existing stand-in for the real `RankMarkers.INFRASPECIFIC` map (word
/// -> `Rank`), predating `rank_markers.rs` (the dedicated rank-handling module, which now
/// carries the full word -> `Rank` map); only the boolean null-check this one call site
/// (`normalise_letter_subdivision_marker` below) needs is kept here rather than routed
/// through that module (see `rank_markers.rs`'s own module doc for the cross-reference).
/// Key set transcribed verbatim from `RankMarkers.java`'s `INFRASPECIFIC` map (including
/// `LETTER_SUBDIVISION` itself, which Java's own map also carries as a key, and the
/// non-letter `"*"` key, which this call site can never actually produce). In practice the
/// calling regex can only ever split a SINGLE ASCII lowercase letter into `segments[0]`
/// (see the doc comment below), so only the `"f"` entry is reachable here — the full key
/// set is nonetheless transcribed in full (not hand-trimmed down to just "f") so this stays
/// an honest stand-in for the real map, not a call-site-specific shortcut.
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
/// the core-only `is_match` form (same reasoning as `GREEK_MARKER_TEST`). Shared with
/// `stripAuthorshipMarkers` (StripAndStash.java:440, ported below as
/// `strip_authorship_markers`) — defining it once here makes it directly reusable by that
/// call site too.
static LETTER_QMARK_LETTER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\p{L}\?\p{L}").unwrap());

/// Java QMARK_BETWEEN_LETTERS (StripAndStash.java:296): `(\p{L})\?(\p{L})`, no flags. No
/// `\s`/`\d`/`\w`/`\b` atoms -> nothing to scope, ported verbatim. Shared with
/// `stripAuthorshipMarkers` (ported below as `strip_authorship_markers`), same as its
/// sibling above.
static QMARK_BETWEEN_LETTERS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\p{L})\?(\p{L})").unwrap());

/// Java `StripAndStash.repairQuestionMarkInWord` (StripAndStash.java:739-748). A "?" inside
/// a word is a transcription artefact for a missing letter ("Istv?nffi") — strips the "?"
/// and glues the surrounding word parts directly together (no placeholder letter is
/// guessed: "Istv?nffi" -> "Istvnffi"), flagging doubtful + `QUESTION_MARKS_REMOVED`. (The
/// identical inline logic also opens the separate `stripAuthorshipMarkers`
/// auxiliary-authorship path (ported below as `strip_authorship_markers`) —
/// `StripAndStash.java` duplicates it rather than sharing a helper, so this port does too,
/// reusing the same static patterns.)
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
/// "Genus species str." is left for NameTokens, which resolves the
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
/// ("Abies null Hood") is NOT touched here — left for `Assemble::flagBlacklistedEpithets`.
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

// ---- Step 13: replaceHomoglyphs ----

/// Java `StripAndStash.replaceHomoglyphs` (StripAndStash.java:839-854):
/// ```java
/// if (UnicodeUtils.containsHomoglyphs(s)) {
///   String repl = UnicodeUtils.replaceHomoglyphs(s, false);
///   if (!repl.equals(s)) {
///     ctx.name.addWarning(Warnings.HOMOGLYHPS);
///     s = repl;
///   }
/// }
/// ```
/// Delegates to [`crate::unicode::contains_homoglyphs`]/[`crate::unicode::replace_homoglyphs`]
/// — the full codepoint -> canonical-char table port (`UnicodeUtils.java:58-139`'s
/// `~175`-line `unicode/homoglyphs.txt` loader; see that module's own doc comment for the
/// loader/table details) — flagging `HOMOGLYHPS` only when the string actually changed.
/// `inclHyphens=false` at this (the table's only) call site: Unicode hyphen variants are
/// already normalised to ASCII by the immediately-preceding `normalise_hyphens` (step 12),
/// per Java's own comment on this method ("Hyphen homoglyphs are intentionally excluded —
/// those are normalised above already"). Both the outer `contains_homoglyphs` fast-path
/// check and the inner `repl != s` re-check are redundant in the overwhelmingly common case
/// (a table hit always changes the string, since no row's canonical ever equals one of its
/// own look-alikes) but ported verbatim anyway, mirroring Java's exact structure.
fn replace_homoglyphs(ctx: &mut ParseContext, s: String) -> String {
    if crate::unicode::contains_homoglyphs(&s) {
        let repl = crate::unicode::replace_homoglyphs(&s);
        if repl != s {
            ctx.name.add_warning(warnings::HOMOGLYHPS);
            return repl;
        }
    }
    s
}

// ---- Step 14: repairWin1252Artefacts (structural — no Pattern; shared w/ stripAuthorshipMarkers) ----

/// Java `StripAndStash.repairWin1252Artefacts(ParsedName, String)`
/// (StripAndStash.java:856-876). Win-1252 -> UTF-8 transcription artefacts the homoglyph
/// table doesn't cover (e.g. "Plesn¡k" should read as "Plesnik") — maps a small set of
/// high-bit punctuation characters to their Latin look-alikes, flagging `HOMOGLYHPS` when
/// anything changed (spot-checked against the Java CLI oracle: "Aus bus Plesn¡k, 1900" ->
/// `authors=["Plesnik"]`, warning "homoglyphs replaced"). Java's real signature takes
/// `ParsedName` directly (shared with the `stripAuthorshipMarkers` auxiliary-authorship
/// path, ported below as `strip_authorship_markers`) — ported with that same signature here
/// (`_name` suffix) so that call site can reuse it directly, which it does;
/// `repair_win1252_artefacts` below is the uniform `(ctx, s)`-shaped wrapper `run`'s
/// dispatcher needs.
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
// Ported (Phase 1 Slice 2, batch 2b). This batch introduced the first `Rank` variants
// beyond the Unranked/Species/Subspecies trio that existed at the time (`model::enums::Rank`
// is now the full 117-constant enumeration — see its own doc comment) — `rank`/`code` are
// shared (B) fields also written by the downstream stages, so they're not part of THIS
// slice's gate, but are set here faithfully anyway since those stages consume them.
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
/// reasoning as `CV_EX`. Shared with `stripAuthorshipMarkers` (ported below as
/// `strip_authorship_markers`).
static HORT_EX: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"\bHort\.(?=[ \t\n\x0B\f\r]+ex[ \t\n\x0B\f\r]+)").unwrap());

/// Java HORTUS_EX (StripAndStash.java:309): `\bhortus[a]?\b(?=\s+ex\s+)`, no flags. Same
/// fancy_regex/ASCII-whitespace reasoning as `CV_EX`. Shared with `stripAuthorshipMarkers`
/// (ported below as `strip_authorship_markers`).
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
/// anchors make `.captures()` here equivalent to Java's `.matches()`. Shared, in Java
/// itself, by TWO call sites: `stripQuotedCultivar` (this batch, 2b, StripAndStash.java:
/// 1035) and `stashPhraseName` (step 55, batch 2e, StripAndStash.java:1609) — ported once
/// here and reused by both, matching Java's own one-method-two-callers shape.
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
/// `\b` (x2) ASCII-scoped. Shared with `stripAuthorshipMarkers` (ported below as
/// `strip_authorship_markers`) and `stripManuscriptMarker` (batch 4) — defined once here,
/// reusable file-wide.
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
///     rank marker so the downstream indet/INFORMAL handling still recognises a
///     species-indet name, rather than leaving a bare uninomial with the marker fully
///     gone);
///   - a captured note containing "ined"/"ms"/"msc"/"unpublished" sets `manuscript = true`
///     (the note text itself already consumed the keyword, so the separate standalone
///     manuscript-marker step, batch 4, won't see it there anymore);
///   - when the name doesn't already carry a rank (`Rank::Unranked`), a gen/fam/var/form
///     prefix on `raw` pins the rank accordingly ("sp"/"spec" is left alone — SPECIES is
///     assigned later, by Assemble, for binomials).
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
// Ported (Phase 1 Slice 2, batch 2c). This batch closes `taxonomicNote` — steps 38-42
// APPEND to it (`ParsedName::add_taxonomic_note`, the Java inline `existing == null ?
// note : existing + " " + note` idiom); step 43 (`stripTaxNote`, the LAST of the six
// taxonomic-note steps to run) OVERWRITES it directly (`ctx.name.taxonomic_note =
// Some(norm)`, mirroring Java's plain `setTaxonomicNote(norm)` call with no
// null-check) since anything more specific was already peeled off by the steps above
// it. All worked examples below were spot-checked against the real Java CLI oracle
// (`name-parser-cli-4.2.0-SNAPSHOT-shaded.jar`), same convention as batches 1-2b.
// ---------------------------------------------------------------------------------

// ---- Step 31: stripAuthorshipPlaceholders ----

/// Java AUTHORSHIP_PLACEHOLDER (StripAndStash.java:226-228):
/// `\s+Not\s+(?:applicable|given|known|recorded|found)\s*$`, `Pattern.CASE_INSENSITIVE`.
/// Has `\s` (x3), no `\p{...}`, no unescaped wildcard -> whole-wrap ASCII scope (`$` left
/// outside per convention). No lookaround/backreference -> plain `regex` crate.
static AUTHORSHIP_PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?-u:\s+Not\s+(?:applicable|given|known|recorded|found)\s*)$").unwrap()
});

/// Java `StripAndStash.stripAuthorshipPlaceholders` (StripAndStash.java:1197-1208). A
/// trailing "Not applicable"/"Not given"/"Not known"/"Not recorded"/"Not found"
/// authorship placeholder (any case) is dropped silently, flagging `AUTHORSHIP_REMOVED`
/// so the bare name (with whatever real author text preceded the placeholder, if any)
/// still parses — spot-checked: "Aus bus Smith Not given" -> authors=["Smith"],
/// warnings=["authorship placeholder removed"]; "Aus bus Not applicable" -> no author,
/// same warning.
fn strip_authorship_placeholders(ctx: &mut ParseContext, s: String) -> String {
    if let Some(m) = AUTHORSHIP_PLACEHOLDER.find(&s) {
        ctx.name.add_warning(warnings::AUTHORSHIP_REMOVED);
        return java_trim(&s[..m.start()]).to_string();
    }
    s
}

// ---- Step 32: stripTrailingSpeciesWord ----

/// Java TRAILING_SPECIES_WORD_TEST (StripAndStash.java:354-355):
/// `^[\p{Lu}][\p{Ll}]+\s+species\s*\.?$`, no flags. Has `\p{Lu}`/`\p{Ll}` (always
/// Unicode) and `\s` (x2) -> only `\s` ASCII-scoped (atom-only, not whole-wrap). Called
/// via `.matches()`, but the pattern already opens with `^` and closes with `$` -> no
/// change needed. Gates the strip to a bare "Title word" + " species[.]" shape only, so a
/// real binomial ("Genus species alba") is never mistaken for this placeholder form.
static TRAILING_SPECIES_WORD_TEST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{Lu}][\p{Ll}]+(?-u:\s+)species(?-u:\s*)\.?$").unwrap());

/// Java TRAILING_SPECIES_WORD (StripAndStash.java:356): `\s+species\s*\.?$`, no flags.
/// Has `\s` (x2), no `\p{...}`, no unescaped wildcard (the `\.` is an escaped literal) ->
/// whole-wrap ASCII scope. No lookaround/backreference -> plain `regex` crate.
static TRAILING_SPECIES_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\s+species\s*)\.?$").unwrap());

/// Java `StripAndStash.stripTrailingSpeciesWord` (StripAndStash.java:1210-1218). A
/// trailing " species"/" species." on an otherwise-bare Title-cased uninomial ("Abies
/// species", "Abies species.") drops the word, producing a plain monomial — spot-checked:
/// both forms -> `uninomial="Abies"` (no rank/INFORMAL marker stashed here; that
/// classification happens in the tokenizer/Assemble).
fn strip_trailing_species_word(_ctx: &mut ParseContext, s: String) -> String {
    if TRAILING_SPECIES_WORD_TEST.is_match(&s) {
        if let Some(m) = TRAILING_SPECIES_WORD.find(&s) {
            return java_trim(&s[..m.start()]).to_string();
        }
    }
    s
}

// ---- Step 33: stripProParte ----

/// Java PRO_PARTE (StripAndStash.java:229-231): `\s*,\s*(?:pro\s+parte|p\.\s*p\.[A-Z]?)\s*$`,
/// `Pattern.CASE_INSENSITIVE`. Has `\s` (x5), no `\p{...}`, no unescaped wildcard (dots
/// escaped) -> whole-wrap ASCII scope; the positive class `[A-Z]` sits inside the wrap
/// too, so under `(?i)` it folds ASCII-only (matching Java's default CASE_INSENSITIVE,
/// which is ASCII-only unless UNICODE_CASE is also set — it isn't here). No
/// lookaround/backreference -> plain `regex` crate.
static PRO_PARTE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?-u:\s*,\s*(?:pro\s+parte|p\.\s*p\.[A-Z]?)\s*)$").unwrap());

/// Java `StripAndStash.stripProParte` (StripAndStash.java:1220-1229). A trailing ",
/// pro parte" / ", p.p." (optionally suffixed by a single capital letter, e.g. "p.p.A")
/// "in part" taxonomic-concept qualifier is stripped silently, flagging doubtful —
/// spot-checked: "Aus bus Smith, pro parte" -> authors=["Smith"], doubtful=true, no note
/// text added (this step never touches `taxonomicNote`).
fn strip_pro_parte(ctx: &mut ParseContext, s: String) -> String {
    if let Some(m) = PRO_PARTE.find(&s) {
        ctx.name.doubtful = true;
        return java_trim(&s[..m.start()]).to_string();
    }
    s
}

// ---- Step 34: stripProSpAnnotation ----

/// Java PRO_SP_ANNOTATION (StripAndStash.java:232-234):
/// `\s+\(\s*pro\s+(?:sp|spec|syn|hyb)\b\.?\s*\)\s*\.?\s*$`, `Pattern.CASE_INSENSITIVE`.
/// Has `\s` (x5) and `\b`, no `\p{...}`, no unescaped wildcard (parens/dots escaped) ->
/// whole-wrap ASCII scope. No lookaround/backreference -> plain `regex` crate.
static PRO_SP_ANNOTATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?-u:\s+\(\s*pro\s+(?:sp|spec|syn|hyb)\b\.?\s*\)\s*\.?\s*)$").unwrap()
});

/// Java `StripAndStash.stripProSpAnnotation` (StripAndStash.java:1231-1239). A trailing
/// " (pro sp.)" / " (pro spec.)" / " (pro syn.)" / " (pro hyb.)" botanical "given as"
/// annotation is stripped silently, working-string only — spot-checked: "Aus bus (pro
/// sp.)" -> `specificEpithet="bus"`, no doubtful flag, no warning, no note.
fn strip_pro_sp_annotation(_ctx: &mut ParseContext, s: String) -> String {
    if let Some(m) = PRO_SP_ANNOTATION.find(&s) {
        return java_trim(&s[..m.start()]).to_string();
    }
    s
}

// ---- Step 35: stripApprovedLists ----

/// Java APPROVED_LISTS (StripAndStash.java:235-237):
/// `\s*\(\s*Approved\s+Lists\s+\d{4}\s*\)\s*\.?\s*$`, `Pattern.CASE_INSENSITIVE`. Has `\s`
/// (x6) and `\d`, no `\p{...}`, no unescaped wildcard -> whole-wrap ASCII scope. No
/// lookaround/backreference -> plain `regex` crate.
static APPROVED_LISTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?-u:\s*\(\s*Approved\s+Lists\s+\d{4}\s*\)\s*\.?\s*)$").unwrap()
});

/// Java `StripAndStash.stripApprovedLists` (StripAndStash.java:1241-1249). A trailing "
/// (Approved Lists YYYY)" bacterial-code annotation is stripped silently, working-string
/// only — spot-checked: "Aus bus Smith (Approved Lists 1980)" -> authors=["Smith"], no
/// other side effect.
fn strip_approved_lists(_ctx: &mut ParseContext, s: String) -> String {
    if let Some(m) = APPROVED_LISTS.find(&s) {
        return java_trim(&s[..m.start()]).to_string();
    }
    s
}

// ---- Step 36: stripMihi ----

/// Java MIHI_TEST (StripAndStash.java:345): `(?i).*\bmihi\b.*`, no
/// `UNICODE_CHARACTER_CLASS`. RESTRUCTURED: called via `.matches()` on a `.*CORE.*` shape
/// -> equivalent to an unanchored `is_match` on CORE alone (same restructuring as
/// `MANUSCRIPT_KEYWORD` in batch 2). `\b` (x2) ASCII-scoped.
static MIHI_TEST: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)(?-u:\bmihi\b)").unwrap());

/// Java MIHI (StripAndStash.java:346): `(?i)\s+mihi\.?(?=\s|$)`, no
/// `UNICODE_CHARACTER_CLASS`. Trailing lookahead (boundary not consumed, so e.g. "Genus
/// species mihi var. epithet mihi" strips BOTH occurrences — the separator before "var."
/// survives the first match to bound the next one) -> `fancy_regex`. `fancy_regex` has no
/// ASCII mode -> `\s` spelled out as the literal ASCII whitespace set. No possessive
/// quantifier in this pattern.
static MIHI: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"(?i)[ \t\n\x0B\f\r]+mihi\.?(?=[ \t\n\x0B\f\r]|$)").unwrap());

/// Java `StripAndStash.stripMihi` (StripAndStash.java:1251-1266). Latin "mihi" ("by me"),
/// a self-attribution placeholder some authors used in place of a real authorship, is
/// stripped wherever it occurs (trailing OR mid-string, per `MIHI`'s non-consuming
/// lookahead) with an `AUTHORSHIP_REMOVED` warning — but ONLY when something actually
/// changed: Java's `if (!s.equals(before))` structural-equality guard (mirrored here via
/// `after != before`) means a "mihi" that satisfies `MIHI_TEST`'s loose `\bmihi\b` gate
/// but doesn't sit in `MIHI`'s required `<whitespace>mihi<whitespace-or-EOS>` position
/// (e.g. as the very first word, with no leading separator to match `\s+`) leaves the
/// string untouched and fires no warning — spot-checked: "Aus bus mihi" ->
/// warnings=["authorship placeholder removed"], "Aus bus mihi. Smith" -> same warning,
/// authors=["Smith"] (mid-string strip splices the surrounding text back together).
fn strip_mihi(ctx: &mut ParseContext, s: String) -> String {
    if MIHI_TEST.is_match(&s) {
        let before = s.clone();
        let after = java_trim(&fancy_replace_all(&MIHI, &s, |_| String::new())).to_string();
        if after != before {
            ctx.name.add_warning(warnings::AUTHORSHIP_REMOVED);
        }
        return after;
    }
    s
}

// ---- Step 37: normaliseAnon ----

/// Java ANON_UPPER (StripAndStash.java:347): `(?<=\s)Anon\b\.?`, no flags (case-sensitive
/// literal "Anon", not `(?i)`). LOOKBEHIND -> `fancy_regex`. `\s` spelled out ASCII
/// (`fancy_regex` has no ASCII mode); `\b` stays `fancy_regex`'s Unicode-default word
/// boundary (the same forced, vanishingly-rare divergence documented at `STRAIN_DESIGNATION`
/// in batch 1 — Java's own `\b` here is ASCII-only, `fancy_regex` has no ASCII `\b`
/// option). No possessive quantifier.
static ANON_UPPER: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"(?<=[ \t\n\x0B\f\r])Anon\b\.?").unwrap());

/// Java ANON_LOWER (StripAndStash.java:348): `(?<=\s)anon\b(?!\.)`, no flags
/// (case-sensitive literal "anon"). LOOKBEHIND + trailing NEGATIVE LOOKAHEAD (excludes an
/// "anon." that's already dotted, so `ANON_UPPER`/this step never double-append a second
/// dot) -> `fancy_regex`, same ASCII-`\s`-spelled-out / default-`\b` reasoning as
/// `ANON_UPPER`. No possessive quantifier.
static ANON_LOWER: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"(?<=[ \t\n\x0B\f\r])anon\b(?!\.)").unwrap());

/// Java `StripAndStash.normaliseAnon` (StripAndStash.java:1268-1274). Working-string-only
/// (no `ctx.name` mutation, no warnings): TWO sequential, UNCONDITIONAL replacements —
/// title-case "Anon"/"Anon." and lower-case "anon" (not already followed by a dot) both
/// normalise to the canonical lower-case "anon." so the authorship
/// parser captures it as a real anonymous-author token — spot-checked: "Aus bus Anon." ->
/// authors=["anon."]; "Aus bus anon" -> authors=["anon."] (same result either input
/// casing).
fn normalise_anon(_ctx: &mut ParseContext, s: String) -> String {
    let s = fancy_replace_all(&ANON_UPPER, &s, |_| "anon.".to_string());
    fancy_replace_all(&ANON_LOWER, &s, |_| "anon.".to_string())
}

// ---- Step 38: stripColonConceptReference ----

/// Java COLON_CONCEPT_REFERENCE (StripAndStash.java:238-240):
/// `\s*:\s+(\p{Lu}[^:]*,\s*\d{3,4})\s*\.?\s*$`, `Pattern.UNICODE_CHARACTER_CLASS` -> keep
/// default Unicode `\s`/`\d`, ported verbatim. `[^:]` is a negated custom class, but
/// UNICODE_CHARACTER_CLASS means the whole pattern stays at the crate's Unicode default
/// anyway (no `(?-u:…)` anywhere), so the "invalid UTF-8 in byte mode" restriction that
/// forces atom-only scoping elsewhere in this file never comes up here. No
/// lookaround/backreference -> plain `regex` crate.
static COLON_CONCEPT_REFERENCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*:\s+(\p{Lu}[^:]*,\s*\d{3,4})\s*\.?\s*$").unwrap());

/// Java `StripAndStash.stripColonConceptReference` (StripAndStash.java:1276-1290). A
/// trailing ": Author, YYYY" botanical taxonomic-concept citation (e.g. "Vespa
/// emarginata Linnaeus, 1758: Fabricius, 1793" — the Linnaeus year is the original
/// publication, Fabricius is the sensu author) APPENDS the captured text to
/// `taxonomicNote`. The explicit ", YYYY" requirement keeps the simpler
/// ": SanctioningAuthor" form (e.g. "Boletus versicolor L. : Fr.", no year) out of this
/// strip — spot-checked: the Linnaeus/Fabricius example ->
/// `taxonomicNote="Fabricius, 1793"`, working string reduces to "Vespa emarginata
/// Linnaeus, 1758".
fn strip_colon_concept_reference(ctx: &mut ParseContext, s: String) -> String {
    if let Some(caps) = COLON_CONCEPT_REFERENCE.captures(&s) {
        let note = java_trim(caps.get(1).unwrap().as_str()).to_string();
        ctx.name.add_taxonomic_note(&note);
        let whole = caps.get(0).unwrap();
        return java_trim(&s[..whole.start()]).to_string();
    }
    s
}

// ---- Shared helper: normaliseLeadingAuct (used by steps 39 and 43) ----

/// Java LEADING_AUCT (StripAndStash.java:310): `^(Auct)`, no flags — literal, case-
/// sensitive "Auct" prefix. No shorthand atoms (`\s`/`\d`/`\w`/`\b`/`\p{...}`) at all ->
/// nothing to scope, ported verbatim (same as `DAGGER`/`HTML_TAG` in batch 2). No
/// lookaround/backreference -> plain `regex` crate.
static LEADING_AUCT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(Auct)").unwrap());

/// Java LEADING_AUCTT (StripAndStash.java:311): `^(Auctt)`, no flags. Same reasoning as
/// `LEADING_AUCT` -> ported verbatim, plain `regex` crate.
static LEADING_AUCTT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(Auctt)").unwrap());

/// Java's repeated `LEADING_AUCTT.matcher(LEADING_AUCT.matcher(x).replaceAll("auct"))
/// .replaceAll("auctt")` idiom (StripAndStash.java:1298, 1360): lower-cases a leading
/// title-case "Auct"/"Auctt" keyword to the canonical lower-case "auct."/"auctt."
/// convention (the trailing dot, if any, is untouched — already part of the input text).
/// Both `LEADING_AUCT`/`LEADING_AUCTT` are case-SENSITIVE, so this only fires on the
/// EXACT casing "Auct"/"Auctt" (capital A, the rest lower) — an all-lowercase note is
/// already correct and left alone, an ALL-CAPS note ("AUCTT.") is left untouched too
/// (neither pattern matches it) — spot-checked against the Java CLI oracle: "Aus bus
/// Auct." -> `taxonomicNote="auct."`, "Aus bus Auctt." -> `taxonomicNote="auctt."`.
/// Because the FIRST replacement already consumes (and lower-cases) any "Auct" prefix,
/// the second regex — which requires a literal capital "Auctt" — can only ever fire on
/// input that did NOT start with "Auct" in the first place, i.e. effectively never after
/// the first step runs; both steps are ported verbatim rather than collapsed into one,
/// since Java itself chains them this way at both call sites (`stripBracketedTaxNote`,
/// `stripTaxNote`) and a faithful port doesn't get to assume away a redundancy the
/// oracle itself carries.
fn normalise_leading_auct(note: &str) -> String {
    let step1 = LEADING_AUCT.replace(note, "auct");
    LEADING_AUCTT.replace(&step1, "auctt").into_owned()
}

// ---- Step 39: stripBracketedTaxNote ----

/// Java BRACKETED_TAX_NOTE (StripAndStash.java:123-125):
/// `\s*\[\s*((?:auctt?|sensu|sec|non|nec|misspelling|misapplied|misident)\b[^\]]*)\]\s*\.?\s*$`,
/// `Pattern.CASE_INSENSITIVE`. `[^\]]` is a negated custom class -> stays OUTSIDE any
/// `(?-u:…)` (`SIC_WITH_COMMENT`/`BRACKETED_NOM_NOTE` precedent from batches 1-2) ->
/// atom-only `\s`/`\b` scoping, not whole-wrap. No lookaround/backreference -> plain
/// `regex` crate.
static BRACKETED_TAX_NOTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?-u:\s*)\[(?-u:\s*)((?:auctt?|sensu|sec|non|nec|misspelling|misapplied|misident)(?-u:\b)[^\]]*)\](?-u:\s*)\.?(?-u:\s*)$",
    )
    .unwrap()
});

/// Java `StripAndStash.stripBracketedTaxNote` (StripAndStash.java:1292-1304). A trailing
/// "[auctt. misspelling for Eunoe]"-style bracket introduced by a taxonomic-concept
/// keyword — the ENTIRE bracket content becomes the taxonomic note (whitespace-collapsed,
/// leading/trailing-trimmed, then leading "Auct"/"Auctt" lower-cased via
/// `normalise_leading_auct`) and APPENDS to `taxonomicNote` — spot-checked: "Eunoa bus
/// Smith [auctt. misspelling for Eunoe]" -> `taxonomicNote="auctt. misspelling for
/// Eunoe"`, authors=["Smith"].
fn strip_bracketed_tax_note(ctx: &mut ParseContext, s: String) -> String {
    if let Some(caps) = BRACKETED_TAX_NOTE.captures(&s) {
        let trimmed = java_trim(caps.get(1).unwrap().as_str());
        let collapsed = WHITESPACE.replace_all(trimmed, " ");
        let note = normalise_leading_auct(&collapsed);
        ctx.name.add_taxonomic_note(&note);
        let whole = caps.get(0).unwrap();
        return java_trim(&s[..whole.start()]).to_string();
    }
    s
}

// ---- Step 40: stripParenTaxNote ----

/// Java PAREN_TAX_NOTE (StripAndStash.java:101-103):
/// `\s*\(\s*((?:nec|non|not)\s+[^)]+)\)\s*\.?\s*$`, `Pattern.CASE_INSENSITIVE`. `[^)]` is
/// a negated custom class -> atom-only `\s` scoping (same precedent as
/// `BRACKETED_TAX_NOTE` above). No lookaround/backreference -> plain `regex` crate.
static PAREN_TAX_NOTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?-u:\s*)\((?-u:\s*)((?:nec|non|not)(?-u:\s+)[^)]+)\)(?-u:\s*)\.?(?-u:\s*)$")
        .unwrap()
});

/// Java `StripAndStash.stripParenTaxNote` (StripAndStash.java:1334-1344). A trailing
/// "(nec/non/not …, YYYY)" parenthesised homonym citation — the WHOLE bracket wraps the
/// note (unlike step 40's sibling `PAREN_NOTE`/`LEADING_HOMONYM_PAREN`, ported further down
/// this file as part of `strip_authorship_markers` — those handle a homonym citation
/// embedded inside an authorship span, not this end-anchored standalone form) — APPENDS the
/// captured text (verbatim, trimmed) to `taxonomicNote` — spot-checked: "Aus bus Smith (non
/// Foo, 1850)" -> `taxonomicNote="non Foo, 1850"`, authors=["Smith"].
fn strip_paren_tax_note(ctx: &mut ParseContext, s: String) -> String {
    if let Some(caps) = PAREN_TAX_NOTE.captures(&s) {
        let note = java_trim(caps.get(1).unwrap().as_str()).to_string();
        ctx.name.add_taxonomic_note(&note);
        let whole = caps.get(0).unwrap();
        return java_trim(&s[..whole.start()]).to_string();
    }
    s
}

// ---- Step 41: stripSensuLatoRemainder ----

/// Java SENSU_LATO_REMAINDER (StripAndStash.java:91-92):
/// `\s+(s\.\s*l\.?|s\.\s*lat\.?|s\.\s*str\.?|s\.\s*ampl\.?)\s+(\S.*?)\s*$`, no flags
/// (deliberately case-sensitive — the Java comment: the marker is lower-case "s", so
/// uppercase author initials like "S. L. Schultes" are never mistaken for it). Has `\s`
/// (many) AND an unescaped wildcard `.` (inside the lazy `.*?`) -> atom-only ASCII
/// scoping (never whole-wrap, matching `MISSING_GENUS_NOTE_KEYWORD`'s precedent: whole-
/// wrapping would also flip `.`'s Unicode-scalar-vs-byte meaning). `\S` (negated
/// shorthand) CANNOT be ASCII-scoped at all — `(?-u:\S)` is rejected by `regex-syntax` with
/// the identical "pattern can match invalid UTF-8" error as a negated custom class (
/// confirmed empirically; same restriction as `[^…]`, just for the shorthand spelling) ->
/// `\S` stays at the crate's Unicode default, unscoped, like every other
/// can't-be-ASCII-scoped atom in this file. The lazy `.*?` needs no `fancy_regex`: the
/// plain `regex` crate is a Pike's-VM/Thompson-NFA linear-time automaton that natively
/// supports non-greedy quantifiers (laziness is a match-preference rule the automaton
/// applies directly, not a backtracking construct) — no lookaround/backreference at all
/// here -> plain `regex` crate.
static SENSU_LATO_REMAINDER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?-u:\s+)(s\.(?-u:\s*)l\.?|s\.(?-u:\s*)lat\.?|s\.(?-u:\s*)str\.?|s\.(?-u:\s*)ampl\.?)(?-u:\s+)(\S.*?)(?-u:\s*)$",
    )
    .unwrap()
});

/// Java `StripAndStash.stripSensuLatoRemainder` (StripAndStash.java:1306-1318). A
/// mid-string "s.l."/"s.lat."/"s.str."/"s.ampl." marker followed by trailing junk that
/// isn't part of the name (e.g. a truncated re-citation after a dash) — the marker
/// (whitespace-removed, lower-cased) APPENDS to `taxonomicNote` and the trailing junk
/// (trimmed) is parked in `ctx.pending_unparsed` (first-writer-wins), populated faithfully
/// for the downstream stages that consume it — spot-checked:
/// "Asplenium trichomanes L. s.lat. - Asplen trich" -> `taxonomicNote="s.lat."`,
/// `unparsed="- Asplen trich"` (the `unparsed` JSON field itself, like `state=PARTIAL`, is
/// assembled by Pipeline/Assemble from `pending_unparsed` — this step
/// only needs to stash the pending value).
fn strip_sensu_lato_remainder(ctx: &mut ParseContext, s: String) -> String {
    if let Some(caps) = SENSU_LATO_REMAINDER.captures(&s) {
        let marker = caps.get(1).unwrap().as_str();
        let note = WHITESPACE.replace_all(marker, "").to_lowercase();
        let remainder = java_trim(caps.get(2).unwrap().as_str()).to_string();
        ctx.name.add_taxonomic_note(&note);
        ctx.set_pending_unparsed(&remainder);
        let whole = caps.get(0).unwrap();
        return java_trim(&s[..whole.start()]).to_string();
    }
    s
}

// ---- Step 42: stripSensuStrictoSS ----

/// Java SENSU_STRICTO_SS (StripAndStash.java:96-97): `\s+s\.\s*s\.?(\s+\S.*?)?\s*$`, no
/// flags (case-sensitive, same reasoning as `SENSU_LATO_REMAINDER`). Has `\s` (several)
/// and `\S` plus an unescaped wildcard `.` -> same atom-only scoping as
/// `SENSU_LATO_REMAINDER` (`\S`/`.` left unscoped, `\s` atoms individually ASCII-scoped).
/// No lookaround/backreference -> plain `regex` crate.
static SENSU_STRICTO_SS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\s+)s\.(?-u:\s*)s\.?((?-u:\s+)\S.*?)?(?-u:\s*)$").unwrap());

/// Java `StripAndStash.stripSensuStrictoSS` (StripAndStash.java:1320-1332). A trailing
/// "s.s." ("s. s." also matches — `\s*` between the two "s."s) sensu-stricto marker
/// APPENDS the FIXED literal "s.s." (not the captured text, which may have had internal
/// spacing) to `taxonomicNote` — matching Java's `existing == null ? "s.s." : existing +
/// " s.s."` inline literal, not a call through `add_taxonomic_note` with a variable.
/// When followed by trailing junk (group 1 present), that junk (trimmed) is parked in
/// `ctx.pending_unparsed`, conditionally — Java's `if (m.group(1) != null)` guard, unlike
/// step 41's sibling where the remainder is unconditionally captured (there, group 2 is
/// required by the pattern itself; here group 1 is optional) — spot-checked: "Aus bus
/// s.s." -> `taxonomicNote="s.s."`, no unparsed; "Aus bus s.s. extra text here" ->
/// `taxonomicNote="s.s."`, `unparsed="extra text here"`.
fn strip_sensu_stricto_ss(ctx: &mut ParseContext, s: String) -> String {
    if let Some(caps) = SENSU_STRICTO_SS.captures(&s) {
        ctx.name.add_taxonomic_note("s.s.");
        if let Some(junk) = caps.get(1) {
            ctx.set_pending_unparsed(java_trim(junk.as_str()));
        }
        let whole = caps.get(0).unwrap();
        return java_trim(&s[..whole.start()]).to_string();
    }
    s
}

// ---- Step 43: stripTaxNote ----

// RULE: patterns on fancy_regex (backtracking) MUST keep Java's possessive/atomic
// quantifiers — only DROP possessives for patterns on the linear `regex` crate. TAX_NOTE
// has no possessive quantifier and no lookaround/backreference at all -> plain `regex`
// crate, nothing to preserve here.
/// Java TAX_NOTE (StripAndStash.java:68-84), `Pattern.CASE_INSENSITIVE` overall, WITH an
/// inline `(?-i:…)` case-SENSITIVE carve-out for the trailing s.l./s.str./s.lat./s.ampl.
/// alternative (lower-case "s", so uppercase author initials "S.L." aren't eaten —
/// mirrors the sibling `SENSU_LATO_REMAINDER`/`SENSU_STRICTO_SS` patterns' own
/// case-sensitivity). Has `\s`/`\b` AND `\p{Lu}` AND unescaped wildcards (`.*`, several)
/// -> atom-only ASCII scoping throughout (never whole-wrap) — `\s`/`\b` individually
/// `(?-u:…)`-scoped, `\p{Lu}` always Unicode, `.` left at default. The inline `(?-i:…)`
/// nests independently of the `(?-u:…)` atom scoping (both toggle unrelated flags, freely
/// combinable). No lookaround/backreference anywhere -> plain `regex` crate. Built via
/// `concat!` (one alternative per line, matching Java's own `"..." + "..."` source
/// layout) so it stays directly diffable against StripAndStash.java line-by-line, same
/// convention as `NOM_NOTE` in batch 2.
static TAX_NOTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)(?-u:\s+),?(?-u:\s*)(",
        r"auctt?(?-u:\b)\.?(?:[,.]?(?-u:\s).*)?",
        r"|sensu(?:(?-u:\s).*)?",
        r"|sec\.?(?:(?-u:\s).*)?",
        r"|nec(?-u:\b)(?:(?-u:\s).*)?",
        r"|nonn?\.?(?-u:\s+)\(?\p{Lu}.*",
        r"|emend(?-u:\b)\.?(?-u:\s+)\(?\p{Lu}.*",
        r"|fide(?-u:\b)\.?(?-u:\s+)\(?\p{Lu}.*",
        r"|according(?-u:\s+)to(?-u:\s+)\p{Lu}.*",
        r"|excl\.(?-u:\s+).*",
        r"|ss(?-u:\b)\.?(?-u:\s+).*",
        r"|(?-i:s\.(?-u:\s*)l\.?|s\.(?-u:\s*)str\.?|s\.(?-u:\s*)lat\.?|s\.(?-u:\s*)ampl\.?)",
        r")$",
    ))
    .unwrap()
});

/// Java INITIAL_DOT_SPACE (StripAndStash.java:314-315):
/// `\b(\p{Lu})\.\s+([\p{Ll}][\p{Ll}]{3,})`, no flags. Has `\b`/`\s` and `\p{Lu}`/`\p{Ll}`
/// -> atom-only ASCII scoping for `\b`/`\s`. No lookaround/backreference -> plain `regex`
/// crate; the replacement pulls both captures back via `$1.$2`, the SAME `$N` syntax Java
/// and the `regex` crate happen to share. IMPORTANT (verified against the real Java CLI
/// oracle, NOT just the Java source's own paraphrasing comment — see
/// `strip_tax_note`'s doc below): group 2 requires an ALL-LOWERCASE run of >= 4 letters,
/// so this only collapses "Initial. word" when the word is a bare lowercase token (e.g. a
/// species-epithet-shaped word) — a capitalised surname like "F. Schmidt" is NOT
/// collapsed (`\p{Ll}` rejects the leading capital "S"); "non F. europaeus" -> "non
/// F.europaeus" IS collapsed.
static INITIAL_DOT_SPACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\b)(\p{Lu})\.(?-u:\s+)([\p{Ll}][\p{Ll}]{3,})").unwrap());

/// Java `StripAndStash.stripTaxNote` (StripAndStash.java:1346-1367). The general,
/// end-anchored taxonomic-note anchor (auct./auctt./sensu/sec./nec/non/nonn./emend./
/// fide/according to/excl./ss/s.l./s.str./s.lat./s.ampl.) — the LAST of the six
/// taxonomic-note steps to run, so anything more specific was already peeled off by
/// `strip_colon_concept_reference`/`strip_bracketed_tax_note`/`strip_paren_tax_note`/
/// `strip_sensu_lato_remainder`/`strip_sensu_stricto_ss` above it. Applies
/// `INITIAL_DOT_SPACE` (collapses a bare-initial-plus-lowercase-word gap) then
/// `normalise_leading_auct` (lower-cases a leading title-case "Auct"/"Auctt") to the
/// captured text, then OVERWRITES `taxonomicNote` directly (`ctx.name.taxonomic_note =
/// Some(norm)` — Java's plain `setTaxonomicNote(norm)`, NO null-check/append, UNLIKE
/// steps 38-42 above) — this is deliberately the one taxonomic-note step in this batch
/// that does NOT call `add_taxonomic_note`. A guard skips entirely when the captured,
/// trimmed group is empty (Java: `if (!raw.isEmpty())`) — the outer match can still have
/// fired on pure separator text with nothing captured after trimming, in which case
/// neither the working string nor `taxonomicNote` should change. After stripping, any
/// now-trailing comma(s) left dangling are removed in a loop (re-trimming each time,
/// same idiom as `strip_nom_note` in batch 2) — spot-checked against the Java CLI oracle:
/// "Chlorobium phaeobacteroides Pfennig, 1968 emend. Imhoff, 2003" ->
/// `taxonomicNote="emend. Imhoff, 2003"`; "... emend Imhoff, 2003" (no dot) -> `"emend
/// Imhoff, 2003"`; "Eulima excellens Verkrüzen fide Paetel, 1887" -> `"fide Paetel,
/// 1887"`; "Procamallanus (Spirocamallanus) soodi Lakshmi & Kumari, 2001 nec (Gupta &
/// Masood, 1988)" -> `"nec (Gupta & Masood, 1988)"`; "Membranipora minuscula Canu, 1911
/// non Hincks, 1882" -> `"non Hincks, 1882"`; "Aus bus auct." -> `"auct."`; "Aus bus
/// Auct."/"Aus bus Auctt." -> `"auct."`/`"auctt."` (title-case lower-cased via
/// `normalise_leading_auct`); "Aus bus s.l." -> `"s.l."` (case-sensitive `(?-i:…)`
/// branch); uppercase "Aus bus Mill. S.L." does NOT match at all (author initials, not a
/// sensu-lato marker).
fn strip_tax_note(ctx: &mut ParseContext, s: String) -> String {
    let caps = match TAX_NOTE.captures(&s) {
        Some(c) => c,
        None => return s,
    };
    let raw = java_trim(caps.get(1).unwrap().as_str()).to_string();
    if raw.is_empty() {
        return s;
    }
    let with_dots = INITIAL_DOT_SPACE.replace_all(&raw, "$1.$2");
    let norm = normalise_leading_auct(&with_dots);
    ctx.name.taxonomic_note = Some(norm);
    let match_start = caps.get(0).unwrap().start();
    let mut result = java_trim(&s[..match_start]).to_string();
    while result.ends_with(',') {
        result.pop();
        result = java_trim(&result).to_string();
    }
    result
}

// ---- Step 44: stripAggregateSuffix ----

/// Java AGGREGATE (StripAndStash.java:138-141):
/// `(?:\s+(?:agg\.?|aggregate|species\s+group|species\s+complex|group|complex)|\s*-\s*group|\s*-\s*aggregate)\s*$`,
/// `Pattern.CASE_INSENSITIVE`. Has `\s` (many), no `\p{...}`, no unescaped wildcard ->
/// whole-wrap ASCII scope. No lookaround/backreference -> plain `regex` crate. NOTE:
/// `regexes::AGGREGATE` (the Phase 0 spike file) already has a same-named pattern, but it
/// predates this port's per-pattern flag rule and is NOT ASCII-scoped (default Unicode
/// `\s` throughout) — reusing it here would introduce a `\s`-scope divergence from Java,
/// so this is a fresh, correctly-scoped definition local to this module, same precedent
/// as `SIC`/`CORRIG` in batch 2 (`strip_sic_and_corrig`'s doc comment).
static AGGREGATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?-u:(?:\s+(?:agg\.?|aggregate|species\s+group|species\s+complex|group|complex)|\s*-\s*group|\s*-\s*aggregate)\s*)$",
    )
    .unwrap()
});

/// Java `StripAndStash.stripAggregateSuffix` (StripAndStash.java:1369-1377). A trailing
/// " agg."/"aggregate"/"species group"/"species complex"/"group"/"complex"/"-group"/
/// "-aggregate" suffix marks a species aggregate: sets `ctx.aggregate = true` (a direct
/// field write, matching Java's package-private `ctx.aggregate = true;` — NOT one of the
/// 10 downstream-independent gate fields; consumed by Assemble to
/// promote `rank` to `SPECIES_AGGREGATE`) and strips the suffix — spot-checked against
/// the Java CLI oracle (rank promotion itself is Assemble's job, so this only confirms
/// the working-string strip leaves a clean binomial in all three shapes):
/// "Achillea millefolium agg." / "... species group" / "...-group" all reduce the working
/// string to "Achillea millefolium".
fn strip_aggregate_suffix(ctx: &mut ParseContext, s: String) -> String {
    if let Some(m) = AGGREGATE.find(&s) {
        ctx.aggregate = true;
        return java_trim(&s[..m.start()]).to_string();
    }
    s
}

// ---------------------------------------------------------------------------------
// Batch 4 (steps 45-52): published-in family (page, in-press, in-author variants,
// IPNI, period-/comma-separated references) + manuscript marker. Ported (Phase 1
// Slice 2, batch 2d). This batch closes `publishedIn`/`publishedInPage`/
// `publishedInYear`/`manuscript` and (jointly with batch 2b's already-ported
// `stripNomNote`/`stripBracketedNomNote`, steps 29/30) `nomenclaturalNote`. Steps 47/
// 48 (`stripInAuthorInParens`/`stripInAuthorCitation`) APPEND to `publishedIn` via the
// new `ParsedName::add_published_in` — Java's inline `existing == null ? ref :
// existing + " " + ref` immediately followed by `setPublishedIn(combined)`; appending
// and re-deriving `publishedInYear` from the COMBINED string are the SAME Java call,
// not two. Steps 49/50/51 (`stripIpniCitation`/`stripPeriodSeparatedReference`/
// `stripCommaPrefixedReference`) OVERWRITE it directly via `set_published_in` (Java's
// plain `setPublishedIn(ref)`, no null-check — nothing upstream of any of them can
// have already populated a reference). Steps 46/49/52 APPEND to `nomenclaturalNote`
// via `add_nomenclatural_note`. New `ParseContext::set_pending_publication_year`
// (first-writer-wins, mirrors `set_pending_imprint_year`) is threaded through from
// steps 48/49/50 and consumed downstream by Pipeline (applied onto the combination
// authorship's year — see `pipeline::mod`'s own doc comment), same
// port-the-producer-and-consumer-together pattern already used for
// `pending_generic_author`/`pending_specific_author`. All worked examples below were
// spot-checked against the real Java CLI oracle, same convention as batches 1-2c.
// ---------------------------------------------------------------------------------

// ---- Step 45: stripPublishedPage ----

/// Java PUBLISHED_PAGE (StripAndStash.java:157-158): `\s*:\s*(\d+(?:[-–]\d+)?)\s*$`, no
/// flags. Has `\s`/`\d`, no `\p{...}`, no unescaped wildcard — but embeds a non-ASCII literal
/// (the en dash, `–`) inside the custom class `[-–]` -> same `IMPRINT_YEAR_QUOTED`/
/// `IMPRINT_YEAR_KEYWORD` precedent from batch 1: only the individual `\s`/`\d` atoms are
/// ASCII-scoped, `[-\x{2013}]` left outside any `(?-u:…)` group (`regex-syntax` rejects a
/// non-ASCII literal inside one — "Unicode not allowed here"). No lookaround/backreference ->
/// plain `regex` crate.
static PUBLISHED_PAGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?-u:\s*):(?-u:\s*)((?-u:\d+)(?:[-\x{2013}](?-u:\d+))?)(?-u:\s*)$").unwrap()
});

/// Java `StripAndStash.stripPublishedPage` (StripAndStash.java:1379-1388). A trailing page
/// reference (": 377", ": 12-18", or with the colon glued/spaced either side, e.g. ":29" /
/// " : 29") is pulled directly into `publishedInPage` (a plain field write —
/// `ParsedAuthorship.setPublishedInPage` has no side effect, unlike `setPublishedIn`) and
/// stripped — spot-checked against the Java CLI oracle: "Anolis marmoratus girafus LAZELL
/// 1964: 377" -> `publishedInPage="377"`, working string "Anolis marmoratus girafus LAZELL
/// 1964"; "Recilia truncatus Dash & Viraktamath, 1998a: 29" (and its glued/extra-spaced
/// variants ":29" / " : 29") all -> `publishedInPage="29"`. Runs BEFORE
/// `strip_in_author_citation` (step 48) so a "Smith, 1900: 12 in Editor" tail strips both
/// cleanly (Java's own comment).
fn strip_published_page(ctx: &mut ParseContext, s: String) -> String {
    if let Some(caps) = PUBLISHED_PAGE.captures(&s) {
        let whole = caps.get(0).unwrap();
        ctx.name.published_in_page = Some(caps[1].to_string());
        return java_trim(&s[..whole.start()]).to_string();
    }
    s
}

// ---- Step 46: stripInPress ----

/// Java IN_PRESS (StripAndStash.java:144-145): `\s+in\s+press\b\.?`,
/// `Pattern.CASE_INSENSITIVE`. Has `\s` (x2) and `\b`, no `\p{...}`, no unescaped wildcard ->
/// whole pattern ASCII-scoped. No lookaround/backreference -> plain `regex` crate. NOT
/// end-anchored in Java (no trailing `$`), so this can in principle match anywhere in the
/// string, not just at the very end — ported as-is (see the function doc below for why that
/// matters to how the strip is applied).
static IN_PRESS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?-u:\s+in\s+press\b\.?)").unwrap());

/// Java `StripAndStash.stripInPress` (StripAndStash.java:1390-1400). A bare " in press"
/// marker sets `manuscript = true` and APPENDS the literal "in press" to `nomenclaturalNote`
/// (`ParsedName::add_nomenclatural_note`, Java's inline `existing == null ? "in press" :
/// existing + " in press"`) — spot-checked against the Java CLI oracle: "Abies alba Mill. in
/// press" -> `manuscript=true`, `nomenclaturalNote="in press"`, working string "Abies alba
/// Mill.". Java's `Matcher.replaceFirst("")` splices out exactly the matched span (which
/// already includes the leading whitespace via the pattern's own `\s+`) with NO subsequent
/// `.trim()` — ported as a direct mid-string splice rather than a truncate-to-match-start, to
/// stay faithful to that (the pattern isn't `$`-anchored, so it need not sit at the very end).
fn strip_in_press(ctx: &mut ParseContext, s: String) -> String {
    if let Some(m) = IN_PRESS.find(&s) {
        ctx.name.manuscript = true;
        ctx.name.add_nomenclatural_note("in press");
        return format!("{}{}", &s[..m.start()], &s[m.end()..]);
    }
    s
}

// ---- Step 47: stripInAuthorInParens ----

/// Java IN_AUTHOR_IN_PARENS (StripAndStash.java:152-154):
/// `\(([^()]*?)\s+(?:in|apud)\s+(\p{Lu}[^()]*?)\)`, `Pattern.UNICODE_CHARACTER_CLASS` -> keep
/// default Unicode `\s`, ported verbatim. Both capture groups are LAZY (`[^()]*?`) — NO
/// lookaround, NO backreference -> the plain `regex` crate suffices (its leftmost-first match
/// semantics reproduce Java's backtracking result exactly for this lookaround-free,
/// backreference-free shape; confirmed empirically in an isolated scratch-crate spike against
/// the exact worked example below, not just assumed). group(1) = basionym author span,
/// group(2) = publication reference.
static IN_AUTHOR_IN_PARENS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(([^()]*?)\s+(?:in|apud)\s+(\p{Lu}[^()]*?)\)").unwrap());

/// Java IN_AUTHOR_YEAR (StripAndStash.java:241-242): `,?\s*(\d{3,4})\s*\.?\s*$`, no flags. Has
/// `\s`/`\d`, no `\p{...}`, no unescaped wildcard -> whole pattern ASCII-scoped. Shared between
/// `strip_in_author_in_parens` (this step) and `strip_in_author_citation` (step 48, below),
/// same as Java's own single static field.
static IN_AUTHOR_YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:,?\s*(\d{3,4})\s*\.?\s*)$").unwrap());

/// Java `StripAndStash.stripInAuthorInParens` (StripAndStash.java:1402-1428). An "in
/// <publication>" citation INSIDE a parenthesised basionym ("Hypsicera femoralis (Geoffroy in
/// Fourcroy, 1785)") is rewritten to just the basionym author, MOVING the publication's own
/// year onto the basionym when the basionym didn't already carry one — so it survives as a
/// normal "(Author, year)" basionym for the authorship parser and code
/// inference — and the publication text APPENDS to `publishedIn` (`add_published_in`, same
/// append-is-setPublishedIn-on-the-combined-string semantics described on that method). Spot-
/// checked against the Java CLI oracle: -> "Hypsicera femoralis (Geoffroy, 1785)" with
/// `publishedIn="Fourcroy, 1785"`, `publishedInYear=1785`. When the basionym ALREADY has its
/// own year ("(Smith, 1780 in Jones, 1900)"), that year is left alone — no double year is
/// appended, matching the `!YEAR_4DIGIT.matcher(basPart).find()` guard (`YEAR_4DIGIT`, batch
/// 1). A MID-STRING SPLICE (`s[..start] + newParens + s[end..]`), NOT a truncation — this can
/// fire on a basionym that isn't at the very end of the input, unlike most other steps in this
/// file. Guarded on a non-empty basionym author AND a reference at least 2 chars long (Java
/// `String.length()`, ported as `.chars().count()` — no astral characters expected in
/// bibliographic reference text).
fn strip_in_author_in_parens(ctx: &mut ParseContext, s: String) -> String {
    if let Some(caps) = IN_AUTHOR_IN_PARENS.captures(&s) {
        let whole = caps.get(0).unwrap();
        let bas_part = java_trim(&caps[1]).to_string();
        let reference = java_trim(&caps[2]).to_string();
        if !bas_part.is_empty() && reference.chars().count() >= 2 {
            ctx.name.add_published_in(&reference);
            let new_parens = match IN_AUTHOR_YEAR.captures(&reference) {
                Some(ym) if !YEAR_4DIGIT.is_match(&bas_part) => {
                    format!("({bas_part}, {})", &ym[1])
                }
                _ => format!("({bas_part})"),
            };
            return format!("{}{}{}", &s[..whole.start()], new_parens, &s[whole.end()..]);
        }
    }
    s
}

// ---- Step 48: stripInAuthorCitation ----

/// Java IN_AUTHOR (StripAndStash.java:148-149): `\s+(?:in|apud)\s+([\p{Lu}][^\s].*)$`, no
/// flags. `\s` (x2) ASCII-only (no `UNICODE_CHARACTER_CLASS`); `\p{Lu}` always Unicode;
/// `[^\s]` is a negated PREDEFINED shorthand class (same as a bare `\S`) — under Java's default
/// (non-Unicode) flags this means "not one of the 6 ASCII whitespace chars", spelled out here
/// as the literal negated ASCII whitespace class `[^\t\n\x0B\f\r ]` at the crate's
/// Unicode-default scope (same `SEROVAR_BARE` precedent from batch 1 — `(?-u:[^\s])`/
/// `(?-u:\S)` are both rejected by `regex-syntax` as "pattern can match invalid UTF-8"); `.*`
/// unescaped wildcard, Unicode default. No lookaround/backreference -> plain `regex` crate.
static IN_AUTHOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?-u:\s+)(?:in|apud)(?-u:\s+)(\p{Lu}[^\t\n\x0B\f\r ].*)$").unwrap()
});

/// Java IN_AUTHOR_PAREN_YEAR (StripAndStash.java:243): `\((\d{4})\)`, no flags. Has `\d`, no
/// `\p{...}`, no unescaped wildcard (the parens are literal, escaped) -> whole pattern
/// ASCII-scoped.
static IN_AUTHOR_PAREN_YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\((\d{4})\))").unwrap());

/// Java `StripAndStash.stripInAuthorCitation` (StripAndStash.java:1430-1462). A trailing " in
/// <Author>" / " apud <Author>" tail ("Cantharus lineolatus Valenciennes in Cuvier &
/// Valenciennes, 1830", "Busk in Chimonides, 1987", "Small apud Britton & Wilson") is the
/// publication reference for the name — APPENDS to `publishedIn` (`add_published_in`) and
/// records a code-neutral `ctx.pending_year`/`pending_year_from_publication` via
/// `ParseContext::set_pending_publication_year` (first-writer-wins; a PARENTHESISED year is
/// tried FIRST so it takes precedence over a bare trailing year when both are present in the
/// same reference, e.g. "Kirchner (1988), Taxon 37: 5" pins 1988, not 5). Runs BEFORE the IPNI
/// / period-/comma-separated-reference patterns (steps 49-51) so an "Author in Source, Title
/// (Year)" tail is fully consumed here rather than partially matched by one of those. A
/// trailing sentence-final period after a closing paren is dropped (`ref.endsWith(").")`); a
/// period after anything else (an author abbreviation like "Fleisch.") is kept. Guarded on the
/// reference being at least 2 chars (Java `String.length()` -> `.chars().count()`, same as
/// step 47). All example refs above spot-checked against the Java CLI oracle, including the
/// "de"-particle author-list negative case ("Yin, Z.W. de Beer & Wingf." must NOT itself be
/// swallowed here — it isn't, since `IN_AUTHOR` requires a literal " in "/" apud " word, which
/// "de Beer" doesn't contain).
fn strip_in_author_citation(ctx: &mut ParseContext, s: String) -> String {
    if let Some(caps) = IN_AUTHOR.captures(&s) {
        let whole = caps.get(0).unwrap();
        let mut reference = java_trim(&caps[1]).to_string();
        if reference.ends_with(").") {
            reference.pop();
            reference = java_trim(&reference).to_string();
        }
        if reference.chars().count() >= 2 {
            ctx.name.add_published_in(&reference);
            if let Some(pyear) = IN_AUTHOR_PAREN_YEAR.captures(&reference) {
                ctx.set_pending_publication_year(&pyear[1]);
            }
            if let Some(ym) = IN_AUTHOR_YEAR.captures(&reference) {
                ctx.set_pending_publication_year(&ym[1]);
            }
            return java_trim(&s[..whole.start()]).to_string();
        }
    }
    s
}

// ---- Step 49: stripIpniCitation ----

/// Java IPNI_CITATION (StripAndStash.java:244-246):
/// `(?<=\s)[\p{Lu}][\p{L}.]+,\s+(.+\(\d{4}\))\.?\s*$`, `Pattern.UNICODE_CHARACTER_CLASS` -> keep
/// default Unicode `\s`, no ASCII scoping needed. LOOKBEHIND `(?<=\s)` (the author span must be
/// preceded by whitespace, checked but not consumed) -> needs `fancy_regex` (the `regex` crate
/// has no lookaround at all). Ported verbatim.
static IPNI_CITATION: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r"(?<=\s)[\p{Lu}][\p{L}.]+,\s+(.+\(\d{4}\))\.?\s*$").unwrap());

/// Java IPNI_EMBEDDED_NOM_NOTE (StripAndStash.java:247-251), `Pattern.CASE_INSENSITIVE` (no
/// `UNICODE_CHARACTER_CLASS`, so Java's own `\s`/`\b`/`\d` here are ASCII-only). Trailing
/// LOOKAHEAD `(?=\(\d{4}\))` (the year parens are checked but not consumed) -> needs
/// `fancy_regex`. `fancy_regex` has no ASCII mode at all (`(?-u:…)` is a hard parse error,
/// unconditionally — see `GREEK_MARKER`'s doc comment in batch 1) -> `\s` spelled out as the
/// literal ASCII whitespace set `[ \t\n\x0B\f\r]`; `\d` spelled out as `[0-9]`
/// (`DOT_BEFORE_ALNUM` precedent, batch 2b — a literal class is unaffected by Unicode mode
/// either way, unlike a shorthand); `\b` left as `fancy_regex`'s only option (Unicode
/// word-boundary), the same rare, forced (not faithfulness-lapse) divergence as
/// `STRAIN_DESIGNATION` (batch 1). Built via `concat!` mirroring Java's own `"..." + "..."`
/// layout, one alternative per line, so it stays directly diffable against StripAndStash.java.
static IPNI_EMBEDDED_NOM_NOTE: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(concat!(
        r"(?i)[ \t\n\x0B\f\r]+((?:in[ \t\n\x0B\f\r]+obs\b\.?,?[ \t\n\x0B\f\r]*)?pro[ \t\n\x0B\f\r]+syn\b\.?",
        r"|nom\b\.?(?:[ \t\n\x0B\f\r]+[a-zA-Z][a-zA-Z.]*)*",
        r"|comb\b\.?(?:[ \t\n\x0B\f\r]+[a-zA-Z][a-zA-Z.]*)*",
        r"|orth\b\.?(?:[ \t\n\x0B\f\r]+[a-zA-Z][a-zA-Z.]*)*)[ \t\n\x0B\f\r]*(?=\([0-9]{4}\))",
    ))
    .unwrap()
});

/// Java IPNI_YEAR (StripAndStash.java:252): `\((\d{4})\)\s*\.?\s*$`, no flags -> whole pattern
/// ASCII-scoped.
static IPNI_YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\((\d{4})\)\s*\.?\s*)$").unwrap());

/// Java `StripAndStash.stripIpniCitation` (StripAndStash.java:1464-1490). An IPNI-style
/// citation ("Kirchn., Annals and Magazine of Natural History (1988).") — an author span,
/// comma, then a publication title ending in a parenthesised year — OVERWRITES `publishedIn`
/// (`set_published_in`, Java's plain `setPublishedIn(ref)` with no null-check, since nothing
/// upstream can have already populated a reference by the time this step's pattern can match)
/// and records the pending publication year. Before overwriting, any nom-note keyword EMBEDDED
/// in the reference text just before the year parens ("Taxon nom. illeg. (1988)", "Taxon pro
/// syn. (1988)", "Taxon in obs., pro syn. (1988)", "Taxon comb. nov. (1988)") is spliced out
/// into `nomenclaturalNote` (APPEND, `add_nomenclatural_note`) and the reference is
/// re-squished (collapse the resulting double space via `MULTI_SPACE`, batch 1, then
/// `java_trim`) — all four spot-checked against the Java CLI oracle. Truncates the working
/// string to `pm.start(1)` (the START OF GROUP 1, NOT the whole match — the lookbehind makes
/// group 0 start at the author's own first letter, so truncating there would drop the author
/// too) then a dangling comma left over from "Author., " is explicitly stripped.
fn strip_ipni_citation(ctx: &mut ParseContext, s: String) -> String {
    let caps = match IPNI_CITATION.captures(&s) {
        Ok(Some(c)) => c,
        _ => return s,
    };
    let group1 = caps.get(1).unwrap();
    let mut reference = java_trim(group1.as_str()).to_string();
    if let Ok(Some(nm)) = IPNI_EMBEDDED_NOM_NOTE.captures(&reference) {
        let note_match = nm.get(1).unwrap();
        ctx.name
            .add_nomenclatural_note(java_trim(note_match.as_str()));
        let spliced = format!(
            "{} {}",
            &reference[..note_match.start()],
            &reference[note_match.end()..]
        );
        reference = java_trim(&MULTI_SPACE.replace_all(&spliced, " ")).to_string();
    }
    ctx.name.set_published_in(&reference);
    if let Some(ym) = IPNI_YEAR.captures(&reference) {
        ctx.set_pending_publication_year(&ym[1]);
    }
    let mut result = java_trim(&s[..group1.start()]).to_string();
    if result.ends_with(',') {
        result.pop();
        result = java_trim(&result).to_string();
    }
    result
}

// ---- Step 50: stripPeriodSeparatedReference ----

// RULE: patterns on fancy_regex (backtracking) MUST keep Java's possessive/atomic
// quantifiers — only DROP possessives for patterns on the linear `regex` crate.
/// Java PERIOD_SEPARATED_REFERENCE (StripAndStash.java:253-266),
/// `Pattern.UNICODE_CHARACTER_CLASS` -> keep default Unicode `\s`/`\p{...}` throughout, no
/// ASCII scoping needed. Internal NEGATIVE LOOKAHEAD (`(?!(?:of|in|de|et|the|und|für)\b)`,
/// excluding the connector words from the filler run) -> needs `fancy_regex` (the `regex`
/// crate has no lookaround at all). The POSSESSIVE quantifier (`*+`) wrapping that same
/// lookahead-guarded group is KEPT here as `*+` (see the RULE above): `fancy_regex` IS a
/// backtracking engine, and Java's own comment (StripAndStash.java:255-257) says the
/// possessive exists so "this can't blow up on a long connector-free run", so keeping it is
/// the faithful port of Java's own quantifier plus defence-in-depth. `fancy_regex` 0.14
/// parses `*+` natively (compiling it to an atomic group), so restoring it is a straight
/// verbatim port, not a restructuring.
///
/// NOTE — unlike `NOM_NOTE` (batch 2b), the possessive here is NOT strictly load-bearing:
/// this pattern's filler group is `\s+ <word>` (each iteration must consume one or more
/// whitespace chars THEN a letter-led word, and the following mandatory tail also opens
/// with `\s+`), so every partition of a whitespace-delimited run is unambiguous and plain
/// greedy `*` cannot backtrack exponentially. Measured directly (isolated scratch-crate
/// spike): `*+` and plain `*` are BOTH linear and near-identical here — ~735µs each on a
/// 640-word connector-free run, growing linearly, no cliff. Contrast `NOM_NOTE`, whose
/// inner `[\s.&]*[a-z][a-z.]*` genuinely allows ambiguous partitions: there greedy `*`
/// explodes to ~18ms (backtrack-limit no-match) while `*+` stays microseconds — THAT is a
/// load-bearing possessive. So `*+` is retained here for faithfulness + defence-in-depth,
/// not because dropping it would ReDoS; the regression test below is a linearity guard, not
/// proof of an averted cliff. Built via `concat!` mirroring Java's own `"..." + "..."`
/// layout.
static PERIOD_SEPARATED_REFERENCE: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(concat!(
        r"\s+[\p{Lu}][\p{L}]{2,}\.\s+",
        r"([\p{Lu}][\p{Ll}]{2,}[\p{L}.]*(?:\s+(?!(?:of|in|de|et|the|und|f\x{fc}r)\b)(?:[\p{Lu}][\p{L}.]+|[\p{Ll}][\p{L}]+))*+",
        r"\s+(?:of|in|de|et|the|und|f\x{fc}r)\s+.*)$",
    ))
    .unwrap()
});

/// Java PAGE_RANGE_TEST (StripAndStash.java:351): `.*\b\d{3,}-\d{3,}\b.*`, no flags. Has `\b`/
/// `\d` (x2 each), unescaped wildcard `.*` bookends -> only `\b`/`\d` ASCII-scoped. Called via
/// `.matches()` on a `.*CORE.*` shape -> RESTRUCTURED to an unanchored `is_match` on the core
/// alone (same precedent as `GREEK_MARKER_TEST`/`MANUSCRIPT_KEYWORD`/… — no name string embeds
/// a newline, so `.` matching "any non-newline char" makes the `.*` bookends a no-op either
/// way).
static PAGE_RANGE_TEST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\b\d{3,}-\d{3,}\b)").unwrap());

/// Java PERIOD_REF_YEAR (StripAndStash.java:267): `\b(\d{4})\b`, no flags -> whole pattern
/// ASCII-scoped. `.find()` takes the FIRST (leftmost) match. Distinct from `ParsedName::
/// set_published_in`'s OWN separate year derivation (which takes the LAST year-shaped match in
/// the full `publishedIn` string) — this extraction only ever feeds `ctx.pending_year`, not
/// `publishedInYear` directly.
static PERIOD_REF_YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?-u:\b(\d{4})\b)").unwrap());

/// Java `StripAndStash.stripPeriodSeparatedReference` (StripAndStash.java:1492-1521). A
/// "Surname. <Reference Title> ... <year> ..." citation — recognised by an English/Latin
/// preposition ("of"/"in"/"de"/"et"/"the"/"und"/"für") inside the title, rare inside author
/// names — OVERWRITES `publishedIn` (`set_published_in`, Java's plain `setPublishedIn(ref)`,
/// no null-check). A reference containing a numeric PAGE RANGE ("1658-1662") is a full
/// bibliographic citation whose trailing number is ambiguous with pagination, so the year is
/// NOT propagated and the strip is flagged `NOMENCLATURAL_REFERENCE` instead; a reference
/// without a page range propagates the first 4-digit year it contains as the pending
/// publication year, no warning. Truncates the working string to `pm.start(1)` (group 1's
/// start, keeping the leading "Surname." span — mirrors `strip_ipni_citation`'s identical
/// group(1)-not-group(0) truncation), then strips a trailing period LEFT OVER from the
/// surname's own abbreviation dot (`if (s.endsWith(".")) … .trim()` — WITH a re-trim here,
/// unlike the earlier `ref`-side period strip below, which has none: Java's own
/// `ref.substring(0, ref.length() - 1)` with no following `.trim()`). All three shapes
/// (page-range / clean-year / author-particle false-positive) spot-checked against the Java
/// CLI oracle.
fn strip_period_separated_reference(ctx: &mut ParseContext, s: String) -> String {
    let caps = match PERIOD_SEPARATED_REFERENCE.captures(&s) {
        Ok(Some(c)) => c,
        _ => return s,
    };
    let group1 = caps.get(1).unwrap();
    let mut reference = java_trim(group1.as_str()).to_string();
    if reference.ends_with('.') {
        reference.pop();
    }
    ctx.name.set_published_in(&reference);
    if PAGE_RANGE_TEST.is_match(&reference) {
        ctx.name.add_warning(warnings::NOMENCLATURAL_REFERENCE);
    } else if let Some(ym) = PERIOD_REF_YEAR.captures(&reference) {
        ctx.set_pending_publication_year(&ym[1]);
    }
    let mut result = java_trim(&s[..group1.start()]).to_string();
    if result.ends_with('.') {
        result.pop();
        result = java_trim(&result).to_string();
    }
    result
}

// ---- Step 51: stripCommaPrefixedReference ----

/// Java COMMA_PREFIXED_REFERENCE (StripAndStash.java:268-276),
/// `Pattern.UNICODE_CHARACTER_CLASS` -> keep default Unicode `\s`/`\p{...}` throughout, no
/// ASCII scoping needed. NO lookaround, NO backreference anywhere (unlike its
/// `PERIOD_SEPARATED_REFERENCE` sibling above: the connector words are excluded from the
/// filler via an explicit alternation branch — `on|and|for` are allowed lowercase filler
/// words, `of|in|de|et|the|und|für` are not — rather than a negative lookahead) -> the plain
/// `regex` crate suffices. Java's possessive quantifier (`*+`) is DROPPED here (ported as
/// plain `*`) per this port's RULE (see `strip_period_separated_reference`'s doc comment
/// above): the `regex` crate is an automaton (Thompson NFA/DFA simulation) with NO
/// backtracking at all, so it is always linear-time in the input length regardless of
/// possessive-or-not — there is categorically no catastrophic-backtracking failure mode
/// possible on this engine, so the possessive carries no meaning and dropping it is a
/// provably-safe restructuring (whereas `PERIOD_SEPARATED_REFERENCE`, on the backtracking
/// `fancy_regex` engine, keeps its `*+` for faithfulness even though — as that static's own
/// NOTE measures — its particular shape also happens to stay linear). (Confirmed empirically
/// during this step's TDD spike that the `regex` crate does not even reject a literal `*+` —
/// it silently reinterprets it as nested repetition `(X*)+`, which happens to match the same
/// language as plain `X*` for this shape but is a confusing way to spell "no possessive
/// semantics exist on this engine"; writing plain `*` is the honest, idiomatic port. A
/// 200-word connector-free run completes in well under 1ms either way — there was never a
/// ReDoS risk to begin with on this engine.) Built via `concat!` mirroring Java's own
/// `"..." + "..."` layout.
static COMMA_PREFIXED_REFERENCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"\s+[\p{Lu}][\p{L}.]+,\s+",
        r"([\p{Lu}][\p{Ll}]{2,}[\p{L}.]*(?:\s+(?:[\p{Lu}][\p{L}.]+|on|and|for))*",
        r"\s+(?:of|in|de|et|the|und|f\x{fc}r)\s+.*)$",
    ))
    .unwrap()
});

/// Java `StripAndStash.stripCommaPrefixedReference` (StripAndStash.java:1523-1540). An
/// "Author(s), <Reference Title> …" citation with NO period after the author span (unlike step
/// 50's `PERIOD_SEPARATED_REFERENCE`) — the title must contain a recognisable connector
/// ("of"/"in"/"de"/"et"/"the"/"und"/"für") so a comma-separated co-author list isn't mistaken
/// for a reference — OVERWRITES `publishedIn` (`set_published_in`) and ALWAYS flags
/// `NOMENCLATURAL_REFERENCE` (unlike step 50, the year is never propagated onto the
/// combination authorship for this form — Java's own comment: the title's year is the
/// publication year of the article, not a zoological/botanical author-year citation).
/// Truncates to `pm.start(1)` (group 1's start, same "keep the author prefix" reasoning as
/// steps 49/50) then strips a dangling trailing comma.
fn strip_comma_prefixed_reference(ctx: &mut ParseContext, s: String) -> String {
    if let Some(caps) = COMMA_PREFIXED_REFERENCE.captures(&s) {
        let group1 = caps.get(1).unwrap();
        let reference = java_trim(group1.as_str()).to_string();
        ctx.name.set_published_in(&reference);
        let mut result = java_trim(&s[..group1.start()]).to_string();
        if result.ends_with(',') {
            result.pop();
            result = java_trim(&result).to_string();
        }
        ctx.name.add_warning(warnings::NOMENCLATURAL_REFERENCE);
        return result;
    }
    s
}

// ---- Step 52: stripManuscriptMarker ----

/// Java MANUSCRIPT_MARKER (StripAndStash.java:277-279):
/// `\s*,?\s+(ined\.?|ms\.?|msc\.?|unpublished)\s*$`, `Pattern.CASE_INSENSITIVE` (no
/// `UNICODE_CHARACTER_CLASS`) -> `\s` ASCII-only. No `\p{...}`, no unescaped wildcard -> whole
/// pattern ASCII-scoped. No lookaround/backreference -> plain `regex` crate.
static MANUSCRIPT_MARKER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?-u:\s*,?\s+(ined\.?|ms\.?|msc\.?|unpublished)\s*)$").unwrap()
});

/// Java `StripAndStash.stripManuscriptMarker` (StripAndStash.java:1542-1555). A trailing
/// manuscript marker ("ined."/"ms."/"msc."/"unpublished", any case, with an optional leading
/// comma) sets `manuscript = true` and APPENDS the LOWER-CASED tag to `nomenclaturalNote`
/// (`add_nomenclatural_note`) — spot-checked against the Java CLI oracle: "Acacia bicolor
/// Bojer ms." -> `manuscript=true`, `nomenclaturalNote="ms."`, working string "Acacia bicolor
/// Bojer". Runs AFTER `strip_in_author_citation` (step 48) so a trailing "Busk ms in
/// Chimonides, 1987" cleanly strips both (the in-author tail first, leaving "Busk ms" for this
/// step to finish).
fn strip_manuscript_marker(ctx: &mut ParseContext, s: String) -> String {
    if let Some(caps) = MANUSCRIPT_MARKER.captures(&s) {
        let whole = caps.get(0).unwrap();
        let tag = caps[1].to_lowercase();
        ctx.name.manuscript = true;
        ctx.name.add_nomenclatural_note(&tag);
        return java_trim(&s[..whole.start()]).to_string();
    }
    s
}

// ---------------------------------------------------------------------------------
// Batch 5 / 2e (steps 53-55, THE FINAL StripAndStash batch): suprarank prefix, leading
// infrageneric marker, phrase name. Landing this batch flips the golden-harness gate
// (`tests/parse_golden.rs`) from a deferred baseline print to a real `assert_eq!(…, 0)`
// over the 10 downstream-independent fields — see that file's own module doc.
//
// NOTE: Java's separate `stripAuthorshipMarkers` auxiliary-authorship reimplementation is
// NOT part of this ordered `run()` dispatch at all (see the investigation §4) — it is only
// ever invoked on the caller-supplied `authorship` string, from `pipeline::mod`'s
// aux-authorship path. It is NOT ported here (alongside these three steps); it's ported
// later in this same file, as its own separate, shorter, ordered step list — see
// `strip_authorship_markers` and the section doc comment above it (Phase 1 Slice 4 Task 4).
//
// This batch also extends `model::enums::Rank` with 18 new variants these three steps
// reference — the suprageneric family-group ranks for step 53, plus SUBGENUS and the
// botanical/zoological section/series pairs for step 54's `RANK_MARKER_MAP_INFRAGENERIC`
// subset + `BOT_TO_ZOOL` remap (see `Rank`'s own doc comment for the itemised list and
// exact Java source line references). Step 55 needs no new variants: SPECIES/SUBSPECIES/
// VARIETY/FORM already exist from earlier batches.
// ---------------------------------------------------------------------------------

// ---- Step 53: stripSupraRankPrefix ----

/// Java `SUPRA_RANK_MARKERS` (StripAndStash.java:1693-1701): the suprageneric
/// family-group rank markers `SUPRA_RANK_PREFIX` (below) can capture as its group 1,
/// mapped to the `Rank` they pin. MINIMAL port covering exactly the 8 distinct words the
/// regex's own alternation (`subfam(?:ily)?|subtrib(?:e)?|supertrib|infratrib|trib(?:e)?`)
/// can produce — a case-folded `match`, not a `HashMap`, following
/// `is_known_infraspecific_marker`'s (batch 1) precedent for a small fixed
/// string-to-value lookup. `word` must already be lower-cased by the caller (mirrors
/// Java's own `pm.group(1).toLowerCase()` immediately before the map lookup).
fn supra_rank_marker(word: &str) -> Option<Rank> {
    match word {
        "subfam" | "subfamily" => Some(Rank::Subfamily),
        "trib" | "tribe" => Some(Rank::Tribe),
        "subtrib" | "subtribe" => Some(Rank::Subtribe),
        "supertrib" => Some(Rank::Supertribe),
        "infratrib" => Some(Rank::Infratribe),
        _ => None,
    }
}

/// Java SUPRA_RANK_PREFIX (StripAndStash.java:1725-1729):
/// `^(?:[\p{Lu}][\p{L}]{2,}\s+)?(subfam(?:ily)?|subtrib(?:e)?|supertrib|infratrib|trib(?:e)?)\.?\s+(?=[\p{Lu}])`,
/// `Pattern.UNICODE_CHARACTER_CLASS | Pattern.CASE_INSENSITIVE` -> keep default Unicode
/// `\s`, add `(?i)`. Trailing LOOKAHEAD `(?=[\p{Lu}])` (the following capitalised name is
/// checked but not consumed, so it stays in the returned remainder) -> needs `fancy_regex`
/// (the `regex` crate has no lookaround at all). No nested/ambiguous quantifiers anywhere
/// in this pattern (the leading family-group is merely optional, not repeated) — no ReDoS
/// exposure to weigh, unlike some of the batch 2d fancy_regex patterns.
static SUPRA_RANK_PREFIX: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(
        r"(?i)^(?:[\p{Lu}][\p{L}]{2,}\s+)?(subfam(?:ily)?|subtrib(?:e)?|supertrib|infratrib|trib(?:e)?)\.?\s+(?=[\p{Lu}])",
    )
    .unwrap()
});

/// Java `StripAndStash.stripSupraRankPrefix` (StripAndStash.java:1557-1570). A leading
/// "<Family> <suprageneric-rank-marker> <Name> [Author …]" ("Poaceae subtrib.
/// Scolochloinae Soreng") — or a bare marker with no family prefix ("subtrib.
/// Scolochloinae Soreng") — pins `ctx.name.rank` from the marker and strips BOTH the
/// optional family prefix and the marker itself (`s[whole.end()..]`, not just past the
/// marker), leaving the inner uninomial (plus any trailing author) for the rest of the
/// pipeline to parse. Unconditional overwrite: Java's `ctx.name.setRank(r)` here carries
/// no "only if unset" guard, unlike step 54's `code` handling below. Both the bare-marker
/// and family-prefixed forms spot-checked against the Java CLI oracle: "subtrib.
/// Scolochloinae Soreng" and "Poaceae subtrib. Scolochloinae Soreng" both come back
/// `rank=SUBTRIBE`, `uninomial=Scolochloinae` (the family prefix, when present, is
/// discarded entirely, not preserved anywhere in the parsed output).
fn strip_supra_rank_prefix(ctx: &mut ParseContext, s: String) -> String {
    if let Ok(Some(caps)) = SUPRA_RANK_PREFIX.captures(&s) {
        let marker = caps.get(1).unwrap().as_str().to_lowercase();
        if let Some(r) = supra_rank_marker(&marker) {
            ctx.name.rank = r;
            let whole = caps.get(0).unwrap();
            return java_trim(&s[whole.end()..]).to_string();
        }
    }
    s
}

// ---- Step 54: stripLeadingInfragenericMarker ----

/// Java `RankUtils.RANK_MARKER_MAP_INFRAGENERIC` (RankUtils.java:90-106) — MINIMAL subset
/// port covering exactly the 13 distinct words `LEADING_INFRAGEN_MARKER` (below) can
/// capture as its group 1. The full Java map also auto-derives an entry for every
/// non-GENUS `Rank::isGenusGroup()` constant's own marker text
/// (`RankUtils.buildRankMarkerMap`) plus a handful of unrelated explicit aliases
/// (`supraser`, …) that this regex's own alternation can never produce — out of scope
/// here per this port's "add only what these steps need" brief. Case-folded `match`, same
/// style as `supra_rank_marker` above; `word` must already be lower-cased by the caller.
fn infrageneric_marker_rank(word: &str) -> Option<Rank> {
    match word {
        "subg" | "subgen" | "subgenus" => Some(Rank::Subgenus),
        "sect" | "section" => Some(Rank::SectionBotany),
        "subsect" | "subsection" => Some(Rank::SubsectionBotany),
        "supersect" | "suprasect" => Some(Rank::SupersectionBotany),
        "ser" | "series" => Some(Rank::SeriesBotany),
        "subser" | "subseries" => Some(Rank::SubseriesBotany),
        _ => None,
    }
}

/// Java `BOT_TO_ZOOL` (StripAndStash.java:1706-1712): botanical-flavoured infrageneric
/// ranks that have a same-NAMED zoological counterpart at a DIFFERENT nominal level
/// (zoological "section"/"series" sit between order and family, not inside a genus — see
/// `Rank.java`'s own placement of `SECTION_ZOOLOGY`/`SERIES_ZOOLOGY` right after
/// suborder, versus `SECTION_BOTANY`/`SERIES_BOTANY` down in the genus-group block).
/// Applied when a leading rank marker is parsed under a caller-supplied ZOOLOGICAL code.
/// Ported as the COMPLETE 6-pair `match`, faithful to the Java literal even though
/// `SuperseriesBotany` (only reachable via a "supraser"/"superser" word — neither is one
/// of `LEADING_INFRAGEN_MARKER`'s 13 alternatives) can never actually be produced by THIS
/// step's own regex — same "port the whole literal, not a call-site-trimmed-down version"
/// precedent as `is_known_infraspecific_marker` (batch 1) and `DIGITS_ONLY` in
/// `stash_trailing_strain_code` (batch 1). The wildcard fallback is exact here, not a
/// faithfulness compromise: Java's own `Map.get()` likewise returns `null` for any `Rank`
/// key it doesn't list (i.e. `Subgenus`, which has no zoological counterpart at all).
fn bot_to_zool(rank: Rank) -> Option<Rank> {
    match rank {
        Rank::SectionBotany => Some(Rank::SectionZoology),
        Rank::SubsectionBotany => Some(Rank::SubsectionZoology),
        Rank::SupersectionBotany => Some(Rank::SupersectionZoology),
        Rank::SeriesBotany => Some(Rank::SeriesZoology),
        Rank::SubseriesBotany => Some(Rank::SubseriesZoology),
        Rank::SuperseriesBotany => Some(Rank::SuperseriesZoology),
        _ => None,
    }
}

/// Java LEADING_INFRAGEN_MARKER (StripAndStash.java:1716-1719):
/// `^(subg|subgen|subgenus|sect|section|subsect|subsection|supersect|suprasect|ser|series|subser|subseries)\.?\s+(?=[\p{Lu}])`,
/// `Pattern.UNICODE_CHARACTER_CLASS | Pattern.CASE_INSENSITIVE` -> keep default Unicode
/// `\s`, add `(?i)`. Trailing LOOKAHEAD (same reasoning as `SUPRA_RANK_PREFIX` above) ->
/// needs `fancy_regex`. No nested/ambiguous quantifiers -> no ReDoS exposure.
static LEADING_INFRAGEN_MARKER: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(
        r"(?i)^(subg|subgen|subgenus|sect|section|subsect|subsection|supersect|suprasect|ser|series|subser|subseries)\.?\s+(?=[\p{Lu}])",
    )
    .unwrap()
});

/// Java `StripAndStash.stripLeadingInfragenericMarker` (StripAndStash.java:1572-1593). A
/// leading infrageneric rank marker with NO genus prefix ("subgen. Trematostoma Sacc.",
/// "sect. Taeda") pins `ctx.name.rank` — remapped to its zoological counterpart via
/// `bot_to_zool` when `ctx.requested_code == ZOOLOGICAL` — and backfills `ctx.name.code`
/// from the (possibly-remapped) rank's own `Rank::code()` ONLY when no code was already
/// set: unlike step 53's unconditional rank overwrite just above, this mirrors Java's own
/// `if (r.getCode() != null && ctx.name.getCode() == null)` guard exactly. All three
/// shapes spot-checked against the Java CLI oracle: "subgen. Trematostoma Sacc." (no
/// caller code) -> `rank=SUBGENUS`, no `code` key at all (Subgenus carries none); "sect.
/// Foo Bar" (no caller code) -> `rank=SECTION_BOTANY`, `code=BOTANICAL` (backfilled from
/// `Rank::code()`); "sect. Taeda" WITH a caller-supplied ZOOLOGICAL code -> `rank=
/// SECTION_ZOOLOGY` (remapped, not SECTION_BOTANY).
fn strip_leading_infrageneric_marker(ctx: &mut ParseContext, s: String) -> String {
    if let Ok(Some(caps)) = LEADING_INFRAGEN_MARKER.captures(&s) {
        let marker = caps.get(1).unwrap().as_str().to_lowercase();
        if let Some(mut r) = infrageneric_marker_rank(&marker) {
            if ctx.requested_code == Some(NomCode::Zoological) {
                if let Some(zool) = bot_to_zool(r) {
                    r = zool;
                }
            }
            ctx.name.rank = r;
            if let Some(code) = r.code() {
                if ctx.name.code.is_none() {
                    ctx.name.code = Some(code);
                }
            }
            let whole = caps.get(0).unwrap();
            return java_trim(&s[whole.end()..]).to_string();
        }
    }
    s
}

// ---- Step 55: stashPhraseName ----

/// Java `PHRASE_RANK_MARKERS` (StripAndStash.java:1661-1668): recognised infraspecific
/// markers that introduce a phrase name, mapped to the `Rank` they pin. Complete port —
/// unlike its two siblings above, every key this (small, `Map.of`-literal) map lists is
/// directly reachable, no call-site-unreachable subset to trim. Case-folded `match`;
/// `word` must already be lower-cased by the caller.
fn phrase_rank_marker(word: &str) -> Option<Rank> {
    match word {
        "sp" | "spec" => Some(Rank::Species),
        "subsp" | "ssp" => Some(Rank::Subspecies),
        "var" => Some(Rank::Variety),
        "form" | "f" => Some(Rank::Form),
        _ => None,
    }
}

/// Java GENUS_SUBGENUS_TEST (StripAndStash.java:352-353):
/// `^[\p{Lu}][\p{Ll}]+\s+\([\p{Lu}][\p{Ll}]+\)$`, NO flags (so Java's own `\s` here is
/// ASCII-only, unlike its Unicode-flagged `PHRASE_GENUS_SUBGENUS` sibling right below).
/// Has `\p{Lu}`/`\p{Ll}` -> only the `\s` atom is ASCII-scoped (not the whole pattern).
static GENUS_SUBGENUS_TEST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{Lu}][\p{Ll}]+(?-u:\s+)\([\p{Lu}][\p{Ll}]+\)$").unwrap());

/// Java PHRASE_GENUS_SUBGENUS (StripAndStash.java:280-282):
/// `^([\p{Lu}][\p{Ll}]+)\s+\(([\p{Lu}][\p{Ll}]+)\)$`, `Pattern.UNICODE_CHARACTER_CLASS` ->
/// keep default Unicode `\s`, ported verbatim.
static PHRASE_GENUS_SUBGENUS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([\p{Lu}][\p{Ll}]+)\s+\(([\p{Lu}][\p{Ll}]+)\)$").unwrap());

// `AUTHOR_START`/`find_author_start` (Java StripAndStash.java:283-285 / 1650-1657) are
// NOT redeclared here: Java's own `findAuthorStart` is already SHARED between an earlier
// step (`stripQuotedCultivar`, StripAndStash.java:1035, batch 2b) and this step
// (StripAndStash.java:1609) — both call the exact same method — so batch 2b's port of
// both already lives earlier in this file and is reused verbatim below.

/// Java PHRASE_NAME (StripAndStash.java:1677-1689), `Pattern.UNICODE_CHARACTER_CLASS` ->
/// keep default Unicode `\s`/`\d` throughout, no ASCII scoping needed anywhere in this
/// pattern.
///
/// **THE LAST remaining Java possessive quantifier in `StripAndStash`** (`[^(]*+`, meant
/// to lock the phrase's leading non-paren run so a greedy `[^(]*` can't backtrack past the
/// first '(' looking for a balanced-parens match that isn't there). Determination: this
/// pattern has NO lookaround and NO backreference anywhere — unlike every other
/// possessive-bearing pattern ported so far (`STRAIN_DESIGNATION`,
/// `PERIOD_SEPARATED_REFERENCE`, …), which all needed `fancy_regex` for an unrelated
/// reason (a backreference or lookaround elsewhere in the same pattern) and so kept their
/// possessive/atomic quantifiers verbatim on that backtracking engine — so this pattern
/// needs only the plain `regex` crate. Per this port's rule (see
/// `strip_comma_prefixed_reference`'s doc comment, batch 2d): possessive quantifiers are
/// DROPPED on patterns that live on the linear `regex` crate, since that engine has no
/// backtracking at all and so cannot exhibit the catastrophic-backtracking failure mode a
/// possessive quantifier guards against — **the possessive is DROPPED here too** (`[^(]*+`
/// ported as plain `[^(]*`). This is not merely "safe because this engine doesn't
/// backtrack": the quantified class itself (`[^(]`, "any char but a literal '('") has
/// exactly ONE possible stopping point for greedy OR possessive alike — right before the
/// first literal '(' remaining in the string, since the class can never itself consume
/// that '(' — so there is no ambiguous partition here for ANY engine, even a backtracking
/// one, to waste time re-trying. Dropping `*+` to `*` is a provably meaning- AND
/// performance-preserving restructuring, stronger than "merely safe on this engine".
/// Built via `concat!` mirroring Java's own `"..." + "..."` layout, one alternative per
/// line, so it stays directly diffable against `StripAndStash.java`.
static PHRASE_NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"^([\p{Lu}][\p{Ll}]+(?:\s+\([\p{Lu}][\p{Ll}]+\))?(?:\s+[\p{Ll}]+)?(?:\s+[\p{Lu}][\p{L}.]+)*)",
        r"\s+(sp|spec|subsp|ssp|var|form|f)\.?",
        r"\s+(",
        r#"[\p{Lu}A-Z\d'"][^(]*\(.+\)[^$]*?"#,
        r#"|['"][^'"]+['"]"#,
        r")\s*$",
    ))
    .unwrap()
});

/// Java `StripAndStash.stashPhraseName` (StripAndStash.java:1595-1644). BOLD/specimen-style
/// "phrase names" name an unidentified/undescribed taxon by a genus (optionally a species)
/// plus a rank marker plus a free-text "phrase" distinguishing the specimen/population,
/// e.g. "Prostanthera sp. Somersbey (B.J.Conn 4024)" — the phrase (here "Somersbey
/// (B.J.Conn 4024)") is stashed on `ctx.name.phrase` UNCONDITIONALLY once a recognised
/// rank marker is found, and the working string is rewritten into one of four shapes
/// depending on the Latin prefix, so NameTokens sees a clean indet
/// name. All four shapes spot-checked against the Java CLI oracle:
///   - a trailing author span after the epithet(s) ("Baeckea Benth. sp. Bygalorie (ABC
///     123)" -> working "Baeckea sp. Benth.") -> the marker is spliced BEFORE that author
///     span so the author still trails as the species author for AuthorshipParser;
///     rank is NOT pinned here — oracle confirms the eventual `rank=SPECIES`
///     comes from downstream re-reading the reinserted "sp." marker, not from this step.
///   - a bare genus (no species) with a marker OTHER than "sp."/"spec." ("Grevillea
///     subsp. 'Short Leaf'" -> `rank=SUBSPECIES`, working "Grevillea") -> rank pinned
///     directly, marker dropped entirely.
///   - a "Genus (Subgenus)" prefix ("Acacia (Botrycephalae) sp. Bygalorie (P.G.Wilson
///     2585)" -> `rank=SPECIES`, `infragenericEpithet=Botrycephalae`, working "Acacia")
///     -> rank pinned, the subgenus extracted directly onto
///     `ctx.name.infrageneric_epithet` and dropped from the working string, leaving the
///     bare genus for AuthorshipSplit's own subgenus-defaulting.
///   - anything else, i.e. a bare genus with a "sp."/"spec." marker, or a full "Genus
///     species" prefix ("Prostanthera sp. Somersbey (B.J.Conn 4024)" -> working
///     "Prostanthera sp.", rank left untouched by this step) -> the marker is appended
///     back verbatim ("Genus[ species] marker.").
fn stash_phrase_name(ctx: &mut ParseContext, s: String) -> String {
    let Some(caps) = PHRASE_NAME.captures(&s) else {
        return s;
    };
    let prefix = collapse_whitespace(caps.get(1).unwrap().as_str());
    let marker = caps.get(2).unwrap().as_str();
    let phrase = collapse_whitespace(caps.get(3).unwrap().as_str());
    let Some(rank) = phrase_rank_marker(&marker.to_lowercase()) else {
        return s;
    };
    ctx.name.phrase = Some(phrase);

    // Java re-trims `prefix` via `.trim()` at each of the three call sites below
    // (`.contains(" ")`, `GENUS_SUBGENUS_TEST`, `PHRASE_GENUS_SUBGENUS`) even though
    // `prefix` was already fully collapsed-and-trimmed by `collapse_whitespace` above — a
    // defensive no-op given that, so it is not repeated at each site here.
    let author_start = find_author_start(&prefix);
    let prefix_is_genus_only = !prefix.contains(' ');
    let prefix_is_genus_plus_subgenus = GENUS_SUBGENUS_TEST.is_match(&prefix);

    if let Some(author_start) = author_start {
        // Splice the marker BEFORE the author span so it trails as the species author for
        // AuthorshipParser to pick up; rank is intentionally left
        // untouched here — the reinserted "marker." drives the rank downstream instead.
        format!(
            "{} {marker}. {}",
            java_trim(&prefix[..author_start]),
            java_trim(&prefix[author_start..]),
        )
    } else if prefix_is_genus_only && rank != Rank::Species {
        ctx.name.rank = rank;
        prefix
    } else if prefix_is_genus_plus_subgenus {
        ctx.name.rank = rank;
        match PHRASE_GENUS_SUBGENUS.captures(&prefix) {
            Some(gm) => {
                ctx.name.infrageneric_epithet = Some(gm.get(2).unwrap().as_str().to_string());
                gm.get(1).unwrap().as_str().to_string()
            }
            None => prefix,
        }
    } else {
        format!("{prefix} {marker}.")
    }
}

// ===================================================================================
// stripAuthorshipMarkers (Phase 1 Slice 4 Task 4): the auxiliary-authorship-path
// reimplementation. Strips a SUBSET of the annotations `run()` above strips from the
// embedded scientific name — reusing several of the SAME pattern constants — from a
// separately-supplied authorship string (the `authorship` parameter of
// `nameparser::parse`/`Pipeline::run`), applying their flags/notes directly onto a
// `ParsedName`. Called only from `pipeline::mod`'s aux-authorship path, on the
// caller-supplied authorship string, before it is tokenised and handed to
// `AuthorshipParser::parse`.
//
// Unlike `run()` above, this path has no `ParseContext` (no working string of its own to
// thread `pending_unparsed`/`pending_year` state through) — its only output is the
// cleaned string, plus side effects applied directly onto `name`. It is therefore NOT
// part of the `run()` dispatcher above and has its own, shorter, ordered list of steps
// (Java `StripAndStash.stripAuthorshipMarkers`, StripAndStash.java:409-525): standalone
// manuscript marker (early return) -> sic-with-comment / sic / corrig -> question-mark
// repair -> win-1252 repair -> hort./hortus ex-placeholder normalisation (CV_EX and the
// bare "ht." marker are DELIBERATELY omitted — Java's aux version calls only HORT_EX/
// HORTUS_EX, not the full 4-step `normaliseHortExPlaceholder` chain) -> leading
// homonym-citation parens (early return) -> parenthesised taxonomic note -> bracketed
// nom-note -> bare nom-note -> taxonomic-note tail.
// ===================================================================================

/// Java STANDALONE_MS (StripAndStash.java:39-40): `(?i)^(?:ined|ms|msc|unpublished)\.?$`,
/// no `UNICODE_CHARACTER_CLASS`. Already fully anchored (`^…$`) so Java's `.matches()`
/// call is identical to `.find()`/`is_match` here. No `\s`/`\d`/`\w`/`\b`/`\p{...}` atoms
/// at all -> nothing to scope, ported verbatim. Only used by `strip_authorship_markers`
/// (a manuscript marker supplied as the WHOLE separate authorship, e.g. authorship="ined."
/// — a marker glued onto an embedded author, "Monterosato ms.", is a different case,
/// handled by `strip_manuscript_marker`/`MANUSCRIPT_MARKER` in the main `run()` dispatch).
static STANDALONE_MS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:ined|ms|msc|unpublished)\.?$").unwrap());

/// Java LEADING_HOMONYM_PAREN (StripAndStash.java:113-114): `^\(\s*(?:non|nec|not)\b`,
/// `Pattern.CASE_INSENSITIVE`. Has `\s`/`\b`, no `\p{...}`, no unescaped wildcard -> whole
/// pattern ASCII-scoped (leading `^` left outside the wrap, per convention — same as
/// `CANDIDATUS_PREFIX`). No lookaround/backreference -> plain `regex` crate.
static LEADING_HOMONYM_PAREN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?-u:\(\s*(?:non|nec|not)\b)").unwrap());

/// Java PAREN_NOTE (StripAndStash.java:108-109):
/// `\(\s*((?:auctt?|sensu|sec)\b[^)]*)\)\s*`, `Pattern.CASE_INSENSITIVE`. `[^)]` is a
/// NEGATED custom class -> stays OUTSIDE any `(?-u:…)` (`BRACKETED_TAX_NOTE`/
/// `SIC_WITH_COMMENT` precedent) -> atom-only `\s`/`\b` scoping. UNANCHORED (no leading/
/// trailing `^`/`$`) — unlike its `BRACKETED_TAX_NOTE`/`PAREN_TAX_NOTE` siblings in the
/// main `run()` dispatch, this note can sit ANYWHERE in the aux authorship string (e.g.
/// "(auct.) Rolfe" — a leading note with a trailing real author), not just at the end. No
/// lookaround/backreference -> plain `regex` crate.
static PAREN_NOTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\((?-u:\s*)((?:auctt?|sensu|sec)(?-u:\b)[^)]*)\)(?-u:\s*)").unwrap()
});

/// Java `StripAndStash.stripAuthorshipMarkers(String authorship, ParsedName name)`
/// (StripAndStash.java:409-525). See the section doc comment above for the step order.
/// Strips inline annotations from an externally-supplied authorship string and applies
/// any flags they imply directly onto `name`. Returns the cleaned authorship, ready for
/// `Tokenizer::tokenize` (`crate::token::tokenize`).
pub(crate) fn strip_authorship_markers(authorship: &str, name: &mut ParsedName) -> String {
    let mut s = java_trim(authorship).to_string();
    if s.is_empty() {
        return s;
    }
    // A standalone manuscript marker as the WHOLE authorship is a manuscript flag, not an
    // author.
    if STANDALONE_MS.is_match(&s) {
        name.manuscript = true;
        return String::new();
    }
    if let Some(m) = SIC_WITH_COMMENT.find(&s) {
        name.original_spelling = Some(true);
        s = format!("{}{}", &s[..m.start()], &s[m.end()..]);
    }
    if let Some(m) = SIC.find(&s) {
        name.original_spelling = Some(true);
        s = format!("{}{}", &s[..m.start()], &s[m.end()..]);
    }
    {
        let padded = format!(" {s}");
        if let Ok(Some(_)) = CORRIG.find(&padded) {
            name.original_spelling = Some(false);
            let stripped = fancy_replace_all(&CORRIG, &padded, |_| String::new());
            s = collapse_whitespace(&stripped);
        }
    }
    // "?" inside a word — transcription artefact for a missing letter ("Istv?nffi"). Strip
    // the ? and glue the surrounding word parts; flag doubtful + warning.
    if s.contains('?') && LETTER_QMARK_LETTER.is_match(&s) {
        s = QMARK_BETWEEN_LETTERS.replace_all(&s, "$1$2").into_owned();
        name.doubtful = true;
        name.add_warning(warnings::QUESTION_MARKS_REMOVED);
    }
    s = repair_win1252_artefacts_name(name, s);
    // "Hort."/"hortus(a)" horticultural placeholder, lower-cased to the canonical "hort.".
    // Deliberately omits `CV_EX`/`HT_MARKER` (see the section doc comment).
    s = fancy_replace_all(&HORT_EX, &s, |_| "hort.".to_string());
    s = fancy_replace_all(&HORTUS_EX, &s, |_| "hort.".to_string());

    // A leading parenthesised homonym citation "(non/nec/not ...)" makes the whole
    // authorship a misapplied/taxonomic note rather than a basionym — capture it verbatim,
    // no author left.
    if LEADING_HOMONYM_PAREN.is_match(&s) {
        let norm = collapse_whitespace(&s);
        name.add_taxonomic_note(&norm);
        return String::new();
    }

    // Parenthesised taxonomic note "(auct.)"/"(sensu ...)"/"(sec ...)" — the parens mark a
    // note, not a basionym. Capture the inner text as the taxonomic note, drop the parens,
    // and keep any real author beside them: "(auct.) Rolfe" -> author Rolfe + note "auct.";
    // "(sensu X, 1878) Y, 1992" -> author "Y, 1992" + note "sensu X, 1878".
    if let Some(caps) = PAREN_NOTE.captures(&s) {
        let inner = java_trim(caps.get(1).unwrap().as_str());
        let norm = WHITESPACE.replace_all(inner, " ").into_owned();
        let norm = normalise_leading_auct(&norm);
        name.add_taxonomic_note(&norm);
        let whole = caps.get(0).unwrap();
        let (start, end) = (whole.start(), whole.end());
        s = java_trim(&format!("{}{}", &s[..start], &s[end..])).to_string();
    }

    // Bracketed nom-notes "(nom. nud.)"/"[nom. cons.]" in the auxiliary authorship —
    // extract into nomenclaturalNote and drop from the string before tokenisation.
    if let Some(caps) = BRACKETED_NOM_NOTE.captures(&s) {
        let raw = java_trim(caps.get(1).unwrap().as_str()).to_string();
        let norm = normalise_nom_note(&raw);
        name.add_nomenclatural_note(&norm);
        let match_start = caps.get(0).unwrap().start();
        s = java_trim(&s[..match_start]).to_string();
    }

    // Bare nomenclatural notes ("nom. illeg.", "comb. nov.", "sp. nov.", …) in the
    // auxiliary authorship — same extraction as `run()`'s `strip_nom_note`, so a
    // separately-supplied authorship behaves like the equivalent tail on a full name.
    // Matched against a space-padded copy so a note anchored at the very START of the
    // string ("nom. illeg.") is caught too — NOM_NOTE requires a leading whitespace.
    let padded_nom = format!(" {s}");
    if let Ok(Some(caps)) = NOM_NOTE.captures(&padded_nom) {
        let raw = java_trim(caps.get(1).unwrap().as_str()).to_string();
        if !raw.is_empty() {
            let norm = normalise_nom_note(&raw);
            name.add_nomenclatural_note(&norm);
            let whole = caps.get(0).unwrap();
            let before = java_trim(&padded_nom[..whole.start()]).to_string();
            let after = java_trim(&padded_nom[whole.end()..]).to_string();
            s = if after.is_empty() {
                before
            } else {
                format!("{before} {after}")
            };
            s = java_trim(&s).to_string();
            while s.ends_with(',') {
                s.pop();
                s = java_trim(&s).to_string();
            }
            if MANUSCRIPT_KEYWORD.is_match(&raw) {
                name.manuscript = true;
            }
        }
    }

    // Strip taxonomic-note tails (sensu, emend., auct., etc.) from the auxiliary
    // authorship string — same patterns `run()`'s `strip_tax_note` applies to the main
    // working string. Matched against a space-padded copy (same reasoning as NOM_NOTE
    // above). UNLIKE every other note-setter in this function, this one DEDUPS against an
    // identical existing taxonomicNote instead of always appending (Java:
    // `existing.equals(norm) ? existing : existing + " " + norm`).
    let padded_tax = format!(" {s}");
    if let Some(caps) = TAX_NOTE.captures(&padded_tax) {
        let raw = java_trim(caps.get(1).unwrap().as_str()).to_string();
        if !raw.is_empty() {
            let with_dots = INITIAL_DOT_SPACE.replace_all(&raw, "$1.$2");
            let norm = normalise_leading_auct(&with_dots);
            name.taxonomic_note = Some(match name.taxonomic_note.take() {
                None => norm.clone(),
                Some(existing) if existing == norm => existing,
                Some(existing) => format!("{existing} {norm}"),
            });
            // Cut `s` right before group(1)'s content. Group(1) is a suffix of
            // `padded_tax` (TAX_NOTE's trailing `)$` anchors it to end-of-padded-string),
            // and `padded_tax` is `s` with exactly one ASCII-space byte prepended, so
            // slicing `padded_tax[..group1_start]` (which still carries that one leading
            // pad byte) and `java_trim`-ing it away yields exactly the same content as
            // Java's `s.substring(0, s.length() - m.group(1).length()).trim()` — without
            // needing any UTF-16-vs-UTF-8 length arithmetic or a same-string byte-offset
            // subtraction that could underflow.
            let group1_start = caps.get(1).unwrap().start();
            s = java_trim(&padded_tax[..group1_start]).to_string();
            while s.ends_with(',') {
                s.pop();
                s = java_trim(&s).to_string();
            }
        }
    }
    java_trim(&s).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::NameType;

    fn ctx(s: &str) -> ParseContext {
        ParseContext::new(s.to_string(), None, None, None)
    }

    /// Phase 1 Slice 2 Task 2 locked the dispatcher order down with every one of the 55
    /// steps a pure passthrough. Batches 1, 2b, 2c, 2d and 2e (steps 1-55, ALL of them as
    /// of this batch) now carry a faithful port, but a clean, unremarkable binomial should
    /// still round-trip untouched: none of steps 1-19's guard conditions (a trailing/glued
    /// "?", a quoted leading monomial, a "Missing "/lowercase-epithet prefix, Greek/star
    /// markers, a letter-subdivision marker, "str"/"strain", imprint-year brackets, "null"
    /// between epithets, Unicode hyphen/Win-1252/double-underscore artefacts, a trailing
    /// OTU code, serovar/serotype, an angle bracket, or HTML), steps 20-30's (a
    /// "Candidatus"/"Ca." prefix, a horticultural "ex" placeholder, a cultivar
    /// Group/grex/quoted epithet, an extinction dagger, a "t.infr." marker, a bracketed
    /// genus, "sic"/"corrig.", a synonymy bracket, or a nom/comb/orth/nomen/sp.nov./
    /// pro-syn. keyword), steps 31-44's (an authorship placeholder, a trailing " species",
    /// "pro parte"/"p.p.", a "(pro sp.)" annotation, "(Approved Lists YYYY)", "mihi",
    /// "Anon"/"anon", or any of the six taxonomic-note/aggregate-suffix keywords), steps
    /// 45-52's (a trailing page reference, "in press", a parenthesised or trailing
    /// "in"/"apud" citation, an IPNI/period-/comma-separated reference, or a manuscript
    /// marker), nor steps 53-55's (a suprarank/leading-infrageneric-rank prefix, or a
    /// phrase-name shape) fire on "Abies alba Mill.". This still locks the same invariant
    /// Task 2 established — a batch landing later can't silently leave a step half-wired —
    /// just now over the complete, fully-ported 55-step dispatcher rather than 55 no-ops.
    #[test]
    fn run_leaves_a_clean_binomial_untouched() {
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
        // "Sess?" through StripAndStash — the "?" is dropped later by the tokenizer,
        // not here.
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

    // ---- Step 13: replaceHomoglyphs ----

    #[test]
    fn cyrillic_lookalike_is_folded_to_latin_and_flags_homoglyphs() {
        let mut c = ctx("x");
        let out = replace_homoglyphs(&mut c, "Aus \u{0430}bus".to_string()); // Cyrillic 'а'
        assert_eq!(out, "Aus abus");
        assert!(c.name.warnings.contains(&warnings::HOMOGLYHPS.to_string()));
    }

    /// The exact corpus shape this step used to leave untouched (see
    /// `tests/parse_golden.rs`'s now-empty `ALLOWLIST`): a specific epithet carrying U+017F
    /// LATIN SMALL LETTER LONG S folds to plain "s".
    #[test]
    fn long_s_in_specific_epithet_is_folded_and_flags_homoglyphs() {
        let mut c = ctx("x");
        let out = replace_homoglyphs(&mut c, "Musca dome\u{017F}tica Linnaeus 1758".to_string());
        assert_eq!(out, "Musca domestica Linnaeus 1758");
        assert!(c.name.warnings.contains(&warnings::HOMOGLYHPS.to_string()));
    }

    #[test]
    fn hybrid_marker_and_ae_ligature_are_not_homoglyphs_here_either() {
        let mut c = ctx("x");
        let out = replace_homoglyphs(&mut c, "Abies \u{00D7} Picea".to_string());
        assert_eq!(out, "Abies \u{00D7} Picea");
        assert!(c.name.warnings.is_empty());

        let mut c2 = ctx("x");
        let out2 = replace_homoglyphs(&mut c2, "Amphisb\u{00E6}na".to_string());
        assert_eq!(out2, "Amphisb\u{00E6}na");
        assert!(c2.name.warnings.is_empty());
    }

    #[test]
    fn plain_name_is_untouched_with_no_warning() {
        let mut c = ctx("x");
        let out = replace_homoglyphs(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
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
        // indet-species handling still recognises it downstream.
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

    // ---- Step 31: stripAuthorshipPlaceholders ----

    #[test]
    fn trailing_not_given_is_stripped_and_flags_authorship_removed() {
        let mut c = ctx("x");
        let out = strip_authorship_placeholders(&mut c, "Aus bus Smith Not given".to_string());
        assert_eq!(out, "Aus bus Smith");
        assert!(c
            .name
            .warnings
            .contains(&warnings::AUTHORSHIP_REMOVED.to_string()));
    }

    #[test]
    fn not_applicable_is_case_insensitive() {
        let mut c = ctx("x");
        let out = strip_authorship_placeholders(&mut c, "Aus bus not applicable".to_string());
        assert_eq!(out, "Aus bus");
        assert!(c
            .name
            .warnings
            .contains(&warnings::AUTHORSHIP_REMOVED.to_string()));
    }

    #[test]
    fn no_placeholder_keyword_is_untouched() {
        let mut c = ctx("x");
        let out = strip_authorship_placeholders(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(c.name.warnings.is_empty());
    }

    // ---- Step 32: stripTrailingSpeciesWord ----

    #[test]
    fn trailing_species_word_is_dropped() {
        let mut c = ctx("x");
        let out = strip_trailing_species_word(&mut c, "Abies species".to_string());
        assert_eq!(out, "Abies");
    }

    #[test]
    fn trailing_species_word_with_dot_is_dropped() {
        let mut c = ctx("x");
        let out = strip_trailing_species_word(&mut c, "Abies species.".to_string());
        assert_eq!(out, "Abies");
    }

    #[test]
    fn real_binomial_ending_in_an_epithet_is_not_mistaken_for_the_placeholder() {
        // TRAILING_SPECIES_WORD_TEST only fires on a bare "Title word" + " species[.]"
        // shape — a genuine binomial like "Genus species alba" must be left alone.
        let mut c = ctx("x");
        let out = strip_trailing_species_word(&mut c, "Genus species alba".to_string());
        assert_eq!(out, "Genus species alba");
    }

    #[test]
    fn no_trailing_species_word_is_untouched() {
        let mut c = ctx("x");
        let out = strip_trailing_species_word(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
    }

    // ---- Step 33: stripProParte ----

    #[test]
    fn trailing_pro_parte_is_stripped_and_flags_doubtful() {
        let mut c = ctx("x");
        let out = strip_pro_parte(&mut c, "Aus bus Smith, pro parte".to_string());
        assert_eq!(out, "Aus bus Smith");
        assert!(c.name.doubtful);
    }

    #[test]
    fn abbreviated_p_p_form_is_also_stripped() {
        let mut c = ctx("x");
        let out = strip_pro_parte(&mut c, "Aus bus Smith, p.p.".to_string());
        assert_eq!(out, "Aus bus Smith");
        assert!(c.name.doubtful);
    }

    #[test]
    fn no_pro_parte_marker_is_untouched() {
        let mut c = ctx("x");
        let out = strip_pro_parte(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(!c.name.doubtful);
    }

    // ---- Step 34: stripProSpAnnotation ----

    #[test]
    fn pro_sp_annotation_is_stripped_silently() {
        let mut c = ctx("x");
        let out = strip_pro_sp_annotation(&mut c, "Aus bus (pro sp.)".to_string());
        assert_eq!(out, "Aus bus");
        assert!(!c.name.doubtful);
        assert!(c.name.warnings.is_empty());
    }

    #[test]
    fn pro_hyb_variant_without_a_dot_is_also_stripped() {
        let mut c = ctx("x");
        let out = strip_pro_sp_annotation(&mut c, "Aus bus (pro hyb)".to_string());
        assert_eq!(out, "Aus bus");
    }

    #[test]
    fn no_pro_sp_annotation_is_untouched() {
        let mut c = ctx("x");
        let out = strip_pro_sp_annotation(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
    }

    // ---- Step 35: stripApprovedLists ----

    #[test]
    fn approved_lists_annotation_is_stripped_silently() {
        let mut c = ctx("x");
        let out = strip_approved_lists(&mut c, "Aus bus Smith (Approved Lists 1980)".to_string());
        assert_eq!(out, "Aus bus Smith");
    }

    #[test]
    fn no_approved_lists_annotation_is_untouched() {
        let mut c = ctx("x");
        let out = strip_approved_lists(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
    }

    // ---- Step 36: stripMihi ----

    #[test]
    fn trailing_mihi_is_stripped_and_flags_authorship_removed() {
        let mut c = ctx("x");
        let out = strip_mihi(&mut c, "Aus bus mihi".to_string());
        assert_eq!(out, "Aus bus");
        assert!(c
            .name
            .warnings
            .contains(&warnings::AUTHORSHIP_REMOVED.to_string()));
    }

    #[test]
    fn mid_string_mihi_is_stripped_and_the_surrounding_text_splices_together() {
        let mut c = ctx("x");
        let out = strip_mihi(&mut c, "Aus bus mihi. Smith".to_string());
        assert_eq!(out, "Aus bus Smith");
        assert!(c
            .name
            .warnings
            .contains(&warnings::AUTHORSHIP_REMOVED.to_string()));
    }

    #[test]
    fn both_occurrences_of_mihi_are_stripped() {
        let mut c = ctx("x");
        let out = strip_mihi(&mut c, "Genus species mihi var. epithet mihi".to_string());
        assert_eq!(out, "Genus species var. epithet");
    }

    #[test]
    fn mihi_glued_to_a_preceding_word_is_not_a_false_positive() {
        // "mihilaris" contains "mihi" as a substring but MIHI_TEST's `\bmihi\b` requires
        // a word boundary on both sides — a boundary fails mid-word ("mihi"+"laris"), so
        // this must never be touched, matching the Java CLI oracle's parse of "Aus
        // mihilaris bus" (specificEpithet="mihilaris", untouched, no warning).
        let mut c = ctx("x");
        let out = strip_mihi(&mut c, "Aus mihilaris bus".to_string());
        assert_eq!(out, "Aus mihilaris bus");
        assert!(c.name.warnings.is_empty());
    }

    #[test]
    fn leading_mihi_with_no_preceding_whitespace_is_not_stripped_by_this_step_in_isolation() {
        // MIHI_TEST's loose `\bmihi\b` gate matches "mihi Aus bus" (mihi is present as a
        // whole word), but MIHI itself additionally requires a LEADING `\s+` before
        // "mihi" — at absolute string-start there is no preceding character at all, so
        // MIHI never matches here and the structural-equality guard (`after != before`)
        // correctly suppresses the warning. (In the FULL run() pipeline this exact input
        // is rewritten by the earlier-running `apply_missing_genus_placeholder`, step 4,
        // into "? mihi Aus bus" first — which DOES have leading whitespace before "mihi"
        // and so IS stripped, confirmed against the Java CLI oracle; this test isolates
        // step 36's own boundary requirement independent of that upstream interaction.)
        let mut c = ctx("x");
        let out = strip_mihi(&mut c, "mihi Aus bus".to_string());
        assert_eq!(out, "mihi Aus bus");
        assert!(c.name.warnings.is_empty());
    }

    #[test]
    fn no_mihi_is_untouched() {
        let mut c = ctx("x");
        let out = strip_mihi(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(c.name.warnings.is_empty());
    }

    // ---- Step 37: normaliseAnon ----

    #[test]
    fn title_case_anon_is_normalised_to_lower_case_with_a_dot() {
        let mut c = ctx("x");
        let out = normalise_anon(&mut c, "Aus bus Anon.".to_string());
        assert_eq!(out, "Aus bus anon.");
    }

    #[test]
    fn bare_lower_case_anon_is_also_normalised() {
        let mut c = ctx("x");
        let out = normalise_anon(&mut c, "Aus bus anon".to_string());
        assert_eq!(out, "Aus bus anon.");
    }

    #[test]
    fn already_normalised_anon_is_left_alone() {
        // ANON_LOWER's negative lookahead `(?!\.)` excludes an "anon" already followed
        // by a dot, and ANON_UPPER requires the literal capital "Anon" — neither fires,
        // so this is idempotent.
        let mut c = ctx("x");
        let out = normalise_anon(&mut c, "Aus bus anon.".to_string());
        assert_eq!(out, "Aus bus anon.");
    }

    #[test]
    fn no_anon_keyword_is_untouched() {
        let mut c = ctx("x");
        let out = normalise_anon(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
    }

    // ---- Step 38: stripColonConceptReference ----

    #[test]
    fn colon_concept_reference_appends_to_taxonomic_note() {
        let mut c = ctx("x");
        let out = strip_colon_concept_reference(
            &mut c,
            "Vespa emarginata Linnaeus, 1758: Fabricius, 1793".to_string(),
        );
        assert_eq!(out, "Vespa emarginata Linnaeus, 1758");
        assert_eq!(c.name.taxonomic_note, Some("Fabricius, 1793".to_string()));
    }

    #[test]
    fn colon_concept_reference_appends_to_an_existing_note() {
        let mut c = ctx("x");
        c.name.taxonomic_note = Some("existing".to_string());
        let out = strip_colon_concept_reference(&mut c, "Aus bus: Fabricius, 1793".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(
            c.name.taxonomic_note,
            Some("existing Fabricius, 1793".to_string())
        );
    }

    #[test]
    fn sanctioning_author_colon_form_without_a_year_is_not_a_concept_reference() {
        // No ", YYYY" after the colon (a sanctioning-author citation, e.g. "Boletus
        // versicolor L. : Fr.") must NOT be captured here — spot-checked against the
        // Java CLI oracle: this form surfaces as `sanctioningAuthor`, a wholly different
        // mechanism, and StripAndStash leaves it untouched.
        let mut c = ctx("x");
        let out = strip_colon_concept_reference(
            &mut c,
            "Vespa emarginata Linnaeus, 1758 : Fr.".to_string(),
        );
        assert_eq!(out, "Vespa emarginata Linnaeus, 1758 : Fr.");
        assert_eq!(c.name.taxonomic_note, None);
    }

    #[test]
    fn no_colon_is_untouched() {
        let mut c = ctx("x");
        let out = strip_colon_concept_reference(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.taxonomic_note, None);
    }

    // ---- Shared helper: normaliseLeadingAuct ----

    #[test]
    fn normalise_leading_auct_lowercases_single_t_form() {
        assert_eq!(normalise_leading_auct("Auct. europ."), "auct. europ.");
    }

    #[test]
    fn normalise_leading_auct_lowercases_double_t_form() {
        assert_eq!(normalise_leading_auct("Auctt. europ."), "auctt. europ.");
    }

    #[test]
    fn normalise_leading_auct_leaves_all_caps_form_untouched() {
        // Both LEADING_AUCT/LEADING_AUCTT are case-sensitive against the EXACT literal
        // "Auct"/"Auctt" (capital A, lower rest) — an all-caps "AUCTT." matches neither.
        assert_eq!(normalise_leading_auct("AUCTT. europ."), "AUCTT. europ.");
    }

    #[test]
    fn normalise_leading_auct_leaves_already_lower_case_form_untouched() {
        assert_eq!(normalise_leading_auct("auctt. europ."), "auctt. europ.");
    }

    // ---- Step 39: stripBracketedTaxNote ----

    #[test]
    fn bracketed_auctt_note_appends_to_taxonomic_note() {
        let mut c = ctx("x");
        let out = strip_bracketed_tax_note(
            &mut c,
            "Eunoa bus Smith [auctt. misspelling for Eunoe]".to_string(),
        );
        assert_eq!(out, "Eunoa bus Smith");
        assert_eq!(
            c.name.taxonomic_note,
            Some("auctt. misspelling for Eunoe".to_string())
        );
    }

    #[test]
    fn bracketed_tax_note_lowercases_a_leading_title_case_auct() {
        let mut c = ctx("x");
        let out = strip_bracketed_tax_note(&mut c, "Aus bus [Auct. europ.]".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("auct. europ.".to_string()));
    }

    #[test]
    fn bracketed_tax_note_collapses_internal_whitespace_runs() {
        let mut c = ctx("x");
        let out = strip_bracketed_tax_note(&mut c, "Aus bus [sensu   Miller   1990]".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("sensu Miller 1990".to_string()));
    }

    #[test]
    fn bracketed_tax_note_appends_to_an_existing_note() {
        let mut c = ctx("x");
        c.name.taxonomic_note = Some("existing".to_string());
        let out = strip_bracketed_tax_note(&mut c, "Aus bus [non Foo]".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("existing non Foo".to_string()));
    }

    #[test]
    fn no_bracketed_tax_note_is_untouched() {
        let mut c = ctx("x");
        let out = strip_bracketed_tax_note(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.taxonomic_note, None);
    }

    // ---- Step 40: stripParenTaxNote ----

    #[test]
    fn paren_non_homonym_citation_appends_to_taxonomic_note() {
        let mut c = ctx("x");
        let out = strip_paren_tax_note(&mut c, "Aus bus Smith (non Foo, 1850)".to_string());
        assert_eq!(out, "Aus bus Smith");
        assert_eq!(c.name.taxonomic_note, Some("non Foo, 1850".to_string()));
    }

    #[test]
    fn paren_nec_and_not_variants_are_also_captured() {
        let mut c = ctx("x");
        let out = strip_paren_tax_note(&mut c, "Aus bus (nec Jones, 1900)".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("nec Jones, 1900".to_string()));
    }

    #[test]
    fn paren_tax_note_appends_to_an_existing_note() {
        let mut c = ctx("x");
        c.name.taxonomic_note = Some("existing".to_string());
        let out = strip_paren_tax_note(&mut c, "Aus bus (non Foo, 1850)".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(
            c.name.taxonomic_note,
            Some("existing non Foo, 1850".to_string())
        );
    }

    #[test]
    fn no_paren_tax_note_is_untouched() {
        let mut c = ctx("x");
        let out = strip_paren_tax_note(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.taxonomic_note, None);
    }

    // ---- Step 41: stripSensuLatoRemainder ----

    #[test]
    fn sensu_lato_marker_with_trailing_junk_appends_note_and_stashes_unparsed() {
        let mut c = ctx("x");
        let out = strip_sensu_lato_remainder(
            &mut c,
            "Asplenium trichomanes L. s.lat. - Asplen trich".to_string(),
        );
        assert_eq!(out, "Asplenium trichomanes L.");
        assert_eq!(c.name.taxonomic_note, Some("s.lat.".to_string()));
        assert_eq!(c.pending_unparsed, Some("- Asplen trich".to_string()));
    }

    #[test]
    fn sensu_lato_remainder_appends_to_an_existing_note() {
        let mut c = ctx("x");
        c.name.taxonomic_note = Some("existing".to_string());
        let out = strip_sensu_lato_remainder(&mut c, "Aus bus s.str. junk here".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("existing s.str.".to_string()));
    }

    #[test]
    fn uppercase_author_initials_are_not_mistaken_for_a_sensu_lato_marker() {
        let mut c = ctx("x");
        let out = strip_sensu_lato_remainder(&mut c, "Aus bus Mill. S.L. Schultes".to_string());
        assert_eq!(out, "Aus bus Mill. S.L. Schultes");
        assert_eq!(c.name.taxonomic_note, None);
    }

    #[test]
    fn bare_trailing_sensu_lato_with_no_junk_is_not_matched_here() {
        // SENSU_LATO_REMAINDER requires a MANDATORY non-empty trailing remainder after
        // the marker — a bare trailing "s.l." with nothing following is left for
        // `strip_tax_note` (step 43) instead.
        let mut c = ctx("x");
        let out = strip_sensu_lato_remainder(&mut c, "Aus bus s.l.".to_string());
        assert_eq!(out, "Aus bus s.l.");
        assert_eq!(c.name.taxonomic_note, None);
    }

    // ---- Step 42: stripSensuStrictoSS ----

    #[test]
    fn trailing_ss_with_no_junk_appends_literal_note_and_stashes_nothing() {
        let mut c = ctx("x");
        let out = strip_sensu_stricto_ss(&mut c, "Aus bus s.s.".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("s.s.".to_string()));
        assert_eq!(c.pending_unparsed, None);
    }

    #[test]
    fn trailing_ss_with_junk_appends_note_and_stashes_the_trimmed_junk() {
        let mut c = ctx("x");
        let out = strip_sensu_stricto_ss(&mut c, "Aus bus s.s. extra text here".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("s.s.".to_string()));
        assert_eq!(c.pending_unparsed, Some("extra text here".to_string()));
    }

    #[test]
    fn sensu_stricto_ss_appends_to_an_existing_note() {
        let mut c = ctx("x");
        c.name.taxonomic_note = Some("existing".to_string());
        let out = strip_sensu_stricto_ss(&mut c, "Aus bus s.s.".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("existing s.s.".to_string()));
    }

    #[test]
    fn no_sensu_stricto_marker_is_untouched() {
        let mut c = ctx("x");
        let out = strip_sensu_stricto_ss(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.taxonomic_note, None);
        assert_eq!(c.pending_unparsed, None);
    }

    // ---- Step 43: stripTaxNote ----

    #[test]
    fn emend_with_a_dot_overwrites_taxonomic_note() {
        let mut c = ctx("x");
        let out = strip_tax_note(
            &mut c,
            "Chlorobium phaeobacteroides Pfennig, 1968 emend. Imhoff, 2003".to_string(),
        );
        assert_eq!(out, "Chlorobium phaeobacteroides Pfennig, 1968");
        assert_eq!(
            c.name.taxonomic_note,
            Some("emend. Imhoff, 2003".to_string())
        );
    }

    #[test]
    fn emend_without_a_dot_is_also_matched() {
        let mut c = ctx("x");
        let out = strip_tax_note(
            &mut c,
            "Chlorobium phaeobacteroides Pfennig, 1968 emend Imhoff, 2003".to_string(),
        );
        assert_eq!(out, "Chlorobium phaeobacteroides Pfennig, 1968");
        assert_eq!(
            c.name.taxonomic_note,
            Some("emend Imhoff, 2003".to_string())
        );
    }

    #[test]
    fn fide_keyword_is_captured() {
        let mut c = ctx("x");
        let out = strip_tax_note(
            &mut c,
            "Eulima excellens Verkrüzen fide Paetel, 1887".to_string(),
        );
        assert_eq!(out, "Eulima excellens Verkrüzen");
        assert_eq!(c.name.taxonomic_note, Some("fide Paetel, 1887".to_string()));
    }

    #[test]
    fn nec_keyword_captures_a_trailing_parenthesised_citation() {
        let mut c = ctx("x");
        let out = strip_tax_note(
            &mut c,
            "Procamallanus (Spirocamallanus) soodi Lakshmi & Kumari, 2001 nec (Gupta & Masood, 1988)"
                .to_string(),
        );
        assert_eq!(
            out,
            "Procamallanus (Spirocamallanus) soodi Lakshmi & Kumari, 2001"
        );
        assert_eq!(
            c.name.taxonomic_note,
            Some("nec (Gupta & Masood, 1988)".to_string())
        );
    }

    #[test]
    fn non_keyword_is_captured() {
        let mut c = ctx("x");
        let out = strip_tax_note(
            &mut c,
            "Membranipora minuscula Canu, 1911 non Hincks, 1882".to_string(),
        );
        assert_eq!(out, "Membranipora minuscula Canu, 1911");
        assert_eq!(c.name.taxonomic_note, Some("non Hincks, 1882".to_string()));
    }

    #[test]
    fn bare_trailing_auct_is_captured_and_lower_cased() {
        let mut c = ctx("x");
        let out = strip_tax_note(&mut c, "Aus bus auct.".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("auct.".to_string()));
    }

    #[test]
    fn title_case_auct_and_auctt_are_lower_cased() {
        let mut c = ctx("x");
        let out = strip_tax_note(&mut c, "Aus bus Auct.".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("auct.".to_string()));

        let mut c2 = ctx("x");
        let out2 = strip_tax_note(&mut c2, "Aus bus Auctt.".to_string());
        assert_eq!(out2, "Aus bus");
        assert_eq!(c2.name.taxonomic_note, Some("auctt.".to_string()));
    }

    #[test]
    fn lower_case_sensu_lato_marker_is_matched_case_sensitively() {
        let mut c = ctx("x");
        let out = strip_tax_note(&mut c, "Aus bus s.l.".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("s.l.".to_string()));
    }

    #[test]
    fn uppercase_author_initials_do_not_match_the_sensu_lato_branch() {
        // The inline `(?-i:…)` case-sensitive carve-out means "S.L." (capital, author
        // initials) is never mistaken for the lower-case sensu-lato marker — spot-checked
        // against the Java CLI oracle: "Aus bus Mill. S.L." ends up with the initials
        // folded into the author string (authors=["Mill.S.L."]), no taxonomicNote at all.
        let mut c = ctx("x");
        let out = strip_tax_note(&mut c, "Aus bus Mill. S.L.".to_string());
        assert_eq!(out, "Aus bus Mill. S.L.");
        assert_eq!(c.name.taxonomic_note, None);
    }

    #[test]
    fn initial_dot_space_collapses_a_bare_initial_before_a_lowercase_word() {
        // group 2 of INITIAL_DOT_SPACE requires an ALL-LOWERCASE word (>= 4 letters) —
        // spot-checked against the Java CLI oracle: "non F. europaeus" collapses the gap
        // ("F. europaeus" -> "F.europaeus") because "europaeus" is all lower-case.
        let mut c = ctx("x");
        let out = strip_tax_note(&mut c, "Aus bus non F. europaeus".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("non F.europaeus".to_string()));
    }

    #[test]
    fn initial_dot_space_does_not_collapse_before_a_capitalised_surname() {
        // Conversely a capitalised surname like "Schmidt" does NOT satisfy `[\p{Ll}]`
        // (lower-case only) as the word after the dot, so the space survives — spot-
        // checked against the Java CLI oracle: "sensu F. Schmidt" keeps its space.
        let mut c = ctx("x");
        let out = strip_tax_note(&mut c, "Aus bus sensu F. Schmidt".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("sensu F. Schmidt".to_string()));
    }

    #[test]
    fn multi_letter_abbreviations_keep_their_space() {
        // INITIAL_DOT_SPACE only collapses a SINGLE capital letter before the dot (a
        // genuine author initial) — "ss." (two letters) must keep its trailing space so
        // abbreviated taxonomic keywords render verbatim, per the Java source's own
        // comment on `stripTaxNote`.
        let mut c = ctx("x");
        let out = strip_tax_note(&mut c, "Aus bus ss. auct. europ.".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("ss. auct. europ.".to_string()));
    }

    #[test]
    fn sec_according_to_and_excl_alternatives_are_all_matched() {
        let mut c1 = ctx("x");
        assert_eq!(
            strip_tax_note(&mut c1, "Aus bus sec. Smith 1990".to_string()),
            "Aus bus"
        );
        assert_eq!(c1.name.taxonomic_note, Some("sec. Smith 1990".to_string()));

        let mut c2 = ctx("x");
        assert_eq!(
            strip_tax_note(&mut c2, "Aus bus according to Smith".to_string()),
            "Aus bus"
        );
        assert_eq!(
            c2.name.taxonomic_note,
            Some("according to Smith".to_string())
        );

        let mut c3 = ctx("x");
        assert_eq!(
            strip_tax_note(&mut c3, "Aus bus excl. var. minor".to_string()),
            "Aus bus"
        );
        assert_eq!(c3.name.taxonomic_note, Some("excl. var. minor".to_string()));
    }

    #[test]
    fn overwrites_rather_than_appends_to_an_existing_note() {
        // The LAST of the six taxonomic-note steps: unlike steps 38-42 above it, this
        // one calls the plain setter (no null-check) and REPLACES any pre-existing note.
        let mut c = ctx("x");
        c.name.taxonomic_note = Some("existing".to_string());
        let out = strip_tax_note(&mut c, "Aus bus sensu Miller".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("sensu Miller".to_string()));
    }

    #[test]
    fn multiple_dangling_commas_before_the_match_are_all_stripped() {
        // A single dangling comma right before the keyword is consumed by TAX_NOTE's own
        // leading `,?`; a SECOND comma (e.g. a doubled ",,") is left over and must be
        // peeled off by the trailing `while (s.endsWith(","))` loop — spot-checked
        // against the Java CLI oracle: "Aus bus,, sensu Miller" -> taxonomicNote "sensu
        // Miller" with a clean "Aus bus" genus/species (no comma artefacts survive).
        let mut c = ctx("x");
        let out = strip_tax_note(&mut c, "Aus bus,, sensu Miller".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("sensu Miller".to_string()));
    }

    #[test]
    fn captured_text_case_is_preserved_verbatim_except_for_the_auct_normalisation() {
        // TAX_NOTE's `(?i)` only affects MATCHING, not the captured text itself — spot-
        // checked against the Java CLI oracle: "Aus bus SENSU Smith" keeps "SENSU"
        // upper-case in the note (only a leading "Auct"/"Auctt" gets normalised).
        let mut c = ctx("x");
        let out = strip_tax_note(&mut c, "Aus bus SENSU Smith".to_string());
        assert_eq!(out, "Aus bus");
        assert_eq!(c.name.taxonomic_note, Some("SENSU Smith".to_string()));
    }

    #[test]
    fn no_tax_note_keyword_is_untouched() {
        let mut c = ctx("x");
        let out = strip_tax_note(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.taxonomic_note, None);
    }

    // ---- Step 44: stripAggregateSuffix ----

    #[test]
    fn agg_suffix_sets_aggregate_and_strips_the_marker() {
        let mut c = ctx("x");
        let out = strip_aggregate_suffix(&mut c, "Achillea millefolium agg.".to_string());
        assert_eq!(out, "Achillea millefolium");
        assert!(c.aggregate);
    }

    #[test]
    fn species_group_suffix_is_also_recognised() {
        let mut c = ctx("x");
        let out = strip_aggregate_suffix(&mut c, "Achillea millefolium species group".to_string());
        assert_eq!(out, "Achillea millefolium");
        assert!(c.aggregate);
    }

    #[test]
    fn hyphenated_group_suffix_is_also_recognised() {
        let mut c = ctx("x");
        let out = strip_aggregate_suffix(&mut c, "Achillea millefolium-group".to_string());
        assert_eq!(out, "Achillea millefolium");
        assert!(c.aggregate);
    }

    #[test]
    fn no_aggregate_suffix_is_untouched() {
        let mut c = ctx("x");
        let out = strip_aggregate_suffix(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(!c.aggregate);
    }

    // ---- Step 45: stripPublishedPage ----
    // Every case below spot-checked against the Java CLI oracle via the full parse pipeline
    // (the field values StripAndStash alone contributes match one-for-one).

    #[test]
    fn published_page_colon_space_digits_is_stashed() {
        let mut c = ctx("x");
        let out = strip_published_page(
            &mut c,
            "Anolis marmoratus girafus LAZELL 1964: 377".to_string(),
        );
        assert_eq!(out, "Anolis marmoratus girafus LAZELL 1964");
        assert_eq!(c.name.published_in_page, Some("377".to_string()));
    }

    #[test]
    fn published_page_glued_colon_is_also_recognised() {
        let mut c = ctx("x");
        let out = strip_published_page(
            &mut c,
            "Recilia truncatus Dash & Viraktamath, 1998a:29".to_string(),
        );
        assert_eq!(out, "Recilia truncatus Dash & Viraktamath, 1998a");
        assert_eq!(c.name.published_in_page, Some("29".to_string()));
    }

    #[test]
    fn published_page_extra_spaced_colon_is_also_recognised() {
        let mut c = ctx("x");
        let out = strip_published_page(
            &mut c,
            "Recilia truncatus Dash & Viraktamath, 1998a : 29".to_string(),
        );
        assert_eq!(out, "Recilia truncatus Dash & Viraktamath, 1998a");
        assert_eq!(c.name.published_in_page, Some("29".to_string()));
    }

    #[test]
    fn published_page_range_with_en_dash_is_kept_verbatim() {
        let mut c = ctx("x");
        let out = strip_published_page(&mut c, "Foo bar Author, 1900: 12\u{2013}18".to_string());
        assert_eq!(out, "Foo bar Author, 1900");
        assert_eq!(c.name.published_in_page, Some("12\u{2013}18".to_string()));
    }

    #[test]
    fn no_published_page_is_untouched() {
        let mut c = ctx("x");
        let out = strip_published_page(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.published_in_page, None);
    }

    // ---- Step 46: stripInPress ----

    #[test]
    fn documented_example_in_press_sets_manuscript_and_appends_nomenclatural_note() {
        let mut c = ctx("x");
        let out = strip_in_press(&mut c, "Abies alba Mill. in press".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(c.name.manuscript);
        assert_eq!(c.name.nomenclatural_note, Some("in press".to_string()));
    }

    #[test]
    fn in_press_is_case_insensitive_and_always_stashes_the_lowercase_literal() {
        let mut c = ctx("x");
        let out = strip_in_press(&mut c, "Abies alba Mill. In Press.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(c.name.manuscript);
        assert_eq!(c.name.nomenclatural_note, Some("in press".to_string()));
    }

    #[test]
    fn in_press_appends_to_an_existing_nomenclatural_note() {
        let mut c = ctx("x");
        c.name.nomenclatural_note = Some("nom. illeg.".to_string());
        let out = strip_in_press(&mut c, "Abies alba Mill. in press".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(
            c.name.nomenclatural_note,
            Some("nom. illeg. in press".to_string())
        );
    }

    #[test]
    fn no_in_press_marker_is_untouched() {
        let mut c = ctx("x");
        let out = strip_in_press(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(!c.name.manuscript);
        assert_eq!(c.name.nomenclatural_note, None);
    }

    // ---- Step 47: stripInAuthorInParens ----

    #[test]
    fn in_author_in_parens_moves_the_publication_year_onto_a_yearless_basionym() {
        let mut c = ctx("x");
        let out = strip_in_author_in_parens(
            &mut c,
            "Hypsicera femoralis (Geoffroy in Fourcroy, 1785)".to_string(),
        );
        assert_eq!(out, "Hypsicera femoralis (Geoffroy, 1785)");
        assert_eq!(c.name.published_in, Some("Fourcroy, 1785".to_string()));
        assert_eq!(c.name.published_in_year, Some(1785));
    }

    #[test]
    fn in_author_in_parens_apud_variant_is_also_recognised() {
        let mut c = ctx("x");
        let out =
            strip_in_author_in_parens(&mut c, "Foo bar (Smith apud Jones, 1900) Mill.".to_string());
        assert_eq!(out, "Foo bar (Smith, 1900) Mill.");
        assert_eq!(c.name.published_in, Some("Jones, 1900".to_string()));
    }

    #[test]
    fn in_author_in_parens_does_not_double_up_a_year_the_basionym_already_has() {
        let mut c = ctx("x");
        let out =
            strip_in_author_in_parens(&mut c, "Foo bar (Smith, 1780 in Jones, 1900)".to_string());
        assert_eq!(out, "Foo bar (Smith, 1780)");
        assert_eq!(c.name.published_in, Some("Jones, 1900".to_string()));
    }

    #[test]
    fn in_author_in_parens_appends_to_an_existing_published_in() {
        let mut c = ctx("x");
        c.name.set_published_in("Earlier ref, 1700");
        let out = strip_in_author_in_parens(
            &mut c,
            "Hypsicera femoralis (Geoffroy in Fourcroy, 1785)".to_string(),
        );
        assert_eq!(out, "Hypsicera femoralis (Geoffroy, 1785)");
        assert_eq!(
            c.name.published_in,
            Some("Earlier ref, 1700 Fourcroy, 1785".to_string())
        );
        assert_eq!(
            c.name.published_in_year,
            Some(1785),
            "the year must be re-derived from the full COMBINED publishedIn string"
        );
    }

    #[test]
    fn plain_parenthesised_basionym_without_in_author_is_untouched() {
        let mut c = ctx("x");
        let out =
            strip_in_author_in_parens(&mut c, "Hypsicera femoralis (Geoffroy, 1785)".to_string());
        assert_eq!(out, "Hypsicera femoralis (Geoffroy, 1785)");
        assert_eq!(c.name.published_in, None);
    }

    // ---- Step 48: stripInAuthorCitation ----

    #[test]
    fn documented_example_busk_in_chimonides_sets_published_in_and_year() {
        let mut c = ctx("x");
        let out = strip_in_author_citation(&mut c, "Busk in Chimonides, 1987".to_string());
        assert_eq!(out, "Busk");
        assert_eq!(c.name.published_in, Some("Chimonides, 1987".to_string()));
        assert_eq!(c.name.published_in_year, Some(1987));
        assert_eq!(c.pending_year, Some("1987".to_string()));
        assert!(c.pending_year_from_publication);
    }

    #[test]
    fn apud_variant_is_also_recognised() {
        let mut c = ctx("x");
        let out = strip_in_author_citation(&mut c, "Small apud Britton & Wilson".to_string());
        assert_eq!(out, "Small");
        assert_eq!(c.name.published_in, Some("Britton & Wilson".to_string()));
    }

    #[test]
    fn trailing_sentence_period_after_closing_paren_is_dropped() {
        let mut c = ctx("x");
        let out =
            strip_in_author_citation(&mut c, "Foo bar Author in Kirchner (1988).".to_string());
        assert_eq!(out, "Foo bar Author");
        assert_eq!(c.name.published_in, Some("Kirchner (1988)".to_string()));
    }

    #[test]
    fn period_after_an_author_abbreviation_is_kept_not_dropped() {
        let mut c = ctx("x");
        let out = strip_in_author_citation(
            &mut c,
            "Papillaria  fuscescens (Hook.) Jaeg. fo. gracilis Card. in Fleisch.".to_string(),
        );
        assert_eq!(
            out,
            "Papillaria  fuscescens (Hook.) Jaeg. fo. gracilis Card."
        );
        assert_eq!(c.name.published_in, Some("Fleisch.".to_string()));
    }

    #[test]
    fn paren_year_takes_precedence_for_pending_year_while_published_in_year_takes_the_last_match() {
        // publishedInYear and pending_year are DISTINCT extractions (see IN_AUTHOR_PAREN_YEAR/
        // IN_AUTHOR_YEAR's doc comments): the former is set_published_in's own "last
        // year-shaped match in the combined string" rule, the latter is
        // set_pending_publication_year's "parenthesised year tried first" rule. Oracle-verified
        // this input yields publishedInYear=1990 but a combinationAuthorship year of 1988 (the
        // pending_year, applied by the Pipeline stage).
        let mut c = ctx("x");
        let out = strip_in_author_citation(
            &mut c,
            "Foo bar Author in Kirchner (1988), 1990".to_string(),
        );
        assert_eq!(out, "Foo bar Author");
        assert_eq!(
            c.name.published_in,
            Some("Kirchner (1988), 1990".to_string())
        );
        assert_eq!(c.name.published_in_year, Some(1990));
        assert_eq!(
            c.pending_year,
            Some("1988".to_string()),
            "the parenthesised year must win over the trailing bare year for pending_year"
        );
    }

    #[test]
    fn no_in_author_tail_is_untouched() {
        let mut c = ctx("x");
        let out = strip_in_author_citation(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.published_in, None);
        assert_eq!(c.pending_year, None);
    }

    #[test]
    fn lowercase_word_after_in_is_not_mistaken_for_an_author() {
        // "in obscurity" — the word after "in"/"apud" must start with an uppercase letter;
        // oracle-verified this whole string survives untouched into the raw authorship text.
        let mut c = ctx("x");
        let out = strip_in_author_citation(&mut c, "Foo bar Smith in obscurity".to_string());
        assert_eq!(out, "Foo bar Smith in obscurity");
        assert_eq!(c.name.published_in, None);
    }

    // ---- Step 49: stripIpniCitation ----

    #[test]
    fn documented_ipni_example_sets_published_in_and_year() {
        let mut c = ctx("x");
        let out = strip_ipni_citation(
            &mut c,
            "Foo bar Kirchn., Annals and Magazine of Natural History (1988).".to_string(),
        );
        assert_eq!(out, "Foo bar Kirchn.");
        assert_eq!(
            c.name.published_in,
            Some("Annals and Magazine of Natural History (1988)".to_string())
        );
        assert_eq!(c.name.published_in_year, Some(1988));
        assert_eq!(c.pending_year, Some("1988".to_string()));
    }

    #[test]
    fn embedded_nom_illeg_note_is_extracted_and_the_reference_is_resquished() {
        let mut c = ctx("x");
        let out = strip_ipni_citation(
            &mut c,
            "Foo bar Kirchn., Taxon nom. illeg. (1988).".to_string(),
        );
        assert_eq!(out, "Foo bar Kirchn.");
        assert_eq!(c.name.nomenclatural_note, Some("nom. illeg.".to_string()));
        assert_eq!(c.name.published_in, Some("Taxon (1988)".to_string()));
    }

    #[test]
    fn embedded_pro_syn_note_with_in_obs_prefix_is_extracted() {
        let mut c = ctx("x");
        let out = strip_ipni_citation(
            &mut c,
            "Foo bar Kirchn., Taxon in obs., pro syn. (1988).".to_string(),
        );
        assert_eq!(out, "Foo bar Kirchn.");
        assert_eq!(
            c.name.nomenclatural_note,
            Some("in obs., pro syn.".to_string())
        );
        assert_eq!(c.name.published_in, Some("Taxon (1988)".to_string()));
    }

    #[test]
    fn embedded_comb_nov_note_is_extracted() {
        let mut c = ctx("x");
        let out = strip_ipni_citation(
            &mut c,
            "Foo bar Kirchn., Taxon comb. nov. (1988).".to_string(),
        );
        assert_eq!(out, "Foo bar Kirchn.");
        assert_eq!(c.name.nomenclatural_note, Some("comb. nov.".to_string()));
        assert_eq!(c.name.published_in, Some("Taxon (1988)".to_string()));
    }

    #[test]
    fn ipni_appends_the_embedded_note_to_an_existing_nomenclatural_note() {
        let mut c = ctx("x");
        c.name.nomenclatural_note = Some("earlier note".to_string());
        let out = strip_ipni_citation(
            &mut c,
            "Foo bar Kirchn., Taxon nom. illeg. (1988).".to_string(),
        );
        assert_eq!(out, "Foo bar Kirchn.");
        assert_eq!(
            c.name.nomenclatural_note,
            Some("earlier note nom. illeg.".to_string())
        );
    }

    #[test]
    fn ipni_overwrites_rather_than_appends_an_existing_published_in() {
        let mut c = ctx("x");
        c.name.set_published_in("Stale ref, 1700");
        let out = strip_ipni_citation(
            &mut c,
            "Foo bar Kirchn., Annals and Magazine of Natural History (1988).".to_string(),
        );
        assert_eq!(out, "Foo bar Kirchn.");
        assert_eq!(
            c.name.published_in,
            Some("Annals and Magazine of Natural History (1988)".to_string()),
            "stripIpniCitation calls the plain setPublishedIn — it must OVERWRITE, not append"
        );
    }

    #[test]
    fn no_ipni_citation_is_untouched() {
        let mut c = ctx("x");
        let out = strip_ipni_citation(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.published_in, None);
    }

    // ---- Step 50: stripPeriodSeparatedReference ----

    #[test]
    fn clean_year_reference_overwrites_published_in_and_propagates_the_year() {
        let mut c = ctx("x");
        let out = strip_period_separated_reference(
            &mut c,
            "Foo bar Smith. Annals of Botany 1988".to_string(),
        );
        assert_eq!(out, "Foo bar Smith");
        assert_eq!(
            c.name.published_in,
            Some("Annals of Botany 1988".to_string())
        );
        assert_eq!(c.name.published_in_year, Some(1988));
        assert_eq!(c.pending_year, Some("1988".to_string()));
        assert!(!c
            .name
            .warnings
            .contains(&warnings::NOMENCLATURAL_REFERENCE.to_string()));
    }

    #[test]
    fn page_range_reference_flags_nomenclatural_reference_and_does_not_propagate_the_year() {
        // Oracle-verified: the reference must ALSO contain a connector word ("of" here) —
        // "Zootaxa 4759: 1658-1662" alone (no connector) never matches PERIOD_SEPARATED_
        // REFERENCE at all, since the pattern's trailing `\s+(?:of|in|...)#\s+.*` suffix is
        // mandatory, not optional.
        let mut c = ctx("x");
        let out = strip_period_separated_reference(
            &mut c,
            "Foo bar Smith. Annals of Zootaxa 1658-1662, 1988".to_string(),
        );
        assert_eq!(out, "Foo bar Smith");
        assert_eq!(
            c.name.published_in,
            Some("Annals of Zootaxa 1658-1662, 1988".to_string())
        );
        assert_eq!(c.name.published_in_year, Some(1988));
        assert!(c
            .name
            .warnings
            .contains(&warnings::NOMENCLATURAL_REFERENCE.to_string()));
        assert_eq!(
            c.pending_year, None,
            "a page-range reference must NOT propagate a pending year"
        );
    }

    #[test]
    fn author_particle_list_is_not_mistaken_for_a_reference() {
        // "Yin, Z.W. de Beer & Wingf." — a comma (not a period) follows "Yin", so this never
        // reaches PERIOD_SEPARATED_REFERENCE's leading `[\p{Lu}][\p{L}]{2,}\.` anchor at all;
        // oracle-verified the whole string survives into the raw (if mangled) authorship text.
        let mut c = ctx("x");
        let out = strip_period_separated_reference(
            &mut c,
            "Foo bar Yin, Z.W. de Beer & Wingf. Mycologia 2004".to_string(),
        );
        assert_eq!(out, "Foo bar Yin, Z.W. de Beer & Wingf. Mycologia 2004");
        assert_eq!(c.name.published_in, None);
    }

    #[test]
    fn no_period_separated_reference_is_untouched() {
        let mut c = ctx("x");
        let out = strip_period_separated_reference(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.published_in, None);
    }

    #[test]
    fn period_separated_reference_stays_linear_on_a_long_connector_free_run() {
        // PERIOD_SEPARATED_REFERENCE runs on fancy_regex (backtracking). A long run of
        // capitalised "words" after "Smith. " with NO connector word anywhere (so the
        // pattern's mandatory trailing `\s+(?:of|in|...)\s+.*` suffix never matches) forces
        // the filler `*+` group to consume the whole run and then fail the suffix. This is a
        // LINEARITY guard, not proof of an averted cliff: as the static's doc NOTE explains
        // and a scratch-crate spike measured directly, this pattern's `\s+ <word>` filler
        // makes every partition unambiguous, so plain greedy `*` is ALSO linear here (~735µs
        // vs `*+`'s ~735µs on a 640-word run) — unlike NOM_NOTE, where the possessive is
        // genuinely load-bearing. The `*+` is kept for faithfulness + defence-in-depth; this
        // test simply pins that the step stays fast regardless of run length.
        let mut c = ctx("x");
        let mut input = "Foo bar Smith.".to_string();
        for i in 0..60 {
            input.push_str(&format!(" Wordxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx{i}"));
        }
        let start = std::time::Instant::now();
        let out = strip_period_separated_reference(&mut c, input.clone());
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 500,
            "strip_period_separated_reference took {elapsed:?} on a connector-free run"
        );
        // No connector word anywhere -> the pattern's mandatory suffix never matches, so the
        // string comes back untouched, same as any other non-matching input.
        assert_eq!(out, input);
        assert_eq!(c.name.published_in, None);
    }

    // ---- Step 51: stripCommaPrefixedReference ----

    #[test]
    fn documented_comma_prefixed_reference_overwrites_published_in_and_flags_the_warning() {
        let mut c = ctx("x");
        let out = strip_comma_prefixed_reference(
            &mut c,
            "Foo bar Smith, Journal of Botany 1988".to_string(),
        );
        assert_eq!(out, "Foo bar Smith");
        assert_eq!(
            c.name.published_in,
            Some("Journal of Botany 1988".to_string())
        );
        assert!(c
            .name
            .warnings
            .contains(&warnings::NOMENCLATURAL_REFERENCE.to_string()));
    }

    #[test]
    fn comma_prefixed_reference_does_not_propagate_a_pending_year() {
        let mut c = ctx("x");
        strip_comma_prefixed_reference(&mut c, "Foo bar Smith, Journal of Botany 1988".to_string());
        assert_eq!(
            c.pending_year, None,
            "a comma-prefixed reference's year must never be propagated onto the authorship"
        );
    }

    #[test]
    fn comma_prefixed_reference_overwrites_rather_than_appends_an_existing_published_in() {
        let mut c = ctx("x");
        c.name.set_published_in("Stale ref, 1700");
        let out = strip_comma_prefixed_reference(
            &mut c,
            "Foo bar Miller & Jones, Annals and Magazine of Natural History (1988)".to_string(),
        );
        assert_eq!(out, "Foo bar Miller & Jones");
        assert_eq!(
            c.name.published_in,
            Some("Annals and Magazine of Natural History (1988)".to_string())
        );
    }

    #[test]
    fn author_particle_list_is_not_mistaken_for_a_comma_prefixed_reference_either() {
        // Same negative example as step 50's sibling test — "Z.W." can't open the captured
        // reference title (fails `[\p{Lu}][\p{Ll}]{2,}`), so this is never mistaken for a ref
        // by this step either.
        let mut c = ctx("x");
        let out = strip_comma_prefixed_reference(
            &mut c,
            "Foo bar Yin, Z.W. de Beer & Wingf. Mycologia 2004".to_string(),
        );
        assert_eq!(out, "Foo bar Yin, Z.W. de Beer & Wingf. Mycologia 2004");
        assert_eq!(c.name.published_in, None);
    }

    #[test]
    fn no_comma_prefixed_reference_is_untouched() {
        let mut c = ctx("x");
        let out = strip_comma_prefixed_reference(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.published_in, None);
        assert!(c.name.warnings.is_empty());
    }

    #[test]
    fn comma_prefixed_reference_does_not_redos_on_a_long_connector_free_run() {
        // COMMA_PREFIXED_REFERENCE runs on the plain `regex` crate (automaton-based, no
        // backtracking at all), so — unlike step 50's fancy_regex-based sibling — there is no
        // possessive quantifier to preserve and no ReDoS failure mode to guard against. Same
        // pathological input shape as step 50's regression test, so a future reader can see
        // both engines were checked, not just assumed.
        let mut c = ctx("x");
        let mut input = "Foo bar Smith,".to_string();
        for i in 0..300 {
            input.push_str(&format!(" Wordxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx{i}"));
        }
        let start = std::time::Instant::now();
        let out = strip_comma_prefixed_reference(&mut c, input.clone());
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 200,
            "strip_comma_prefixed_reference took {elapsed:?} on a connector-free run"
        );
        assert_eq!(out, input);
    }

    // ---- Step 52: stripManuscriptMarker ----

    #[test]
    fn trailing_ms_marker_sets_manuscript_and_appends_the_lowercased_tag() {
        let mut c = ctx("x");
        let out = strip_manuscript_marker(&mut c, "Acacia bicolor Bojer ms.".to_string());
        assert_eq!(out, "Acacia bicolor Bojer");
        assert!(c.name.manuscript);
        assert_eq!(c.name.nomenclatural_note, Some("ms.".to_string()));
    }

    #[test]
    fn ined_marker_with_leading_comma_is_also_recognised() {
        let mut c = ctx("x");
        let out = strip_manuscript_marker(&mut c, "Foo bar Author, ined.".to_string());
        assert_eq!(out, "Foo bar Author");
        assert!(c.name.manuscript);
        assert_eq!(c.name.nomenclatural_note, Some("ined.".to_string()));
    }

    #[test]
    fn unpublished_and_msc_markers_are_also_recognised() {
        let mut c1 = ctx("x");
        let out1 = strip_manuscript_marker(&mut c1, "Foo bar Author unpublished".to_string());
        assert_eq!(out1, "Foo bar Author");
        assert_eq!(c1.name.nomenclatural_note, Some("unpublished".to_string()));

        let mut c2 = ctx("x");
        let out2 = strip_manuscript_marker(&mut c2, "Foo bar Author msc".to_string());
        assert_eq!(out2, "Foo bar Author");
        assert_eq!(c2.name.nomenclatural_note, Some("msc".to_string()));
    }

    #[test]
    fn marker_case_is_lowercased_in_the_stashed_tag() {
        let mut c = ctx("x");
        let out = strip_manuscript_marker(&mut c, "Foo bar Author MS.".to_string());
        assert_eq!(out, "Foo bar Author");
        assert_eq!(c.name.nomenclatural_note, Some("ms.".to_string()));
    }

    #[test]
    fn manuscript_marker_appends_to_an_existing_nomenclatural_note() {
        let mut c = ctx("x");
        c.name.nomenclatural_note = Some("earlier note".to_string());
        let out = strip_manuscript_marker(&mut c, "Foo bar Author ms.".to_string());
        assert_eq!(out, "Foo bar Author");
        assert_eq!(
            c.name.nomenclatural_note,
            Some("earlier note ms.".to_string())
        );
    }

    #[test]
    fn manuscript_marker_after_in_author_strip_still_matches() {
        // "Busk ms in Chimonides, 1987" — in the real pipeline, strip_in_author_citation
        // (step 48) already consumed " in Chimonides, 1987" before this step runs, leaving
        // "Busk ms" for this step alone to finish (see the full-run test below).
        let mut c = ctx("x");
        let out = strip_manuscript_marker(&mut c, "Busk ms".to_string());
        assert_eq!(out, "Busk");
        assert!(c.name.manuscript);
        assert_eq!(c.name.nomenclatural_note, Some("ms".to_string()));
    }

    #[test]
    fn no_manuscript_marker_is_untouched() {
        let mut c = ctx("x");
        let out = strip_manuscript_marker(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert!(!c.name.manuscript);
        assert_eq!(c.name.nomenclatural_note, None);
    }

    // ---- Batch 2d cross-step interaction (full `run()`) ----

    #[test]
    fn full_run_strips_both_in_author_citation_and_manuscript_marker_in_order() {
        // Oracle-verified end-to-end (StripAndStash's own contribution): "Aus bus Busk ms in
        // Chimonides, 1987" must have the in-author tail stripped first (step 48), leaving
        // "Aus bus Busk ms" for the manuscript marker (step 52) to finish.
        let mut c = ctx("Aus bus Busk ms in Chimonides, 1987");
        run(&mut c);
        assert_eq!(c.working, "Aus bus Busk");
        assert!(c.name.manuscript);
        assert_eq!(c.name.nomenclatural_note, Some("ms".to_string()));
        assert_eq!(c.name.published_in, Some("Chimonides, 1987".to_string()));
        assert_eq!(c.name.published_in_year, Some(1987));
    }

    // ---- Step 53: stripSupraRankPrefix ----

    #[test]
    fn bare_suprageneric_marker_pins_rank_and_strips_the_marker() {
        // Oracle-verified: "subtrib. Scolochloinae Soreng" -> rank=SUBTRIBE,
        // uninomial=Scolochloinae, authors=[Soreng].
        let mut c = ctx("x");
        let out = strip_supra_rank_prefix(&mut c, "subtrib. Scolochloinae Soreng".to_string());
        assert_eq!(out, "Scolochloinae Soreng");
        assert_eq!(c.name.rank, Rank::Subtribe);
    }

    #[test]
    fn family_prefixed_suprageneric_marker_strips_both_family_and_marker() {
        // Oracle-verified: "Poaceae subtrib. Scolochloinae Soreng" -> IDENTICAL parse to
        // the bare-marker form above (rank=SUBTRIBE, uninomial=Scolochloinae) — the
        // family prefix "Poaceae" is discarded entirely, not preserved anywhere.
        let mut c = ctx("x");
        let out =
            strip_supra_rank_prefix(&mut c, "Poaceae subtrib. Scolochloinae Soreng".to_string());
        assert_eq!(out, "Scolochloinae Soreng");
        assert_eq!(c.name.rank, Rank::Subtribe);
    }

    #[test]
    fn tribe_and_subfamily_markers_are_also_recognised() {
        // Oracle-verified: "trib. Triticeae Dumort." -> rank=TRIBE; "subfam. Pooideae" ->
        // rank=SUBFAMILY.
        let mut c1 = ctx("x");
        let out1 = strip_supra_rank_prefix(&mut c1, "trib. Triticeae Dumort.".to_string());
        assert_eq!(out1, "Triticeae Dumort.");
        assert_eq!(c1.name.rank, Rank::Tribe);

        let mut c2 = ctx("x");
        let out2 = strip_supra_rank_prefix(&mut c2, "subfam. Pooideae".to_string());
        assert_eq!(out2, "Pooideae");
        assert_eq!(c2.name.rank, Rank::Subfamily);
    }

    #[test]
    fn no_supra_rank_prefix_is_untouched() {
        let mut c = ctx("x");
        let out = strip_supra_rank_prefix(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.rank, Rank::Unranked);
    }

    // ---- Step 54: stripLeadingInfragenericMarker ----

    #[test]
    fn leading_infragen_marker_pins_rank_and_strips_the_marker() {
        // Oracle-verified: "subgen. Trematostoma Sacc." -> rank=SUBGENUS, no `code` key at
        // all in the wire output (Subgenus carries none — no backfill).
        let mut c = ctx("x");
        let out =
            strip_leading_infrageneric_marker(&mut c, "subgen. Trematostoma Sacc.".to_string());
        assert_eq!(out, "Trematostoma Sacc.");
        assert_eq!(c.name.rank, Rank::Subgenus);
        assert_eq!(c.name.code, None);
    }

    #[test]
    fn leading_infragen_marker_backfills_unset_code_from_the_ranks_own_code() {
        // Oracle-verified: "sect. Foo Bar" (no caller-supplied code) -> rank=SECTION_BOTANY,
        // code=BOTANICAL (backfilled — Subgenus above has no code, but SectionBotany does).
        let mut c = ctx("x");
        let out = strip_leading_infrageneric_marker(&mut c, "sect. Foo Bar".to_string());
        assert_eq!(out, "Foo Bar");
        assert_eq!(c.name.rank, Rank::SectionBotany);
        assert_eq!(c.name.code, Some(NomCode::Botanical));
    }

    #[test]
    fn leading_infragen_marker_no_trailing_dot_form_is_also_recognised() {
        // The marker's trailing dot is optional (`\.?`); "series" (no dot) must match too.
        let mut c = ctx("x");
        let out = strip_leading_infrageneric_marker(&mut c, "series Foo Bar".to_string());
        assert_eq!(out, "Foo Bar");
        assert_eq!(c.name.rank, Rank::SeriesBotany);
        assert_eq!(c.name.code, Some(NomCode::Botanical));
    }

    #[test]
    fn leading_infragen_marker_remaps_to_zoological_counterpart_under_zoological_code() {
        // Oracle-verified (via a ColDP TSV row with code=Zoological): "sect. Taeda" ->
        // rank=SECTION_ZOOLOGY (remapped from SECTION_BOTANY), not SECTION_BOTANY.
        let mut c = ParseContext::new("x".to_string(), None, None, Some(NomCode::Zoological));
        let out = strip_leading_infrageneric_marker(&mut c, "sect. Taeda".to_string());
        assert_eq!(out, "Taeda");
        assert_eq!(c.name.rank, Rank::SectionZoology);
        assert_eq!(c.name.code, Some(NomCode::Zoological));
    }

    #[test]
    fn leading_infragen_marker_subgenus_has_no_zoological_counterpart_to_remap_to() {
        // Oracle-verified (via a ColDP TSV row with code=Zoological): "subgen. Trematostoma
        // Sacc." -> rank stays SUBGENUS even under a caller-supplied ZOOLOGICAL code — Java's
        // BOT_TO_ZOOL map has no SUBGENUS entry, so `bot_to_zool` returns None and the rank
        // is left as-is (`r` unchanged, not remapped).
        let mut c = ParseContext::new("x".to_string(), None, None, Some(NomCode::Zoological));
        let out =
            strip_leading_infrageneric_marker(&mut c, "subgen. Trematostoma Sacc.".to_string());
        assert_eq!(out, "Trematostoma Sacc.");
        assert_eq!(c.name.rank, Rank::Subgenus);
    }

    #[test]
    fn leading_infragen_marker_does_not_overwrite_an_already_set_code() {
        // Java: `if (r.getCode() != null && ctx.name.getCode() == null)` — an existing
        // code (e.g. from an earlier step in the same run, such as strip_candidatus
        // setting BACTERIAL) must survive untouched, unlike step 53's unconditional rank
        // overwrite.
        let mut c = ctx("x");
        c.name.code = Some(NomCode::Bacterial);
        let out = strip_leading_infrageneric_marker(&mut c, "sect. Foo Bar".to_string());
        assert_eq!(out, "Foo Bar");
        assert_eq!(c.name.rank, Rank::SectionBotany, "rank is still pinned");
        assert_eq!(
            c.name.code,
            Some(NomCode::Bacterial),
            "a pre-existing code must not be overwritten"
        );
    }

    #[test]
    fn no_leading_infragen_marker_is_untouched() {
        let mut c = ctx("x");
        let out = strip_leading_infrageneric_marker(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.rank, Rank::Unranked);
        assert_eq!(c.name.code, None);
    }

    // ---- Step 55: stashPhraseName ----

    #[test]
    fn phrase_name_bare_genus_with_species_marker_sets_phrase_but_leaves_rank_untouched() {
        // Oracle-verified: "Prostanthera sp. Somersbey (B.J.Conn 4024)" -> phrase=
        // "Somersbey (B.J.Conn 4024)". The eventual full-pipeline `rank=SPECIES` comes from
        // NameTokens re-reading the reinserted "sp." marker, NOT from this
        // step directly — Java's own stashPhraseName has no setRank call on this branch.
        let mut c = ctx("x");
        let out = stash_phrase_name(
            &mut c,
            "Prostanthera sp. Somersbey (B.J.Conn 4024)".to_string(),
        );
        assert_eq!(out, "Prostanthera sp.");
        assert_eq!(c.name.phrase, Some("Somersbey (B.J.Conn 4024)".to_string()));
        assert_eq!(
            c.name.rank,
            Rank::Unranked,
            "bare genus + \"sp.\" marker must NOT pin rank directly in this step"
        );
    }

    #[test]
    fn phrase_name_bare_genus_with_non_species_marker_pins_rank_and_drops_the_marker() {
        // Oracle-verified: "Grevillea subsp. 'Short Leaf'" -> rank=SUBSPECIES,
        // phrase="'Short Leaf'" (quotes are part of the captured phrase text, not
        // stripped), uninomial reduces to the bare genus "Grevillea".
        let mut c = ctx("x");
        let out = stash_phrase_name(&mut c, "Grevillea subsp. 'Short Leaf'".to_string());
        assert_eq!(out, "Grevillea");
        assert_eq!(c.name.rank, Rank::Subspecies);
        assert_eq!(c.name.phrase, Some("'Short Leaf'".to_string()));
    }

    #[test]
    fn phrase_name_digit_leading_quoted_phrase_is_also_recognised() {
        // Oracle-verified: "Baeckea ssp. 2 (LJM 2019)" -> rank=SUBSPECIES,
        // phrase="2 (LJM 2019)" — group 3's first alternative allows a leading digit.
        let mut c = ctx("x");
        let out = stash_phrase_name(&mut c, "Baeckea ssp. 2 (LJM 2019)".to_string());
        assert_eq!(out, "Baeckea");
        assert_eq!(c.name.rank, Rank::Subspecies);
        assert_eq!(c.name.phrase, Some("2 (LJM 2019)".to_string()));
    }

    #[test]
    fn phrase_name_genus_plus_subgenus_extracts_infrageneric_epithet() {
        // Oracle-verified: "Acacia (Botrycephalae) sp. Bygalorie (P.G.Wilson 2585)" ->
        // rank=SPECIES, infragenericEpithet=Botrycephalae, uninomial reduces to "Acacia"
        // (the "(Botrycephalae)" parens are dropped from the working string entirely,
        // extracted onto a dedicated field instead).
        let mut c = ctx("x");
        let out = stash_phrase_name(
            &mut c,
            "Acacia (Botrycephalae) sp. Bygalorie (P.G.Wilson 2585)".to_string(),
        );
        assert_eq!(out, "Acacia");
        assert_eq!(c.name.rank, Rank::Species);
        assert_eq!(
            c.name.infrageneric_epithet,
            Some("Botrycephalae".to_string())
        );
        assert_eq!(
            c.name.phrase,
            Some("Bygalorie (P.G.Wilson 2585)".to_string())
        );
    }

    #[test]
    fn phrase_name_trailing_author_splices_the_marker_before_the_author() {
        // Oracle-verified: "Baeckea Benth. sp. Bygalorie (ABC 123)" -> working rewritten to
        // "Baeckea sp. Benth." (marker moved BEFORE the author span so it trails as the
        // species author for AuthorshipParser); rank untouched by this step
        // (same "downstream infers it from the reinserted marker" situation as the bare
        // "sp."-marker case above).
        let mut c = ctx("x");
        let out = stash_phrase_name(&mut c, "Baeckea Benth. sp. Bygalorie (ABC 123)".to_string());
        assert_eq!(out, "Baeckea sp. Benth.");
        assert_eq!(c.name.phrase, Some("Bygalorie (ABC 123)".to_string()));
        assert_eq!(c.name.rank, Rank::Unranked);
    }

    #[test]
    fn phrase_name_double_quoted_phrase_without_parens_is_also_recognised() {
        // Oracle-verified: "Prostanthera sp. \"Big Leaf\"" -> phrase="\"Big Leaf\""
        // (quotes kept verbatim, matching group 3's second, no-parens alternative).
        let mut c = ctx("x");
        let out = stash_phrase_name(&mut c, "Prostanthera sp. \"Big Leaf\"".to_string());
        assert_eq!(out, "Prostanthera sp.");
        assert_eq!(c.name.phrase, Some("\"Big Leaf\"".to_string()));
    }

    #[test]
    fn phrase_without_parens_or_quotes_is_not_recognised_as_a_phrase_name() {
        // Group 3 requires EITHER parens OR quotes in the phrase text; a bare trailing
        // author span with neither ("P.G.Wilson 2585 F.Muell.") must not match at all —
        // oracle-verified: this exact string parses via the ordinary (doubtful) authorship
        // path instead, with no `phrase` field set.
        let mut c = ctx("x");
        let out = stash_phrase_name(&mut c, "Baeckea sp. P.G.Wilson 2585 F.Muell.".to_string());
        assert_eq!(out, "Baeckea sp. P.G.Wilson 2585 F.Muell.");
        assert_eq!(c.name.phrase, None);
    }

    #[test]
    fn no_phrase_name_shape_is_untouched() {
        let mut c = ctx("x");
        let out = stash_phrase_name(&mut c, "Abies alba Mill.".to_string());
        assert_eq!(out, "Abies alba Mill.");
        assert_eq!(c.name.phrase, None);
        assert_eq!(c.name.rank, Rank::Unranked);
    }

    // ---- Batch 2e cross-step interaction (full `run()`) ----

    #[test]
    fn full_run_strips_a_leading_infrageneric_marker_and_backfills_the_code() {
        // Oracle-verified end-to-end: "sect. Taeda" -> rank=SECTION_BOTANY, code=BOTANICAL,
        // with none of steps 1-52 interfering before step 54 gets to it (a bare "sect."
        // prefix has a period right after it, so it doesn't trip step 4's
        // missing-genus-placeholder heuristic the way an unpunctuated word would).
        let mut c = ctx("sect. Taeda");
        run(&mut c);
        assert_eq!(c.working, "Taeda");
        assert_eq!(c.name.rank, Rank::SectionBotany);
        assert_eq!(c.name.code, Some(NomCode::Botanical));
    }

    #[test]
    fn full_run_stashes_a_phrase_name() {
        // Oracle-verified end-to-end (StripAndStash's own contribution): "Prostanthera sp.
        // Somersbey (B.J.Conn 4024)" -> phrase="Somersbey (B.J.Conn 4024)", working
        // rewritten to "Prostanthera sp." — none of steps 1-52 interfere on this clean,
        // capital-letter-led input before step 55 gets to it.
        let mut c = ctx("Prostanthera sp. Somersbey (B.J.Conn 4024)");
        run(&mut c);
        assert_eq!(c.working, "Prostanthera sp.");
        assert_eq!(c.name.phrase, Some("Somersbey (B.J.Conn 4024)".to_string()));
    }

    // ===================================================================================
    // strip_authorship_markers (Phase 1 Slice 4 Task 4) — the aux-authorship-path
    // reimplementation. `name()` builds a fresh `ParsedName` since this function has no
    // `ParseContext` of its own.
    // ===================================================================================

    fn name() -> ParsedName {
        ParsedName::default()
    }

    #[test]
    fn standalone_manuscript_marker_sets_manuscript_and_returns_empty() {
        let mut n = name();
        let out = strip_authorship_markers("ined.", &mut n);
        assert_eq!(out, "");
        assert!(n.manuscript);
    }

    #[test]
    fn standalone_manuscript_marker_matches_case_insensitively_without_the_dot() {
        let mut n = name();
        let out = strip_authorship_markers("MS", &mut n);
        assert_eq!(out, "");
        assert!(n.manuscript);
    }

    #[test]
    fn blank_authorship_returns_empty_without_touching_name() {
        let mut n = name();
        let out = strip_authorship_markers("   ", &mut n);
        assert_eq!(out, "");
        assert_eq!(n, ParsedName::default());
    }

    #[test]
    fn sic_with_comment_strips_and_flags_original_spelling_true() {
        let mut n = name();
        let out = strip_authorship_markers("L. (sic, misspelling)", &mut n);
        assert_eq!(out, "L.");
        assert_eq!(n.original_spelling, Some(true));
    }

    #[test]
    fn plain_sic_flags_original_spelling_true() {
        let mut n = name();
        let out = strip_authorship_markers("L. (sic)", &mut n);
        assert_eq!(out, "L.");
        assert_eq!(n.original_spelling, Some(true));
    }

    #[test]
    fn corrig_flags_original_spelling_false() {
        let mut n = name();
        let out = strip_authorship_markers("L. corrig.", &mut n);
        assert_eq!(out, "L.");
        assert_eq!(n.original_spelling, Some(false));
    }

    #[test]
    fn question_mark_transcription_artefact_glues_and_flags_doubtful() {
        let mut n = name();
        let out = strip_authorship_markers("Istv?nffi", &mut n);
        assert_eq!(out, "Istvnffi");
        assert!(n.doubtful);
        assert!(n
            .warnings
            .contains(&warnings::QUESTION_MARKS_REMOVED.to_string()));
    }

    #[test]
    fn win1252_artefact_is_repaired_with_a_homoglyphs_warning() {
        let mut n = name();
        let out = strip_authorship_markers("Plesn\u{00A1}k", &mut n);
        assert_eq!(out, "Plesnik");
        assert!(n.warnings.contains(&warnings::HOMOGLYHPS.to_string()));
    }

    #[test]
    fn hort_ex_placeholder_is_lowercased() {
        let mut n = name();
        let out = strip_authorship_markers("Hort. ex Voss", &mut n);
        assert_eq!(out, "hort. ex Voss");
    }

    #[test]
    fn hortus_ex_placeholder_is_lowercased() {
        let mut n = name();
        let out = strip_authorship_markers("hortus ex Someone", &mut n);
        assert_eq!(out, "hort. ex Someone");
    }

    #[test]
    fn cv_ex_is_deliberately_left_untouched() {
        // Unlike the main `run()` dispatch's `normalise_hort_ex_placeholder` (which also
        // handles CV_EX/HT_MARKER), the aux path calls ONLY HORT_EX/HORTUS_EX — see the
        // section doc comment.
        let mut n = name();
        let out = strip_authorship_markers("cv. ex Someone", &mut n);
        assert_eq!(out, "cv. ex Someone");
    }

    #[test]
    fn leading_homonym_paren_captures_the_whole_string_as_taxonomic_note() {
        let mut n = name();
        let out = strip_authorship_markers("(non Smith, 1900)", &mut n);
        assert_eq!(out, "");
        assert_eq!(n.taxonomic_note, Some("(non Smith, 1900)".to_string()));
    }

    #[test]
    fn paren_note_splits_off_a_leading_note_and_keeps_the_trailing_author() {
        let mut n = name();
        let out = strip_authorship_markers("(auct.) Rolfe", &mut n);
        assert_eq!(out, "Rolfe");
        assert_eq!(n.taxonomic_note, Some("auct.".to_string()));
    }

    #[test]
    fn paren_note_keeps_a_leading_author_case_intact() {
        // The Java doc-comment example: "(sensu X, 1878) Y, 1992" -> author "Y, 1992" +
        // note "sensu X, 1878" (unchanged — only a leading "Auct"/"Auctt" is lower-cased).
        let mut n = name();
        let out = strip_authorship_markers("(sensu X, 1878) Y, 1992", &mut n);
        assert_eq!(out, "Y, 1992");
        assert_eq!(n.taxonomic_note, Some("sensu X, 1878".to_string()));
    }

    #[test]
    fn bracketed_nom_note_is_extracted_and_stripped() {
        let mut n = name();
        let out = strip_authorship_markers("L. (nom. cons.)", &mut n);
        assert_eq!(out, "L.");
        assert_eq!(n.nomenclatural_note, Some("nom. cons.".to_string()));
    }

    #[test]
    fn bare_nom_note_is_extracted_and_stripped() {
        let mut n = name();
        let out = strip_authorship_markers("L. nom. illeg.", &mut n);
        assert_eq!(out, "L.");
        assert_eq!(n.nomenclatural_note, Some("nom. illeg.".to_string()));
    }

    #[test]
    fn bare_nom_note_anchored_at_the_very_start_is_still_caught() {
        // NOM_NOTE itself requires a leading whitespace boundary — the space-padded copy
        // ("`padded_nom`") is what lets a note anchored at position 0 of `s` match at all.
        let mut n = name();
        let out = strip_authorship_markers("nom. illeg.", &mut n);
        assert_eq!(out, "");
        assert_eq!(n.nomenclatural_note, Some("nom. illeg.".to_string()));
    }

    #[test]
    fn bare_nom_note_with_ined_keyword_flags_manuscript() {
        let mut n = name();
        let out = strip_authorship_markers("L. sp. nov. ined.", &mut n);
        assert_eq!(out, "L.");
        assert!(n.manuscript);
    }

    #[test]
    fn tax_note_sensu_is_extracted_and_stripped() {
        let mut n = name();
        let out = strip_authorship_markers("L. sensu Smith", &mut n);
        assert_eq!(out, "L.");
        assert_eq!(n.taxonomic_note, Some("sensu Smith".to_string()));
    }

    #[test]
    fn tax_note_initial_dot_space_is_collapsed() {
        // Doc-comment example on INITIAL_DOT_SPACE: "non F. europaeus" -> "non F.europaeus".
        let mut n = name();
        let out = strip_authorship_markers("L. non F. europaeus", &mut n);
        assert_eq!(out, "L.");
        assert_eq!(n.taxonomic_note, Some("non F.europaeus".to_string()));
    }

    #[test]
    fn tax_note_leading_auct_title_case_is_lowercased() {
        let mut n = name();
        let out = strip_authorship_markers("L. Auct.", &mut n);
        assert_eq!(out, "L.");
        assert_eq!(n.taxonomic_note, Some("auct.".to_string()));
    }

    #[test]
    fn tax_note_dedups_against_an_identical_existing_note() {
        // UNLIKE every other note-setter in this function, the final TAX_NOTE step dedups
        // rather than always appending — calling it twice with the same trailing note must
        // not double it up.
        let mut n = name();
        n.taxonomic_note = Some("sensu Smith".to_string());
        let out = strip_authorship_markers("L. sensu Smith", &mut n);
        assert_eq!(out, "L.");
        assert_eq!(n.taxonomic_note, Some("sensu Smith".to_string()));
    }

    #[test]
    fn tax_note_appends_a_distinct_existing_note() {
        let mut n = name();
        n.taxonomic_note = Some("auct.".to_string());
        let out = strip_authorship_markers("L. sensu Smith", &mut n);
        assert_eq!(out, "L.");
        assert_eq!(n.taxonomic_note, Some("auct. sensu Smith".to_string()));
    }

    #[test]
    fn plain_authorship_with_no_markers_is_returned_verbatim() {
        let mut n = name();
        let out = strip_authorship_markers("Mill.", &mut n);
        assert_eq!(out, "Mill.");
        assert_eq!(n, ParsedName::default());
    }
}
