# Phase 1 + Phase 2 Consolidation Status

**Date:** 2026-07-11  
**Status:** COMPLETE — both phases shipped and verified

## Phase 1: Core Parser Port

**Objective:** Full fidelity Rust implementation of the Java `NameParserImpl` pipeline.

**Status: COMPLETE**

The entire parser pipeline is ported and functionally equivalent to the Java original:

- **Full pipeline:** Preflight validation → StripAndStash (55 transformation steps) → Tokenizer → AuthorshipSplit → NameTokens → AuthorshipParser → CodeInference → Assemble → homoglyph replacement
- **Rank enum:** All 117 rank constants ported
- **Parsed field parity:** All 30 `ParsedName` output fields at **zero mismatches** across 8,017 test names (verified via `parse_golden.rs` golden-file test corpus with empty allowlist)
- **Test coverage:** 627 core parser tests passing

### Key verification

The Phase 1 parity claim is end-to-end verified:
- `tests/parse_golden.rs` runs all 8,017 names from the benchmark dataset through the Rust parser and compares each of the 30 `ParsedName` fields to the pre-recorded Java baseline. Mismatch allowlist is empty.
- This corpus is identical to what the Java `NameParserImplTest` suite uses, so Phase 1 inherits all the Java test assertions' coverage.

## Phase 2: Native CLI + Cross-Validation + Benchmark

**Objective:** Production-ready CLI with byte-identical output to the Java CLI, cross-validated against 11,302 diverse names, and full-pipeline performance measurement.

**Status: COMPLETE**

### CLI implementation (`nameparser-cli` crate)

- **Commands:** `parse` (JSONL output, byte-identical to Java), `benchmark`, `compare` (set-insensitive warnings-order diffing)
- **Architecture:** clap isolated to the CLI crate; core parser has zero CLI dependencies
- **Output parity:** JSONL output is byte-identical to the Java CLI (modulo set-identical warnings-order differences, handled by `compare`)

### Benchmark results

Full-pipeline performance, same machine, release build, 3 repeats, identical parse counts per type:

| Metric | Rust | Java | Speedup |
|--------|------|------|---------|
| **Avg (µs/name)** | 13.73 | 28.77 | **2.1×** |
| **p50 (µs/name)** | 14.46 | 24.79 | 1.7× |
| **p95 (µs/name)** | 32.29 | 78.50 | **2.4×** |

**Note:** The Phase 2 spike deferred the full-pipeline measurement; this is the first end-to-end benchmark and represents the real production speedup.

### Cross-validation

Rust CLI vs Java CLI output:

| Corpus | Count | Match | Parity |
|--------|-------|-------|--------|
| benchmark-data.txt | 8,017 | 8,017 | 100% |
| names-with-authors | 14 | 14 | 100% |
| hybrids | 4 | 4 | 100% |
| other | 13 | 13 | 100% |
| otu | 20 | 20 | 100% |
| placeholder | 8 | 8 | 100% |
| viruses | 3,226 | 3,226 | 100% |
| **TOTAL** | **11,302** | **11,302** | **100%** |

**Residual:** 5 rows with set-identical warnings (Java `HashSet` vs Rust `Vec` iteration order); `compare` command handles this with order-insensitive set comparison.

**Zero core bugs found** during cross-validation. All discrepancies are warnings-order only.

### Test coverage

- Core + CLI: **678 total tests passing**

## Deferred / Next Phases

The following are explicitly deferred and will be addressed in later phases:

### Not yet available

- **`col-names.tsv` (6.3M rows):** The full-scale cross-validation requires the user to drop this file into `testdata/`; current cross-validation is 11,302 diverse names.

### Deferred features

- `validate` CLI command (offline LLM audit of parsed fields)
- CSV, TSV, JSON `parse` output formats (JSONL is the parity baseline)
- ColDP auto-detection for input parsing

### Planned phases (per design roadmap)

- **Phase 3:** Java FFM (Panama) binding (`nameparser-ffi` cdylib + `NameParserRust implements NameParser`; requires Java 22+)
- **Phase 4:** Python (PyO3) and R (extendr) bindings
- **Phase 5:** Backend cutover + retire Java pipeline

## Summary

Phase 1 + Phase 2 are production-ready. The Rust implementation achieves full field-parity with the Java original, ships a native CLI with byte-identical output, demonstrates 2.1× average speedup and 2.4× tail-latency improvement, and has been cross-validated against 11,302 diverse names with zero core bugs.
