// SPDX-License-Identifier: Apache-2.0
//! Ported StripAndStash patterns. The 169 possessive quantifiers in the Java sources are
//! dropped: the `regex` crate is a linear-time automaton, so possessive/greedy is moot.

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
}
