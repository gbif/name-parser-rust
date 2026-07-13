// SPDX-License-Identifier: Apache-2.0
//! NameFormatter golden harness — cross-validates the Rust [`ParsedName`] formatter
//! (`src/format.rs`, the port of Java `org.gbif.nameparser.util.NameFormatter`) against the
//! real Java `NameFormatter` over the ~8017-name benchmark corpus.
//!
//! The oracle `testdata/expected-format.tsv` has one row per corpus line, produced by the
//! Java `FormatOracle` (parse with `NameParserImpl`, then apply the five public
//! `NameFormatter` renderings):
//!
//! ```text
//! input \t ok \t canonical \t canonicalWithoutAuthorship \t canonicalMinimal \t canonicalComplete \t authorshipComplete
//! ```
//!
//! For every row where BOTH sides parse (Java `ok=true` and Rust `parse` returns `Ok`), this
//! diffs all five renderings. Because the Rust parser already reproduces Java's `ParsedName`
//! byte-for-byte (`parse_golden.rs`, 0 diffs), any difference here is a pure
//! formatter-logic difference — asserted to 0. Rows only one side parses are a parse-partition
//! matter already gated by `parse_golden.rs`; they're reported here but don't fail this test.
//!
//! Regenerate the oracle (Java 25 + the name-parser-cli shaded jar on the classpath):
//! ```text
//! [ -s "$HOME/.sdkman/bin/sdkman-init.sh" ] && source "$HOME/.sdkman/bin/sdkman-init.sh"
//! JAR=$(ls /Users/markus/code/gbif/name-parser/name-parser-cli/target/name-parser-cli-*-shaded.jar | head -1)
//! javac -cp "$JAR" -d <scratch> FormatOracle.java
//! java -cp "$JAR:<scratch>" FormatOracle < testdata/benchmark-data.txt > testdata/expected-format.tsv
//! ```
//! The `FormatOracle.java` source lives at `tools/FormatOracle.java` (see `tools/README.md`
//! for the full regenerate recipe). The `.tsv` is git-ignored; this test SKIPs when it is
//! absent, and the always-on structural coverage lives in `src/format.rs`'s own unit tests.

/// The five rendering columns, in TSV order after `input`/`ok`. Each names the Java method it
/// came from and the Rust method it is diffed against, so a failure message is self-explaining.
const COLUMNS: [&str; 5] = [
    "canonical",
    "canonicalWithoutAuthorship",
    "canonicalMinimal",
    "canonicalComplete",
    "authorshipComplete",
];

/// Cap on how many example mismatches are printed per column.
const EXAMPLE_CAP: usize = 8;

/// Inputs the 5.0.0 parser deliberately parses differently from the frozen 4.2.0 Java oracle — the
/// informal "tag capture" enhancement (Phase 5) — so their formatted forms differ too: the newly
/// captured `phrase` now renders (`"Elaeocarpus sp. Rocky Creek"`) where Java dropped it
/// (`"Elaeocarpus sp."`). Kept 1:1 with `parse_golden::INFORMAL_5_0_0_DIVERGENCES` (the two golden
/// binaries share no module). Skipped here so the intended change doesn't trip the 4.2.0 formatter
/// regression gate; the curated NEW-shape golden is P5's job.
const INFORMAL_5_0_0_DIVERGENCES: &[&str] = &[
    "Lacanobia sp. nr. subjuncta Bold:Aab, 0925",
    "Burkholderia sp. (Gigaspora margarita endosymbiont)",
    "Elaeocarpus sp. Rocky Creek",
];

/// Reverse the `FormatOracle.esc` escaping (`\\`, `\t`, `\r`, `\n`).
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('\\') => out.push('\\'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('n') => out.push('\n'),
                Some(other) => {
                    // Not an escape this oracle emits — keep verbatim.
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Rust `Option<String>` -> the oracle's empty-string convention (Java `null` -> "").
fn nz(s: Option<String>) -> String {
    s.unwrap_or_default()
}

struct Mismatch {
    line: usize,
    input: String,
    java: String,
    rust: String,
}

#[test]
fn matches_java_name_formatter_over_corpus() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../testdata/expected-format.tsv"
    );
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!(
                "SKIP: formatter oracle {path} not found — regenerate it with FormatOracle \
                 (see this test's module doc)"
            );
            return;
        }
    };

    let mut total = 0usize;
    let mut both_parsed = 0usize;
    let mut both_unparsable = 0usize;
    let mut partition_mismatches: Vec<Mismatch> = Vec::new();
    let mut counts = [0usize; 5];
    let mut examples: [Vec<Mismatch>; 5] = Default::default();

    for (idx, line) in data.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let line_no = idx + 1;
        let cols: Vec<&str> = line.split('\t').collect();
        assert!(
            cols.len() == 7,
            "line {line_no}: expected 7 tab-separated columns, got {}: {line:?}",
            cols.len()
        );
        let input = unescape(cols[0]);
        let java_ok = cols[1] == "true";
        total += 1;

        let rust = nameparser::parse(&input, None, None, None);
        match (java_ok, rust) {
            (true, Ok(pn)) => {
                both_parsed += 1;
                // Deliberate 5.0.0 informal tag-capture divergences (see the const's doc): the
                // captured phrase now renders where Java dropped it. Skip the formatter diff.
                if INFORMAL_5_0_0_DIVERGENCES.contains(&input.as_str()) {
                    continue;
                }
                let rust_vals = [
                    nz(pn.canonical_name()),
                    nz(pn.canonical_name_without_authorship()),
                    nz(pn.canonical_name_minimal()),
                    nz(pn.canonical_name_complete()),
                    nz(pn.authorship_complete()),
                ];
                for i in 0..5 {
                    let expected = unescape(cols[2 + i]);
                    if expected != rust_vals[i] {
                        counts[i] += 1;
                        if examples[i].len() < EXAMPLE_CAP {
                            examples[i].push(Mismatch {
                                line: line_no,
                                input: input.clone(),
                                java: expected,
                                rust: rust_vals[i].clone(),
                            });
                        }
                    }
                }
            }
            (false, Err(_)) => {
                // Both sides agree the name is unparsable — nothing to format, full agreement.
                both_unparsable += 1;
            }
            (java_ok, rust) => {
                // GENUINE partition disagreement: exactly one side parsed. Such a row can't be
                // formatter-compared (one side has no ParsedName), so it must not exist for this
                // test's numbers to mean anything. This is gated in full by parse_golden.rs;
                // asserted 0 here too as a self-guard.
                partition_mismatches.push(Mismatch {
                    line: line_no,
                    input: input.clone(),
                    java: format!("ok={java_ok}"),
                    rust: (if rust.is_ok() { "Ok" } else { "Err" }).to_string(),
                });
            }
        }
    }

    eprintln!(
        "format golden: {total} rows, {both_parsed} both-parsed, {both_unparsable} \
         both-unparsable, {} genuine parse-partition mismatches",
        partition_mismatches.len()
    );
    for m in partition_mismatches.iter().take(20) {
        eprintln!(
            "  PARTITION line {}: {:?} — java {}, rust {}",
            m.line, m.input, m.java, m.rust
        );
    }
    for i in 0..5 {
        let pct = if both_parsed == 0 {
            0.0
        } else {
            100.0 * counts[i] as f64 / both_parsed as f64
        };
        eprintln!(
            "  {:<28} {:>5} mismatches ({pct:>5.1}% of both-parsed rows)",
            COLUMNS[i], counts[i]
        );
    }
    for i in 0..5 {
        if !examples[i].is_empty() {
            eprintln!(
                "  --- {} (first {} example(s)) ---",
                COLUMNS[i],
                examples[i].len()
            );
            for m in &examples[i] {
                eprintln!(
                    "    line {}: {:?}\n      java={:?}\n      rust={:?}",
                    m.line, m.input, m.java, m.rust
                );
            }
        }
    }

    // Self-guard (full gate is parse_golden.rs): no row may parse on exactly one side, else
    // its renderings can't be compared and the counts above would silently under-sample.
    assert_eq!(
        partition_mismatches.len(),
        0,
        "{} rows parse on exactly one side (see stderr) — formatter comparison is incomplete",
        partition_mismatches.len()
    );

    // GATE: every rendering must match the Java oracle on every both-parsed row. Asserted
    // per-column so a failure names the exact renderer that broke; re-run with --nocapture
    // for up to EXAMPLE_CAP concrete failing inputs per column.
    for i in 0..5 {
        assert_eq!(
            counts[i], 0,
            "{} formatter mismatches on {:?} (Rust vs Java, over {both_parsed} both-parsed \
             rows) — see stderr with --nocapture",
            counts[i], COLUMNS[i]
        );
    }
}
