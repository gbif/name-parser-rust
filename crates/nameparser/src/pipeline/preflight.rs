// SPDX-License-Identifier: Apache-2.0
//! Java `org.gbif.nameparser.pipeline.Preflight` — the 33-pattern early gate that rejects
//! viruses, hybrid formulas, placeholders, OTU codes and other non-scientific-name input
//! before the rest of the pipeline tokenises it.

use std::sync::LazyLock;

use regex::Regex;

use crate::model::{NameType, NomCode, ParseError};
use crate::pipeline::ParseContext;
use crate::unicode::java_trim;
use crate::viral::is_viral;

// Per-pattern flag rule applied throughout this file (see the Task 5 brief/report for the
// full rationale, worked out once here rather than repeated per pattern):
//
//   * Java `Pattern.UNICODE_CHARACTER_CLASS` patterns keep the `regex` crate's DEFAULT
//     Unicode-aware shorthand classes (`\s`/`\d`/`\w`/`\b` all Unicode already) — no
//     `(?-u:…)` scoping. `\p{Lu}`/`\p{Ll}`/`\p{L}` are Unicode in both engines regardless.
//   * Java `CASE_INSENSITIVE`-or-no-flags patterns become `(?i)` (only where Java actually
//     set the flag/inline modifier) with every `\s`/`\d`/`\w`/`\b` ASCII-scoped via
//     `(?-u:…)`, since Java did NOT set UNICODE_CHARACTER_CLASS for these and so its
//     shorthand classes are ASCII-only. `\p{Lu}`/`\p{Ll}`/`\p{L}` (where present) are left
//     untouched in the ambient Unicode scope.
//   * Where a pattern has no `\p{…}` class at all, the WHOLE alternation is wrapped in one
//     `(?-u:…)` group rather than scoping each shorthand class individually — flag scoping
//     applies to the entire AST subtree under the group (including nested character
//     classes like `[\s_-]`), so this is exactly equivalent to per-atom scoping, just
//     shorter and harder to typo across a long alternation. This is NEVER done for a
//     pattern containing an unescaped wildcard `.` (disabling `u` turns `.` from "any
//     Unicode scalar value" into "any byte", a real semantic change — regex-syntax rejects
//     `(?-u:…)` outright when it contains `\p{…}`/`\W`, but silently changes `.`'s meaning
//     instead of erroring) — those patterns (DELETE_MARKER, NON_HOMONYM, NN_PLACEHOLDER)
//     scope only the individual `\s` atoms, leaving every `.` outside any `(?-u:…)` group so
//     it stays the default Unicode "any scalar value except newline".
//   * Java's possessive quantifiers (`++`, `*+`) appear only in `ZOOLOGICAL_BINOMIAL`;
//     dropped to plain greedy (`+`, `*`) per this port's established convention (see
//     `regexes.rs`'s module doc) — the `regex` crate is a linear-time automaton with no
//     backtracking, so possessive-vs-greedy cannot change whether an overall match exists.
//   * Every one of the 33 patterns below is expressible in the linear `regex` crate; NONE
//     needs `fancy_regex`. Exactly one, `PURE_ALPHANUM`, is restructured (its lookahead
//     `(?=.*\d)` dropped) rather than ported verbatim — see that pattern's own doc comment.

// ---------- VIRUS ----------
// Match anything ending in -virus / -viruses / -viroid / -viroids / -phage(s) /
// -satellite, plus standalone viral keywords. Word characters can precede the
// suffix (so "Sapovirus", "papillomavirus", "C2-like viruses" all match).
// Java: `Pattern.CASE_INSENSITIVE`. No `\p{…}`, so the whole alternation is `(?-u:…)`-wrapped.
static VIRUS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?-u:(?:viru(?:s|ses)\b|viroid(?:s)?\b|phages?|virion(?:s)?\b|\bsatellite\b|(?:alpha|beta|delta|circular)[\s_-]*satellites?\b|\b(?:Clecru|Milvet|Subclov)satellite\b|bacteriophages?\b|\b[MSC]?NPV\b|\bGV\b|\bICTV\b|(?:fusion\s+)?vector\b|\bprions?\b|\bparticles?\b|\breplicons?\b|\bRNA\b))",
    )
    .unwrap()
});

// "Genus species [(Subgenus)] [Author], YYYY" — zoological author-year pattern.
// Allows VIRUS-matching epithets (vector, virus, phage) to be parsed as real species when
// an explicit Title-cased author + 4-digit year follows. Java: `Pattern.UNICODE_CHARACTER_CLASS`,
// every quantifier possessive (ReDoS-hardened — see Java's own comment); possessive → greedy
// per this port's convention (linear-time automaton, no backtracking to guard against).
static ZOOLOGICAL_BINOMIAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\p{Lu}\p{Ll}+\s+(?:\(\p{Lu}\p{Ll}+\)\s+)?\p{Ll}[\p{Ll}\-]*\s+(?:\([^)]*\)\s*)?\p{Lu}[\p{Lu}.,&'\-\s]*\p{Ll}[\p{L}.,&'\-\s]*\b(1[6-9]\d\d|20\d\d)\b",
    )
    .unwrap()
});

/// Java: `Pattern.UNICODE_CHARACTER_CLASS`, no scoping.
static CLEAN_BINOMIAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\p{Lu}\p{Ll}+(?:\s+\(\p{Lu}\p{Ll}+\))?\s+\p{Ll}[\p{Ll}\d\-]*$").unwrap()
});
/// Java: `Pattern.UNICODE_CHARACTER_CLASS`, no scoping.
static CLEAN_MONOMIAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\p{Lu}[\p{Ll}\-]+$").unwrap());

/// Java: `Pattern.CASE_INSENSITIVE`. No `\s`/`\d`/`\w`/`\b` at all, so nothing to ASCII-scope.
static SOFT_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:vector|prions?|particles?|replicons?|rna)$").unwrap());
// A soft virus-word appearing as the leading Title-cased GENUS token — "Prion vittatus",
// "Prion Lacépède, 1799". These are real animal genera (Prion = petrels), not viruses. The
// reject only fires on them when a genuinely viral token (HARD_VIRUS below) is also present.
// Case-sensitive and anchored so a lowercase epithet ("Euragallia prion") or a viral genus with
// a suffix ("Rnavirus …", where "Rna" is not followed by a word boundary) never matches.
// Java: no flags at all (deliberately case-SENSITIVE — no `(?i)`). Has `\b`, no `\p{…}` → whole
// pattern (after `^`) is `(?-u:…)`-wrapped.
static SOFT_GENUS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?-u:(?:Vector|Prions?|Particles?|Replicons?|Rna)\b)").unwrap());
// The genuinely viral triggers (VIRUS minus the ambiguous English SOFT_WORDs). When only a soft
// word matched, there is no real viral signal and a leading soft-genus is let through to parse.
// Java: `Pattern.CASE_INSENSITIVE`. Same treatment as VIRUS.
static HARD_VIRUS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?-u:(?:viru(?:s|ses)\b|viroid(?:s)?\b|phages?|virion(?:s)?\b|\bsatellite\b|(?:alpha|beta|delta|circular)[\s_-]*satellites?\b|\b(?:Clecru|Milvet|Subclov)satellite\b|bacteriophages?\b|\b[MSC]?NPV\b|\bGV\b|\bICTV\b))",
    )
    .unwrap()
});
/// Java: `Pattern.CASE_INSENSITIVE`. No `\s`/`\d`/`\w`/`\b`, nothing to ASCII-scope.
static HARD_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:virus|viroid|phages?|virion|satellite)$").unwrap());
/// Java: `Pattern.UNICODE_CHARACTER_CLASS`, no scoping.
static AUTH_ZOO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\(?\p{Lu}\p{Ll}{2,}.*\b(?:1[6-9]\d\d|20\d\d)\b").unwrap());

// ---------- HYBRID FORMULA ----------
// Hybrid-formula detection is done structurally in looks_like_hybrid_formula() below
// rather than with a single regex — it needs to inspect the spans on either side of
// the cross to avoid false positives on single-genus notho markers.

// ---------- NO_NAME ----------
// Pure alphanumeric codes / OTU identifiers.
/// Java: `Pattern.CASE_INSENSITIVE`. No `\s`/`\d`/`\w`/`\b` (`[A-Z0-9]`/`[:_-]` are explicit
/// ranges, not shorthand classes), nothing to ASCII-scope.
static OTU_BOLD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^BOLD[:_-][A-Z0-9]+$").unwrap());
/// Java: `Pattern.CASE_INSENSITIVE`. Has `\d` → scoped.
static OTU_SH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^SH(?-u:\d{6,})\.[0-9A-Z.]+$").unwrap());
/// Java: `Pattern.CASE_INSENSITIVE`. Has `\d` (in a class) → scoped.
static OTU_GTDB_UBA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:UBA|GTDB|GCA|GCF)(?-u:[\d_-]+)$").unwrap());
// Anything starting with a digit, or containing only mixed letters+digits (no Latin epithet
// shape). Java: `Pattern.UNICODE_CHARACTER_CLASS`, `^(?=.*\d)[\p{L}\d_.\-]+$` — a lookahead
// requiring a digit somewhere. RESTRUCTURED: the lookahead is dropped here (the `regex` crate
// has no lookaround at all) — this is sound rather than a behavioural gap because the ONLY
// call site (below, in `run`) already ANDs the match with a separate `has_digit(s)` check:
// Java's `PURE_ALPHANUM.matcher(s).matches() && hasDigit(s)` already has the digit requirement
// twice over (once via the lookahead, once via the explicit call), so
// `restructured_matches(s) && has_digit(s)` is exactly equivalent to Java's
// `PURE_ALPHANUM.matches(s) && hasDigit(s)` at that one call site.
static PURE_ALPHANUM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\p{L}\d_.\-]+$").unwrap());
// Patterns like "Basal_Cryptophyceae-1" — single underscore-separated parts with a digit
// suffix. Java: `Pattern.UNICODE_CHARACTER_CLASS`, no scoping.
// NB (dead-code finding, not fixed — faithful port keeps it): every character PR2_LIKE's
// regex can match ({L, `_`, `-`, digit}) is already inside PURE_ALPHANUM's broader alphabet
// ({L, digit, `_`, `.`, `-`}), and both call sites require `hasDigit(s)`; the only extra
// requirement PR2_LIKE's caller adds is `s.contains("-")`. So whenever PR2_LIKE's full
// call-site condition would hold, PURE_ALPHANUM's (checked first, above) already does too —
// this appears to make PR2_LIKE unreachable in the ORIGINAL JAVA source itself (independent
// of the restructuring above), not something this port introduced. Ported verbatim anyway,
// dead code and all, since removing it would be an unfaithful "fix" of Preflight.java.
static PR2_LIKE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{L}]+_[\p{L}\-]+(?:-\d+)?$").unwrap());
/// Java: `Pattern.CASE_INSENSITIVE`. Has `\s` (x3), no `\p{…}`, no wildcard `.` (only escaped
/// `\.`) → whole pattern (after `^`) is `(?-u:…)`-wrapped.
static GEN_NOV: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?-u:Gen\.?\s*nov\.?(?:\s+(?:sp|species)\.?\s*nov\.?)?\s*)$").unwrap()
});
// GTDB/SILVA specimen codes: anything ending with "sp" + 8+ digits after whitespace.
/// Java: `Pattern.CASE_INSENSITIVE`. Has `\b`, `\d`, no `\p{…}` → whole pattern wrapped
/// (keeping the trailing `$` outside the wrap since anchors aren't `u`-sensitive either way).
static OTU_SPECIMEN_SUFFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?-u:\bsp\d{8,})$").unwrap());

// ---------- PLACEHOLDER ----------
/// Java: `Pattern.CASE_INSENSITIVE`. Has `\s`/`\b` (many), no `\p{…}`, no wildcard `.` (only
/// escaped `\.`) → whole alternation `(?-u:…)`-wrapped.
static PLACEHOLDER_KEYWORDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?-u:(?:\(delete\)|\b(?:incertae[\s_]*sedis|inc\.\s*sed\.?|incertaesedis|not\s+assigned|unassigned|unknown|unaccepted|unidentified|undetermined|undet|indet\.?|indeterminate|uncultured|undescribed(?:\s+(?:species|genus|family))?|temp\s+dummy(?:\s+name)?)\b))",
    )
    .unwrap()
});
/// Java: `Pattern.CASE_INSENSITIVE`. Has `\s` (in `[-\s]`), no `\p{…}` → whole pattern
/// (after `^`) wrapped.
static PLACEHOLDER_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?-u:(?:Unident|Undescribed|IncertaeSedis|Undet)(?:[-\s]|$))").unwrap()
});
/// Java: no flags. Has `\s` AND `\p{Ll}` → only `\s` is scoped (matches the brief's own
/// worked example verbatim: `^\?\s+\p{Ll}` → `^\?(?-u:\s+)\p{Ll}`).
static QUESTION_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\?(?-u:\s+)\p{Ll}").unwrap());
// Two or more leading question marks ("?? not a name", "? ? foo"). A single leading "?" is a
// valid missing-genus placeholder ("? gryphoidis"), but a run of them is junk — the tokeniser
// would otherwise emit one "?" placeholder per mark and coerce the input into a nonsensical
// "? ? …" INFORMAL name. Reject as OTHER instead.
/// Java: no flags. Has `\s`, no `\p{…}` → whole pattern (after `^`) wrapped.
static MULTI_QUESTION_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\?(?-u:\s*)\?").unwrap());
/// Java: no flags (deliberately case-sensitive: `[Nn]` spells out the one letter allowed to
/// vary, so a lowercase-only "n. n." never matches). Has `\s` (x3) AND a wildcard `.` inside
/// `\(.*\)` → only the three `\s` atoms are individually scoped, leaving the wildcard `.`
/// (and the two literal `\.` after "N"/"n") untouched.
static NN_PLACEHOLDER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^N\.(?-u:\s*)[Nn]\.?(?:(?-u:\s*)\(.*\))?(?-u:\s*)$").unwrap());
// "Genus indet." / "Genus indet" patterns are INFORMAL, not PLACEHOLDER.
/// Java: `Pattern.UNICODE_CHARACTER_CLASS`, no scoping.
static INDET_SPECIES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[\p{Lu}][\p{L}\-]+(?:\s+[\p{Lu}][\p{L}]+)?\s+(?:indet|undet)\.?\s*$").unwrap()
});

// "clade" as a standalone word: a phylogenetic clade label, not a Linnean name.
/// Java: `Pattern.CASE_INSENSITIVE`. Has `\b` (x2), no `\p{…}` → whole pattern wrapped.
static CLADE_KEYWORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?-u:\bclade\b)").unwrap());

// Monomial aggregate forms: "Iteaphila-group" / "Bartonella group" — informal
// taxonomic group labels that can refer to any rank, so we reject them as INFORMAL.
/// Java: `Pattern.UNICODE_CHARACTER_CLASS`, no scoping.
static MONOMIAL_AGGREGATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[\p{Lu}][\p{L}]+(?:-group|\s+group|-complex|\s+complex)$").unwrap()
});

// "-lineage" / " lineage" labels ("Vermistella-lineage", "NC12A-lineage", "he2-lineage"):
// informal phylogenetic lineage names that, like the -group / -complex aggregates, can
// refer to any rank. Unlike those, the stem is often an OTU-/strain-like code with digits
// or a lowercase start, so the stem accepts any letter case and embedded digits.
/// Java: `Pattern.UNICODE_CHARACTER_CLASS`, no scoping.
static LINEAGE_LABEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{L}][\p{L}\d]*(?:-lineage|\s+lineage)$").unwrap());

// ---------- Precompiled in-method literals ----------
/// Java: no flags. No `\s`/`\d`/`\w`/`\b`; called via `.matches()` on an UNANCHORED Java
/// pattern, so the Rust port adds explicit `^…$` to reproduce full-match semantics.
static HTML_ENTITY_NAMED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:&[a-zA-Z]+;)$").unwrap());
/// Java: no flags. Has `\d`; same unanchored-pattern-called-via-`.matches()` situation as
/// above → `^…$` added, `\d` scoped (no `\p{…}` present, safe to wrap the whole thing).
static HTML_ENTITY_NUMERIC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?-u:&#\d+;)$").unwrap());
/// Java: no flags (moot — always called against an already-lowercased string). Has `\s`
/// (x3) AND wildcard `.` (in `.*`, x3) → only the `\s` atoms are individually scoped;
/// unanchored Java pattern called via `.matches()` → `^…$` added.
static DELETE_MARKER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:.*(?-u:\s))?delete(?:(?-u:\s).*|,.*|(?-u:\s*))$").unwrap());
/// Java: no compile flags, but an inline `(?i)` in the pattern text itself. Has `\s` (x2)
/// AND `\p{Lu}`/`\p{L}` AND wildcard `.` (in the trailing `.*`) → only the two `\s` atoms
/// are individually scoped; unanchored Java pattern called via `.matches()` → `^…$` added.
static NON_HOMONYM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?i)non(?-u:\s+)\p{Lu}\p{L}+(?:(?-u:\s).*)?$").unwrap());
/// Java: no flags. Has `\s` (x2) AND `\p{Ll}` → only `\s` scoped; already `^…$`-anchored in
/// Java so no extra full-match wrap is needed.
static QUESTION_ONLY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\?(?-u:\s+)\p{Ll}+(?-u:\s*)$").unwrap());
/// Java: `Pattern.UNICODE_CHARACTER_CLASS`, no scoping.
static LATIN_WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\p{L}][\p{L}.\-]+").unwrap());
/// Java: `Pattern.UNICODE_CHARACTER_CLASS`, no scoping.
static LATIN_WORD_MIN2: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\p{L}]{2,}").unwrap());
/// Java: `Pattern.UNICODE_CHARACTER_CLASS`. Has `\b`, kept Unicode (matches Java exactly,
/// since UNICODE_CHARACTER_CLASS makes Java's own `\b` Unicode-aware here too).
static AUTHOR_ABBREV: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[\p{Lu}][\p{L}]*\.").unwrap());

/// If the input matches a non-scientific category, returns `Err(ParseError)` with the
/// appropriate [`NameType`] (mirrors Java throwing `UnparsableNameException`). Otherwise
/// returns `Ok(())` silently, letting the rest of the pipeline continue.
///
/// Port of Java `Preflight.run(String original, ParseContext ctx)`. Control-flow order is
/// exactly Java's: empty → single-letter/abbrev → HTML-entity → delete-markers →
/// non-homonym → placeholder-keywords → virus gate → monomial-aggregate → lineage →
/// multi-question → question-placeholder → clade → OTU/code group → hybrid-formula.
pub fn run(original: &str, ctx: &mut ParseContext) -> Result<(), ParseError> {
    // `ctx.working` is a plain owned `String`; copy the trimmed text out so `s` doesn't
    // keep an outstanding borrow of `ctx` alive across the later `&mut ctx` needed by
    // `apply_virus_gate`.
    let s: String = java_trim(&ctx.working).to_string();
    if s.is_empty() {
        return Err(ParseError::new(NameType::Other, None, original));
    }

    // Inputs that are too short or that are just an HTML entity stub (no real name content)
    // — bail out before any regex work touches them. A single bare letter ("X" / "a") is not
    // a name. An abbreviated genus ("B.") is allowed because the dot marks it as a stand-in
    // for a longer name.
    //
    // Java re-trims the already-trimmed `s` into a local `t` (`String t = s.trim();`) —
    // trimming is idempotent, so `t` is always structurally identical to `s`; `s` is used
    // directly here instead of a redundant second trim.
    if count_letters(&s) == 1 {
        // Java indexes `t.charAt(0)`/`t.charAt(1)` (UTF-16 code units); ported as Unicode
        // scalar (`char`) indexing, the closest faithful analogue without a UTF-16
        // dependency (diverges only for astral-plane single-letter input, vanishingly rare
        // for genus abbreviations — same tradeoff already documented for `Pipeline::run`'s
        // `MAX_LENGTH` guard).
        let chars: Vec<char> = s.chars().collect();
        let is_abbrev = chars.len() == 2 && chars[1] == '.' && chars[0].is_alphabetic();
        if !is_abbrev {
            return Err(ParseError::new(NameType::Other, None, original));
        }
    }
    if HTML_ENTITY_NAMED.is_match(&s) || HTML_ENTITY_NUMERIC.is_match(&s) {
        return Err(ParseError::new(NameType::Other, None, original));
    }

    // NO_NAME markers (text deletion / discard tags). A leading "non " followed by a proper
    // Latin name is a homonym citation, not a deletion marker — leave those to the regular
    // parser path. Only reject "non" when it precedes a single short word or punctuation
    // (the typical checklist-cleanup leftover).
    //
    // Java's `s.toLowerCase()` uses the JVM default locale; Rust's `to_lowercase()` is the
    // full-Unicode default-locale-independent mapping. Every needle searched for below
    // ("tobedeleted", "(delete)", …) is pure ASCII, so the two only-`toLowerCase()`-flavour
    // divergences that exist in principle (Turkish dotless-I, etc.) cannot change whether
    // these ASCII substrings are found — inert for this call site.
    let lower = s.to_lowercase();
    if lower.contains("tobedeleted")
        || lower.contains("(delete)")
        || s.starts_with('@')
        || DELETE_MARKER.is_match(&lower)
        || lower.contains("[delete]")
        || lower.contains("[none]")
    {
        return Err(ParseError::new(NameType::Other, None, original));
    }
    if lower.starts_with("non ") && (!NON_HOMONYM.is_match(&s) || s.contains('=')) {
        return Err(ParseError::new(NameType::Other, None, original));
    }

    // Placeholder keywords first — some placeholder strings contain "virus" (e.g.
    // "uncultured virus") and the explicit keyword wins over the virus marker.
    if (PLACEHOLDER_KEYWORDS.is_match(&s)
        || NN_PLACEHOLDER.is_match(&s)
        || PLACEHOLDER_PREFIX.is_match(&s)
        || s.starts_with("[unassigned]")
        || s.eq_ignore_ascii_case("Unaccepted"))
        && !INDET_SPECIES.is_match(&s)
    {
        return Err(ParseError::new(NameType::Placeholder, None, original));
    }

    // Virus — check before the leading-question-mark placeholder so that "? circular
    // satellites" reads as a virus rather than an unstructured placeholder.
    // Clean ICTV binomials/monomials with a viral genus suffix are let through to parse;
    // legacy vernacular virus names become OTHER + NomCode::Virus.
    apply_virus_gate(&s, ctx, original)?;

    // Monomial-aggregate forms ("Iteaphila-group", "Bartonella group", "Foo-complex"): a
    // single uninomial followed by an aggregate marker is an informal taxonomic grouping
    // label that the parser model can't represent.
    if MONOMIAL_AGGREGATE.is_match(&s) {
        return Err(ParseError::new(NameType::Informal, None, original));
    }

    // "-lineage" / " lineage" labels — informal phylogenetic lineage names (any rank).
    // Checked before the OTU/code rejections below so digit/lowercase stems like
    // "NC12A-lineage" and "he2-lineage" are flagged INFORMAL rather than OTHER.
    if LINEAGE_LABEL.is_match(&s) {
        return Err(ParseError::new(NameType::Informal, None, original));
    }

    // Leading "?? …" — a run of two or more question marks is junk, not a missing-genus
    // placeholder (that is a single "?"). Reject before the single-"?" handling below.
    if MULTI_QUESTION_PREFIX.is_match(&s) {
        return Err(ParseError::new(NameType::Other, None, original));
    }

    // Leading "? <epithet>" — placeholder for missing genus. Only fully unparsable when
    // there's nothing else on the line; with authorship/year following, the missing-genus
    // form is reconstructed downstream (see StripAndStash).
    if QUESTION_PREFIX.is_match(&s) && !INDET_SPECIES.is_match(&s) && QUESTION_ONLY.is_match(&s) {
        return Err(ParseError::new(NameType::Placeholder, None, original));
    }

    // Phylogenetic clade label — not a Linnean name.
    if CLADE_KEYWORD.is_match(&s) {
        return Err(ParseError::new(NameType::Informal, None, original));
    }

    // Pure code-like NO_NAME — use the normalised (trimmed) form as the exception name.
    if OTU_BOLD.is_match(&s)
        || OTU_GTDB_UBA.is_match(&s)
        || GEN_NOV.is_match(&s)
        || s.starts_with('@')
    {
        return Err(ParseError::new(NameType::Other, None, s));
    }
    // SH identifiers are canonical in uppercase.
    if OTU_SH.is_match(&s) {
        return Err(ParseError::new(NameType::Other, None, s.to_uppercase()));
    }
    // Pure alphanumeric mash with digit (no spaces) and no obvious Latin epithet.
    if !s.contains(' ')
        && PURE_ALPHANUM.is_match(&s)
        && has_digit(&s)
        && !is_plausible_single_word_name(&s)
    {
        return Err(ParseError::new(NameType::Other, None, s));
    }
    // PR2-style underscored name with hyphenated digit suffix → NO_NAME.
    if PR2_LIKE.is_match(&s) && s.contains('-') && has_digit(&s) {
        return Err(ParseError::new(NameType::Other, None, s));
    }
    // GTDB/SILVA specimen codes ending with "sp" + 8+ digits (e.g. "18JY21-1 sp004344915").
    if s.contains(' ') && OTU_SPECIMEN_SUFFIX.is_match(&s) {
        return Err(ParseError::new(NameType::Other, None, s));
    }
    // Multi-word input whose last token is a known OTU code (e.g. "Festuca sp. BOLD:ACW2100").
    if s.contains(' ') {
        let last = last_word(&s);
        if OTU_BOLD.is_match(last) {
            return Err(ParseError::new(NameType::Other, None, last));
        }
        if OTU_SH.is_match(last) {
            return Err(ParseError::new(NameType::Other, None, last.to_uppercase()));
        }
    }

    // Hybrid formula — only when the cross sits between two distinct name spans.
    if looks_like_hybrid_formula(&s) {
        return Err(ParseError::new(NameType::Formula, None, original));
    }

    Ok(())
}

/// Port of Java `private static void applyVirusGate(String s, ParseContext ctx, String
/// original)`. Bucket A: a clean uni/binomial whose genus/monomial carries a viral rank
/// suffix parses, recording `ctx.viral_shape`. Bucket B: everything else that trips the
/// VIRUS marker is either rescued (zoological author-year override, soft-genus, caller
/// asserted a non-virus code, soft/hard virus-word rescues) or rejected as
/// `OTHER + NomCode::Virus`.
fn apply_virus_gate(s: &str, ctx: &mut ParseContext, original: &str) -> Result<(), ParseError> {
    let clean = CLEAN_BINOMIAL.is_match(s) || CLEAN_MONOMIAL.is_match(s);
    let req = ctx.requested_code;

    // Bucket A: clean uni/binomial whose genus/monomial carries a viral rank suffix.
    if clean && is_viral(first_word(s)) {
        if req.is_none() || req == Some(NomCode::Virus) {
            ctx.viral_shape = true;
        }
        return Ok(()); // parse; a non-virus caller code is kept and wins over inference
    }
    if !VIRUS.is_match(s) {
        return Ok(()); // no viral trigger at all
    }
    // Inline "Genus [(Subgenus)] epithet Author, YYYY" overrides a stray viral token.
    if ZOOLOGICAL_BINOMIAL.is_match(s) {
        return Ok(());
    }
    if req == Some(NomCode::Virus) {
        if clean {
            ctx.viral_shape = true;
            return Ok(());
        }
        return Err(ParseError::new(
            NameType::Other,
            Some(NomCode::Virus),
            original,
        ));
    }
    // Soft virus-word as the leading GENUS with no genuinely viral token present → a real
    // animal genus (e.g. "Prion vittatus", "Prion Lacépède, 1799"), not a virus. Covers both
    // the clean binomial and the authored-monomial forms, which the last-word SOFT_WORD
    // rescue below misses.
    if SOFT_GENUS.is_match(s) && !HARD_VIRUS.is_match(s) {
        return Ok(());
    }
    if clean && req.is_some() {
        return Ok(()); // caller asserts a non-virus code for a clean binomial
    }
    if clean && SOFT_WORD.is_match(last_word(s)) {
        return Ok(()); // bucket B soft
    }
    if clean && HARD_WORD.is_match(last_word(s)) {
        if let Some(auth) = ctx.authorship_input.as_deref() {
            if AUTH_ZOO.is_match(java_trim(auth)) {
                return Ok(()); // bucket B hard, rescued by a separately supplied zoological author+year
            }
        }
    }
    Err(ParseError::new(
        NameType::Other,
        Some(NomCode::Virus),
        original,
    ))
}

fn first_word(s: &str) -> &str {
    match s.find(' ') {
        Some(idx) => &s[..idx],
        None => s,
    }
}

fn last_word(s: &str) -> &str {
    match s.rfind(' ') {
        Some(idx) => &s[idx + 1..],
        None => s,
    }
}

/// Port of Java `private static boolean looksLikeHybridFormula(String s)`. A hybrid formula
/// has the cross between two NAME spans where the left side contains a *binomial* (Genus +
/// epithet) or an authored monomial. A single "× epithet" (notho marker) on either side does
/// NOT count.
///
/// Java indexes UTF-16 code units (`s.charAt(i)`, `s.substring(..)`); ported here over
/// `char_indices()` (byte offset, Unicode scalar value) pairs — `i` becomes a char-position
/// index into that Vec, and the paired byte offsets are used for the actual `&str` slicing,
/// so multi-byte UTF-8 sequences are never bisected (Rust `char`s need no surrogate-pair
/// handling at all, unlike Java `char`).
fn looks_like_hybrid_formula(s: &str) -> bool {
    let cs: Vec<(usize, char)> = s.char_indices().collect();
    let n = cs.len();
    for i in 0..n {
        let c = cs[i].1;
        let cross = c == '×';
        let ascii_x = !cross
            && (c == 'x' || c == 'X')
            && i > 0
            && i + 1 < n
            && cs[i - 1].1 == ' '
            && cs[i + 1].1 == ' ';
        let plus = !cross
            && !ascii_x
            && c == '+'
            && i > 0
            && i + 1 < n
            && cs[i - 1].1 == ' '
            && cs[i + 1].1 == ' ';
        if !cross && !ascii_x && !plus {
            continue;
        }
        if i == 0 || cs[i - 1].1 != ' ' {
            continue;
        }
        // Right of cross must be whitespace-separated; otherwise ×x is a notho marker
        // glued to an epithet (e.g. "var. ×alpina").
        if i + 1 >= n || cs[i + 1].1 != ' ' {
            continue;
        }
        let left = java_trim(&s[..cs[i].0]);
        let right = java_trim(&s[cs[i + 1].0..]);
        // Accept "?" as a valid right side (the second taxon is unspecified).
        let right_ok = contains_latin_word(right) || right.starts_with('?');
        if !right_ok {
            continue;
        }
        if count_latin_words(left) >= 2 || has_author_abbrev(left) {
            return true;
        }
        // Graft-chimera formula: single genus on each side (e.g. "Crataegus + Mespilus").
        // Java re-trims `right.trim()` a second time here even though `right` is already
        // trimmed above; idempotent, so `right` is used directly.
        if plus
            && count_latin_words(left) == 1
            && !left.is_empty()
            && left.chars().next().is_some_and(|c| c.is_uppercase())
            && !right.is_empty()
            && right.chars().next().is_some_and(|c| c.is_uppercase())
        {
            return true;
        }
    }
    false
}

fn count_latin_words(s: &str) -> usize {
    LATIN_WORD.find_iter(s).count()
}

fn contains_latin_word(s: &str) -> bool {
    LATIN_WORD_MIN2.is_match(s)
}

fn has_author_abbrev(s: &str) -> bool {
    AUTHOR_ABBREV.is_match(s)
}

/// java.lang.Character.isLetter — see `token.rs`'s `is_letter` for the same approximation
/// (`is_alphabetic`, slightly broader than Java's L*-only category) used throughout this
/// port; kept as a separate private fn here since `token.rs`'s is module-private.
fn count_letters(s: &str) -> usize {
    s.chars().filter(|c| c.is_alphabetic()).count()
}

/// java.lang.Character.isDigit — see `token.rs`'s `is_digit` for the same ASCII-only
/// approximation used throughout this port (non-ASCII decimal digits are rare in names).
fn has_digit(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_digit())
}

/// Conservative check: a single-token name with at least one digit is plausibly NOT
/// scientific.
fn is_plausible_single_word_name(s: &str) -> bool {
    !has_digit(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a fresh `ParseContext` the way `Pipeline::run` would just before calling
    /// `Preflight.run`, and calls it — per the brief, unit-testing Preflight directly
    /// rather than through the (still-skeletal) full pipeline.
    fn check(input: &str) -> Result<(), ParseError> {
        let mut ctx = ParseContext::new(input.to_string(), None, None, None);
        run(input, &mut ctx)
    }

    // ---------- category: virus ----------

    #[test]
    fn virus_vernacular_name_is_rejected_other_virus() {
        let err = check("Tobacco mosaic virus").unwrap_err();
        assert_eq!(err.type_, NameType::Other);
        assert_eq!(err.code, Some(NomCode::Virus));
    }

    #[test]
    fn soft_genus_prion_is_rescued_and_parses() {
        // Prion (petrels) is a real animal genus; the soft word "prion" alone (no HARD_VIRUS
        // token also present) must not trip the virus gate. Java `virusFalsePositiveAnimals()`.
        assert!(check("Prion vittatus").is_ok());
    }

    #[test]
    fn zoological_binomial_with_author_year_overrides_stray_viral_token() {
        // Documented motivating example for ZOOLOGICAL_BINOMIAL (Java commit db606be): a real
        // species whose epithet is a VIRUS-triggering word ("vector") is rescued by an inline
        // "Genus species Author, YYYY" citation.
        assert!(check("Ceylonesmus vector Chamberlin, 1941").is_ok());
    }

    #[test]
    fn clean_ictv_binomial_parses_with_viral_shape_flag_set() {
        // Bucket A: a clean binomial whose genus carries an ICTV viral suffix parses, and
        // Preflight records viral_shape for a later Assemble stage to turn into NomCode::Virus.
        let mut ctx = ParseContext::new("Lausannevirus francensis".to_string(), None, None, None);
        assert!(run("Lausannevirus francensis", &mut ctx).is_ok());
        assert!(ctx.viral_shape);
    }

    // ---------- category: hybrid formula ----------

    #[test]
    fn cross_between_two_binomials_is_a_hybrid_formula() {
        let err = check("Homo sapiens x Homo neanderthalensis").unwrap_err();
        assert_eq!(err.type_, NameType::Formula);
        assert_eq!(err.code, None);
    }

    #[test]
    fn graft_chimera_plus_formula_is_rejected() {
        let err = check("Crataegus + Mespilus").unwrap_err();
        assert_eq!(err.type_, NameType::Formula);
    }

    // ---------- category: OTU / specimen code ----------

    #[test]
    fn bold_code_is_rejected_other() {
        let err = check("BOLD:ACW2100").unwrap_err();
        assert_eq!(err.type_, NameType::Other);
        assert_eq!(err.code, None);
    }

    #[test]
    fn sh_code_is_rejected_other_with_uppercased_name() {
        let err = check("sh460441.07fu").unwrap_err();
        assert_eq!(err.type_, NameType::Other);
        assert_eq!(err.name, "SH460441.07FU");
    }

    #[test]
    fn trailing_bold_code_in_multiword_input_is_rejected_with_last_word_as_name() {
        let err = check("Festuca sp. BOLD:ACW2100").unwrap_err();
        assert_eq!(err.type_, NameType::Other);
        assert_eq!(err.name, "BOLD:ACW2100");
    }

    // ---------- category: informal ----------

    #[test]
    fn monomial_group_aggregate_is_informal() {
        let err = check("Iteaphila-group").unwrap_err();
        assert_eq!(err.type_, NameType::Informal);
    }

    #[test]
    fn lineage_label_is_informal() {
        let err = check("Vermistella-lineage").unwrap_err();
        assert_eq!(err.type_, NameType::Informal);
    }

    #[test]
    fn clade_keyword_is_informal() {
        let err = check("Amauropeltoid clade").unwrap_err();
        assert_eq!(err.type_, NameType::Informal);
    }

    // ---------- category: other (junk / non-names) ----------

    #[test]
    fn double_question_mark_prefix_is_other_not_informal() {
        let err = check("?? not a name").unwrap_err();
        assert_eq!(err.type_, NameType::Other);
    }

    #[test]
    fn bare_gen_nov_is_other() {
        let err = check("Gen.nov.").unwrap_err();
        assert_eq!(err.type_, NameType::Other);
    }

    #[test]
    fn delete_marker_is_other() {
        let err = check("Scelotes tobedeleted , 1999").unwrap_err();
        assert_eq!(err.type_, NameType::Other);
    }

    #[test]
    fn html_named_entity_only_is_other() {
        let err = check("&amp;").unwrap_err();
        assert_eq!(err.type_, NameType::Other);
    }

    #[test]
    fn single_bare_letter_is_other() {
        let err = check("X").unwrap_err();
        assert_eq!(err.type_, NameType::Other);
    }

    #[test]
    fn single_letter_abbreviation_with_dot_is_not_rejected_by_the_letter_count_guard() {
        // "B." is an abbreviated genus stand-in, not a bare letter -- Preflight's own
        // letter-count guard must let it through (downstream stages, not yet ported in this
        // slice, are responsible for actually expanding/handling it).
        assert!(check("B.").is_ok());
    }

    #[test]
    fn empty_input_is_other() {
        let err = check("").unwrap_err();
        assert_eq!(err.type_, NameType::Other);
    }

    // ---------- category: placeholder ----------

    #[test]
    fn incertae_sedis_is_placeholder() {
        let err = check("incertae sedis").unwrap_err();
        assert_eq!(err.type_, NameType::Placeholder);
    }

    #[test]
    fn nn_placeholder_is_placeholder() {
        let err = check("N.N.").unwrap_err();
        assert_eq!(err.type_, NameType::Placeholder);
    }

    #[test]
    fn single_question_mark_alone_is_placeholder() {
        let err = check("? unclassed").unwrap_err();
        assert_eq!(err.type_, NameType::Placeholder);
    }

    // ---------- category: scientific (Ok) ----------

    #[test]
    fn clean_binomial_parses() {
        assert!(check("Abies alba").is_ok());
    }

    // ---------- additional edge cases surfaced during the port ----------

    #[test]
    fn hard_virus_word_is_rescued_by_separately_supplied_zoological_authorship() {
        // Bucket B hard rescue: "vector" (a HARD_WORD, since "clean" here refers to the
        // scientificName alone) has no inline author+year, but a separately supplied
        // authorship string that itself looks like a zoological author+year citation
        // rescues it. Mirrors Java's `virusFalsePositiveAnimals` two-argument `assertName`
        // calls (e.g. "Aspilota vector" / "Belokobylskij, 2007").
        let mut ctx = ParseContext::new(
            "Aspilota vector".to_string(),
            Some("Belokobylskij, 2007".to_string()),
            None,
            None,
        );
        assert!(run("Aspilota vector", &mut ctx).is_ok());
    }

    #[test]
    fn gca_code_is_rejected_other() {
        // Java test data only exercises the "UBA" alternative of OTU_GTDB_UBA directly;
        // this covers the untested "GCA" alternative in the same alternation.
        let err = check("GCA_000123").unwrap_err();
        assert_eq!(err.type_, NameType::Other);
    }

    #[test]
    fn non_prefix_followed_by_a_proper_latin_name_is_not_rejected() {
        // A leading "non " followed by what looks like a genuine homonym-citation author
        // name (NON_HOMONYM matches, no "=") must NOT be treated as a deletion marker.
        assert!(check("non Linnaeus").is_ok());
    }

    #[test]
    fn non_prefix_with_equals_sign_is_still_rejected() {
        // Even when NON_HOMONYM would otherwise match, an embedded "=" (a synonymy marker
        // in the typical checklist-cleanup leftover) still forces rejection.
        let err = check("non Linnaeus = Smith").unwrap_err();
        assert_eq!(err.type_, NameType::Other);
    }

    #[test]
    fn question_prefixed_epithet_with_trailing_authorship_is_not_preflight_rejected() {
        // QUESTION_ONLY only fires when there's nothing else on the line; with an author
        // citation following, the missing-genus placeholder is reconstructed downstream
        // (not yet ported in this slice), so Preflight itself must let it through.
        assert!(check("? gryphoidis Bourguignat 1870").is_ok());
    }

    #[test]
    fn virus_gate_classifies_adversarial_input_without_hanging() {
        // Java's `virusGateNoCatastrophicBacktracking` guards ZOOLOGICAL_BINOMIAL's
        // possessive-quantifier ReDoS hardening against a backtracking engine. The `regex`
        // crate has no backtracking at all (linear-time automaton), so this can't
        // pathologically hang here the way an unguarded Java `Matcher.find()` could — this
        // test documents that guarantee rather than defending against real risk.
        let adversarial = format!("Rnavirus bus {}", "Aa.-".repeat(30));
        let start = std::time::Instant::now();
        let _ = check(&adversarial);
        assert!(start.elapsed() < std::time::Duration::from_secs(3));
    }
}
