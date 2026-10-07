// SPDX-License-Identifier: Apache-2.0

//! Authors whose two surnames are joined by "y", loaded once from the embedded
//! `resources/double-surnames.tsv`: Spanish double surnames such as `Bolívar y Pieltain` or
//! `Dusmet y Alonso`, one person each. [`crate::pipeline::authorship_parser`] joins an `X y Y`
//! into one author only when the pair is listed; any other `y` separates two people, because
//! the string alone cannot tell `Dusmet y Alonso` (one person) from `Spix y Agassiz` (two).
//!
//! A pair is keyed on the word before the `y` and the first word after it that is no particle
//! (`de`, `del`, `la`), lower-cased and without accents: `Bolivar y Pieltain` matches the line
//! `Bolívar y Pieltain`, `Graells y de la Agüera` the key `graells y aguera`.

use std::collections::HashSet;
use std::sync::LazyLock;

use unicode_normalization::UnicodeNormalization;

const DOUBLE_SURNAMES_TSV: &str = include_str!("../../resources/double-surnames.tsv");

/// Words after the `y` that open the maternal surname rather than being it: `Asso y del Río`.
const PARTICLES: &[&str] = &["de", "del", "la"];

/// The keys of every listed spelling (the surnames column and its variants).
static KEYS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    DOUBLE_SURNAMES_TSV
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .skip(1) // the column header
        .flat_map(|line| {
            let mut cols = line.split('\t');
            let surnames = cols.next().unwrap_or_default();
            let variants = cols.nth(3).unwrap_or_default();
            std::iter::once(surnames).chain(variants.split('|').filter(|v| !v.is_empty()))
        })
        .filter_map(form_key)
        .collect()
});

/// True when `left y right` names one person: `left` is the word before the `y`, `right` the
/// first word after it that is no particle (see [`skips`]).
pub(crate) fn joins(left: &str, right: &str) -> bool {
    KEYS.contains(&key(left, right))
}

/// True for a particle that opens the maternal surname after the `y` and is skipped over to
/// reach the surname itself.
pub(crate) fn skips(word: &str) -> bool {
    PARTICLES.contains(&word)
}

fn key(left: &str, right: &str) -> String {
    format!("{} y {}", fold(left), fold(right))
}

/// Lower case, accents dropped.
fn fold(word: &str) -> String {
    word.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

/// The key of a spelling as listed: `Dus. y Alon.` → `dus y alon`, `C.Bolívar y Pieltain` →
/// `bolivar y pieltain`, `Caballero y C[aballero]` → `caballero y c`.
fn form_key(form: &str) -> Option<String> {
    let (before, after) = form.split_once(" y ")?;
    let left = before
        .trim_end_matches('.')
        .rsplit(|c: char| c.is_whitespace() || c == '.')
        .next()?;
    let right = after
        .split_whitespace()
        .find(|w| !skips(w))?
        .split(['.', ',', '['])
        .next()?;
    (!left.is_empty() && !right.is_empty()).then(|| key(left, right))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_loads_every_person_and_spelling() {
        let people = DOUBLE_SURNAMES_TSV
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .count()
            - 1;
        assert_eq!(people, 164);
        assert!(KEYS.len() >= people, "every person has at least one key");
    }

    #[test]
    fn spellings_are_matched_without_case_or_accents() {
        assert!(joins("Bolívar", "Pieltain"));
        assert!(joins("Bolivar", "Pieltain"));
        assert!(joins("BOLIVAR", "PIELTAIN"));
        // listed misspellings and abbreviations
        assert!(joins("Bolivar", "Pidtain"));
        assert!(joins("Dus", "Alon"));
        assert!(joins("Caballero", "C"));
        // the particle after the y is skipped
        assert!(joins("Graells", "Agüera"));
        assert!(joins("Asso", "Río"));
        // a hyphenated paternal surname is one word
        assert!(joins("Sánchez-Monge", "Parellada"));
    }

    #[test]
    fn two_people_are_not_listed() {
        assert!(!joins("Spix", "Agassiz"));
        assert!(!joins("Ruiz", "Pavón"));
        assert!(!joins("Rivero", "Serna"));
    }

    #[test]
    fn form_keys() {
        assert_eq!(form_key("Dus. y Alon.").as_deref(), Some("dus y alon"));
        assert_eq!(
            form_key("C.Bolívar y Pieltain").as_deref(),
            Some("bolivar y pieltain")
        );
        assert_eq!(
            form_key("Caballero y C[aballero]").as_deref(),
            Some("caballero y c")
        );
        assert_eq!(
            form_key("Graells y de la Agüera").as_deref(),
            Some("graells y aguera")
        );
        assert_eq!(form_key("Linnaeus"), None);
    }
}
