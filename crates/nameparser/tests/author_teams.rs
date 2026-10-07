// SPDX-License-Identifier: Apache-2.0
//! How an authorship is cut into people.
//!
//! A Spanish `y` joins the two surnames of one person (`Bolívar y Pieltain`, `Dusmet y Alonso`)
//! far more often than it joins two people: of the 3,588 ChecklistBank rows with a capitalised
//! `X y Y`, about 3,200 are double surnames. Java read every `y` as `&`. It stays a separator
//! where the string shows two people: a hyphenated double surname before it (`Ruiz-Carranza y
//! Lynch`), initials after it (`Skelton y G.R.South`), an abbreviation before it (`Amy. y Serv.`),
//! or a list it closes (`Smith, Jones y Brown`). Two people written `Spix y Agassiz` are the cost.
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
}

#[test]
fn a_spanish_y_between_people_stays_a_separator() {
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
