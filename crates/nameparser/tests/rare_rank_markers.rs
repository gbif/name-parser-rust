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

/// Markers below the subspecies in older literature: `m.` (morpha, monstrositas or modificatio),
/// `monstr.`, `mod.`, `morpha`. `m.` used to be glued onto the species author as an initial or
/// read as the epithet, the real epithet as an author.
#[test]
fn infrasubspecific_markers_of_older_literature() {
    // the stray "?." is dropped
    assert_name("Phalaris canariensis L. m. ?. bracteata Jansen & Wacht.")
        .infra_species(
            "Phalaris",
            "canariensis",
            Rank::InfrasubspecificName,
            "bracteata",
        )
        .specific_authors(None, &["L."])
        .comb_authors(None, &["Jansen", "Wacht."])
        .nothing_else();
    // Breuning's morphae: not turned into a subspecies by the zoological code
    assert_name("Eunidia simplex m. bifuscomaculata Breuning, 1957")
        .infra_species(
            "Eunidia",
            "simplex",
            Rank::InfrasubspecificName,
            "bifuscomaculata",
        )
        .comb_authors(Some("1957"), &["Breuning"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Aquilegia vulgaris monstr. corniculata (Vill.) Graebn. & P.Graebn.")
        .infra_species(
            "Aquilegia",
            "vulgaris",
            Rank::InfrasubspecificName,
            "corniculata",
        )
        .comb_authors(None, &["Graebn.", "P.Graebn."])
        .bas_authors(None, &["Vill."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Cladonia gracilis mod. albinea Sandst.")
        .infra_species(
            "Cladonia",
            "gracilis",
            Rank::InfrasubspecificName,
            "albinea",
        )
        .comb_authors(None, &["Sandst."])
        .nothing_else();
    assert_name("Arvicola terrestris morpha subalpina")
        .infra_species("Arvicola", "terrestris", Rank::Morph, "subalpina")
        .code(NomCode::Zoological)
        .nothing_else();
    // a capital M. stays an initial
    assert_name("Abies alba M. Smith")
        .species("Abies", "alba")
        .comb_authors(None, &["M.Smith"])
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
