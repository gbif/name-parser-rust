// SPDX-License-Identifier: Apache-2.0

//! The in-authors of an `in` citation: the authors of the work a name was published in,
//! "Natterer in Kuhl, 1817" → `Kuhl`, "Boucher ex DC. in Lam. & DC." → `Lam.`, `DC.` (ICN Art. 46.2,
//! ICZN Recommendation 51E). The citation itself stays in `publishedIn`; this only reads an author
//! team out of it, and only when the text after the `in` is nothing but one, the way the authorship
//! parser would read authors. A reference, "Mill. in Gard. Dict. ed. 8: no. 1. 1768" or "Lesson,
//! 1839 in Ann. Sci. Nat.", has none: when in doubt there are no in-authors, as a journal taken for
//! an author pollutes the comparison of authors, while a missing in-author only loses what the
//! model never had before.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use crate::pipeline::authorship_parser;
use crate::token::tokenize;

const TITLE_WORDS_TXT: &str = include_str!("../../resources/citation-title-words.txt");

/// Words of a title that are no author: `Dict`, `Sci`, `Nat`, `Fl`, `Prodr`, `Herb` …
static TITLE_WORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    TITLE_WORDS_TXT
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
});

/// A word of a title ([`TITLE_WORDS`]), in lower case and without its dot.
pub(crate) fn is_title_word(word: &str) -> bool {
    TITLE_WORDS.contains(word)
}

/// The year closing a cited work: `, 1817`, ` 1883.`, ` (1830)`, `, [1851]`, `, 1900a`, `1838-1840`.
static TRAILING_YEAR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[\s,]*[(\[]?(?-u:\d{4})[a-z]?(?:\s*[-–]\s*(?-u:\d{2,4}))?[)\]]?\.?\s*$").unwrap()
});

/// The editors marker closing a cited work's team, before its year: `Löbl & Smetana (eds.), 2011`,
/// `Grandcolas (ed.), 2008`, `Chen, editor, 1993`. The editors are the authors the work is cited by.
static EDITORS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:,?\s*[(\[]\s*(?:[Ee]ds?|[Ee]ditors?|[Hh]rsg)\.?\s*[)\]]|,\s*(?:[Ee]ds?|[Ee]ditors?)\.?)\s*$",
    )
    .unwrap()
});

/// The author team of the work an `in` citation names, or an empty list when `citation` is no
/// author team: what is left after its year and editors marker has a digit, colon, bracket or
/// quote, another `in` or an `ex`, anything the authorship parser does not read as plain authors,
/// a bare initial (`T. & Laugier`), a possessive (`Griffith's Cuvier`, an edition of a work) or a
/// word of a title ([`TITLE_WORDS`]). Linnaeus is the one author cited by a single letter, `L.`.
pub(crate) fn team(citation: &str) -> Vec<String> {
    let text = TRAILING_YEAR.replace(citation, "");
    let text = EDITORS.replace(&text, "");
    if text == "L." {
        return vec![text.into_owned()];
    }
    if text.contains("'s ")
        || text.contains("’s ")
        || text.chars().any(|c| {
            c.is_ascii_digit() || matches!(c, ':' | '(' | ')' | '[' | ']' | '"' | '“' | '”' | '„')
        })
    {
        return Vec::new();
    }
    let tokens = tokenize(&text);
    if !authorship_parser::looks_like_author_run(&tokens, 0, tokens.len(), false)
        || tokens.iter().any(|t| t.text == "ex")
    {
        return Vec::new();
    }
    let st = authorship_parser::parse(&tokens, 0);
    let authors = st.combination.authors;
    if st.unparsed_from >= 0
        || st.basionym_present
        || st.basionym.exists()
        || st.combination.anonymous
        || st.combination.year.is_some()
        || !st.combination.ex_authors.is_empty()
        || authors.is_empty()
        || !authors.iter().all(|a| is_author(a))
    {
        return Vec::new();
    }
    authors
}

/// One author as the authorship parser writes it: some word of two letters or more, no bare
/// initials, and no word of a title (`Gard.Dict.`, `Ann.Sci.Nat.`, `Nova Acta`).
fn is_author(author: &str) -> bool {
    let words: Vec<String> = author
        .split(|c: char| c == '.' || c == '-' || c.is_whitespace())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    words
        .iter()
        .any(|w| w.chars().filter(|c| c.is_alphabetic()).count() >= 2)
        && !words.iter().any(|w| TITLE_WORDS.contains(w.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Vec<String> {
        team(s)
    }

    #[test]
    fn author_teams() {
        assert_eq!(t("Kuhl, 1817"), ["Kuhl"]);
        assert_eq!(t("Lam. & DC."), ["Lam.", "DC."]);
        assert_eq!(t("Van Heurck, 1883"), ["Van Heurck"]);
        assert_eq!(t("Cleve & Grunow, 1880"), ["Cleve", "Grunow"]);
        assert_eq!(t("Rabenh."), ["Rabenh."]);
        assert_eq!(t("Miller, 1920"), ["Miller"]);
        assert_eq!(
            t("Cuvier & Valenciennes (1830)"),
            ["Cuvier", "Valenciennes"]
        );
        assert_eq!(t("Schönherr, C.J."), ["C.J.Schönherr"]);
        assert_eq!(t("Müll. Arg."), ["Müll.Arg."]);
        assert_eq!(t("Hand.-Mazz."), ["Hand.-Mazz."]);
        assert_eq!(t("L."), ["L."]);
        assert_eq!(t("Löbl & Smetana (eds.), 2011"), ["Löbl", "Smetana"]);
        assert_eq!(t("Grandcolas (ed.), 2008"), ["Grandcolas"]);
        assert_eq!(t("Underw. & Britton (eds.)"), ["Underw.", "Britton"]);
        assert_eq!(t("Chen, editor, 1993"), ["Chen"]);
        assert_eq!(t("Rech. f. (ed.)"), ["Rech.f."]);
    }

    #[test]
    fn references() {
        assert!(t("Gard. Dict. ed. 8: no. 1. 1768").is_empty());
        assert!(t("Gard. Dict.").is_empty());
        assert!(t("Ann. Sci. Nat.").is_empty());
        assert!(t("Nova Acta").is_empty());
        assert!(t("Mart. Fl. Bras.").is_empty());
        assert!(t("A. P. de Candolle, Prodr.").is_empty());
        assert!(t("Herb. Sonder").is_empty());
        assert!(t("Van Heurck, Syn. Diat. Belg. 1883").is_empty());
        assert!(t("Griffith's Cuvier 1832").is_empty());
        assert!(t("Phycol. Germ.").is_empty());
        assert!(t("Tian, Li, Yang & Sun, in Chen, editor, 1993").is_empty());
        assert!(t("Fl.Dan. ex Fr.").is_empty());
        assert!(t("T. & Laugier").is_empty());
        assert!(t("B.S.G.").is_empty());
        assert!(t("anon. 1837").is_empty());
        assert!(t("Kirchner (1988), Taxon 37").is_empty());
    }
}
