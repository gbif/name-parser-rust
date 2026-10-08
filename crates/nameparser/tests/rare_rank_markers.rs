// SPDX-License-Identifier: Apache-2.0
//! Rank markers rarer than subsp./var./f. and sect./subg.: they used to be read as an epithet or
//! end up inside the authorship.

mod common;
use common::*;
use nameparser::model::{NamePart, NomCode, Rank};

#[test]
fn rare_infraspecific_markers() {
    assert_name("Scilla vindobonensis lus. alba (Beck) Kereszty")
        .infra_species("Scilla", "vindobonensis", Rank::Lusus, "alba")
        .comb_authors(None, &["Kereszty"])
        .bas_authors(None, &["Beck"])
        .code(NomCode::Botanical)
        .nothing_else();
    // Sudre's microgène of Rubus
    assert_name("Rubus hirtus Rchb. microg pectinatus Sudre & Gavet")
        .infra_species("Rubus", "hirtus", Rank::InfraspecificName, "pectinatus")
        .comb_authors(None, &["Sudre", "Gavet"])
        .specific_authors(None, &["Rchb."])
        .nothing_else();
    // the "sssp." typo of "ssp."
    assert_name("Centaurea subciliaris Boiss. & Heldr. sssp. acarnanica Matthas")
        .infra_species("Centaurea", "subciliaris", Rank::Subspecies, "acarnanica")
        .comb_authors(None, &["Matthas"])
        .specific_authors(None, &["Boiss.", "Heldr."])
        .nothing_else();
    // a marker word where the epithet belongs is the epithet
    assert_name("Haliotis cracherodii var. lusus Finlay, 1927")
        .infra_species("Haliotis", "cracherodii", Rank::Variety, "lusus")
        .comb_authors(Some("1927"), &["Finlay"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn rare_infrageneric_markers() {
    // Fries's tribus within a genus
    assert_name("Agaricus tr. Hypholoma Fr.")
        .infrageneric_at("Agaricus", Rank::InfragenericName, "Hypholoma")
        .comb_authors(None, &["Fr."])
        .nothing_else();
    assert_name("Hieracium unr. Verbasciformia Arv.-Touv.")
        .infrageneric_at("Hieracium", Rank::InfragenericName, "Verbasciformia")
        .comb_authors(None, &["Arv.-Touv."])
        .nothing_else();
    // a notho marker after the genus author
    assert_name("Aconitum W. Mucher nothosect. Acopellus")
        .infrageneric_at("Aconitum", Rank::SectionBotany, "Acopellus")
        .notho(&[NamePart::Infrageneric])
        .generic_authors(None, &["W.Mucher"])
        .code(NomCode::Botanical)
        .nothing_else();
}
