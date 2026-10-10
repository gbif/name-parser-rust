// SPDX-License-Identifier: Apache-2.0
//! 5.0.0 `NameType::Identifier` — anchorless, scheme-prefixed *machine identifiers* (UNITE SH,
//! BOLD BINs, OTU/ASV/… operational units, standalone culture-collection accessions), reclassified
//! out of the catch-all `Other`; plus Part B — a culture-collection accession *trailing a
//! determined name* is captured as the `phrase` instead of being misread as an author. See
//! `docs/nametype-identifier-design.md`.

mod common;
use common::*;
use nameparser::model::NameType;
use nameparser::model::{NamePart, NomCode, Rank};

// ---- Part A: anchorless machine identifiers -> NameType::Identifier ---------------------------

#[test]
fn unite_sh_and_bold_bins_are_identifiers() {
    assert_unparsable("SH1957732.10FU", NameType::Identifier);
    assert_unparsable("BOLD:AAA0001", NameType::Identifier);
    // a lowercase SH still canonicalises to uppercase AND is now an Identifier (was OTHER)
    assert_unparsable("sh460441.07fu", NameType::Identifier);
}

#[test]
fn a_bold_bin_or_sh_after_a_name_is_its_phrase() {
    // #101: the taxon the barcode cluster was identified to is kept, the code is the designation
    assert_informal("Decapoda sp. BOLD:AAF5952")
        .taxon("Decapoda")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. BOLD:AAF5952")
        .nothing_else();
    assert_informal("Russula sp. SH1957732.10FU")
        .taxon("Russula")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. SH1957732.10FU")
        .nothing_else();
    assert_informal("Tachinidae BOLD:AAA1234")
        .taxon("Tachinidae")
        .taxon_rank(Rank::Unranked)
        .rank(Rank::Unranked)
        .phrase("BOLD:AAA1234")
        .nothing_else();
    assert_name("Aus bus BOLD:AAF5952")
        .species("Aus", "bus")
        .phrase("BOLD:AAF5952")
        .type_(NameType::Informal)
        .nothing_else();
    assert_name("Labeo cf. bata BOLD:AAA1234")
        .species("Labeo", "bata")
        .qualifiers(&[(NamePart::Specific, "cf.")])
        .phrase("BOLD:AAA1234")
        .type_(NameType::Informal)
        .nothing_else();
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
    assert_name("Uba fallai Fletcher, 1938")
        .species("Uba", "fallai")
        .comb_authors(Some("1938"), &["Fletcher"])
        .code(NomCode::Zoological)
        .nothing_else();
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
        .phrase("DSM 19832")
        .nothing_else();
    assert_name("Escherichia coli ATCC 11775")
        .species("Escherichia", "coli")
        .type_(NameType::Informal)
        .phrase("ATCC 11775")
        .nothing_else();
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

#[test]
fn a_strain_or_voucher_code_after_a_name_is_its_phrase() {
    // #84: never an author, and never title-cased (`Scgc Aaa011-E11`)
    assert_name("Candidatus Liberibacter americanus PW_SP")
        .species("Liberibacter", "americanus")
        .candidatus()
        .phrase("PW_SP")
        .code(NomCode::Bacterial)
        .type_(NameType::Informal)
        .nothing_else();
    assert_name("Candidatus Iainarchaeum andersonii SCGC AAA011-E11")
        .species("Iainarchaeum", "andersonii")
        .candidatus()
        .phrase("SCGC AAA011-E11")
        .code(NomCode::Bacterial)
        .type_(NameType::Informal)
        .nothing_else();
    assert_name("Candidatus Caldatribacterium saccharofermentans OP9-77CS")
        .species("Caldatribacterium", "saccharofermentans")
        .candidatus()
        .phrase("OP9-77CS")
        .code(NomCode::Bacterial)
        .type_(NameType::Informal)
        .nothing_else();
    // an acronym and a number that is no year: a voucher, not the author RLB of 7550
    assert_name("Acacia cf. asperulacea RLB 7550")
        .species("Acacia", "asperulacea")
        .qualifiers(&[(NamePart::Specific, "cf.")])
        .phrase("RLB 7550")
        .type_(NameType::Informal)
        .nothing_else();
    assert_name("Acetobacter aceti IFO 3283")
        .species("Acetobacter", "aceti")
        .phrase("IFO 3283")
        .type_(NameType::Informal)
        .nothing_else();
    // an author in capitals with a year stays an author
    assert_name("Aus bus SMITH 1900")
        .species("Aus", "bus")
        .comb_authors(Some("1900"), &["Smith"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn everything_after_the_sp_of_an_aggregate_is_the_phrase() {
    // #84: the voucher was read as the author "E.S.2.Ufsm" and "sp." dropped
    assert_name("Aegla longirostri complex sp. UFSM ES2")
        .binomial("Aegla", None, "longirostri", Rank::SpeciesAggregate)
        .phrase("sp. UFSM ES2")
        .type_(NameType::Informal)
        .nothing_else();
    assert_name("Acropora hyacinthus complex sp. A JTL-2012")
        .binomial("Acropora", None, "hyacinthus", Rank::SpeciesAggregate)
        .phrase("sp. A JTL-2012")
        .type_(NameType::Informal)
        .nothing_else();
    assert_name("Mycobacterium tuberculosis complex sp. 3511-120")
        .binomial(
            "Mycobacterium",
            None,
            "tuberculosis",
            Rank::SpeciesAggregate,
        )
        .phrase("sp. 3511-120")
        .type_(NameType::Informal)
        .nothing_else();
}
