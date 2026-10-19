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
fn glued_to_the_subgenus_brackets_it_doubts_the_subgenus() {
    // the epithet was lost to the unparsed rest
    assert_name("Glenea (Macroglenea)? anulicornis Pic, 1926")
        .species_ig("Glenea", "Macroglenea", "anulicornis")
        .qualifiers(&[(NamePart::Infrageneric, "?")])
        .comb_authors(Some("1926"), &["Pic"])
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
fn glued_to_the_last_epithet_it_doubts_that_epithet_not_an_author() {
    assert_name("Abies alba?")
        .species("Abies", "alba")
        .qualifiers(&[(NamePart::Specific, "?")])
        .type_(NameType::Informal)
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
    assert_name_auth("Abies alba?", "Mill.")
        .species("Abies", "alba")
        .qualifiers(&[(NamePart::Specific, "?")])
        .comb_authors(None, &["Mill."])
        .type_(NameType::Informal)
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
    assert_name("Abies alba var. nana?")
        .infra_species("Abies", "alba", Rank::Variety, "nana")
        .qualifiers(&[(NamePart::Infraspecific, "?")])
        .type_(NameType::Informal)
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
}

#[test]
fn glued_to_a_uninomial_it_doubts_the_uninomial() {
    for name in ["Abies?", "Abies ?"] {
        assert_name(name)
            .monomial("Abies")
            .qualifiers(&[(NamePart::Generic, "?")])
            .type_(NameType::Informal)
            .doubtful()
            .warning(&[warnings::QUESTION_MARKS_REMOVED])
            .nothing_else();
    }
}

#[test]
fn trailing_a_bare_name_it_doubts_the_last_epithet_glued_or_not() {
    assert_name("Abies alba ?")
        .species("Abies", "alba")
        .qualifiers(&[(NamePart::Specific, "?")])
        .type_(NameType::Informal)
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
    assert_name("Abies alba var. nana ?")
        .infra_species("Abies", "alba", Rank::Variety, "nana")
        .qualifiers(&[(NamePart::Infraspecific, "?")])
        .type_(NameType::Informal)
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
}

#[test]
fn trailing_a_bare_subgenus_it_doubts_the_subgenus() {
    assert_name("Faunus (Pseudobellardia) ?")
        .infrageneric_at("Faunus", Rank::InfragenericName, "Pseudobellardia")
        .qualifiers(&[(NamePart::Infrageneric, "?")])
        .type_(NameType::Informal)
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
}

#[test]
fn trailing_a_phrase_it_only_doubts_the_name() {
    assert_name("Siphonodendron sp. a ?")
        .genus_rank("Siphonodendron", Rank::Species)
        .phrase("sp. a")
        .type_(NameType::Informal)
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED, warnings::INDETERMINED])
        .nothing_else();
}

#[test]
fn trailing_an_authorship_it_only_doubts_the_name() {
    assert_name("Abies alba Mill. ?")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
    assert_name("Aus bus (Smith, 1886) ?")
        .species("Aus", "bus")
        .bas_authors(Some("1886"), &["Smith"])
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
}

#[test]
fn glued_to_an_author_it_marks_the_authorship_uncertain() {
    assert_name("Aus bus Sess?")
        .species("Aus", "bus")
        .comb_authors(None, &["Sess"])
        .doubtful()
        .warning(&[warnings::UNCERTAIN_AUTHORSHIP])
        .nothing_else();
    assert_name("Mikania attenuata DC.?")
        .species("Mikania", "attenuata")
        .comb_authors(None, &["DC."])
        .doubtful()
        .warning(&[warnings::UNCERTAIN_AUTHORSHIP])
        .nothing_else();
    assert_name("Calicium claviculare var. pusillum Ach.?")
        .infra_species("Calicium", "claviculare", Rank::Variety, "pusillum")
        .comb_authors(None, &["Ach."])
        .doubtful()
        .warning(&[warnings::UNCERTAIN_AUTHORSHIP])
        .nothing_else();
    assert_name_auth("Mikania attenuata", "DC.?")
        .species("Mikania", "attenuata")
        .comb_authors(None, &["DC."])
        .doubtful()
        .warning(&[warnings::UNCERTAIN_AUTHORSHIP])
        .nothing_else();
}

#[test]
fn a_doubted_genus_anchors_the_name() {
    assert_eq!(anchor("Abies? alba").as_deref(), Some("Abies"));
    assert_eq!(anchor("?Sydonia alba").as_deref(), Some("Sydonia"));
    assert_eq!(anchor("?Monotremata").as_deref(), Some("Monotremata"));
    // and so does another qualifier of the genus (#192)
    assert_eq!(
        anchor("cf. Platypeltis croftii").as_deref(),
        Some("Platypeltis")
    );
    assert_eq!(anchor("aff. Anthurium").as_deref(), Some("Anthurium"));
}

#[test]
fn a_qualifier_already_there_stays_and_the_question_mark_still_doubts() {
    for name in [
        "Afiacyathus cf. sibiricus?",
        "Machairodus cf. crenatidens ?",
    ] {
        let epithet = name.split(' ').nth(2).unwrap().trim_end_matches([' ', '?']);
        assert_name(name)
            .species(name.split(' ').next().unwrap(), epithet)
            .qualifiers(&[(NamePart::Specific, "cf.")])
            .type_(NameType::Informal)
            .doubtful()
            .warning(&[warnings::QUESTION_MARKS_REMOVED])
            .nothing_else();
    }
}

#[test]
fn glued_to_an_author_after_a_slash_or_in_square_brackets() {
    assert_name("Ophryoglena bacterocaryon Roque et al./Savoie?, 1965")
        .species("Ophryoglena", "bacterocaryon")
        .doubtful()
        .warning(&[warnings::UNCERTAIN_AUTHORSHIP]);
    assert_name("Baridius brannani Schaufuss, C. [or L.W.?] , 1866")
        .species("Baridius", "brannani")
        .doubtful()
        .warning(&[warnings::UNCERTAIN_AUTHORSHIP]);
}

#[test]
fn a_question_mark_no_step_placed_still_doubts_the_name() {
    for name in [
        "Argiope Argiope pentagona?",
        "Colpidium cucullus Kent, sensu?, 1881",
        "Agrodiaetus merzbacherii (Cram?)",
        "Acontiodus sp.?",
    ] {
        let pn = nameparser::parse(name, None, None, None)
            .parsed()
            .cloned()
            .unwrap_or_else(|| panic!("{name} parses"));
        assert!(pn.doubtful, "{name}");
    }
    // a "?" between letters is a lost letter, a "?" after a digit a doubtful year
    for name in [
        "Aphis myrsinitidis Petrovi?-Obradovi? & Leclant, 1998",
        "Aus bus Smith, 1887?",
    ] {
        let pn = nameparser::parse(name, None, None, None)
            .parsed()
            .cloned()
            .unwrap_or_else(|| panic!("{name} parses"));
        assert!(!pn.doubtful, "{name}");
    }
}
