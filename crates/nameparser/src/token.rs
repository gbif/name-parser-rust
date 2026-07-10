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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_construction() {
        let t = Token { kind: TokenKind::Word, text: "Aus".into(), start: 0, end: 3 };
        assert_eq!(t.kind, TokenKind::Word);
        assert_eq!(t.text, "Aus");
    }
}
