// SPDX-License-Identifier: Apache-2.0
use nameparser::token::{tokenize, TokenKind};

/// Map Rust TokenKind to the Java enum constant name (its `toString()`).
fn java_kind(k: TokenKind) -> &'static str {
    match k {
        TokenKind::Word => "WORD",
        TokenKind::Number => "NUMBER",
        TokenKind::HybridMark => "HYBRID_MARK",
        TokenKind::OpenParen => "OPEN_PAREN",
        TokenKind::CloseParen => "CLOSE_PAREN",
        TokenKind::OpenBracket => "OPEN_BRACKET",
        TokenKind::CloseBracket => "CLOSE_BRACKET",
        TokenKind::OpenBrace => "OPEN_BRACE",
        TokenKind::CloseBrace => "CLOSE_BRACE",
        TokenKind::Comma => "COMMA",
        TokenKind::Semicolon => "SEMICOLON",
        TokenKind::Colon => "COLON",
        TokenKind::Dot => "DOT",
        TokenKind::Ampersand => "AMPERSAND",
        TokenKind::Dagger => "DAGGER",
        TokenKind::Other => "OTHER",
    }
}

fn rust_stream(name: &str) -> String {
    tokenize(name)
        .iter()
        .map(|t| format!("{}:{}", java_kind(t.kind), t.text))
        .collect::<Vec<_>>()
        .join("\u{1F}")
}

#[test]
fn matches_java_tokenizer_over_corpus() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../testdata/golden/expected-tokens.tsv"
    );
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("SKIP: oracle {path} not found — run Task 3 Step 3 to generate it");
            return;
        }
    };

    let mut total = 0usize;
    let mut mismatches = 0usize;
    for line in data.lines() {
        let (name, expected) = match line.split_once('\t') {
            Some(pair) => pair,
            None => (line, ""),
        };
        total += 1;
        let got = rust_stream(name);
        if got != expected {
            mismatches += 1;
            if mismatches <= 30 {
                eprintln!("DIFF: {name}\n  exp: {expected}\n  got: {got}");
            }
        }
    }
    eprintln!("tokenizer golden: {total} names, {mismatches} mismatches");
    assert_eq!(
        mismatches, 0,
        "tokenizer diverges from Java on {mismatches}/{total} names"
    );
}
