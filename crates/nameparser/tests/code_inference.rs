// SPDX-License-Identifier: Apache-2.0
//! Code inference from evidence in the name itself, measured against the codes ChecklistBank
//! datasets declare (`tools/code_inference_eval.py`).

mod common;
use common::*;
use nameparser::model::{NamePart, NomCode, Rank};

#[test]
fn a_hybrid_is_botanical() {
    // zoology names no hybrids, so the year after the author is no zoological evidence
    assert_name("×Agropogon P. Fourn. 1934")
        .monomial("Agropogon")
        .notho(&[NamePart::Generic])
        .comb_authors(Some("1934"), &["P.Fourn."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Typha x smirnovii", "Mavrodiev, 2000")
        .species("Typha", "smirnovii")
        .notho(&[NamePart::Specific])
        .comb_authors(Some("2000"), &["Mavrodiev"])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn an_approved_lists_citation_is_bacterial() {
    // the 1980 Approved Lists of Bacterial Names exist under the prokaryote code only
    assert_name_auth("Mycoplasma cynos", "Rosendal 1973 (Approved Lists 1980)")
        .species("Mycoplasma", "cynos")
        .comb_authors(Some("1973"), &["Rosendal"])
        .code(NomCode::Bacterial)
        .nom_note("Approved Lists 1980")
        .nothing_else();
}

#[test]
fn an_emendation_is_no_zoological_evidence() {
    // "emend." is prokaryote and botanical usage: the year beside it no longer makes the name
    // zoological
    assert_name_auth("Gangjinia", "Lee et al. 2011 emend. Yoon et al. 2014")
        .monomial_rank("Gangjinia", Rank::Unranked)
        .comb_authors(Some("2011"), &["Lee", "al."])
        .sensu("emend. Yoon et al. 2014")
        .nothing_else();
}
