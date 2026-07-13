// SPDX-License-Identifier: Apache-2.0
//! NameFormatter golden harness — regression-checks the Rust [`ParsedName`] formatter
//! (`src/format.rs`, the port of Java `org.gbif.nameparser.util.NameFormatter`) over the
//! ~8017-name benchmark corpus.
//!
//! The snapshot `testdata/golden/expected-format.tsv` has one row per corpus line — the FIVE public
//! `NameFormatter` renderings of the parse of each input:
//!
//! ```text
//! input \t ok \t canonical \t canonicalWithoutAuthorship \t canonicalMinimal \t canonicalComplete \t authorshipComplete
//! ```
//!
//! **This is a Rust regression snapshot, not a live Java oracle**: the Java `NameParserImpl` /
//! `FormatOracle` are gone, so the snapshot is regenerated from the current Rust formatter (the
//! `#[ignore]`d [`regenerate`] test below) and re-baselined when behaviour changes intentionally.
//! Its ancestry is the Java `FormatOracle` output the port was cross-validated against. For every
//! row where BOTH sides parse this diffs all five renderings, asserted to 0; rows only one side
//! parses are a parse-partition matter already gated by `parse_golden.rs`, reported but not failed.
//!
//! Re-baseline, then REVIEW the git diff (the intentional-change log) before committing:
//! ```text
//! cargo test -p gbif-name-parser --test format_golden regenerate -- --ignored
//! git diff testdata/golden/expected-format.tsv
//! ```
//! The always-on structural coverage lives in `src/format.rs`'s own unit tests.

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

/// The `FormatOracle.esc` escaping (inverse of [`unescape`]): `\\`, `\t`, `\r`, `\n`.
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}

/// Regenerate `testdata/golden/expected-format.tsv` from the CURRENT Rust formatter output — the
/// re-base workflow (this golden is a Rust regression SNAPSHOT, not a live Java oracle; the Java
/// `NameParserImpl`/`FormatOracle` are gone). Reuses the existing row set (the corpus inputs in
/// column 0) and rewrites the `ok` + 5 rendering columns. Run with:
/// `cargo test -p gbif-name-parser --test format_golden regenerate -- --ignored`; then REVIEW the
/// git diff (it is the intentional-change log) before committing.
#[test]
#[ignore = "regeneration utility — rewrites the golden snapshot; run manually then review the diff"]
fn regenerate() {
    let data = std::fs::read_to_string(GOLDEN_PATH).expect("existing golden to reuse its input rows");
    let mut out = String::with_capacity(data.len());
    for line in data.lines() {
        if line.is_empty() {
            continue;
        }
        let input = unescape(line.split('\t').next().unwrap_or(""));
        let (ok, cells): (bool, [String; 5]) = match nameparser::parse_name(&input, None, None, None) {
            Ok(pn) => (
                true,
                [
                    nz(pn.canonical_name()),
                    nz(pn.canonical_name_without_authorship()),
                    nz(pn.canonical_name_minimal()),
                    nz(pn.canonical_name_complete()),
                    nz(pn.authorship_complete()),
                ],
            ),
            Err(_) => (false, Default::default()),
        };
        out.push_str(&esc(&input));
        out.push('\t');
        out.push_str(if ok { "true" } else { "false" });
        for cell in &cells {
            out.push('\t');
            out.push_str(&esc(cell));
        }
        out.push('\n');
    }
    std::fs::write(GOLDEN_PATH, out).expect("write regenerated golden");
    eprintln!("regenerated {GOLDEN_PATH}");
}

const GOLDEN_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/golden/expected-format.tsv"
);

struct Mismatch {
    line: usize,
    input: String,
    java: String,
    rust: String,
}

#[test]
fn matches_java_name_formatter_over_corpus() {
    let path = GOLDEN_PATH;
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

        let rust = nameparser::parse_name(&input, None, None, None);
        match (java_ok, rust) {
            (true, Ok(pn)) => {
                both_parsed += 1;
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
