// SPDX-License-Identifier: Apache-2.0
//! Java `org.gbif.nameparser.util.UnicodeUtils` — ported subset. Only `normalizeQuotes` is
//! ported here; the homoglyph-replacement table in the same Java class is intentionally
//! NOT ported (out of scope for this slice).

/// Unicode apostrophe / single-quote variants that normalise to the ASCII apostrophe `'`.
/// Verbatim transcription of Java `UnicodeUtils.SINGLE_QUOTES`
/// (`"`´ʹʻʼʽˊˋ‘’‚‛′‵❛❜＇"`).
/// Every codepoint is a 4-hex-digit Java `\u` escape, i.e. a single UTF-16 code unit in
/// the Basic Multilingual Plane outside the surrogate range — so iterating Rust `char`s
/// (Unicode scalar values) reproduces Java's `char`-at-a-time (UTF-16 code unit) loop
/// exactly for this fixed table.
const SINGLE_QUOTES: &[char] = &[
    '\u{0060}', '\u{00B4}', '\u{02B9}', '\u{02BB}', '\u{02BC}', '\u{02BD}', '\u{02CA}', '\u{02CB}',
    '\u{0091}', '\u{0092}', '\u{2018}', '\u{2019}', '\u{201A}', '\u{201B}', '\u{2032}', '\u{2035}',
    '\u{275B}', '\u{275C}', '\u{FF07}',
];

/// Unicode double-quote / double-prime variants that normalise to the ASCII double quote
/// `"`. Verbatim transcription of Java `UnicodeUtils.DOUBLE_QUOTES`
/// (`"“”„‟″‶❝❞〝〞〟＂"`).
const DOUBLE_QUOTES: &[char] = &[
    '\u{0093}', '\u{0094}', '\u{201C}', '\u{201D}', '\u{201E}', '\u{201F}', '\u{2033}', '\u{2036}',
    '\u{275D}', '\u{275E}', '\u{301D}', '\u{301E}', '\u{301F}', '\u{FF02}',
];

/// Faithful port of Java `String.trim()`: strips only leading/trailing chars whose
/// codepoint is <= U+0020 (NOT the full Unicode White_Space set that Rust's str::trim uses).
/// Use this everywhere the Java source calls `.trim()`.
///
/// Java's `String.trim()` only removes characters with codepoint values 0–32 (space through
/// control chars), while Rust's `str::trim()` removes the entire Unicode `White_Space` category.
/// This means non-breaking space (U+00A0) and other Unicode spaces are trimmed by Rust but NOT
/// by Java, causing empty/length guard logic to diverge.
pub fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| (c as u32) <= 0x20)
}

/// Normalises the many unicode apostrophe / single-quote variants to the ASCII apostrophe
/// (') and the unicode double-quote variants to the ASCII double quote ("). Author names
/// and quoted/provisional names routinely arrive with curly, prime, modifier-letter,
/// low-9, fullwidth or Windows-1252 quotes; collapsing them to ASCII keeps tokenisation
/// and the parsed output consistent regardless of the input's quote style.
///
/// Faithful port of Java `UnicodeUtils.normalizeQuotes(String)`. Java's `null`-in/`null`-out
/// case is not ported: this takes and returns non-nullable Rust types, so callers threading
/// an optional authorship string (e.g. `Option<&str>`) map over it instead
/// (`authorship.map(normalize_quotes)`), which reproduces the same null-forwarding at the
/// `Option` level.
pub fn normalize_quotes(x: &str) -> String {
    x.chars()
        .map(|c| {
            if SINGLE_QUOTES.contains(&c) {
                '\''
            } else if DOUBLE_QUOTES.contains(&c) {
                '"'
            } else {
                c
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_curly_single_quotes_to_ascii_apostrophe() {
        assert_eq!(normalize_quotes("d\u{2019}Urville"), "d'Urville");
        assert_eq!(normalize_quotes("\u{2018}Aus\u{2019}"), "'Aus'");
    }

    #[test]
    fn folds_curly_double_quotes_to_ascii_double_quote() {
        assert_eq!(normalize_quotes("\u{201C}Aus bus\u{201D}"), "\"Aus bus\"");
    }

    #[test]
    fn folds_backtick_acute_and_fullwidth_apostrophe_variants() {
        assert_eq!(normalize_quotes("\u{0060}"), "'");
        assert_eq!(normalize_quotes("\u{00B4}"), "'");
        assert_eq!(normalize_quotes("\u{FF07}"), "'");
    }

    #[test]
    fn folds_low9_and_prime_double_quote_variants() {
        assert_eq!(normalize_quotes("\u{201E}Aus\u{201F}"), "\"Aus\"");
        assert_eq!(normalize_quotes("\u{2033}"), "\"");
    }

    #[test]
    fn leaves_plain_ascii_and_already_normalised_quotes_untouched() {
        assert_eq!(normalize_quotes("O'Brien"), "O'Brien");
        assert_eq!(normalize_quotes("\"Aus bus\" Smith"), "\"Aus bus\" Smith");
        assert_eq!(normalize_quotes("Abies alba Mill."), "Abies alba Mill.");
    }

    #[test]
    fn java_trim_strips_only_up_to_u0020() {
        assert_eq!(java_trim("  x\t"), "x");      // ASCII space + tab (<=0x20) stripped
        assert_eq!(java_trim("\n x \r"), "x");    // newline/CR stripped
        assert_eq!(java_trim("\u{00A0}x\u{00A0}"), "\u{00A0}x\u{00A0}"); // NBSP (>0x20) NOT stripped
        assert_eq!(java_trim("\u{00A0}"), "\u{00A0}"); // NBSP-only stays non-empty, matching Java
    }
}
