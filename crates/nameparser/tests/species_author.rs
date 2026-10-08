// SPDX-License-Identifier: Apache-2.0
//! The species author written inside an infraspecific or cultivar name — between the species
//! epithet and the rank marker, or before a cultivar epithet — is the species' authorship, kept in
//! `specific_authorship`. Only an autonym, which has no author of its own (ICN Art. 26.1), takes it
//! as its authorship.

mod common;
use common::*;
use nameparser::model::{NamePart, NameType, NomCode, Rank};

#[test]
fn a_species_author_before_the_rank_marker_is_kept() {
    // it used to be dropped
    assert_name("Festuca ovina L. subsp. guestfalica (Boenn. ex Rchb.) K.Richt.")
        .infra_species("Festuca", "ovina", Rank::Subspecies, "guestfalica")
        .specific_authors(None, &["L."])
        .bas_authors(None, &["Rchb."])
        .bas_ex_authors(None, &["Boenn."])
        .comb_authors(None, &["K.Richt."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Serjania meridionalis Cambess. var. paucidentata Radlk.")
        .infra_species("Serjania", "meridionalis", Rank::Variety, "paucidentata")
        .specific_authors(None, &["Cambess."])
        .comb_authors(None, &["Radlk."])
        .nothing_else();
}

#[test]
fn an_autonym_takes_the_species_author_as_its_own() {
    assert_name("Abies alba Mill. var. alba")
        .infra_species("Abies", "alba", Rank::Variety, "alba")
        .comb_authors(None, &["Mill."])
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
        .comb_authors(None, &["Hook.f.", "Wilson"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Parnassia foliosa Hook.f. et Thomson var. japonica (Nakai) Ohwi")
        .infra_species("Parnassia", "foliosa", Rank::Variety, "japonica")
        .comb_authors(None, &["Ohwi"])
        .bas_authors(None, &["Nakai"])
        .specific_authors(None, &["Hook.f.", "Thomson"])
        .code(NomCode::Botanical)
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
        .code(NomCode::Botanical)
        .nothing_else();
    // a hybrid sign after the rank marker
    assert_name("Eriophorum ×medium Andersson subsp. ×medium")
        .infra_species("Eriophorum", "medium", Rank::Subspecies, "medium")
        .notho(&[NamePart::Specific, NamePart::Infraspecific])
        .comb_authors(None, &["Andersson"])
        .code(NomCode::Botanical)
        .nothing_else();
}
