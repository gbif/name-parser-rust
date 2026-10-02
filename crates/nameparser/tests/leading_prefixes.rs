// SPDX-License-Identifier: Apache-2.0
//! Names that open with something other than a genus — a placeholder word (`unclassified`), a
//! generic organism label (`bacterium Ac10`), a classification path (`supf. Arrenuroidea fam.
//! Arrenuridae`) or a stray rank label (`Sp. Abacobius jekelii`). Surveyed from the 67.5M CLB
//! verbatim names: each used to lose the real taxon, read as the author of a bogus first word.

mod common;
use common::*;
use nameparser::model::NameType;

// ---- unclassified ------------------------------------------------------------------------------

#[test]
fn unclassified_is_a_placeholder() {
    for name in [
        "unclassified Aaadonta",
        "unclassified Aaadonta constricta",
        "unclassified eubacterium",
        "Unclassified Bacteria",
    ] {
        assert_unparsable(name, NameType::Placeholder);
    }
}

#[test]
fn unclassified_virus_keeps_its_virus_code() {
    assert_unparsable_code(
        "Grapevine red globe virus (unclassified)",
        NameType::Other,
        nameparser::model::NomCode::Virus,
    );
}

// ---- generic organism label + strain code ------------------------------------------------------

#[test]
fn organism_label_with_a_strain_code_is_an_identifier() {
    for name in [
        "bacterium Ac10",
        "archaeon Ea1",
        "bacterium A-15",
        "haloarchaeon BitternsUMGM",
        "bacterium Rauti",
        "bacterium str. S36",
        "bacterium sp. A15",
        "actinomycete P032.UGM060121-02",
        "marine actinobacterium F10",
        "soil bacterium PM2-P1-17",
        "delta proteobacterium S2250",
        "bacterium enrichment culture clone OB115",
        "fungal sp. ARIZ AZ0346",
        "actinobacterium SCGC AAA015-M09",
        "bacterium '081007-Grout Aid-gul'",
    ] {
        assert_unparsable(name, NameType::Identifier);
    }
}

#[test]
fn organism_label_without_a_code_is_unparsable_other() {
    for name in [
        "endosymbiont of Chlamys farreri",
        "bacterium symbiont of Osedax sp.",
        "coryneform bacterium",
        "anammox bacterium",
    ] {
        assert_unparsable(name, NameType::Other);
    }
}

#[test]
fn organism_word_as_a_missing_genus_epithet_is_untouched() {
    // a real epithet + author with the genus missing still takes the missing-genus reading
    assert_name("fungi Meigen, 1830")
        .species("?", "fungi")
        .type_(NameType::Placeholder)
        .comb_authors(Some("1830"), &["Meigen"])
        .code(nameparser::model::NomCode::Zoological)
        .warning(&["epithet without genus"])
        .nothing_else();
}

#[test]
fn named_symbiont_with_a_genus_is_untouched() {
    if let nameparser::ParseResult::Unparsable(e) = nameparser::parse(
        "Wolbachia endosymbiont of Drosophila simulans",
        None,
        None,
        None,
    ) {
        assert_ne!(e.type_, NameType::Identifier);
    }
}

#[test]
fn organism_label_with_a_culture_accession_or_a_letter_is_an_identifier() {
    for name in [
        "bacterium DSM 6505",
        "bacterium RCC 1888",
        "cyanobacterium PCC 7702",
        "actinobacterium D",
    ] {
        assert_unparsable(name, NameType::Identifier);
    }
}
