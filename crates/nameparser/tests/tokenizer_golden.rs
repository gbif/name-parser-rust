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

/// The corpus token-stream gate. Its ancestry is a Java `Tokenizer` oracle from the port, but the
/// Java parser was retired at 5.0.0 and this is a RUST REGRESSION SNAPSHOT now: re-baseline it (and
/// review the git diff — that diff is the intentional-change log) whenever the tokenizer changes on
/// purpose. Deliberate divergences from the original Java stream so far, all from
/// gbif/name-parser-rust#16's unhyphenated leading-numeral epithet rule:
/// `Rhynchophorus 13punctatus …` and `Euxoa nr. idahoensis sp. 1clay` (both gained a correct
/// epithet the Java stream's `NUMBER` + `WORD` split threw away), and `Staphylococcus phage
/// 80alpha` (a token-stream-only change: the virus gate rejects that name before and after).
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

/// Re-baseline [`matches_java_tokenizer_over_corpus`]'s snapshot from the current tokenizer. Run
/// with `cargo test -p gbif-name-parser --test tokenizer_golden regenerate -- --ignored`, then
/// REVIEW the git diff before committing. Mirrors `format_golden`'s regeneration utility; the row
/// set (column 0) is reused as-is, only the token stream in column 1 is rewritten.
#[test]
#[ignore = "regeneration utility — rewrites the golden snapshot; run manually then review the diff"]
fn regenerate() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../testdata/golden/expected-tokens.tsv"
    );
    let data = std::fs::read_to_string(path).expect("golden snapshot must exist to re-baseline it");
    let mut out = String::with_capacity(data.len());
    for line in data.lines() {
        let name = line.split_once('\t').map_or(line, |(n, _)| n);
        out.push_str(name);
        out.push('\t');
        out.push_str(&rust_stream(name));
        out.push('\n');
    }
    std::fs::write(path, out).expect("golden snapshot must be writable");
}
