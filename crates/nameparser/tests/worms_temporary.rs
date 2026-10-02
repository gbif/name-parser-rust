// SPDX-License-Identifier: Apache-2.0
//! WoRMS "temporary names": the 464 names WoRMS flags `taxonomicStatus = temporary name` — mostly
//! placeholders (`X incertae sedis`, `[unassigned] X`), provisional designations
//! (`Genus n_sp_NIWA_SO254 [of Dohrmann et al., 2023]`) and informal groupings. Each case is pinned
//! in both call shapes it reaches us in: the full string (name + authority concatenated) and the
//! ChecklistBank shape (name and authorship passed separately, plus the WoRMS rank as a hint).

mod common;
use common::*;
use nameparser::model::{NamePart, NameType, NomCode, Rank};

// ---- still-correct cases pinned so the fixes below can't regress them -------------------------

#[test]
fn incertae_sedis_variants_are_placeholders() {
    for name in [
        "Abyssochrysoidea incertae sedis",
        "Cephalopoda <i>incertae sedis</i>",
        "Ophiuroidea Incertae sedis",
        "Erpocotyle (incertae sedis)",
        "Paleopneustina incertae sedis A",
        "Sordariomycetes incertae sedis (fam.)",
        "Astrophorida incertae sedis Hooper & Maldonado, 2002",
        "[unassigned] Decapodiformes",
    ] {
        assert_unparsable(name, NameType::Placeholder);
    }
}

#[test]
fn fish_suborders_are_scientific_monomials() {
    assert_name("Acanthistioidei")
        .monomial("Acanthistioidei")
        .nothing_else();
    assert_name("Trigloidei")
        .monomial("Trigloidei")
        .nothing_else();
}

#[test]
fn bracketed_of_citation_after_sp_stays_in_the_phrase() {
    assert_informal("Accacladocoelium sp. [of Sokolov et al., 2025]")
        .taxon("Accacladocoelium")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. [of Sokolov et al., 2025]")
        .nothing_else();
}

// ---- A. a non-breaking space must not hide a placeholder ---------------------------------------

#[test]
fn nbsp_incertae_sedis_is_a_placeholder() {
    assert_unparsable(
        "Assimineidae\u{a0}incertae\u{a0}sedis",
        NameType::Placeholder,
    );
    assert_unparsable_rank(
        "Assimineidae\u{a0}incertae\u{a0}sedis",
        Rank::Genus,
        NameType::Placeholder,
    );
}

#[test]
fn nbsp_separated_name_parses_like_a_spaced_one() {
    assert_name("Abies\u{a0}alba\u{a0}Mill.")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .nothing_else();
}

// ---- B. underscore-glued provisional designations ---------------------------------------------

#[test]
fn underscore_new_species_designation_is_informal() {
    assert_informal("Aulocalyx n_sp_NIWA_SO254 [of Dohrmann et al., 2023]")
        .taxon("Aulocalyx")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("n_sp_NIWA_SO254 [of Dohrmann et al., 2023]")
        .nothing_else();
    assert_informal_hinted(
        "Aulocalyx n_sp_NIWA_SO254",
        Some("[of Dohrmann et al., 2023]"),
        Some(Rank::Species),
        None,
    )
    .taxon("Aulocalyx")
    .taxon_rank(Rank::Genus)
    .rank(Rank::Species)
    .phrase("n_sp_NIWA_SO254 [of Dohrmann et al., 2023]")
    .nothing_else();
}

#[test]
fn underscore_sp_designation_is_informal() {
    assert_informal("Eurythenes sp_DISCOLL_PAP_B [of Horton et al., 2020]")
        .taxon("Eurythenes")
        .rank(Rank::Species)
        .phrase("sp_DISCOLL_PAP_B [of Horton et al., 2020]");
    assert_informal("Oerstedia sp_Bering [of Chernyshev & Polyakova, 2022]")
        .taxon("Oerstedia")
        .rank(Rank::Species)
        .phrase("sp_Bering [of Chernyshev & Polyakova, 2022]");
    assert_informal("Parandaniexis sp_JAVA")
        .taxon("Parandaniexis")
        .rank(Rank::Species)
        .phrase("sp_JAVA");
}

#[test]
fn underscore_new_subspecies_designation_keeps_the_species() {
    assert_name("Farrea occa n_ssp_NIWA_SO254 [of Dohrmann et al., 2023]")
        .binomial("Farrea", None, "occa", Rank::Subspecies)
        .type_(NameType::Informal)
        .phrase("n_ssp_NIWA_SO254 [of Dohrmann et al., 2023]")
        .nothing_else();
}

#[test]
fn glued_new_genus_designation_is_informal_not_a_binomial() {
    assert_informal("Rossellidae_n_gen [of Dohrmann et al., 2023]")
        .taxon("Rossellidae")
        .rank(Rank::Genus)
        .phrase("n_gen [of Dohrmann et al., 2023]");
    assert_informal("Rossellidae_n_gen n_sp_NIWA_SO254 [of Dohrmann et al., 2023]")
        .taxon("Rossellidae")
        .rank(Rank::Species)
        .phrase("n_gen n_sp_NIWA_SO254 [of Dohrmann et al., 2023]");
}

#[test]
fn underscore_designation_canonical_has_no_synthetic_sp_marker() {
    // the designation already names the rank — rendering must not prepend its own "sp."
    for (input, canonical) in [
        (
            "Aulocalyx n_sp_NIWA_SO254 [of Dohrmann et al., 2023]",
            "Aulocalyx n_sp_NIWA_SO254 [of Dohrmann et al., 2023]",
        ),
        ("Parandaniexis sp_JAVA", "Parandaniexis sp_JAVA"),
        (
            "Rossellidae_n_gen n_sp_NIWA_SO254 [of Dohrmann et al., 2023]",
            "Rossellidae n_gen n_sp_NIWA_SO254 [of Dohrmann et al., 2023]",
        ),
    ] {
        match nameparser::parse(input, None, None, None) {
            nameparser::ParseResult::Informal(i) => assert_eq!(i.canonical_name(), canonical),
            other => panic!("expected `{input}` to be Informal, got {other:?}"),
        }
    }
}

#[test]
fn separate_authorship_of_a_provisional_name_lands_in_the_phrase() {
    // The embedded form swallows a trailing authorship into the phrase
    // (`Cantuaria sp. Forster, 1968` → phrase `sp. Forster, 1968`); the CLB shape must not
    // silently drop it instead.
    assert_informal_hinted(
        "Cantuaria sp.",
        Some("Forster, 1968"),
        Some(Rank::Species),
        None,
    )
    .taxon("Cantuaria")
    .taxon_rank(Rank::Genus)
    .rank(Rank::Species)
    .phrase("sp. Forster, 1968")
    .nothing_else();
}

#[test]
fn redundant_separate_authorship_is_not_appended_twice() {
    // sources often repeat the authorship in both columns, not always identically
    for authorship in ["Forster, 1968", "Forster 1968", "Forster"] {
        assert_informal_hinted(
            "Cantuaria sp. Forster, 1968",
            Some(authorship),
            Some(Rank::Species),
            None,
        )
        .taxon("Cantuaria")
        .phrase("sp. Forster, 1968");
    }
    // a bare marker repeated as the "authorship"
    assert_informal_hinted("Saprinus sp.", Some("sp."), Some(Rank::Genus), None)
        .taxon("Saprinus")
        .phrase("sp.");
}

#[test]
fn separate_of_citation_of_a_designation_lands_in_the_phrase() {
    assert_informal_hinted(
        "Accacladocoelium sp.",
        Some("[of Sokolov et al., 2025]"),
        Some(Rank::Species),
        None,
    )
    .taxon("Accacladocoelium")
    .rank(Rank::Species)
    .phrase("sp. [of Sokolov et al., 2025]")
    .nothing_else();
    assert_name_hinted(
        "Farrea occa n_ssp_NIWA_SO254",
        Some("[of Dohrmann et al., 2023]"),
        Some(Rank::Subspecies),
        None,
    )
    .binomial("Farrea", None, "occa", Rank::Subspecies)
    .type_(NameType::Informal)
    .phrase("n_ssp_NIWA_SO254 [of Dohrmann et al., 2023]")
    .nothing_else();
    // the same citation repeated in both columns is not appended twice
    assert_informal_hinted(
        "Accacladocoelium sp. [of Sokolov et al., 2025]",
        Some("[of Sokolov et al., 2025]"),
        None,
        None,
    )
    .taxon("Accacladocoelium")
    .phrase("sp. [of Sokolov et al., 2025]");
}

// ---- C. a bracketed single word is an annotation, not an author --------------------------------

#[test]
fn bracketed_single_word_is_a_phrase_not_an_author() {
    assert_informal("Acanthoecidae [Nudiform]")
        .taxon("Acanthoecidae")
        .phrase("[Nudiform]");
    assert_informal("Leptocephalus [Moringuidae]")
        .taxon("Leptocephalus")
        .phrase("[Moringuidae]");
    assert_informal_hinted(
        "Leptocephalus",
        Some("[Ophichthidae]"),
        Some(Rank::Genus),
        None,
    )
    .taxon("Leptocephalus")
    .rank(Rank::Genus)
    .phrase("[Ophichthidae]");
}

#[test]
fn bracketed_author_in_the_separate_authorship_is_still_an_author() {
    // `[Renier]`, `[Boucek]`, `[Röding]` are common anonymous-work authors in CLB's authorship
    // column — only a family-group name in brackets is an annotation.
    assert_name_hinted("Spongia", Some("[Renier]"), Some(Rank::Genus), None)
        .monomial_rank("Spongia", Rank::Genus)
        .comb_authors(None, &["Renier"])
        .nothing_else();
}

#[test]
fn bracketed_anonymous_author_with_year_is_still_an_author() {
    // ICZN Recommendation 51D: the author of an anonymous work in square brackets.
    assert_name("Aus bus [Hübner], 1806")
        .species("Aus", "bus")
        .comb_authors(Some("1806"), &["Hübner"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn parenthesised_single_word_is_still_a_basionym_author() {
    assert_name("Abies alba (Müller)")
        .species("Abies", "alba")
        .bas_authors(None, &["Müller"])
        .code(NomCode::Zoological)
        .nothing_else();
}

// ---- D. single-letter epithets are informal designations ---------------------------------------

#[test]
fn single_letter_species_epithet_is_informal() {
    // ICZN Art. 11.9.1: a species-group name has more than one letter. `a` is also an author
    // particle, which used to swallow it into the authorship.
    for letter in ["a", "b"] {
        assert_name(&format!(
            "Collettea {letter} Blazewicz-Paszkowycz & Larsen, 2005"
        ))
        .species("Collettea", letter)
        .type_(NameType::Informal)
        .comb_authors(Some("2005"), &["Blazewicz-Paszkowycz", "Larsen"])
        .code(NomCode::Zoological)
        .nothing_else();
    }
    assert_name_hinted(
        "Collettea a",
        Some("Blazewicz-Paszkowycz & Larsen, 2005"),
        Some(Rank::Species),
        None,
    )
    .species("Collettea", "a")
    .type_(NameType::Informal)
    .comb_authors(Some("2005"), &["Blazewicz-Paszkowycz", "Larsen"])
    .nothing_else();
}

#[test]
fn abbreviated_particle_after_the_genus_is_still_an_author_particle() {
    assert_name("Micropleura v Linstow, 1906")
        .monomial("Micropleura")
        .comb_authors(Some("1906"), &["v Linstow"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn single_letter_after_an_infraspecific_marker_is_the_epithet() {
    // mirrors the existing uppercase handling (`var. A` → infraspecificEpithet `A`, INFORMAL)
    for letter in ["a", "b"] {
        assert_name(&format!("Undella hyalina var. {letter} Brandt, 1907"))
            .infra_species("Undella", "hyalina", Rank::Variety, letter)
            .type_(NameType::Informal)
            .comb_authors(Some("1907"), &["Brandt"])
            .code(NomCode::Zoological)
            .nothing_else();
    }
}

#[test]
fn letter_ahead_of_a_real_epithet_is_not_a_designation() {
    // an informal rank letter before the epithet, after a marker…
    assert_name("Tetraria compar var. b minor Kük.")
        .infra_species("Tetraria", "compar", Rank::Variety, "minor")
        .comb_authors(None, &["Kük."])
        .nothing_else();
    // …and abbreviated or split letter epithets in the species slot keep their type
    for name in [
        "Curculio c.album Scopoli, J.A., 1763",
        "Drepana z nigrum Bryk, 1942",
    ] {
        match nameparser::parse(name, None, None, None) {
            nameparser::ParseResult::Parsed(pn) => {
                assert_eq!(pn.type_, NameType::Scientific, "{name}")
            }
            other => panic!("expected `{name}` to parse, got {other:?}"),
        }
    }
}

#[test]
fn parenthesised_letter_after_an_infraspecific_marker_is_the_phrase() {
    // the #16 designation path keeps it verbatim, like `var. 3`
    for letter in ["a", "A"] {
        assert_name(&format!(
            "Paraconchoecia oblonga f. ({letter}) Müller, 1906"
        ))
        .binomial("Paraconchoecia", None, "oblonga", Rank::Form)
        .phrase(&format!("({letter})"))
        .type_(NameType::Informal)
        .comb_authors(Some("1906"), &["Müller"])
        .code(NomCode::Zoological)
        .nothing_else();
    }
}

// ---- E. a parenthesised sensu is a taxonomic note ----------------------------------------------

#[test]
fn parenthesised_sensu_is_a_taxonomic_note() {
    assert_name("Gregariella splendida (sensu Reeve, 1858)")
        .species("Gregariella", "splendida")
        .sensu("sensu Reeve, 1858")
        .nothing_else();
}

#[test]
fn parenthesised_sensu_of_a_provisional_name_stays_in_the_phrase() {
    // the flat Informal result has no note slot — the citation must not be dropped
    assert_informal("Coprosma species a (sensu Eagle)")
        .taxon("Coprosma")
        .rank(Rank::Species)
        .phrase("species a (sensu Eagle)");
}

// ---- F. a fully quoted multi-word label is not a name ------------------------------------------

#[test]
fn fully_quoted_group_label_is_unparsable() {
    assert_unparsable("\"Lower Heterobranchia\"", NameType::Other);
}

#[test]
fn fully_quoted_name_is_parsed_without_its_quotes() {
    // a CSV/export artefact: the quotes wrap a perfectly good name
    assert_name("\"Accipiter bicolor (Vieillot, 1817)\"")
        .species("Accipiter", "bicolor")
        .bas_authors(Some("1817"), &["Vieillot"])
        .code(NomCode::Zoological)
        .nothing_else();
}

// ---- G. trace fossils / ichnotaxa ---------------------------------------------------------------

#[test]
fn trace_fossil_label_with_an_anchor_is_informal() {
    assert_informal("Echinoid trace fossils (ichnotaxa)")
        .taxon("Echinoid")
        .phrase("trace fossils (ichnotaxa)");
    assert_informal("Echinoid trace fossils")
        .taxon("Echinoid")
        .phrase("trace fossils");
    assert_informal("Trilobita-trace fossils")
        .taxon("Trilobita")
        .phrase("trace fossils");
}

#[test]
fn trace_fossil_label_without_an_anchor_is_unparsable() {
    assert_unparsable("Trace fossils", NameType::Other);
    assert_unparsable("Trace-fossils", NameType::Other);
    assert_unparsable("Trace-fossils attributed to", NameType::Other);
}

#[test]
fn ichno_genera_and_bare_anchor_stay_scientific() {
    assert_name("Ichnospongia")
        .monomial("Ichnospongia")
        .nothing_else();
    assert_name("Echinoid").monomial("Echinoid").nothing_else();
}

// ---- H. a leading question mark doubts the genus ----------------------------------------------

#[test]
fn leading_question_mark_doubts_the_genus() {
    // like `Sydonia? alba` (a `?` qualifier, INFORMAL, doubtful) — the genus must not be lost
    for name in ["?Sydonia alba", "? Sydonia alba"] {
        assert_name(name)
            .species("Sydonia", "alba")
            .qualifiers(&[(NamePart::Generic, "?")])
            .type_(NameType::Informal)
            .doubtful()
            .warning(&["question marks removed"])
            .nothing_else();
    }
    assert_name("?Lupocyclus sexspinosus Leene, 1940")
        .species("Lupocyclus", "sexspinosus")
        .qualifiers(&[(NamePart::Generic, "?")])
        .type_(NameType::Informal)
        .doubtful()
        .warning(&["question marks removed"])
        .comb_authors(Some("1940"), &["Leene"])
        .code(NomCode::Zoological)
        .nothing_else();
    // a bare uninomial stays a Parsed name, so the qualifier and the flag survive
    assert_name("?Monotremata")
        .monomial("Monotremata")
        .qualifiers(&[(NamePart::Generic, "?")])
        .doubtful()
        .warning(&["question marks removed"])
        .nothing_else();
}

#[test]
fn leading_question_mark_before_an_indet_marker_keeps_the_genus() {
    assert_informal_hinted(
        "?Archaeopharetra sp.",
        Some("of Zhuravlev & Gravestock 1994"),
        Some(Rank::Species),
        None,
    )
    .taxon("Archaeopharetra")
    .rank(Rank::Species)
    .phrase("sp. of Zhuravlev & Gravestock 1994");
}

#[test]
fn leading_question_mark_before_an_epithet_is_still_a_missing_genus() {
    assert_name("? alba Smith")
        .species("?", "alba")
        .comb_authors(None, &["Smith"])
        .nothing_else();
}

#[test]
fn leading_question_mark_with_a_subgenus_doubts_the_genus() {
    assert_name("? Callidium (Phymatodes) semicircularis")
        .species_ig("Callidium", "Phymatodes", "semicircularis")
        .qualifiers(&[(NamePart::Generic, "?")])
        .type_(NameType::Informal)
        .doubtful()
        .warning(&["question marks removed"])
        .nothing_else();
}
