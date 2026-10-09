// SPDX-License-Identifier: Apache-2.0
//! How an authorship is cut into people.
//!
//! A Spanish `y` joins the two surnames of one person (`Bolívar y Pieltain`, `Dusmet y Alonso`)
//! as often as it joins two people (`Spix y Agassiz`, `Rivero y Serna`), and the string alone
//! cannot tell which. An `X y Y` is one author only when the pair is listed in
//! `resources/double-surnames.tsv` (164 people from ChecklistBank, COL, Wikidata and the CLB person
//! registry); any other `y` separates two people, as in Java.
//!
//! Two initials sharing one surname are two people of that name (`A. & D. Löve`, `H. & A.
//! Adams`), where Java kept the lone `A.` as an author of its own.

mod common;
use common::*;
use nameparser::model::NomCode;

#[test]
fn a_spanish_y_joins_the_two_surnames_of_one_person() {
    assert_single_author("Martinez y Saez")
        .comb_authors(None, &["Martinez y Saez"])
        .nothing_else();
    assert_authorship("Dusmet y Alonso, 1915", &["Dusmet y Alonso"])
        .comb_authors(Some("1915"), &["Dusmet y Alonso"])
        .nothing_else();
    assert_authorship("(Vilanova y Piera, 1863)", &[])
        .bas_authors(Some("1863"), &["Vilanova y Piera"])
        .nothing_else();
    // another separator in the team: the y is inside a person
    assert_authorship("Almera & Bofill y Poch, 1894", &["Almera", "Bofill y Poch"])
        .comb_authors(Some("1894"), &["Almera", "Bofill y Poch"])
        .nothing_else();
    assert_authorship("Truan y Luard & Witt, 1888", &["Truan y Luard", "Witt"])
        .comb_authors(Some("1888"), &["Truan y Luard", "Witt"])
        .nothing_else();
    // leading initials, and an abbreviated maternal surname
    assert_single_author("J.J.Rodríguez y Femenías")
        .comb_authors(None, &["J.J.Rodríguez y Femenías"])
        .nothing_else();
    assert_authorship("Caballero y C., 1943", &["Caballero y C."])
        .comb_authors(Some("1943"), &["Caballero y C."])
        .nothing_else();
    // without accents, misspelt or abbreviated as the data writes them
    assert_single_author("Rodriguez y Femenias")
        .comb_authors(None, &["Rodriguez y Femenias"])
        .nothing_else();
    assert_authorship("Bolivar y Pidtain, 1930", &["Bolivar y Pidtain"])
        .comb_authors(Some("1930"), &["Bolivar y Pidtain"])
        .nothing_else();
    assert_single_author("Dus. y Alon.")
        .comb_authors(None, &["Dus. y Alon."])
        .nothing_else();
    // a particle opening the maternal surname
    assert_authorship("Graells y de la Agüera, 1858", &["Graells y de la Agüera"])
        .comb_authors(Some("1858"), &["Graells y de la Agüera"])
        .nothing_else();
}

#[test]
fn a_spanish_y_between_people_stays_a_separator() {
    // two people the list does not hold
    assert_authorship("Spix y Agassiz, 1829", &["Spix", "Agassiz"])
        .comb_authors(Some("1829"), &["Spix", "Agassiz"])
        .nothing_else();
    assert_authorship("(Rivero y Serna, 1986)", &[])
        .bas_authors(Some("1986"), &["Rivero", "Serna"])
        .nothing_else();
    // a hyphenated double surname already holds both surnames of its person
    assert_authorship("Ruiz-Carranza y Lynch, 1991", &["Ruiz-Carranza", "Lynch"])
        .comb_authors(Some("1991"), &["Ruiz-Carranza", "Lynch"])
        .nothing_else();
    // initials start the next person
    assert_authorship("Skelton y G.R.South", &["Skelton", "G.R.South"])
        .comb_authors(None, &["Skelton", "G.R.South"])
        .nothing_else();
    // abbreviated surnames (Amyot & Serville)
    assert_authorship("Amy. y Serv.", &["Amy.", "Serv."])
        .comb_authors(None, &["Amy.", "Serv."])
        .nothing_else();
    // the y closes a list
    assert_authorship("Smith, Jones y Brown, 1990", &["Smith", "Jones", "Brown"])
        .comb_authors(Some("1990"), &["Smith", "Jones", "Brown"])
        .nothing_else();
}

#[test]
fn spanish_double_surnames_in_a_name_string() {
    assert_name("Carabus (Tanaocarabus) hendrichsi Bolvar y Pieltain, Rotger & Coronado 1967")
        .species_ig("Carabus", "Tanaocarabus", "hendrichsi")
        .comb_authors(Some("1967"), &["Bolvar y Pieltain", "Rotger", "Coronado"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Dicranum saxatile Lagasca y Segura, García & Clemente y Rubio, 1802")
        .species("Dicranum", "saxatile")
        .comb_authors(
            Some("1802"),
            &["Lagasca y Segura", "García", "Clemente y Rubio"],
        )
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn two_initials_share_the_surname() {
    assert_authorship("A. & D. Löve", &["A.Löve", "D.Löve"])
        .comb_authors(None, &["A.Löve", "D.Löve"])
        .nothing_else();
    assert_authorship("(H. & A. Adams, 1853)", &[])
        .bas_authors(Some("1853"), &["H.Adams", "A.Adams"])
        .nothing_else();
    assert_authorship("R. & G. Forst.", &["R.Forst.", "G.Forst."])
        .comb_authors(None, &["R.Forst.", "G.Forst."])
        .nothing_else();
    // Linnaeus keeps his own name
    assert_authorship("L. & D. Don", &["L.", "D.Don"])
        .comb_authors(None, &["L.", "D.Don"])
        .nothing_else();
}

/// "et al." glued into one word, as `etal`, `Etal` or `etal.`, is still "et al." — Java took it
/// for a surname or a second author, or stopped parsing at it (30 ChecklistBank rows).
#[test]
fn a_glued_etal_is_et_al() {
    assert_name("Pachypus baroniensis Ahrens, Bazzato, Lopez, etal, 2026")
        .species("Pachypus", "baroniensis")
        .comb_authors(Some("2026"), &["Ahrens", "Bazzato", "Lopez", "al."])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_authorship(
        "Ahrens, Bazzato, Lopez, etal, 2026",
        &["Ahrens", "Bazzato", "Lopez", "al."],
    )
    .comb_authors(Some("2026"), &["Ahrens", "Bazzato", "Lopez", "al."])
    .nothing_else();
    assert_authorship("Bianchi etal. 2015", &["Bianchi", "al."])
        .comb_authors(Some("2015"), &["Bianchi", "al."])
        .nothing_else();
    assert_authorship("(Ragsdale etal. 2011)", &[])
        .bas_authors(Some("2011"), &["Ragsdale", "al."])
        .nothing_else();
    assert_authorship("Ying Xu etal.,2012", &["Ying Xu", "al."])
        .comb_authors(Some("2012"), &["Ying Xu", "al."])
        .nothing_else();
    // after a conjunction: the "&" or "and" is the "et" already
    assert_authorship("Fang & etal, 2007", &["Fang", "al."])
        .comb_authors(Some("2007"), &["Fang", "al."])
        .nothing_else();
    assert_authorship("Wang & Etal, 2001", &["Wang", "al."])
        .comb_authors(Some("2001"), &["Wang", "al."])
        .nothing_else();
    assert_authorship("Xing and etal 2018", &["Xing", "al."])
        .comb_authors(Some("2018"), &["Xing", "al."])
        .nothing_else();
    // a surname that merely starts with it
    assert_authorship("Étallon, 1859", &["Étallon"])
        .comb_authors(Some("1859"), &["Étallon"])
        .nothing_else();
}

/// A capitalised "Et Al." is "et al." too — it gave the author "Al.", rendered "Wilson & Al.".
#[test]
fn a_capitalised_et_al_is_et_al() {
    for raw in [
        "Wilson Et Al., 2013",
        "Wilson ET AL., 2013",
        "Wilson et Al. 2013",
    ] {
        assert_name_auth("Abies alba", raw)
            .species("Abies", "alba")
            .comb_authors(Some("2013"), &["Wilson", "al."])
            .code(NomCode::Zoological)
            .nothing_else();
    }
    // a surname after "Et" stays one
    assert_authorship("Smith Et Alvarez", &["Smith", "Alvarez"])
        .comb_authors(None, &["Smith", "Alvarez"])
        .nothing_else();
}

/// A chain of ex-authors keeps them all, each once: Java kept only the last.
#[test]
fn a_chain_of_ex_authors_keeps_them_all() {
    assert_name_auth(
        "Festuca pachyphylla",
        "Degen ex Nyár. ex Csürös, Gergely & Pop",
    )
    .species("Festuca", "pachyphylla")
    .comb_authors(None, &["Csürös", "Gergely", "Pop"])
    .comb_ex_authors(&["Degen", "Nyár."])
    .code(NomCode::Botanical)
    .nothing_else();
    assert_name_auth("Ageratum conyzoides", "Sieber ex Sieber ex Steudel")
        .species("Ageratum", "conyzoides")
        .comb_authors(None, &["Steudel"])
        .comb_ex_authors(&["Sieber"])
        .code(NomCode::Botanical)
        .nothing_else();
}
