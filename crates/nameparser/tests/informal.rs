// SPDX-License-Identifier: Apache-2.0
//! The 5.0.0 informal / semistructured band, tested through the three-way `parse` via the
//! fluent `assert_informal` / `assert_name` DSL helpers. Cases are lifted from the reservoir
//! samples of the 67.5M verbatim-corpus study (`docs/superpowers/findings/`): overwhelmingly
//! molecular / DNA-barcoding provisional species `Genus sp. <specimen/culture/BOLD code>`.
//!
//! The design contract this file pins:
//!  * a supraspecific taxon carrying a provisional designation with NO species epithet → `Informal`,
//!    a flat `taxon` + `taxon_rank` + `rank` + `phrase` + `code`;
//!  * a name WITH a species epithet (a binomial core — incl. cf./aff. and infraspecific-indet) stays
//!    `Parsed`, so its `specific_authorship` (unrepresentable by a flat anchor) survives;
//!  * a determined monomial (`Rhizobium`) stays `Parsed`/SCIENTIFIC — not informal.

mod common;
use common::*;
use nameparser::model::{NamePart, NameType, Rank};
use nameparser::ParseResult;

// ---- Informal: supraspecific anchor + provisional designation, no species epithet -------------

#[test]
fn molecular_provisional_species_with_a_captured_tag() {
    // ~99.8% of the band: genus-anchored, SPECIES rank, a specimen/culture/BOLD code phrase.
    assert_informal("Serratia sp. RE1-2a")
        .taxon("Serratia")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("RE1-2a")
        .nothing_else();
    assert_informal("Plasmodium sp. SYBOR9")
        .taxon("Plasmodium")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("SYBOR9")
        .nothing_else();
}

#[test]
fn multi_token_specimen_tag_is_captured_as_the_phrase() {
    // The 5.0.0 tag-capture enhancement rescues the ~382k rows whose multi-token trailing tag the
    // 4.2.0 parser dropped (or misread as an author): the whole verbatim tail becomes the phrase.
    assert_informal("Rhizobium sp. RMCC TR1811")
        .taxon("Rhizobium")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("RMCC TR1811")
        .nothing_else();
    assert_informal("Ichneumonidae sp. UAM Ento 145060")
        .taxon("Ichneumonidae") // a family, but the parser's best guess is the genus slot (not backbone-validated)
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("UAM Ento 145060")
        .nothing_else();
}

#[test]
fn species_n_with_a_trailing_note_keeps_the_whole_tail_as_the_phrase() {
    // "once a phrase starts, it runs to the end": everything after "(sp|spec|species) N" is part of
    // the phrase, verbatim — so a trailing "(=synonym)" note is NOT split off as a subgenus/epithet.
    // Surfaced by the CoL backend's dwca/17 fixture.
    assert_informal("Dichanthelium species 12 (=chrysopsidifolium)")
        .taxon("Dichanthelium")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("species 12 (=chrysopsidifolium)")
        .nothing_else();
}

#[test]
fn australian_herbarium_locality_convention() {
    // "Genus sp. <Locality>" — the type-specimen-based convention; the locality becomes the phrase
    // instead of the 4.2.0 parser's misread "author Rocky Creek".
    assert_informal("Elaeocarpus sp. Rocky Creek")
        .taxon("Elaeocarpus")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("Rocky Creek")
        .nothing_else();
}

#[test]
fn numbered_placeholder() {
    // Phrase leading tokens are dominated by bare numbers (sp. 1, sp. 2, …).
    assert_informal("Allium sp. 1")
        .taxon("Allium")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("1")
        .nothing_else();
}

#[test]
fn bare_genus_sp_has_no_phrase() {
    // A bare "Genus sp." — indeterminate, no distinguishing tag.
    assert_informal("Rhizobium sp.")
        .taxon("Rhizobium")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .no_phrase()
        .nothing_else();
}

#[test]
fn single_uppercase_letter_designator() {
    // "Genus sp. E" — a single-letter informal designator captured as the phrase.
    assert_informal("Bryozoan sp. E")
        .taxon("Bryozoan")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("E")
        .nothing_else();
}

// ---- Boundary: a species epithet is present → must STAY Parsed, NOT Informal ------------------

#[test]
fn cf_binomial_stays_parsed_with_its_qualifier() {
    // A complete binomial that was only "informal" via an open-nomenclature qualifier — the
    // qualifier is an annotation (epithetQualifier), not a reclassification.
    assert_name("Salicornia cf. patula")
        .species("Salicornia", "patula")
        .type_(NameType::Informal)
        .qualifiers(&[(NamePart::Specific, "cf.")])
        .nothing_else();
}

#[test]
fn aff_binomial_with_authorship_stays_parsed() {
    // aff. on a complete binomial WITH authorship — the clearest reason it must stay Parsed: a flat
    // Informal anchor could not represent the species-level authorship.
    assert_name("Turritella aff. adulterata Deshayes 1820-1851")
        .species("Turritella", "adulterata")
        .comb_authors(Some("1820"), &["Deshayes"])
        .qualifiers(&[(NamePart::Specific, "aff.")])
        .type_(NameType::Informal);
}

#[test]
fn infraspecific_indeterminate_stays_parsed() {
    // "Salix alba subsp. B" has a species epithet ("alba"), so it stays Parsed — a flat Informal
    // could not hold an infraspecific-level designation hanging off a determined species.
    assert_name("Salix alba subsp. B")
        .infra_species("Salix", "alba", Rank::Subspecies, "B")
        .type_(NameType::Informal);
}

#[test]
fn binomial_with_a_species_n_tag_stays_parsed_keeping_the_phrase() {
    // "Genus epithet species N" / "Genus epithet sp. N" — a placeholder tag appended to a binomial.
    // The species epithet is present, so it stays Parsed; the trailing tag is preserved as the phrase
    // (type INFORMAL) rather than reading "species"/"sp" as a (blacklisted) infraspecific epithet and
    // dropping the number. Surfaced by the CoL backend's dwca/17 fixture.
    assert_name("Dichanthelium chrysopsidifolium species 12")
        .species("Dichanthelium", "chrysopsidifolium")
        .type_(NameType::Informal)
        .phrase("species 12")
        .nothing_else();
    assert_name("Dichanthelium chrysopsidifolium sp. 12")
        .species("Dichanthelium", "chrysopsidifolium")
        .type_(NameType::Informal)
        .phrase("12")
        .nothing_else();
}

#[test]
fn bare_determined_genus_stays_parsed_scientific() {
    // "Rhizobium" alone is a determined SCIENTIFIC monomial — NOT informal (no provisional marker).
    assert_name("Rhizobium")
        .monomial("Rhizobium")
        .type_(NameType::Scientific)
        .nothing_else();
}

// ---- Monomial-aggregate / lineage groupings: anchored → Informal, anchorless → OTHER -----------

#[test]
fn monomial_aggregate_groups_are_rescued_to_informal() {
    // 5.0.0 rescue (see pipeline::preflight): an anchored monomial-aggregate (group/complex) or a
    // clean-genus "-lineage" becomes an Informal — the monomial is the anchor, the marker the phrase.
    assert_informal("Bartonella group")
        .taxon("Bartonella")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Unranked)
        .phrase("group")
        .nothing_else();
    assert_informal("Vermistella-lineage")
        .taxon("Vermistella")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Unranked)
        .phrase("lineage")
        .nothing_else();
}

#[test]
fn anchorless_clade_and_code_labels_are_unparsable_other() {
    // Anchorless phylogenetic clade labels ("Unnamed clade") and OTU-/strain-code lineage stems
    // ("NC12A-lineage") have no clean single-taxon anchor → Unparsable(OTHER).
    for input in [
        "Amauropeltoid clade",
        "Unnamed clade",
        "NC12A-lineage",
        "he2-lineage",
    ] {
        match nameparser::parse(input, None, None, None) {
            ParseResult::Unparsable(e) => assert_eq!(
                e.type_,
                NameType::Other,
                "`{input}` should be Unparsable(OTHER)"
            ),
            other => panic!("expected `{input}` Unparsable(OTHER), got {other:?}"),
        }
    }
}
