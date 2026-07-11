// SPDX-License-Identifier: Apache-2.0

//! `nameparser-py` — a native PyO3 `cdylib` exposing [`nameparser::parse`] to Python
//! (Phase 4a). Unlike `nameparser-ffi` (the Phase 3 Java binding), there is no C-ABI /
//! JSON-marshalling floor here: PyO3 wraps the core `nameparser::model::ParsedName`
//! directly in a `#[pyclass]`, with getters mapping each core field to its idiomatic
//! Python type. Compiles to a module literally named `nameparser` (see `[lib] name` in
//! this crate's `Cargo.toml`) — `import nameparser` in Python.
//!
//! This crate is the toolchain-risk gate for Phase 4a (Task 1 of the Phase 4a plan): it
//! proves pyo3 + pythonize + maturin build cleanly against this machine's Python before
//! the full field surface is built out. Scope is deliberately narrow — a HANDFUL of
//! `ParsedName` getters (`rank`, `genus`, `specific_epithet`, `infraspecific_epithet`,
//! `warnings`) are exposed here, enough to prove the round trip. A later task fleshes
//! this out to all 30 core fields plus nested `Authorship`, `to_dict`, `parse_all`.
//!
//! The core's `nameparser::ParsedName`/`Rank`/`NomCode`/`ParseError` types live under
//! `nameparser::model::*`, not re-exported at the `nameparser` crate root — same path
//! `nameparser-ffi` (the Phase 3 sibling binding) already uses. Only `nameparser::parse`
//! itself is a top-level re-export.
//!
//! **Every reference to the core crate below is written `::nameparser::...`, with a
//! leading `::`.** This crate's own compiled library is *also* named `nameparser` (see
//! `[lib] name` in `Cargo.toml` — it is the importable Python module name) and this file
//! declares a `#[pymodule] fn nameparser` (below) — that function item and the
//! `nameparser` path dependency then share one bare identifier in this module's scope,
//! which `rustc` rejects as ambiguous (E0659). The leading `::` forces resolution to
//! start at the extern-prelude crate root, unambiguously naming the dependency.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use ::nameparser::model::{NomCode, Rank};

// NOTE: this expands to a `unexpected `cfg` condition value: `gil-refs`` build warning
// under pyo3 0.22.x — the `create_exception!`/`impl_exception_boilerplate!` expansion
// checks `cfg(feature = "gil-refs")`, a pyo3-internal feature name this crate never
// declares (we don't opt into pyo3's deprecated GIL-Ref API). Harmless (fixed upstream
// in later pyo3 releases); an `#[allow(unexpected_cfgs)]` on this line does NOT
// suppress it (outer attributes on a `macro_rules!` invocation aren't forwarded into
// its expansion), and a crate-wide `#![allow(unexpected_cfgs)]` was deliberately not
// added so a real unexpected-cfg elsewhere still surfaces. Left as a known, cosmetic,
// non-blocking warning — see the Phase 4a Task 1 report for the exact text.
create_exception!(nameparser, UnparsableNameError, PyException);

/// Wraps the core [`nameparser::model::ParsedName`] for Python. Field access goes
/// through `#[getter]`s below rather than exposing the struct fields directly —
/// enum-typed fields (like `rank`) have no `Display`/`.name()` on the core (it is
/// serde-only), so they are rendered to a Python `str` via `pythonize::pythonize`,
/// reusing the very same `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` impl the
/// JSON/Java wire format already relies on (see the core's `model::enums` module doc).
#[pyclass(name = "ParsedName")]
pub struct PyParsedName {
    inner: ::nameparser::model::ParsedName,
}

#[pymethods]
impl PyParsedName {
    // enum -> str via pythonize (no .name()/Display exists on the core enums)
    #[getter]
    fn rank(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize::pythonize(py, &self.inner.rank)?.into())
    }

    #[getter]
    fn genus(&self) -> Option<String> {
        self.inner.genus.clone()
    }

    #[getter]
    fn specific_epithet(&self) -> Option<String> {
        self.inner.specific_epithet.clone()
    }

    #[getter]
    fn infraspecific_epithet(&self) -> Option<String> {
        self.inner.infraspecific_epithet.clone()
    }

    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner.warnings.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ParsedName(rank={:?}, genus={:?})",
            self.inner.rank, self.inner.genus
        )
    }
}

/// Parses a scientific name — the Python-facing entry point wrapping
/// [`nameparser::parse`]. `rank`/`code`, when given, are the same `SCREAMING_SNAKE_CASE`
/// names the core's own JSON/Java wire format uses (e.g. `"SPECIES"`, `"ZOOLOGICAL"`),
/// resolved via [`Rank::from_name`]/[`NomCode::from_name`] — the same hint-parsing the
/// CLI and the Java FFM binding (`nameparser-ffi`) already use. Raises
/// [`UnparsableNameError`], carrying the core [`nameparser::model::ParseError`]'s own
/// message, when `name` cannot be parsed.
#[pyfunction]
#[pyo3(signature = (name, authorship=None, rank=None, code=None))]
fn parse(
    name: &str,
    authorship: Option<&str>,
    rank: Option<&str>,
    code: Option<&str>,
) -> PyResult<PyParsedName> {
    let rank = rank.and_then(Rank::from_name);
    let code = code.and_then(NomCode::from_name);
    match ::nameparser::parse(name, authorship, rank, code) {
        Ok(pn) => Ok(PyParsedName { inner: pn }),
        Err(e) => Err(UnparsableNameError::new_err(e.message)),
    }
}

#[pymodule]
fn nameparser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_class::<PyParsedName>()?;
    m.add(
        "UnparsableNameError",
        m.py().get_type_bound::<UnparsableNameError>(),
    )?;
    Ok(())
}
