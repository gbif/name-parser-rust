# Phase 4a — Python (PyO3) binding

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use `- [ ]`.

**Goal:** Ship a native Python binding for the Rust name-parser core — `import nameparser; nameparser.parse("Abies alba Mill.")` → a typed `ParsedName` object — built as a wheel via maturin, with full field parity to the core validated over the corpus. Realizes design §6.5 (Python via PyO3 + maturin) and the #1 motivation (polyglot reach). The R (extendr) binding is a separate slice (Phase 4b).

**Architecture:** A new `crates/nameparser-py` crate (`cdylib`, PyO3) wrapping `nameparser::parse` **natively** — no C-ABI, no FFM marshalling floor (unlike the Java path). `parse(...)` returns a `#[pyclass] ParsedName` exposing every core field via getters (enums as their serialized string names, author/warning lists as `list[str]`, nested authorships as `Authorship` objects); unparsable names raise `UnparsableNameError`. A `to_dict()` (via `pythonize` over the core's `serde::Serialize`) gives the complete structure and makes corpus parity testing trivial. Built + tested in a project-local `.venv` with `maturin develop`.

**Tech Stack:** Rust 2021, `pyo3` 0.22 (`extension-module` + `abi3-py39` features, for Python-3.9+ forward-compatible wheels), `pythonize` (serde → Python), `maturin` (build backend); Python 3.13 in a local `.venv`; `pytest` for the parity test.

## Global Constraints

- **Bind the core NATIVELY.** Depend on `nameparser` (path); reuse `nameparser::parse` + the `pub` `ParsedName`/`Authorship`/`CombinedAuthorship` + `Rank::from_name`/`NomCode::from_name`. **Do not modify the core crate** (everything needed is already `pub`). No JSON on the hot path — `to_dict()` is a convenience, not how `parse` returns.
- **The `.venv` and build artifacts are git-ignored** — add `/.venv/`, `crates/nameparser-py/target/` is covered by the workspace `/target`. Never commit a venv or a wheel.
- **Faithful field surface:** the Python `ParsedName` must expose ALL 30 core fields (below). The corpus parity test (Task 3) is the completeness gate — a missing/mismapped field shows up as a diff.
- **Enums cross as their serialized string names** (e.g. `"SPECIES"`, `"ZOOLOGICAL"`, `"SUBSPECIES"`), matching the JSON/Java `.name()` convention — NOT Python enums (keep it simple + string-comparable). `rank`/`code` inputs to `parse` accept the same strings (via `from_name`) or `None`.
- **IMPORTANT — the core enums have NO `Display`/`.name()` method.** They are serde-only (`#[serde(rename_all="SCREAMING_SNAKE_CASE")]`, verified). So render enum, `notho` (`Vec<NamePart>`), `epithet_qualifier` (`BTreeMap<NamePart,String>`) and nested-authorship fields to Python via **`pythonize(py, &self.inner.<field>)`** — it maps any serde-Serialize value to the idiomatic Python type (enum→`str`, `Option`→`None`/value, `Vec`→`list`, `BTreeMap`→`dict`, struct→`dict`). Do NOT call a `.name()` that doesn't exist, and do NOT add one to the core. Plain scalar fields (`Option<String>`, `bool`, `Option<i32>`) use direct `.clone()` getters (native `str`/`None`/`bool`/`int`) — no pythonize needed.
- SPDX header (`// SPDX-License-Identifier: Apache-2.0`) on Rust files; crate version `0.0.0`.
- **Working dir** `/Users/markus/code/gbif/name-parser-rust/`. Preamble: `export PATH="$HOME/.cargo/bin:$PATH"`. Python is `python3` (3.13) on PATH.

## The core surface (verified — the binding maps exactly these)

```rust
pub fn parse(name: &str, authorship: Option<&str>, rank: Option<Rank>, code: Option<NomCode>)
    -> Result<ParsedName, ParseError>          // ParseError has: type_ (NameType), a code, a message

pub struct ParsedName {                        // 30 fields — all pub
  rank: Rank, code: Option<NomCode>,
  uninomial/genus/infrageneric_epithet/specific_epithet/infraspecific_epithet/cultivar_epithet/phrase: Option<String>,
  generic_authorship/specific_authorship: Option<CombinedAuthorship>,
  candidatus/extinct/doubtful/manuscript: bool,
  notho: Option<Vec<NamePart>>, original_spelling: Option<bool>,
  epithet_qualifier: Option<BTreeMap<NamePart,String>>,
  type_: NameType, state: State,
  taxonomic_note/nomenclatural_note/published_in/published_in_page/unparsed/sanctioning_author: Option<String>,
  published_in_year: Option<i32>,
  warnings: Vec<String>,
  combination_authorship/basionym_authorship: Authorship,
}
pub struct Authorship { authors: Vec<String>, ex_authors: Vec<String>, year: Option<String>, imprint_year: Option<String> }
pub struct CombinedAuthorship { combination_authorship: Authorship, basionym_authorship: Authorship, sanctioning_author: Option<String> }
// Enums: Rank, NomCode, NameType, NamePart, State — all serialize to a SCREAMING_SNAKE string name; Rank/NomCode have from_name.
```

---

## Task 1: `nameparser-py` crate scaffold + `parse()` + maturin build + smoke test

**Files:** Create `crates/nameparser-py/{Cargo.toml,pyproject.toml,src/lib.rs}`; modify the workspace `Cargo.toml` (add member) and `.gitignore` (`/.venv/`).

**This is the toolchain-risk gate — get pyo3 + maturin + the venv building before building out the API.**

- [ ] **Step 1: crate + workspace + gitignore.** `crates/nameparser-py/Cargo.toml`:
  ```toml
  [package]
  name = "nameparser-py"
  version = "0.0.0"
  edition = "2021"
  license = "Apache-2.0"

  [lib]
  name = "nameparser"          # the importable Python module name
  crate-type = ["cdylib"]

  [dependencies]
  nameparser = { path = "../nameparser" }
  pyo3 = { version = "0.22", features = ["extension-module", "abi3-py39"] }
  pythonize = "0.22"           # must match the pyo3 minor version
  ```
  `pyproject.toml`:
  ```toml
  [build-system]
  requires = ["maturin>=1.5,<2.0"]
  build-backend = "maturin"

  [project]
  name = "nameparser"
  version = "0.0.0"
  description = "GBIF scientific name parser — Rust core, Python binding"
  requires-python = ">=3.9"
  license = { text = "Apache-2.0" }

  [tool.maturin]
  manifest-path = "crates/nameparser-py/Cargo.toml"
  module-name = "nameparser"
  features = ["pyo3/extension-module"]
  ```
  (Put `pyproject.toml` at the crate dir; maturin's `manifest-path`/`module-name` wire it up. Add `/.venv/` to the repo `.gitignore`.)

- [ ] **Step 2: minimal `src/lib.rs`** — enough to prove the round-trip. A `#[pyclass] ParsedName` holding `nameparser::ParsedName` with a HANDFUL of getters (`rank`, `genus`, `specific_epithet`, `infraspecific_epithet`, `warnings`) + `__repr__`; the module `parse` fn; `UnparsableNameError`:
  ```rust
  use pyo3::prelude::*;
  use pyo3::exceptions::PyException;
  use pyo3::create_exception;

  create_exception!(nameparser, UnparsableNameError, PyException);

  #[pyclass(name = "ParsedName")]
  pub struct PyParsedName { inner: nameparser::ParsedName }

  #[pymethods]
  impl PyParsedName {
      // enum → str via pythonize (no .name() exists on the core enums)
      #[getter] fn rank(&self, py: Python<'_>) -> PyResult<PyObject> { Ok(pythonize::pythonize(py, &self.inner.rank)?.into()) }
      #[getter] fn genus(&self) -> Option<String> { self.inner.genus.clone() }
      #[getter] fn specific_epithet(&self) -> Option<String> { self.inner.specific_epithet.clone() }
      #[getter] fn infraspecific_epithet(&self) -> Option<String> { self.inner.infraspecific_epithet.clone() }
      #[getter] fn warnings(&self) -> Vec<String> { self.inner.warnings.clone() }
      fn __repr__(&self) -> String { format!("ParsedName(rank={:?}, genus={:?})", self.inner.rank, self.inner.genus) }
  }

  #[pyfunction]
  #[pyo3(signature = (name, authorship=None, rank=None, code=None))]
  fn parse(name: &str, authorship: Option<&str>, rank: Option<&str>, code: Option<&str>) -> PyResult<PyParsedName> {
      let rank = rank.and_then(nameparser::Rank::from_name);
      let code = code.and_then(nameparser::NomCode::from_name);
      match nameparser::parse(name, authorship, rank, code) {
          Ok(pn) => Ok(PyParsedName { inner: pn }),
          Err(e) => Err(UnparsableNameError::new_err(/* message from ParseError */ format!("{e:?}"))),
      }
  }

  #[pymodule]
  fn nameparser(m: &Bound<'_, PyModule>) -> PyResult<()> {
      m.add_function(wrap_pyfunction!(parse, m)?)?;
      m.add_class::<PyParsedName>()?;
      m.add("UnparsableNameError", m.py().get_type_bound::<UnparsableNameError>())?;
      Ok(())
  }
  ```
  Verify the exact enum→string accessor by reading `crates/nameparser/src/model/enums.rs` (the method that yields the serialized SCREAMING_SNAKE name — it may be a `Display`, a `name()`, or the serde rename; use whatever the core exposes, matching what `parse_golden`/the CLI emit). Match the real `ParseError` shape for the message/type.

- [ ] **Step 3: build in a venv + smoke test.** Run:
  ```sh
  python3 -m venv .venv && . .venv/bin/activate
  pip install -q maturin pytest
  maturin develop -m crates/nameparser-py/Cargo.toml
  python -c "import nameparser; p=nameparser.parse('Vulpes vulpes silaceus Miller, 1907'); print(p.rank, p.genus, p.specific_epithet, p.infraspecific_epithet)"
  ```
  Expected: `SUBSPECIES Vulpes vulpes silaceus`. Then a raise check: `nameparser.parse('Tobacco mosaic virus')` → `nameparser.UnparsableNameError`.
  If `maturin develop` fails on the pyo3/abi3 config, resolve it here (this is the task's whole point).

- [ ] **Step 4: commit** `Phase 4a: nameparser-py crate + parse() + maturin build`.

---

## Task 2: full `ParsedName` surface + nested authorship + `parse_all` + `to_dict` + `__repr__`

**Files:** `crates/nameparser-py/src/lib.rs` (expand); add `crates/nameparser-py/tests/` or Python tests under `crates/nameparser-py/python/tests/`.

- [ ] **Step 1: `Authorship` pyclass** — `#[pyclass(name="Authorship")]` over `nameparser::Authorship`, getters `authors -> list[str]`, `ex_authors -> list[str]`, `year -> Optional[str]`, `imprint_year -> Optional[str]`, `__repr__`. (For `CombinedAuthorship` — `generic_authorship`/`specific_authorship` — expose as a small `CombinedAuthorship` pyclass with `combination_authorship`/`basionym_authorship` (Authorship) + `sanctioning_author`, OR flatten; pick one, keep it consistent with the core.)

- [ ] **Step 2: ALL 30 `ParsedName` getters.** Add the remaining getters, each mapping one core field to the idiomatic Python type:
  - `Option<String>` → `Optional[str]`; `bool` → `bool`; `Option<bool>` → `Optional[bool]`; `Option<i32>` → `Optional[int]` (`published_in_year`).
  - enums → their serialized string name: `code -> Optional[str]`, `type` (from `type_`) `-> str`, `state -> str`.
  - `warnings -> list[str]`; `notho -> Optional[list[str]]` (each `NamePart` as its string name); `epithet_qualifier -> Optional[dict[str,str]]` (NamePart-name → qualifier).
  - `combination_authorship`/`basionym_authorship -> Authorship`; `generic_authorship`/`specific_authorship -> Optional[CombinedAuthorship]`.
  - Expose `type` as the Python attribute name (not `type_`) — Rust `type_` is a keyword workaround; Python users want `.type`.
  - Booleans and every remaining field (`uninomial`, `infrageneric_epithet`, `cultivar_epithet`, `phrase`, `candidatus`, `original_spelling`, `extinct`, `taxonomic_note`, `nomenclatural_note`, `published_in`, `published_in_page`, `unparsed`, `doubtful`, `manuscript`, `sanctioning_author`).

- [ ] **Step 2b: `to_dict()` + `parse_all`.**
  - `#[pymethods] fn to_dict(&self, py) -> PyResult<PyObject>`: `pythonize(py, &self.inner)` — the complete structure straight from the core's `serde::Serialize` (this is the parity oracle in Task 3 and the escape hatch for anything a getter doesn't surface).
  - `#[pyfunction] parse_all(names, authorship=None, rank=None, code=None) -> list[Optional[ParsedName]]`: iterate an input sequence of names; each element is a `ParsedName` on success or `None` on `UnparsableName` (do NOT raise mid-batch). Document the None-on-unparsable contract.
  - Update `__repr__` to something useful (rank + the reconstructed name atoms).

- [ ] **Step 3: Python unit tests** (`crates/nameparser-py/python/tests/test_api.py`, run in the venv). Cover: the Vulpes subspecies (genus/epithet/rank/`combination_authorship.authors`==["Miller"]/`.year`=="1907"); a hybrid (`notho`); a name with 2+ `warnings`; `parse('Tobacco mosaic virus')` raises `UnparsableNameError`; `parse_all(["Abies alba","Tobacco mosaic virus"])` → `[ParsedName, None]`; `to_dict()` returns a dict whose keys match the JSON field names; enum fields are strings. `maturin develop` + `pytest` green.

- [ ] **Step 4: commit** `Phase 4a: full ParsedName surface + parse_all + to_dict`.

---

## Task 3: corpus parity test — Python output vs the core oracle → 0 diffs

**Files:** `crates/nameparser-py/python/tests/test_parity.py`.

**Realizes the gate: the Python binding produces byte-for-byte the core's parse over the corpus.**

- [ ] **Step 1: oracle.** Reuse this repo's `testdata/benchmark-data.txt` (+ the 6 test corpora). For each name, the oracle is the core's own serialization — obtain it by running the Rust CLI once (`./target/release/nameparser-cli parse --input=testdata/<f>.txt --output=- --quiet`) into `<f>.rust.jsonl` (git-ignored), OR by calling `nameparser.parse(...).to_dict()` and comparing against the Java oracle produced the same way the Phase-2/3 parity tests do. Simplest + strongest: compare Python `to_dict()` against the **Java** CLI jsonl (the independent oracle), reusing the shaded jar, so this also re-confirms Python↔Java parity.

- [ ] **Step 2: the test.** For every corpus name: `d = nameparser.parse(name).to_dict()` (or `None`/raise handling for unparsable); compare `d` to the oracle row with `warnings`/`notho`/`epithetQualifier` compared **order-insensitively** (mirror the Phase-2 `UNORDERED_FIELD_KEYS`), everything else exact. For unparsable names both sides must agree (Python raises `UnparsableNameError` with a `NameType`/`code` matching the oracle's error row). Tally; **assert 0 diffs** over all corpora (~11,302). Print examples on failure. Skip gracefully (pytest skip) if the Java jar isn't present, falling back to the Rust-CLI oracle.

- [ ] **Step 3: run + commit.** In the venv: `maturin develop` (release: `--release`) then `pytest crates/nameparser-py/python/tests/ -v`. Expect 0 diffs. Commit `Phase 4a: Python corpus parity test — zero diffs vs the core/Java oracle`.

---

## Task 4: packaging + Phase 4a status doc

**Files:** `crates/nameparser-py/README.md`; `docs/superpowers/findings/2026-07-11-phase4a-python-status.md`; ensure `pyproject.toml` metadata is release-ready.

- [ ] **Step 1: wheel build check.** `maturin build --release -m crates/nameparser-py/Cargo.toml` → confirm a `.whl` is produced under `target/wheels/` (git-ignored). Note its filename (abi3 → one wheel per platform, Python-version-independent). Do NOT publish.
- [ ] **Step 2: `crates/nameparser-py/README.md`** — install (`pip install nameparser-gbif` once published, or `maturin develop` from source), a 5-line usage example (`parse`, field access, `parse_all`, `to_dict`, catching `UnparsableNameError`), and the JDK-free/native note.
- [ ] **Step 3: status doc** — record: the Python binding works + is parity-clean (N names, 0 diffs); the API (`parse`/`parse_all`/`ParsedName`/`Authorship`/`to_dict`/`UnparsableNameError`); native (no FFM floor — inherits the batch profile from `benchmarks.md`); deferred: cibuildwheel cross-platform CI wheels + PyPI publish (follow-on), and **Phase 4b (R/extendr)** next. Commit `Phase 4a: Python binding status`.

## Self-Review
Realizes design §6.5's Python binding, natively (no C-ABI floor). Deferred + stated: CI cross-platform wheels + PyPI publish; the R binding (Phase 4b). Type consistency: `parse`/`parse_all` (Task 1/2) return the `ParsedName` pyclass (Task 2) whose `to_dict` (Task 2b) feeds the parity test (Task 3); enums are strings both in and out; the field surface is completeness-gated by Task 3's corpus diff. Toolchain risk (pyo3/maturin/venv) is front-loaded into Task 1.
