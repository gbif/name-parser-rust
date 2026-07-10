// SPDX-License-Identifier: Apache-2.0

//! Integration test: `nameparser-cli compare` end-to-end through the *compiled binary* — two
//! small JSONL fixture files (written to a temp dir per test) diffed via a real process spawn,
//! matching the pattern `tests/parse_cli.rs`/`tests/benchmark_cli.rs` already establish for
//! `parse`/`benchmark`. The pure diffing/formatting logic (`diff_element`, `json_eq`,
//! `OrderedCounter`, `CompareReport::print_summary`, …) already has dedicated unit tests in
//! `src/main.rs`; this file only covers what those can't: the real binary, real file I/O, CLI
//! argument parsing, and stdout/exit-code/output-file behaviour.

use std::io::Write as _;
use std::process::Command;

fn run_cli(cli_args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_nameparser-cli"))
        .args(cli_args)
        .output()
        .expect("failed to run nameparser-cli")
}

/// Writes `contents` to a fresh temp file whose name embeds `label` (distinct per test, so
/// parallel test threads never collide) and the current process id (so a stale file left by a
/// previously-aborted run can't collide with this run either). Returns its path.
fn write_temp_jsonl(label: &str, contents: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "nameparser-cli-compare-test-{label}-{}.jsonl",
        std::process::id()
    ));
    let mut f = std::fs::File::create(&path).expect("failed to create temp fixture file");
    f.write_all(contents.as_bytes())
        .expect("failed to write temp fixture file");
    path
}

#[test]
fn compare_reports_identical_files_as_fully_identical() {
    let content = "{\"line\":1,\"input\":\"Abies alba Mill.\",\"parsed\":{\"rank\":\"SPECIES\"}}\n\
                   {\"line\":2,\"input\":\"x\",\"error\":{\"type\":\"OTHER\",\"message\":\"m\"}}\n";
    let a = write_temp_jsonl("identical-a", content);
    let b = write_temp_jsonl("identical-b", content);

    let output = run_cli(&["compare", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Rows compared:      2"), "{stdout}");
    assert!(stdout.contains("Rows identical:     2"), "{stdout}");
    assert!(stdout.contains("Rows differing:     0 (0.00%)"), "{stdout}");
    assert!(!stdout.contains("Status transitions"), "{stdout}");
    assert!(!stdout.contains("Top differing fields"), "{stdout}");

    let _ = std::fs::remove_file(&a);
    let _ = std::fs::remove_file(&b);
}

/// The primary TDD case this task's brief calls for: a known, real diff must be detected and
/// reported (rows differing, the field path, both values, and the status transition).
#[test]
fn compare_reports_a_known_field_diff_and_a_status_transition() {
    let a = write_temp_jsonl(
        "knowndiff-a",
        "{\"line\":1,\"input\":\"Abies alba\",\"parsed\":{\"rank\":\"SPECIES\",\"genus\":\"Abies\"}}\n\
         {\"line\":2,\"input\":\"y\",\"parsed\":{\"rank\":\"SPECIES\"}}\n",
    );
    let b = write_temp_jsonl(
        "knowndiff-b",
        "{\"line\":1,\"input\":\"Abies alba\",\"parsed\":{\"rank\":\"SPECIES\",\"genus\":\"Abia\"}}\n\
         {\"line\":2,\"input\":\"y\",\"error\":{\"type\":\"OTHER\",\"message\":\"nope\"}}\n",
    );

    let output = run_cli(&["compare", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("Rows compared:      2"), "{stdout}");
    assert!(stdout.contains("Rows identical:     0"), "{stdout}");
    assert!(
        stdout.contains("Rows differing:     2 (100.00%)"),
        "{stdout}"
    );
    assert!(stdout.contains("PARSED→ERROR"), "{stdout}");
    assert!(stdout.contains("parsed.genus"), "{stdout}");
    assert!(stdout.contains("\"Abies\""), "{stdout}");
    assert!(stdout.contains("\"Abia\""), "{stdout}");
    assert!(stdout.contains("Line 1 \"Abies alba\""), "{stdout}");

    let _ = std::fs::remove_file(&a);
    let _ = std::fs::remove_file(&b);
}

/// The second primary TDD case this task's brief calls for: `parsed.warnings` differing only in
/// array order (same content, Java-HashSet-order vs Rust-Vec-insertion-order) must report
/// IDENTICAL, not a false-positive diff.
#[test]
fn compare_treats_the_warnings_array_as_a_set_and_reports_identical() {
    let a = write_temp_jsonl(
        "warnorder-a",
        "{\"line\":1,\"input\":\"x\",\"parsed\":{\"rank\":\"SPECIES\",\"warnings\":[\"NAME_UNPARSABLE\",\"HOMOGLYPH\"]}}\n",
    );
    let b = write_temp_jsonl(
        "warnorder-b",
        "{\"line\":1,\"input\":\"x\",\"parsed\":{\"rank\":\"SPECIES\",\"warnings\":[\"HOMOGLYPH\",\"NAME_UNPARSABLE\"]}}\n",
    );

    let output = run_cli(&["compare", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Rows compared:      1"), "{stdout}");
    assert!(
        stdout.contains("Rows identical:     1"),
        "warnings-order-only difference must NOT count as a diff:\n{stdout}"
    );
    assert!(stdout.contains("Rows differing:     0 (0.00%)"), "{stdout}");

    let _ = std::fs::remove_file(&a);
    let _ = std::fs::remove_file(&b);
}

#[test]
fn compare_still_flags_a_genuine_warnings_content_difference() {
    let a = write_temp_jsonl(
        "warncontent-a",
        "{\"line\":1,\"input\":\"x\",\"parsed\":{\"rank\":\"SPECIES\",\"warnings\":[\"HOMOGLYPH\"]}}\n",
    );
    let b = write_temp_jsonl(
        "warncontent-b",
        "{\"line\":1,\"input\":\"x\",\"parsed\":{\"rank\":\"SPECIES\",\"warnings\":[\"HOMOGLYPH\",\"NAME_UNPARSABLE\"]}}\n",
    );

    let output = run_cli(&["compare", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Rows differing:     1 (100.00%)"),
        "a genuine content difference in warnings must still be reported:\n{stdout}"
    );
    assert!(stdout.contains("parsed.warnings"), "{stdout}");

    let _ = std::fs::remove_file(&a);
    let _ = std::fs::remove_file(&b);
}

#[test]
fn compare_ignore_whitespace_suppresses_spacing_only_differences() {
    let a = write_temp_jsonl(
        "ignorews-a",
        "{\"line\":1,\"input\":\"x\",\"parsed\":{\"rank\":\"SPECIES\",\"genus\":\"Abies  alba\"}}\n",
    );
    let b = write_temp_jsonl(
        "ignorews-b",
        "{\"line\":1,\"input\":\"x\",\"parsed\":{\"rank\":\"SPECIES\",\"genus\":\"Abiesalba\"}}\n",
    );

    let without = run_cli(&["compare", a.to_str().unwrap(), b.to_str().unwrap()]);
    let with_flag = run_cli(&[
        "compare",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        "--ignore-whitespace",
    ]);
    let stdout_without = String::from_utf8(without.stdout).unwrap();
    let stdout_with = String::from_utf8(with_flag.stdout).unwrap();

    assert!(
        stdout_without.contains("Rows differing:     1 (100.00%)"),
        "{stdout_without}"
    );
    assert!(
        stdout_with.contains("Rows differing:     0 (0.00%)"),
        "--ignore-whitespace must strip whitespace before comparing:\n{stdout_with}"
    );

    let _ = std::fs::remove_file(&a);
    let _ = std::fs::remove_file(&b);
}

#[test]
fn compare_reports_extra_rows_when_one_file_is_longer() {
    let a = write_temp_jsonl(
        "extra-a",
        "{\"line\":1,\"input\":\"x\",\"parsed\":{\"rank\":\"SPECIES\"}}\n",
    );
    let b = write_temp_jsonl(
        "extra-b",
        "{\"line\":1,\"input\":\"x\",\"parsed\":{\"rank\":\"SPECIES\"}}\n\
         {\"line\":2,\"input\":\"y\",\"parsed\":{\"rank\":\"SPECIES\"}}\n\
         {\"line\":3,\"input\":\"z\",\"parsed\":{\"rank\":\"SPECIES\"}}\n",
    );

    let output = run_cli(&["compare", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Rows compared:      1"), "{stdout}");
    assert!(stdout.contains("Extra rows in B: 2"), "{stdout}");

    let _ = std::fs::remove_file(&a);
    let _ = std::fs::remove_file(&b);
}

#[test]
fn compare_writes_diffs_to_an_output_file_and_prints_a_trailer() {
    let a = write_temp_jsonl(
        "outputfile-a",
        "{\"line\":1,\"input\":\"x\",\"parsed\":{\"rank\":\"SPECIES\",\"genus\":\"Abies\"}}\n",
    );
    let b = write_temp_jsonl(
        "outputfile-b",
        "{\"line\":1,\"input\":\"x\",\"parsed\":{\"rank\":\"SPECIES\",\"genus\":\"Abia\"}}\n",
    );
    let diffs_path = std::env::temp_dir().join(format!(
        "nameparser-cli-compare-test-diffs-{}.txt",
        std::process::id()
    ));

    let output = run_cli(&[
        "compare",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        &format!("--output={}", diffs_path.to_str().unwrap()),
    ]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Per-row diffs written to"), "{stdout}");
    // The per-row dump (the "Line 1 ..." header) must go to the file, not stdout — though the
    // summary's "Top differing fields" section (which also names "parsed.genus") always goes
    // to stdout regardless of --output, so check for the dump's row header specifically.
    assert!(
        !stdout.contains("Line 1 \"x\""),
        "the per-row diff dump should be in the file, not stdout:\n{stdout}"
    );

    let diffs = std::fs::read_to_string(&diffs_path).expect("diffs file should exist");
    assert!(diffs.contains("Line 1 \"x\""), "{diffs}");
    assert!(diffs.contains("parsed.genus"), "{diffs}");

    let _ = std::fs::remove_file(&a);
    let _ = std::fs::remove_file(&b);
    let _ = std::fs::remove_file(&diffs_path);
}

#[test]
fn compare_max_diffs_caps_the_per_row_dump_but_not_the_aggregate_count() {
    let mut a_lines = String::new();
    let mut b_lines = String::new();
    for i in 1..=5 {
        a_lines.push_str(&format!(
            "{{\"line\":{i},\"input\":\"n{i}\",\"parsed\":{{\"rank\":\"SPECIES\",\"genus\":\"A\"}}}}\n"
        ));
        b_lines.push_str(&format!(
            "{{\"line\":{i},\"input\":\"n{i}\",\"parsed\":{{\"rank\":\"SPECIES\",\"genus\":\"B\"}}}}\n"
        ));
    }
    let a = write_temp_jsonl("maxdiffs-a", &a_lines);
    let b = write_temp_jsonl("maxdiffs-b", &b_lines);

    let output = run_cli(&[
        "compare",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        "--max-diffs=2",
    ]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Rows differing:     5 (100.00%)"),
        "{stdout}"
    );
    assert!(
        stdout.contains("further per-row diffs suppressed (--max-diffs=2)"),
        "{stdout}"
    );
    assert_eq!(
        stdout.matches("Line ").count(),
        2,
        "only 2 rows' diffs should be dumped:\n{stdout}"
    );

    let _ = std::fs::remove_file(&a);
    let _ = std::fs::remove_file(&b);
}

#[test]
fn compare_requires_two_positional_files_and_exits_with_code_2() {
    let output = run_cli(&["compare"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty on a usage error"
    );
}

#[test]
fn compare_reports_a_clear_error_when_a_file_is_missing() {
    let b = write_temp_jsonl(
        "missingfile-b",
        "{\"line\":1,\"input\":\"x\",\"parsed\":{}}\n",
    );
    let output = run_cli(&["compare", "/no/such/path/a.jsonl", b.to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("/no/such/path/a.jsonl"),
        "expected the missing path named in the error, stderr was:\n{stderr}"
    );
    let _ = std::fs::remove_file(&b);
}
