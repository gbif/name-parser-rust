// SPDX-License-Identifier: Apache-2.0
//! Placeholder authorships — "Missing", "Not specified", "Unknown", "Not applicable" — and the
//! "not validly publ." status.
//!
//! ChecklistBank sources fill an empty authorship column with a placeholder: "Missing" (22k rows)
//! and "Not specified" (19k) alone. `org.gbif:name-parser` 4.2.0 dropped only a trailing "Not
//! applicable" / "Not given" / "Not known" / "Not recorded" / "Not found" on the name string, with
//! the "authorship placeholder removed" warning (ChecklistBank's AUTHORSHIP_REMOVED issue); the
//! separately supplied authorship never ran that step, so every placeholder there became an
//! author. Now both paths drop the wider set, keeping any real author in front of it and flagging
//! the removal with the same warning. Deliberate changes against Java parity.

mod common;
use common::*;
use nameparser::model::{warnings, NameType, NomCode, Rank};

#[test]
fn a_placeholder_as_the_whole_authorship_is_removed_and_flagged() {
    for authorship in [
        "Missing",
        "Not specified",
        "Not applicable",
        "Not applicable.",
        "Unknown",
        "NONE",
        "NA",
        "author unknown",
        "Author",
    ] {
        assert_name_auth("Aspelta baltica", authorship)
            .species("Aspelta", "baltica")
            .warning(&[warnings::AUTHORSHIP_REMOVED])
            .nothing_else();
    }
}

#[test]
fn a_placeholder_after_a_real_author_or_basionym_is_removed() {
    assert_name_auth("Abies alba", "Mill. Not given")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
    assert_name_auth("Monas vulgaris", "(Cienkowski) Unknown")
        .species("Monas", "vulgaris")
        .bas_authors(None, &["Cienkowski"])
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn a_placeholder_on_the_name_string_is_removed() {
    assert_name("Grindelia platyphylla Author")
        .species("Grindelia", "platyphylla")
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
    assert_name("Ascidia zara author unknown")
        .species("Ascidia", "zara")
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
    // "unknown" no longer makes the whole record a placeholder name when it is the authorship
    assert_name("Caobangia Unknown")
        .monomial("Caobangia")
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
    // ... but a name that is nothing but the placeholder word is left alone
    assert_name("None").monomial("None").nothing_else();
    assert_unparsable("Unknown", NameType::Placeholder);
}

#[test]
fn a_placeholder_does_not_join_a_provisional_phrase() {
    assert_informal_hinted("Leiognathus sp.", Some("Not applicable"), None, None)
        .taxon("Leiognathus")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp.")
        .nothing_else();
}

#[test]
fn not_validly_published_is_a_nomenclatural_note() {
    assert_name_auth(
        "Bridelia scleroneuroides var. typica",
        "Gehrm., not validly publ.",
    )
    .infra_species("Bridelia", "scleroneuroides", Rank::Variety, "typica")
    .comb_authors(None, &["Gehrm."])
    .nom_note("not validly publ.")
    .nothing_else();
    // repeated in the separate authorship, it is recorded once
    assert_name_auth(
        "Bridelia scleroneuroides var. typica Gehrm., not validly publ.",
        "Gehrm., not validly publ.",
    )
    .infra_species("Bridelia", "scleroneuroides", Rank::Variety, "typica")
    .comb_authors(None, &["Gehrm."])
    .nom_note("not validly publ.")
    .nothing_else();
}
