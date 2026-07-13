# SPDX-License-Identifier: Apache-2.0
"""Unit tests for the `nameparser` PyO3 binding (Phase 4a, Task 2).

Run from the repo root, inside the project venv, after `maturin develop`:

    . .venv/bin/activate
    maturin develop -m crates/nameparser-py/Cargo.toml
    pytest crates/nameparser-py/python/tests/ -v

These exercise the *binding* (getters, exceptions, `to_dict`, `parse_all`) — not the
parser's own correctness, which is covered exhaustively by the core crate's golden
harness (`crates/nameparser/tests/parse_golden.rs`) against the Java oracle. Every
input name below is chosen because it's already known-good there.
"""

import pytest

import nameparser


def sci(name: str, *args, **kwargs) -> nameparser.ParsedName:
    """`parse()` a name known to be scientific, narrowed to `ParsedName` for the type checker —
    `parse()` returns `ParsedName | Informal` in 5.0.0, so a `ParsedName`-field access needs the
    variant pinned. Doubles as an assertion that these inputs really do parse to a `ParsedName`."""
    r = nameparser.parse(name, *args, **kwargs)
    assert isinstance(r, nameparser.ParsedName)
    return r


def test_vulpes_subspecies_core_fields_and_combination_authorship():
    # Golden reference row 1 (see crates/nameparser/src/model/name.rs's
    # `wire_matches_reference_row_1_vulpes_vulpes_silaceus`).
    pn = nameparser.parse("Vulpes vulpes silaceus Miller, 1907")

    assert isinstance(pn, nameparser.ParsedName)
    assert pn.rank == "SUBSPECIES"
    assert pn.code == "ZOOLOGICAL"
    assert pn.genus == "Vulpes"
    assert pn.specific_epithet == "vulpes"
    assert pn.infraspecific_epithet == "silaceus"
    assert pn.type == "SCIENTIFIC"
    assert pn.state == "COMPLETE"

    ca = pn.combination_authorship
    assert isinstance(ca, nameparser.Authorship)
    assert ca.authors == ["Miller"]
    assert ca.ex_authors == []
    assert ca.year == "1907"
    assert ca.imprint_year is None

    # Fields that don't apply to this name stay unset/native-empty.
    assert pn.uninomial is None
    assert pn.notho is None
    assert pn.warnings == []
    assert pn.basionym_authorship.authors == []


def test_hybrid_name_sets_notho():
    pn = sci("×Abies alba nothovar. rubra")
    assert pn.notho is not None
    assert pn.notho != []
    assert all(isinstance(part, str) for part in pn.notho)
    assert "INFRASPECIFIC" in pn.notho


def test_name_with_multiple_warnings():
    pn = sci("Senecio fuchsii C.C.Gmel. subsp. fuchsii var. fuchsii")
    assert len(pn.warnings) >= 2
    assert all(isinstance(w, str) for w in pn.warnings)
    assert "name was quadrinomial" in pn.warnings


def test_parse_unparsable_name_raises():
    with pytest.raises(nameparser.UnparsableNameError) as excinfo:
        nameparser.parse("Tobacco mosaic virus")
    assert "Tobacco mosaic virus" in str(excinfo.value)


def test_unparsable_name_error_exposes_structured_attributes():
    # Phase 4a Task 4: closes the Task 3 review's [Important] finding — the exception used to
    # carry only a message; it now mirrors Java's `UnparsableNameException.getType()`/
    # `getCode()`/`getName()` (org.gbif.nameparser.api.UnparsableNameException) as plain Python
    # attributes. "Tobacco mosaic virus" is rejected by the virus gate as
    # `ParseError::new(NameType::Other, Some(NomCode::Virus), original)`
    # (crates/nameparser/src/pipeline/preflight.rs's `apply_virus_gate`), so `name_type` is
    # "OTHER" (not "SCIENTIFIC"/etc.) and `code` is "VIRUS", not None.
    with pytest.raises(nameparser.UnparsableNameError) as excinfo:
        nameparser.parse("Tobacco mosaic virus")
    exc = excinfo.value

    assert exc.name_type == "OTHER"
    assert exc.code == "VIRUS"
    assert exc.name == "Tobacco mosaic virus"
    # str() is unchanged: still exactly the core's `ParseError.message`, not a repr of the
    # attributes/args tuple.
    assert str(exc) == "Unparsable OTHER name: Tobacco mosaic virus"


def test_unparsable_name_error_code_is_none_when_no_code_applies():
    # A name rejected for a reason that carries no NomCode — a single bare letter, per
    # `crates/nameparser/src/pipeline/preflight.rs`'s `count_letters(&s) == 1` guard
    # (`ParseError::new(NameType::Other, None, original)`; also covered by the core's own
    # `single_bare_letter_is_other` unit test) — must expose `code is None`, not the string
    # "None" or a missing attribute.
    with pytest.raises(nameparser.UnparsableNameError) as excinfo:
        nameparser.parse("X")
    exc = excinfo.value
    assert exc.name_type == "OTHER"
    assert exc.code is None
    assert exc.name == "X"


def test_parse_all_returns_none_for_unparsable_without_raising():
    results = nameparser.parse_all(["Abies alba", "Tobacco mosaic virus"])
    assert len(results) == 2
    assert isinstance(results[0], nameparser.ParsedName)
    assert results[0].genus == "Abies"
    assert results[1] is None


def test_to_dict_uses_wire_field_names_and_string_enums():
    pn = nameparser.parse("Vulpes vulpes silaceus Miller, 1907")
    d = pn.to_dict()

    # Wire (JSON/Java) field names, not the Python snake_case getter names.
    for key in (
        "rank",
        "code",
        "genus",
        "specificEpithet",
        "infraspecificEpithet",
        "type",
        "state",
        "warnings",
        "combinationAuthorship",
        "basionymAuthorship",
    ):
        assert key in d, f"missing wire key {key!r} in to_dict(): {sorted(d)}"

    # snake_case Python getter names must NOT leak into the dict.
    assert "specific_epithet" not in d
    assert "type_" not in d

    # Enum-typed fields serialize as plain strings, matching the JSON wire format.
    assert d["rank"] == "SUBSPECIES"
    assert d["code"] == "ZOOLOGICAL"
    assert d["type"] == "SCIENTIFIC"
    assert d["state"] == "COMPLETE"

    assert d["combinationAuthorship"] == {"authors": ["Miller"], "exAuthors": [], "year": "1907"}


def test_type_attribute_not_type_underscore():
    pn = sci("Homo sapiens")
    assert pn.type == "SCIENTIFIC"
    assert not hasattr(pn, "type_")


def test_classes_report_the_nameparser_module_not_builtins():
    # Regression guard for the Task 1 report's flagged follow-up: `#[pyclass]` needs
    # `module = "nameparser"` or `type(x)` shows as `builtins.X` instead of
    # `nameparser.X`.
    pn = sci("Abies alba Mill.")
    assert type(pn).__module__ == "nameparser"
    assert type(pn.combination_authorship).__module__ == "nameparser"


def test_authorship_to_dict_and_repr():
    pn = sci("Abies alba Mill.")
    ca = pn.combination_authorship
    assert ca.to_dict() == {"authors": ["Mill."], "exAuthors": []}
    assert "Mill." in repr(ca)


def test_parsed_name_repr_includes_rank_and_name_atoms():
    pn = sci("Abies alba Mill.")
    r = repr(pn)
    assert "Abies" in r
    assert "alba" in r
    assert "Species" in r  # Rust Debug form of the Rank enum, not the wire SCREAMING_SNAKE_CASE


def test_name_formatter_renderings():
    # Expected values are Java-authoritative (real NameFormatter over the same inputs).
    pn = sci("Abies alba Mill.")
    assert pn.canonical_name() == "Abies alba Mill."
    assert pn.canonical_name_without_authorship() == "Abies alba"
    assert pn.canonical_name_minimal() == "Abies alba"
    assert pn.canonical_name_complete() == "Abies alba Mill."
    assert pn.authorship_complete() == "Mill."
    assert str(pn) == "Abies alba Mill."  # __str__ is the canonical name

    # authorship-less name -> None
    assert sci("Abies alba").authorship_complete() is None

    # minimal drops the infrageneric genus + rank marker
    assert sci("Astragalus subg. Cercidothrix").canonical_name_minimal() == "Cercidothrix"

    # notho hybrid marker with its space
    assert nameparser.parse("×Agropogon littoralis").canonical_name() == "× Agropogon littoralis"

    # html markup italicises the name parts
    assert (
        sci("Abies alba Mill.").canonical_name_complete_html()
        == "<i>Abies</i> <i>alba</i> Mill."
    )


# ---- 5.0.0 informal / semistructured band (parse() returns ParsedName | Informal) ----


def test_informal_molecular_provisional_species():
    # Genus sp. <specimen tag> — parse() returns an Informal, not a ParsedName. Values are
    # Rust-authoritative (the same core is_informal/to_informal the Java + R bindings mirror).
    r = nameparser.parse("Serratia sp. RE1-2a")
    assert isinstance(r, nameparser.Informal)
    assert not isinstance(r, nameparser.ParsedName)
    assert r.taxon == "Serratia"
    assert r.taxon_rank == "GENUS"
    assert r.rank == "SPECIES"
    assert r.phrase == "RE1-2a"
    assert r.code is None
    assert r.to_dict() == {
        "taxon": "Serratia",
        "taxonRank": "GENUS",
        "rank": "SPECIES",
        "phrase": "RE1-2a",
    }
    assert "Serratia" in repr(r)
    assert r.canonical_name() == "Serratia sp. RE1-2a"  # round-trips to the canonical string
    assert str(r) == "Serratia sp. RE1-2a"  # __str__ is the canonical name, as for ParsedName


def test_informal_multi_token_specimen_tag_is_captured_as_phrase():
    # The 5.0.0 tag-capture enhancement (rescues the ~382k "tag not captured" corpus rows).
    r = nameparser.parse("Rhizobium sp. RMCC TR1811")
    assert isinstance(r, nameparser.Informal)
    assert r.taxon == "Rhizobium"
    assert r.taxon_rank == "GENUS"
    assert r.phrase == "RMCC TR1811"


def test_informal_bare_genus_sp_has_no_phrase():
    r = nameparser.parse("Rhizobium sp.")
    assert isinstance(r, nameparser.Informal)
    assert r.taxon == "Rhizobium"
    assert r.rank == "SPECIES"
    assert r.phrase is None
    assert r.canonical_name() == "Rhizobium sp."  # bare "Genus sp." — synthesised marker, no phrase


def test_cf_binomial_stays_parsed_not_informal():
    # A species epithet is present (a binomial core), so it stays a ParsedName — the qualifier is
    # an annotation, and specificAuthorship (unrepresentable by a flat anchor) is preserved.
    r = nameparser.parse("Salicornia cf. patula")
    assert isinstance(r, nameparser.ParsedName)
    assert not isinstance(r, nameparser.Informal)
    assert r.specific_epithet == "patula"
    assert r.type == "INFORMAL"  # the ParsedName's own type nuance is kept inside a Parsed result


def test_parse_all_mixes_parsed_and_informal_and_none():
    results = nameparser.parse_all(["Abies alba", "Serratia sp. RE1-2a", "Tobacco mosaic virus"])
    assert isinstance(results[0], nameparser.ParsedName)
    assert isinstance(results[1], nameparser.Informal)
    assert results[2] is None  # unparsable → None (never raises mid-batch)
