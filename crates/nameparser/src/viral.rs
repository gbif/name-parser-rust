// SPDX-License-Identifier: Apache-2.0
//! Java `org.gbif.nameparser.pipeline.ViralSuffix` — recognises the standardized ICTV
//! viral rank suffixes on a single word (genus, monomial, or higher-taxon name). Per
//! MSL41 every virus genus ends in one of these, so the suffix alone is a reliable virus
//! signal.
//!
//! Only the **singular** canonical suffixes are matched, so Linnaean look-alikes such as
//! the mollusk genus `Crassatellites` (`-satellites`) are not misread as viral.

use regex::Regex;
use std::sync::LazyLock;

/// Java `GENUS` (ViralSuffix.java:17-18): genus-rank viral suffixes. Java compiles this
/// with only `Pattern.CASE_INSENSITIVE` — no `UNICODE_CHARACTER_CLASS`/`UNICODE_CASE`, and
/// the pattern itself has no `\s`/`\d`/`\w`/`\b` shorthand classes — so the per-pattern
/// flag rule has nothing to ASCII-scope here. `CASE_INSENSITIVE` becomes the inline `(?i)`
/// flag, matching this port's existing convention (see `regexes.rs`); every alternative is
/// a pure-ASCII literal, so Rust's Unicode-aware `(?i)` and Java's ASCII-only
/// `CASE_INSENSITIVE` behave identically here.
static GENUS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:virus|viroid|satellite|viriform)$").unwrap());

/// Java `HIGHER` (ViralSuffix.java:24-30): higher-taxon viral suffixes (realm, kingdom,
/// phylum, subphylum, class, subclass, order, suborder, family, subfamily ranks). Java
/// spells this as five `+`-concatenated string literals purely for source line-wrapping;
/// concatenated they form one alternation, reproduced verbatim (same 15 suffixes, same
/// order) as a single literal here — same `(?i)`-only reasoning as `GENUS` above. Longer
/// suffixes (7+ chars) are unambiguous; the short realm suffix `-viria` and kingdom suffix
/// `-virae` are included unguarded because ICTV MSL41 has zero subrealm taxa (the
/// formerly-guarded `-vira` is omitted entirely, avoiding false positives such as the
/// hummingbird genus `Elvira` or the word `Mahavira`).
static HIGHER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:viridae|viroidae|satellitidae|virinae|viroinae|satellitinae|virales|virineae|viricetes|viricetidae|viricotina|viricota|virites|viria|virae)$",
    )
    .unwrap()
});

/// Java `ViralSuffix.isViral(String word)` (ViralSuffix.java:32-36): `true` if `word` ends
/// in a standardized ICTV viral rank suffix — genus rank (`GENUS`) or a higher rank
/// (`HIGHER`). Java's `.find()` is an unanchored search (a match may start anywhere in the
/// input, not just at position 0); Rust's `Regex::is_match` is likewise unanchored, so both
/// patterns' trailing `$` alone makes this a suffix check with no boundary requirement
/// before it — faithfully preserved, not "improved" with an added `\b`.
///
/// Java's `if (word == null) return false;` guard has no Rust analogue: `&str` cannot be
/// null. A caller modelling an absent word should short-circuit on `Option<&str>` before
/// calling this function. `ViralSuffix.java` declares no other methods, so nothing else
/// was skipped.
pub fn is_viral(word: &str) -> bool {
    GENUS.is_match(word) || HIGHER.is_match(word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_genus_example_from_java_doc_is_viral() {
        // The exact example named in this port's brief, and a genuine ICTV virus genus.
        assert!(is_viral("Lausannevirus"));
    }

    #[test]
    fn plain_genus_is_not_viral() {
        assert!(!is_viral("Abies"));
    }

    #[test]
    fn all_genus_suffixes_match() {
        for suffix in ["virus", "viroid", "satellite", "viriform"] {
            let word = format!("Xxx{suffix}");
            assert!(is_viral(&word), "expected {word:?} to be viral (GENUS)");
        }
    }

    #[test]
    fn all_higher_suffixes_match() {
        for suffix in [
            "viridae",
            "viroidae",
            "satellitidae",
            "virinae",
            "viroinae",
            "satellitinae",
            "virales",
            "virineae",
            "viricetes",
            "viricetidae",
            "viricotina",
            "viricota",
            "virites",
            "viria",
            "virae",
        ] {
            let word = format!("Xxx{suffix}");
            assert!(is_viral(&word), "expected {word:?} to be viral (HIGHER)");
        }
    }

    #[test]
    fn known_ictv_family_and_order_names_are_viral() {
        assert!(is_viral("Poxviridae")); // family
        assert!(is_viral("Mononegavirales")); // order
    }

    #[test]
    fn singular_satellite_guard_avoids_mollusc_false_positive() {
        // Java doc: "Crassatellites" (mollusk genus, plural "-satellites") must NOT match;
        // only the singular "-satellite" suffix is viral.
        assert!(!is_viral("Crassatellites"));
    }

    #[test]
    fn dropped_vira_suffix_avoids_documented_false_positives() {
        // Java doc: the formerly-guarded "-vira" suffix was dropped entirely (not merely
        // guarded) specifically to avoid these two false positives.
        assert!(!is_viral("Elvira"));
        assert!(!is_viral("Mahavira"));
    }

    #[test]
    fn match_is_case_insensitive() {
        assert!(is_viral("LAUSANNEVIRUS"));
        assert!(is_viral("lausannevirus"));
        assert!(is_viral("POXVIRIDAE"));
    }

    #[test]
    fn suffix_need_not_be_preceded_by_a_word_boundary() {
        // Java's patterns have no `\b` before the suffix, just a literal-text `$` anchor:
        // any word ending in the suffix qualifies, boundary or not. Must not "improve" on
        // this by adding \b during the port.
        assert!(is_viral("xvirus"));
    }

    #[test]
    fn suffix_must_be_at_the_end_not_merely_present() {
        // "Virusia" contains "virus" but does not END in it - the `$` anchor must hold.
        assert!(!is_viral("Virusia"));
    }

    #[test]
    fn empty_word_is_not_viral() {
        assert!(!is_viral(""));
    }
}
