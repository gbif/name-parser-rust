// SPDX-License-Identifier: Apache-2.0
//! 5.0.0 `NameType::Identifier` — anchorless, scheme-prefixed *machine identifiers* (UNITE SH,
//! BOLD BINs, OTU/ASV/… operational units, standalone culture-collection accessions), reclassified
//! out of the catch-all `Other`; plus Part B — a culture-collection accession *trailing a
//! determined name* is captured as the `phrase` instead of being misread as an author. See
//! `docs/superpowers/specs/2026-07-14-nametype-identifier-design.md`.

mod common;
use common::*;
use nameparser::model::NameType;

// ---- Part A: anchorless machine identifiers -> NameType::Identifier ---------------------------

#[test]
fn unite_sh_and_bold_bins_are_identifiers() {
    assert_unparsable("SH1957732.10FU", NameType::Identifier);
    assert_unparsable("BOLD:AAA0001", NameType::Identifier);
    // a lowercase SH still canonicalises to uppercase AND is now an Identifier (was OTHER)
    assert_unparsable("sh460441.07fu", NameType::Identifier);
}

#[test]
fn otu_asv_and_assembly_units_are_identifiers() {
    for s in [
        "OTU-17",
        "OTU 34",
        "ASV_103",
        "zOTU44",
        "MAG-24",
        "UBA12345",
        "GCA_000123",
    ] {
        assert_unparsable(s, NameType::Identifier);
    }
}

#[test]
fn standalone_culture_collection_accessions_are_identifiers() {
    for s in [
        "DSM 10",
        "ATCC 11775",
        "ATCC BAA-123",
        "CBS 123.89",
        "LMG 6923T",
        "ATCC-11775",
        "JCM 1002",
    ] {
        assert_unparsable(s, NameType::Identifier);
    }
}

// ---- Boundary: NOT identifiers ---------------------------------------------------------------

#[test]
fn a_genus_starting_with_an_identifier_prefix_stays_a_name() {
    // "Uba" is a real beetle genus, not the UBA scheme (the whole-string + trailing-digit guard).
    assert_name("Uba fallai Fletcher, 1938").species("Uba", "fallai");
}

#[test]
fn descriptive_junk_stays_other_not_identifier() {
    // prose descriptors are not machine identifiers — they stay OTHER (or another non-Identifier).
    assert_unparsable("Clade A", NameType::Other);
}

// ---- Part B: a trailing culture accession is captured as the phrase --------------------------

#[test]
fn trailing_culture_accession_on_a_binomial_becomes_the_phrase() {
    // "DSM 19832" / "ATCC 11775" is a strain annotation, not an author — captured verbatim
    // (acronym included) as the phrase; the binomial core stays Parsed with type INFORMAL.
    assert_name("Aquimarina muelleri DSM 19832")
        .species("Aquimarina", "muelleri")
        .type_(NameType::Informal)
        .phrase("DSM 19832");
    assert_name("Escherichia coli ATCC 11775")
        .species("Escherichia", "coli")
        .type_(NameType::Informal)
        .phrase("ATCC 11775");
}

#[test]
fn a_real_trailing_author_is_not_mistaken_for_an_accession() {
    // "Mill." is an author, not a curated collection acronym -> stays SCIENTIFIC with authorship.
    assert_name("Abies alba Mill.")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .type_(NameType::Scientific)
        .nothing_else();
}
