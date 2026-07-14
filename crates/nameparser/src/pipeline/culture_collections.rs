// SPDX-License-Identifier: Apache-2.0

//! Curated culture-collection acronyms (ATCC, DSM, CBS, …), loaded once from the embedded
//! `resources/culture-collections.txt` (blank lines and `#`-comment lines skipped). Shared by two
//! call sites that recognise a culture-collection accession `<ACRONYM> <body>`:
//!  - [`crate::pipeline::preflight`] — a *standalone* accession is a [`crate::model::NameType`]
//!    `Identifier` rather than the catch-all `Other`;
//!  - `stash_trailing_strain_code` in [`crate::pipeline::stripandstash`] — an accession *trailing a
//!    determined name* is stashed as the `phrase` instead of being misread as an author.
//!
//! Membership is **case-sensitive** (the acronyms are stored uppercase, as culture codes are
//! conventionally written): matching case-insensitively would sweep up lowercase words. This is a
//! deliberately conservative seed list — see
//! `docs/superpowers/specs/2026-07-14-nametype-identifier-design.md` for the growth policy.

use std::collections::HashSet;
use std::sync::LazyLock;

const CULTURE_COLLECTIONS_TXT: &str = include_str!("../../resources/culture-collections.txt");

/// The acronyms, uppercase (exactly as stored in the resource file).
static ACRONYMS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    CULTURE_COLLECTIONS_TXT
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
});

/// A regex alternation of the acronyms, **longest first** so a longer acronym (`DSMZ`) is not
/// pre-empted by a shorter prefix of it (`DSM`) — already grouped and anchorless, ready to splice
/// into a larger pattern: `(?:DSMZ|NCIMB|…|SAG)`.
pub(crate) fn acronym_alternation() -> &'static str {
    static ALT: LazyLock<String> = LazyLock::new(|| {
        let mut v: Vec<&str> = ACRONYMS.iter().copied().collect();
        v.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
        format!("(?:{})", v.join("|"))
    });
    &ALT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_list_loads_and_skips_the_comment_header() {
        // 29 acronyms in the seed file; the 5 `#`-comment header lines and any blanks are skipped.
        assert_eq!(ACRONYMS.len(), 29);
        assert!(!ACRONYMS.iter().any(|a| a.starts_with('#')));
    }

    #[test]
    fn alternation_contains_the_major_acronyms() {
        let alt = acronym_alternation();
        for a in ["ATCC", "DSM", "DSMZ", "CBS", "JCM", "NBRC", "LMG"] {
            assert!(alt.contains(a), "{a} must be in the alternation {alt}");
        }
    }

    #[test]
    fn alternation_lists_longer_acronyms_before_their_prefixes() {
        let alt = acronym_alternation();
        // DSMZ must appear before DSM so "DSMZ 123" isn't matched as "DSM" + stray "Z 123"
        let dsmz = alt.find("DSMZ").expect("DSMZ present");
        let dsm = alt
            .find("DSM|")
            .or_else(|| alt.find("DSM)"))
            .expect("DSM present");
        assert!(dsmz < dsm, "DSMZ must precede the bare DSM in {alt}");
        assert!(alt.starts_with("(?:"));
    }
}
