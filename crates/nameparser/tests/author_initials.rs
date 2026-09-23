// SPDX-License-Identifier: Apache-2.0
//! Initials the authorship parser invented (#21, #22).
//!
//! Zoological author strings put initials after the surname, `LeConte, J.L.` or `Walker F`, and
//! the parser turns them round into `J.L.LeConte`. Two author strings looked enough like that
//! to be turned round too, each making one author out of something that is not an initial:
//!
//!   * **#21** — an all-capitals author after `&` / `et` became the initials of the author before
//!     it: `Lam. & DC.` → `D.C.Lam.`, one invented person instead of Lamarck and de Candolle.
//!   * **#22** — the generation `I` of `G. B. Sowerby I` became a leading initial,
//!     `I.G.B.Sowerby`, although `II` and `III` were always kept.
//!
//! Both are inherited from `org.gbif:name-parser` 4.2.0, so these are deliberate changes against
//! Java parity.

mod common;
use common::*;
use nameparser::model::NomCode;

// ---- #21: `&` / `et` always separate two authors -------------------------------------------------

#[test]
fn an_all_capitals_author_after_ampersand_stays_a_separate_author() {
    assert_authorship("Lam. & DC.", &["Lam.", "DC."]);
    assert_authorship("Poir. & DC.", &["Poir.", "DC."]);
    assert_authorship("Kindb. & DC", &["Kindb.", "DC"]);
    assert_authorship("Lam. & A.DC.", &["Lam.", "A.DC."]);
    assert_authorship("Mill. & HBK.", &["Mill.", "HBK."]);
}

#[test]
fn et_and_and_separate_authors_just_like_ampersand() {
    assert_authorship("Lam. et DC.", &["Lam.", "DC."]);
    assert_authorship("Lam. and DC.", &["Lam.", "DC."]);
}

#[test]
fn the_basionym_team_is_kept_apart_too() {
    assert_name_auth("Abies alba", "(Lam. & DC.) Kunth")
        .species("Abies", "alba")
        .bas_authors(None, &["Lam.", "DC."])
        .comb_authors(None, &["Kunth"])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn an_embedded_authorship_is_kept_apart_too() {
    assert_name("Abies alba Lam. & DC.")
        .species("Abies", "alba")
        .comb_authors(None, &["Lam.", "DC."])
        .nothing_else();
}

#[test]
fn an_abbreviated_author_before_ampersand_is_never_turned_round() {
    // Laporte & Gory, Mulsant & Rey, Milne-Edwards & Haime: two authors, not `G.Lap.`.
    assert_authorship("Lap. & G.", &["Lap.", "G."]);
    assert_authorship("Muls. & R.", &["Muls.", "R."]);
    assert_authorship("Edw. & H.", &["Edw.", "H."]);
}

#[test]
fn a_capital_run_after_ampersand_is_an_author_not_initials() {
    assert_authorship("Dunal & A.DC.", &["Dunal", "A.DC."]);
    assert_authorship("MULSANT & REY 1852", &["Mulsant", "REY"]);
    assert_authorship("ZHOU & LIU 1981", &["Zhou", "LIU"]);
}

#[test]
fn a_comma_lost_to_ampersand_still_joins_surname_and_initials() {
    // `Lea, A.M.` delivered as `Lea & A.M.` — one author, as Java read it.
    assert_authorship("Lea & A.M., 1906", &["A.M.Lea"]);
    assert_authorship("Denis & J-R, 1924", &["J-R.Denis"]);
    assert_authorship("Say & T., 1826", &["T.Say"]);
}

#[test]
fn surname_then_initials_still_turns_round() {
    // The inversion itself stays: a comma joins a surname to its initials.
    assert_authorship("LeConte, J.L.", &["J.L.LeConte"]);
    assert_authorship("Walker, F.", &["F.Walker"]);
    assert_authorship("Yin, T.-H.", &["T.-H.Yin"]);
    assert_authorship("Smith, J. & Jones, K.", &["J.Smith", "K.Jones"]);
    assert_authorship("Zhang F & Pan Z-X", &["F.Zhang", "Z-X.Pan"]);
    assert_authorship("DC. & Lam.", &["DC.", "Lam."]);
    assert_authorship("Humb. & Bonpl.", &["Humb.", "Bonpl."]);
}

// ---- #22: the generation `I` ---------------------------------------------------------------

#[test]
fn generation_i_after_leading_initials_stays_behind_the_surname() {
    // An author with leading initials does not also carry initials behind the surname.
    assert_authorship("G. B. Sowerby I, 1825", &["G.B.Sowerby I"]);
    assert_authorship("G.B. Sowerby I", &["G.B.Sowerby I"]);
    assert_authorship("G.B.Sowerby I", &["G.B.Sowerby I"]);
    // II and III always were kept.
    assert_authorship("G. B. Sowerby II, 1842", &["G.B.Sowerby II"]);
    assert_authorship("G. B. Sowerby III, 1912", &["G.B.Sowerby III"]);
}

#[test]
fn generation_i_next_to_a_later_generation_stays_behind_the_surname() {
    assert_authorship("Sowerby I & Sowerby II", &["Sowerby I", "Sowerby II"]);
}

#[test]
fn generation_i_in_a_name_and_in_a_basionym() {
    assert_name("Conus textile G. B. Sowerby I, 1825")
        .species("Conus", "textile")
        .comb_authors(Some("1825"), &["G.B.Sowerby I"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_auth("Haliotis asinina", "(Broderip & G. B. Sowerby I, 1829)")
        .species("Haliotis", "asinina")
        .bas_authors(Some("1829"), &["Broderip", "G.B.Sowerby I"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn a_trailing_i_without_that_evidence_stays_an_initial() {
    // Ambiguous on its own: `Sowerby I` may be the generation or the initial.
    assert_authorship("Sowerby I", &["I.Sowerby"]);
    // A dotted `I.` is an initial (`Kim I.` = I. Kim), even after leading initials.
    assert_authorship("Kim I.", &["I.Kim"]);
    assert_authorship("G.B. Sowerby I.", &["I.G.B.Sowerby"]);
    // V and X are left alone.
    assert_authorship("Smith V", &["V.Smith"]);
}
