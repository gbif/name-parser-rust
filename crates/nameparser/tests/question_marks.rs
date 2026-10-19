// SPDX-License-Identifier: Apache-2.0
//! A question mark in a name: the open-nomenclature doubt of an identification. Glued to the end of
//! a word it doubts that word (`Ferganoconcha? oblonga`, the usual palaeontological postfix);
//! standing free or glued to the next word it doubts the next epithet (`Abies ? alba`, `Abies
//! ?alba`), or the last one when nothing follows. Glued to an author it marks the authorship
//! uncertain (`Sess?`, `DC.?`).

mod common;
use common::*;
use nameparser::model::{warnings, NamePart, NameType, Rank};

fn anchor(input: &str) -> Option<String> {
    nameparser::parse(input, None, None, None)
        .parsed()
        .and_then(|pn| pn.anchor())
        .and_then(|a| a.canonical_name())
}

#[test]
fn glued_to_the_genus_it_doubts_the_genus() {
    assert_name("Abies? alba")
        .species("Abies", "alba")
        .qualifiers(&[(NamePart::Generic, "?")])
        .type_(NameType::Informal)
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
    assert_name("Eupoikilofusa? rhomba Smith, 1900")
        .species("Eupoikilofusa", "rhomba")
        .qualifiers(&[(NamePart::Generic, "?")])
        .comb_authors(Some("1900"), &["Smith"])
        .type_(NameType::Informal)
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
}

#[test]
fn glued_to_the_species_it_doubts_the_species() {
    assert_name("Buteo borealis? ventralis")
        .infra_species("Buteo", "borealis", Rank::InfraspecificName, "ventralis")
        .qualifiers(&[(NamePart::Specific, "?")])
        .type_(NameType::Informal)
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
}

#[test]
fn standing_free_it_doubts_the_next_epithet() {
    for name in ["Abies ? alba", "Abies ?alba"] {
        assert_name(name)
            .species("Abies", "alba")
            .qualifiers(&[(NamePart::Specific, "?")])
            .type_(NameType::Informal)
            .doubtful()
            .warning(&[warnings::QUESTION_MARKS_REMOVED])
            .nothing_else();
    }
    assert_name("Buteo borealis ? ventralis")
        .infra_species("Buteo", "borealis", Rank::InfraspecificName, "ventralis")
        .qualifiers(&[(NamePart::Infraspecific, "?")])
        .type_(NameType::Informal)
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
}

#[test]
fn a_doubted_genus_anchors_the_name() {
    assert_eq!(anchor("Abies? alba").as_deref(), Some("Abies"));
    assert_eq!(anchor("?Sydonia alba").as_deref(), Some("Sydonia"));
    assert_eq!(anchor("?Monotremata").as_deref(), Some("Monotremata"));
    // another qualifier of the genus still leaves nothing
    assert_eq!(anchor("cf. Platypeltis croftii"), None);
}
