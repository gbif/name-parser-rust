// SPDX-License-Identifier: Apache-2.0
//! Names that open with something other than a genus — a placeholder word (`unclassified`), a
//! generic organism label (`bacterium Ac10`), a classification path (`supf. Arrenuroidea fam.
//! Arrenuridae`) or a stray rank label (`Sp. Abacobius jekelii`). Surveyed from the 67.5M CLB
//! verbatim names: each used to lose the real taxon, read as the author of a bogus first word.

mod common;
use common::*;
use nameparser::model::NameType;

// ---- unclassified ------------------------------------------------------------------------------

#[test]
fn unclassified_is_a_placeholder() {
    for name in [
        "unclassified Aaadonta",
        "unclassified Aaadonta constricta",
        "unclassified eubacterium",
        "Unclassified Bacteria",
    ] {
        assert_unparsable(name, NameType::Placeholder);
    }
}

#[test]
fn unclassified_virus_keeps_its_virus_code() {
    assert_unparsable_code(
        "Grapevine red globe virus (unclassified)",
        NameType::Other,
        nameparser::model::NomCode::Virus,
    );
}
