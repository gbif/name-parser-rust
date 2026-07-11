# Phase 4b: R (extendr) Binding Status

**Date:** 2026-07-11
**Status:** COMPLETE — native R binding built, parity-clean, documented

## Phase 4b: R Binding

**Objective:** Ship a native R binding for the Rust `nameparser` core — a vectorized
`parse_names(scientificname)` returning a tibble, fitting the `rgbif` ecosystem, plus a
lossless `parse_name_json()` escape hatch that doubles as the cross-validation parity
oracle — with full field parity to the core validated over the corpus. Realizes design §6.5
(R via extendr) and the polyglot-reach motivation, mirroring Phase 4a's Python binding.

**Status: COMPLETE**

## Deliverable

`bindings/r` — a self-contained R package `nameparser` (mirroring the self-contained
`bindings/java/` Maven module), built with [extendr](https://extendr.rs) 0.9 +
[rextendr](https://extendr.rs/rextendr/) 0.5.0. Its Rust crate (`bindings/r/src/rust/`) is
detached from the Cargo workspace (own empty `[workspace]` table) and depends on the core
`nameparser` crate (published name `scientific-name-parser`) by relative path, aliased
`nameparser_core` to avoid an extern-name collision with the binding crate's own `[lib] name
= 'nameparser'` (required for R's `useDynLib(nameparser)`).

### API surface

```r
library(nameparser)

parse_names(scientificname, authorship = NULL, rank = NULL, code = NULL) -> tibble
# one row per input name, NEVER throws: parsed = FALSE + NA atoms on an unparsable name.
# 35 columns: scientificName, parsed, error, type, rank, code, uninomial, genus,
# infragenericEpithet, specificEpithet, infraspecificEpithet, cultivarEpithet, phrase,
# candidatus, notho, originalSpelling, epithetQualifier, extinct, taxonomicNote,
# nomenclaturalNote, publishedIn, publishedInYear, publishedInPage, unparsed, doubtful,
# manuscript, state, combinationAuthors, combinationExAuthors, combinationYear,
# basionymAuthors, basionymExAuthors, basionymYear, sanctioningAuthor, warnings.

parse_name_json(name, authorship = NULL, rank = NULL, code = NULL) -> character(1)
# lossless: the core's own serde JSON for one name, byte-identical to the CLI's `parsed`
# object on success, or {"error":{"type",["code"],"message"}} on failure (code omitted,
# not null, when absent -- byte-identical to nameparser-ffi's unparsable_json/nameparser-cli's
# render_row).
```

Enums (`rank`, `code`, `type`, `state`, `notho` elements) cross as plain
`SCREAMING_SNAKE_CASE` strings (`"SPECIES"`, `"ZOOLOGICAL"`) via the core's own serde
representation — no hand-maintained enum-name table, matching the Python binding's
`pythonize` and the JSON wire format. `rank`/`code` **inputs** accept the same strings via
the core's `Rank::from_name`/`NomCode::from_name`.

Only `parse_names`/`parse_name_json` are exported (`NAMESPACE`); the low-level Rust-backed
`parse_names_impl`/`parse_name_json_impl` are package-internal (see "This pass" below).

## This pass (Task 4): parity gate + fold-in fixes from the Task 2/3 reviews + docs

Tasks 1–3 built the toolchain spike, the full 35-column scalar/enum surface, and the
authorship/warnings flattening + `parse_name_json()`. This pass:

1. **Fixed `parse_name_json()`'s error envelope** (`[Important]` from the Task 3 review): it
   previously built the `Err` case via `serde_json::json!`, which produced
   `{"code":null,"message":...,"name":...,"type":...}` — alphabetical keys (no
   `preserve_order`), a spurious `name` key, and `"code":null` instead of omitting `code`
   when absent. Rewrote it as a hand-assembled `unparsable_json` helper — a direct port of
   `crates/nameparser-ffi/src/lib.rs`'s private function of the same name — producing
   `{"error":{"type":...,["code":...,]"message":...}}` byte-for-byte, matching the FFI/CLI
   error shape exactly. The success branch (already a verified byte-identical oracle) is
   unchanged.
2. **Made `parse_names_impl`/`parse_name_json_impl` package-internal.** Both were previously
   `@export`ed via a bare `/// @export` Rust doc comment, making them exported-but-largely-
   undocumented (roxygen2 had silently dropped `parse_names_impl.Rd` for lack of a title) —
   an `R CMD check` NOTE waiting to happen. Removed `/// @export` from both; `NAMESPACE` now
   exports only `parse_names`/`parse_name_json` (the hand-written tibble/JSON wrappers in
   `R/parse.R`, which still call the impls internally — ordinary same-package visibility,
   no export needed). Gave `parse_names_impl` a real (non-exported) doc comment while at it,
   so `rextendr::document()` now emits a clean `parse_names_impl.Rd` too instead of silently
   dropping it.
3. **`DESCRIPTION` `Version`: `0.0.0.9000` → `0.1.0`**, aligning with the core
   `scientific-name-parser` crate and the Python distribution, both already at `0.1.0`.
4. **Closed a test-coverage gap**: the committed suite exercised `combinationAuthors`/
   `combinationYear`/`basionymAuthors` (NA case) and merely checked `"warnings" %in%
   names(out)`, but never a populated `combinationExAuthors`, `basionymExAuthors`,
   `basionymYear`, `sanctioningAuthor`, or a genuinely non-empty `warnings` value. Added
   targeted assertions for all five, using inputs verified interactively first (not
   guessed): `"Abies alba Wedd. ex Sch. Bip."` (combination-level ex-author split),
   `"Abies alba (Wedd. ex Sch. Bip.) Rehder"` (basionym-level ex-author split),
   `"Abies alba (Wang & Liu, 1996) Rehder"` (basionym year), `"Agaricus arvensis L. : Fr."`
   (fungal sanctioning colon), and `"Buteo borealis ? ventralis"` (a real
   `"question marks removed"` warning + `doubtful = TRUE`) — all fixtures traceable to the
   core crate's own unit tests.
5. **Documented (not added) the `errorCode` limitation** — see "Known limitations" below.

## Parity gate: PASSED

**Zero mismatches over the full benchmark corpus**, `parse_name_json()` vs. the same
independent Java oracle (`name-parser-cli-4.2.0-SNAPSHOT-shaded.jar`,
`testdata/expected-parse.jsonl`) the native CLI and Python binding validate against:

| Corpus | Count | Parsed (oracle) | Error (oracle) | Mismatches |
|---|---:|---:|---:|---:|
| benchmark-data.txt | 8,017 | 4,672 | 3,345 | **0** |

`tests/testthat/test-parity.R` compares, per name: for rows the oracle parsed, the full
`parsed` object (warnings-set + key-order normalized, since Java's `warnings` is a
`HashSet<String>` — no other field needs order-insensitive comparison, because
`jsonlite::toJSON`'s own re-serialization is what's diffed, not the raw wire bytes); for
rows the oracle errored, that our side errored too (a presence check, not a shape check —
the corrected `Err`-envelope shape itself is separately pinned byte-for-byte by dedicated
assertions in `test-parse-names.R`, see "This pass" item 1 above).
The oracle file is git-ignored and regenerated locally (`crates/nameparser/tests/
parse_golden.rs`'s module doc has the regeneration command); it was present in this
environment, so the gate ran for real — not a skip — 3.3s wall time for all 8,017 names.

## Performance

Native — `#[extendr]` functions return Rust values directly into R's C API (`extendr-api`'s
own marshalling), with no C-ABI cdylib, no JSON wire format, and no FFM-style downcall in
the `parse_names()` path (`parse_name_json()` does serialize to a JSON string, but only
because that is its deliberately lossless/oracle-comparable contract, not because the
binding needs JSON internally). As with the Python binding, there is no FFI-marshalling
floor to measure a ratio against — no dedicated R-specific benchmark was run this phase;
throughput inherits the core's own batch-throughput profile documented in the root
[`benchmarks.md`](../../../benchmarks.md). Establishing an R-specific call-overhead number
(e.g. `bench::mark()` against `rgbif::name_parse()`'s HTTP-backed implementation, which is
not an apples-to-apples comparison anyway since that function is a *web service* client, not
an in-process parser) is left as follow-on work if ever needed.

## Test summary

```
$ source "$HOME/.cargo/env"
$ Rscript -e 'rextendr::document("bindings/r"); devtools::load_all("bindings/r", quiet=TRUE); testthat::test_dir("bindings/r/tests/testthat")'
...
[ FAIL 0 | WARN 0 | SKIP 0 | PASS 41 ]
```

- `test-parse-names.R` (39 assertions across 7 `test_that` blocks) — scalar/enum columns,
  `NA` handling on unparsable rows, the full authorship/ex-author/basionym-year/sanctioning-
  author flattening (including this pass's 5 newly-covered columns), `parse_name_json()`'s
  `Ok` shape, and (new this pass) `parse_name_json()`'s corrected `Err` shape.
- `test-parity.R` (1 test, 2 assertions, the corpus gate) — 8,017 names, 0 mismatches.

`cargo clippy --lib --manifest-path=bindings/r/src/rust/Cargo.toml` — clean, 0 warnings.

## Known limitations / deferrals

- **`errorCode` is not a `parse_names()` tibble column.** An unparsable row surfaces `type`
  (`NameType`) and `error` (the message) but not `ParseError.code` (`NomCode`, e.g.
  `"VIRUS"`) — adding a 36th column, and a 6th parallel list entry across `Cols`/
  `with_capacity`/`push_ok`/`push_err`/`List::from_names_and_values`, for one `Err`-only
  field was judged out of scope for this phase. `parse_name_json()` carries it losslessly
  already (verified: `parse_name_json("Tobacco mosaic virus")` includes `"code":"VIRUS"`).
- **`genericAuthorship`/`specificAuthorship` are JSON-only** — the niche botanical
  `CombinedAuthorship` bundles are not flattened into tibble columns; use
  `parse_name_json()`.
- **No assembled canonical `authorship` string** — only the parsed parts are exposed
  (`combinationAuthors`, `combinationYear`, ...); assembling one display-ready string is
  deferred pending a core `NameFormatter` (not yet ported from the Java library).
- **Not on CRAN** — needs the whole dependency graph (the core parser path-dependency plus
  its own crates.io tree) vendored into the source tarball for CRAN's offline-build policy.
  Today's install path is a local checkout or `remotes::install_github()`.

All four are documented in `bindings/r/README.md`, not silently left as gaps.

## Summary

Phase 4b delivers a production-shaped native R binding: zero-mismatch corpus parity (8,017
names) against the same independent Java oracle the CLI and Python binding validate
against, a clean public surface (`NAMESPACE` exports exactly `parse_names`/
`parse_name_json`; the two Rust-backed impl functions are package-internal), a `0.1.0`
version aligned with the core crate and the Python distribution, and closed test-coverage
gaps on the authorship/warnings columns. Phase 4 (Python + R bindings) is now complete;
Phase 5 (backend cutover) is next.
