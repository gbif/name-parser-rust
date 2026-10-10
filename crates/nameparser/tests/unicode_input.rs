// SPDX-License-Identifier: Apache-2.0
//! Characters a name carries in from its source file or keyboard that are not part of it: a byte
//! order mark, invisible format characters, fullwidth ASCII forms, decomposed accents, a scanned
//! zero for an O.

mod common;
use common::*;
use nameparser::model::{warnings, NomCode, Rank};

#[test]
fn a_byte_order_mark_is_ignored() {
    // the first cell of a UTF-8 file with a BOM
    assert_name("\u{feff}Abies alba Mill.")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .nothing_else();
}

#[test]
fn invisible_format_characters_are_ignored() {
    // zero-width spaces and joiners (the soft hyphen has a test of its own)
    assert_name("Abies\u{200b} alba\u{200d} Mill.\u{2060}")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .nothing_else();
}

/// A soft hyphen marks where a word may break: it splits nothing, and between a lower-case and a
/// capital letter it is the hyphen of a double name.
#[test]
fn a_soft_hyphen_splits_no_word() {
    assert_name("Oedogonium platygynum var. novae\u{ad}zelandiae Hirn")
        .infra_species("Oedogonium", "platygynum", Rank::Variety, "novaezelandiae")
        .comb_authors(None, &["Hirn"])
        .nothing_else();
    assert_name("Basilia speiseri (Miranda\u{ad}Ribeiro) Miranda-Ribeiro")
        .species("Basilia", "speiseri")
        .comb_authors(None, &["Miranda-Ribeiro"])
        .bas_authors(None, &["Miranda-Ribeiro"])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn fullwidth_forms_are_their_ascii_counterparts() {
    // East Asian keyboards type fullwidth letters and punctuation; the homoglyph table alone read
    // the fullwidth `ｌ` as the digit `1` ("Mi11.")
    assert_name("Abies alba Ｍｉｌｌ．")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .warning(&[warnings::HOMOGLYHPS])
        .nothing_else();
    assert_name_auth("Prosthodendrium luzonicum", "（Tubangui，1928）")
        .species("Prosthodendrium", "luzonicum")
        .bas_authors(Some("1928"), &["Tubangui"])
        .warning(&[warnings::HOMOGLYHPS])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn decomposed_accents_are_composed() {
    // NFD input, each accent a combining mark after its letter: Java split the word at each mark
    assert_name("Crisia romanica Za\u{301}gors\u{30c}ek & Szabo\u{301} 2008")
        .species("Crisia", "romanica")
        .comb_authors(Some("2008"), &["Zágoršek", "Szabó"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn a_zero_opening_a_word_is_a_scanned_capital_o() {
    assert_name("Phyllodoce mucosa 0ersted, 1843")
        .species("Phyllodoce", "mucosa")
        .comb_authors(Some("1843"), &["Oersted"])
        .warning(&[warnings::HOMOGLYHPS])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Attelabus 0l.")
        .monomial("Attelabus")
        .comb_authors(None, &["Ol."])
        .warning(&[warnings::HOMOGLYHPS])
        .nothing_else();
    // a code with more digits is no word
    assert_informal("Shewanella sp. 0m-11")
        .taxon("Shewanella")
        .taxon_rank(nameparser::model::Rank::Genus)
        .rank(nameparser::model::Rank::Species)
        .phrase("sp. 0m-11")
        .nothing_else();
}

#[test]
fn a_replacement_character_inside_a_word_is_a_missing_letter() {
    // a broken encoding leaves U+FFFD for the letter; it no longer splits the word
    assert_name("Fusinus eucos\u{FFFD}nius")
        .species("Fusinus", "eucosnius")
        .doubtful()
        .warning(&[warnings::UNUSUAL_CHARACTERS])
        .nothing_else();
    // in an author it is kept: "Guene" is no author (#104)
    assert_name("Macrotes cordovaria Guen\u{FFFD}e 1857")
        .species("Macrotes", "cordovaria")
        .comb_authors(Some("1857"), &["Guen\u{FFFD}e"])
        .code(NomCode::Zoological)
        .warning(&[warnings::UNUSUAL_CHARACTERS])
        .nothing_else();
}

#[test]
fn a_cyrillic_o_is_a_letter_o_not_a_zero() {
    // the homoglyph table lists it on the row of the digit 0
    assert_name("Anabarhynchus longepil\u{03BF}sus Lyneborg, 1992")
        .species("Anabarhynchus", "longepilosus")
        .comb_authors(Some("1992"), &["Lyneborg"])
        .code(NomCode::Zoological)
        .warning(&[warnings::HOMOGLYHPS])
        .nothing_else();
}

/// The Turkish dotless ı is a letter of its own in a surname (`Altıner`, `Yıldırım`), not a
/// look-alike of `i`; in the genus or an epithet it still is one (ChecklistBank dataset 1157).
#[test]
fn a_turkish_dotless_i_stays_in_a_surname() {
    assert_authorship("Altıner, 1988", &["Altıner"])
        .comb_authors(Some("1988"), &["Altıner"])
        .nothing_else();
    assert_name("Pseudovidalinidae Altıner, 1988")
        .monomial("Pseudovidalinidae")
        .comb_authors(Some("1988"), &["Altıner"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Abies alba Yıldırım & Kılıç")
        .species("Abies", "alba")
        .comb_authors(None, &["Yıldırım", "Kılıç"])
        .nothing_else();
    assert_name("Abies alıba Mill.")
        .species("Abies", "aliba")
        .comb_authors(None, &["Mill."])
        .warning(&[warnings::HOMOGLYHPS])
        .nothing_else();
}

#[test]
fn a_letter_lost_to_a_broken_encoding_stays_in_the_author() {
    // #104: the "?" or U+FFFD stands for a letter; glued, it made up an author "Srensen"
    assert_name("Anelasmocephalus lycosinus (S?rensen, 1873)")
        .species("Anelasmocephalus", "lycosinus")
        .bas_authors(Some("1873"), &["S?rensen"])
        .code(NomCode::Zoological)
        .warning(&[warnings::UNUSUAL_CHARACTERS])
        .nothing_else();
    assert_name("Adaina microdactyla (H\u{FFFD}bner, 1813)")
        .species("Adaina", "microdactyla")
        .bas_authors(Some("1813"), &["H\u{FFFD}bner"])
        .code(NomCode::Zoological)
        .warning(&[warnings::UNUSUAL_CHARACTERS])
        .nothing_else();
    assert_name_auth("Absconditella fossarum", "V?zda & Pišút, 1985")
        .species("Absconditella", "fossarum")
        .comb_authors(Some("1985"), &["V?zda", "Pišút"])
        .code(NomCode::Zoological)
        .warning(&[warnings::UNUSUAL_CHARACTERS])
        .nothing_else();
    // a lost apostrophe is put back, "dOrbigny" was read as an epithet
    assert_name("Aus bus d?Orbigny, 1839")
        .species("Aus", "bus")
        .comb_authors(Some("1839"), &["d'Orbigny"])
        .code(NomCode::Zoological)
        .warning(&[warnings::UNUSUAL_CHARACTERS])
        .nothing_else();
    // in an epithet the letter stays glued, flagged as before
    assert_name("Fusinus eucos?nius")
        .species("Fusinus", "eucosnius")
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
}
