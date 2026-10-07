// SPDX-License-Identifier: Apache-2.0
//! Anonymous authors and square brackets around authors — `Authorship::anonymous`.
//!
//! A work published anonymously is cited as "Anon." in zoology (ICZN Recommendation 51D) and
//! "anon." in botany; an author known only from external evidence goes in square brackets "to
//! show the original anonymity" (Rec. 51D): `[Denis & Schiffermüller], 1775`. In botany a
//! bracketed author in front of the validating one is a pre-starting-point author instead —
//! `Lupinus [Tourn.] L.`, cited `Lupinus Tourn. ex L.` by the current code. The brackets are read
//! by shape, for every code:
//!
//! - (a) an anonymous word alone in an author slot is the flag, not an author;
//! - (b) `Anonymous [Bennett]` and (c) a wholly bracketed author slot set the flag and keep the
//!   attributed authors;
//! - (d) bracketed authors before a real author become ex authors;
//! - (e) any other bracket keeps the old behaviour (the brackets are dropped);
//! - (f) an anonymous ex author and a team mixing "Anonymous" with a real author stay strings.
//!
//! `org.gbif:name-parser` 4.2.0 kept "anon." as an author string and dropped every bracket, so
//! all of this is a deliberate change against Java parity.

mod common;
use common::*;
use nameparser::model::{NomCode, Rank};

fn authorship(name: &str, authorship: Option<&str>, code: Option<NomCode>) -> String {
    nameparser::parse_name(name, authorship, None, code)
        .unwrap_or_else(|e| panic!("`{name}` should parse: {e:?}"))
        .authorship_complete()
        .unwrap_or_default()
}

#[test]
fn a_lone_anonymous_word_is_the_flag() {
    for authorship in [
        "Anon.",
        "anon.",
        "Anon",
        "Anonymous",
        "Anonymus",
        "Anonyme",
        "ANONYMOUS",
    ] {
        assert_name_auth("Aus bus", authorship)
            .species("Aus", "bus")
            .comb_anon(None, &[])
            .nothing_else();
    }
    assert_name("Bancroftia Anonymous, 1838")
        .monomial("Bancroftia")
        .comb_anon(Some("1838"), &[])
        .code(NomCode::Zoological)
        .nothing_else();
    // in the basionym too
    assert_name("Abalistes stellatus (Anonymous, 1798)")
        .species("Abalistes", "stellatus")
        .bas_anon(Some("1798"), &[])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Agapanthus umbellatus var. giganteus (Anon.) L.H.Bailey")
        .infra_species("Agapanthus", "umbellatus", Rank::Variety, "giganteus")
        .bas_anon(None, &[])
        .comb_authors(None, &["L.H.Bailey"])
        .code(NomCode::Botanical)
        .nothing_else();
    // the publishing author behind an ex author
    assert_name_auth("Aus bus", "Sw. ex Anon.")
        .species("Aus", "bus")
        .comb_anon(None, &[])
        .comb_ex_authors(&["Sw."])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn bracketed_authors_of_an_anonymous_work() {
    assert_name("Acleris forskoliana [Denis & Schiffermüller], 1775")
        .species("Acleris", "forskoliana")
        .comb_anon(Some("1775"), &["Denis", "Schiffermüller"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Acanthocardia spinosa ([Lightfoot], 1786)")
        .species("Acanthocardia", "spinosa")
        .bas_anon(Some("1786"), &["Lightfoot"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_auth("Clupea ovalis", "Anonymous [Bennett], 1830")
        .species("Clupea", "ovalis")
        .comb_anon(Some("1830"), &["Bennett"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Rhinobatos typus (Anonymous [Bennett], 1830)")
        .species("Rhinobatos", "typus")
        .bas_anon(Some("1830"), &["Bennett"])
        .code(NomCode::Zoological)
        .nothing_else();
    // botanical anonymous works: Aiton's Hortus Kewensis, Clairville's Manuel d'herborisation
    assert_name_auth("Tragopogon nervosus", "[Soland.]")
        .species("Tragopogon", "nervosus")
        .comb_anon(None, &["Soland."])
        .nothing_else();
    assert_name_auth("Aus bus", "(Gouan) [Clairv.]")
        .species("Aus", "bus")
        .bas_authors(None, &["Gouan"])
        .comb_anon(None, &["Clairv."])
        .code(NomCode::Botanical)
        .nothing_else();
    // a bracketed year ends the author slot; alone it is the year
    assert_name_auth("Aus bus", "[Hübner], [1806]")
        .species("Aus", "bus")
        .comb_anon(Some("1806"), &["Hübner"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn a_bracketed_pre_starting_point_author_is_an_ex_author() {
    // Paris Code 1956 Rec. 50D: "[Tourn.] L." or "Tourn. ex L."; current codes keep only "ex"
    assert_name("Lupinus [Tourn.] L.")
        .monomial("Lupinus")
        .comb_authors(None, &["L."])
        .comb_ex_authors(&["Tourn."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Cuphea lanceolata", "[Dryand.] Ait.")
        .species("Cuphea", "lanceolata")
        .comb_authors(None, &["Ait."])
        .comb_ex_authors(&["Dryand."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Aus bus", "[Kar. & Kir.] Regel")
        .species("Aus", "bus")
        .comb_authors(None, &["Regel"])
        .comb_ex_authors(&["Kar.", "Kir."])
        .code(NomCode::Botanical)
        .nothing_else();
    // the bracket may already carry its "ex"
    assert_name_auth(
        "Pseudocercospora dendrobii",
        "[Sawada ex] Goh & W. H. Hsieh",
    )
    .species("Pseudocercospora", "dendrobii")
    .comb_authors(None, &["Goh", "W.H.Hsieh"])
    .comb_ex_authors(&["Sawada"])
    .code(NomCode::Botanical)
    .nothing_else();
}

#[test]
fn other_brackets_are_left_alone() {
    // supplied initials, a year inside the brackets, an in-citation, a manuscript marker
    assert_name_auth("Aus bus", "Gerstaecker, [C.E.] A., 1871")
        .species("Aus", "bus")
        .comb_authors(Some("1871"), &["C.E.A.Gerstaecker"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_auth("Aus bus", "[Hübner, 1806]")
        .species("Aus", "bus")
        .comb_authors(Some("1806"), &["Hübner"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_auth("Notochlamys hexactes", "([Péron in] Lamarck, 1819)")
        .species("Notochlamys", "hexactes")
        .bas_authors(Some("1819"), &["Péron in Lamarck"])
        .code(NomCode::Zoological)
        .nothing_else();
    // bare initials in brackets are no pre-starting-point author
    assert_name_auth("Aus bus", "[C.] Smith")
        .species("Aus", "bus")
        .comb_authors(None, &["C.Smith"])
        .nothing_else();
}

#[test]
fn an_anonymous_ex_author_or_team_member_stays_a_string() {
    assert_name_auth("Depierrea", "Anon. ex Schltdl.")
        .monomial("Depierrea")
        .comb_authors(None, &["Schltdl."])
        .comb_ex_authors(&["anon."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Serranus confertus", "Anonymous & Bennett, 1830")
        .species("Serranus", "confertus")
        .comb_authors(Some("1830"), &["Anonymous", "Bennett"])
        .code(NomCode::Zoological)
        .nothing_else();
    // a lower-case "anonym…" is an epithet, never an author
    assert_name("Aegeria anonyma")
        .species("Aegeria", "anonyma")
        .nothing_else();
}

#[test]
fn rendered_per_code() {
    assert_eq!(
        authorship("Aus bus", Some("Anon., 1830"), None),
        "Anon., 1830"
    );
    assert_eq!(
        authorship("Aus bus", Some("Anon., 1830"), Some(NomCode::Zoological)),
        "Anon., 1830"
    );
    assert_eq!(
        authorship("Aus bus", Some("anon."), Some(NomCode::Botanical)),
        "anon."
    );
    assert_eq!(
        authorship("Physalospora rubiginosa (Fr.) anon.", None, None),
        "(Fr.) anon."
    );
    assert_eq!(
        authorship("Aus bus", Some("Sw. ex Anon."), Some(NomCode::Botanical)),
        "Sw. ex anon."
    );
    assert_eq!(
        authorship(
            "Acleris forskoliana [Denis & Schiffermüller], 1775",
            None,
            None
        ),
        "[Denis & Schiffermüller], 1775"
    );
    assert_eq!(
        authorship("Rhinobatos typus (Anonymous [Bennett], 1830)", None, None),
        "([Bennett], 1830)"
    );
    assert_eq!(
        authorship("Lupinus [Tourn.] L.", None, None),
        "Tourn. ex L."
    );
}
