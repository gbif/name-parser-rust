// SPDX-License-Identifier: Apache-2.0
//! Characters a name carries in from its source file or keyboard that are not part of it: a byte
//! order mark, invisible format characters, fullwidth ASCII forms.

mod common;
use common::*;
use nameparser::model::{warnings, NomCode};

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
    // zero-width spaces and joiners (not the soft hyphen, see `unicode::normalize_input`)
    assert_name("Abies\u{200b} alba\u{200d} Mill.\u{2060}")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
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
