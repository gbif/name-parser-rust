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

#[test]
fn an_unranked_marker_marks_an_unranked_name() {
    // #98: IPNI's "[unranked]" put the epithet into the authorship
    assert_name("Aetheorhiza bulbosa [unranked] montana (Willk.) Gand.")
        .infra_species("Aetheorhiza", "bulbosa", Rank::InfraspecificName, "montana")
        .bas_authors(None, &["Willk."])
        .comb_authors(None, &["Gand."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Viola [unranked] Mirabiles Nyman")
        .infrageneric_at("Viola", Rank::InfragenericName, "Mirabiles")
        .comb_authors(None, &["Nyman"])
        .nothing_else();
    assert_name_auth("Abelia unranked Biflorae", "Zabel")
        .infrageneric_at("Abelia", Rank::InfragenericName, "Biflorae")
        .comb_authors(None, &["Zabel"])
        .nothing_else();
    assert_name("Poa (unranked) Arctophila")
        .infrageneric_at("Poa", Rank::InfragenericName, "Arctophila")
        .nothing_else();
    // the marker the formatter writes for it reads back
    assert_name("Bromus hordeaceus infrasp. ferronii (Mabille) P.M.Sm.")
        .infra_species("Bromus", "hordeaceus", Rank::InfraspecificName, "ferronii")
        .bas_authors(None, &["Mabille"])
        .comb_authors(None, &["P.M.Sm."])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn an_infrageneric_marker_inside_the_brackets() {
    // #88: the epithet was lost, the bracket read as the basionym author "sect.Auriculella"
    assert_name("Achatinella (sect. Auriculella) Pfeiffer 1854")
        .infrageneric_at("Achatinella", Rank::SectionBotany, "Auriculella")
        .comb_authors(Some("1854"), &["Pfeiffer"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Zygopetalum (sect. Cheiradenia)", "(Lindl.) Kuntze")
        .infrageneric_at("Zygopetalum", Rank::SectionBotany, "Cheiradenia")
        .bas_authors(None, &["Lindl."])
        .comb_authors(None, &["Kuntze"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Begonia (sect. Diploclinium) labordei", "H. Lev.")
        .species_ig("Begonia", "Diploclinium", "labordei")
        .comb_authors(None, &["H.Lev."])
        .nothing_else();
}

#[test]
fn a_superspecies_is_a_species_aggregate() {
    // #87: the marker became the epithet, and all superspecies of a genus one name
    assert_name_auth("Eosembia supersp. thoracica", "(Ross, 2007)")
        .binomial("Eosembia", None, "thoracica", Rank::SpeciesAggregate)
        .bas_authors(Some("2007"), &["Ross"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Acestrura heliodor superspecies")
        .binomial("Acestrura", None, "heliodor", Rank::SpeciesAggregate)
        .nothing_else();
}

#[test]
fn a_lowercase_infrageneric_epithet_after_its_marker_is_capitalised() {
    // #89: it became a species epithet
    assert_name("Hygrocybe sect. obtusae (A.H. Sm. & Hesler) Bon")
        .infrageneric_at("Hygrocybe", Rank::SectionBotany, "Obtusae")
        .bas_authors(None, &["A.H.Sm.", "Hesler"])
        .comb_authors(None, &["Bon"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Limacinia subgen limacinula Saccardo & D.Saccardo")
        .infrageneric_at("Limacinia", Rank::Subgenus, "Limacinula")
        .comb_authors(None, &["Saccardo", "D.Saccardo"])
        .nothing_else();
    // an author abbreviated like a marker is none
    assert_name("Pocockia Ser. ex DC.")
        .monomial("Pocockia")
        .comb_authors(None, &["DC."])
        .comb_ex_authors(&["Ser."])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn a_greek_letter_in_brackets_numbers_an_infraspecific_name() {
    // #90: written out in brackets, the letter dropped the epithet; as β it was understood
    assert_name_auth("Achnanthes brevipes [beta] salina", "Kützing")
        .infra_species("Achnanthes", "brevipes", Rank::InfraspecificName, "salina")
        .comb_authors(None, &["Kützing"])
        .nothing_else();
    assert_name_auth(
        "Batrachospermum moniliforme (epsilon)viridis",
        "(Bory) J.E. Duby",
    )
    .infra_species(
        "Batrachospermum",
        "moniliforme",
        Rank::InfraspecificName,
        "viridis",
    )
    .bas_authors(None, &["Bory"])
    .comb_authors(None, &["J.E.Duby"])
    .code(NomCode::Botanical)
    .nothing_else();
    // after a rank marker it is a mere label
    assert_name_auth("Epithemia gibba var. .(gamma)parallela", "Grunow")
        .infra_species("Epithemia", "gibba", Rank::Variety, "parallela")
        .comb_authors(None, &["Grunow"])
        .nothing_else();
}
