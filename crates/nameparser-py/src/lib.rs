// SPDX-License-Identifier: Apache-2.0

//! `nameparser-py` — a native PyO3 `cdylib` exposing [`::nameparser::parse`] to Python.
//! Unlike `nameparser-ffi` (the Phase 3 Java binding), there is no C-ABI /
//! JSON-marshalling floor here: PyO3 wraps the core `nameparser::model::ParsedName`
//! directly in a `#[pyclass]`, with getters mapping each core field to its idiomatic
//! Python type. Compiles to a module literally named `nameparser` (see `[lib] name` in
//! this crate's `Cargo.toml`) — `import nameparser` in Python.
//!
//! Phase 4a Task 1 proved the pyo3 + pythonize + maturin toolchain builds cleanly
//! against this machine's Python, with a handful of `ParsedName` getters (`rank`,
//! `genus`, `specific_epithet`, `infraspecific_epithet`, `warnings`) wired up to prove
//! the round trip. This (Task 2) is the completed field surface: all 30 `ParsedName`
//! getters — including the nested [`PyAuthorship`] pyclass for
//! `combination_authorship`/`basionym_authorship` — plus `to_dict()` (the complete
//! structure straight from the core's own `serde::Serialize`, the parity oracle for
//! later cross-validation and the escape hatch for anything a typed getter doesn't
//! surface), `parse_all()` for batch parsing, and a descriptive `__repr__`.
//!
//! Phase 4a Task 4 (this pass) added structured `.name_type`/`.code`/`.name` attributes to
//! [`UnparsableNameError`] (see [`unparsable_name_error`]), closing the Task 3 review's
//! `[Important]` finding that the exception previously carried only a message, unlike Java's
//! `UnparsableNameException.getType()`/`getCode()`/`getName()`.
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
//! start at the extern-prelude crate root, unambiguously naming the dependency. The same
//! ambiguity applies inside intra-doc-comment links (rustdoc resolves them against the
//! same module scope), so doc links to core items are written `` [`::nameparser::...`] ``
//! too, not `` [`nameparser::...`] ``.

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

// NOTE: a second, similarly cosmetic clippy-only false positive lives in the same
// family as the `gil-refs` one above — `cargo clippy -W clippy::all` reports "useless
// conversion to the same type: `pyo3::PyErr`" at three `PyResult`-returning
// `#[pymethods]`/`#[pyfunction]` items below (`PyAuthorship::to_dict`,
// `PyParsedName::to_dict`, `parse`), pointing at each item's signature line rather than
// any line this crate actually wrote. Getters (`#[getter]`, e.g. `rank`/`code` below)
// use the exact same `pythonize(..)?.into()` body shape and are NOT flagged — the lint
// fires inside pyo3's own macro-generated wrapper for plain methods/functions, a
// distinct code path from the getter one, not in anything visible in this file. An
// `#[allow(clippy::useless_conversion)]` on the affected `fn` item does NOT suppress it
// (confirmed directly against this build), for the same "attribute doesn't reach the
// macro's own generated code" reason `#[allow(unexpected_cfgs)]` fails to suppress the
// `gil-refs` warning above — left equally as a known, cosmetic, `cargo test`/`cargo
// build`-silent, `cargo clippy`-only warning.

/// Wraps the core [`::nameparser::model::Authorship`] for Python — the type of
/// [`PyParsedName::combination_authorship`] / [`PyParsedName::basionym_authorship`]:
/// authors, ex-authors and the year of a name (recombination) or basionym, but no "in"
/// authors (those are part of the `published_in` citation).
///
/// NOT used for `generic_authorship`/`specific_authorship` (`Option<CombinedAuthorship>`
/// — a *different*, bundling type in the core, holding its own combination + basionym
/// authorship plus a sanctioning author): those two are surfaced instead as a plain
/// `Optional[dict]` via `pythonize`, one level less structured — see
/// [`PyParsedName::generic_authorship`]'s doc comment for the rationale.
#[pyclass(name = "Authorship", module = "nameparser")]
pub struct PyAuthorship {
    inner: ::nameparser::model::Authorship,
}

#[pymethods]
impl PyAuthorship {
    #[getter]
    fn authors(&self) -> Vec<String> {
        self.inner.authors.clone()
    }

    #[getter]
    fn ex_authors(&self) -> Vec<String> {
        self.inner.ex_authors.clone()
    }

    #[getter]
    fn year(&self) -> Option<String> {
        self.inner.year.clone()
    }

    #[getter]
    fn imprint_year(&self) -> Option<String> {
        self.inner.imprint_year.clone()
    }

    /// The complete structure straight from the core's own `serde::Serialize` impl —
    /// `{"authors": [...], "exAuthors": [...], "year": ..., "imprintYear": ...}` — the
    /// same escape hatch [`PyParsedName::to_dict`] provides at the top level.
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize::pythonize(py, &self.inner)?.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "Authorship(authors={:?}, ex_authors={:?}, year={:?}, imprint_year={:?})",
            self.inner.authors, self.inner.ex_authors, self.inner.year, self.inner.imprint_year
        )
    }
}

/// Wraps the core [`::nameparser::model::ParsedName`] for Python. Field access goes
/// through `#[getter]`s below rather than exposing the struct fields directly —
/// enum-typed fields (like `rank`) have no `Display`/`.name()` on the core (it is
/// serde-only), so they are rendered to a Python `str` via `pythonize::pythonize`,
/// reusing the very same `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` impl the
/// JSON/Java wire format already relies on (see the core's `model::enums` module doc).
///
/// Getters below follow the exact field order of the core struct itself (see
/// `model::name::ParsedName`'s own doc comment: its own 16 fields, then
/// `ParsedAuthorship`'s 11, then `CombinedAuthorship`'s 3, flattened) — 30 fields total,
/// one getter each.
#[pyclass(name = "ParsedName", module = "nameparser")]
pub struct PyParsedName {
    inner: ::nameparser::model::ParsedName,
}

#[pymethods]
impl PyParsedName {
    // ---- ParsedName's own 16 fields ----

    // enum -> str via pythonize (no .name()/Display exists on the core enums)
    #[getter]
    fn rank(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize::pythonize(py, &self.inner.rank)?.into())
    }

    /// `Optional[str]` — `None` when no nomenclatural code applies/was inferred.
    #[getter]
    fn code(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize::pythonize(py, &self.inner.code)?.into())
    }

    #[getter]
    fn uninomial(&self) -> Option<String> {
        self.inner.uninomial.clone()
    }

    #[getter]
    fn genus(&self) -> Option<String> {
        self.inner.genus.clone()
    }

    /// `Optional[dict]` via `pythonize`, NOT the [`PyAuthorship`] pyclass —
    /// `generic_authorship` is a `CombinedAuthorship` (itself bundling a combination
    /// authorship, a basionym authorship, and a sanctioning author), unlike
    /// `combination_authorship`/`basionym_authorship` below, which are plain
    /// [`::nameparser::model::Authorship`] and so get the richer pyclass wrapper. See
    /// this crate's module doc / [`PyAuthorship`]'s doc comment for the same split.
    #[getter]
    fn generic_authorship(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize::pythonize(py, &self.inner.generic_authorship)?.into())
    }

    #[getter]
    fn infrageneric_epithet(&self) -> Option<String> {
        self.inner.infrageneric_epithet.clone()
    }

    #[getter]
    fn specific_epithet(&self) -> Option<String> {
        self.inner.specific_epithet.clone()
    }

    /// See [`Self::generic_authorship`]'s doc comment — the same `Optional[dict]`
    /// treatment, for the same reason.
    #[getter]
    fn specific_authorship(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize::pythonize(py, &self.inner.specific_authorship)?.into())
    }

    #[getter]
    fn infraspecific_epithet(&self) -> Option<String> {
        self.inner.infraspecific_epithet.clone()
    }

    #[getter]
    fn cultivar_epithet(&self) -> Option<String> {
        self.inner.cultivar_epithet.clone()
    }

    #[getter]
    fn phrase(&self) -> Option<String> {
        self.inner.phrase.clone()
    }

    #[getter]
    fn candidatus(&self) -> bool {
        self.inner.candidatus
    }

    /// `Optional[list[str]]` — each [`::nameparser::model::NamePart`] rendered as its
    /// SCREAMING_SNAKE_CASE name (e.g. `["INFRASPECIFIC"]`), via `pythonize`.
    #[getter]
    fn notho(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize::pythonize(py, &self.inner.notho)?.into())
    }

    #[getter]
    fn original_spelling(&self) -> Option<bool> {
        self.inner.original_spelling
    }

    /// `Optional[dict[str, str]]` — `NamePart` name -> qualifier (e.g.
    /// `{"SPECIFIC": "cf."}`), via `pythonize`.
    #[getter]
    fn epithet_qualifier(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize::pythonize(py, &self.inner.epithet_qualifier)?.into())
    }

    /// Python-facing attribute name is `type` (via `#[pyo3(name = "type")]`) — the Rust
    /// method itself can't be literally named `type`, a reserved keyword, so it follows
    /// the same `type_` trailing-underscore convention the core field uses.
    #[getter]
    #[pyo3(name = "type")]
    fn type_(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize::pythonize(py, &self.inner.type_)?.into())
    }

    // ---- ParsedAuthorship's own 11 fields ----

    #[getter]
    fn extinct(&self) -> bool {
        self.inner.extinct
    }

    #[getter]
    fn taxonomic_note(&self) -> Option<String> {
        self.inner.taxonomic_note.clone()
    }

    #[getter]
    fn nomenclatural_note(&self) -> Option<String> {
        self.inner.nomenclatural_note.clone()
    }

    #[getter]
    fn published_in(&self) -> Option<String> {
        self.inner.published_in.clone()
    }

    #[getter]
    fn published_in_year(&self) -> Option<i32> {
        self.inner.published_in_year
    }

    #[getter]
    fn published_in_page(&self) -> Option<String> {
        self.inner.published_in_page.clone()
    }

    #[getter]
    fn unparsed(&self) -> Option<String> {
        self.inner.unparsed.clone()
    }

    #[getter]
    fn doubtful(&self) -> bool {
        self.inner.doubtful
    }

    #[getter]
    fn manuscript(&self) -> bool {
        self.inner.manuscript
    }

    #[getter]
    fn state(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize::pythonize(py, &self.inner.state)?.into())
    }

    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner.warnings.clone()
    }

    // ---- CombinedAuthorship's own 3 fields, flattened onto the core struct ----

    /// The [`PyAuthorship`] pyclass — unlike `generic_authorship`/`specific_authorship`
    /// above, this field is a plain, always-present `Authorship` (never absent on the
    /// core side), so it gets the richer pyclass wrapper instead of a raw `pythonize`d
    /// dict.
    #[getter]
    fn combination_authorship(&self) -> PyAuthorship {
        PyAuthorship {
            inner: self.inner.combination_authorship.clone(),
        }
    }

    #[getter]
    fn basionym_authorship(&self) -> PyAuthorship {
        PyAuthorship {
            inner: self.inner.basionym_authorship.clone(),
        }
    }

    #[getter]
    fn sanctioning_author(&self) -> Option<String> {
        self.inner.sanctioning_author.clone()
    }

    // ---- name formatter (Java `org.gbif.nameparser.util.NameFormatter`) ----

    /// The full scientific name with authorship in its canonical form (Java
    /// `NameFormatter.canonical`). `None` if the name renders empty.
    fn canonical_name(&self) -> Option<String> {
        self.inner.canonical_name()
    }

    /// The canonical name without any authorship (Java
    /// `NameFormatter.canonicalWithoutAuthorship`).
    fn canonical_name_without_authorship(&self) -> Option<String> {
        self.inner.canonical_name_without_authorship()
    }

    /// The three bare name parts, unicode folded to ascii, no markers or authorship (Java
    /// `NameFormatter.canonicalMinimal`).
    fn canonical_name_minimal(&self) -> Option<String> {
        self.inner.canonical_name_minimal()
    }

    /// The full name with all details, incl. non-code-compliant informal remarks (Java
    /// `NameFormatter.canonicalComplete`).
    fn canonical_name_complete(&self) -> Option<String> {
        self.inner.canonical_name_complete()
    }

    /// As `canonical_name_complete` but with `<i>…</i>` markup (Java
    /// `NameFormatter.canonicalCompleteHtml`).
    fn canonical_name_complete_html(&self) -> Option<String> {
        self.inner.canonical_name_complete_html()
    }

    /// The full concatenated authorship incl. the sanctioning author (Java
    /// `NameFormatter.authorshipComplete`), or `None` when the name has no authorship.
    fn authorship_complete(&self) -> Option<String> {
        self.inner.authorship_complete()
    }

    /// `str(pn)` is the canonical name (empty string if it renders empty).
    fn __str__(&self) -> String {
        self.inner.canonical_name().unwrap_or_default()
    }

    // ---- escape hatch + repr ----

    /// The complete `ParsedName` structure straight from the core's own
    /// `serde::Serialize` impl — every field, keyed by its JSON/Java wire name (e.g.
    /// `combinationAuthorship`, `specificEpithet`), values omitted exactly as
    /// `serde_json::to_value` would omit them (see the core's `model::name` module doc
    /// for the full field-order/omission contract). This is the parity oracle used by
    /// golden cross-validation and the escape hatch for anything a typed getter above
    /// doesn't surface.
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize::pythonize(py, &self.inner)?.into())
    }

    /// Rank plus the full canonical name (`canonical_name`, with rank markers, hybrid
    /// signs and authorship) — a debugging aid. `rank` uses the Rust `Debug` form (e.g.
    /// `Species`), not the wire `SCREAMING_SNAKE_CASE`.
    fn __repr__(&self) -> String {
        format!(
            "ParsedName(rank={:?}, name={:?})",
            self.inner.rank,
            self.inner.canonical_name().unwrap_or_default()
        )
    }
}

/// Builds the [`UnparsableNameError`] `PyErr` for a core [`::nameparser::model::ParseError`],
/// attaching three Python attributes onto the raised instance in addition to the exception's
/// own message (unchanged — `str(err)` is still exactly `e.message`, as it always was):
/// `.name_type` (`str`), `.code` (`Optional[str]`), `.name` (`str`) — mirroring Java's
/// `UnparsableNameException.getType()`/`getCode()`/`getName()`
/// (`org.gbif.nameparser.api.UnparsableNameException`). `create_exception!` (above) makes
/// `UnparsableNameError` an ordinary Python `Exception` subclass with no `__slots__`, so its
/// instances carry a `__dict__` and accept arbitrary attributes via `setattr` — the simplest
/// path to structured fields that actually works, without a hand-written
/// `#[pyclass(extends = PyException)]` override of `__new__`/`__init__` (which would also risk
/// changing `args`/`str()` semantics). `name_type`/`code` are rendered via `pythonize`, the same
/// path every enum-typed getter in this file already uses (the core enums have no
/// `.name()`/`Display` — see this module's doc comment).
fn unparsable_name_error(py: Python<'_>, e: ::nameparser::model::ParseError) -> PyErr {
    match try_attach_error_attrs(py, e) {
        Ok(err) => err,
        // Reflective `setattr` of three plain (str-keyed) attributes onto a freshly
        // constructed, `__dict__`-backed exception instance cannot realistically fail — but
        // `setattr` returns `PyResult`, so surface a genuine failure honestly (e.g. an
        // out-of-memory `PyErr` from the interpreter) rather than panicking or silently
        // dropping it.
        Err(setattr_err) => setattr_err,
    }
}

fn try_attach_error_attrs(py: Python<'_>, e: ::nameparser::model::ParseError) -> PyResult<PyErr> {
    let err = UnparsableNameError::new_err(e.message);
    let value = err.value_bound(py).as_any();
    value.setattr("name_type", pythonize::pythonize(py, &e.type_)?)?;
    value.setattr("code", pythonize::pythonize(py, &e.code)?)?;
    value.setattr("name", e.name)?;
    Ok(err)
}

/// Parses a scientific name — the Python-facing entry point wrapping
/// [`::nameparser::parse`]. `rank`/`code`, when given, are the same `SCREAMING_SNAKE_CASE`
/// names the core's own JSON/Java wire format uses (e.g. `"SPECIES"`, `"ZOOLOGICAL"`),
/// resolved via [`Rank::from_name`]/[`NomCode::from_name`] — the same hint-parsing the
/// CLI and the Java FFM binding (`nameparser-ffi`) already use. Raises
/// [`UnparsableNameError`] when `name` cannot be parsed — see [`unparsable_name_error`] for
/// the `.name_type`/`.code`/`.name` attributes it carries.
#[pyfunction]
#[pyo3(signature = (name, authorship=None, rank=None, code=None))]
fn parse(
    py: Python<'_>,
    name: &str,
    authorship: Option<&str>,
    rank: Option<&str>,
    code: Option<&str>,
) -> PyResult<PyParsedName> {
    let rank = rank.and_then(Rank::from_name);
    let code = code.and_then(NomCode::from_name);
    match ::nameparser::parse(name, authorship, rank, code) {
        Ok(pn) => Ok(PyParsedName { inner: pn }),
        Err(e) => Err(unparsable_name_error(py, e)),
    }
}

/// Parses a batch of scientific names in one call. `authorship`/`rank`/`code` are the
/// same optional hints [`parse`] takes, applied uniformly to every name in `names`.
///
/// **Contract: never raises mid-batch.** Each output element is the parsed
/// [`PyParsedName`] on success, or `None` — NOT a raised [`UnparsableNameError`] — for
/// any name the core cannot parse, so one bad name in a large batch can't abort the
/// whole call. Callers that need the specific [`::nameparser::model::ParseError`] for a
/// failing name should call [`parse`] on that name individually instead.
#[pyfunction]
#[pyo3(signature = (names, authorship=None, rank=None, code=None))]
fn parse_all(
    names: Vec<String>,
    authorship: Option<&str>,
    rank: Option<&str>,
    code: Option<&str>,
) -> Vec<Option<PyParsedName>> {
    let rank = rank.and_then(Rank::from_name);
    let code = code.and_then(NomCode::from_name);
    names
        .iter()
        .map(
            |name| match ::nameparser::parse(name, authorship, rank, code) {
                Ok(pn) => Some(PyParsedName { inner: pn }),
                Err(_) => None,
            },
        )
        .collect()
}

#[pymodule]
fn nameparser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(parse_all, m)?)?;
    m.add_class::<PyParsedName>()?;
    m.add_class::<PyAuthorship>()?;
    m.add(
        "UnparsableNameError",
        m.py().get_type_bound::<UnparsableNameError>(),
    )?;
    Ok(())
}
