// SPDX-License-Identifier: Apache-2.0
//! The species author written inside an infraspecific or cultivar name — between the species
//! epithet and the rank marker, or before a cultivar epithet — is the species' authorship, kept in
//! `specific_authorship`. Only an autonym, which has no author of its own (ICN Art. 26.1), takes it
//! as its authorship.

mod common;
use common::*;
use nameparser::model::{warnings, NamePart, NameType, NomCode, Rank};

#[test]
fn a_species_author_before_the_rank_marker_is_kept() {
    // it used to be dropped
    assert_name("Festuca ovina L. subsp. guestfalica (Boenn. ex Rchb.) K.Richt.")
        .infra_species("Festuca", "ovina", Rank::Subspecies, "guestfalica")
        .specific_authors(None, &["L."])
        .bas_authors(None, &["Rchb."])
        .bas_ex_authors(None, &["Boenn."])
        .comb_authors(None, &["K.Richt."])
        .nothing_else();
    assert_name("Serjania meridionalis Cambess. var. paucidentata Radlk.")
        .infra_species("Serjania", "meridionalis", Rank::Variety, "paucidentata")
        .specific_authors(None, &["Cambess."])
        .comb_authors(None, &["Radlk."])
        .nothing_else();
}

#[test]
fn an_autonym_keeps_the_species_author_apart() {
    // an autonym has no author of its own (ICN Art. 26.1, #86; Java made it the autonym's)
    assert_name("Abies alba Mill. var. alba")
        .infra_species("Abies", "alba", Rank::Variety, "alba")
        .specific_authors(None, &["Mill."])
        .nothing_else();
    assert_name("Crepis arenaria (Pomel) Pomel subsp. arenaria")
        .infra_species("Crepis", "arenaria", Rank::Subspecies, "arenaria")
        .specific_authors(None, &["Pomel"])
        .specific_bas_authors(None, &["Pomel"])
        .nothing_else();
    // given after the autonym, it is the name's as written
    assert_name("Abies alba subsp. alba Mill.")
        .infra_species("Abies", "alba", Rank::Subspecies, "alba")
        .comb_authors(None, &["Mill."])
        .nothing_else();
}

#[test]
fn a_separate_authorship_repeating_an_autonyms_species_author_is_no_authorship_of_it() {
    // VASCAN repeats the species author in the authorship column (#86)
    assert_name_auth(
        "Amelanchier alnifolia (Nuttall) Nuttall ex M. Roemer var. alnifolia",
        "(Nuttall) Nuttall ex M. Roemer",
    )
    .infra_species("Amelanchier", "alnifolia", Rank::Variety, "alnifolia")
    .specific_authors(None, &["M.Roemer"])
    .specific_ex_authors(&["Nuttall"])
    .specific_bas_authors(None, &["Nuttall"])
    .nothing_else();
}

#[test]
fn a_cultivar_without_its_own_author_keeps_the_species_author_apart() {
    assert_name("Acer campestre L. cv. 'nanum'")
        .cultivar_sp("Acer", "campestre", "nanum")
        .specific_authors(None, &["L."])
        .nothing_else();
    // with its own author the cultivar author is the name's authorship, as before
    assert_name("Acer campestre L. cv. 'Elsrijk' Broerse")
        .cultivar_sp("Acer", "campestre", "Elsrijk")
        .specific_authors(None, &["L."])
        .comb_authors(None, &["Broerse"])
        .nothing_else();
    // a cultivar author starting with a particle, and a species author after a hybrid sign
    assert_name("Acer saccharinum L. cv. 'Asplenifolium' de Bie")
        .cultivar_sp("Acer", "saccharinum", "Asplenifolium")
        .specific_authors(None, &["L."])
        .comb_authors(None, &["de Bie"])
        .nothing_else();
    assert_name("Symphoricarpos x chenaultii Rehder cv. 'Erect' Door. ex Koppeschaar")
        .cultivar_sp("Symphoricarpos", "chenaultii", "Erect")
        .notho(&[NamePart::Specific])
        .specific_authors(None, &["Rehder"])
        .comb_authors(None, &["Koppeschaar"])
        .comb_ex_authors(&["Door."])
        .nothing_else();
}

#[test]
fn a_provisional_infraspecific_designation_keeps_the_species_author_apart() {
    // the phrase has no author of its own
    assert_name("Acacia mutabilis Maslin subsp. Young River (G.F. Craig 2052)")
        .binomial("Acacia", None, "mutabilis", Rank::Subspecies)
        .phrase("Young River (G.F. Craig 2052)")
        .specific_authors(None, &["Maslin"])
        .type_(NameType::Informal)
        .nothing_else();
}

#[test]
fn a_species_author_with_a_filius_in_a_team_ends_at_the_rank_marker() {
    assert_name("Conostomum pusillum Hook.f. & Wilson var. pusillum")
        .infra_species("Conostomum", "pusillum", Rank::Variety, "pusillum")
        .specific_authors(None, &["Hook.f.", "Wilson"])
        .nothing_else();
    assert_name("Parnassia foliosa Hook.f. et Thomson var. japonica (Nakai) Ohwi")
        .infra_species("Parnassia", "foliosa", Rank::Variety, "japonica")
        .comb_authors(None, &["Ohwi"])
        .bas_authors(None, &["Nakai"])
        .specific_authors(None, &["Hook.f.", "Thomson"])
        .nothing_else();
}

#[test]
fn more_species_author_shapes_before_the_rank_marker() {
    // the Dutch "'t", and an "ex" right after the basionym bracket
    assert_name("Phedimus aizoon (L.) 't Hart var. floribundus (Nakai) H.Ohba")
        .infra_species("Phedimus", "aizoon", Rank::Variety, "floribundus")
        .comb_authors(None, &["H.Ohba"])
        .bas_authors(None, &["Nakai"])
        .specific_authors(None, &["'t Hart"])
        .specific_bas_authors(None, &["L."])
        .nothing_else();
    // a hybrid sign after the rank marker
    assert_name("Eriophorum ×medium Andersson subsp. ×medium")
        .infra_species("Eriophorum", "medium", Rank::Subspecies, "medium")
        .notho(&[NamePart::Specific, NamePart::Infraspecific])
        .specific_authors(None, &["Andersson"])
        .code(NomCode::Botanical)
        .nothing_else();
}

/// A numeral epithet with a space for its hyphen: `Sphex 2 punctata` lost its number and
/// `Episyron rufipes 7. maculatus` its whole epithet (ChecklistBank dataset 2141).
#[test]
fn a_spaced_numeral_epithet() {
    assert_name("Sphex 2 punctata Fabricius, 1793")
        .species("Sphex", "2-punctata")
        .comb_authors(Some("1793"), &["Fabricius"])
        .nothing_else();
    assert_name("Coccinella 12 guttata Poda, 1761")
        .species("Coccinella", "12-guttata")
        .comb_authors(Some("1761"), &["Poda"])
        .nothing_else();
    assert_name("Episyron rufipes 7. maculatus")
        .infra_species(
            "Episyron",
            "rufipes",
            Rank::InfraspecificName,
            "7-maculatus",
        )
        .nothing_else();
    assert_name("Compsosoma (Compsosoma) 5 notatum")
        .species_ig("Compsosoma", "Compsosoma", "5-notatum")
        .nothing_else();
    // a number before any other word is no numeral epithet
    if let nameparser::ParseResult::Parsed(p) =
        nameparser::parse("Bilimbia sphaeroides 1 muscorum", None, None, None)
    {
        assert_ne!(p.infraspecific_epithet.as_deref(), Some("1-muscorum"));
    }
}

#[test]
fn a_capitalised_species_epithet_is_put_in_lower_case() {
    // #99: Linnaeus's "Ovis Aries" — the epithet became part of an author "Aries Linnæus"
    assert_name_hinted(
        "Ovis Aries Linnæus, 1758",
        None,
        Some(Rank::Species),
        Some(NomCode::Zoological),
    )
    .species("Ovis", "aries")
    .comb_authors(Some("1758"), &["Linnæus"])
    .code(NomCode::Zoological)
    .warning(&[warnings::CAPITALISED_EPITHET])
    .nothing_else();
    assert_name_auth("Hyla Everetti", "Boulenger, 1897")
        .species("Hyla", "everetti")
        .comb_authors(Some("1897"), &["Boulenger"])
        .warning(&[warnings::CAPITALISED_EPITHET])
        .nothing_else();
    assert_name("Ischyodon Johnsonii (Agassiz, 1843)")
        .species("Ischyodon", "johnsonii")
        .bas_authors(Some("1843"), &["Agassiz"])
        .warning(&[warnings::CAPITALISED_EPITHET])
        .nothing_else();
    // a genus with a surname of two words keeps it
    assert_name_auth("Homalaspis Milne Edwards, 1863", "Milne Edwards 1863")
        .monomial("Homalaspis")
        .comb_authors(Some("1863"), &["Milne Edwards"])
        .nothing_else();
}

#[test]
fn fil_before_a_rank_marker_is_a_filius() {
    // #221: as "f." is, the infraspecific name was left unparsed
    assert_name("Allium bourgeaui Rech. fil. subsp. bourgeaui")
        .infra_species("Allium", "bourgeaui", Rank::Subspecies, "bourgeaui")
        .specific_authors(None, &["Rech.fil."])
        .autonym()
        .nothing_else();
    assert_name("Allium bourgeaui Rech. f. subsp. bourgeaui")
        .infra_species("Allium", "bourgeaui", Rank::Subspecies, "bourgeaui")
        .specific_authors(None, &["Rech.f."])
        .autonym()
        .nothing_else();
    assert_name("Allium cepa Rech. fil. var. alba Smith")
        .infra_species("Allium", "cepa", Rank::Variety, "alba")
        .specific_authors(None, &["Rech.fil."])
        .comb_authors(None, &["Smith"])
        .nothing_else();
}

#[test]
fn a_code_is_no_author() {
    // #213: a capitalised word with a digit after a uninomial is an interim code
    for (input, taxon, code) in [
        ("Priscula Dup9", "Priscula", "Dup9"),
        ("Priscula Ven02/80-085", "Priscula", "Ven02/80-085"),
        ("Actia Janzen05", "Actia", "Janzen05"),
        ("Eois BioLep24", "Eois", "BioLep24"),
        ("Adiopa Poole01DHJ01", "Adiopa", "Poole01DHJ01"),
        // a trap's number looks like a year
        ("Capitonius Malaise1924", "Capitonius", "Malaise1924"),
    ] {
        assert_informal(input)
            .taxon(taxon)
            .taxon_rank(Rank::Unranked)
            .rank(Rank::Unranked)
            .phrase(code)
            .nothing_else();
    }
    // with a slash after a binomial, a code as any other
    assert_informal("Agrobacterium pusense L2/2/1")
        .taxon("Agrobacterium pusense")
        .taxon_rank(Rank::Species)
        .rank(Rank::Species)
        .phrase("L2/2/1")
        .nothing_else();
    // a year glued to the author is split off
    assert_name("Tympanuchus cupido (Linnaeus1758)")
        .species("Tympanuchus", "cupido")
        .bas_authors(Some("1758"), &["Linnaeus"])
        .nothing_else();
    assert_name("Antennella ritchiei Totton1930")
        .species("Antennella", "ritchiei")
        .comb_authors(Some("1930"), &["Totton"])
        .nothing_else();
    assert_name("Bacillaria paxillifera (O.F. Müller) T. Marsson1901")
        .species("Bacillaria", "paxillifera")
        .bas_authors(None, &["O.F.Müller"])
        .comb_authors(Some("1901"), &["T.Marsson"])
        .nothing_else();
    // an isolate's name after an organism label stays one
    assert_informal("Cyanobacteria bacterium Shimoda2014")
        .taxon("Cyanobacteria")
        .phrase("bacterium Shimoda2014")
        .rank(Rank::Species)
        .taxon_rank(Rank::Genus)
        .nothing_else();
    assert_informal("Hypoderaeum sp. Hubei2014")
        .taxon("Hypoderaeum")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. Hubei2014")
        .nothing_else();
    assert_name_auth("Antennella ritchiei", "Totton1930")
        .species("Antennella", "ritchiei")
        .comb_authors(Some("1930"), &["Totton"])
        .nothing_else();
    // what is left is flagged
    assert_name("Aus bus Smith2, 1900")
        .species("Aus", "bus")
        .comb_authors(Some("1900"), &["Smith2"])
        .warning(&[warnings::AUTHOR_WITH_NUMBER])
        .nothing_else();
}
