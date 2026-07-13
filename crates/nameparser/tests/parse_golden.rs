// SPDX-License-Identifier: Apache-2.0
//! Full-parse golden harness — error-classification gate (Phase 1, Task 6) + full
//! `ParsedName` field parity (Phase 1 Slice 4 Task 4 — the back end: AuthorshipParser,
//! CodeInference, Assemble, `stripAuthorshipMarkers` all wired into `Pipeline::run` — plus a
//! following Phase 1 Slice 4 change that ported `replace_homoglyphs`'s backing table,
//! closing the field-parity gate's last residual).
//!
//! Diffs Rust `nameparser::parse()` against the golden SNAPSHOT
//! (`testdata/golden/expected-parse.jsonl`) over the ~8017-name benchmark corpus
//! (`testdata/benchmark-data.txt`). **This is a Rust regression snapshot, not a live Java oracle**:
//! the Java `NameParserImpl` was retired at 5.0.0, so the snapshot is regenerated FROM THE RUST CLI
//! and re-baselined (regenerate + review the git diff) whenever behaviour changes intentionally —
//! the file's git history is the intentional-change log. Its ancestry is the Java 4.2.0 output the
//! port was cross-validated against; Rust is the reference implementation now. Two gates, BOTH
//! asserted (a "Java"/"error"/"parsed" below is a snapshot row, not a live Java call):
//!
//!   1. **Error classification (asserted 0, unchanged since Task 6).** The parsed/unparsable
//!      PARTITION (Java `error` <=> Rust `Err`; Java `parsed` <=> Rust `Ok`), and, for rows
//!      unparsable on both sides, that the `NameType`/`NomCode` agree.
//!   2. **Full `ParsedName` field parity (asserted 0, no exceptions).** For
//!      rows both sides parse, diffs EVERY field of the Java `parsed` object —
//!      [`ALL_FIELD_KEYS`], the complete 30-field wire shape
//!      documented on `model::name::ParsedName`'s own field-order doc comment (`rank` ..
//!      `sanctioningAuthor`) — via `serde_json::Value` on both sides (an absent JSON key
//!      means unset/`None`, matching the model's `skip_serializing_if`).
//!
//!      Three fields are Java `Set`/`Map`-shaped, whose iteration order is not an
//!      insertion-order guarantee (`warnings`: `HashSet<String>`; `notho`:
//!      `EnumSet<NamePart>`; `epithetQualifier`: `EnumMap<NamePart, String>`) — these three
//!      ([`UNORDERED_FIELD_KEYS`]) are compared order-insensitively (as sets/maps — see
//!      [`json_eq_unordered`]). Every other field — including the nested authorship
//!      objects (`combinationAuthorship`/`basionymAuthorship`/`genericAuthorship`/
//!      `specificAuthorship`), whose own `authors`/`exAuthors` are genuinely ordered lists
//!      — is compared with plain `serde_json::Value` equality (correct here since a JSON
//!      *object*'s own key order never matters for `==`, this crate's `serde_json` has no
//!      `preserve_order` feature enabled, and a positional list compare is exactly what an
//!      ordered author list needs).
//!
//!      This is the milestone this task is named for: the Rust parser reproduces Java's
//!      full `ParsedName` — every parsed field, not just a downstream-independent subset —
//!      over the whole corpus, with 0 mismatches on every one of the 30 fields. [`ALLOWLIST`]
//!      is currently EMPTY — kept as infrastructure (not deleted) so a future regression
//!      that's already root-caused and consciously deferred can be allowlisted explicitly,
//!      the same way it briefly held a documented 4-count entry each for `specificEpithet`/
//!      `warnings` (the `replace_homoglyphs` stub, StripAndStash step 13 — since ported in
//!      full, closing that gap to 0).
//!
//! Re-baseline the snapshot from the current Rust CLI, then REVIEW the git diff (the
//! intentional-change log) before committing:
//! ```text
//! cargo build --release -p nameparser-cli
//! ./target/release/nameparser-cli parse --input=testdata/benchmark-data.txt \
//!   --output=testdata/golden/expected-parse.jsonl --quiet
//! git diff testdata/golden/expected-parse.jsonl
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

/// The complete 30-field wire shape of the Java `parsed` object, in wire order — see
/// `model::name::ParsedName`'s own field-order doc comment (`ParsedName`'s own 16 fields,
/// then `ParsedAuthorship`'s 11, then `CombinedAuthorship`'s 3). Every field Java's
/// `ParsedName` can serialize is diffed here — this is the full-parity gate the module doc
/// describes.
const ALL_FIELD_KEYS: [&str; 30] = [
    "rank",
    "code",
    "uninomial",
    "genus",
    "genericAuthorship",
    "infragenericEpithet",
    "specificEpithet",
    "specificAuthorship",
    "infraspecificEpithet",
    "cultivarEpithet",
    "phrase",
    "candidatus",
    "notho",
    "originalSpelling",
    "epithetQualifier",
    "type",
    "extinct",
    "taxonomicNote",
    "nomenclaturalNote",
    "publishedIn",
    "publishedInYear",
    "publishedInPage",
    "unparsed",
    "doubtful",
    "manuscript",
    "state",
    "warnings",
    "combinationAuthorship",
    "basionymAuthorship",
    "sanctioningAuthor",
];

/// Cap on how many example mismatches are printed per field.
const FIELD_EXAMPLE_CAP: usize = 5;

/// Fields Java serialises from a `Set`/`Map`-like collection whose iteration order is not
/// an insertion-order guarantee: `warnings` (`HashSet<String>`), `notho`
/// (`EnumSet<NamePart>`), `epithetQualifier` (`EnumMap<NamePart, String>`). Routed through
/// [`json_eq_unordered`] rather than plain `serde_json::Value` equality — see that
/// function's own doc comment. Every other field (including the nested authorship
/// objects, whose `authors`/`exAuthors` arrays ARE genuinely ordered) uses plain equality.
const UNORDERED_FIELD_KEYS: [&str; 3] = ["warnings", "notho", "epithetQualifier"];

/// A tiny, explicitly documented allowance for a field known to carry a residual count
/// against the Java oracle for a cause already identified and deferred by design elsewhere
/// in this port — NOT a general escape hatch. Every entry here must name its root cause.
/// Checked as `actual <= allowed` (a regression that *increases* the count still fails).
///
/// **Currently EMPTY — every field is full 0-mismatch parity.** This held
/// `[("specificEpithet", 4), ("warnings", 4)]` until the `replace_homoglyphs` table port
/// below closed it out: both entries traced to the SAME single root cause and the SAME 2
/// unique corpus names (each appearing twice in the corpus — lines 4429/5826 "Musca
/// domeſtica Linnaeus 1758" and 4430/5827 "Amphisbæna fuliginoſa Linnaeus 1758").
/// `replace_homoglyphs` (`pipeline::stripandstash`, step 13) was a DOCUMENTED no-op stub
/// because Java's `UnicodeUtils.replaceHomoglyphs` backing table (a codepoint ->
/// canonical-char resource, `crate::unicode`'s `HOMOGLYPHS`) was a sizeable sub-project of
/// its own, deferred by design since Phase 1 Slice 2 (StripAndStash), well before Phase 1
/// Slice 4. Both names contain U+017F LATIN SMALL LETTER LONG S ("ſ", an 18th-century
/// typographic variant of "s") in their specific epithet — "domeſtica"/"fuliginoſa" — which
/// Java's table folds to plain "s" (-> `specificEpithet`) and flags with the (non-gated
/// everywhere else) `HOMOGLYHPS` warning (-> `warnings`); the stub left both untouched. That
/// gap was confirmed NARROWLY scoped to this one character, not a broader one: the corpus's
/// other exotic-script-adjacent row, "Dreyfusia nüßlini" (German eszett "ß", lines
/// 4431/5828), was already untouched by Java's own table too (0 mismatches — `ß` is a
/// legitimate letter, not a homoglyph) and `genus="Amphisbæna"` (containing the "æ"
/// ligature) is ALSO left alone by Java on both of the affected rows (a ligature, not a
/// homoglyph — `replace_homoglyphs` does not fold it either, post-port) — so the residual
/// was exactly this one codepoint, not a systemic divergence. The table port loaded the
/// real table into `crate::unicode` (see its module doc for the loader/table details) and
/// wired it into step 13, closing both counts to 0.
const ALLOWLIST: &[(&str, usize)] = &[];

fn allowed_mismatches(key: &str) -> usize {
    ALLOWLIST
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, n)| *n)
        .unwrap_or(0)
}

/// Order-insensitive equality for a JSON value Java serialises from a `Set`/`Map`-like
/// collection: `warnings`'s `HashSet<String>`, `notho`'s `EnumSet<NamePart>` (both -> a JSON
/// array) and `epithetQualifier`'s `EnumMap<NamePart, String>` (-> a JSON object). Java's
/// `HashSet`/`EnumSet` iteration order is not guaranteed to match insertion order, and
/// `serde_json::Value`'s own `PartialEq` for the `Array` variant IS positional — so these
/// fields need this explicit set-shaped comparison. `epithetQualifier` (a JSON *object*)
/// would in practice already compare order-insensitively via plain `==` given this crate's
/// `serde_json` dependency has no `preserve_order` feature enabled (its `Map` is
/// `BTreeMap`-backed, canonically key-ordered regardless of insertion order) — but it's
/// routed through here too rather than leaning on that feature-flag default, so the
/// comparison stays correct even if that default ever changes.
///
/// `None`/absent on both sides is equal; one side absent and the other present (even an
/// empty array/object) is a mismatch, matching plain `Option`/`Value` equality — only the
/// *internal* ordering of a doubly-present array/object is ignored.
fn json_eq_unordered(jv: Option<&serde_json::Value>, rv: Option<&serde_json::Value>) -> bool {
    match (jv, rv) {
        (None, None) => true,
        (Some(a), Some(b)) => match (a, b) {
            (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                let mut a_sorted: Vec<String> = a.iter().map(|v| v.to_string()).collect();
                let mut b_sorted: Vec<String> = b.iter().map(|v| v.to_string()).collect();
                a_sorted.sort();
                b_sorted.sort();
                a_sorted == b_sorted
            }
            (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                let mut a_pairs: Vec<(String, String)> =
                    a.iter().map(|(k, v)| (k.clone(), v.to_string())).collect();
                let mut b_pairs: Vec<(String, String)> =
                    b.iter().map(|(k, v)| (k.clone(), v.to_string())).collect();
                a_pairs.sort();
                b_pairs.sort();
                a_pairs == b_pairs
            }
            _ => a == b,
        },
        _ => false,
    }
}

/// Dispatches to [`json_eq_unordered`] for [`UNORDERED_FIELD_KEYS`], plain
/// `serde_json::Value` equality for every other field (including the nested authorship
/// objects — see the module doc for why plain equality is correct there too).
fn fields_equal(key: &str, jv: Option<&serde_json::Value>, rv: Option<&serde_json::Value>) -> bool {
    if UNORDERED_FIELD_KEYS.contains(&key) {
        json_eq_unordered(jv, rv)
    } else {
        jv == rv
    }
}

#[test]
fn matches_java_error_classification_over_corpus() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../testdata/golden/expected-parse.jsonl"
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

    // ASSERTED (see the module doc): rows where both sides parsed, per-field mismatch
    // tallies + capped examples over the full 30-field `ParsedName` wire shape.
    let mut both_parsed = 0usize;
    let mut field_mismatch_counts: HashMap<&'static str, usize> =
        ALL_FIELD_KEYS.iter().map(|&k| (k, 0)).collect();
    let mut field_mismatch_examples: HashMap<&'static str, Vec<Mismatch>> =
        ALL_FIELD_KEYS.iter().map(|&k| (k, Vec::new())).collect();

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
                // Both parsed. Diff every field of the full `ParsedName` wire shape —
                // asserted 0 below (modulo ALLOWLIST), see the module doc.
                both_parsed += 1;
                let java_obj = java_parsed
                    .expect("java_error is None so java_parsed is Some (asserted above)");
                let rust_value =
                    serde_json::to_value(&rust_pn).expect("ParsedName always serializes to JSON");
                for &key in ALL_FIELD_KEYS.iter() {
                    let jv = java_obj.get(key);
                    let rv = rust_value.get(key);
                    if !fields_equal(key, jv, rv) {
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

    // Per-field mismatch counts over the `both_parsed` rows, for the full 30-field
    // `ParsedName` wire shape. Printed unconditionally (cheap, and useful context even on
    // a passing run); asserted to all be 0 (modulo ALLOWLIST) below.
    eprintln!(
        "parse golden (full ParsedName field parity — Task 4 back end wired in): {both_parsed} both-parsed rows"
    );
    for &key in ALL_FIELD_KEYS.iter() {
        let n = field_mismatch_counts[key];
        let pct = if both_parsed == 0 {
            0.0
        } else {
            100.0 * n as f64 / both_parsed as f64
        };
        eprintln!("  {key:<21} {n:>5} mismatches ({pct:>5.1}% of both-parsed rows)");
    }
    for &key in ALL_FIELD_KEYS.iter() {
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

    // GATE (Phase 1 Slice 4 — the milestone): every one of the 30 fields of the
    // full `ParsedName` wire shape must match the Java oracle, up to the (currently empty,
    // see its own doc comment) ALLOWLIST. Asserted per-field (not as a single pooled total) so
    // a future regression's failure message names the exact field that broke; re-run with
    // `--nocapture` for up to `FIELD_EXAMPLE_CAP` concrete failing inputs per field from the
    // eprintln! block above.
    for &key in ALL_FIELD_KEYS.iter() {
        let n = field_mismatch_counts[key];
        let allowed = allowed_mismatches(key);
        assert!(
            n <= allowed,
            "{n} mismatches on field {key:?} (Java vs Rust, over {both_parsed} both-parsed \
             rows), only {allowed} allowed — see stderr with --nocapture for example inputs"
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

/// Direct unit check of `json_eq_unordered`'s own logic, independent of whatever the
/// corpus currently contains: an array/object differing only in element/key order must
/// compare equal (unlike plain `serde_json::Value` equality, which — for the `Array`
/// variant — IS positional); differing content, length, or one-sided presence must not.
#[test]
fn json_eq_unordered_ignores_ordering_but_not_content() {
    use serde_json::json;

    // Arrays (`notho`'s shape): same elements, different order -> equal under this helper,
    // even though plain `Value` equality disagrees (sanity-checked below).
    let a = json!(["SPECIFIC", "GENERIC"]);
    let b = json!(["GENERIC", "SPECIFIC"]);
    assert_ne!(
        a, b,
        "sanity check: plain equality IS positional for JSON arrays"
    );
    assert!(json_eq_unordered(Some(&a), Some(&b)));

    // Objects (`epithetQualifier`'s shape): same pairs, different insertion order -> equal.
    let c = json!({"GENERIC": "aff.", "SPECIFIC": "cf."});
    let d = json!({"SPECIFIC": "cf.", "GENERIC": "aff."});
    assert!(json_eq_unordered(Some(&c), Some(&d)));

    // Differing content, length, or presence must still mismatch.
    assert!(!json_eq_unordered(
        Some(&json!(["SPECIFIC"])),
        Some(&json!(["GENERIC"]))
    ));
    assert!(!json_eq_unordered(
        Some(&json!(["SPECIFIC"])),
        Some(&json!(["SPECIFIC", "GENERIC"]))
    ));
    assert!(json_eq_unordered(None, None));
    assert!(!json_eq_unordered(Some(&json!(["SPECIFIC"])), None));
    assert!(!json_eq_unordered(None, Some(&json!(["SPECIFIC"]))));
}
