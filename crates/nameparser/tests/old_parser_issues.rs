// SPDX-License-Identifier: Apache-2.0
//! The names of issues filed against the Java parser (label "old parser"), one test per issue.

mod common;
use common::*;
use nameparser::model::{warnings, NameType, NomCode, Rank};

#[test]
fn issue_126_temp_is_an_author() {
    assert_name("Encyonema muelleri f. ventricosa (Temp. & M. Perag.) Czarn.")
        .infra_species("Encyonema", "muelleri", Rank::Form, "ventricosa")
        .comb_authors(None, &["Czarn."])
        .bas_authors(None, &["Temp.", "M.Perag."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Pinnularia tabellaria var. stauroneiformis Temp. & Peragallo")
        .infra_species("Pinnularia", "tabellaria", Rank::Variety, "stauroneiformis")
        .comb_authors(None, &["Temp.", "Peragallo"])
        .nothing_else();
    assert_name("Rhachomyces girardii Lepesme & G. Temp. ex Santam.")
        .species("Rhachomyces", "girardii")
        .comb_authors(None, &["Santam."])
        .comb_ex_authors(&["Lepesme", "G.Temp."])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn issue_129_subgen_marker() {
    assert_name("Aus subgen. Bus cus")
        .species_ig("Aus", "Bus", "cus")
        .nothing_else();
}

#[test]
fn issue_130_superspecies() {
    assert_name("Euplocania supersp. enderleini")
        .binomial("Euplocania", None, "enderleini", Rank::SpeciesAggregate)
        .nothing_else();
}

#[test]
fn issue_132_misapplied_name_with_its_reference() {
    assert_name(
        "Ficus exasperata auct. non Vahl: De Wildeman & Durand in Ann. Mus. Congo Belge, B, Bot., \
         ser. 2, 1: 54. 1899",
    )
    .species("Ficus", "exasperata")
    .sensu(
        "auct. non Vahl: De Wildeman & Durand in Ann. Mus. Congo Belge, B, Bot., ser. 2, 1: 54. \
         1899",
    )
    .nothing_else();
}

#[test]
fn issue_133_filius_as_epithet() {
    assert_name("Polynema filius")
        .species("Polynema", "filius")
        .nothing_else();
}

#[test]
fn issue_137_not_validly_published() {
    assert_name("Antidesma lanceolatum Hook.f. & Thomson, not validly publ.")
        .species("Antidesma", "lanceolatum")
        .comb_authors(None, &["Hook.f.", "Thomson"])
        .nom_note("not validly publ.")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Asperula galioides Bourg. ex Willk. & Lange, not validly publ.")
        .species("Asperula", "galioides")
        .comb_authors(None, &["Willk.", "Lange"])
        .comb_ex_authors(&["Bourg."])
        .nom_note("not validly publ.")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Babiana pygmaea Spreng. ex Steud., not validly publ.")
        .species("Babiana", "pygmaea")
        .comb_authors(None, &["Steud."])
        .comb_ex_authors(&["Spreng."])
        .nom_note("not validly publ.")
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn issue_138_pro_synonymo() {
    assert_name("Agrostis arenaria Schur, pro syn.")
        .species("Agrostis", "arenaria")
        .comb_authors(None, &["Schur"])
        .nom_note("pro syn.")
        .nothing_else();
    assert_name("Agrostis aspera Willd. ex Steud., pro syn.")
        .species("Agrostis", "aspera")
        .comb_authors(None, &["Steud."])
        .comb_ex_authors(&["Willd."])
        .nom_note("pro syn.")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Aegopogon gracilis Cav. ex Roem. & Schult., pro syn.")
        .species("Aegopogon", "gracilis")
        .comb_authors(None, &["Roem.", "Schult."])
        .comb_ex_authors(&["Cav."])
        .nom_note("pro syn.")
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn issue_140_bracketed_ex_authors_of_bacteria() {
    assert_name_auth(
        "Abies alba",
        "(ex Veillon and Zuber 1898) Gharbia and Shah 1992",
    )
    .species("Abies", "alba")
    .comb_authors(Some("1992"), &["Gharbia", "Shah"])
    .comb_ex_authors(&["Veillon", "Zuber"])
    .code(NomCode::Bacterial)
    .nothing_else();
    assert_name_auth(
        "Abies alba",
        "(ex Coronelli et al. 1974) Tamura et al. 2011",
    )
    .species("Abies", "alba")
    .comb_authors(Some("2011"), &["Tamura", "al."])
    .comb_ex_authors(&["Coronelli", "al."])
    .code(NomCode::Bacterial)
    .nothing_else();
    assert_name_auth("Abies alba", "(ex Ahrens 1968) Rüger and Höfle 1992")
        .species("Abies", "alba")
        .comb_authors(Some("1992"), &["Rüger", "Höfle"])
        .comb_ex_authors(&["Ahrens"])
        .code(NomCode::Bacterial)
        .nothing_else();
    assert_name_auth(
        "Abies alba",
        "(ex Kreier and Ristic 1963) Ristic and Kreier 1984",
    )
    .species("Abies", "alba")
    .comb_authors(Some("1984"), &["Ristic", "Kreier"])
    .comb_ex_authors(&["Kreier", "Ristic"])
    .code(NomCode::Bacterial)
    .nothing_else();
}

#[test]
fn issue_146_pro_parte_authorships() {
    assert_authorship(
        "(Farr) Calder & Roy L. Taylor p.p.",
        &["Calder", "Roy L.Taylor"],
    )
    .comb_authors(None, &["Calder", "Roy L.Taylor"])
    .bas_authors(None, &["Farr"])
    .doubtful()
    .nothing_else();
    assert_authorship("Lepage p.p.", &["Lepage"])
        .comb_authors(None, &["Lepage"])
        .doubtful()
        .nothing_else();
    assert_authorship("Linnaeus p.p.", &["Linnaeus"])
        .comb_authors(None, &["Linnaeus"])
        .doubtful()
        .nothing_else();
}

#[test]
fn issue_148_ex_without_ex_author() {
    assert_authorship("ex DC.", &["DC."])
        .comb_authors(None, &["DC."])
        .nothing_else();
    assert_authorship("ex Blanco", &["Blanco"])
        .comb_authors(None, &["Blanco"])
        .nothing_else();
}

#[test]
fn issue_152_auct_is_no_epithet() {
    assert_name("Rumex orbicularis auct.")
        .species("Rumex", "orbicularis")
        .sensu("auct.")
        .nothing_else();
}

#[test]
fn issue_154_temp_in_the_basionym() {
    assert_authorship("(Temp. & Brun) Cleve", &["Cleve"])
        .comb_authors(None, &["Cleve"])
        .bas_authors(None, &["Temp.", "Brun"])
        .nothing_else();
}

#[test]
fn issue_156_numbered_species_with_or_without_dot() {
    assert_informal("Galaxias sp. 3")
        .taxon("Galaxias")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. 3")
        .nothing_else();
    assert_informal("Galaxias sp 3")
        .taxon("Galaxias")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp 3")
        .nothing_else();
}

#[test]
fn issue_158_named_hybrid() {
    assert_name("Carex ×subviridula (Kükenthal) Fernald")
        .species("Carex", "subviridula")
        .notho(&[nameparser::model::NamePart::Specific])
        .comb_authors(None, &["Fernald"])
        .bas_authors(None, &["Kükenthal"])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn issue_159_hybrid_formulas() {
    assert_unparsable("Carex folliculata × Carex lurida", NameType::Formula);
    assert_unparsable("Carex paleacea × Carex stricta", NameType::Formula);
    assert_unparsable_rank(
        "Asplenium germanicum x trichomanes C.Chr.",
        Rank::Species,
        NameType::Formula,
    );
}

#[test]
fn issue_161_two_letter_epithet() {
    assert_name("Apogonichthys fo")
        .species("Apogonichthys", "fo")
        .nothing_else();
}

#[test]
fn issue_162_lower_case_word_in_brackets() {
    assert_name("Belonella (pacifica) borealis")
        .species_ig("Belonella", "Pacifica", "borealis")
        .doubtful()
        .nothing_else();
}

#[test]
fn issue_163_junior_and_senior_as_epithets() {
    assert_name("Anomia sella subsp. junior")
        .infra_species("Anomia", "sella", Rank::Subspecies, "junior")
        .nothing_else();
    assert_name("Triton vestitus var. senior")
        .infra_species("Triton", "vestitus", Rank::Variety, "senior")
        .nothing_else();
}

#[test]
fn issue_164_diacritics_in_epithets() {
    assert_name("Congeria kušceri")
        .species("Congeria", "kušceri")
        .nothing_else();
    assert_name("Leucomastus bureši")
        .species("Leucomastus", "bureši")
        .nothing_else();
}

#[test]
fn issue_165_standalone_subgenera() {
    assert_name("Calathus (Lindrothius) KURNAKOV 1961")
        .infrageneric_at("Calathus", Rank::Subgenus, "Lindrothius")
        .comb_authors(Some("1961"), &["Kurnakov"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Hymenophyllum subgen. Hymenoglossum (Presl) R.M.Tryon & A.Tryon")
        .infrageneric_at("Hymenophyllum", Rank::Subgenus, "Hymenoglossum")
        .comb_authors(None, &["R.M.Tryon", "A.Tryon"])
        .bas_authors(None, &["Presl"])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn issue_168_not_of_is_a_note() {
    assert_name("Labrus bimaculatus (not of Linnaeus, 1758)")
        .species("Labrus", "bimaculatus")
        .sensu("not of Linnaeus, 1758")
        .nothing_else();
}

#[test]
fn issue_169_van_den_is_no_epithet() {
    assert_name("Schwetzochromis torrenticola (Thys van den Audenaerde, 1963)")
        .species("Schwetzochromis", "torrenticola")
        .bas_authors(Some("1963"), &["Thys van den Audenaerde"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Schwetzochromis torrenticola Thys van den Audenaerde, 1963")
        .species("Schwetzochromis", "torrenticola")
        .comb_authors(Some("1963"), &["Thys van den Audenaerde"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn issue_170_sensu_lato_with_a_space() {
    assert_name("Acacia acinacea s. l.")
        .species("Acacia", "acinacea")
        .sensu("s. l.")
        .nothing_else();
    assert_name("Acacia acinacea s.l.")
        .species("Acacia", "acinacea")
        .sensu("s.l.")
        .nothing_else();
}

#[test]
fn issue_127_full_width_punctuation() {
    assert_name_auth("Angerianus longicornis", "Liu & Xu，sp. nov.")
        .species("Angerianus", "longicornis")
        .comb_authors(None, &["Liu", "Xu"])
        .nom_note("sp. nov.")
        .warning(&[warnings::HOMOGLYHPS])
        .nothing_else();
    assert_name_auth("Angerianus longicornis", "Liu & Mu，sp.nov.")
        .species("Angerianus", "longicornis")
        .comb_authors(None, &["Liu", "Mu"])
        .nom_note("sp. nov.")
        .warning(&[warnings::HOMOGLYHPS])
        .nothing_else();
    assert_name_auth("Angerianus longicornis", "L. Li & S. H. Li, sp. nov.")
        .species("Angerianus", "longicornis")
        .comb_authors(None, &["L.Li", "S.H.Li"])
        .nom_note("sp. nov.")
        .nothing_else();
    assert_name_auth("Abies alba", "Spaeth，1914")
        .species("Abies", "alba")
        .comb_authors(Some("1914"), &["Spaeth"])
        .code(NomCode::Zoological)
        .warning(&[warnings::HOMOGLYHPS])
        .nothing_else();
    assert_name_auth("Abies alba", "（Spaeth）")
        .species("Abies", "alba")
        .bas_authors(None, &["Spaeth"])
        .code(NomCode::Zoological)
        .warning(&[warnings::HOMOGLYHPS])
        .nothing_else();
    // the note inside the basionym bracket, before its year
    assert_name_auth("Ilema badia", "(Chao，comb．nov．,1986)")
        .species("Ilema", "badia")
        .bas_authors(Some("1986"), &["Chao"])
        .nom_note("comb. nov.")
        .code(NomCode::Zoological)
        .warning(&[warnings::HOMOGLYHPS])
        .nothing_else();
}
