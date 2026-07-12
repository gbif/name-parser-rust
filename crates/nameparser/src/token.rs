// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Word,
    Number,
    HybridMark,
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    OpenBrace,
    CloseBrace,
    Comma,
    Semicolon,
    Colon,
    Dot,
    Ampersand,
    Dagger,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    /// The verbatim matched text.
    pub text: String,
    /// Byte offset into the input (UTF-8). NB: Java uses UTF-16 code-unit offsets.
    pub start: usize,
    pub end: usize,
}

/// java.lang.Character.isWhitespace — Unicode space separators EXCEPT the non-breaking
/// ones (U+00A0, U+2007, U+202F), plus the ASCII/control whitespace incl. the file/group/
/// record/unit separators. Rust's `char::is_whitespace` differs: it INCLUDES U+00A0 and
/// EXCLUDES U+001C–U+001F. This helper matches Java. (Global constraint: Unicode semantics.)
/// `pub(crate)`: also used by `pipeline::stripandstash`'s `first_word` (Java
/// `StripAndStash.firstWord`, Phase 1 Slice 2 Task 3), not just this module's own tokenizer.
pub(crate) fn is_whitespace_java(c: char) -> bool {
    match c {
        '\u{00A0}' | '\u{2007}' | '\u{202F}' => false,
        '\u{001C}' | '\u{001D}' | '\u{001E}' | '\u{001F}' => true,
        _ => c.is_whitespace(),
    }
}

/// Java `org.gbif.nameparser.token.AuthorParticles.PARTICLES` — lower-case particles that
/// combine with a following capitalised author surname ("de Vriese", "Van Heurck", "von
/// der Linde"). Full 50-entry set transcribed verbatim from `AuthorParticles.java` (and
/// verified programmatically against it — an exact set match, no entries dropped or
/// added). The first call site (`StripAndStash::apply_missing_genus_placeholder`, Phase 1
/// Slice 2 Task 3) only needed the boolean `is_particle` — `NameTokens`/`AuthorshipSplit`/
/// `AuthorshipParser` now call it too (mirroring Java's own `isCapitalisedParticle`, which
/// needs the same table), so porting the full set once here rather than a hand-trimmed
/// subset paid off.
const PARTICLES: &[&str] = &[
    "a", "ab", "af", "ap", "auf", "d", "da", "dal", "dalla", "dalle", "dallo", "das", "de",
    "degli", "dei", "del", "della", "delle", "delli", "dello", "der", "des", "di", "do", "dos",
    "du", "el", "in", "la", "las", "le", "les", "lo", "los", "of", "ofver", "te", "ten", "ter",
    "und", "v", "van", "vd", "ven", "vom", "von", "y", "zu", "zum", "zur",
];

/// Java `AuthorParticles.isParticle(String)`: `PARTICLES.contains(word.toLowerCase())`.
/// Java's `.toLowerCase()` uses the JVM default locale; Rust's `.to_lowercase()` is the
/// full-Unicode locale-independent mapping — every entry in `PARTICLES` is pure ASCII, so
/// the two only-differ-for-Turkish-dotless-I-etc. divergences that exist in principle
/// cannot change whether any of these ASCII words match (same reasoning already applied to
/// `Preflight::run`'s `s.to_lowercase()` call).
pub(crate) fn is_particle(word: &str) -> bool {
    PARTICLES.contains(&word.to_lowercase().as_str())
}

#[cfg(test)]
mod author_particles_tests {
    use super::is_particle;

    #[test]
    fn recognises_lowercase_and_mixed_case_particles() {
        assert!(is_particle("van"));
        assert!(is_particle("Van"));
        assert!(is_particle("VAN"));
        assert!(is_particle("de"));
    }

    #[test]
    fn rejects_non_particle_words() {
        assert!(!is_particle("Smith"));
        assert!(!is_particle("denheyeri"));
        assert!(!is_particle(""));
    }
}

/// java.lang.Character.isLetter — Unicode general category L*. Rust's `is_alphabetic` is the
/// (slightly broader) Alphabetic property. Divergences are surfaced by the Task 3 golden diff.
fn is_letter(c: char) -> bool {
    c.is_alphabetic()
}

/// java.lang.Character.isDigit — Unicode Nd. Spike approximation: ASCII digits only.
/// Non-ASCII decimal digits (rare in names) will show up as Task 3 mismatches for triage.
fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

fn is_all_upper(s: &str) -> bool {
    for c in s.chars() {
        if is_letter(c) && !c.is_uppercase() {
            return false;
        }
    }
    true
}

/// Single-pass tokenizer. Faithful port of the Java `Tokenizer.tokenize`.
/// Whitespace is consumed and not emitted. Letter runs may include internal hyphens,
/// apostrophes, underscores and stray '!' when flanked by letters/digits.
pub fn tokenize(input: &str) -> Vec<Token> {
    let cs: Vec<(usize, char)> = input.char_indices().collect();
    let n = cs.len();
    let byte_at = |k: usize| -> usize {
        if k < n {
            cs[k].0
        } else {
            input.len()
        }
    };
    let mut out: Vec<Token> = Vec::with_capacity((n / 4).max(4));
    let mut k = 0usize;

    while k < n {
        let c = cs[k].1;

        if is_whitespace_java(c) {
            k += 1;
            continue;
        }

        if is_letter(c) {
            let word_start = k;
            k += 1;
            while k < n {
                let c2 = cs[k].1;
                if is_letter(c2) || is_digit(c2) {
                    k += 1;
                    continue;
                }
                // internal hyphen / apostrophe / underscore / stray '!' glued between letters/digits
                if matches!(
                    c2,
                    '-' | '\''
                        | '\u{2019}'
                        | '_'
                        | '!'
                        | '\u{2010}'
                        | '\u{2011}'
                        | '\u{2012}'
                        | '\u{2013}'
                        | '\u{2014}'
                ) && k + 1 < n
                {
                    let next = cs[k + 1].1;
                    if is_letter(next) || is_digit(next) {
                        k += 1;
                        continue;
                    }
                }
                break;
            }
            let start_b = byte_at(word_start);
            let end_b = byte_at(k);
            let word = &input[start_b..end_b];
            let first = word.chars().next().unwrap();
            let char_count = word.chars().count();

            if char_count == 1 && (first == 'x' || first == 'X') {
                let left_ok = word_start == 0 || is_whitespace_java(cs[word_start - 1].1);
                let right_ok = k == n || is_whitespace_java(cs[k].1);
                let kind = if left_ok && right_ok {
                    TokenKind::HybridMark
                } else {
                    TokenKind::Word
                };
                out.push(Token {
                    kind,
                    text: word.to_string(),
                    start: start_b,
                    end: end_b,
                });
                continue;
            } else if char_count >= 2 && (first == 'x' || first == 'X') {
                let second = word.chars().nth(1).unwrap();
                if second.is_uppercase() && !is_all_upper(word) {
                    let first_len = first.len_utf8();
                    out.push(Token {
                        kind: TokenKind::HybridMark,
                        text: word[..first_len].to_string(),
                        start: start_b,
                        end: start_b + first_len,
                    });
                    out.push(Token {
                        kind: TokenKind::Word,
                        text: word[first_len..].to_string(),
                        start: start_b + first_len,
                        end: end_b,
                    });
                    continue;
                }
            }
            out.push(Token {
                kind: TokenKind::Word,
                text: word.to_string(),
                start: start_b,
                end: end_b,
            });
            continue;
        }

        if is_digit(c) {
            let num_start = k;
            k += 1;
            while k < n && is_digit(cs[k].1) {
                k += 1;
            }
            // "11-punctata": a number glued to hyphen + letter is a leading-numeral epithet word
            if k + 1 < n && cs[k].1 == '-' && is_letter(cs[k + 1].1) {
                k += 1; // consume hyphen
                while k < n {
                    let c2 = cs[k].1;
                    if is_letter(c2) || is_digit(c2) {
                        k += 1;
                        continue;
                    }
                    if c2 == '-' && k + 1 < n {
                        let next = cs[k + 1].1;
                        if is_letter(next) || is_digit(next) {
                            k += 1;
                            continue;
                        }
                    }
                    break;
                }
                let s = byte_at(num_start);
                let e = byte_at(k);
                out.push(Token {
                    kind: TokenKind::Word,
                    text: input[s..e].to_string(),
                    start: s,
                    end: e,
                });
                continue;
            }
            let s = byte_at(num_start);
            let e = byte_at(k);
            out.push(Token {
                kind: TokenKind::Number,
                text: input[s..e].to_string(),
                start: s,
                end: e,
            });
            continue;
        }

        let kind = match c {
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,
            '[' => TokenKind::OpenBracket,
            ']' => TokenKind::CloseBracket,
            '{' => TokenKind::OpenBrace,
            '}' => TokenKind::CloseBrace,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            ':' => TokenKind::Colon,
            '.' => TokenKind::Dot,
            '&' => TokenKind::Ampersand,
            '\u{00D7}' => TokenKind::HybridMark, // ×
            '+' => TokenKind::HybridMark,
            '\u{2020}' => TokenKind::Dagger, // †
            _ => TokenKind::Other,
        };
        let s = byte_at(k);
        let e = byte_at(k + 1);
        out.push(Token {
            kind,
            text: input[s..e].to_string(),
            start: s,
            end: e,
        });
        k += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(input: &str) -> Vec<(TokenKind, String)> {
        tokenize(input)
            .into_iter()
            .map(|t| (t.kind, t.text))
            .collect()
    }

    #[test]
    fn simple_binomial() {
        use TokenKind::*;
        assert_eq!(
            kinds("Aus bus"),
            vec![(Word, "Aus".into()), (Word, "bus".into())]
        );
    }

    #[test]
    fn author_year() {
        use TokenKind::*;
        assert_eq!(
            kinds("Aus bus L., 1758"),
            vec![
                (Word, "Aus".into()),
                (Word, "bus".into()),
                (Word, "L".into()),
                (Dot, ".".into()),
                (Comma, ",".into()),
                (Number, "1758".into()),
            ]
        );
    }

    #[test]
    fn internal_hyphen_and_apostrophe_stay_in_word() {
        assert_eq!(kinds("Hartmann-Schroder").len(), 1);
        assert_eq!(kinds("d'urvilleana").len(), 1);
        assert_eq!(kinds("pu!chra").len(), 1); // stray '!' between letters
    }

    #[test]
    fn leading_numeral_epithet_is_one_word() {
        use TokenKind::*;
        assert_eq!(kinds("11-punctata"), vec![(Word, "11-punctata".into())]);
        assert_eq!(kinds("2-pustulata"), vec![(Word, "2-pustulata".into())]);
    }

    #[test]
    fn bare_number_is_number() {
        use TokenKind::*;
        assert_eq!(kinds("1758"), vec![(Number, "1758".into())]);
    }

    #[test]
    fn hybrid_marks() {
        use TokenKind::*;
        // Unicode ×
        assert_eq!(
            kinds("Aus \u{00D7}bus"),
            vec![
                (Word, "Aus".into()),
                (HybridMark, "\u{00D7}".into()),
                (Word, "bus".into())
            ]
        );
        // bare ASCII x between spaces
        assert_eq!(
            kinds("Aus x bus"),
            vec![
                (Word, "Aus".into()),
                (HybridMark, "x".into()),
                (Word, "bus".into())
            ]
        );
        // xFoo — leading x glued to a capitalised word splits off
        assert_eq!(
            kinds("xBus"),
            vec![(HybridMark, "x".into()), (Word, "Bus".into())]
        );
    }

    #[test]
    fn punctuation_and_dagger() {
        use TokenKind::*;
        assert_eq!(
            kinds("\u{2020}Aus (Bus)"),
            vec![
                (Dagger, "\u{2020}".into()),
                (Word, "Aus".into()),
                (OpenParen, "(".into()),
                (Word, "Bus".into()),
                (CloseParen, ")".into()),
            ]
        );
    }
}
