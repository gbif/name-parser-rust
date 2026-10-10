// SPDX-License-Identifier: Apache-2.0
//! Names that open with something other than a genus — a placeholder word (`unclassified`), a
//! generic organism label (`bacterium Ac10`), a classification path (`supf. Arrenuroidea fam.
//! Arrenuridae`) or a stray rank label (`Sp. Abacobius jekelii`). Surveyed from the 67.5M CLB
//! verbatim names: each used to lose the real taxon, read as the author of a bogus first word.

mod common;
use common::*;
use nameparser::model::{NameType, NomCode, Rank};

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

// ---- generic organism label + strain code ------------------------------------------------------

#[test]
fn a_vernacular_organism_label_with_a_capital_is_no_name() {
    // #103: the capitalised word is no genus, nor a host binomial one with three epithets
    for name in [
        "Adelie penguin guano bacterium 92",
        "Amazonian soil bacterium M14",
        "Acromyrmex echinatior fungal symbiont Acech322",
    ] {
        assert_unparsable(name, NameType::Identifier);
    }
    for name in [
        "Acyrthosiphon pisum primary endosymbiont",
        "Ammonia oxidizing bacteria",
        "Acyrthosiphon kondoi symbiont bacterium",
    ] {
        assert_unparsable(name, NameType::Other);
    }
    // a Latin epithet is no label
    assert_name("Andrena minutula alga Warncke, 1974")
        .infra_species("Andrena", "minutula", Rank::Subspecies, "alga")
        .comb_authors(Some("1974"), &["Warncke"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn organism_label_with_a_strain_code_is_an_identifier() {
    for name in [
        "bacterium Ac10",
        "archaeon Ea1",
        "bacterium A-15",
        "haloarchaeon BitternsUMGM",
        "bacterium Rauti",
        "bacterium str. S36",
        "bacterium sp. A15",
        "actinomycete P032.UGM060121-02",
        "marine actinobacterium F10",
        "soil bacterium PM2-P1-17",
        "delta proteobacterium S2250",
        "bacterium enrichment culture clone OB115",
        "fungal sp. ARIZ AZ0346",
        "actinobacterium SCGC AAA015-M09",
        "bacterium '081007-Grout Aid-gul'",
    ] {
        assert_unparsable(name, NameType::Identifier);
    }
}

#[test]
fn organism_label_without_a_code_is_unparsable_other() {
    for name in [
        "endosymbiont of Chlamys farreri",
        "bacterium symbiont of Osedax sp.",
        "coryneform bacterium",
        "anammox bacterium",
    ] {
        assert_unparsable(name, NameType::Other);
    }
}

#[test]
fn organism_word_as_a_missing_genus_epithet_is_untouched() {
    // a real epithet + author with the genus missing still takes the missing-genus reading: a
    // placeholder, unparsable, whose parts the raw parse keeps
    assert_unparsable("fungi Meigen, 1830", NameType::Placeholder);
    assert_raw_name("fungi Meigen, 1830")
        .species("?", "fungi")
        .comb_authors(Some("1830"), &["Meigen"])
        .code(nameparser::model::NomCode::Zoological)
        .warning(&["epithet without genus"])
        .type_(NameType::Placeholder)
        .nothing_else();
}

#[test]
fn named_symbiont_with_a_genus_is_informal() {
    // not an identifier: the genus anchors an unnamed species, like `Burkholderia sp. (Gigaspora
    // margarita endosymbiont)`; the label is no epithet, it opens the phrase with the host
    assert_informal("Wolbachia endosymbiont of Drosophila simulans")
        .taxon("Wolbachia")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("endosymbiont of Drosophila simulans")
        .nothing_else();
    assert_informal("Wolbachia endosymbiont")
        .taxon("Wolbachia")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("endosymbiont")
        .nothing_else();
    // a strain code behind a higher taxon's "bacterium"
    assert_informal("Acidimicrobiales bacterium JGI 01_E13")
        .taxon("Acidimicrobiales")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("bacterium JGI 01_E13")
        .nothing_else();
    assert_informal("Acidimicrobiales bacterium")
        .taxon("Acidimicrobiales")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("bacterium")
        .nothing_else();
    // without an author "Navicula bacterium" is a species too
    assert_name("Navicula bacterium")
        .species("Navicula", "bacterium")
        .nothing_else();
    // with an author it is a real epithet: the diatom Navicula bacterium
    assert_name("Navicula bacterium Frenguelli")
        .species("Navicula", "bacterium")
        .comb_authors(None, &["Frenguelli"])
        .nothing_else();
    // a symbiont named after its host has no anchor
    assert_unparsable("Acyrthosiphon kondoi endosymbiont", NameType::Other);
}

#[test]
fn organism_label_word_before_an_author_or_infraspecific_name_is_a_real_epithet() {
    assert_name("Russula archaea R. Heim")
        .species("Russula", "archaea")
        .comb_authors(None, &["R.Heim"])
        .nothing_else();
    assert_name("Rinodina archaea (Ach.) Arnold")
        .species("Rinodina", "archaea")
        .comb_authors(None, &["Arnold"])
        .bas_authors(None, &["Ach."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Rinodina archaea f. cinerascens H. Magn.")
        .infra_species("Rinodina", "archaea", Rank::Form, "cinerascens")
        .comb_authors(None, &["H.Magn."])
        .nothing_else();
    assert_name("Orania archaea hitomiae Houart & Moe, 2011")
        .infra_species("Orania", "archaea", Rank::Subspecies, "hitomiae")
        .comb_authors(Some("2011"), &["Houart", "Moe"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Lithochytris archaea Riedel and Sanfilippo, 1970")
        .species("Lithochytris", "archaea")
        .comb_authors(Some("1970"), &["Riedel", "Sanfilippo"])
        .code(NomCode::Zoological)
        .nothing_else();
    // `archaea` after a genus is the Latin "ancient", no label
    assert_name("Neoschoengastia archaea")
        .species("Neoschoengastia", "archaea")
        .nothing_else();
}

#[test]
fn organism_label_with_a_culture_accession_or_a_letter_is_an_identifier() {
    for name in [
        "bacterium DSM 6505",
        "bacterium RCC 1888",
        "cyanobacterium PCC 7702",
        "actinobacterium D",
    ] {
        assert_unparsable(name, NameType::Identifier);
    }
}

// ---- rank-prefixed classification paths ---------------------------------------------------------

#[test]
fn rank_prefixed_lineage_keeps_the_last_taxon_at_its_rank() {
    for (name, uninomial, rank) in [
        (
            "supf. Arrenuroidea fam. Arrenuridae",
            "Arrenuridae",
            Rank::Family,
        ),
        (
            "fam. Clavidae gen. Cordylophora",
            "Cordylophora",
            Rank::Genus,
        ),
        ("subo. Apocrita supf. Apoidea", "Apoidea", Rank::Superfamily),
        (
            "subf. Brachytroninae gen. Dendroaeschna",
            "Dendroaeschna",
            Rank::Genus,
        ),
        (
            "info. Anomopoda fam. Daphniidae",
            "Daphniidae",
            Rank::Family,
        ),
        ("phy. Annelida cla. Polychaeta", "Polychaeta", Rank::Class),
        (
            "supo. Decapodiformes ord. Teuthida",
            "Teuthida",
            Rank::Order,
        ),
        ("subc. Copepoda ord. Cyclopoida", "Cyclopoida", Rank::Order),
        (
            "infc. Marsupialia ord. Diprotodontia",
            "Diprotodontia",
            Rank::Order,
        ),
        ("fam. Arrenuridae", "Arrenuridae", Rank::Family),
        ("(supergen. Allopsontus)", "Allopsontus", Rank::Supergenus),
    ] {
        assert_name(name)
            .monomial_rank(uninomial, rank)
            .nothing_else();
    }
}

#[test]
fn rank_prefixed_taxon_keeps_its_authorship() {
    assert_name("fam. Arrenuridae Thor, 1900")
        .monomial_rank("Arrenuridae", Rank::Family)
        .comb_authors(Some("1900"), &["Thor"])
        .code(nameparser::model::NomCode::Zoological)
        .nothing_else();
}

// ---- a stray leading species label --------------------------------------------------------------

#[test]
fn leading_species_label_is_dropped_from_a_binomial() {
    for name in [
        "Sp. Abacobius jekelii",
        "sp. Abacobius jekelii",
        "spec. Abacobius jekelii",
    ] {
        assert_name(name)
            .species("Abacobius", "jekelii")
            .nothing_else();
    }
    assert_name("Sp. Lupocyclus sexspinosus Leene, 1940")
        .species("Lupocyclus", "sexspinosus")
        .comb_authors(Some("1940"), &["Leene"])
        .code(nameparser::model::NomCode::Zoological)
        .nothing_else();
}

#[test]
fn leading_species_label_before_a_trinomial_keeps_the_trinomial() {
    assert_name("Sp. Antitrophus lygodesmiae pisum")
        .infra_species(
            "Antitrophus",
            "lygodesmiae",
            Rank::InfraspecificName,
            "pisum",
        )
        .nothing_else();
}

#[test]
fn leading_species_label_before_a_subgenus_binomial_is_dropped() {
    assert_name("Sp. Eubulus (Cryptorhynchus) orthomasticus")
        .species_ig("Eubulus", "Cryptorhynchus", "orthomasticus")
        .nothing_else();
}

/// A lone epithet whose author comes separately, or whose rank is species or below, lacks its
/// genus: `denisi` + `(Arlé, 1939)` was made the genus "Denisi" of an informal name (ChecklistBank
/// dataset 2130). The same name with its author in the string was a placeholder already.
#[test]
fn a_lone_epithet_lacks_its_genus() {
    for (name, auth, rank) in [
        ("denisi", Some("(Arlé, 1939)"), Some(Rank::Species)),
        ("denisi", Some("(Arlé, 1939)"), None),
        ("denisi", None, Some(Rank::Species)),
        ("denisi (Arlé, 1939)", None, None),
    ] {
        match nameparser::parse(name, auth, rank, None) {
            nameparser::ParseResult::Unparsable(e) => {
                assert_eq!(e.type_, NameType::Placeholder, "{name} {auth:?}")
            }
            other => panic!("{name} {auth:?} {rank:?}: expected a placeholder, got {other:?}"),
        }
    }
    // a lone word alone is still a uninomial
    assert_name("denisi").monomial("Denisi");
}

/// A lower-cased genus with its subgenus stays a name: the bracket holds no author.
#[test]
fn a_lower_cased_genus_with_its_subgenus() {
    assert_name("balea (Balea) swalesi").species_ig("Balea", "Balea", "swalesi");
}
