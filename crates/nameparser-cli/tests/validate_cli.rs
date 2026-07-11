// SPDX-License-Identifier: Apache-2.0

//! Integration test: `nameparser-cli validate` end-to-end through the *compiled binary* —
//! complements the in-process reproducibility test in `src/validate.rs`'s own `#[cfg(test)]`
//! module (which calls `run_validate`/`select` directly) with two things that genuinely need a
//! real subprocess:
//!   - real command-line argument parsing (a typo'd `#[arg(long)]` name would compile fine and
//!     still pass an in-process test that builds `ValidateArgs` by hand, but would break here);
//!   - the `--input`-missing fail-fast path, which calls `std::process::exit(2)` — calling that
//!     in-process would kill the whole test-runner process rather than just failing one test.

use std::path::PathBuf;
use std::process::Command;

/// A small, already-committed, non-trivial corpus (mixed authorships, a couple of names with
/// year/date oddities likely to carry warnings) — good enough for an end-to-end smoke test
/// without needing a new fixture file.
const CORPUS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/names-with-authors.txt"
);

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_nameparser-cli"))
        .args(args)
        .output()
        .expect("failed to run nameparser-cli")
}

/// A process/time-unique path under the system temp dir — avoids a temp-file crate dependency
/// for a couple of small, self-cleaning fixtures.
fn temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "nameparser-cli-validate-cli-test-{label}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
    ))
}

#[test]
fn validate_dry_run_fails_fast_with_exit_2_when_input_is_missing() {
    let missing = temp_path("missing-input.txt");
    let output = run_cli(&[
        "validate",
        &format!("--input={}", missing.display()),
        "--dry-run",
    ]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "stderr: {stderr}");
    assert!(stderr.contains("Input not found"), "stderr was: {stderr}");
    // Matches Java's actionable second line (`ValidateCli.main`'s "col-names.tsv is a large,
    // gitignored, user-supplied file..." hint) — proves the full message, not just the exit
    // code, made it to stderr.
    assert!(
        stderr.contains("--input=PATH"),
        "expected the actionable hint mentioning --input=PATH, got: {stderr}"
    );
}

#[test]
fn validate_dry_run_runs_end_to_end_over_a_committed_corpus() {
    assert!(
        std::path::Path::new(CORPUS).exists(),
        "corpus {CORPUS} not found"
    );
    let report = temp_path("report.jsonl");

    let output = run_cli(&[
        "validate",
        &format!("--input={CORPUS}"),
        &format!("--output={}", report.display()),
        "--dry-run",
        "--cache=none",
        "--budget=10",
        "--sample-normal=5",
        "--seed=17",
    ]);

    assert!(
        output.status.success(),
        "nameparser-cli exited with {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Scanned"), "stderr was: {stderr}");
    assert!(stderr.contains("Dry run: built"), "stderr was: {stderr}");
    assert!(stderr.contains("no API calls made"), "stderr was: {stderr}");

    // Task 3: the exact first-batch LLM request payload is also dumped to stderr (Java's
    // `dumpFirstBatch`) — verified here via the real compiled binary, complementing the
    // in-process `user_message` shape tests in `src/validate.rs`.
    assert!(
        stderr.contains("--- first batch payload (dry run) ---"),
        "stderr was: {stderr}"
    );
    let (_, payload) = stderr
        .split_once("--- first batch payload (dry run) ---\n")
        .expect("payload header must be followed by the payload");
    let (header, json_part) = payload
        .split_once('\n')
        .expect("payload has a header line then a JSON array line");
    assert!(
        header.starts_with("Judge each of the following ") && header.ends_with(" parser results."),
        "unexpected payload header: {header}"
    );
    let arr: serde_json::Value = serde_json::from_str(json_part.trim_end())
        .unwrap_or_else(|e| panic!("payload JSON array must parse: {e}\n{json_part}"));
    assert!(arr.is_array(), "payload body must be a JSON array: {arr}");
    for item in arr.as_array().unwrap() {
        assert!(item.get("index").is_some(), "item missing 'index': {item}");
        assert!(item.get("input").is_some(), "item missing 'input': {item}");
        assert!(
            item.get("parsed").is_some() || item.get("unparsable").is_some(),
            "item missing both 'parsed' and 'unparsable': {item}"
        );
        assert!(
            item.get("canonical").is_none(),
            "'canonical' must be omitted (deferred): {item}"
        );
    }

    let report_text = std::fs::read_to_string(&report).expect("report file must exist");
    assert!(!report_text.trim().is_empty(), "report must not be empty");
    for line in report_text.lines() {
        let row: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("invalid JSON row {line:?}: {e}"));
        assert!(row.get("line").is_some(), "row missing 'line': {row}");
        assert!(row.get("input").is_some(), "row missing 'input': {row}");
        assert!(
            row.get("parsed").is_some() || row.get("error").is_some(),
            "row missing both 'parsed' and 'error': {row}"
        );
        assert!(
            row.get("verdict").is_none(),
            "no verdict field yet (Task 2 has no cache/judge): {row}"
        );
    }

    let _ = std::fs::remove_file(&report);
}
