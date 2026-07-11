// SPDX-License-Identifier: Apache-2.0

//! Integration test: `nameparser-cli parse` end-to-end through the *compiled binary* (not
//! just the library function) — proves the CLI wrapper's JSONL envelope (line numbering, key
//! order/omission, compact-JSON formatting) reproduces the Java CLI's `JsonlWriter`/Gson
//! output, not just that `nameparser::parse()` itself has field parity (which
//! `crates/nameparser/tests/parse_golden.rs` already proves, via `serde_json::Value` diffing
//! that deliberately ignores JSON-object key order and array order for `warnings`/`notho`/
//! `epithetQualifier`).
//!
//! The full-corpus test below pipes `testdata/benchmark-data.txt` (the same ~8017-name corpus)
//! through the built binary and compares every row against `testdata/expected-parse.jsonl`
//! (generated straight from the real Java CLI — see that file's header/`parse_golden.rs`'s
//! module doc for the regeneration command). Rows are compared with plain string equality
//! FIRST (the strongest possible check — true byte-for-byte identity), falling back to a
//! structural compare that treats `parsed.warnings` as a set only when the raw bytes differ.
//! That fallback exists for exactly one pre-existing, documented reason (confirmed to be the
//! ONLY source of raw-byte differences over the whole corpus at the time of writing): Java's
//! `warnings` field is a `HashSet<String>`, whose iteration order is a deterministic function
//! of the strings' hash codes, not of insertion order — while the Rust model's
//! `warnings: Vec<String>` preserves insertion order. Both orders are individually
//! deterministic (reruns are stable on each side), they just don't always agree with each
//! other when a name carries 2+ warnings. This is a core-crate (`ParsedName`) modeling
//! characteristic from Phase 1, already carried by that phase's own golden harness
//! (`UNORDERED_FIELD_KEYS`) — not something this CLI task changes or should paper over by,
//! say, sorting warnings before writing them (which would just trade Java's hash-bucket order
//! for an arbitrary alphabetical one, with no better a claim to being "the" canonical order).

use std::process::Command;

const ORACLE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/expected-parse.jsonl"
);
const CORPUS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/benchmark-data.txt"
);

fn run_cli(cli_args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_nameparser-cli"))
        .args(cli_args)
        .output()
        .expect("failed to run nameparser-cli")
}

/// Structural equality for one parsed JSONL row that treats `parsed.warnings` as a SET rather
/// than a positionally-ordered array (see this module's doc comment for why); every other
/// field, at every nesting depth — including every other array — is compared with ordinary
/// `serde_json::Value` equality.
fn rows_match(a: &str, b: &str) -> bool {
    fn normalize(line: &str) -> serde_json::Value {
        let mut v: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("invalid JSON line {line:?}: {e}"));
        if let Some(warnings) = v
            .pointer_mut("/parsed/warnings")
            .and_then(|w| w.as_array_mut())
        {
            warnings.sort_by(|x, y| x.as_str().cmp(&y.as_str()));
        }
        v
    }
    a == b || normalize(a) == normalize(b)
}

#[test]
fn parse_over_the_benchmark_corpus_matches_the_java_cli_oracle() {
    let Ok(expected) = std::fs::read_to_string(ORACLE) else {
        eprintln!("SKIP: oracle {ORACLE} not found — see parse_golden.rs's module doc to regenerate it");
        return;
    };
    assert!(
        std::path::Path::new(CORPUS).exists(),
        "corpus {CORPUS} not found"
    );

    let output = run_cli(&[
        "parse",
        &format!("--input={CORPUS}"),
        "--output=-",
        "--quiet",
    ]);
    assert!(
        output.status.success(),
        "nameparser-cli exited with {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let actual = String::from_utf8(output.stdout).expect("stdout must be valid UTF-8");

    let actual_lines: Vec<&str> = actual.lines().collect();
    let expected_lines: Vec<&str> = expected.lines().collect();
    assert_eq!(
        actual_lines.len(),
        expected_lines.len(),
        "row count differs: CLI produced {}, oracle has {}",
        actual_lines.len(),
        expected_lines.len()
    );

    let mut mismatches = 0usize;
    for (i, (a, e)) in actual_lines.iter().zip(expected_lines.iter()).enumerate() {
        if !rows_match(a, e) {
            mismatches += 1;
            if mismatches <= 10 {
                eprintln!("MISMATCH row {}:\n  cli:    {a}\n  oracle: {e}", i + 1);
            }
        }
    }
    assert_eq!(
        mismatches, 0,
        "{mismatches} row(s) differ from the Java CLI oracle beyond warnings-set-order — see stderr"
    );
}

/// Small, human-scale companion to the full-corpus test above: three names piped through
/// stdin/stdout, asserted against rows captured from a real side-by-side run of this binary
/// and the Java CLI's shaded jar on the identical input (confirmed byte-for-byte identical
/// there, `diff` empty) — pins the exact JSONL text for a success row, an error-with-code row
/// (the `Tobacco mosaic virus` -> `NomCode::VIRUS` case), and multi-field authorship.
#[test]
fn parse_stdin_to_stdout_matches_a_verified_java_cli_transcript() {
    let output = {
        use std::io::Write as _;
        use std::process::Stdio;
        let mut child = Command::new(env!("CARGO_BIN_EXE_nameparser-cli"))
            .args(["parse", "--input=-", "--output=-", "--quiet"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn nameparser-cli");
        child
            .stdin
            .take()
            .expect("stdin was piped")
            .write_all(
                b"Abies alba Mill.\nVulpes vulpes silaceus Miller, 1907\nTobacco mosaic virus\n",
            )
            .expect("failed to write to child stdin");
        child.wait_with_output().expect("failed to wait on child")
    };

    assert!(
        output.status.success(),
        "nameparser-cli exited with {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let actual = String::from_utf8(output.stdout).expect("stdout must be valid UTF-8");
    let lines: Vec<&str> = actual.lines().collect();
    assert_eq!(lines.len(), 3, "expected exactly 3 JSONL rows, got: {actual:?}");

    assert_eq!(
        lines[0],
        r#"{"line":1,"input":"Abies alba Mill.","parsed":{"rank":"SPECIES","genus":"Abies","specificEpithet":"alba","candidatus":false,"type":"SCIENTIFIC","extinct":false,"doubtful":false,"manuscript":false,"state":"COMPLETE","warnings":[],"combinationAuthorship":{"authors":["Mill."],"exAuthors":[]},"basionymAuthorship":{"authors":[],"exAuthors":[]}}}"#
    );
    assert_eq!(
        lines[1],
        r#"{"line":2,"input":"Vulpes vulpes silaceus Miller, 1907","parsed":{"rank":"SUBSPECIES","code":"ZOOLOGICAL","genus":"Vulpes","specificEpithet":"vulpes","infraspecificEpithet":"silaceus","candidatus":false,"type":"SCIENTIFIC","extinct":false,"doubtful":false,"manuscript":false,"state":"COMPLETE","warnings":[],"combinationAuthorship":{"authors":["Miller"],"exAuthors":[],"year":"1907"},"basionymAuthorship":{"authors":[],"exAuthors":[]}}}"#
    );
    assert_eq!(
        lines[2],
        r#"{"line":3,"input":"Tobacco mosaic virus","error":{"type":"OTHER","code":"VIRUS","message":"Unparsable OTHER name: Tobacco mosaic virus"}}"#
    );
}

/// The `parse` command echoes the EXTRACTED (trimmed, first-tab-column) name in the `input`
/// field — matching the Java CLI (`ParseCli` sets `input = row.name()`, a `PlainTextReader.trim()`
/// value, verified byte-for-byte against the shaded jar). A leading/trailing-space input is
/// trimmed in both the parse AND the echo, so a padded line round-trips to the trimmed name.
#[test]
fn parse_echoes_the_trimmed_name_in_input_matching_java() {
    let output = {
        use std::io::Write as _;
        use std::process::Stdio;
        let mut child = Command::new(env!("CARGO_BIN_EXE_nameparser-cli"))
            .args(["parse", "--input=-", "--output=-", "--quiet"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn nameparser-cli");
        child
            .stdin
            .take()
            .expect("stdin was piped")
            .write_all(b"  Abies alba Mill.  \n")
            .expect("failed to write to child stdin");
        child.wait_with_output().expect("failed to wait on child")
    };
    assert!(output.status.success());
    let actual = String::from_utf8(output.stdout).expect("stdout must be valid UTF-8");
    let line = actual.lines().next().expect("expected one JSONL row");
    assert!(
        line.starts_with(r#"{"line":1,"input":"Abies alba Mill.","parsed":{"#),
        "`input` must echo the TRIMMED name (matching Java), not the raw padded line: {line}"
    );
    assert!(
        line.contains(r#""genus":"Abies""#) && line.contains(r#""specificEpithet":"alba""#),
        "the parse produces the clean atoms: {line}"
    );
}
