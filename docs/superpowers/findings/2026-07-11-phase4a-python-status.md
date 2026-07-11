# Phase 4a: Python (PyO3) Binding Status

**Date:** 2026-07-11
**Status:** COMPLETE — native Python binding built, parity-clean, packaged, typed

## Phase 4a: Python Binding

**Objective:** Ship a native Python binding for the Rust `nameparser` core — `import
nameparser; nameparser.parse(...)` → a typed `ParsedName` — with full field parity to the
core validated over the corpus. Realizes design §6.5 (Python via PyO3 + maturin) and the
polyglot-reach motivation. No ≥N× speed gate was set for this phase (unlike Phase 3's FFM
gate) — the binding is native, so there's no marshalling floor to measure against in the
first place; see "Performance" below.

**Status: COMPLETE**

## Deliverable

`crates/nameparser-py` — a PyO3 `cdylib` (crate name `nameparser-py`, compiled library name
`nameparser`) depending on the core `nameparser` crate by path, natively: no C-ABI, no JSON
wire format, no FFM. `PyParsedName`/`PyAuthorship` wrap the core `ParsedName`/`Authorship`
structs directly; getters map each field to its idiomatic Python type via `.clone()` (plain
scalars) or `pythonize::pythonize` (enums, `Vec`, `BTreeMap`, nested structs — the core enums
are serde-only, with no `.name()`/`Display`).

### API surface

```python
import nameparser

nameparser.parse(name, authorship=None, rank=None, code=None) -> ParsedName        # raises UnparsableNameError
nameparser.parse_all(names, authorship=None, rank=None, code=None) -> list[ParsedName | None]  # never raises

class ParsedName:      # all 30 core fields, one property each (snake_case)
    rank, code, uninomial, genus, generic_authorship, infrageneric_epithet,
    specific_epithet, specific_authorship, infraspecific_epithet, cultivar_epithet,
    phrase, candidatus, notho, original_spelling, epithet_qualifier, type,
    extinct, taxonomic_note, nomenclatural_note, published_in, published_in_year,
    published_in_page, unparsed, doubtful, manuscript, state, warnings,
    combination_authorship, basionym_authorship, sanctioning_author
    def to_dict(self) -> dict: ...     # complete structure, JSON/Java wire field names
    def __repr__(self) -> str: ...

class Authorship:      # combination_authorship / basionym_authorship's type
    authors, ex_authors, year, imprint_year
    def to_dict(self) -> dict: ...
    def __repr__(self) -> str: ...

class UnparsableNameError(Exception):
    name_type: str          # e.g. "OTHER" — mirrors Java UnparsableNameException.getType()
    code: str | None        # e.g. "VIRUS", or None — mirrors .getCode()
    name: str                # mirrors .getName()
    # str(err) is unchanged: the core ParseError's own message, verbatim.
```

Enums (`rank`, `code`, `type`, `state`, `notho` elements, `UnparsableNameError.name_type`/
`.code`) cross as plain SCREAMING_SNAKE_CASE strings (`"SPECIES"`, `"ZOOLOGICAL"`) — the
JSON/Java `.name()` convention — not a Python `enum.Enum`; `rank`/`code` **inputs** accept
the same strings via the core's `Rank::from_name`/`NomCode::from_name`.

### This pass (Task 4): structured `UnparsableNameError` + packaging + docs

Task 3's review flagged an `[Important]` finding: the exception exposed only a message,
unlike Java's `UnparsableNameException.getType()`/`getCode()`/`getName()`, and the corpus
parity test couldn't independently verify `error.code` (present on 6,510/6,609 error rows)
as a result. This pass:

1. **Added `.name_type`/`.code`/`.name`** to `UnparsableNameError` (`lib.rs`'s
   `unparsable_name_error`/`try_attach_error_attrs`): `create_exception!` produces an
   ordinary Python `Exception` subclass with a `__dict__`, so the three values are attached
   via plain `setattr` on the raised instance after construction — `str(err)`/`.args` are
   completely unchanged (still exactly the core message, single-arg tuple). Verified with a
   `pyright`-checked script and a unit test
   (`test_api.py::test_unparsable_name_error_exposes_structured_attributes`, plus a
   `code is None` case).
2. **Extended the corpus parity test** (`test_parity.py`) to assert `exc.name_type ==
   error["type"]` and `exc.code == error.get("code")` against the oracle for every one of
   the ~6,609 error rows in the full corpus, not just the two hand-picked unit-test names —
   closing the Task 3 review gap for real. Still 0 diffs (see below).
3. **`nameparser.pyi` + `py.typed`** — a hand-written type stub (no stub generator in the
   toolchain) covering the full API above, plus a PEP 561 marker. maturin auto-detects both
   at the crate root (next to `Cargo.toml`) and repackages them into the wheel as
   `nameparser/__init__.pyi` / `nameparser/py.typed` inside the auto-generated `nameparser`
   package directory (confirmed by inspecting the built wheel's contents — no explicit
   `[tool.maturin] include` needed; one was tried first and reverted once shown redundant).
   `pyright` against a script exercising the whole surface: **0 errors, 0 warnings**, every
   field/method resolving to its real type, and a genuinely nonexistent attribute
   correctly flagged — the stub is authoritative, not silently falling back to `Any`.
4. **Cleanups** (Task 3 review Minor findings): removed dead `_TYPE_PREFIX`/`import re` and
   the unused `TESTDATA_DIR` import from `test_parity.py`; removed the unused `import pytest`
   from `test_getter_consistency.py`; the `oracle_outcome[1].get(...)` optional-access nit is
   now moot — the error branch was rewritten from scratch for the type/code check and uses
   `oracle_error = oracle_outcome[1] or {}` before any `.get`.
5. **Packaging**: `[project] name = "gbif-name-parser"` in `pyproject.toml` (plain
   `nameparser` is taken on PyPI by an unrelated package) — `[tool.maturin] module-name =
   "nameparser"` unchanged, so `import nameparser` is unaffected either way.

## Parity gate: PASSED

**Zero diffs on real data**, Python `to_dict()` / `UnparsableNameError` vs. the independent
Java `name-parser` oracle (`name-parser-cli-4.2.0-SNAPSHOT-shaded.jar`):

| Corpus | Count | Diffs |
|---|---:|---:|
| benchmark-data.txt | 8,017 | 0 |
| names-with-authors.txt | 14 | 0 |
| hybrids.txt | 4 | 0 |
| other.txt | 13 | 0 |
| otu.txt | 20 | 0 |
| placeholder.txt | 8 | 0 |
| viruses.txt | 3,226 | 0 |
| **TOTAL** | **11,302** | **0** |

`test_parity.py` compares, per name: parsability agreement; when both sides parse,
`to_dict()` field-for-field against the oracle's `parsed` object (`warnings`/`notho`/
`epithetQualifier` order-insensitively — they're backed by a Java `HashSet`/insertion-order
`Vec` respectively — everything else exact); when both fail, `UnparsableNameError`'s
`message` **and, as of this pass,** `name_type`/`code` against the oracle's `error` object.
15/15 tests pass across all three files (`test_api.py`, `test_getter_consistency.py`,
`test_parity.py`) — full command + counts in "Verification" below.

## Performance

Native — no C-ABI, no FFM downcall, no JSON marshalling on the `parse()`/`parse_all()` path
(`to_dict()` is a convenience for parity testing / an escape hatch, not how results are
returned). Unlike Phase 3's Java FFM binding, there is no object-marshalling floor between
Rust and the caller to measure a ratio against in the first place — PyO3's getters read the
already-built Rust struct's fields directly into Python objects on access, with no
intermediate serialization step. No dedicated Python-specific benchmark was run this phase;
the binding inherits the native core's own batch-throughput profile documented in the root
[`benchmarks.md`](../../../benchmarks.md) (`nameparser-cli`, out-of-process: **13.73 µs/name
average, ~2.10× faster than the Java parser's 28.77 µs/name**, p95 2.43×, no JVM/GC/regex-
backtracking tail) — `parse()`/`parse_all()` call the identical `nameparser::parse` that CLI
figure already measures, with PyO3's per-call overhead (GIL acquisition, struct wrapping,
getter dispatch) added on top but no FFI/serialization layer the way the Java path has.
Establishing a precise Python-call-overhead number (e.g. a `pytest-benchmark`/`timeit` A/B
against `NameParserImpl`-via-JPype or similar) is explicitly **not** part of this phase's
scope and is left as follow-on work if a Python-specific figure is ever needed.

## Test summary

```
$ . .venv/bin/activate
$ maturin develop --release -m crates/nameparser-py/Cargo.toml
$ pytest crates/nameparser-py/python/tests/ -v -s
...
15 passed in 1.81s
```

- `test_api.py` (12 tests) — binding-surface unit tests: field/type mapping, `parse_all`'s
  none-on-unparsable contract, `to_dict()` wire-key/string-enum shape, module-qualified
  `__module__`, `__repr__`, and (new this pass) `UnparsableNameError.name_type`/`.code`/
  `.name` for both a coded (virus) and uncoded (bare single letter) rejection.
- `test_getter_consistency.py` (2 tests) — every getter agrees with `to_dict()` over a
  ~50-name representative sample (a coverage gap `test_parity.py` alone can't close, since
  it only ever calls `to_dict()`), plus a pinned check for the two named
  `generic_authorship`/`specific_authorship` examples.
- `test_parity.py` (1 test, the corpus gate) — 11,302 names, 0 diffs, including the new
  `name_type`/`code` check on every one of the corpus's error rows.

`cargo check -p nameparser-py` — clean (only the two pre-existing, documented-benign
`gil-refs`/clippy cosmetic warnings noted in `lib.rs`'s own comments; unrelated to this
pass). No Rust-level `#[test]`s in this crate — all testing is via the Python suite above,
by design (a PyO3 binding's correctness is a Python-observable property).

## Packaging

`maturin build --release -m crates/nameparser-py/Cargo.toml` → a single `abi3` wheel (stable
Python C ABI, `py39` floor — one wheel per platform, not per Python minor version):

```
target/wheels/gbif_name_parser-0.0.0-cp39-abi3-macosx_11_0_arm64.whl
```

(git-ignored, not published). Verified wheel contents (`python -m zipfile -l`):
`nameparser/__init__.py` (maturin-generated `from .nameparser import *` shim),
`nameparser/__init__.pyi` (the stub), `nameparser/nameparser.abi3.so` (the compiled
extension), `nameparser/py.typed`, plus standard `*.dist-info` (`Name: gbif-name-parser`,
`Version: 0.0.0`, an auto-generated CycloneDX SBOM). `import nameparser` is unaffected by
the distribution-name change — only `pip install <name>` / PyPI-facing metadata changed.

## Deferred / Next Phases

### cibuildwheel cross-platform CI wheels + PyPI publish (deferred)

Only a local `aarch64-apple-darwin` wheel has been built and verified this phase. Producing
Linux/Windows/x86_64-macOS wheels (via `cibuildwheel` or `maturin-action` in CI) and an
actual `twine`/`maturin publish` to PyPI under the `gbif-name-parser` name are both
explicitly out of scope here and left as follow-on infrastructure work — no CI workflow
exists yet for this crate.

### Phase 4b: R (extendr) binding — next

The R binding is a separate slice, reusing the same core crate the way this Python binding
and the Phase 3 Java binding both do (a new `crates/nameparser-r` via
[`extendr`](https://extendr.github.io/), analogous to `nameparser-py`'s PyO3 shape).
**Correction to this task's own brief:** it stated `rextendr` was already installed; checked
directly on this machine (`Rscript -e 'installed.packages()'`) and it is **not** — only base
R itself is present (`/opt/homebrew/bin/R`, R 4.6.1, via Homebrew). Installing `rextendr`
(`install.packages("rextendr")`, which also needs a working `cargo`/Rust toolchain — already
present in this repo — and `usethis`) is Phase 4b's own first step, not a completed
prerequisite.

## Summary

Phase 4a delivers a production-shaped native Python binding: zero-diff corpus parity
(11,302 names, including — as of this pass — the `UnparsableNameError.name_type`/`.code`
attributes, not just its message) against the independent Java oracle, a fully typed public
API (`nameparser.pyi` + `py.typed`, pyright-clean), and a locally-verified `abi3` wheel under
the `gbif-name-parser` PyPI name. No C-ABI/FFM marshalling floor exists on this path, so
(unlike Phase 3) there was no speed gate to fall short of — the binding calls the core
directly and inherits its native batch-throughput numbers. Remaining work before a public
release is purely packaging infrastructure (cross-platform CI wheels, PyPI publish), tracked
as deferred above; Phase 4b (R/extendr) is next, with `rextendr` itself still to be
installed.
