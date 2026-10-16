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

#[test]
fn issue_139_names_found_by_robert_mesibov() {
    assert_name("Solanum juzepczukii f. ckaisalla Lechn.")
        .infra_species("Solanum", "juzepczukii", Rank::Form, "ckaisalla")
        .comb_authors(None, &["Lechn."])
        .nothing_else();
    // "ht" is short for "hort."
    assert_name("Adiantum sanctae ht")
        .species("Adiantum", "sanctae")
        .comb_authors(None, &["hort."])
        .nothing_else();
    assert_name("Citrus clementina hort. ex Tanaka")
        .species("Citrus", "clementina")
        .comb_authors(None, &["Tanaka"])
        .comb_ex_authors(&["hort."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Pittosporum hosmeri var. st.-johnii")
        .infra_species("Pittosporum", "hosmeri", Rank::Variety, "st.-johnii")
        .nothing_else();
    // "manuscript" spelled out, before the author
    assert_name("Enoplolindenius castronus manuscript Leclercq")
        .species("Enoplolindenius", "castronus")
        .comb_authors(None, &["Leclercq"])
        .manuscript()
        .nom_note("ms")
        .nothing_else();
    assert_name("Dendrobiastes hyperythrus mjöbergi")
        .infra_species(
            "Dendrobiastes",
            "hyperythrus",
            Rank::InfraspecificName,
            "mjöbergi",
        )
        .nothing_else();
    assert_name("Eremobates microscopulata ms")
        .species("Eremobates", "microscopulata")
        .manuscript()
        .nom_note("ms")
        .nothing_else();
    // "nsp" is short for "n. sp."; before the epithet it opens a provisional name
    assert_informal("Anania nsp hortulata")
        .taxon("Anania")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("nsp hortulata")
        .nothing_else();
    assert_name("Pseudosinella biunguiculata nsp")
        .species("Pseudosinella", "biunguiculata")
        .nom_note("nsp.")
        .nothing_else();
    assert_name("Brachymenium saint-pierrei Thériot, 1931")
        .species("Brachymenium", "saint-pierrei")
        .comb_authors(Some("1931"), &["Thériot"])
        .code(NomCode::Zoological)
        .nothing_else();
    // the same marker as "n. sp."
    assert_informal("Jardinella n. sp. keel")
        .taxon("Jardinella")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("n. sp. keel")
        .nothing_else();
    assert_name("Coscinodiscus densus Grove & Sturt manuscript in Rattray, 1890")
        .species("Coscinodiscus", "densus")
        .comb_authors(Some("1890"), &["Rattray"])
        .comb_ex_authors(&["Grove", "Sturt"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn issue_128_emended_and_manuscript_authorships() {
    // the ex-author whose name the authors made available, in a bracket after the year
    assert_ex_authorship(
        "Loeblich & Tappan, 1957 (ex. Le Calvez, 1949)",
        Some("Le Calvez"),
        &["Loeblich", "Tappan"],
    )
    .comb_authors(Some("1957"), &["Loeblich", "Tappan"])
    .comb_ex_authors(&["Le Calvez"])
    .nothing_else();
    assert_authorship("(Cushman) em. Loeblich & Tappan, 1957", &[])
        .bas_authors(None, &["Cushman"])
        .sensu("em. Loeblich & Tappan, 1957")
        .nothing_else();
    assert_authorship("(Myatlyuk, 1949 emend. Myatlyuk, 1953)", &[])
        .bas_authors(Some("1949"), &["Myatlyuk"])
        .sensu("emend. Myatlyuk, 1953")
        .nothing_else();
    // a manuscript mark in a bracket of its own
    assert_authorship("Plummer (ms.) , 1926", &["Plummer"])
        .comb_authors(Some("1926"), &["Plummer"])
        .manuscript()
        .nom_note("ms.")
        .nothing_else();
    assert_authorship("(Ten Dam, 1947) em. Hofker, 1952", &[])
        .bas_authors(Some("1947"), &["Ten Dam"])
        .sensu("em. Hofker, 1952")
        .nothing_else();
    assert_authorship(
        "Schlagintweit in Schlagintweit & Mitchell, 202",
        &["Schlagintweit"],
    )
    .comb_authors(Some("202"), &["Schlagintweit"])
    .published_in("Schlagintweit & Mitchell, 202")
    .nothing_else();
    assert_authorship("(Haeusler) em. Oesterle, 1968", &[])
        .bas_authors(None, &["Haeusler"])
        .sensu("em. Oesterle, 1968")
        .nothing_else();
    assert_authorship("(Gutschick Weiner & Young) em. Conkin & Conkin, 1964", &[])
        .bas_authors(None, &["Gutschick Weiner", "Young"])
        .sensu("em. Conkin & Conkin, 1964")
        .nothing_else();
    // more of the kind: "(MS)" between the author and the work publishing the name, an
    // author's manuscript name a later author published
    assert_name("Asteriscus wega Valenciennes (MS) in Perrier, 1869")
        .species("Asteriscus", "wega")
        .comb_authors(Some("1869"), &["Perrier"])
        .comb_ex_authors(&["Valenciennes"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Hirundo leucopyga Meyen (Licht. MS) 1834")
        .species("Hirundo", "leucopyga")
        .comb_authors(Some("1834"), &["Meyen"])
        .comb_ex_authors(&["Licht."])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Chromis Oken, 1817 (ex Cuvier)")
        .monomial("Chromis")
        .comb_authors(Some("1817"), &["Oken"])
        .comb_ex_authors(&["Cuvier"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn issue_150_in_litteris() {
    assert_name("Anterhynchium alecto lalepi (Cheesm.i.l.)")
        .infra_species("Anterhynchium", "alecto", Rank::Subspecies, "lalepi")
        .bas_authors(None, &["Cheesm."])
        .manuscript()
        .nom_note("i.l.")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Litorina vilis (Menke in litt.)")
        .species("Litorina", "vilis")
        .bas_authors(None, &["Menke"])
        .manuscript()
        .nom_note("in litt.")
        .code(NomCode::Zoological)
        .nothing_else();
    // published by the author before the bracket
    assert_name("Acanthocinus rusticus Klug (Dej. in litt.) 1829")
        .species("Acanthocinus", "rusticus")
        .comb_authors(Some("1829"), &["Klug"])
        .comb_ex_authors(&["Dej."])
        .code(NomCode::Zoological)
        .nothing_else();
}
