// SPDX-License-Identifier: Apache-2.0

//! Chinese, Korean and Taiwanese authors written surname first with the given name spelled
//! out: `Liu, Xian-wei`, `Park, Jong-Seok`, `Yang, Jeng-tze`, `Wang, Yuwen`. The comma between
//! surname and given name splits them into two authors; [`crate::pipeline::authorship_parser`]
//! asks [`given_name_after`] whether the second is the given name of the first.
//!
//! A word is told by its syllables. The syllable sets are generated from the initials, vowels
//! and finals of three romanisations, not listed: Hanyu Pinyin, Korean (Revised Romanisation
//! plus the older spellings that personal names keep: `Lee`, `Yoon`, `Choi`, `Park`) and
//! Wade-Giles (apostrophes dropped, with the usual Taiwanese variants). The pinyin set follows
//! the pinyin combination rules; the other two over-generate a little, which costs nothing
//! here because both the surname and every part of the given name must be syllables.
//!
//! - A **hyphenated** given name (`Xian-wei`, `Jong-Seok`) after a one-syllable surname is
//!   always the surname's: a hyphenated surname of syllables (`Ow-Yang`, `Siu-Ting`) after a
//!   lone syllable surname is too rare to matter, and `Al-Farraj`, `Ben-Dov` or `Ngoc-Ho` each
//!   have a half that is no syllable. Either half may be a two-syllable pinyin word
//!   (`Yu-Lingzi`).
//! - An **unhyphenated** given name (`Yuwen`, `Fasheng`) must split into exactly two pinyin
//!   syllables, the second starting with a consonant, after a pinyin surname, and only when no
//!   other author of the team is a bare surname. `Fan, Chiba & Wang` lists surnames only, and
//!   so do `Sen, Saha & Raychaudhuri` and `Long, Fuge & Smith`.

use std::collections::HashSet;
use std::sync::LazyLock;

/// Pinyin finals by initial (`""` = no initial), toneless, `v` for `ü`.
const PINYIN: &[(&str, &str)] = &[
    ("", "a o e ai ei ao ou an en ang eng er"),
    ("b p", "a o ai ei ao an en ang eng u i ie iao ian in ing"),
    (
        "m",
        "a o e ai ei ao ou an en ang eng u i ie iao iu ian in ing",
    ),
    ("f", "a o ei ou an en ang eng u"),
    (
        "d t",
        "a e ai ei ao ou an en ang eng ong u uo ui uan un i ie iao iu ian ing",
    ),
    (
        "n l",
        "a e ai ei ao ou an en ang eng ong u uo uan un i ie iao iu ian in iang ing v ve ue",
    ),
    (
        "g k h",
        "a e ai ei ao ou an en ang eng ong u ua uo uai ui uan un uang",
    ),
    ("j q x", "i ia ie iao iu ian in iang ing iong u ue uan un"),
    (
        "zh ch sh",
        "i a e ai ei ao ou an en ang eng ong u ua uo uai ui uan un uang",
    ),
    ("r", "i e ao ou an en ang eng ong u uo ui uan un"),
    (
        "z c s",
        "i a e ai ei ao ou an en ang eng ong u uo ui uan un",
    ),
    ("y", "a o e ao ou an ang i in ing ong u ue uan un"),
    ("w", "a o ai ei an en ang eng u"),
];

/// Korean: initials × vowels × finals, Revised Romanisation plus the spellings personal names
/// keep (`oo`, `ee`, `oi`).
const KOREAN_INITIALS: &str = "g k kk n d t tt r l m b p pp s ss j jj ch h y w";
const KOREAN_VOWELS: &str = "a ae ya yae eo e yeo ye o wa wae oe oi yo u oo wo we wi yu eu ui i ee";
const KOREAN_FINALS: &str = "k n l m p ng";
/// Surname spellings outside the scheme.
const KOREAN_EXTRA: &str = "park ahn young oh";

/// Wade-Giles: initials × finals, apostrophes dropped, plus the Taiwanese variants (`yu`,
/// `yun`, `yan`, `wa`, `we`, `ze`, `zu`).
const WADE_GILES_INITIALS: &str = "p m f t n l k h ch hs sh j ts tz s ss sz y w";
const WADE_GILES_FINALS: &str = "a o e eh ai ei ao ou an en ang eng erh ih u ung i ia ieh iao \
    iu ien in iang ing iung ua uo uai ui uei uan un uang ue uen yu yun yan wa we ze zu";

static PINYIN_SYLLABLES: LazyLock<HashSet<String>> = LazyLock::new(|| {
    PINYIN
        .iter()
        .flat_map(|(initials, finals)| combine(initials, finals))
        .collect()
});

static SYLLABLES: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let mut all = PINYIN_SYLLABLES.clone();
    for vowel in KOREAN_VOWELS.split_whitespace() {
        for initial in std::iter::once("").chain(KOREAN_INITIALS.split_whitespace()) {
            for last in std::iter::once("").chain(KOREAN_FINALS.split_whitespace()) {
                all.insert(format!("{initial}{vowel}{last}"));
            }
        }
    }
    all.extend(KOREAN_EXTRA.split_whitespace().map(str::to_string));
    all.extend(combine("", WADE_GILES_FINALS));
    all.extend(combine(WADE_GILES_INITIALS, WADE_GILES_FINALS));
    all
});

/// Every `initial + final`; an empty `initials` stands for the finals on their own.
fn combine(initials: &str, finals: &str) -> Vec<String> {
    let initials: Vec<&str> = if initials.is_empty() {
        vec![""]
    } else {
        initials.split_whitespace().collect()
    };
    initials
        .iter()
        .flat_map(|i| finals.split_whitespace().map(move |f| format!("{i}{f}")))
        .collect()
}

/// `word` lower-cased and without apostrophes (`Ch'en`), or `None` when it holds anything but
/// ASCII letters.
fn fold(word: &str) -> Option<String> {
    let folded: String = word
        .chars()
        .filter(|&c| c != '\'')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    (!folded.is_empty() && folded.chars().all(|c| c.is_ascii_lowercase())).then_some(folded)
}

fn is_syllable(word: &str) -> bool {
    fold(word).is_some_and(|w| SYLLABLES.contains(&w))
}

fn is_pinyin(word: &str) -> bool {
    fold(word).is_some_and(|w| PINYIN_SYLLABLES.contains(&w))
}

fn starts_with_vowel(s: &str) -> bool {
    s.starts_with(['a', 'e', 'i', 'o', 'u', 'v'])
}

/// Exactly two pinyin syllables, both opening with a consonant, and no single syllable:
/// `Yuwen`, `Fasheng`, `Xiangwei` — but not `Xian` (one syllable) or `Lee` (`le` + `e`).
fn two_pinyin_syllables(word: &str) -> bool {
    let Some(w) = fold(word) else {
        return false;
    };
    if PINYIN_SYLLABLES.contains(&w) || starts_with_vowel(&w) {
        return false;
    }
    (2..w.len().saturating_sub(1)).any(|k| {
        PINYIN_SYLLABLES.contains(&w[..k])
            && PINYIN_SYLLABLES.contains(&w[k..])
            && !starts_with_vowel(&w[k..])
    })
}

/// How `given` reads as the given name of the surname `surname` written before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GivenName {
    /// `Xian-wei`, `Jong-Seok`: certain.
    Hyphenated,
    /// `Yuwen`: only when the rest of the team is no list of bare surnames.
    Joined,
}

/// Whether `given` is the given name of `surname`, written surname first with a comma between
/// them. `None` for any other pair, including every Western one.
pub(crate) fn given_name_after(surname: &str, given: &str) -> Option<GivenName> {
    if !is_syllable(surname) || !surname.starts_with(|c: char| c.is_ascii_uppercase()) {
        return None;
    }
    let first = given.chars().next()?;
    if !first.is_ascii_uppercase() {
        return None;
    }
    if let Some((a, b)) = given.split_once('-') {
        let part = |p: &str| is_syllable(p) || two_pinyin_syllables(p);
        return (!b.contains('-') && part(a) && part(b)).then_some(GivenName::Hyphenated);
    }
    (is_pinyin(surname) && two_pinyin_syllables(given)).then_some(GivenName::Joined)
}

/// An author written as a bare surname, no initials and no given name: `Smith`, `O'Kennon`,
/// `Barrion-Dupo` — one word with a capital first, no dot and some lower case.
pub(crate) fn is_bare_surname(author: &str) -> bool {
    author.chars().next().is_some_and(char::is_uppercase)
        && author.chars().any(char::is_lowercase)
        && !author.contains(['.', ' '])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hyphenated_given_names() {
        for (surname, given) in [
            ("Liu", "Xian-wei"),
            ("Yin", "Zi-Wei"),
            ("Zheng", "Fa-ke"),
            ("Park", "Jong-Seok"),
            ("Ahn", "Kee-Jeong"),
            ("Oh", "Kwang-Sik"),
            ("Yang", "Jeng-tze"),
            ("Liu", "Hsing-Che"),
            ("Zhou", "Yu-Lingzi"),
        ] {
            assert_eq!(
                given_name_after(surname, given),
                Some(GivenName::Hyphenated),
                "{surname}, {given}"
            );
        }
    }

    #[test]
    fn hyphenated_surnames_stay_apart() {
        for (surname, given) in [
            ("Zhao", "Al-Farraj"),
            ("Chao", "Carvalho-Filho"),
            ("Lin", "Ngoc-Ho"),
            ("Furth", "Ben-Dov"),
            ("Brown", "Siu-Ting"),
            ("Barrion", "Barrion-Dupo"),
            ("Tan", "Bishop-Hurley"),
        ] {
            assert_eq!(given_name_after(surname, given), None, "{surname}, {given}");
        }
    }

    #[test]
    fn joined_given_names() {
        for (surname, given) in [("Wang", "Yuwen"), ("Liu", "Xiangwei"), ("Zhang", "Fasheng")] {
            assert_eq!(
                given_name_after(surname, given),
                Some(GivenName::Joined),
                "{surname}, {given}"
            );
        }
    }

    #[test]
    fn surnames_that_are_no_given_names() {
        for (surname, given) in [
            ("Xing", "Yan"),
            ("Wang", "Li"),
            ("Liu", "Xian"),
            ("Li", "Lee"),
            ("Zhang", "Ouyang"),
            ("Fan", "Sato"),
            ("Smith", "Yuwen"),
            ("Kim", "Yuwen"),
        ] {
            assert_eq!(given_name_after(surname, given), None, "{surname}, {given}");
        }
    }
}
