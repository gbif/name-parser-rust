// SPDX-License-Identifier: Apache-2.0
//! The anonymous author and manuscript markers in a separately supplied authorship.
//!
//! `org.gbif:name-parser` 4.2.0 normalised "Anon"/"Anon." to the author "anon." and stripped a
//! trailing manuscript marker ("ined.", "ms.") into the `manuscript` flag only on the name string.
//! A separately supplied authorship — how ChecklistBank parses almost every name — skipped both,
//! so the same author came out as "Anon." or "anon." depending on the column, and "(Fr.) anon.
//! ined." gave the author "anon.ined." without the flag. Both paths now agree, and an anonymous
//! author alone — any spelling, "Anonymous" included — is `Authorship::anonymous`, not an author
//! string (see `anonymous_authorship.rs`). Deliberate changes against Java parity.

mod common;
use common::*;
use nameparser::model::{warnings, NomCode, Rank};

#[test]
fn anon_is_normalised_in_both_paths() {
    for (name, authorship) in [
        ("Eurilenus", "Anon., 1829"),
        ("Aus bus", "Anon"),
        ("Physalospora sepincoliformis", "(De Not.) Anon."),
        ("Depierrea", "Anon. ex Schltdl."),
        ("Cercis arizonica", "Anon ex Patraw, 1932"),
        ("Pohlia pallens", "(Sw. ex Anon.) J.J.Amann"),
    ] {
        assert_paths_agree(name, authorship);
    }
    assert_name_auth("Physalospora sepincoliformis", "(De Not.) Anon.")
        .species("Physalospora", "sepincoliformis")
        .comb_anon(None, &[])
        .bas_authors(None, &["De Not."])
        .code(NomCode::Botanical)
        .nothing_else();
    // the spelled-out forms are the anonymous flag too
    assert_name_auth("Aus bus", "Anonymous")
        .species("Aus", "bus")
        .comb_anon(None, &[])
        .nothing_else();
}

#[test]
fn a_trailing_manuscript_marker_sets_the_flag_in_both_paths() {
    for (name, authorship) in [
        ("Hygrocybe pratensis var. roseipes", "anon. ined."),
        ("Acetabula acetabulum", "(L.) anon. ined."),
        ("Abies alba", "Mill. ined."),
        ("Acosmeta", "Moeschler, MS."),
        ("Mycetia macrocarpa", "F.C.How ex ined."),
    ] {
        assert_paths_agree(name, authorship);
    }
    assert_name_auth("Acetabula acetabulum", "(L.) anon. ined.")
        .species("Acetabula", "acetabulum")
        .comb_anon(None, &[])
        .bas_authors(None, &["L."])
        .nom_note("ined.")
        .manuscript()
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Acosmeta", "Moeschler, MS.")
        .monomial("Acosmeta")
        .comb_authors(None, &["Moeschler"])
        .nom_note("ms.")
        .manuscript()
        .nothing_else();
    // the unpublished name is How's: a dangling "ex" must not drop him
    assert_name_auth("Mycetia macrocarpa", "F.C.How ex ined.")
        .species("Mycetia", "macrocarpa")
        .comb_authors(None, &["F.C.How"])
        .nom_note("ined.")
        .manuscript()
        .nothing_else();
    // repeated in both columns, the marker is recorded once
    assert_name_auth("Weiseria palustris anon. ined.", "anon. ined.")
        .species("Weiseria", "palustris")
        .comb_anon(None, &[])
        .nom_note("ined.")
        .manuscript()
        .nothing_else();
    assert_name_auth("Hygrocybe pratensis var. roseipes", "anon. ined.")
        .infra_species("Hygrocybe", "pratensis", Rank::Variety, "roseipes")
        .comb_anon(None, &[])
        .nom_note("ined.")
        .manuscript()
        .nothing_else();
}

#[test]
fn an_in_citation_of_an_anonymous_work() {
    // "anon." is a lower-case host, which the in-citation split did not accept
    assert_paths_agree("Solen truncatus", "Swainson in Anon. 1837");
    assert_name("Solen truncatus Swainson in Anon. 1837")
        .species("Solen", "truncatus")
        .comb_authors(Some("1837"), &["Swainson"])
        .published_in("anon. 1837")
        .published_in_year(Some(1837))
        .nothing_else();
}

/// A question mark glued to the manuscript marker is doubt, like a free-standing one; it kept the
/// marker from being read, and "ined" became an infraspecific epithet.
#[test]
fn a_question_mark_glued_to_the_manuscript_marker() {
    assert_name("Oxalis_barrelieri ined.?")
        .species("Oxalis", "barrelieri")
        .nom_note("ined.")
        .manuscript()
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
    assert_name("Betula pubescens subsp. suecica Gunnarss. ined.?")
        .infra_species("Betula", "pubescens", Rank::Subspecies, "suecica")
        .comb_authors(None, &["Gunnarss."])
        .nom_note("ined.")
        .manuscript()
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
}
