// SPDX-License-Identifier: Apache-2.0

//! Integration test: `nameparser-cli benchmark` runs end-to-end through the *compiled binary*
//! and produces a report shaped like the Java CLI's `BenchmarkCli` output (see that class's doc
//! comment, and `crates/nameparser-cli/src/main.rs`'s `benchmark` section, for the exact
//! behavioural contract this reproduces). The concrete timing numbers are inherently
//! machine-dependent and are deliberately NOT asserted here — this only proves the command
//! succeeds and prints every section/shape the Java report has (count/total/avg/min/p50/p95/max
//! plus the by-name-type breakdown), that the report goes to stdout and nothing else does, and
//! that a missing input file fails the way Java's does. The exact numbers-in-anger, and the
//! actual Rust-vs-Java throughput comparison, are a one-off measurement recorded in
//! `benchmarks.md` at the repo root — not something a fast test suite should pin.
//!
//! The pure formatting/math helpers (`percentile`, `fmt_nanos`, `name_type_label`, the exact
//! `BenchmarkReport::print` layout) already have dedicated unit tests in `src/main.rs`; this
//! file only covers what those can't: the real binary, real file I/O, and process exit codes.

use std::process::Command;

const CORPUS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/benchmark-data.txt"
);
const WORKSPACE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn run_cli(cli_args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_nameparser-cli"))
        .args(cli_args)
        .output()
        .expect("failed to run nameparser-cli")
}

/// Independently recomputes how many rows `benchmark` should report parsing: every line of
/// `path` that is non-empty, doesn't start with `#`, and isn't blank after trimming. This is the
/// same rule as `BenchmarkCli.run`/`main.rs`'s `extract_benchmark_name`, re-derived here (rather
/// than calling into the binary's internals) so this test doesn't just check the implementation
/// against itself.
fn expected_benchmark_row_count(path: &str) -> usize {
    std::fs::read_to_string(path)
        .expect("corpus must be readable")
        .lines()
        .filter(|raw| !raw.is_empty() && !raw.starts_with('#') && !raw.trim().is_empty())
        .count()
}

#[test]
fn benchmark_over_the_full_corpus_produces_a_report_shaped_like_javas() {
    assert!(
        std::path::Path::new(CORPUS).exists(),
        "corpus {CORPUS} not found"
    );
    let expected_count = expected_benchmark_row_count(CORPUS);

    // The exact invocation this task measures Rust against Java with.
    let output = run_cli(&["benchmark", "--warmup", &format!("--input={CORPUS}")]);
    assert!(
        output.status.success(),
        "nameparser-cli exited with {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout must be valid UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr must be valid UTF-8");

    assert!(
        stdout.starts_with(&format!("Parsed names: {expected_count} (")),
        "unexpected report header, stdout was:\n{stdout}"
    );
    for label in [
        "Total:",
        "Average:",
        "Min:",
        "p50:",
        "p95:",
        "Max:",
        "Breakdown by name type:",
    ] {
        assert!(
            stdout.contains(label),
            "missing {label:?} in report:\n{stdout}"
        );
    }
    // This corpus is known (from the core crate's own golden harness) to parse mostly as
    // SCIENTIFIC — spot check the breakdown carries at least that one row, without pinning the
    // exact counts (which belong to benchmarks.md, not a correctness test).
    assert!(
        stdout.contains("SCIENTIFIC"),
        "expected a SCIENTIFIC row in the breakdown:\n{stdout}"
    );

    // Report goes to stdout only; the warmup banner is stderr-only, never mixed into stdout.
    assert!(
        stderr.contains("Warming up"),
        "expected a warmup banner on stderr:\n{stderr}"
    );
    assert!(
        !stdout.contains("Warming up"),
        "warmup banner leaked onto stdout:\n{stdout}"
    );
}

#[test]
fn benchmark_resolves_its_default_input_relative_to_the_current_directory() {
    // No --input at all: exercises DEFAULT_BENCHMARK_INPUT ("testdata/benchmark-data.txt"),
    // meaningful only relative to the workspace root — matches how this repo's own README-style
    // invocation (`cd name-parser-rust && .../nameparser-cli benchmark`) is actually run.
    let output = Command::new(env!("CARGO_BIN_EXE_nameparser-cli"))
        .arg("benchmark")
        .current_dir(WORKSPACE_ROOT)
        .output()
        .expect("failed to run nameparser-cli");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout must be valid UTF-8");
    assert!(
        stdout.starts_with("Parsed names: "),
        "stdout was:\n{stdout}"
    );
}

#[test]
fn benchmark_reports_input_not_found_and_exits_with_code_2() {
    let output = run_cli(&["benchmark", "--input=/no/such/path/benchmark-data.txt"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Input not found"),
        "expected an 'Input not found' message, stderr was:\n{stderr}"
    );
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty on a missing-input error"
    );
}
