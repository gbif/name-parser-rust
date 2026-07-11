# SPDX-License-Identifier: Apache-2.0
"""Type stub for the native `nameparser` extension module (Phase 4a Python binding).

Hand-written, not generated — the compiled `#[pyclass]`/`#[pyfunction]` items in
`crates/nameparser-py/src/lib.rs` have no Python-level source for a stub generator (e.g.
`pyo3-stub-gen`) to read; this crate doesn't depend on one. Field order and names below
follow `PyParsedName`'s own `#[getter]`s in that file (which in turn follow
`crates/nameparser/src/model/name.rs`'s 30-field `ParsedName`, documented there): ParsedName's
own 16 fields, then ParsedAuthorship's 11, then CombinedAuthorship's 3, flattened. Keep this
file in sync by hand whenever `lib.rs`'s getter surface changes — nothing enforces the two
stay aligned automatically.

Enum-typed fields (`rank`, `code`, `type`, `state`, and each element of `notho`) are plain
`str` — the SCREAMING_SNAKE_CASE wire name (e.g. `"SPECIES"`, `"ZOOLOGICAL"`), matching the
JSON/Java `.name()` convention — NOT a Python `enum.Enum`. See `lib.rs`'s module doc comment
for why: the core Rust enums are serde-only (no `.name()`/`Display`), rendered to Python via
`pythonize`.
"""

from typing import Any

class Authorship:
    """One authorship citation — a recombination or a basionym. Always present (never `None`
    itself) as `ParsedName.combination_authorship`/`.basionym_authorship`, though its own
    fields may all be empty/`None`. NOT the type of `ParsedName.generic_authorship`/
    `.specific_authorship` — those are a different, bundling core type
    (`CombinedAuthorship`) surfaced as a plain `Optional[dict]` instead; see those
    properties' docstrings below.
    """

    @property
    def authors(self) -> list[str]: ...
    @property
    def ex_authors(self) -> list[str]: ...
    @property
    def year(self) -> str | None: ...
    @property
    def imprint_year(self) -> str | None: ...
    def to_dict(self) -> dict[str, Any]:
        """The complete structure straight from the core's own `serde::Serialize` impl —
        `{"authors": [...], "exAuthors": [...], "year": ..., "imprintYear": ...}`."""
        ...
    def __repr__(self) -> str: ...

class ParsedName:
    """A fully or partially parsed scientific name — the return type of `parse()`/each
    non-`None` element of `parse_all()`. One read-only property per core field; see
    `crates/nameparser/src/model/name.rs`'s `ParsedName` doc comment for field-by-field
    parsing semantics (this stub only documents the Python-facing type of each).
    """

    # ---- ParsedName's own 16 fields ----
    @property
    def rank(self) -> str: ...
    @property
    def code(self) -> str | None:
        """`None` when no nomenclatural code applies/was inferred."""
        ...
    @property
    def uninomial(self) -> str | None: ...
    @property
    def genus(self) -> str | None: ...
    @property
    def generic_authorship(self) -> dict[str, Any] | None:
        """A `CombinedAuthorship` dict (`combinationAuthorship`/`basionymAuthorship`/
        `sanctioningAuthor` keys, wire-cased) when the generic/uninomial part carries its own
        authorship (e.g. a sectional/subgeneric combination), else `None`. Unlike
        `combination_authorship`/`basionym_authorship` below, this is a raw `pythonize`d
        dict, not an `Authorship` instance — see `lib.rs`'s `PyParsedName::generic_authorship`
        doc comment for the rationale.
        """
        ...
    @property
    def infrageneric_epithet(self) -> str | None: ...
    @property
    def specific_epithet(self) -> str | None: ...
    @property
    def specific_authorship(self) -> dict[str, Any] | None:
        """Same `CombinedAuthorship`-shaped `Optional[dict]` as `generic_authorship`, for the
        specific epithet."""
        ...
    @property
    def infraspecific_epithet(self) -> str | None: ...
    @property
    def cultivar_epithet(self) -> str | None: ...
    @property
    def phrase(self) -> str | None: ...
    @property
    def candidatus(self) -> bool: ...
    @property
    def notho(self) -> list[str] | None:
        """Which name part(s) a hybrid marker (`×`/`x`) applies to, each rendered as its
        `NamePart` wire name (e.g. `["INFRASPECIFIC"]`); `None` for a non-hybrid name."""
        ...
    @property
    def original_spelling(self) -> bool | None: ...
    @property
    def epithet_qualifier(self) -> dict[str, str] | None:
        """`NamePart` wire name -> qualifier, e.g. `{"SPECIFIC": "cf."}`."""
        ...
    @property
    def type(self) -> str:
        """The `NameType` wire name (e.g. `"SCIENTIFIC"`). Python attribute is `type`, not
        `type_` — `type_` is only a Rust-side keyword workaround."""
        ...

    # ---- ParsedAuthorship's own 11 fields ----
    @property
    def extinct(self) -> bool: ...
    @property
    def taxonomic_note(self) -> str | None: ...
    @property
    def nomenclatural_note(self) -> str | None: ...
    @property
    def published_in(self) -> str | None: ...
    @property
    def published_in_year(self) -> int | None: ...
    @property
    def published_in_page(self) -> str | None: ...
    @property
    def unparsed(self) -> str | None: ...
    @property
    def doubtful(self) -> bool: ...
    @property
    def manuscript(self) -> bool: ...
    @property
    def state(self) -> str:
        """The `State` wire name: `"COMPLETE"`, `"PARTIAL"`, or `"NONE"`."""
        ...
    @property
    def warnings(self) -> list[str]: ...

    # ---- CombinedAuthorship's own 3 fields, flattened onto the core struct ----
    @property
    def combination_authorship(self) -> Authorship: ...
    @property
    def basionym_authorship(self) -> Authorship: ...
    @property
    def sanctioning_author(self) -> str | None: ...

    # ---- escape hatch + repr ----
    def to_dict(self) -> dict[str, Any]:
        """The complete `ParsedName` structure straight from the core's own
        `serde::Serialize` impl, keyed by its JSON/Java wire name (e.g.
        `combinationAuthorship`, `specificEpithet`) — every field this class's properties
        expose, plus the canonical escape hatch if a future core field lags behind this
        stub/the compiled getters."""
        ...
    def __repr__(self) -> str: ...

class UnparsableNameError(Exception):
    """Raised by `parse()` when `name` cannot be parsed into a `ParsedName`. Mirrors Java's
    `org.gbif.nameparser.api.UnparsableNameException` (`getType()`/`getCode()`/`getName()`).

    `str(err)` is the core's own `ParseError.message`, unchanged (e.g.
    `"Unparsable OTHER name: Tobacco mosaic virus"`) — NOT a repr of `(name_type, code,
    name)`.
    """

    name_type: str
    """The `NameType` wire name, e.g. `"OTHER"`."""
    code: str | None
    """The `NomCode` wire name when known despite the name being unparsable (e.g. `"VIRUS"`),
    else `None`."""
    name: str
    """The name text the core attempted to parse — not always byte-identical to the `name`
    argument passed to `parse()` (e.g. for a few OTU/barcode-style inputs, this is an
    extracted substring)."""

def parse(
    name: str,
    authorship: str | None = ...,
    rank: str | None = ...,
    code: str | None = ...,
) -> ParsedName:
    """Parses one scientific name (optionally with a separately-supplied authorship string,
    and/or a rank/nomenclatural-code hint). `rank`/`code`, when given, are the same
    SCREAMING_SNAKE_CASE wire names `ParsedName.rank`/`.code` return (e.g. `"SPECIES"`,
    `"ZOOLOGICAL"`); an unrecognized hint string is treated as absent, not an error.

    Raises `UnparsableNameError` if `name` cannot be parsed.
    """
    ...

def parse_all(
    names: list[str],
    authorship: str | None = ...,
    rank: str | None = ...,
    code: str | None = ...,
) -> list[ParsedName | None]:
    """Parses a batch of names in one call; `authorship`/`rank`/`code` are the same optional
    hints `parse()` takes, applied uniformly to every name in `names`.

    Never raises mid-batch: each result element is a `ParsedName` on success, or `None` (NOT
    a raised `UnparsableNameError`) for any name the core cannot parse. Call `parse()` on a
    specific name individually to get its `UnparsableNameError` (with `.name_type`/`.code`).
    """
    ...
