// SPDX-License-Identifier: Apache-2.0
//! Species epithets that are spelled exactly like an author particle — `Zodarion van
//! Bosmans, 2009`, `Cantuaria delli Forster, 1968`.
//!
//! `Genus <particle> Author, year` is structurally ambiguous and no amount of lexical
//! analysis settles it: `Allidothrips zur Strassen, 1968` really is the genus *Allidothrips*
//! authored by Richard *zur Strassen*, while `Zodarion van Bosmans, 2009` really is the
//! species *Zodarion van* authored by *Bosmans*. Both readings are legal; telling them apart
//! needs knowledge the parser does not have.
//!
//! The one signal that *does* settle it is the caller's rank hint. So:
//!
//!   * **no rank hint** — unchanged, the particle wins and the input is read as a uninomial
//!     with a particled author. This is the safe default: across the CoL corpus
//!     (`testdata/colxr26.6-names.txt`) 2960 distinct names match `Genus <particle> …` and the
//!     large majority (`de` 1683, `van` 572, `von` 396, `zur` 35) really are particled authors —
//!     a 40-name sample checked against ChecklistBank came back 40/40 genera, none species.
//!   * **rank hint of species or below** — the caller has asserted this is a species, so the
//!     word in the epithet slot is taken as the epithet and the rest as the authorship.
//!
//! Reported from ChecklistBank dataset 56185 (World Spider Catalog), where all 7 affected
//! names carried `col:rank=species` and came back flagged
//! `unparsable name` + `indetermined` + `parsed name differs`: under a `SPECIES` hint the old
//! boundary rule found no epithet at all and fell into the indetermined branch, which
//! *also* discarded the authorship — so `Cantuaria delli Forster, 1968` lost both `delli`
//! and `Forster, 1968`. See `src/token.rs` for the 50-entry particle table and
//! `src/pipeline/authorship_split.rs` `find_boundary` for the rule.

mod common;
use common::*;
use nameparser::model::{NomCode, Rank};

// ---- the 7 ChecklistBank dataset 56185 (World Spider Catalog) names --------------------------

#[test]
fn wsc_particle_epithets_parse_as_species_under_a_species_rank_hint() {
    assert_name_rank("Cantuaria delli Forster, 1968", Rank::Species)
        .species("Cantuaria", "delli")
        .comb_authors(Some("1968"), &["Forster"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_rank("Gasparia delli (Forster, 1955)", Rank::Species)
        .species("Gasparia", "delli")
        .bas_authors(Some("1955"), &["Forster"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_rank("Eresus da Lin & Li, 2022", Rank::Species)
        .species("Eresus", "da")
        .comb_authors(Some("2022"), &["Lin", "Li"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_rank("Leptonetela la Wang & Li, 2017", Rank::Species)
        .species("Leptonetela", "la")
        .comb_authors(Some("2017"), &["Wang", "Li"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_rank("Malamatidia zu Jäger & Dankittipakul, 2010", Rank::Species)
        .species("Malamatidia", "zu")
        .comb_authors(Some("2010"), &["Jäger", "Dankittipakul"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_rank("Orcevia zu Yu & Zhang, 2023", Rank::Species)
        .species("Orcevia", "zu")
        .comb_authors(Some("2023"), &["Yu", "Zhang"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_rank("Zodarion van Bosmans, 2009", Rank::Species)
        .species("Zodarion", "van")
        .comb_authors(Some("2009"), &["Bosmans"])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// The same 7 with no rank hint stay uninomials with a particled author — the deliberate,
/// unchanged default for the ambiguous case. Pinned here so the no-hint path cannot drift
/// silently: flipping any of these is a corpus-wide behaviour change, not a bug fix.
#[test]
fn the_same_names_without_a_rank_hint_stay_uninomials_with_a_particled_author() {
    assert_name("Cantuaria delli Forster, 1968")
        .monomial("Cantuaria")
        .comb_authors(Some("1968"), &["delli Forster"]);
    assert_name("Eresus da Lin & Li, 2022")
        .monomial("Eresus")
        .comb_authors(Some("2022"), &["da Lin", "Li"]);
    assert_name("Leptonetela la Wang & Li, 2017")
        .monomial("Leptonetela")
        .comb_authors(Some("2017"), &["la Wang", "Li"]);
    assert_name("Zodarion van Bosmans, 2009")
        .monomial("Zodarion")
        .comb_authors(Some("2009"), &["van Bosmans"]);
}

// ---- boundary: genuine particled authors must not be split into an epithet -------------------

/// Real uninomials whose author carries a particle. Without a hint (and with a genus-group
/// hint) the particle must stay glued to the surname — these are the majority case the
/// no-hint default protects.
#[test]
fn genuine_particled_authors_of_uninomials_are_not_split() {
    // Richard zur Strassen, thrips taxonomist — 40 such genera in the CoL corpus.
    assert_name("Allidothrips zur Strassen, 1968")
        .monomial("Allidothrips")
        .comb_authors(Some("1968"), &["zur Strassen"]);
    assert_name_rank("Allidothrips zur Strassen, 1968", Rank::Genus)
        .monomial_rank("Allidothrips", Rank::Genus)
        .comb_authors(Some("1968"), &["zur Strassen"]);
    // Stefano delle Chiaje.
    assert_name("Balanoglossus delle Chiaje, 1829")
        .monomial("Balanoglossus")
        .comb_authors(Some("1829"), &["delle Chiaje"]);
}

/// A multi-word particle chain ("van den Boom", "von der Linde") is never an epithet — a
/// species epithet is a single word, so a particle followed by another LOWER-case word keeps
/// the author reading. Note this cannot be a particle-table test: "den" is not in the table.
#[test]
fn a_particle_chain_is_never_taken_as_an_epithet() {
    assert_name("Cladoniicola van den Boom, 2001")
        .monomial("Cladoniicola")
        .comb_authors(Some("2001"), &["van den Boom"]);
    assert_name("Verrucaria von der Linde, 1902")
        .monomial("Verrucaria")
        .comb_authors(Some("1902"), &["von der Linde"]);
    // Under a (mistaken) SPECIES hint the chain is still refused: rather than inventing the
    // epithet "van" and the author "den Boom", the name stays indetermined.
    assert_informal_hinted(
        "Cladoniicola van den Boom, 2001",
        None,
        Some(Rank::Species),
        None,
    )
    .taxon("Cladoniicola")
    .taxon_rank(Rank::Genus)
    .rank(Rank::Species);
    assert_informal_hinted(
        "Verrucaria von der Linde, 1902",
        None,
        Some(Rank::Species),
        None,
    )
    .taxon("Verrucaria")
    .rank(Rank::Species);
}

/// The hint only reaches the epithet slot: once a real epithet has been seen, a following
/// particle is still an author, hint or no hint.
#[test]
fn a_particle_after_a_real_epithet_is_still_an_author() {
    assert_name_rank("Cladoniicola staurospora van den Boom, 2001", Rank::Species)
        .species("Cladoniicola", "staurospora")
        .comb_authors(Some("2001"), &["van den Boom"]);
    assert_name_rank("Aaaba nodosa de Laubenfels, 1936", Rank::Species)
        .species("Aaaba", "nodosa")
        .comb_authors(Some("1936"), &["de Laubenfels"])
        .code(NomCode::Zoological);
}

/// Words that merely *look* like particles but are not in the table ("dela", "den") already
/// parsed as epithets without a hint — the hint must not disturb them.
#[test]
fn non_table_prefix_like_epithets_are_unaffected_by_the_hint() {
    assert_name_rank("Antaplaga dela Druce, 1904", Rank::Species)
        .species("Antaplaga", "dela")
        .comb_authors(Some("1904"), &["Druce"])
        .code(NomCode::Zoological);
    assert_name_rank("Agnetina den Cao, T.K.T. & Bae, 2006", Rank::Species)
        .species("Agnetina", "den")
        .comb_authors(Some("2006"), &["T.K.T.Cao", "Bae"])
        .code(NomCode::Zoological);
}
