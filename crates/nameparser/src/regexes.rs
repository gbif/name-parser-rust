// SPDX-License-Identifier: Apache-2.0
//! Ported StripAndStash patterns. The Java sources carry ~20 possessive quantifiers (14 in
//! Preflight, 6 in StripAndStash); they're dropped here because the `regex` crate is a
//! linear-time automaton, so possessive/greedy is moot.

use fancy_regex::Regex as FancyRegex;
use regex::Regex;
use std::sync::LazyLock;

/// Java SIC (StripAndStash.java:29-30): "[sic]" / "(sic)" / "[sic!]" with no inner comma.
pub static SIC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*[(\[]\s*sic\s*!?\s*[)\]]").unwrap());

/// Java AGGREGATE (StripAndStash.java:138-141): trailing " agg." / "species group" / "-group" …
pub static AGGREGATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:\s+(?:agg\.?|aggregate|species\s+group|species\s+complex|group|complex)|\s*-\s*group|\s*-\s*aggregate)\s*$",
    )
    .unwrap()
});

/// Java IN_PRESS (StripAndStash.java:144-145): trailing " in press".
pub static IN_PRESS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\s+in\s+press\b\.?").unwrap());

/// Java PUBLISHED_PAGE (StripAndStash.java:157-158): trailing " : 377" / ": 12-18".
pub static PUBLISHED_PAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*:\s*(\d+(?:[-\x{2013}]\d+)?)\s*$").unwrap());

/// Java TAX_NOTE (StripAndStash.java:68-84): trailing taxonomic-concept note.
/// Ported verbatim; note the inner `(?-i:…)` case-sensitive group for the s.l./s.str. markers,
/// which the `regex` crate supports directly.
pub static TAX_NOTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\s+,?\s*(auctt?\b\.?(?:[,.]?\s.*)?|sensu(?:\s.*)?|sec\.?(?:\s.*)?|nec\b(?:\s.*)?|nonn?\.?\s+\(?\p{Lu}.*|emend\b\.?\s+\(?\p{Lu}.*|fide\b\.?\s+\(?\p{Lu}.*|according\s+to\s+\p{Lu}.*|excl\.\s+.*|ss\b\.?\s+.*|(?-i:s\.\s*l\.?|s\.\s*str\.?|s\.\s*lat\.?|s\.\s*ampl\.?))$",
    )
    .unwrap()
});

// Java CORRIG (StripAndStash.java:35-36): `\s*[\(\[]\s*corrig\.?\s*[\)\]]|(?<=\s)corrig\.?(?=\s|$)`.
//
// Verdict (feeds Task 7): prefer restructuring onto `regex` where the lookaround can be
// re-expressed as captured, re-inserted boundaries — it stays dependency-free and linear-time,
// at the cost of having to reason once, per pattern, about whether consuming the boundary
// chars is observable downstream — and that reasoning is genuinely fiddly: see the `^`
// discussion below, a subtle-asymmetry trap the naive translation walks straight into. Reserve
// `fancy-regex` for patterns that resist restructuring (e.g. lookaround interacting with
// backreferences, or boundaries too entangled to capture cleanly) — it is a faithful drop-in
// but adds a backtracking engine to the dependency graph.

// --- CORRIG, restructured onto the linear `regex` engine (no lookaround) ---
// The bracketed alternative needs no lookaround. The bare alternative replaces Java's
// zero-width (?<=\s)…(?=\s|$) by CAPTURING the boundaries and putting them back, so the
// boundary characters are preserved instead of consumed (for isolated matches; see adjacency caveat below).
//
// GOTCHA (found via a manual probe beyond the brief's 3 tests, not by the tests below): Java's
// two boundaries are NOT symmetric, and the tempting `(^|\s)…(\s|$)` translation silently breaks
// that asymmetry. `(?<=\s)` demands an *actual preceding character* that is whitespace — at
// absolute start-of-input there is no preceding character at all, so the lookbehind fails and
// Java leaves a leading "corrig. Rest" untouched. `(?=\s|$)` has no such trap because `$` is
// spelled out explicitly as an alternative. Writing the left side as `(^|\s)` "for symmetry"
// makes start-of-input an accepted boundary, which strips a case Java does not strip — confirmed
// by running the verbatim fancy-regex pattern against "corrig. Aus bus" (`is_match` == false).
// The faithful restructuring must therefore drop the `^` alternative and require a captured
// `\s`, full stop:
static CORRIG_BRACKETED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*[(\[]\s*corrig\.?\s*[)\]]").unwrap());

// KNOWN LIMITATION of this capture-and-splice restructuring: the boundaries are CONSUMED,
// not zero-width like Java's lookaround. Because regex::replace_all matches are non-overlapping,
// two ADJACENT bare "corrig." tokens sharing one whitespace boundary diverge from Java
// (e.g. "a b corrig. corrig. c": Java strips both; this strips only the first).
// strip_corrig_fancy (verbatim lookaround) stays faithful. Realistic name data has no adjacent
// markers, but this is why lookaround patterns with possible adjacency should prefer fancy-regex.
// SECOND, DISTINCT adjacency mechanism: a bare "corrig." immediately abutting a bracketed one
// (e.g. "x corrig.(corrig.) y") over-strips in this restructured variant too — the bracketed
// pass runs first and its removal manufactures a fresh whitespace boundary that lets the bare
// pass then match a "corrig." that never had a real boundary in Java's single-pass original;
// strip_corrig_fancy is unaffected, which is why CORRIG itself should use the fancy variant in
// Phase 1.
static CORRIG_BARE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\s)corrig\.?(\s|$)").unwrap());

/// Collapse whitespace runs to a single space and trim — mirrors Java's
/// `WHITESPACE.matcher(...).replaceAll(" ").trim()` applied around CORRIG.
fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn strip_corrig_restructured(s: &str) -> String {
    // Java applies CORRIG with a LEADING-SPACE PAD (StripAndStash.java:432,1117:
    // `CORRIG.matcher(" " + s)`) + whitespace-collapse + trim, so a standalone/leading
    // "corrig." IS stripped. We replicate that harness. The pad is exactly why the left
    // boundary is (\s) and not ^: the pad guarantees a real preceding whitespace char,
    // matching Java's (?<=\s). Lesson: lookaround faithfulness depends on the CALL-SITE
    // harness, not just the pattern — see findings doc §2 and §5.2.
    let padded = format!(" {s}");
    let no_bracket = CORRIG_BRACKETED.replace_all(&padded, "");
    let no_bare = CORRIG_BARE.replace_all(&no_bracket, "${1}${2}");
    collapse_ws(&no_bare)
}

// --- CORRIG verbatim via fancy-regex (lookaround supported; backtracking) ---
// Input is length-bounded by the pipeline's MAX_LENGTH cap, so the backtracking engine is
// not a ReDoS risk here. This is the escape-hatch strategy for irreducible patterns.
static CORRIG_FANCY: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(r"\s*[(\[]\s*corrig\.?\s*[)\]]|(?<=\s)corrig\.?(?=\s|$)").unwrap()
});

pub fn strip_corrig_fancy(s: &str) -> String {
    // Same leading-space-pad + collapse harness as the restructured variant.
    let padded = format!(" {s}");
    let mut result = String::with_capacity(padded.len());
    let mut last = 0usize;
    for m in CORRIG_FANCY.find_iter(&padded) {
        let m = match m {
            Ok(m) => m,
            Err(_) => break,
        };
        result.push_str(&padded[last..m.start()]);
        last = m.end();
    }
    result.push_str(&padded[last..]);
    collapse_ws(&result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sic_removed() {
        assert_eq!(
            SIC.replace_all("Ameiva plei (sic) Bibron", "").as_ref(),
            "Ameiva plei Bibron"
        );
    }

    #[test]
    fn aggregate_trimmed() {
        assert_eq!(
            AGGREGATE
                .replace_all("Achillea millefolium agg.", "")
                .as_ref(),
            "Achillea millefolium"
        );
    }

    #[test]
    fn in_press_removed() {
        assert_eq!(
            IN_PRESS.replace_all("Aus bus Smith in press", "").as_ref(),
            "Aus bus Smith"
        );
    }

    #[test]
    fn published_page_captured() {
        let caps = PUBLISHED_PAGE.captures("Aus bus Smith : 377").unwrap();
        assert_eq!(&caps[1], "377");
    }

    #[test]
    fn tax_note_stripped() {
        assert_eq!(
            TAX_NOTE.replace_all("Aus bus sensu Smith", "").as_ref(),
            "Aus bus"
        );
        // case-sensitive s.l. marker still matches lower-case
        assert_eq!(TAX_NOTE.replace_all("Aus bus s.l.", "").as_ref(), "Aus bus");
    }

    #[test]
    fn corrig_bracketed() {
        assert_eq!(
            strip_corrig_restructured("Aus bus (corrig.) Smith"),
            "Aus bus Smith"
        );
        assert_eq!(
            strip_corrig_fancy("Aus bus (corrig.) Smith"),
            "Aus bus Smith"
        );
    }

    #[test]
    fn corrig_bare_word_between_spaces() {
        // The bare form removes just "corrig." and leaves the surrounding spaces (Java behaviour);
        // downstream whitespace normalisation collapses them. We assert the collapsed form.
        let collapse = |s: String| s.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            collapse(strip_corrig_restructured("Aus bus corrig. Smith")),
            "Aus bus Smith"
        );
        assert_eq!(
            collapse(strip_corrig_fancy("Aus bus corrig. Smith")),
            "Aus bus Smith"
        );
    }

    #[test]
    fn corrig_not_matched_mid_word() {
        // "corrigenda" must NOT be touched (word-boundary behaviour of the bare form).
        assert_eq!(
            strip_corrig_restructured("Aus corrigenda Smith"),
            "Aus corrigenda Smith"
        );
        assert_eq!(
            strip_corrig_fancy("Aus corrigenda Smith"),
            "Aus corrigenda Smith"
        );
    }

    #[test]
    fn case_insensitive_flag_is_active() {
        // (?i) patterns must match UPPER-cased keywords, matching Java's Pattern.CASE_INSENSITIVE.
        assert_eq!(
            AGGREGATE
                .replace_all("Achillea millefolium AGG.", "")
                .as_ref(),
            "Achillea millefolium"
        );
        assert_eq!(
            TAX_NOTE.replace_all("Aus bus SENSU Smith", "").as_ref(),
            "Aus bus"
        );
    }

    #[test]
    fn tax_note_sl_marker_is_case_sensitive() {
        // The (?-i:…) carve-out means UPPER-case author initials "S.L." are NOT the sensu-lato
        // marker (StripAndStash.java:79-81), while lower-case "s.l." IS. This is the whole point
        // of the scoped case-sensitivity toggle, so it must be tested directly.
        assert!(
            !TAX_NOTE.is_match("Aus bus Mill. S.L."),
            "uppercase S.L. must not match the sensu-lato marker"
        );
        assert!(
            TAX_NOTE.is_match("Aus bus s.l."),
            "lower-case s.l. must match the marker"
        );
    }

    #[test]
    fn corrig_strips_leading_and_standalone_marker_like_java() {
        // Java pads with a leading space (StripAndStash.java:432,1117) so a leading or
        // standalone "corrig." IS stripped. Both variants replicate that.
        assert_eq!(
            strip_corrig_restructured("corrig. Peters, 1878"),
            "Peters, 1878"
        );
        assert_eq!(strip_corrig_fancy("corrig. Peters, 1878"), "Peters, 1878");
        assert_eq!(strip_corrig_restructured("corrig."), "");
        assert_eq!(strip_corrig_fancy("corrig."), "");
    }
}
