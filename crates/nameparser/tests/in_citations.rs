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
use nameparser::model::{NomCode, ParsedName, Rank};

/// The name with its authorship embedded, and the same name with the authorship passed
/// separately, must agree on every authorship field.
fn assert_paths_agree(name: &str, authorship: &str) {
    let full = format!("{name} {authorship}");
    let embedded = nameparser::parse_name(&full, None, None, None)
        .unwrap_or_else(|e| panic!("`{full}` should parse: {e:?}"));
    let separate = nameparser::parse_name(name, Some(authorship), None, None)
        .unwrap_or_else(|e| panic!("`{name}` + `{authorship}` should parse: {e:?}"));
    let fields = |p: &ParsedName| {
        (
            p.combination_authorship.clone(),
            p.basionym_authorship.clone(),
            p.published_in.clone(),
            p.published_in_year,
            p.code,
            p.state,
            p.unparsed.clone(),
        )
    };
    assert_eq!(
        fields(&separate),
        fields(&embedded),
        "`{name}` + `{authorship}` (separate) vs `{full}` (embedded)"
    );
}

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
        .nothing_else();
}

#[test]
fn the_host_team_does_not_become_authors_of_the_name() {
    assert_authorship(
        "Valenciennes in Cuvier & Valenciennes, 1850",
        &["Valenciennes"],
    )
    .published_in("Cuvier & Valenciennes, 1850")
    .published_in_year(Some(1850));
    assert_authorship("Hustedt in Schmidt et al., 1925", &["Hustedt"])
        .published_in("Schmidt et al., 1925");
    assert_authorship("Nees & Mart. in Nova Acta", &["Nees", "Mart."]).published_in("Nova Acta");
}

#[test]
fn an_abbreviated_author_keeps_its_own_name() {
    // Was `Trel.in J.F.Macbr.` — one author, with the space before `in` gone.
    assert_authorship("Trel. in J.F.Macbr.", &["Trel."]).published_in("J.F.Macbr.");
    // Was `D.C.Benth.in` ex Pohl: the host `DC.` was read as the initials of `Benth.in`.
    assert_ex_authorship("Pohl ex Benth. in DC.", Some("Pohl"), &["Benth."]).published_in("DC.");
    assert_authorship("Müll.Arg. in DC.", &["Müll.Arg."]).published_in("DC.");
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
    .nothing_else();
    assert_name_auth(
        "Brachysira zellensis f. difficilis (Grunow in Van Heurck) P. B. Ham. in Hamilton, Poulin, Charles & Angell",
        "(Grunow in Van Heurck) P. B. Ham. in Hamilton, Poulin, Charles & Angell",
    )
    .infra_species("Brachysira", "zellensis", Rank::Form, "difficilis")
    .bas_authors(None, &["Grunow"])
    .comb_authors(None, &["P.B.Ham."])
    .published_in("Van Heurck Hamilton, Poulin, Charles & Angell")
    .code(NomCode::Botanical)
    .nothing_else();
}
