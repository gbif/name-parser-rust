// SPDX-License-Identifier: Apache-2.0
//! Imprint years: the year printed on a work that was actually published in another year.
//!
//! ICZN Recommendation 22A.2.3 cites the actual year first, then the imprint year "enclosed in
//! parentheses or other brackets and quotation marks", and gives four equivalent forms:
//! `Storr, 1970 ("1969")`, `Storr, 1970 ["1969"]`, `Storr, 1970 (imprint 1969)` and
//! `Storr, 1970 (not 1969)`, plus `(Peters, 1876 ["1877"])` inside a basionym's parentheses
//! (<https://code.iczn.org/date-of-publication/article-22-citation-of-date/>). The examples
//! themselves are pinned in `impl_10.rs` (`iczn_imprint`); these carry every form into the
//! basionym and onto the quotation marks ChecklistBank sources use (gbif/name-parser-rust#31).

mod common;
use common::*;
use nameparser::model::NomCode;

#[test]
fn every_iczn_form_inside_the_basionym() {
    for input in [
        "Anomalopus truncatus (Peters, 1876 (\"1877\"))",
        "Anomalopus truncatus (Peters, 1876 [\"1877\"])",
        "Anomalopus truncatus (Peters, 1876 (imprint 1877))",
        "Anomalopus truncatus (Peters, 1876 (not 1877))",
        "Anomalopus truncatus (Peters, 1876 [1877])",
    ] {
        assert_name(input)
            .species("Anomalopus", "truncatus")
            .bas_authors(Some("1876"), &["Peters"])
            .bas_imprint_year("1877")
            .code(NomCode::Zoological)
            .nothing_else();
    }
}

#[test]
fn single_and_typographic_quotation_marks() {
    // CLB: "Athanasiadis, 2018 ['2019']" gave two stray "'" authors
    assert_name_auth("Carlskottsbergia", "Athanasiadis, 2018 ['2019']")
        .monomial("Carlskottsbergia")
        .comb_authors(Some("2018"), &["Athanasiadis"])
        .imprint_year("2019")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Ctenotus alacer Storr, 1970 ('1969')")
        .species("Ctenotus", "alacer")
        .comb_authors(Some("1970"), &["Storr"])
        .imprint_year("1969")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Anomalopus truncatus (Peters, 1876 ['1877'])")
        .species("Anomalopus", "truncatus")
        .bas_authors(Some("1876"), &["Peters"])
        .bas_imprint_year("1877")
        .code(NomCode::Zoological)
        .nothing_else();
    // CLB: Leptanilla
    assert_name_auth("Leptanilla israelis", "Kugler, 1987 (\u{201C}1986\u{201D})")
        .species("Leptanilla", "israelis")
        .comb_authors(Some("1987"), &["Kugler"])
        .imprint_year("1986")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Ctenotus alacer Storr, 1970 [\u{2018}1969\u{2019}]")
        .species("Ctenotus", "alacer")
        .comb_authors(Some("1970"), &["Storr"])
        .imprint_year("1969")
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn each_authorship_keeps_its_own_imprint_year() {
    assert_name("Anomalopus truncatus (Peters, 1876 [\"1877\"]) Smith, 1990 (not 1989)")
        .species("Anomalopus", "truncatus")
        .bas_authors(Some("1876"), &["Peters"])
        .bas_imprint_year("1877")
        .comb_authors(Some("1990"), &["Smith"])
        .imprint_year("1989")
        .nothing_else();
}

#[test]
fn an_apostrophe_in_an_author_name_stays() {
    // the quotation-mark rule only drops a "'" next to a year
    assert_name("Aus bus 't Hart, 1970 ['1969']")
        .species("Aus", "bus")
        .comb_authors(Some("1970"), &["'t Hart"])
        .imprint_year("1969")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Aus bus (d'Orbigny, 1839 ('1838'))")
        .species("Aus", "bus")
        .bas_authors(Some("1839"), &["d'Orbigny"])
        .bas_imprint_year("1838")
        .code(NomCode::Zoological)
        .nothing_else();
}
