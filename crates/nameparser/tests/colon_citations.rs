// SPDX-License-Identifier: Apache-2.0
//! A colon in an authorship: an "in:" citation, a sanctioning author or a concept reference.

mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::NomCode;

#[test]
fn in_with_a_colon_is_an_in_citation() {
    // ~400 ChecklistBank authorships, read as a concept reference: the publication became the
    // taxonomic note
    assert_name_auth(
        "Aonidiella taxus",
        "Henderson & Hodgson in: Hodgson & Henderson, 2000",
    )
    .species("Aonidiella", "taxus")
    .comb_authors(Some("2000"), &["Henderson", "Hodgson"])
    .published_in("Hodgson & Henderson, 2000")
    .published_in_year(Some(2000))
    .nothing_else();
    assert_name_auth("Aus bus", "Pogue, in: Brown, 2019")
        .species("Aus", "bus")
        .comb_authors(Some("2019"), &["Pogue"])
        .published_in("Brown, 2019")
        .published_in_year(Some(2019))
        .nothing_else();
    assert_name_auth(
        "Mastigodiaptomus reidae",
        "Mercado-Salas, 2013 In: Gutiérrez-Aguirre, Mercado-Salas & Cervantes-Martínez, 2013",
    )
    .species("Mastigodiaptomus", "reidae")
    .comb_authors(Some("2013"), &["Mercado-Salas"])
    .published_in("Gutiérrez-Aguirre, Mercado-Salas & Cervantes-Martínez, 2013")
    .published_in_year(Some(2013))
    .code(NomCode::Zoological)
    .nothing_else();
    assert_name_auth("Aus bus", "Young {in}: Young & Lu, 1988")
        .species("Aus", "bus")
        .comb_authors(Some("1988"), &["Young"])
        .published_in("Young & Lu, 1988")
        .published_in_year(Some(1988))
        .nothing_else();
}

#[test]
fn a_year_after_the_sanctioning_author_is_the_names() {
    // it used to become the taxonomic note "Fr., 1821"
    assert_name_auth("Boletus edulis", "Bull. : Fr., 1821")
        .species("Boletus", "edulis")
        .comb_authors(Some("1821"), &["Bull."])
        .sanct_author("Fr.")
        .nothing_else();
    // without the comma it used to be dropped
    assert_name("Boletus edulis Bull.:Fr. 1821")
        .species("Boletus", "edulis")
        .comb_authors(Some("1821"), &["Bull."])
        .sanct_author("Fr.")
        .nothing_else();
    assert_name_auth("Clitocybe nebularis", "Pers.:Fr., 1801")
        .species("Clitocybe", "nebularis")
        .comb_authors(Some("1801"), &["Pers."])
        .sanct_author("Fr.")
        .nothing_else();
}

#[test]
fn a_sanctioned_basionym_keeps_its_sanctioning_author() {
    // 9.9k ChecklistBank authorships; Java dropped the basionym's sanctioning author
    assert_name_auth("Merulius lacrimans", "(Wulfen : Fr.) Schum.")
        .species("Merulius", "lacrimans")
        .comb_authors(None, &["Schum."])
        .bas_authors(None, &["Wulfen"])
        .bas_sanct_author("Fr.")
        .nothing_else();
    // sanctioned twice, by Persoon and then Fries
    assert_name_auth("Russula sanguinea", "(Bull. : Pers.) Fr. : Fr.")
        .species("Russula", "sanguinea")
        .comb_authors(None, &["Fr."])
        .bas_authors(None, &["Bull."])
        .sanct_author("Fr.")
        .bas_sanct_author("Pers.")
        .nothing_else();
    // a year after Fries' or Persoon's name is the basionym's own
    assert_name_auth("Agaricus compactus", "(Pers.:Fr., 1801) Fr.")
        .species("Agaricus", "compactus")
        .comb_authors(None, &["Fr."])
        .bas_authors(Some("1801"), &["Pers."])
        .bas_sanct_author("Fr.")
        .nothing_else();
}

#[test]
fn a_colon_after_a_year_is_still_a_concept_reference() {
    assert_name("Vespa emarginata Linnaeus, 1758: Fabricius, 1793")
        .species("Vespa", "emarginata")
        .comb_authors(Some("1758"), &["Linnaeus"])
        .sensu("Fabricius, 1793")
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn a_diacritic_coded_with_a_colon_is_restored() {
    // 8.6k ChecklistBank authorships from bryophyte sources; the colon was read as a sanctioning
    // author's ("C.Mu" sanctioned by "2ller")
    assert_name_auth("Polytrichum baldwinii", "C. Mu:2ller, 1896")
        .species("Polytrichum", "baldwinii")
        .comb_authors(Some("1896"), &["C.Müller"])
        .warning(&[warnings::HOMOGLYHPS])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Bryum algovicum", "Sendtn. ex C. Mu:2ller")
        .species("Bryum", "algovicum")
        .comb_authors(None, &["C.Müller"])
        .comb_ex_authors(&["Sendtn."])
        .warning(&[warnings::HOMOGLYHPS])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Syrrhopodon binsteadii", "The:4riot & Dixon, 1916")
        .species("Syrrhopodon", "binsteadii")
        .comb_authors(Some("1916"), &["Thériot", "Dixon"])
        .warning(&[warnings::HOMOGLYHPS])
        .code(NomCode::Botanical)
        .nothing_else();
    // alone, the coded author used to look like a strain code
    assert_name_auth("Bryum pallescens", "A:1ngstro:2m")
        .species("Bryum", "pallescens")
        .comb_authors(None, &["Ångström"])
        .warning(&[warnings::HOMOGLYHPS])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Grimmia unicolor", "Vondra:4c:3ek, 1950")
        .species("Grimmia", "unicolor")
        .comb_authors(Some("1950"), &["Vondráček"])
        .warning(&[warnings::HOMOGLYHPS])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Fissidens crispus", "Mun:6oz")
        .species("Fissidens", "crispus")
        .comb_authors(None, &["Muñoz"])
        .warning(&[warnings::HOMOGLYHPS])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Dicranella heteromalla", "(Hedw.) Corbie:9re")
        .species("Dicranella", "heteromalla")
        .comb_authors(None, &["Corbière"])
        .bas_authors(None, &["Hedw."])
        .warning(&[warnings::HOMOGLYHPS])
        .code(NomCode::Botanical)
        .nothing_else();
}
