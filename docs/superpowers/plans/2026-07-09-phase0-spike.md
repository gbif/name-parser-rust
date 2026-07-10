# Name Parser Rust — Phase 0 Spike Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove — with real numbers, not estimates — that the GBIF name parser's pipeline can be ported to Rust faithfully and runs faster, before committing to the full single-source-of-truth rewrite (approach B).

**Architecture:** Stand up a Cargo workspace with a pure-Rust `nameparser` core crate. Port the one self-contained, regex-free unit (the tokenizer) and validate it byte-for-byte against the Java tokenizer over the real benchmark corpus, surfacing every Java↔Rust Unicode-semantics gap. Then port a batch of real regex patterns — including the `CORRIG` lookaround and the clean anchored strippers — to measure the regex engine's speed and feel the lookaround-removal pain. Finish with a criterion benchmark and a findings doc that answers the go/no-go.

**Tech Stack:** Rust (edition 2021), `regex` crate (linear-time, RE2-lineage), `fancy-regex` (lookaround escape hatch), `criterion` (benchmarking). Oracle generated from the existing Java parser (Java 17, Maven).

## Global Constraints

- **Faithful port, not a redesign.** Every ported unit must reproduce the Java behaviour exactly. Behaviour changes (even bug fixes) are recorded in the findings doc as explicit divergences, never applied silently.
- **Reference source lives in a sibling repo:** the Java implementation is at `/Users/markus/code/gbif/name-parser/`. All "Java ref" paths below are relative to that repo root.
- **Rust `Token` offsets are byte offsets** (UTF-8), whereas Java `Token.start/end` are UTF-16 code-unit indices. This is a known, accepted divergence — the golden diff compares `kind:text` only, never offsets. Record it; do not try to match UTF-16 offsets in the spike.
- **Unicode semantics are the #1 spike risk.** Java `Character.isLetter/isDigit/isWhitespace/isUpperCase` do **not** map 1:1 to Rust `char` methods. Use the Java-faithful helpers defined in Task 2; every residual mismatch is triaged in Task 7.
- **License:** Apache-2.0, matching the Java repo. Add the SPDX header `// SPDX-License-Identifier: Apache-2.0` to each Rust source file.
- **Crate version stays `0.0.0`** for the whole spike — this is throwaway-or-promote exploratory code, not a release.
- **Working directory** for all commands is the repo root `/Users/markus/code/gbif/name-parser-rust/` unless stated otherwise.

---

## File Structure

| File | Responsibility |
|---|---|
| `Cargo.toml` | Workspace manifest (members list) |
| `crates/nameparser/Cargo.toml` | Core crate manifest + deps + bench registration |
| `crates/nameparser/src/lib.rs` | Crate root; re-exports `token`, `regexes` |
| `crates/nameparser/src/token.rs` | `TokenKind`, `Token`, `tokenize()` — the ported tokenizer + Java-faithful char helpers |
| `crates/nameparser/src/regexes.rs` | Ported regex patterns (`LazyLock<Regex>`) + the `CORRIG` lookaround, both ways |
| `crates/nameparser/tests/tokenizer_golden.rs` | Bulk golden diff of `tokenize()` vs the Java oracle over the corpus |
| `crates/nameparser/benches/parse.rs` | Criterion benchmark: tokenizer + regex batch → µs/name |
| `tools/TokenDump.java` | Java oracle generator: reads names, emits `name<TAB>kind:text␟kind:text…` |
| `testdata/benchmark-data.txt` | Copy of the 8k-name benchmark corpus (from the Java CLI data dir) |
| `testdata/expected-tokens.tsv` | Generated oracle (git-ignored; regenerated on demand) |
| `.gitignore` | Ignore `target/`, `testdata/expected-tokens.tsv` |
| `docs/superpowers/findings/2026-07-09-phase0-spike-findings.md` | The go/no-go writeup |

---

## Task 1: Workspace scaffolding + token model

**Files:**
- Create: `Cargo.toml`
- Create: `crates/nameparser/Cargo.toml`
- Create: `crates/nameparser/src/lib.rs`
- Create: `crates/nameparser/src/token.rs` (model only in this task)
- Create: `.gitignore`

**Interfaces:**
- Produces: `nameparser::token::TokenKind` (enum: `Word, Number, HybridMark, OpenParen, CloseParen, OpenBracket, CloseBracket, OpenBrace, CloseBrace, Comma, Semicolon, Colon, Dot, Ampersand, Dagger, Other`), and `nameparser::token::Token { kind: TokenKind, text: String, start: usize, end: usize }`.

- [ ] **Step 1: Initialise the repo and workspace manifest**

Run:
```bash
cd /Users/markus/code/gbif/name-parser-rust
git init -q
```

Create `Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = ["crates/nameparser"]
```

Create `.gitignore`:
```gitignore
/target
Cargo.lock
testdata/expected-tokens.tsv
```

- [ ] **Step 2: Create the crate manifest**

Create `crates/nameparser/Cargo.toml`:
```toml
[package]
name = "nameparser"
version = "0.0.0"
edition = "2021"
license = "Apache-2.0"
publish = false

[dependencies]
regex = "1.11"
fancy-regex = "0.14"

[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "parse"
harness = false
```

- [ ] **Step 3: Create the crate root**

Create `crates/nameparser/src/lib.rs`:
```rust
// SPDX-License-Identifier: Apache-2.0
pub mod regexes;
pub mod token;
```

- [ ] **Step 4: Write the failing model test**

Create `crates/nameparser/src/token.rs`:
```rust
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
```

- [ ] **Step 5: Build and test**

Run: `cargo test -p nameparser --lib`
Expected: compiles; `token::tests::token_construction` passes. (`regexes.rs` does not exist yet — that's Step 6.)

- [ ] **Step 6: Add an empty regexes module so the crate builds**

Create `crates/nameparser/src/regexes.rs`:
```rust
// SPDX-License-Identifier: Apache-2.0
// Patterns are added in Task 4.
```

Run: `cargo build -p nameparser`
Expected: clean build.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Phase 0 spike: cargo workspace + token model"
```

---

## Task 2: Port the tokenizer

Java ref: `name-parser/src/main/java/org/gbif/nameparser/token/Tokenizer.java` (full file, 133 lines). This is a single-pass, regex-free codepoint scanner — the cleanest possible first port.

**Files:**
- Modify: `crates/nameparser/src/token.rs` (add helpers + `tokenize`)

**Interfaces:**
- Produces: `nameparser::token::tokenize(input: &str) -> Vec<Token>`.

- [ ] **Step 1: Write the failing tokenizer tests**

These cases come directly from the Java tokenizer's documented behaviour (its doc comments and inline rules). Add to the `tests` module in `crates/nameparser/src/token.rs`:
```rust
    fn kinds(input: &str) -> Vec<(TokenKind, String)> {
        tokenize(input).into_iter().map(|t| (t.kind, t.text)).collect()
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
            vec![(Word, "Aus".into()), (HybridMark, "\u{00D7}".into()), (Word, "bus".into())]
        );
        // bare ASCII x between spaces
        assert_eq!(
            kinds("Aus x bus"),
            vec![(Word, "Aus".into()), (HybridMark, "x".into()), (Word, "bus".into())]
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p nameparser --lib`
Expected: FAIL — `tokenize` is not defined (`cannot find function tokenize`).

- [ ] **Step 3: Implement the Java-faithful char helpers**

Add near the top of `crates/nameparser/src/token.rs` (after the `Token` struct):
```rust
/// java.lang.Character.isWhitespace — Unicode space separators EXCEPT the non-breaking
/// ones (U+00A0, U+2007, U+202F), plus the ASCII/control whitespace incl. the file/group/
/// record/unit separators. Rust's `char::is_whitespace` differs: it INCLUDES U+00A0 and
/// EXCLUDES U+001C–U+001F. This helper matches Java. (Global constraint: Unicode semantics.)
fn is_whitespace_java(c: char) -> bool {
    match c {
        '\u{00A0}' | '\u{2007}' | '\u{202F}' => false,
        '\u{001C}' | '\u{001D}' | '\u{001E}' | '\u{001F}' => true,
        _ => c.is_whitespace(),
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
```

- [ ] **Step 4: Implement `tokenize`**

Add to `crates/nameparser/src/token.rs`. This mirrors the Java control flow one-to-one; the only structural change is iterating a `Vec<(byte_offset, char)>` instead of UTF-16 index arithmetic.
```rust
/// Single-pass tokenizer. Faithful port of the Java `Tokenizer.tokenize`.
/// Whitespace is consumed and not emitted. Letter runs may include internal hyphens,
/// apostrophes, underscores and stray '!' when flanked by letters/digits.
pub fn tokenize(input: &str) -> Vec<Token> {
    let cs: Vec<(usize, char)> = input.char_indices().collect();
    let n = cs.len();
    let byte_at = |k: usize| -> usize {
        if k < n { cs[k].0 } else { input.len() }
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
                    '-' | '\'' | '\u{2019}' | '_' | '!'
                        | '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}'
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
                let kind = if left_ok && right_ok { TokenKind::HybridMark } else { TokenKind::Word };
                out.push(Token { kind, text: word.to_string(), start: start_b, end: end_b });
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
            out.push(Token { kind: TokenKind::Word, text: word.to_string(), start: start_b, end: end_b });
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
                out.push(Token { kind: TokenKind::Word, text: input[s..e].to_string(), start: s, end: e });
                continue;
            }
            let s = byte_at(num_start);
            let e = byte_at(k);
            out.push(Token { kind: TokenKind::Number, text: input[s..e].to_string(), start: s, end: e });
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
        out.push(Token { kind, text: input[s..e].to_string(), start: s, end: e });
        k += 1;
    }
    out
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p nameparser --lib`
Expected: PASS — all tokenizer tests green.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "Phase 0 spike: port the tokenizer with Java-faithful char helpers"
```

---

## Task 3: Golden-diff the tokenizer against the Java oracle over the corpus

This is the harness-methodology proof and the Unicode-gap detector. It builds a tiny Java oracle, runs it over the real 8k-name benchmark corpus, and asserts the Rust tokenizer produces the identical token stream.

**Files:**
- Create: `tools/TokenDump.java`
- Create: `testdata/benchmark-data.txt` (copied)
- Create: `crates/nameparser/tests/tokenizer_golden.rs`

**Interfaces:**
- Consumes: `nameparser::token::{tokenize, TokenKind}` from Task 2.

- [ ] **Step 1: Copy the benchmark corpus into the spike repo**

Run:
```bash
cp /Users/markus/code/gbif/name-parser/name-parser-cli/data/benchmark-data.txt \
   testdata/benchmark-data.txt
wc -l testdata/benchmark-data.txt
```
Expected: ~8000 lines.

- [ ] **Step 2: Write the Java oracle generator**

Create `tools/TokenDump.java`:
```java
// SPDX-License-Identifier: Apache-2.0
import java.io.*;
import java.nio.charset.StandardCharsets;
import org.gbif.nameparser.token.Token;
import org.gbif.nameparser.token.Tokenizer;

/** Reads names (one per line; text before the first TAB is the name), emits
 *  "name<TAB>KIND:text\u001FKIND:text..." so the Rust golden test can diff token streams. */
public class TokenDump {
  public static void main(String[] args) throws Exception {
    try (BufferedReader r = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8));
         BufferedWriter w = new BufferedWriter(new OutputStreamWriter(System.out, StandardCharsets.UTF_8))) {
      String line;
      while ((line = r.readLine()) != null) {
        int tab = line.indexOf('\t');
        String name = tab >= 0 ? line.substring(0, tab) : line;
        if (name.isBlank() || name.startsWith("#")) continue;
        StringBuilder sb = new StringBuilder();
        for (Token t : Tokenizer.tokenize(name)) {
          if (sb.length() > 0) sb.append('\u001F');
          sb.append(t.kind).append(':').append(t.text);
        }
        w.write(name);
        w.write('\t');
        w.write(sb.toString());
        w.write('\n');
      }
    }
  }
}
```

- [ ] **Step 3: Build the Java parser jar and generate the oracle**

Run (builds the parser module, then compiles+runs the dumper against its classes):
```bash
# 1. Build the Java parser (produces target/classes with the Tokenizer)
mvn -q -f /Users/markus/code/gbif/name-parser/pom.xml -pl name-parser -am \
    -DskipTests install

# 2. Locate the compiled classes dir
JAVACP=/Users/markus/code/gbif/name-parser/name-parser/target/classes

# 3. Compile the dumper against it
javac -cp "$JAVACP" -d tools/out tools/TokenDump.java

# 4. Generate the oracle
java -cp "$JAVACP:tools/out" TokenDump \
    < testdata/benchmark-data.txt > testdata/expected-tokens.tsv
wc -l testdata/expected-tokens.tsv
```
Expected: `expected-tokens.tsv` has one line per non-comment corpus name (~4700+).

- [ ] **Step 4: Write the golden-diff integration test**

Create `crates/nameparser/tests/tokenizer_golden.rs`:
```rust
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
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/expected-tokens.tsv");
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
    assert_eq!(mismatches, 0, "tokenizer diverges from Java on {mismatches}/{total} names");
}
```

- [ ] **Step 5: Run the golden test and record the first result**

Run: `cargo test -p nameparser --test tokenizer_golden -- --nocapture`
Expected: it prints `tokenizer golden: N names, M mismatches`. **M may be > 0 on the first run** — that is the point of the spike. Copy the printed `DIFF:` lines somewhere; they feed Task 7.

- [ ] **Step 6: Triage and close mismatches**

For each distinct mismatch, classify it:
- **Helper-semantics gap** (e.g. a non-breaking space or a non-ASCII digit tokenised differently) → fix the relevant helper in `token.rs` (`is_whitespace_java`/`is_digit`/`is_letter`) to match Java, re-run.
- **Genuinely irreducible Unicode difference** → leave it, and note the exact input + cause for the findings doc.

Re-run until the only remaining mismatches are documented-irreducible:
Run: `cargo test -p nameparser --test tokenizer_golden -- --nocapture`
Expected: `mismatches` is 0, or a small number you have written down with root causes.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Phase 0 spike: tokenizer golden-diff harness vs Java oracle"
```

---

## Task 4: Port the clean anchored regex strippers

Ports five real patterns that use no lookaround, so they translate straight to the `regex` crate. These both prove the common case is trivial and provide the payload for the speed benchmark.

Java ref: `name-parser/src/main/java/org/gbif/nameparser/pipeline/StripAndStash.java` — `SIC` (:29-30), `AGGREGATE` (:138-141), `IN_PRESS` (:144-145), `PUBLISHED_PAGE` (:157-158), `TAX_NOTE` (:68-84).

**Files:**
- Modify: `crates/nameparser/src/regexes.rs`

**Interfaces:**
- Produces: `nameparser::regexes::{SIC, AGGREGATE, IN_PRESS, PUBLISHED_PAGE, TAX_NOTE}` — each a `LazyLock<regex::Regex>`.

- [ ] **Step 1: Write the failing pattern tests**

Add to `crates/nameparser/src/regexes.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sic_removed() {
        assert_eq!(SIC.replace_all("Ameiva plei (sic) Bibron", "").as_ref(), "Ameiva plei Bibron");
    }

    #[test]
    fn aggregate_trimmed() {
        assert_eq!(AGGREGATE.replace_all("Achillea millefolium agg.", "").as_ref(), "Achillea millefolium");
    }

    #[test]
    fn in_press_removed() {
        assert_eq!(IN_PRESS.replace_all("Aus bus Smith in press", "").as_ref(), "Aus bus Smith");
    }

    #[test]
    fn published_page_captured() {
        let caps = PUBLISHED_PAGE.captures("Aus bus Smith : 377").unwrap();
        assert_eq!(&caps[1], "377");
    }

    #[test]
    fn tax_note_stripped() {
        assert_eq!(TAX_NOTE.replace_all("Aus bus sensu Smith", "").as_ref(), "Aus bus");
        // case-sensitive s.l. marker still matches lower-case
        assert_eq!(TAX_NOTE.replace_all("Aus bus s.l.", "").as_ref(), "Aus bus");
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p nameparser --lib regexes`
Expected: FAIL — `SIC`, `AGGREGATE`, etc. are not defined.

- [ ] **Step 3: Implement the patterns**

Replace the placeholder body of `crates/nameparser/src/regexes.rs` (keep the `tests` module) with:
```rust
// SPDX-License-Identifier: Apache-2.0
//! Ported StripAndStash patterns. The 169 possessive quantifiers in the Java sources are
//! dropped: the `regex` crate is a linear-time automaton, so possessive/greedy is moot.

use regex::Regex;
use std::sync::LazyLock;

/// Java SIC (StripAndStash.java:29-30): "[sic]" / "(sic)" / "[sic!]" with no inner comma.
pub static SIC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*[(\[]\s*sic\s*!?\s*[)\]]").unwrap());

/// Java AGGREGATE (StripAndStash.java:138-141): trailing " agg." / "species group" / "-group" …
pub static AGGREGATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:\s+(?:agg\.?|aggregate|species\s+group|species\s+complex|group|complex)|\s*-\s*group|\s*-\s*aggregate)\s*$",
    )
    .unwrap()
});

/// Java IN_PRESS (StripAndStash.java:144-145): trailing " in press".
pub static IN_PRESS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\s+in\s+press\b\.?").unwrap());

/// Java PUBLISHED_PAGE (StripAndStash.java:157-158): trailing " : 377" / ": 12-18".
pub static PUBLISHED_PAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*:\s*(\d+(?:[-\x{2013}]\d+)?)\s*$").unwrap());

/// Java TAX_NOTE (StripAndStash.java:68-84): trailing taxonomic-concept note.
/// Ported verbatim; note the inner `(?-i:…)` case-sensitive group for the s.l./s.str. markers,
/// which the `regex` crate supports directly.
pub static TAX_NOTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\s+,?\s*(auctt?\b\.?(?:[,.]?\s.*)?|sensu(?:\s.*)?|sec\.?(?:\s.*)?|nec\b(?:\s.*)?|nonn?\.?\s+\(?\p{Lu}.*|emend\b\.?\s+\(?\p{Lu}.*|fide\b\.?\s+\(?\p{Lu}.*|according\s+to\s+\p{Lu}.*|excl\.\s+.*|ss\b\.?\s+.*|(?-i:s\.\s*l\.?|s\.\s*str\.?|s\.\s*lat\.?|s\.\s*ampl\.?))$",
    )
    .unwrap()
});
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p nameparser --lib regexes`
Expected: PASS — all five pattern tests green. (If `TAX_NOTE` fails to compile, the `regex` crate rejected a construct — record which one; that is a real portability finding for Task 7.)

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "Phase 0 spike: port five lookaround-free StripAndStash patterns"
```

---

## Task 5: The lookaround probe — `CORRIG` two ways

`CORRIG` is the smallest pattern that uses **both** a lookbehind and a lookahead, so it is the ideal probe for "how painful is lookaround removal". Port it two ways: restructured onto the pure-`regex` engine, and verbatim via `fancy-regex`. Record which is preferable.

Java ref: `name-parser/src/main/java/org/gbif/nameparser/pipeline/StripAndStash.java:35-36`:
`\s*[\(\[]\s*corrig\.?\s*[\)\]]|(?<=\s)corrig\.?(?=\s|$)`

**Files:**
- Modify: `crates/nameparser/src/regexes.rs`

**Interfaces:**
- Produces: `nameparser::regexes::strip_corrig_restructured(&str) -> String` and `nameparser::regexes::strip_corrig_fancy(&str) -> String`.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `crates/nameparser/src/regexes.rs`:
```rust
    #[test]
    fn corrig_bracketed() {
        assert_eq!(strip_corrig_restructured("Aus bus (corrig.) Smith"), "Aus bus Smith");
        assert_eq!(strip_corrig_fancy("Aus bus (corrig.) Smith"), "Aus bus Smith");
    }

    #[test]
    fn corrig_bare_word_between_spaces() {
        // The bare form removes just "corrig." and leaves the surrounding spaces (Java behaviour);
        // downstream whitespace normalisation collapses them. We assert the collapsed form.
        let collapse = |s: String| s.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(collapse(strip_corrig_restructured("Aus bus corrig. Smith")), "Aus bus Smith");
        assert_eq!(collapse(strip_corrig_fancy("Aus bus corrig. Smith")), "Aus bus Smith");
    }

    #[test]
    fn corrig_not_matched_mid_word() {
        // "corrigenda" must NOT be touched (word-boundary behaviour of the bare form).
        assert_eq!(strip_corrig_restructured("Aus corrigenda Smith"), "Aus corrigenda Smith");
        assert_eq!(strip_corrig_fancy("Aus corrigenda Smith"), "Aus corrigenda Smith");
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p nameparser --lib regexes`
Expected: FAIL — `strip_corrig_restructured` / `strip_corrig_fancy` not defined.

- [ ] **Step 3: Implement both variants**

Add to `crates/nameparser/src/regexes.rs` (imports at top: add `use fancy_regex::Regex as FancyRegex;`):
```rust
// --- CORRIG, restructured onto the linear `regex` engine (no lookaround) ---
// The bracketed alternative needs no lookaround. The bare alternative replaces Java's
// zero-width (?<=\s)…(?=\s|$) by CAPTURING the boundaries and putting them back, so the
// boundary characters are preserved instead of consumed.
static CORRIG_BRACKETED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*[(\[]\s*corrig\.?\s*[)\]]").unwrap());
static CORRIG_BARE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^|\s)corrig\.?(\s|$)").unwrap());

pub fn strip_corrig_restructured(s: &str) -> String {
    let s = CORRIG_BRACKETED.replace_all(s, "");
    // ${1}${2} (braced) so the two group refs can't be misparsed as one name.
    CORRIG_BARE.replace_all(&s, "${1}${2}").into_owned()
}

// --- CORRIG verbatim via fancy-regex (lookaround supported; backtracking) ---
// Input is length-bounded by the pipeline's MAX_LENGTH cap, so the backtracking engine is
// not a ReDoS risk here. This is the escape-hatch strategy for irreducible patterns.
static CORRIG_FANCY: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(r"\s*[(\[]\s*corrig\.?\s*[)\]]|(?<=\s)corrig\.?(?=\s|$)").unwrap()
});

pub fn strip_corrig_fancy(s: &str) -> String {
    // fancy-regex does NOT mirror the regex crate's `replace_all`: its match methods return
    // Result (backtracking can hit a limit). This manual splice — the fancy-regex equivalent of
    // `replace_all(s, "")` — is itself an ergonomics finding for the doc. An Err or end-of-matches
    // terminates the loop.
    let mut result = String::with_capacity(s.len());
    let mut last = 0usize;
    for m in CORRIG_FANCY.find_iter(s) {
        let m = match m {
            Ok(m) => m,
            Err(_) => break,
        };
        result.push_str(&s[last..m.start()]);
        last = m.end();
    }
    result.push_str(&s[last..]);
    result
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p nameparser --lib regexes`
Expected: PASS — all `corrig_*` tests green.

- [ ] **Step 5: Note the verdict in a code comment**

Add a short `//` note at the top of the CORRIG section recording which approach you'd standardise on and why (restructuring is dependency-free but must reason about boundary consumption; `fancy-regex` is a faithful drop-in but pulls in a backtracking engine). This one-liner feeds Task 7.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "Phase 0 spike: CORRIG lookaround probe, restructured vs fancy-regex"
```

---

## Task 6: Criterion benchmark — measure µs/name

Measures the two things Rust is claimed to win: raw scanning speed (tokenizer) and regex-engine speed (the batch of ported patterns), over the same 8k-name corpus the Java `benchmark` command uses.

**Files:**
- Create: `crates/nameparser/benches/parse.rs`

**Interfaces:**
- Consumes: `nameparser::token::tokenize`, `nameparser::regexes::{SIC, AGGREGATE, TAX_NOTE, PUBLISHED_PAGE}`.

- [ ] **Step 1: Write the benchmark**

Create `crates/nameparser/benches/parse.rs`:
```rust
// SPDX-License-Identifier: Apache-2.0
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nameparser::regexes;
use nameparser::token::tokenize;

fn load_corpus() -> Vec<String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/benchmark-data.txt");
    std::fs::read_to_string(path)
        .expect("benchmark-data.txt missing — run Task 3 Step 1")
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split('\t').next().unwrap().to_string())
        .collect()
}

fn bench(c: &mut Criterion) {
    let names = load_corpus();
    let count = names.len();
    eprintln!("corpus: {count} names (divide the reported time by {count} for µs/name)");

    c.bench_function("tokenize_corpus", |b| {
        b.iter(|| {
            for n in &names {
                black_box(tokenize(black_box(n)));
            }
        })
    });

    c.bench_function("regex_batch_corpus", |b| {
        b.iter(|| {
            for n in &names {
                black_box(regexes::SIC.replace_all(n, ""));
                black_box(regexes::AGGREGATE.replace_all(n, ""));
                black_box(regexes::TAX_NOTE.replace_all(n, ""));
                black_box(regexes::PUBLISHED_PAGE.replace_all(n, ""));
            }
        })
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
```

- [ ] **Step 2: Run the benchmark**

Run: `cargo bench -p nameparser`
Expected: criterion prints a time per iteration for `tokenize_corpus` and `regex_batch_corpus`. Each iteration processes the whole corpus once.

- [ ] **Step 3: Compute and record µs/name**

Take criterion's reported per-iteration time, divide by the corpus name count printed in Step 1:
- `tokenize_corpus`: `time_per_iter / count` = µs/name for tokenisation.
- `regex_batch_corpus`: `time_per_iter / count` = µs/name for the 4-pattern batch.

Write both numbers down for Task 7. Reference point: the Java **full** parse is ~28 µs/name (these components are a fraction of that, so expect single-digit or sub-µs numbers — the ratio is what matters).

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "Phase 0 spike: criterion benchmark for tokenizer + regex batch"
```

---

## Task 7: Findings doc — the go/no-go

Consolidate the spike's evidence into a decision document.

**Files:**
- Create: `docs/superpowers/findings/2026-07-09-phase0-spike-findings.md`

- [ ] **Step 1: Write the findings doc from the template**

Create `docs/superpowers/findings/2026-07-09-phase0-spike-findings.md` and fill every bracketed field from the earlier tasks:
```markdown
# Phase 0 Spike — Findings

**Date:** [fill]
**Question:** Can the GBIF name parser port to Rust faithfully and faster (approach B)?

## 1. Port faithfulness (tokenizer golden diff)
- Corpus size: [N] names.
- Mismatches after triage: [M].
- Irreducible Unicode/semantics gaps found: [list each input + root cause, e.g. non-breaking
  space, non-ASCII decimal digit], and how Phase 1 will handle each.
- Verdict: [tokenizer is/ isn't a mechanical port].

## 2. Lookaround pain (CORRIG probe)
- Restructured-onto-`regex`: [worked? how fiddly? boundary-consumption gotchas?].
- `fancy-regex` verbatim: [worked? drop-in?].
- Recommended default strategy for the ~29 lookaround/backref patterns: [restructure where
  trivial, fancy-regex for the rest — or other], with reasoning.
- Any pattern the `regex` crate rejected outright: [list, e.g. from TAX_NOTE].

## 3. Speed (criterion vs Java)
- tokenize: [X] µs/name.
- regex batch (4 patterns): [Y] µs/name.
- Java full parse reference: 28 µs/name.
- Extrapolated expectation for the full Rust pipeline: [reasoning].

## 4. Recommendation
- [ ] Proceed to Phase 1 (faithful whole-pipeline port).
- [ ] Proceed, but revise the design because: [...].
- [ ] Stop / reconsider because: [...].

## 5. Carry-forward risks for Phase 1
- [e.g. NOM_NOTE (StripAndStash.java:45-57) is the heaviest possessive+lookahead pattern —
  port and benchmark it early as the worst case.]
- [Unicode helper library decision: adopt `unicode-general-category` for exact isLetter/isDigit?]
```

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "Phase 0 spike: findings + go/no-go recommendation"
```

---

## Self-Review

**Spec coverage** (against `docs/superpowers/specs/2026-07-09-name-parser-rust-design.md` §11 "To validate in the spike"):
- "Rust-vs-Java raw parse speedup on a representative sample" → Task 6.
- "How painful the ~29 lookaround/backref patterns are" → Task 5 (CORRIG) + Task 4 (verbatim clean ports); NOM_NOTE flagged as carry-forward in Task 7.
- "Java↔Rust regex/Unicode semantic gaps surfaced by the first stages' corpus diff" → Task 3 (tokenizer golden diff) + Task 4 Step 4 (regex-construct rejection).
- **Deferred deliberately (documented here, not a gap):** the **object-build fraction of the 28 µs** and the **FFM boundary cost (JMH)** are Java-side measurements requiring the Java 22 + cdylib toolchain. They are the subject of a **follow-on plan (Phase 0b)**, gated on this spike returning "proceed". This keeps the spike a single-toolchain (pure-Rust + a one-file Java oracle) unit.

**Placeholder scan:** The only bracketed `[...]` fields are inside the Task 7 findings *template*, which is intentionally fill-in-the-blank output. All code steps contain complete, runnable code.

**Type consistency:** `TokenKind`/`Token` defined in Task 1, used unchanged in Tasks 2/3/6. `tokenize` signature (`&str -> Vec<Token>`) consistent across Tasks 2/3/6. `java_kind` (Task 3) covers every `TokenKind` variant defined in Task 1. Regex statics (`SIC`/`AGGREGATE`/`TAX_NOTE`/`PUBLISHED_PAGE`) defined in Task 4, consumed in Task 6. `strip_corrig_*` defined and consumed within Task 5.

**Scope:** Single subsystem (a pure-Rust spike). No decomposition needed.
