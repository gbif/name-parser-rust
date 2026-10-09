// SPDX-License-Identifier: Apache-2.0
//! ChecklistBank's call shape: the name, its authorship passed separately, and the record's rank
//! (and sometimes its code) as hints — `parse(scientificName, authorship, rank, code)`.
//!
//! Every assertion here also runs on the embedded and redundant paths (see `common/paths.rs`), so
//! these pin behaviour that must not depend on which column the authorship came in.

mod common;
use common::*;
use nameparser::model::{warnings, NomCode, Rank};

#[test]
fn a_separate_authorship_drives_code_inference() {
    // a zoological author-year citation, whichever column it arrives in
    assert_name_auth("Bombus rubicundus", "Smith, 1854")
        .species("Bombus", "rubicundus")
        .comb_authors(Some("1854"), &["Smith"])
        .code(NomCode::Zoological)
        .nothing_else();
    // a botanical recombination without a year
    assert_name_auth("Cybianthus brasiliensis", "(Mez) Lundell")
        .species("Cybianthus", "brasiliensis")
        .bas_authors(None, &["Mez"])
        .comb_authors(None, &["Lundell"])
        .code(NomCode::Botanical)
        .nothing_else();
    // a zoological trinomial is a subspecies once the code is known
    assert_name_auth("Barbichthys laevis sumatranus", "Volz, 1904")
        .infra_species("Barbichthys", "laevis", Rank::Subspecies, "sumatranus")
        .comb_authors(Some("1904"), &["Volz"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn the_separate_authorship_wins_and_drives_the_code_when_both_are_given() {
    // A caller (unusually) passing a different authorship than the name string carries: the
    // separate one is applied last, so it is the name's authorship, and it decides the code. The
    // conflict is flagged.
    assert_name_auth("Aus bus Smith, 1900", "(L.) Mill.")
        .species("Aus", "bus")
        .bas_authors(None, &["L."])
        .comb_authors(None, &["Mill."])
        .code(NomCode::Botanical)
        .warning(&[warnings::AUTHORSHIP_CONFLICT])
        .nothing_else();
    // it replaces the whole authorship: no basionym author is left from the name string
    assert_name_auth("Aus bus (L.) Smith", "Mill.")
        .species("Aus", "bus")
        .comb_authors(None, &["Mill."])
        .warning(&[warnings::AUTHORSHIP_CONFLICT])
        .nothing_else();
    // an authorship of another part of the name is flagged, the name string's terminal one kept
    assert_name_auth(
        "Cyprinus carpio Linnaeus, 1758 ssp. murgo Dybowski, 1869",
        "Linnaeus, 1758",
    )
    .infra_species("Cyprinus", "carpio", Rank::Subspecies, "murgo")
    .specific_authors(Some("1758"), &["Linnaeus"])
    .comb_authors(Some("1869"), &["Dybowski"])
    .code(NomCode::Zoological)
    .warning(&[warnings::AUTHORSHIP_CONFLICT])
    .nothing_else();
    // holding less is no conflict, and the name string's year and brackets stay: no citations mixed
    assert_name_auth(
        "Mulleripicus funebris (Valenciennes, 1826)",
        "Achille Valenciennes",
    )
    .species("Mulleripicus", "funebris")
    .bas_authors(Some("1826"), &["Valenciennes"])
    .nothing_else();
    assert_name_auth("Dumbletonius Dugdale, 1986", "Dugdale")
        .monomial("Dumbletonius")
        .comb_authors(Some("1986"), &["Dugdale"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn a_rank_hint_ranks_a_trinomial_without_a_marker() {
    assert_name_hinted(
        "Abies alba alpina",
        Some("Mill."),
        Some(Rank::Variety),
        None,
    )
    .infra_species("Abies", "alba", Rank::Variety, "alpina")
    .comb_authors(None, &["Mill."])
    .nothing_else();
    assert_name_hinted(
        "Vulpes vulpes silaceus",
        Some("Miller, 1907"),
        Some(Rank::Subspecies),
        None,
    )
    .infra_species("Vulpes", "vulpes", Rank::Subspecies, "silaceus")
    .comb_authors(Some("1907"), &["Miller"])
    .code(NomCode::Zoological)
    .nothing_else();
    // the hint names the rank even where the code would suggest another one
    assert_name_hinted(
        "Apalis jacksoni jacksoni",
        Some("Sharpe, 1891"),
        Some(Rank::Form),
        None,
    )
    .infra_species("Apalis", "jacksoni", Rank::Form, "jacksoni")
    .comb_authors(Some("1891"), &["Sharpe"])
    .code(NomCode::Zoological)
    .nothing_else();
    // a cultivar rank needs a cultivar epithet, which a plain trinomial does not have
    assert_name_hinted(
        "Abies alba alpina",
        Some("Mill."),
        Some(Rank::Cultivar),
        None,
    )
    .infra_species("Abies", "alba", Rank::InfraspecificName, "alpina")
    .comb_authors(None, &["Mill."])
    .nothing_else();
}

#[test]
fn a_code_hint_wins_over_the_authorship() {
    // the record's code is the dataset's statement; an author-year citation only infers one
    assert_name_hinted(
        "Aus bus",
        Some("Smith, 1900"),
        None,
        Some(NomCode::Botanical),
    )
    .species("Aus", "bus")
    .comb_authors(Some("1900"), &["Smith"])
    .code(NomCode::Botanical)
    .nothing_else();
}

#[test]
fn a_cultivar_keeps_its_species_author_when_the_cultivar_author_comes_separately() {
    assert_name_auth("Acer campestre L. cv. 'Elsrijk'", "Broerse")
        .cultivar_sp("Acer", "campestre", "Elsrijk")
        .specific_authors(None, &["L."])
        .comb_authors(None, &["Broerse"])
        .nothing_else();
}

#[test]
fn spec_with_a_separate_authorship_is_the_published_epithet() {
    // the dot-less `spec` + an authorship rescue sees the authorship in its own column too
    assert_name_auth("Hemicloeina spec", "Platnick, 2002")
        .species("Hemicloeina", "spec")
        .comb_authors(Some("2002"), &["Platnick"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_auth("Gobiosoma spec", "(Ginsburg, 1939)")
        .species("Gobiosoma", "spec")
        .bas_authors(Some("1939"), &["Ginsburg"])
        .code(NomCode::Zoological)
        .nothing_else();
    // with the dot it stays provisional, and the authorship rides along in the phrase
    assert_informal_hinted("Hemicloeina spec.", Some("Platnick, 2002"), None, None)
        .taxon("Hemicloeina")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("spec. Platnick, 2002")
        .nothing_else();
}

/// What a separate authorship leaves unparsed is added to what the name string left unparsed,
/// not put in its place.
#[test]
fn the_unparsed_rest_of_both_columns_is_kept() {
    assert_name_auth("Abies alba Mill. foo 1234 bar", "L. foo bar baz")
        .species("Abies", "alba")
        .comb_authors(None, &["L."])
        .partial("foo 1234 bar foo bar baz")
        .warning(&[warnings::AUTHORSHIP_CONFLICT])
        .nothing_else();
}
