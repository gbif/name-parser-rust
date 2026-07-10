// SPDX-License-Identifier: Apache-2.0
//! Full-parse golden harness — error-classification gate (Phase 1, Task 6).
//!
//! Diffs Rust `nameparser::parse()` against the real Java CLI oracle
//! (`testdata/expected-parse.jsonl`) over the ~8017-name benchmark corpus
//! (`testdata/benchmark-data.txt`). Per the plan's Global Constraint ("this slice's gate
//! is error-classification only"), this test does NOT compare parsed-field content —
//! only:
//!   1. the parsed/unparsable PARTITION (Java `error` <=> Rust `Err`; Java `parsed` <=>
//!      Rust `Ok`), and
//!   2. for rows unparsable on both sides, that the `NameType`/`NomCode` agree.
//!
//! Downstream pipeline stages (StripAndStash onward) aren't ported yet, so a `parsed`/`Ok`
//! agreement's field CONTENT is out of scope until later slices.
//!
//! Regenerate the oracle with (see the Task 6 brief for the authoritative command):
//! ```text
//! export PATH="$HOME/.cargo/bin:$PATH"; [ -s "$HOME/.sdkman/bin/sdkman-init.sh" ] && source "$HOME/.sdkman/bin/sdkman-init.sh"
//! JAR=$(ls /Users/markus/code/gbif/name-parser/name-parser-cli/target/name-parser-cli-*-shaded.jar | head -1)
//! java -jar "$JAR" parse --input=- --output=- --format=jsonl \
//!   < testdata/benchmark-data.txt > testdata/expected-parse.jsonl 2>/dev/null
//! ```

use nameparser::model::{NameType, NomCode};

/// Java `Enum.name()` string for a Rust wire enum (e.g. `NameType::Other` -> `"OTHER"`).
/// Reuses the enum's own `Serialize` impl (`#[serde(rename_all = "SCREAMING_SNAKE_CASE")]`)
/// rather than a hand-maintained match, so it can never drift from the wire format —
/// mirrors the private `model::java_name` helper in the lib crate, re-derived here since
/// this integration test only sees the crate's public API.
fn java_name<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .expect("wire enums always serialize to a JSON string")
        .trim_matches('"')
        .to_string()
}

/// One tallied disagreement between the Java oracle and Rust, kept for the capped
/// stderr listing.
struct Mismatch {
    line: i64,
    input: String,
    detail: String,
}

#[test]
fn matches_java_error_classification_over_corpus() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../testdata/expected-parse.jsonl"
    );
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("SKIP: oracle {path} not found — run Task 6 Step 1 to generate it");
            return;
        }
    };

    let mut total = 0usize;
    let mut partition_mismatches: Vec<Mismatch> = Vec::new();
    let mut type_code_mismatches: Vec<Mismatch> = Vec::new();

    for (idx, line) in data.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let row: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line {}: invalid oracle JSON: {e}\n{line}", idx + 1));
        let input = row["input"]
            .as_str()
            .unwrap_or_else(|| panic!("line {}: oracle row missing `input`", idx + 1));
        let line_no = row["line"].as_i64().unwrap_or(idx as i64 + 1);
        total += 1;

        let java_error = row.get("error");
        let java_parsed = row.get("parsed");
        assert!(
            java_error.is_some() ^ java_parsed.is_some(),
            "line {line_no}: oracle row has neither/both error+parsed, input={input:?}"
        );

        let rust_result = nameparser::parse(input, None, None, None);

        match (java_error, rust_result) {
            (Some(err), Ok(_)) => {
                partition_mismatches.push(Mismatch {
                    line: line_no,
                    input: input.to_string(),
                    detail: format!(
                        "java=Err({}) rust=Ok",
                        err["type"].as_str().unwrap_or("?")
                    ),
                });
            }
            (None, Err(rust_err)) => {
                partition_mismatches.push(Mismatch {
                    line: line_no,
                    input: input.to_string(),
                    detail: format!(
                        "java=Ok rust=Err({}, {:?})",
                        java_name(&rust_err.type_),
                        rust_err.code.map(|c| java_name(&c))
                    ),
                });
            }
            (Some(err), Err(rust_err)) => {
                // Both sides agree on the partition — compare type/code.
                let exp_type = err["type"].as_str().unwrap_or("");
                let got_type = java_name(&rust_err.type_);
                let exp_code = err.get("code").and_then(|c| c.as_str());
                let got_code = rust_err.code.map(|c| java_name(&c));
                if exp_type != got_type || exp_code != got_code.as_deref() {
                    type_code_mismatches.push(Mismatch {
                        line: line_no,
                        input: input.to_string(),
                        detail: format!(
                            "type: java={exp_type} rust={got_type}; code: java={exp_code:?} rust={got_code:?}"
                        ),
                    });
                }
            }
            (None, Ok(_)) => {
                // Both parsed. Parsed-field CONTENT is not gated this slice (downstream
                // pipeline stages aren't ported yet).
            }
        }
    }

    eprintln!(
        "parse golden (error-classification): {total} rows, {} partition-mismatches, {} type/code-mismatches",
        partition_mismatches.len(),
        type_code_mismatches.len()
    );
    for m in partition_mismatches.iter().take(20) {
        eprintln!("  PARTITION line {}: {:?} — {}", m.line, m.input, m.detail);
    }
    for m in type_code_mismatches.iter().take(20) {
        eprintln!("  TYPE/CODE  line {}: {:?} — {}", m.line, m.input, m.detail);
    }

    assert_eq!(
        partition_mismatches.len(),
        0,
        "{} partition mismatches (Java error/parsed vs Rust Err/Ok disagree) — see stderr with --nocapture",
        partition_mismatches.len()
    );
    assert_eq!(
        type_code_mismatches.len(),
        0,
        "{} type/code mismatches on rows both sides agree are unparsable — see stderr with --nocapture",
        type_code_mismatches.len()
    );
}

/// Sanity-checks this file's own enum-name mapping against the reference rows quoted in
/// the plan (Global Constraints, "Reference: exact JSONL shape"), independent of whatever
/// the corpus currently contains — guards against `java_name` itself drifting silently.
#[test]
fn java_name_matches_the_plans_reference_error_rows() {
    assert_eq!(java_name(&NameType::Other), "OTHER");
    assert_eq!(java_name(&NameType::Formula), "FORMULA");
    assert_eq!(java_name(&NomCode::Virus), "VIRUS");
}
