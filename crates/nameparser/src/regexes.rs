// SPDX-License-Identifier: Apache-2.0
//! Ported StripAndStash patterns. The 169 possessive quantifiers in the Java sources are
//! dropped: the `regex` crate is a linear-time automaton, so possessive/greedy is moot.

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
// boundary characters are preserved instead of consumed.
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
static CORRIG_BARE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\s)corrig\.?(\s|$)").unwrap());

pub fn strip_corrig_restructured(s: &str) -> String {
    let s = CORRIG_BRACKETED.replace_all(s, "");
    // ${1}${2} (braced) so the two group refs can't be misparsed as one name.
    CORRIG_BARE.replace_all(&s, "${1}${2}").into_owned()
}

// --- CORRIG verbatim via fancy-regex (lookaround supported; backtracking) ---
// Input is length-bounded by the pipeline's MAX_LENGTH cap, so the backtracking engine is
// not a ReDoS risk here. This is the escape-hatch strategy for irreducible patterns.
static CORRIG_FANCY: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(r"\s*[(\[]\s*corrig\.?\s*[)\]]|(?<=\s)corrig\.?(?=\s|$)").unwrap()
});

pub fn strip_corrig_fancy(s: &str) -> String {
    // fancy-regex does NOT mirror the regex crate's `replace_all`: its match methods return
    // Result (backtracking can hit a limit). This manual splice — the fancy-regex equivalent of
    // `replace_all(s, "")` — is itself an ergonomics finding for the doc. An Err or end-of-matches
    // terminates the loop.
    let mut result = String::with_capacity(s.len());
    let mut last = 0usize;
    for m in CORRIG_FANCY.find_iter(s) {
        let m = match m {
            Ok(m) => m,
            Err(_) => break,
        };
        result.push_str(&s[last..m.start()]);
        last = m.end();
    }
    result.push_str(&s[last..]);
    result
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
    fn corrig_bare_requires_a_real_preceding_space() {
        // Beyond the brief's 3 prescribed tests: a self-review probe found that Java's two
        // CORRIG boundaries are asymmetric. `(?<=\s)` demands an actual preceding whitespace
        // *character* — at absolute start-of-input there is none, so Java leaves a leading
        // "corrig. Rest" untouched, even though `(?=\s|$)` on the right explicitly treats
        // end-of-input as a valid boundary. Confirmed against the verbatim fancy-regex pattern
        // (`is_match("corrig. Aus bus")` == false) before fixing CORRIG_BARE to match: the
        // faithful restructuring is `(\s)corrig\.?(\s|$)`, NOT the tempting-but-wrong
        // `(^|\s)corrig\.?(\s|$)` (which would over-eagerly strip this case).
        assert_eq!(
            strip_corrig_restructured("corrig. Aus bus"),
            "corrig. Aus bus"
        );
        assert_eq!(strip_corrig_fancy("corrig. Aus bus"), "corrig. Aus bus");
        // A standalone "corrig." with nothing around it at all is untouched for the same reason.
        assert_eq!(strip_corrig_restructured("corrig."), "corrig.");
        assert_eq!(strip_corrig_fancy("corrig."), "corrig.");
    }
}
