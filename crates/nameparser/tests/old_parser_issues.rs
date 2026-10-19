// SPDX-License-Identifier: Apache-2.0
//! The names of issues filed against the Java parser (label "old parser"), one test per issue.

mod common;
use common::*;
use nameparser::model::{warnings, NamePart, NameType, NomCode, Rank};

#[test]
fn issue_126_temp_is_an_author() {
    assert_name("Encyonema muelleri f. ventricosa (Temp. & M. Perag.) Czarn.")
        .infra_species("Encyonema", "muelleri", Rank::Form, "ventricosa")
        .comb_authors(None, &["Czarn."])
        .bas_authors(None, &["Temp.", "M.Perag."])
        .nothing_else();
    assert_name("Pinnularia tabellaria var. stauroneiformis Temp. & Peragallo")
        .infra_species("Pinnularia", "tabellaria", Rank::Variety, "stauroneiformis")
        .comb_authors(None, &["Temp.", "Peragallo"])
        .nothing_else();
    assert_name("Rhachomyces girardii Lepesme & G. Temp. ex Santam.")
        .species("Rhachomyces", "girardii")
        .comb_authors(None, &["Santam."])
        .comb_ex_authors(&["Lepesme", "G.Temp."])
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
        .nothing_else();
    assert_name("Asperula galioides Bourg. ex Willk. & Lange, not validly publ.")
        .species("Asperula", "galioides")
        .comb_authors(None, &["Willk.", "Lange"])
        .comb_ex_authors(&["Bourg."])
        .nom_note("not validly publ.")
        .nothing_else();
    assert_name("Babiana pygmaea Spreng. ex Steud., not validly publ.")
        .species("Babiana", "pygmaea")
        .comb_authors(None, &["Steud."])
        .comb_ex_authors(&["Spreng."])
        .nom_note("not validly publ.")
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
        .nothing_else();
    assert_name("Aegopogon gracilis Cav. ex Roem. & Schult., pro syn.")
        .species("Aegopogon", "gracilis")
        .comb_authors(None, &["Roem.", "Schult."])
        .comb_ex_authors(&["Cav."])
        .nom_note("pro syn.")
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
        .infrageneric_at("Calathus", Rank::InfragenericName, "Lindrothius")
        .comb_authors(Some("1961"), &["Kurnakov"])
        .nothing_else();
    assert_name("Hymenophyllum subgen. Hymenoglossum (Presl) R.M.Tryon & A.Tryon")
        .infrageneric_at("Hymenophyllum", Rank::Subgenus, "Hymenoglossum")
        .comb_authors(None, &["R.M.Tryon", "A.Tryon"])
        .bas_authors(None, &["Presl"])
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
        .nothing_else();
    assert_name("Schwetzochromis torrenticola Thys van den Audenaerde, 1963")
        .species("Schwetzochromis", "torrenticola")
        .comb_authors(Some("1963"), &["Thys van den Audenaerde"])
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
        .warning(&[warnings::HOMOGLYHPS])
        .nothing_else();
    assert_name_auth("Abies alba", "（Spaeth）")
        .species("Abies", "alba")
        .bas_authors(None, &["Spaeth"])
        .warning(&[warnings::HOMOGLYHPS])
        .nothing_else();
    // the note inside the basionym bracket, before its year
    assert_name_auth("Ilema badia", "(Chao，comb．nov．,1986)")
        .species("Ilema", "badia")
        .bas_authors(Some("1986"), &["Chao"])
        .nom_note("comb. nov.")
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
        .nothing_else();
    assert_name("Hirundo leucopyga Meyen (Licht. MS) 1834")
        .species("Hirundo", "leucopyga")
        .comb_authors(Some("1834"), &["Meyen"])
        .comb_ex_authors(&["Licht."])
        .nothing_else();
    assert_name("Chromis Oken, 1817 (ex Cuvier)")
        .monomial("Chromis")
        .comb_authors(Some("1817"), &["Oken"])
        .comb_ex_authors(&["Cuvier"])
        .nothing_else();
}

#[test]
fn issue_150_in_litteris() {
    assert_name("Anterhynchium alecto lalepi (Cheesm.i.l.)")
        .infra_species("Anterhynchium", "alecto", Rank::InfraspecificName, "lalepi")
        .bas_authors(None, &["Cheesm."])
        .manuscript()
        .nom_note("i.l.")
        .nothing_else();
    assert_name("Litorina vilis (Menke in litt.)")
        .species("Litorina", "vilis")
        .bas_authors(None, &["Menke"])
        .manuscript()
        .nom_note("in litt.")
        .nothing_else();
    // published by the author before the bracket
    assert_name("Acanthocinus rusticus Klug (Dej. in litt.) 1829")
        .species("Acanthocinus", "rusticus")
        .comb_authors(Some("1829"), &["Klug"])
        .comb_ex_authors(&["Dej."])
        .nothing_else();
}

#[test]
fn issue_153_unknown_authors() {
    assert_authorship("(Mayer) Unknown", &[])
        .bas_authors(None, &["Mayer"])
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
    assert_authorship("(Mayer) Unknown Author", &[])
        .bas_authors(None, &["Mayer"])
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
    assert_authorship("(unknown) C. Agardh", &["C.Agardh"])
        .comb_authors(None, &["C.Agardh"])
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
    assert_authorship("(Cienkowski) Unknown", &[])
        .bas_authors(None, &["Cienkowski"])
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
    assert_authorship("(Z. Wang et al.) unknown", &[])
        .bas_authors(None, &["Z.Wang", "al."])
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
    // a placeholder of the name string's own: no placeholder taxon
    assert_name("Lepraria hueana (B. de Lesd.) unknown")
        .species("Lepraria", "hueana")
        .bas_authors(None, &["B.de Lesd."])
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
    assert_name("Turricaspia moquiana author(s) unknown")
        .species("Turricaspia", "moquiana")
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
    // an unknown cultivar stays a placeholder, anchored on its genus (#185)
    assert_placeholder("Saccharum hybrid cultivar (unknown)")
        .taxon("Saccharum")
        .taxon_rank(Rank::Unranked)
        .rank(Rank::Unranked)
        .phrase("hybrid cultivar (unknown)")
        .nothing_else();
}

#[test]
fn issue_142_emendation_in_the_basionym_bracket() {
    assert_name("Pachyptera kerere (Aubl. emend. Splitg.) Sandwith")
        .species("Pachyptera", "kerere")
        .comb_authors(None, &["Sandwith"])
        .bas_authors(None, &["Aubl."])
        .sensu("emend. Splitg.")
        .nothing_else();
    assert_name("Paulschulzia pseudovolvox (Schulz Em. Teil.) Skuja")
        .species("Paulschulzia", "pseudovolvox")
        .comb_authors(None, &["Skuja"])
        .bas_authors(None, &["Schulz"])
        .sensu("Em. Teil.")
        .nothing_else();
}

#[test]
fn issue_149_sine_ref_is_no_author() {
    assert_authorship(
        "(T.Anderson ex C.B.Clarke & sine ref.) Bremek.",
        &["Bremek."],
    )
    .comb_authors(None, &["Bremek."])
    .bas_authors(None, &["C.B.Clarke"])
    .bas_ex_authors(None, &["T.Anderson"])
    .nom_note("sine ref.")
    .nothing_else();
    assert_name("Ipomopsis bridgesii (A. Gray, sine ref.) Wherry")
        .species("Ipomopsis", "bridgesii")
        .comb_authors(None, &["Wherry"])
        .bas_authors(None, &["A.Gray"])
        .nom_note("sine ref.")
        .nothing_else();
    // "sine" is an epithet too
    assert_name("Tactusa sine Fibiger, 2010")
        .species("Tactusa", "sine")
        .comb_authors(Some("2010"), &["Fibiger"])
        .nothing_else();
}

#[test]
fn issue_145_sensu_lato_in_the_sense_of() {
    assert_name("Acacia acinacea s.l. sensu Maslin (1996)")
        .species("Acacia", "acinacea")
        .sensu("s.l. sensu Maslin (1996)")
        .nothing_else();
    assert_name("Xanthium orientale L. s. l. sensu Tomasello")
        .species("Xanthium", "orientale")
        .comb_authors(None, &["L."])
        .sensu("s. l. sensu Tomasello")
        .nothing_else();
}

#[test]
fn issue_136_pro_minore_parte() {
    // "pro minima parte", like "p.p." the concept is the name's only in part
    assert_name("Angelica p. min. p.")
        .monomial("Angelica")
        .doubtful()
        .nothing_else();
    assert_name("Aconitum hians Rchb. p. max. p.")
        .species("Aconitum", "hians")
        .comb_authors(None, &["Rchb."])
        .doubtful()
        .nothing_else();
}

#[test]
fn issue_151_a_lower_case_last_author() {
    assert_name("Callogorgia flabellum grandis Kükenthal & gorzawsky, 1908")
        .infra_species(
            "Callogorgia",
            "flabellum",
            Rank::InfraspecificName,
            "grandis",
        )
        .comb_authors(Some("1908"), &["Kükenthal", "gorzawsky"])
        .nothing_else();
    assert_name("Scytinelytra Amyot & serville 1843")
        .monomial("Scytinelytra")
        .comb_authors(Some("1843"), &["Amyot", "serville"])
        .nothing_else();
}

#[test]
fn issue_157_failed_names() {
    assert_informal("Ancistrus sp. \"INPA 43862\"")
        .taxon("Ancistrus")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. \"INPA 43862\"")
        .nothing_else();
    assert_informal("Microrhagus sp. 7")
        .taxon("Microrhagus")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. 7")
        .nothing_else();
    assert_name("Peckoltia cf. vittata (Steindachner, 1881)")
        .species("Peckoltia", "vittata")
        .qualifiers(&[(nameparser::model::NamePart::Specific, "cf.")])
        .type_(NameType::Informal)
        .bas_authors(Some("1881"), &["Steindachner"])
        .nothing_else();
    assert_name("Proechimys chrysaeolus (Thomas, 1898)*")
        .species("Proechimys", "chrysaeolus")
        .bas_authors(Some("1898"), &["Thomas"])
        .nothing_else();
    // a capitalised compound epithet
    assert_name("Ceraphron Sancti-Vincenti")
        .species("Ceraphron", "sancti-vincenti")
        .warning(&[warnings::CAPITALISED_EPITHET])
        .nothing_else();
    assert_name_auth("Plotus Novae-Hollandiae", "Gould, 1847")
        .species("Plotus", "novae-hollandiae")
        .comb_authors(Some("1847"), &["Gould"])
        .warning(&[warnings::CAPITALISED_EPITHET])
        .nothing_else();
}

#[test]
fn issue_143_rank_linea() {
    // until gbif/name-parser#107 adds the rank, an unranked infrageneric name
    assert_name("linea Parodiae Mottram")
        .infrageneric("Parodiae")
        .rank(Rank::InfragenericName)
        .comb_authors(None, &["Mottram"])
        .nothing_else();
    // after a genus "linea" is an epithet
    assert_name("Catasticta linea Brown 1939")
        .species("Catasticta", "linea")
        .comb_authors(Some("1939"), &["Brown"])
        .nothing_else();
}

#[test]
fn issue_155_trichosternus_spec_is_a_species() {
    // listed as a real name, a species without its author too
    assert_name("Trichosternus spec")
        .species("Trichosternus", "spec")
        .nothing_else();
    assert_name("Trichosternus spec Darlington, 1962")
        .species("Trichosternus", "spec")
        .comb_authors(Some("1962"), &["Darlington"])
        .nothing_else();
    // any other genus with a bare "spec" is an indeterminate species
    assert_informal("Abies spec")
        .taxon("Abies")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("spec")
        .nothing_else();
}

#[test]
fn issue_160_a_french_name_is_no_scientific_name() {
    assert_unparsable("Le dragon Bonnaterre, 1788", NameType::Other);
    assert_unparsable("Le Dragon Bonnaterre, 1788", NameType::Other);
    // "La" and "Des" are genera
    assert_name("La paloma Bleszynski, 1966")
        .species("La", "paloma")
        .comb_authors(Some("1966"), &["Bleszynski"])
        .nothing_else();
}

#[test]
fn issue_141_non_note_of_the_species() {
    // the note is the species', not the variety's: dropped and named in a warning
    assert_name("Thlaspi alpestre L. non Jacq. var. calaminare Lej.")
        .infra_species("Thlaspi", "alpestre", Rank::Variety, "calaminare")
        .specific_authors(None, &["L."])
        .comb_authors(None, &["Lej."])
        .warning(&["Removed: non Jacq."])
        .nothing_else();
    assert_name("Dirina repanda auct., non Fr. f. stenhammari (Stenh.) Clauzade & Cl. Roux")
        .infra_species("Dirina", "repanda", Rank::Form, "stenhammari")
        .comb_authors(None, &["Clauzade", "Cl.Roux"])
        .bas_authors(None, &["Stenh."])
        .warning(&["Removed: auct., non Fr."])
        .nothing_else();
    assert_name("Setaria ambigua (Guss.) Guss. non (Ten.) Mérat")
        .species("Setaria", "ambigua")
        .comb_authors(None, &["Guss."])
        .bas_authors(None, &["Guss."])
        .sensu("non (Ten.) Mérat")
        .nothing_else();
}

#[test]
fn issue_131_species_near_is_informal() {
    use nameparser::model::NamePart::{Generic, Specific};
    // "nr." (near) qualifies the epithet: an informal name, no infraspecific epithet "nr"
    assert_name("Camponotus vicinus nr. Mayr, 1870")
        .species("Camponotus", "vicinus")
        .qualifiers(&[(Specific, "nr.")])
        .type_(NameType::Informal)
        .comb_authors(Some("1870"), &["Mayr"])
        .nothing_else();
    assert_name("Hydraena puncticollis nr. Sharp 1882")
        .species("Hydraena", "puncticollis")
        .qualifiers(&[(Specific, "nr.")])
        .type_(NameType::Informal)
        .comb_authors(Some("1882"), &["Sharp"])
        .nothing_else();
    assert_name("Temnothorax andrei nr. (Emery, 1895)")
        .species("Temnothorax", "andrei")
        .qualifiers(&[(Specific, "nr.")])
        .type_(NameType::Informal)
        .bas_authors(Some("1895"), &["Emery"])
        .nothing_else();
    assert_name("Trouessartia funiculata nr.")
        .species("Trouessartia", "funiculata")
        .qualifiers(&[(Specific, "nr.")])
        .type_(NameType::Informal)
        .nothing_else();
    assert_name("Euglossa (Euglossa) deceptrix nr.")
        .species_ig("Euglossa", "Euglossa", "deceptrix")
        .qualifiers(&[(Specific, "nr.")])
        .type_(NameType::Informal)
        .nothing_else();
    assert_name("camponotus nr.fastigatus")
        .species("Camponotus", "fastigatus")
        .qualifiers(&[(Specific, "nr.")])
        .type_(NameType::Informal)
        .nothing_else();
    // "sp. nr.", "spec. nr.": a species near the one named
    assert_name("Phytocoris ceanothicus, sp. nr.")
        .species("Phytocoris", "ceanothicus")
        .qualifiers(&[(Specific, "nr.")])
        .type_(NameType::Informal)
        .nothing_else();
    assert_name("Dracaena goldieana spec.nr. Damen")
        .species("Dracaena", "goldieana")
        .qualifiers(&[(Specific, "nr.")])
        .type_(NameType::Informal)
        .comb_authors(None, &["Damen"])
        .nothing_else();
    assert_name("Bitoma parallela [sp.nr.]")
        .species("Bitoma", "parallela")
        .qualifiers(&[(Specific, "nr.")])
        .type_(NameType::Informal)
        .nothing_else();
    // in front of the genus, as "cf." is
    assert_name("nr. Orbiculoidea sampsoni")
        .species("Orbiculoidea", "sampsoni")
        .qualifiers(&[(Generic, "nr.")])
        .type_(NameType::Informal)
        .nothing_else();
    assert_informal("Leucanopsis sp.10 (nr. falacra)")
        .taxon("Leucanopsis")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp.10 (nr. falacra)")
        .nothing_else();
}

#[test]
fn issue_134_australian_phrase_names_are_informal() {
    // a genus and its phrase
    for (input, taxon, rank, phrase) in [
        (
            "Pultenaea sp. 'Olinda' (Coveny 6616)",
            "Pultenaea",
            Rank::Species,
            "sp. 'Olinda' (Coveny 6616)",
        ),
        (
            "Marsilea sp. Neutral Junction (D.E.Albrecht 9192)",
            "Marsilea",
            Rank::Species,
            "sp. Neutral Junction (D.E.Albrecht 9192)",
        ),
        (
            "Dampiera sp. Central Wheatbelt (L.W.Sage, F.Hort, C.A.Hollister LWS2321)",
            "Dampiera",
            Rank::Species,
            "sp. Central Wheatbelt (L.W.Sage, F.Hort, C.A.Hollister LWS2321)",
        ),
        // the phrase of a rank other than species leaves out its marker, which the rank gives
        (
            "Baeckea ssp. 2 (LJM 2019)",
            "Baeckea",
            Rank::Subspecies,
            "2 (LJM 2019)",
        ),
        (
            "Baeckea var 2 (LJM 2019)",
            "Baeckea",
            Rank::Variety,
            "2 (LJM 2019)",
        ),
        (
            "Baeckea sp. Bunney Road (S.Patrick 4059)",
            "Baeckea",
            Rank::Species,
            "sp. Bunney Road (S.Patrick 4059)",
        ),
        (
            "Prostanthera sp. Bundjalung Nat. Pk. (B.J.Conn 3471)",
            "Prostanthera",
            Rank::Species,
            "sp. Bundjalung Nat. Pk. (B.J.Conn 3471)",
        ),
        (
            "Toechima sp. East Alligator (J.Russell-Smith 8418) NT Herbarium",
            "Toechima",
            Rank::Species,
            "sp. East Alligator (J.Russell-Smith 8418) NT Herbarium",
        ),
        (
            "Goodenia sp. Bachsten Creek (M.D. Barrett 685) WA Herbarium",
            "Goodenia",
            Rank::Species,
            "sp. Bachsten Creek (M.D. Barrett 685) WA Herbarium",
        ),
        (
            "Baeckea sp. Beringbooding (AR Main 11/9/1957)",
            "Baeckea",
            Rank::Species,
            "sp. Beringbooding (AR Main 11/9/1957)",
        ),
        (
            "Sida sp. Walhallow Station (C.Edgood 28/Oct/94)",
            "Sida",
            Rank::Species,
            "sp. Walhallow Station (C.Edgood 28/Oct/94)",
        ),
        (
            "Sida sp. B (C.Dunlop 1739)",
            "Sida",
            Rank::Species,
            "sp. B (C.Dunlop 1739)",
        ),
        (
            "Baeckea sp. Calingiri (F.Hort 1710)",
            "Baeckea",
            Rank::Species,
            "sp. Calingiri (F.Hort 1710)",
        ),
        (
            "Baeckea sp. East Yuna (R Spjut & C Edson 7077)",
            "Baeckea",
            Rank::Species,
            "sp. East Yuna (R Spjut & C Edson 7077)",
        ),
        (
            "Acacia sp. Goodlands (BR Maslin 7761) [aff. resinosa]",
            "Acacia",
            Rank::Species,
            "sp. Goodlands (BR Maslin 7761) [aff. resinosa]",
        ),
        (
            "Acacia sp. Manmanning (BR Maslin 7711) [aff. multispicata]",
            "Acacia",
            Rank::Species,
            "sp. Manmanning (BR Maslin 7711) [aff. multispicata]",
        ),
        (
            "Acacia sp. \"Morning Glory\"",
            "Acacia",
            Rank::Species,
            "sp. \"Morning Glory\"",
        ),
    ] {
        assert_informal(input)
            .taxon(taxon)
            .taxon_rank(Rank::Genus)
            .rank(rank)
            .phrase(phrase)
            .nothing_else();
    }
    // a subgenus and its phrase
    assert_informal("Atrichornis (Rahcinta) sp Glory (BR Maslin 7711)")
        .taxon("Atrichornis subg. Rahcinta")
        .taxon_rank(Rank::Subgenus)
        .rank(Rank::Species)
        .phrase("Glory (BR Maslin 7711)")
        .nothing_else();
    // a species and its phrase
    assert_informal("Grevillea brachystylis subsp. Busselton (G.J.Keighery s.n. 28/8/1985)")
        .taxon("Grevillea brachystylis")
        .taxon_rank(Rank::Species)
        .rank(Rank::Subspecies)
        .phrase("Busselton (G.J.Keighery s.n. 28/8/1985)")
        .nothing_else();
    assert_informal("Acacia mutabilis subsp. Young River (G.F.Craig 2052)")
        .taxon("Acacia mutabilis")
        .taxon_rank(Rank::Species)
        .rank(Rank::Subspecies)
        .phrase("Young River (G.F.Craig 2052)")
        .nothing_else();
    // the species author is the anchor's
    let pn = nameparser::parse_name(
        "Acacia mutabilis Maslin subsp. Young River (G.F.Craig 2052)",
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        pn.anchor().unwrap().canonical_name().as_deref(),
        Some("Acacia mutabilis Maslin")
    );
    assert_name("Acacia mutabilis Maslin subsp. Young River (G.F.Craig 2052)")
        .binomial("Acacia", None, "mutabilis", Rank::Subspecies)
        .specific_authors(None, &["Maslin"])
        .phrase("Young River (G.F.Craig 2052)")
        .type_(NameType::Informal)
        .nothing_else();
}

#[test]
fn issue_123_a_shouted_common_name_is_no_author() {
    // a common name in capitals and brackets on a taxon above the genus is removed, with a warning:
    // it read as basionym authors, or as a subgenus
    let clitellata = "(OLIGOCHAETE, LEECH AND BRANCHIOBDELLID)";
    for (name, authorship, uninomial, rank, removed) in [
        (
            "Clitellata",
            Some(clitellata),
            "Clitellata",
            Rank::Class,
            clitellata,
        ),
        (
            "Clitellata (OLIGOCHAETE, LEECH AND BRANCHIOBDELLID)",
            None,
            "Clitellata",
            Rank::Class,
            clitellata,
        ),
        (
            "Dermestidae",
            Some("(CARPET BEETLES)"),
            "Dermestidae",
            Rank::Family,
            "(CARPET BEETLES)",
        ),
        (
            "Thysanoptera",
            Some("(THRIPS)"),
            "Thysanoptera",
            Rank::Order,
            "(THRIPS)",
        ),
    ] {
        assert_name_hinted(name, authorship, Some(rank), Some(NomCode::Zoological))
            .monomial_rank(uninomial, rank)
            .code(NomCode::Zoological)
            .warning(&[&format!("{}{removed}", warnings::REMOVED_PREFIX)])
            .nothing_else();
    }
    // without a rank, a family-group ending tells
    assert_name("Formicidae (ANTS)")
        .monomial("Formicidae")
        .warning(&["Removed: (ANTS)"])
        .nothing_else();
    // on a genus a shouted bracket is a subgenus
    assert_name("Tetramorium (XIPHOMYRMEX)")
        .infrageneric_at("Tetramorium", Rank::InfragenericName, "XIPHOMYRMEX")
        .nothing_else();
}

#[test]
fn issue_125_author_teams_with_initials() {
    // a team of authors with their initials first that lost its commas
    assert_name("Eucalyptus roycei S.G.M. Carr D.J. Carr & A.S. George")
        .species("Eucalyptus", "roycei")
        .comb_authors(None, &["S.G.M.Carr", "D.J.Carr", "A.S.George"])
        .nothing_else();
    assert_name("Lactarius lanceolatus O.K. Mill G. Laursen")
        .species("Lactarius", "lanceolatus")
        .comb_authors(None, &["O.K.Mill", "G.Laursen"])
        .nothing_else();
    assert_name("Puccinia jaceae G.H. Otth G.H. Otth")
        .species("Puccinia", "jaceae")
        .comb_authors(None, &["G.H.Otth", "G.H.Otth"])
        .nothing_else();
    // a particle between the initials, or a forename written out, keeps one author
    assert_name("Purpura lapillus (J. De C. Sowerby, 1823)")
        .species("Purpura", "lapillus")
        .bas_authors(Some("1823"), &["J.De C.Sowerby"])
        .nothing_else();
    assert_name("Carex nigra Roy L. Taylor")
        .species("Carex", "nigra")
        .comb_authors(None, &["Roy L.Taylor"])
        .nothing_else();
    // the other names of the issue
    assert_name("Uvella rosacea J.B.G.G.M. Bory")
        .species("Uvella", "rosacea")
        .comb_authors(None, &["J.B.G.G.M.Bory"])
        .nothing_else();
    assert_name("Bryocyclops campaneri Rocha C.E.F. & Bjornberg M.H.G.C., 1987")
        .species("Bryocyclops", "campaneri")
        .comb_authors(Some("1987"), &["C.E.F.Rocha", "M.H.G.C.Bjornberg"])
        .nothing_else();
    assert_name("Fercurtia Gougerot & le Renard, 1978")
        .monomial("Fercurtia")
        .comb_authors(Some("1978"), &["Gougerot", "le Renard"])
        .nothing_else();
    assert_name("Melaleucus sellata Chevrolat L.A.A, 1882")
        .species("Melaleucus", "sellata")
        .comb_authors(Some("1882"), &["L.A.A.Chevrolat"])
        .nothing_else();
    assert_name("Rotylenchus ascalpi Scotto la Massese & Germani, 2001")
        .species("Rotylenchus", "ascalpi")
        .comb_authors(Some("2001"), &["Scotto la Massese", "Germani"])
        .nothing_else();
    assert_name("Bacillus kaustophilus (ex Prickett, 1928) Priest et al., 1989")
        .species("Bacillus", "kaustophilus")
        .comb_authors(Some("1989"), &["Priest", "al."])
        .comb_ex_authors(&["Prickett"])
        .code(NomCode::Bacterial)
        .nothing_else();
    assert_name("Argulus multicolor Schuurmans Stekhoven J.H. Jr, 1937")
        .species("Argulus", "multicolor")
        .comb_authors(Some("1937"), &["J.H.Schuurmans Stekhoven Jr"])
        .nothing_else();
    assert_name("Cerithium papillosum Sowerby G.B. II, 1855")
        .species("Cerithium", "papillosum")
        .comb_authors(Some("1855"), &["G.B.Sowerby II"])
        .nothing_else();
}

#[test]
fn issue_125_in_before_ex() {
    // "A in B ex C": C took up the name of A; the work cited between is removed with a warning
    assert_name("Calamagrostis heterophylla Wedd. in Urb. ex Pilg.")
        .species("Calamagrostis", "heterophylla")
        .comb_authors(None, &["Pilg."])
        .comb_ex_authors(&["Wedd."])
        .warning(&["Removed: in Urb."])
        .nothing_else();
    assert_name("Aristolochia abbreviata Mart. in DC. ex Duch.")
        .species("Aristolochia", "abbreviata")
        .comb_authors(None, &["Duch."])
        .comb_ex_authors(&["Mart."])
        .warning(&["Removed: in DC."])
        .nothing_else();
    assert_name("Pohlia ampullacea Hampe in C. Müller ex Gangulee, 1974")
        .species("Pohlia", "ampullacea")
        .comb_authors(Some("1974"), &["Gangulee"])
        .comb_ex_authors(&["Hampe"])
        .warning(&["Removed: in C. Müller"])
        .nothing_else();
    assert_name("Asplenium tenuifrons var. tenellum Wall. apud T. Moore ex R. Sim")
        .infra_species("Asplenium", "tenuifrons", Rank::Variety, "tenellum")
        .comb_authors(None, &["R.Sim"])
        .comb_ex_authors(&["Wall."])
        .warning(&["Removed: apud T. Moore"])
        .nothing_else();
    assert_name("Steccherinum ochraceum (Pers. in Gmelin ex Fr.) S.F. Gray")
        .species("Steccherinum", "ochraceum")
        .comb_authors(None, &["S.F.Gray"])
        .bas_authors(None, &["Fr."])
        .bas_ex_authors(None, &["Pers."])
        .warning(&["Removed: in Gmelin"])
        .nothing_else();
    // a citation with its year names the work of the name
    assert_name("Cheilinus radiatus Valenciennes in Cuvier & Valenciennes, 1840 ex Ehrenberg")
        .species("Cheilinus", "radiatus")
        .comb_authors(None, &["Valenciennes"])
        .published_in("Cuvier & Valenciennes, 1840 ex Ehrenberg");
}

#[test]
fn issue_125_an_ex_author_in_lower_case() {
    // the issue's "Hieracium multicaule sp. schaffner ex Sch.Bip." is written without the "sp."
    assert_name("Hieracium multicaule schaffner ex Sch.Bip.")
        .species("Hieracium", "multicaule")
        .comb_authors(None, &["Sch.Bip."])
        .comb_ex_authors(&["schaffner"])
        .nothing_else();
    assert_name("Tamarix indica koen. ex Roxb.")
        .species("Tamarix", "indica")
        .comb_authors(None, &["Roxb."])
        .comb_ex_authors(&["koen."])
        .nothing_else();
    assert_name("Viguiera pinnatilobata var. megaphylla butterwick ex B.L.Turner")
        .infra_species("Viguiera", "pinnatilobata", Rank::Variety, "megaphylla")
        .comb_authors(None, &["B.L.Turner"])
        .comb_ex_authors(&["butterwick"])
        .nothing_else();
}

#[test]
fn issue_167_made_up_ranks_are_unranked_infrageneric_names() {
    // Ranks made up under ICN Art. 4.3, cited in IPNI, have no rank of their own yet; cyclus waits
    // for gbif/name-parser#108
    assert_name("Malus nothogrex Prunifoliae (Juz.) Langenf.")
        .infrageneric_at("Malus", Rank::InfragenericName, "Prunifoliae")
        .notho(&[NamePart::Infrageneric])
        .comb_authors(None, &["Langenf."])
        .bas_authors(None, &["Juz."])
        .nothing_else();
    assert_name("Veronica Gruppe Pellidosperma E.B.J.Lehm.")
        .infrageneric_at("Veronica", Rank::InfragenericName, "Pellidosperma")
        .comb_authors(None, &["E.B.J.Lehm."])
        .nothing_else();
    assert_name("Lonicera gruppe Americanae")
        .infrageneric_at("Lonicera", Rank::InfragenericName, "Americanae")
        .nothing_else();
    assert_name("Abies genitor Setistrobilus A.E.Murray")
        .infrageneric_at("Abies", Rank::InfragenericName, "Setistrobilus")
        .comb_authors(None, &["A.E.Murray"])
        .nothing_else();
    assert_name("Abies subgenitor Setistrobilus A.E.Murray")
        .infrageneric_at("Abies", Rank::InfragenericName, "Setistrobilus")
        .comb_authors(None, &["A.E.Murray"])
        .nothing_else();
    assert_name("Abies agglom. Sibiria A.E.Murray")
        .infrageneric_at("Abies", Rank::InfragenericName, "Sibiria")
        .comb_authors(None, &["A.E.Murray"])
        .nothing_else();
    assert_name("Artemisia cycl. Junceum (Poljakov) Filatova")
        .infrageneric_at("Artemisia", Rank::InfragenericName, "Junceum")
        .comb_authors(None, &["Filatova"])
        .bas_authors(None, &["Poljakov"])
        .nothing_else();
    assert_name("Philodendron grex Baursia Rchb. ex Schott")
        .infrageneric_at("Philodendron", Rank::InfragenericName, "Baursia")
        .comb_authors(None, &["Schott"])
        .comb_ex_authors(&["Rchb."])
        .nothing_else();
    assert_name("Cerasus Grex Typocerasus Koehne")
        .infrageneric_at("Cerasus", Rank::InfragenericName, "Typocerasus")
        .comb_authors(None, &["Koehne"])
        .nothing_else();
    // the bare names of the issue
    for (name, genus, epithet) in [
        ("Abies genitor Setistrobilus", "Abies", "Setistrobilus"),
        ("Abies subgenitor Setistrobilus", "Abies", "Setistrobilus"),
        ("Abies agglom. Sibiria", "Abies", "Sibiria"),
        ("Artemisia cycl. Junceum", "Artemisia", "Junceum"),
        ("Veronica Gruppe Pellidosperma", "Veronica", "Pellidosperma"),
    ] {
        assert_name(name)
            .infrageneric_at(genus, Rank::InfragenericName, epithet)
            .nothing_else();
    }
    assert_name("Malus nothogrex Prunifoliae")
        .infrageneric_at("Malus", Rank::InfragenericName, "Prunifoliae")
        .notho(&[NamePart::Infrageneric])
        .nothing_else();
    // Mottram's linea, an unranked infrageneric name until gbif/name-parser#107 (#143)
    assert_name("linea Rebutiae")
        .infrageneric("Rebutiae")
        .rank(Rank::InfragenericName)
        .nothing_else();

    // the same words are species epithets before an author with a year, a team, initials or none
    assert_name("Pheidole grex Wilson, 2003")
        .species("Pheidole", "grex")
        .comb_authors(Some("2003"), &["Wilson"])
        .nothing_else();
    assert_name("Stigmidium grex Alstrup & Olech")
        .species("Stigmidium", "grex")
        .comb_authors(None, &["Alstrup", "Olech"])
        .nothing_else();
    assert_name("Diospyros grex F. White")
        .species("Diospyros", "grex")
        .comb_authors(None, &["F.White"])
        .nothing_else();
    assert_name("Mesoleptus genitor (Forster, 1876)")
        .species("Mesoleptus", "genitor")
        .bas_authors(Some("1876"), &["Forster"])
        .nothing_else();
    assert_name("Vargula grex Kornicker in Kornicker & Thomassin, 1998")
        .species("Vargula", "grex")
        .comb_authors(Some("1998"), &["Kornicker"])
        .comb_in_authors(&["Kornicker", "Thomassin"])
        .published_in("Kornicker & Thomassin, 1998")
        .published_in_year(Some(1998))
        .nothing_else();
    // …and Gruppe an author
    assert_name("Monedula punctata decorata Gruppe Taschenberg, 1870").infra_species(
        "Monedula",
        "punctata",
        Rank::InfraspecificName,
        "decorata",
    );
}

#[test]
fn issue_167_a_notho_subtribe() {
    assert_name("nothosubtrib. Pachynyctocereinae P.V.Heath")
        .monomial_rank("Pachynyctocereinae", Rank::Subtribe)
        .notho(&[NamePart::Generic])
        .comb_authors(None, &["P.V.Heath"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("nothosubtrib. Pachynyctocereinae")
        .monomial_rank("Pachynyctocereinae", Rank::Subtribe)
        .notho(&[NamePart::Generic])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn issue_167_an_epithet_of_words_apart_is_hyphenated() {
    // ICN Art. 23.1, 60.11; IPNI cites it as "var. ville de gand", author Hort
    assert_name("Anthurium scherzerianum var. ville de gand Hort")
        .infra_species("Anthurium", "scherzerianum", Rank::Variety, "ville-de-gand")
        .comb_authors(None, &["Hort"])
        .nothing_else();
    assert_name("Anthurium scherzerianum var. ville de gand")
        .infra_species("Anthurium", "scherzerianum", Rank::Variety, "ville-de-gand")
        .nothing_else();
}

#[test]
fn a_subgenus_cited_in_its_brackets() {
    // the Prokaryotic Code's Rule 10c: the subgenus author goes inside the brackets. There is no
    // place for it in a species' name, so it is dropped with a warning.
    assert_name(
        "Acetobacter (subgen. Gluconoacetobacter Yamada and Kondo 1985) liquefaciens (Asai 1935) Gosselé et al. 1983",
    )
    .species_ig("Acetobacter", "Gluconoacetobacter", "liquefaciens")
    .bas_authors(Some("1935"), &["Asai"])
    .comb_authors(Some("1983"), &["Gosselé", "al."])
    .warning(&["Removed: Yamada and Kondo 1985"])
    .nothing_else();
    // on its own it is the authorship
    assert_name("Acetobacter (subgen. Gluconoacetobacter Yamada and Kondo 1985)")
        .infrageneric_at("Acetobacter", Rank::Subgenus, "Gluconoacetobacter")
        .comb_authors(Some("1985"), &["Yamada", "Kondo"])
        .nothing_else();
}

#[test]
fn a_zoological_division_within_a_genus_is_unranked() {
    // Dejean's and Mulsant's "div." is no botanical divisio
    assert_name_code("Feronia (div. Percus) Dejean, 1828", NomCode::Zoological)
        .infrageneric_at("Feronia", Rank::InfragenericName, "Percus")
        .comb_authors(Some("1828"), &["Dejean"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Feronia (div. Percus) Dejean, 1828")
        .infrageneric_at("Feronia", Rank::InfragenericName, "Percus")
        .comb_authors(Some("1828"), &["Dejean"])
        .nothing_else();
    assert_name("Rosa div. Caninae")
        .infrageneric_at("Rosa", Rank::DivisionBotany, "Caninae")
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn issue_171_zoological_sections() {
    // ZooBank's sections within a genus are unranked infrageneric names, not botanical sections
    for (name, epithet) in [
        ("Balanus (sect. Ortho-balanus) Hoek, 1913", "Ortho-balanus"),
        (
            "Balanus (sect. Membrano-balanus) Hoek, 1913",
            "Membrano-balanus",
        ),
        (
            "Balanus (sect. Patello-balanus) Hoek, 1913",
            "Patello-balanus",
        ),
    ] {
        assert_name(name)
            .infrageneric_at("Balanus", Rank::InfragenericName, epithet)
            .comb_authors(Some("1913"), &["Hoek"])
            .nothing_else();
    }
    assert_name_code(
        "Achatinella (sect. Auriculella) Pfeiffer 1854",
        NomCode::Zoological,
    )
    .infrageneric_at("Achatinella", Rank::InfragenericName, "Auriculella")
    .comb_authors(Some("1854"), &["Pfeiffer"])
    .code(NomCode::Zoological)
    .nothing_else();
    // a marker written out is the zoologists' too
    assert_name("Aphodius (section Cytoderhinus) Seabra 1909")
        .infrageneric_at("Aphodius", Rank::InfragenericName, "Cytoderhinus")
        .comb_authors(Some("1909"), &["Seabra"])
        .nothing_else();
    assert_name("Balanus (section Ortho-Balanus) minutus")
        .species_ig("Balanus", "Ortho-Balanus", "minutus")
        .nothing_else();
    assert_name("Hypanis (section Adacna)")
        .infrageneric_at("Hypanis", Rank::InfragenericName, "Adacna")
        .nothing_else();
    assert_name("Balanus (sect. Membrano-balanus) longirostrum Hoek, 1913")
        .species_ig("Balanus", "Membrano-balanus", "longirostrum")
        .comb_authors(Some("1913"), &["Hoek"])
        .nothing_else();
    // the subgenera of the issue
    assert_name("Balanus (Bathybalanus) pentacrini Hoek, 1913")
        .species_ig("Balanus", "Bathybalanus", "pentacrini")
        .comb_authors(Some("1913"), &["Hoek"])
        .nothing_else();
    assert_name("Balanus (megabalanus) tintinnabulum costatus Hoek, 1913")
        .infra_species(
            "Balanus",
            "tintinnabulum",
            Rank::InfraspecificName,
            "costatus",
        )
        .infrageneric("Megabalanus")
        .comb_authors(Some("1913"), &["Hoek"])
        .doubtful()
        .nothing_else();
    for (name, epithet) in [
        ("Balanus (Armatobalanus) Hoek, 1913", "Armatobalanus"),
        ("Balanus (Megabalanus) Hoek, 1913", "Megabalanus"),
    ] {
        assert_name(name)
            .infrageneric_at("Balanus", Rank::InfragenericName, epithet)
            .comb_authors(Some("1913"), &["Hoek"])
            .nothing_else();
    }
    // a botanist's section keeps its rank, with or without a year, a comma or brackets
    assert_name("Tradescantia (sect. Zebrina) Pellegrini 2017")
        .infrageneric_at("Tradescantia", Rank::SectionBotany, "Zebrina")
        .comb_authors(Some("2017"), &["Pellegrini"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Cephalozia sect. Prionolobus", "Spruce, 1882")
        .infrageneric_at("Cephalozia", Rank::SectionBotany, "Prionolobus")
        .comb_authors(Some("1882"), &["Spruce"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Guatteria (sect. Trichostemon) R.E.Fr.")
        .infrageneric_at("Guatteria", Rank::SectionBotany, "Trichostemon")
        .comb_authors(None, &["R.E.Fr."])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn issue_124_vectors_and_plasmids_are_no_viruses() {
    // NCBI's cloning vectors and plasmids are no organisms, nor viruses
    for name in [
        "H-Pelican lacZ transformation vector",
        "H-Stinger GFP transformation vector",
        "Cre expression vector",
        "20S RNA replicon",
        "Plasmid pACYC184",
        "Accepter plasmid pBac3xP3DsRed2attP",
        "Agrobacterium vitis plasmid pTiAB4",
        "Gateway destination plasmid",
    ] {
        assert_unparsable_no_code(name, NameType::Other);
    }
    // a viral word keeps the code
    assert_unparsable_code(
        "Baculovirus expression vector pLM",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code("Gyrovirus GyV3", NameType::Other, NomCode::Virus);
    // a plasmid in brackets is a note on its host
    assert_name("Agrobacterium rhizogenes (RI plasmid PRI1724)")
        .species("Agrobacterium", "rhizogenes")
        .bas_authors(None, &["RI plasmid Pri1724"]);
    // the epithet vector with a dated author
    assert_name("Desmoxytes vector (Chamberlin, 1941)")
        .species("Desmoxytes", "vector")
        .bas_authors(Some("1941"), &["Chamberlin"])
        .nothing_else();
    // alone the trinomial is shaped as `Cre expression vector`
    assert_name_auth(
        "Neoaemula vector vector",
        "Mackinnon, Hiller, Long & Marshall, 2008",
    )
    .infra_species("Neoaemula", "vector", Rank::InfraspecificName, "vector")
    .comb_authors(Some("2008"), &["Mackinnon", "Hiller", "Long", "Marshall"])
    .nothing_else();
}
