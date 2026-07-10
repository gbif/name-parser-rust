// SPDX-License-Identifier: Apache-2.0
//! Full-parse golden harness — error-classification gate (Phase 1, Task 6) + the
//! downstream-independent-field gate (Phase 1 Slice 2, batch 2e — the FINAL StripAndStash
//! batch flips this from a baseline print to a permanent `assert_eq!(…, 0)`).
//!
//! Diffs Rust `nameparser::parse()` against the real Java CLI oracle
//! (`testdata/expected-parse.jsonl`) over the ~8017-name benchmark corpus
//! (`testdata/benchmark-data.txt`). Two independent gates, BOTH now asserted:
//!
//!   1. **Error classification (asserted 0, unchanged since Task 6).** The parsed/unparsable
//!      PARTITION (Java `error` <=> Rust `Err`; Java `parsed` <=> Rust `Ok`), and, for rows
//!      unparsable on both sides, that the `NameType`/`NomCode` agree.
//!   2. **Downstream-independent fields (asserted 0, as of Phase 1 Slice 2 batch 2e).** For
//!      rows both sides parse, diffs the 10 fields that `StripAndStash` (now FULLY ported —
//!      all 55 steps) exclusively populates — `extinct`, `originalSpelling`,
//!      `nomenclaturalNote`, `taxonomicNote`, `publishedIn`, `publishedInPage`,
//!      `publishedInYear`, `manuscript`, `candidatus`, `cultivarEpithet` — via
//!      `serde_json::Value` on both sides (an absent JSON key means unset/`None`, matching
//!      the model's `skip_serializing_if`). Batches 2a-2d drove each per-field mismatch
//!      count down as the steps that set it landed (see
//!      `docs/superpowers/plans/2026-07-10-phase1-stripandstash.md` for the trajectory);
//!      batch 2e (steps 53-55, the last 3) brings every one of them to 0, and this gate now
//!      permanently `assert_eq!`s that count instead of merely printing it. Because these 10
//!      fields are set ONLY by StripAndStash and never touched again by any later (not yet
//!      ported) stage, this gate is a full, corpus-scale parity guarantee for them, not a
//!      snapshot — a regression in ANY of the 55 steps that touches one of these fields
//!      fails this test.
//!
//! Regenerate the oracle with (see the Task 6 brief for the authoritative command):
//! ```text
//! export PATH="$HOME/.cargo/bin:$PATH"; [ -s "$HOME/.sdkman/bin/sdkman-init.sh" ] && source "$HOME/.sdkman/bin/sdkman-init.sh"
//! JAR=$(ls /Users/markus/code/gbif/name-parser/name-parser-cli/target/name-parser-cli-*-shaded.jar | head -1)
//! java -jar "$JAR" parse --input=- --output=- --format=jsonl \
//!   < testdata/benchmark-data.txt > testdata/expected-parse.jsonl 2>/dev/null
//! ```

use std::collections::HashMap;

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

/// The 10 downstream-independent fields `StripAndStash` (Phase 1 Slice 2) exclusively
/// populates — Java JSON key names, in wire order (see `model::name::ParsedName`'s
/// field-order doc comment: `extinct`..`cultivarEpithet` interleave `ParsedName`'s own
/// fields with `ParsedAuthorship`'s). Same list as the plan's "Downstream-independent
/// field set" section.
const FIELD_KEYS: [&str; 10] = [
    "extinct",
    "originalSpelling",
    "nomenclaturalNote",
    "taxonomicNote",
    "publishedIn",
    "publishedInPage",
    "publishedInYear",
    "manuscript",
    "candidatus",
    "cultivarEpithet",
];

/// Cap on how many example mismatches are printed per field (mirrors the `.take(20)` cap
/// already used for the partition/type-code listings, just smaller since there are 10
/// fields rather than 2 categories).
const FIELD_EXAMPLE_CAP: usize = 5;

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

    // ASSERTED (see the module doc): rows where both sides parsed, and per-field mismatch
    // tallies + capped examples over the 10 downstream-independent fields StripAndStash
    // (now fully ported) populates.
    let mut both_parsed = 0usize;
    let mut field_mismatch_counts: HashMap<&'static str, usize> =
        FIELD_KEYS.iter().map(|&k| (k, 0)).collect();
    let mut field_mismatch_examples: HashMap<&'static str, Vec<Mismatch>> =
        FIELD_KEYS.iter().map(|&k| (k, Vec::new())).collect();

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
                    detail: format!("java=Err({}) rust=Ok", err["type"].as_str().unwrap_or("?")),
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
            (None, Ok(rust_pn)) => {
                // Both parsed. Diff the 10 downstream-independent fields StripAndStash
                // (Phase 1 Slice 2, now fully ported) populates — asserted 0 below, see
                // the module doc; full parsed-name parity (tokenised name parts,
                // authorship, etc.) stays out of scope until later slices.
                both_parsed += 1;
                let java_obj = java_parsed
                    .expect("java_error is None so java_parsed is Some (asserted above)");
                let rust_value =
                    serde_json::to_value(&rust_pn).expect("ParsedName always serializes to JSON");
                for &key in FIELD_KEYS.iter() {
                    let jv = java_obj.get(key);
                    let rv = rust_value.get(key);
                    if jv != rv {
                        *field_mismatch_counts.get_mut(key).unwrap() += 1;
                        let examples = field_mismatch_examples.get_mut(key).unwrap();
                        if examples.len() < FIELD_EXAMPLE_CAP {
                            examples.push(Mismatch {
                                line: line_no,
                                input: input.to_string(),
                                detail: format!("java={jv:?} rust={rv:?}"),
                            });
                        }
                    }
                }
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

    // Per-field mismatch counts over the `both_parsed` rows, for the 10
    // downstream-independent fields StripAndStash (now fully ported, Phase 1 Slice 2
    // batch 2e) populates. Printed unconditionally (cheap, and useful context even on a
    // passing run); asserted to all be 0 below.
    eprintln!(
        "parse golden (downstream-independent fields — StripAndStash fully ported, gate ASSERTED): {both_parsed} both-parsed rows"
    );
    for &key in FIELD_KEYS.iter() {
        let n = field_mismatch_counts[key];
        let pct = if both_parsed == 0 {
            0.0
        } else {
            100.0 * n as f64 / both_parsed as f64
        };
        eprintln!("  {key:<18} {n:>5} mismatches ({pct:>5.1}% of both-parsed rows)");
    }
    for &key in FIELD_KEYS.iter() {
        let examples = &field_mismatch_examples[key];
        if !examples.is_empty() {
            eprintln!("  --- {key} (first {} example(s)) ---", examples.len());
            for m in examples {
                eprintln!("    line {}: {:?} — {}", m.line, m.input, m.detail);
            }
        }
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

    // GATE (Phase 1 Slice 2 batch 2e — the final StripAndStash batch): every one of the 10
    // downstream-independent fields must have exactly 0 mismatches against the Java
    // oracle. StripAndStash (steps 1-55, all now ported) is the ONLY stage that sets these
    // fields and is never touched again by any later stage, so — unlike full parsed-name
    // parity, still out of scope until the tokenizer/authorship-parser/Assemble stages are
    // ported — this is a complete, permanent, corpus-scale parity guarantee for these 10
    // fields specifically, not a snapshot. Asserted per-field (not as a single pooled
    // total) so a future regression's failure message names the exact field that broke;
    // re-run with `--nocapture` for up to `FIELD_EXAMPLE_CAP` concrete failing inputs per
    // field from the eprintln! block above.
    for &key in FIELD_KEYS.iter() {
        let n = field_mismatch_counts[key];
        assert_eq!(
            n, 0,
            "{n} mismatches on downstream-independent field {key:?} (Java vs Rust, over \
             {both_parsed} both-parsed rows) — see stderr with --nocapture for example inputs"
        );
    }
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
