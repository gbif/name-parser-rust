# SPDX-License-Identifier: Apache-2.0
"""Getter/`to_dict()` consistency test (Phase 4a, Task 3 follow-up — closes the
Task-2-review-flagged coverage gap).

`test_parity.py` diffs `ParsedName.to_dict()` against the oracle — but `to_dict()` goes
straight through `pythonize(py, &self.inner)`, bypassing every individual `#[getter]` in
`crates/nameparser-py/src/lib.rs` entirely. A getter with a copy-paste bug (wrong field,
wrong wire-key lookup, an inverted `Option`, …) would be invisible to the corpus parity
test and still pass it, because `to_dict()` never calls the getters at all — the two are
independent code paths that happen to read the same underlying `self.inner`.

This test exercises that other path: for a representative sample of names, it parses via
`nameparser.parse`, calls `to_dict()`, and asserts every one of the 30 `ParsedName` getters
returns exactly what `to_dict()` reports under the corresponding wire key — plus the same
for the two nested `Authorship` getters' own 4 fields each. Field <-> wire-key pairs below
are taken directly from `crates/nameparser/src/model/name.rs`'s `#[serde(rename = …)]`
attributes (not from `crates/nameparser-py/src/lib.rs`'s getter bodies) so a getter that
silently drifted from the wire contract cannot also grade its own homework.
"""
from __future__ import annotations

import pytest

import nameparser
from _corpus import CORPORA, read_corpus

# python ParsedName getter -> to_dict() wire key, for every scalar/enum/simple-collection
# field. Straight from crates/nameparser/src/model/name.rs's field list + #[serde(rename)]
# attributes (26 of ParsedName's 30 fields; the remaining 4 — the two Authorship-typed and
# two CombinedAuthorship-typed fields — need their own nested comparison, below).
SIMPLE_GETTERS: dict[str, str] = {
    "rank": "rank",
    "code": "code",
    "uninomial": "uninomial",
    "genus": "genus",
    "infrageneric_epithet": "infragenericEpithet",
    "specific_epithet": "specificEpithet",
    "infraspecific_epithet": "infraspecificEpithet",
    "cultivar_epithet": "cultivarEpithet",
    "phrase": "phrase",
    "candidatus": "candidatus",
    "notho": "notho",
    "original_spelling": "originalSpelling",
    "epithet_qualifier": "epithetQualifier",
    "type": "type",
    "extinct": "extinct",
    "taxonomic_note": "taxonomicNote",
    "nomenclatural_note": "nomenclaturalNote",
    "published_in": "publishedIn",
    "published_in_year": "publishedInYear",
    "published_in_page": "publishedInPage",
    "unparsed": "unparsed",
    "doubtful": "doubtful",
    "manuscript": "manuscript",
    "state": "state",
    "warnings": "warnings",
    "sanctioning_author": "sanctioningAuthor",
}

# The two plain-Authorship-typed getters (always present, never None) -> wire key, each
# compared field-by-field below against the same 4 sub-keys Authorship's own Rust struct
# defines (authors/exAuthors/year/imprintYear).
AUTHORSHIP_GETTERS: dict[str, str] = {
    "combination_authorship": "combinationAuthorship",
    "basionym_authorship": "basionymAuthorship",
}

# The two Optional[CombinedAuthorship]-typed getters, which — unlike combination_/
# basionym_authorship above — return a raw pythonize'd dict (or None), directly comparable
# to to_dict()'s sub-object with no field-by-field unpacking needed.
COMBINED_AUTHORSHIP_GETTERS: dict[str, str] = {
    "generic_authorship": "genericAuthorship",
    "specific_authorship": "specificAuthorship",
}

assert (
    len(SIMPLE_GETTERS) + len(AUTHORSHIP_GETTERS) + len(COMBINED_AUTHORSHIP_GETTERS) == 30
), "expected exactly the 30 ParsedName getters the Phase 4a plan's core surface documents"


def _assert_getters_match_to_dict(name: str, pn: "nameparser.ParsedName", d: dict) -> None:
    for attr, wire_key in SIMPLE_GETTERS.items():
        getter_val = getattr(pn, attr)
        dict_val = d.get(wire_key)
        assert getter_val == dict_val, (
            f"{name!r}: pn.{attr} = {getter_val!r} but to_dict()[{wire_key!r}] = "
            f"{dict_val!r}"
        )

    for attr, wire_key in AUTHORSHIP_GETTERS.items():
        authorship = getattr(pn, attr)
        sub = d.get(wire_key) or {}
        assert authorship.authors == sub.get("authors", []), (
            f"{name!r}: pn.{attr}.authors = {authorship.authors!r} but "
            f"to_dict()[{wire_key!r}]['authors'] = {sub.get('authors')!r}"
        )
        assert authorship.ex_authors == sub.get("exAuthors", []), (
            f"{name!r}: pn.{attr}.ex_authors = {authorship.ex_authors!r} but "
            f"to_dict()[{wire_key!r}]['exAuthors'] = {sub.get('exAuthors')!r}"
        )
        assert authorship.year == sub.get("year"), (
            f"{name!r}: pn.{attr}.year = {authorship.year!r} but "
            f"to_dict()[{wire_key!r}]['year'] = {sub.get('year')!r}"
        )
        assert authorship.imprint_year == sub.get("imprintYear"), (
            f"{name!r}: pn.{attr}.imprint_year = {authorship.imprint_year!r} but "
            f"to_dict()[{wire_key!r}]['imprintYear'] = {sub.get('imprintYear')!r}"
        )

    for attr, wire_key in COMBINED_AUTHORSHIP_GETTERS.items():
        getter_val = getattr(pn, attr)
        dict_val = d.get(wire_key)
        assert getter_val == dict_val, (
            f"{name!r}: pn.{attr} = {getter_val!r} but to_dict()[{wire_key!r}] = "
            f"{dict_val!r}"
        )


# Explicitly required by the Task 3 brief: a hybrid, a name with authorship, a name with
# warnings, and one name each populating generic_authorship / specific_authorship. These
# four are lifted from crates/nameparser/src/model/name.rs's own golden fixtures and
# crates/nameparser-py/python/tests/test_api.py's existing Task 2 tests (already
# known-good there), plus the two exact examples the brief names by string.
EXPLICIT_SAMPLE: list[str] = [
    "Cordia (Adans.) Kuntze sect. Salimori",  # populates generic_authorship
    "Acer campestre L. cv. 'Elsrijk' Broerse",  # populates specific_authorship
    "×Abies alba nothovar. rubra",  # hybrid -> notho
    "Senecio fuchsii C.C.Gmel. subsp. fuchsii var. fuchsii",  # >= 2 warnings
    "Vulpes vulpes silaceus Miller, 1907",  # combination_authorship (authors + year)
]

# Cap on the sample size — "~50" per the brief. The 5 small, curated corpora
# (hybrids/names-with-authors/other/otu/placeholder) are pulled in wholesale for breadth
# (some entries there are deliberately unparsable "tobedeleted"-style junk data and are
# skipped below, not padding the count), topped up with an evenly spaced slice of
# benchmark-data.txt.
SAMPLE_SIZE = 50


def _build_candidate_names() -> list[str]:
    names: list[str] = []
    for n in EXPLICIT_SAMPLE:
        if n not in names:
            names.append(n)
    for corpus in ("hybrids", "names-with-authors", "other", "otu", "placeholder"):
        for n in read_corpus(corpus):
            if n not in names:
                names.append(n)
    bulk = read_corpus("benchmark-data")
    step = max(1, len(bulk) // (SAMPLE_SIZE * 2))
    for n in bulk[::step]:
        if n not in names:
            names.append(n)
    return names


def test_getters_agree_with_to_dict_over_a_representative_sample():
    """For ~50 names spanning the corpora (including a hybrid, an authorship-bearing name,
    a warnings-bearing name, and the two named generic_authorship/specific_authorship
    examples), every individual getter must agree with the corresponding `to_dict()` value
    — the coverage `test_parity.py` (which only ever calls `to_dict()`) cannot provide,
    since `to_dict()` bypasses the getters entirely."""
    candidates = _build_candidate_names()

    sampled: list[str] = []
    saw_hybrid = False
    saw_warnings = False
    saw_authorship = False
    saw_generic_authorship = False
    saw_specific_authorship = False

    for name in candidates:
        if len(sampled) >= SAMPLE_SIZE:
            break
        try:
            pn = nameparser.parse(name)
        except nameparser.UnparsableNameError:
            continue  # this test only makes sense for names that actually parse

        d = pn.to_dict()
        _assert_getters_match_to_dict(name, pn, d)
        sampled.append(name)

        if pn.notho:
            saw_hybrid = True
        if pn.warnings:
            saw_warnings = True
        if pn.combination_authorship.authors or pn.basionym_authorship.authors:
            saw_authorship = True
        if pn.generic_authorship is not None:
            saw_generic_authorship = True
        if pn.specific_authorship is not None:
            saw_specific_authorship = True

    print(
        f"\ngetter-consistency sample: {len(sampled)} names checked "
        f"(candidates offered: {len(candidates)}) across {len(CORPORA)} corpora"
    )

    assert len(sampled) >= 40, (
        f"only {len(sampled)} parsable names were sampled (wanted ~{SAMPLE_SIZE}) — the "
        f"candidate pool may have shrunk or a corpus file may be missing"
    )
    assert saw_hybrid, "sample never exercised a hybrid (notho) getter"
    assert saw_warnings, "sample never exercised a non-empty warnings getter"
    assert saw_authorship, "sample never exercised a populated authorship getter"
    assert saw_generic_authorship, "sample never exercised generic_authorship"
    assert saw_specific_authorship, "sample never exercised specific_authorship"


def test_the_two_named_examples_populate_generic_and_specific_authorship_respectively():
    """Pinpoint check for the two names the Task 3 brief calls out by string — guards
    against `_build_candidate_names`'s dedup/sampling logic ever silently dropping them,
    independent of the broader sample-driven test above."""
    cordia = nameparser.parse("Cordia (Adans.) Kuntze sect. Salimori")
    assert cordia.generic_authorship is not None
    assert cordia.generic_authorship["combinationAuthorship"]["authors"] == ["Kuntze"]
    assert cordia.generic_authorship["basionymAuthorship"]["authors"] == ["Adans."]
    assert cordia.specific_authorship is None

    acer = nameparser.parse("Acer campestre L. cv. 'Elsrijk' Broerse")
    assert acer.specific_authorship is not None
    assert acer.specific_authorship["combinationAuthorship"]["authors"] == ["L."]
    assert acer.generic_authorship is None
