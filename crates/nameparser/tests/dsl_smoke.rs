// SPDX-License-Identifier: Apache-2.0
//! Smoke test for the `common` assertion DSL itself (the Rust port of Java `NameAssertion`).
//! Proves the builder + the strict `nothing_else()` closer work end to end. The ported Java
//! suites live in the other `tests/*.rs` files.

mod common;
use common::*;
use nameparser::model::{NameType, NomCode, Rank};

#[test]
fn infraspecies_with_authorship_and_code() {
    assert_name("Vulpes vulpes silaceus Miller, 1907")
        .infra_species("Vulpes", "vulpes", Rank::Subspecies, "silaceus")
        .comb_authors(Some("1907"), &["Miller"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn plain_binomial_with_author_no_year() {
    assert_name("Abies alba Mill.")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .nothing_else();
}

#[test]
fn unparsable_virus_carries_type_and_code() {
    assert_unparsable_code("Tobacco mosaic virus", NameType::Other, NomCode::Virus);
}
