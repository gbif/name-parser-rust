// SPDX-License-Identifier: Apache-2.0
//! Long author teams (#123). The Java parser timed out on some of them; every author must come
//! through, in order, whatever the length of the team.

mod common;
use common::*;
use nameparser::model::{warnings, NomCode, Rank};

#[test]
fn a_long_team_keeps_every_author() {
    assert_name(
        "Abies alba de Puytorac, Batisse, Bohatier, Corliss, Deroux, Didier, Dragesco, \
         Fryd-Versavel, Grain, Grollere, Horasse, Mode, Laval, Roque, Savoie & Tuffrau, 1974",
    )
    .species("Abies", "alba")
    .comb_authors(
        Some("1974"),
        &[
            "de Puytorac",
            "Batisse",
            "Bohatier",
            "Corliss",
            "Deroux",
            "Didier",
            "Dragesco",
            "Fryd-Versavel",
            "Grain",
            "Grollere",
            "Horasse",
            "Mode",
            "Laval",
            "Roque",
            "Savoie",
            "Tuffrau",
        ],
    )
    .code(NomCode::Zoological)
    .nothing_else();
}

#[test]
fn a_team_of_ampersands_with_spelled_out_given_names() {
    assert_name(
        "Trachypithecus popa Roos & M. Helgen & Portela Miguez & May Lay Thant & Lwin & Ko Lin & \
         Lin & Mar Yi & Soe & Mar Hein & Nyein Nyein Myint & Ahmed & Chetry & Urh & Grace Veatch \
         & Duncan & Kamminga & A. H. Chua & Yao & Matauschek & Meyer & Liu & Li & Nadler & Fan & \
         Khac Quyet & Hofreiter & Zinner & Momberg 2020",
    )
    .species("Trachypithecus", "popa")
    .comb_authors(
        Some("2020"),
        &[
            "Roos",
            "M.Helgen",
            "Portela Miguez",
            "May Lay Thant",
            "Lwin",
            "Ko Lin",
            "Lin",
            "Mar Yi",
            "Soe",
            "Mar Hein",
            "Nyein Nyein Myint",
            "Ahmed",
            "Chetry",
            "Urh",
            "Grace Veatch",
            "Duncan",
            "Kamminga",
            "A.H.Chua",
            "Yao",
            "Matauschek",
            "Meyer",
            "Liu",
            "Li",
            "Nadler",
            "Fan",
            "Khac Quyet",
            "Hofreiter",
            "Zinner",
            "Momberg",
        ],
    )
    .warning(&[warnings::LONG_NAME])
    .nothing_else();
}

#[test]
fn a_team_of_forty_eight_authors() {
    assert_name(
        "Racosperma Bruneau, de Queiroz, Ringelberg, Borges, Bortoluzzi, Brown, Cardoso, Clark, \
         Conceição, Cota, Demeulenaere, de Stefano, Ebinger, Ferm, Fonseca-Cortés, Gagnon, \
         Grether, Guerra, Haston, Herendeen, Hernández, Hopkins, Huamantupa-Chuquimaco, Hughes, \
         Ickert-Bond, Iganci, Koenen, Lewis, de Lima, de Lima, Luckow, Marazzi, Maslin, Morales, \
         Morim, Murphy, O’Donnell, Oliveira, Oliveira, Rando, Ribeiro, Ribeiro, Santos, Seigler, \
         da Silva, Simon, Soares, and Terra 2024",
    )
    .monomial_rank("Racosperma", Rank::Unranked)
    .comb_authors(
        Some("2024"),
        &[
            "Bruneau",
            "de Queiroz",
            "Ringelberg",
            "Borges",
            "Bortoluzzi",
            "Brown",
            "Cardoso",
            "Clark",
            "Conceição",
            "Cota",
            "Demeulenaere",
            "de Stefano",
            "Ebinger",
            "Ferm",
            "Fonseca-Cortés",
            "Gagnon",
            "Grether",
            "Guerra",
            "Haston",
            "Herendeen",
            "Hernández",
            "Hopkins",
            "Huamantupa-Chuquimaco",
            "Hughes",
            "Ickert-Bond",
            "Iganci",
            "Koenen",
            "Lewis",
            "de Lima",
            "de Lima",
            "Luckow",
            "Marazzi",
            "Maslin",
            "Morales",
            "Morim",
            "Murphy",
            "O'Donnell",
            "Oliveira",
            "Oliveira",
            "Rando",
            "Ribeiro",
            "Ribeiro",
            "Santos",
            "Seigler",
            "da Silva",
            "Simon",
            "Soares",
            "Terra",
        ],
    )
    .code(NomCode::Zoological)
    .warning(&[warnings::LONG_NAME])
    .nothing_else();
}

#[test]
fn a_long_all_capital_team_is_title_cased() {
    assert_name("Embolemus brachypterus OLMI, GUGLIELMINO, MÜLLER, CAPRADOSSI, and PERKOVSKY 2021")
        .species("Embolemus", "brachypterus")
        .comb_authors(
            Some("2021"),
            &["Olmi", "Guglielmino", "Müller", "Capradossi", "Perkovsky"],
        )
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Rhynchozoon brasiliensis ALMEIDA, SOUZA, MENEGOLA & VIEIRA 2017")
        .species("Rhynchozoon", "brasiliensis")
        .comb_authors(Some("2017"), &["Almeida", "Souza", "Menegola", "Vieira"])
        .code(NomCode::Zoological)
        .nothing_else();
    // initials before all-capital surnames, and an "et" before the last author
    assert_name(
        "Elasmostemon paisii E. M. FRIIS, P. R. CRANE, K. R. PEDERSEN, M. M. MENDES et J. KVACEK \
         2022",
    )
    .species("Elasmostemon", "paisii")
    .comb_authors(
        Some("2022"),
        &[
            "E.M.Friis",
            "P.R.Crane",
            "K.R.Pedersen",
            "M.M.Mendes",
            "J.Kvacek",
        ],
    )
    .code(NomCode::Zoological)
    .nothing_else();
    assert_name(
        "Saccharomycopsis praedatoria A.R.O. SANTOS, BARROS, BATISTA, G.F.L. SOUZA, ALVARENGA, \
         ABEGG, T.K. SATO, HITTINGER, LACHANCE & ROSA",
    )
    .species("Saccharomycopsis", "praedatoria")
    .comb_authors(
        None,
        &[
            "A.R.O.Santos",
            "Barros",
            "Batista",
            "G.F.L.Souza",
            "Alvarenga",
            "Abegg",
            "T.K.Sato",
            "Hittinger",
            "Lachance",
            "Rosa",
        ],
    )
    .nothing_else();
}
