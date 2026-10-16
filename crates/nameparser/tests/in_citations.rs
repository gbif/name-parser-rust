// SPDX-License-Identifier: Apache-2.0
//! `in` / `apud` citations in a separately supplied authorship (#20).
//!
//! `Grunow in Van Heurck, 1883` names the author (`Grunow`) and the work that published the name
//! (`Van Heurck, 1883`). Embedded in the name string, StripAndStash always split the two: the host
//! goes to `publishedIn`, its year becomes the combination year. A separately supplied authorship —
//! `parse(name, authorship, …)`, and so also `parseAuthorship` — skipped that step, so the whole
//! citation reached the authorship parser: the host stayed glued to the author (`Grunow in Van
//! Heurck`), the host's own `&` / `et al.` split it into further authors (`Valenciennes`, `al.`),
//! and after an abbreviation the space before `in` was lost (`Trel.in J.F.Macbr.`).
//!
//! ChecklistBank parses almost every name that way, because sources deliver scientific name and
//! authorship in separate columns. Inherited from `org.gbif:name-parser` 4.2.0, so this is a
//! deliberate change against Java parity, not a parity fix.

mod common;
use common::*;
use nameparser::model::{NomCode, Rank};

#[test]
fn separate_authorship_splits_an_in_citation_like_the_embedded_one() {
    for (name, authorship) in [
        ("Actinocyclus australis", "Grunow in Van Heurck, 1883"),
        (
            "Cantharus lineolatus",
            "Valenciennes in Cuvier & Valenciennes, 1850",
        ),
        ("Navicula perpusilla", "Hustedt in Schmidt et al., 1925"),
        ("Platax teira", "(Forster in Bloch & Schneider, 1801)"),
        ("Surirella gemma", "(Ehrenberg) Ralfs in Pritchard, 1861"),
        ("Piper hispidum", "Trel. in J.F.Macbr."),
        ("Marchantia polymorpha", "Nees & Mart. in Nova Acta"),
        ("Croton lobatus", "Pohl ex Benth. in DC."),
        ("Croton lobatus", "Müll.Arg. in DC."),
        ("Xolisma turquini", "Small apud Britton & Wilson"),
        ("Hypsicera femoralis", "(Geoffroy in Fourcroy, 1785)"),
    ] {
        assert_paths_agree(name, authorship);
    }
}

#[test]
fn separate_authorship_in_citation_goes_to_published_in() {
    assert_name_auth("Actinocyclus australis", "Grunow in Van Heurck, 1883")
        .species("Actinocyclus", "australis")
        .comb_authors(Some("1883"), &["Grunow"])
        .published_in("Van Heurck, 1883")
        .published_in_year(Some(1883))
        .comb_in_authors(&["Van Heurck"])
        .nothing_else();
}

#[test]
fn the_host_team_does_not_become_authors_of_the_name() {
    assert_authorship(
        "Valenciennes in Cuvier & Valenciennes, 1850",
        &["Valenciennes"],
    )
    .published_in("Cuvier & Valenciennes, 1850")
    .published_in_year(Some(1850))
    .comb_authors(Some("1850"), &["Valenciennes"])
    .comb_in_authors(&["Cuvier", "Valenciennes"])
    .nothing_else();
    assert_authorship("Hustedt in Schmidt et al., 1925", &["Hustedt"])
        .published_in("Schmidt et al., 1925")
        .comb_authors(Some("1925"), &["Hustedt"])
        .published_in_year(Some(1925))
        .comb_in_authors(&["Schmidt", "al."])
        .nothing_else();
    assert_authorship("Nees & Mart. in Nova Acta", &["Nees", "Mart."])
        .published_in("Nova Acta")
        .comb_authors(None, &["Nees", "Mart."])
        .nothing_else();
}

#[test]
fn an_abbreviated_author_keeps_its_own_name() {
    // Was `Trel.in J.F.Macbr.` — one author, with the space before `in` gone.
    assert_authorship("Trel. in J.F.Macbr.", &["Trel."])
        .published_in("J.F.Macbr.")
        .comb_authors(None, &["Trel."])
        .comb_in_authors(&["J.F.Macbr."])
        .nothing_else();
    // Was `D.C.Benth.in` ex Pohl: the host `DC.` was read as the initials of `Benth.in`.
    assert_ex_authorship("Pohl ex Benth. in DC.", Some("Pohl"), &["Benth."])
        .published_in("DC.")
        .comb_authors(None, &["Benth."])
        .comb_ex_authors(&["Pohl"])
        .comb_in_authors(&["DC."])
        .nothing_else();
    assert_authorship("Müll.Arg. in DC.", &["Müll.Arg."])
        .published_in("DC.")
        .comb_authors(None, &["Müll.Arg."])
        .comb_in_authors(&["DC."])
        .nothing_else();
}

#[test]
fn an_in_citation_inside_the_basionym_parentheses() {
    let pn = nameparser::parse_name(
        "Platax teira",
        Some("(Forster in Bloch & Schneider, 1801)"),
        Some(Rank::Species),
        None,
    )
    .unwrap();
    assert_eq!(pn.basionym_authorship.authors, vec!["Forster"]);
    assert_eq!(pn.basionym_authorship.year.as_deref(), Some("1801"));
    assert!(!pn.combination_authorship.exists());
    assert_eq!(pn.published_in.as_deref(), Some("Bloch & Schneider, 1801"));

    let pn = nameparser::parse_name(
        "Surirella gemma",
        Some("(Ehrenberg) Ralfs in Pritchard, 1861"),
        None,
        None,
    )
    .unwrap();
    assert_eq!(pn.basionym_authorship.authors, vec!["Ehrenberg"]);
    assert_eq!(pn.combination_authorship.authors, vec!["Ralfs"]);
    assert_eq!(pn.combination_authorship.year.as_deref(), Some("1861"));
    assert_eq!(pn.published_in.as_deref(), Some("Pritchard, 1861"));
}

#[test]
fn a_redundant_authorship_does_not_repeat_the_reference() {
    // Sources often repeat the authorship in both columns (see `impl_08::redundant_authorship`).
    // Both copies are stripped now; the reference the name string gave is the one recorded.
    assert_name_auth(
        "Actinocyclus australis Grunow in Van Heurck, 1883",
        "Grunow in Van Heurck, 1883",
    )
    .species("Actinocyclus", "australis")
    .comb_authors(Some("1883"), &["Grunow"])
    .published_in("Van Heurck, 1883")
    .published_in_year(Some(1883))
    .comb_in_authors(&["Van Heurck"])
    .nothing_else();

    // The copies need not be identical: 5,514 distinct ChecklistBank names repeat the citation
    // in a shorter or differently punctuated form.
    assert_name_auth(
        "Conus aureus Hwass in Bruguiere, 1792",
        "Hwass in Bruguiere",
    )
    .species("Conus", "aureus")
    .comb_authors(Some("1792"), &["Hwass"])
    .published_in("Bruguiere, 1792")
    .published_in_year(Some(1792))
    .comb_in_authors(&["Bruguiere"])
    .nothing_else();
    // the combination's in-citation is the name's publication, not the basionym's
    assert_name_auth(
        "Brachysira zellensis f. difficilis (Grunow in Van Heurck) P. B. Ham. in Hamilton, Poulin, Charles & Angell",
        "(Grunow in Van Heurck) P. B. Ham. in Hamilton, Poulin, Charles & Angell",
    )
    .infra_species("Brachysira", "zellensis", Rank::Form, "difficilis")
    .bas_authors(None, &["Grunow"])
    .comb_authors(None, &["P.B.Ham."])
    .published_in("Hamilton, Poulin, Charles & Angell")
    .code(NomCode::Botanical)
    .comb_in_authors(&["Hamilton", "Poulin", "Charles", "Angell"])
    .bas_in_authors(&["Van Heurck"])
    .nothing_else();
}

#[test]
fn a_capitalised_or_bracketed_in_is_a_citation_too() {
    // 1,251 ChecklistBank rows write the in-citation with a capital: all of them citations
    assert_name("Monitor pulcher Leach In Bowdich, 1819")
        .species("Monitor", "pulcher")
        .comb_authors(Some("1819"), &["Leach"])
        .published_in("Bowdich, 1819")
        .published_in_year(Some(1819))
        .comb_in_authors(&["Bowdich"])
        .nothing_else();
    assert_name("Zornia diphylla var. leptophylla Benth. In Mart. Fl. Bras.")
        .infra_species("Zornia", "diphylla", Rank::Variety, "leptophylla")
        .comb_authors(None, &["Benth."])
        .published_in("Mart. Fl. Bras.")
        .nothing_else();
    // a citation cut off after its "In" leaves no author behind
    assert_name("Asterocheres unicus Johnsson, In")
        .species("Asterocheres", "unicus")
        .comb_authors(None, &["Johnsson"])
        .nothing_else();
    // "[Dryander in] Aiton": the author in brackets, published in Aiton's work
    assert_name("Glycine debilis [Dryander in] Aiton")
        .species("Glycine", "debilis")
        .comb_authors(None, &["Dryander"])
        .published_in("Aiton")
        .comb_in_authors(&["Aiton"])
        .nothing_else();
}

#[test]
fn an_in_citation_in_its_own_bracket() {
    // #96: WoRMS writes it "(in: …)"; the authorship was cut to "Reid (in" and the year lost
    assert_name_auth(
        "Rheocyclops carolinianus",
        "Reid (in: Reid, Strayer, McArthur, Stibbe & Lewis), 1999",
    )
    .species("Rheocyclops", "carolinianus")
    .comb_authors(Some("1999"), &["Reid"])
    .published_in("Reid, Strayer, McArthur, Stibbe & Lewis, 1999")
    .published_in_year(Some(1999))
    .comb_in_authors(&["Reid", "Strayer", "McArthur", "Stibbe", "Lewis"])
    .nothing_else();
    assert_name_auth("Nirmus anagrapsus", "Nitzsch (in Giebel), 1866")
        .species("Nirmus", "anagrapsus")
        .comb_authors(Some("1866"), &["Nitzsch"])
        .published_in("Giebel, 1866")
        .published_in_year(Some(1866))
        .comb_in_authors(&["Giebel"])
        .nothing_else();
    // NCBI's higher taxon of a homonym is no citation
    assert_informal("Bostrychia sp. (in: Rhodophyta)")
        .taxon("Bostrychia")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. (in: Rhodophyta)")
        .nothing_else();
}

// ---- in-authors (#172) ------------------------------------------------------------------------
//
// The authors of the work an `in` citation names are the in-authors of the authorship that cites
// it, the combination's or the basionym's (name-parser-api 5.6 `Authorship.inAuthors`): a name is
// cited by its author or by the work's, "Natterer in Kuhl, 1817" or "Kuhl, 1817". Only an author
// team gets them, a reference does not; `publishedIn` keeps the whole citation either way.

#[test]
fn in_authors_of_the_combination_and_the_basionym() {
    assert_name("Pipistrellus kuhlii (Natterer in Kuhl, 1817)")
        .species("Pipistrellus", "kuhlii")
        .bas_authors(Some("1817"), &["Natterer"])
        .bas_in_authors(&["Kuhl"])
        .published_in("Kuhl, 1817")
        .published_in_year(Some(1817))
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Atriplex prostrata Boucher ex DC. in Lam. & DC.")
        .species("Atriplex", "prostrata")
        .comb_authors(None, &["DC."])
        .comb_ex_authors(&["Boucher"])
        .comb_in_authors(&["Lam.", "DC."])
        .published_in("Lam. & DC.")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Navicula cryptocephala Grunow in Van Heurck, 1883")
        .species("Navicula", "cryptocephala")
        .comb_authors(Some("1883"), &["Grunow"])
        .comb_in_authors(&["Van Heurck"])
        .published_in("Van Heurck, 1883")
        .published_in_year(Some(1883))
        .nothing_else();
    assert_name("Achnanthes brevipes var. intermedia (Kützing) Cleve in Cleve & Grunow, 1880")
        .infra_species("Achnanthes", "brevipes", Rank::Variety, "intermedia")
        .bas_authors(None, &["Kützing"])
        .comb_authors(Some("1880"), &["Cleve"])
        .comb_in_authors(&["Cleve", "Grunow"])
        .published_in("Cleve & Grunow, 1880")
        .published_in_year(Some(1880))
        .code(NomCode::Botanical)
        .nothing_else();
    // the basionym's, while the combination has none
    assert_name("Nitzschia sigma (Grunow in Van Heurck) W.Sm.")
        .species("Nitzschia", "sigma")
        .bas_authors(None, &["Grunow"])
        .bas_in_authors(&["Van Heurck"])
        .comb_authors(None, &["W.Sm."])
        .published_in("Van Heurck")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Amphora ovalis (Kütz.) Kütz. in Rabenh.")
        .species("Amphora", "ovalis")
        .bas_authors(None, &["Kütz."])
        .comb_authors(None, &["Kütz."])
        .comb_in_authors(&["Rabenh."])
        .published_in("Rabenh.")
        .code(NomCode::Botanical)
        .nothing_else();
    // both: the basionym's host is no longer lost to the combination's citation
    assert_name("Aus bus (Smith in Jones, 1900) Brown in Miller, 1920")
        .species("Aus", "bus")
        .bas_authors(Some("1900"), &["Smith"])
        .bas_in_authors(&["Jones"])
        .comb_authors(Some("1920"), &["Brown"])
        .comb_in_authors(&["Miller"])
        .published_in("Miller, 1920")
        .published_in_year(Some(1920))
        .nothing_else();
}

#[test]
fn in_authors_of_large_teams() {
    assert_name("Navicula hustedtii Hustedt in Schmidt et al., 1925")
        .species("Navicula", "hustedtii")
        .comb_authors(Some("1925"), &["Hustedt"])
        .comb_in_authors(&["Schmidt", "al."])
        .published_in("Schmidt et al., 1925")
        .published_in_year(Some(1925))
        .nothing_else();
    assert_name_auth(
        "Aus bus",
        "Cavalier-Smith & Bass in Bass, Chao, Nikolaev, Yabuki, Ishida, Berney, Pakzad, Wylezich & \
         Cavalier-Smith, 2009",
    )
    .species("Aus", "bus")
    .comb_authors(Some("2009"), &["Cavalier-Smith", "Bass"])
    .comb_in_authors(&[
        "Bass",
        "Chao",
        "Nikolaev",
        "Yabuki",
        "Ishida",
        "Berney",
        "Pakzad",
        "Wylezich",
        "Cavalier-Smith",
    ])
    .published_in(
        "Bass, Chao, Nikolaev, Yabuki, Ishida, Berney, Pakzad, Wylezich & Cavalier-Smith, 2009",
    )
    .published_in_year(Some(2009))
    .nothing_else();
    // the editors of a work are the authors it is cited by
    assert_name_auth(
        "Allomalia gemmaria",
        "Alonso-Zarazaga in Löbl & Smetana (eds.), 2011",
    )
    .species("Allomalia", "gemmaria")
    .comb_authors(Some("2011"), &["Alonso-Zarazaga"])
    .comb_in_authors(&["Löbl", "Smetana"])
    .published_in("Löbl & Smetana (eds.), 2011")
    .published_in_year(Some(2011))
    .nothing_else();
    // Linnaeus, the one author cited by a single letter
    assert_name_auth("Aecidium berberidis", "J.F. Gmelin in L.")
        .species("Aecidium", "berberidis")
        .comb_authors(None, &["J.F.Gmelin"])
        .comb_in_authors(&["L."])
        .published_in("L.")
        .nothing_else();
}

#[test]
fn a_reference_has_no_in_authors() {
    assert_name("Abies alba Mill. in Gard. Dict. ed. 8: no. 1. 1768")
        .species("Abies", "alba")
        .comb_authors(Some("1768"), &["Mill."])
        .published_in("Gard. Dict. ed. 8: no. 1. 1768")
        .published_in_year(Some(1768))
        .nothing_else();
    assert_name("Puffinus lherminieri Lesson, 1839 in Ann. Sci. Nat.")
        .species("Puffinus", "lherminieri")
        .comb_authors(Some("1839"), &["Lesson"])
        .published_in("Ann. Sci. Nat.")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Rhizoclonium hieroglyphicum (C.Agardh) Kütz. in Phycol. Germ.")
        .species("Rhizoclonium", "hieroglyphicum")
        .bas_authors(None, &["C.Agardh"])
        .comb_authors(None, &["Kütz."])
        .published_in("Phycol. Germ.")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Adelium cupreum", "Gray in Griffith's Cuvier 1832")
        .species("Adelium", "cupreum")
        .comb_authors(Some("1832"), &["Gray"])
        .published_in("Griffith's Cuvier 1832")
        .published_in_year(Some(1832))
        .nothing_else();
    // a contribution to a contribution: whose work is meant is left open
    assert_name_auth("Achnanthes obtusa", "Ehrenberg in Ralfs in Pritchard, 1861")
        .species("Achnanthes", "obtusa")
        .comb_authors(Some("1861"), &["Ehrenberg"])
        .published_in("Ralfs in Pritchard, 1861")
        .published_in_year(Some(1861))
        .nothing_else();
    // a bare initial is no author
    assert_name_auth("Alauda bilopha", "Temminck in T. & Laugier 1823")
        .species("Alauda", "bilopha")
        .comb_authors(Some("1823"), &["Temminck"])
        .published_in("T. & Laugier 1823")
        .published_in_year(Some(1823))
        .nothing_else();
}

#[test]
fn in_authors_alike_on_both_paths() {
    for (name, authorship) in [
        ("Pipistrellus kuhlii", "(Natterer in Kuhl, 1817)"),
        ("Atriplex prostrata", "Boucher ex DC. in Lam. & DC."),
        ("Navicula cryptocephala", "Grunow in Van Heurck, 1883"),
        ("Nitzschia sigma", "(Grunow in Van Heurck) W.Sm."),
        ("Amphora ovalis", "(Kütz.) Kütz. in Rabenh."),
        ("Aus bus", "(Smith in Jones, 1900) Brown in Miller, 1920"),
        ("Abies alba", "Mill. in Gard. Dict. ed. 8: no. 1. 1768"),
        ("Puffinus lherminieri", "Lesson, 1839 in Ann. Sci. Nat."),
        ("Navicula hustedtii", "Hustedt in Schmidt et al., 1925"),
        (
            "Rheocyclops carolinianus",
            "Reid (in: Reid, Strayer, McArthur, Stibbe & Lewis), 1999",
        ),
    ] {
        assert_paths_agree(name, authorship);
    }
}
