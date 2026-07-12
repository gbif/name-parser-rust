# `nameparser-py` — native Python binding

`import nameparser` gives you the Rust `nameparser` core's `parse()` directly, as a
[PyO3](https://pyo3.rs) extension module. Unlike `bindings/java` (which crosses the JVM/native
boundary via `java.lang.foreign`, either JSON or a flat struct on the wire), there is **no
C-ABI or marshalling floor here**: PyO3 wraps the core `nameparser::model::ParsedName` struct
directly, with one getter per field mapping it straight to its idiomatic Python type — no JVM,
no JSON round-trip, no separately-installed native library to locate at runtime.

Field parity with the core is validated over this repo's full ~11,302-name test corpus,
cross-checked against the independent Java `name-parser` oracle: **0 diffs** (see
`crates/nameparser-py/python/tests/test_parity.py` and
`docs/superpowers/findings/2026-07-11-phase4a-python-status.md`).

## Install

Not yet published to PyPI. Until then, build from source with
[maturin](https://www.maturin.rs) inside this repo's project-local venv:

```sh
python3 -m venv .venv && . .venv/bin/activate
pip install maturin
maturin develop --release -m crates/nameparser-py/Cargo.toml
```

`maturin develop` compiles the Rust cdylib and installs it into the active virtualenv as an
editable `nameparser` package (including the `nameparser.pyi` type stub and a `py.typed`
marker — this package is fully typed; see [Editors / type checkers](#editors--type-checkers)
below). Once published, the same package will install with:

```sh
pip install gbif-name-parser   # → import nameparser
```

(The PyPI **distribution** name is `gbif-name-parser` — matching the Rust core crate on
crates.io, so it's one name across both registries — but the **importable module** stays
`nameparser` either way; see `pyproject.toml`.)

## Usage

```python
import nameparser

pn = nameparser.parse("Vulpes vulpes silaceus Miller, 1907")
print(pn.rank, pn.genus, pn.specific_epithet, pn.infraspecific_epithet)   # SUBSPECIES Vulpes vulpes silaceus
print(pn.combination_authorship.authors, pn.combination_authorship.year)  # ['Miller'] 1907
print(pn.to_dict()["type"])                                               # SCIENTIFIC — full dict, wire (JSON/Java) field names

results = nameparser.parse_all(["Abies alba", "Tobacco mosaic virus"])    # batch: never raises
print(results)                                                            # [ParsedName(...), None]

try:
    nameparser.parse("Tobacco mosaic virus")
except nameparser.UnparsableNameError as e:
    print(e.name_type, e.code, e.name)   # OTHER VIRUS Tobacco mosaic virus
    print(str(e))                        # Unparsable OTHER name: Tobacco mosaic virus
```

Every one of `ParsedName`'s 30 core fields is exposed as a Python property (snake_case, e.g.
`specific_epithet`, `combination_authorship`, `published_in_year`); enum-typed fields (`rank`,
`code`, `type`, `state`, and each element of `notho`) come across as plain SCREAMING_SNAKE_CASE
strings (`"SPECIES"`, `"ZOOLOGICAL"`, …) — the same convention the JSON/Java wire format uses —
not a Python `enum.Enum`. `rank`/`code` **inputs** to `parse()`/`parse_all()` accept those same
strings (or `None`). `to_dict()` (on both `ParsedName` and the nested `Authorship`) returns the
complete structure straight from the core's own `serde::Serialize` impl, keyed by the JSON/Java
wire field names (`specificEpithet`, not `specific_epithet`) — the escape hatch for anything a
typed getter doesn't surface, and the parity oracle the corpus test itself diffs against.

`UnparsableNameError.name_type`/`.code`/`.name` mirror Java's
`org.gbif.nameparser.api.UnparsableNameException`'s `getType()`/`getCode()`/`getName()`; `str(e)`
is still exactly the core's own message, unchanged.

## Editors / type checkers

`nameparser.pyi` (shipped inside the package, alongside a `py.typed` marker) gives Pyright/mypy/
your editor full attribute-level types for `parse`/`parse_all`/`ParsedName`/`Authorship`/
`UnparsableNameError` — no more "unknown attribute" noise on `pn.specific_epithet` or
`e.name_type`. It's hand-written (the compiled extension has no Python source for a stub
generator to read) and kept in sync by hand with `src/lib.rs`'s getters; see its own module
docstring.

## Native, no JDK required

This binding never starts a JVM and needs no `java`/`JAVA_HOME` on the machine that runs it —
unlike `bindings/java` (which requires JDK 22+ for `java.lang.foreign`) — because it compiles
directly against the Rust core, in-process, with PyO3 doing the Rust↔Python marshalling at the
Rust/CPython C-API level rather than crossing a JVM boundary at all. Performance-wise it
inherits the native CLI's batch-throughput profile (see the root [`BENCHMARKS.md`](../../BENCHMARKS.md)):
there is no per-call FFM downcall or JSON re-serialization step the way the Java binding's
`NameParserRust` has, since `parse()`/`parse_all()` call `nameparser::parse` directly.

## Development

```sh
. .venv/bin/activate
maturin develop --release -m crates/nameparser-py/Cargo.toml
pytest crates/nameparser-py/python/tests/ -v
```

Three test files under `python/tests/`:

- `test_api.py` — unit tests for the binding surface itself (getters, `to_dict()`,
  `parse_all()`'s none-on-unparsable contract, `UnparsableNameError`'s structured attributes).
- `test_getter_consistency.py` — checks every `#[getter]` agrees with `to_dict()` over a
  representative sample (a path `test_parity.py` alone can't exercise, since it only ever
  calls `to_dict()`).
- `test_parity.py` — the corpus parity gate: diffs `nameparser.parse(name).to_dict()`, and
  `UnparsableNameError`'s message/`name_type`/`code`, against an independent Java-oracle (or
  Rust-CLI-fallback) row for every one of the ~11,302 corpus names. Needs a JDK on `PATH`
  (falls back to the release `nameparser-cli` binary if the Java shaded jar isn't available).

Build a wheel (not published) with:

```sh
maturin build --release -m crates/nameparser-py/Cargo.toml   # → target/wheels/*.whl (git-ignored)
```

The wheel is `abi3` (built against the stable Python C ABI, `py39` floor) — one wheel per
platform, working across Python 3.9+, not one per Python minor version.
