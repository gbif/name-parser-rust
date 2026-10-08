// SPDX-License-Identifier: Apache-2.0
//! What an authorship carries besides its authors — manuscript markers, emendations, references,
//! notes, a lone particle — kept out of the author list. Frequencies are from ChecklistBank's
//! verbatim authorship column.

mod common;
use common::*;
use nameparser::model::{warnings, NomCode, Rank};

#[test]
fn in_litteris_is_an_unpublished_name() {
    // 600 CLB rows: "in litt." / "i.l." (in a letter), unpublished like "ms."
    assert_name_auth("Arachnospila osmana", "Wolf, in litt.")
        .species("Arachnospila", "osmana")
        .comb_authors(None, &["Wolf"])
        .nom_note("in litt.")
        .manuscript()
        .nothing_else();
    assert_name_auth("Aus bus", "Blüthgen i.l.")
        .species("Aus", "bus")
        .comb_authors(None, &["Blüthgen"])
        .nom_note("i.l.")
        .manuscript()
        .nothing_else();
}

#[test]
fn em_between_two_authors_is_an_emendation() {
    // 557 CLB rows, mostly foraminifera: "Sigal Em. Moullade" = Sigal emend. Moullade
    assert_name_auth("Ticinella bejaouaensis", "Sigal Em. Moullade, 1966")
        .species("Ticinella", "bejaouaensis")
        .comb_authors(None, &["Sigal"])
        // kept verbatim, as "emend" without its dot is
        .sensu("Em. Moullade, 1966")
        .nothing_else();
    // alone it is an author's initials: Emil Schmid
    assert_name_auth("Braya trinkleri", "Em. Schmid")
        .species("Braya", "trinkleri")
        .comb_authors(None, &["Em.Schmid"])
        .nothing_else();
}

#[test]
fn a_reference_with_volume_and_page_is_no_author() {
    // 362 CLB rows: IPNI-style "Gen. Pl. 1: 563. 1865." after the authors
    assert_name_auth("Batesia", "Spruce ex Benth., Gen. Pl. 1: 563. 1865.")
        .monomial("Batesia")
        .comb_authors(Some("1865"), &["Benth."])
        .comb_ex_authors(&["Spruce"])
        .published_in("Gen. Pl. 1: 563. 1865")
        .published_in_year(Some(1865))
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn an_approved_lists_citation_with_a_comma_is_bacterial_too() {
    // 298 CLB rows
    assert_name_auth(
        "Enterobacter",
        "Hormaeche & Edwards, 1960 (Approved Lists, 1980)",
    )
    .monomial("Enterobacter")
    .comb_authors(Some("1960"), &["Hormaeche", "Edwards"])
    .code(NomCode::Bacterial)
    .nothing_else();
}

#[test]
fn an_of_citation_is_the_taxonomic_note() {
    // 201 CLB rows: the concept of other authors, as WoRMS writes it
    assert_name_auth("Trochurus speciosus", "of Hawle & Corda 1847")
        .species("Trochurus", "speciosus")
        .sensu("of Hawle & Corda 1847")
        .nothing_else();
}

#[test]
fn nomen_nudum_in_brackets_is_the_nomenclatural_note() {
    // 156 CLB rows
    assert_name_auth("Chrysopelea erythrochloris", "SCHLEGEL 1826 (nomen nudum)")
        .species("Chrysopelea", "erythrochloris")
        .comb_authors(Some("1826"), &["Schlegel"])
        .nom_note("nomen nudum")
        .code(NomCode::Zoological)
        .nothing_else();
    // spelled out, "illegitimum" is zoologists' usage (botanists write "nom. illeg."): no botanical
    // vote against the year's zoological one
    assert_name_auth("Testudo macropus", "Walbaun, 1782 (nomen illegitimum)")
        .species("Testudo", "macropus")
        .comb_authors(Some("1782"), &["Walbaun"])
        .nom_note("nomen illegitimum")
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn a_manuscript_marker_before_the_year() {
    // 49 CLB rows: "Schwager ms., 1866"
    assert_name_auth("Textularia trigeri", "Schwager ms., 1866")
        .species("Textularia", "trigeri")
        .comb_authors(Some("1866"), &["Schwager"])
        .nom_note("ms.")
        .manuscript()
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn a_lone_particle_is_no_authorship() {
    // 26 CLB rows: "de" alone in the authorship column
    assert_name_auth("Platosphus gervais", "de")
        .species("Platosphus", "gervais")
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
}

#[test]
fn vide_is_a_secondary_reference() {
    // "see …": a note on where the name was found, not its author or its publication (also
    // gna_04's `Porina reussi`)
    assert_name_auth(
        "Chondrosoma loeve",
        "Örsted (vide Koren & Danielssen, 1876)",
    )
    .species("Chondrosoma", "loeve")
    .comb_authors(None, &["Örsted"])
    .sensu("vide Koren & Danielssen, 1876")
    .nothing_else();
}

#[test]
fn a_synonym_remark_is_the_taxonomic_note() {
    // the name string's twin is gna_01's `Döringina Ihering 1929 (synonym)`
    assert_name_auth("Brandisia", "Brandis (synonym)")
        .monomial("Brandisia")
        .comb_authors(None, &["Brandis"])
        .sensu("synonym")
        .nothing_else();
}

#[test]
fn an_of_note_needs_authors_outside_any_bracket_or_corporate_name() {
    // a corporate author: the "of" is part of its name
    assert_name_auth("Elimaea grandis", "Research Group of Orthoptera, 1983")
        .species("Elimaea", "grandis")
        .comb_authors(Some("1983"), &["Research Group of Orthoptera"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_auth(
        "Rana maculosa chayuensis",
        "Ye in Sichuan Institute of Biology Herpetology Department, 1977",
    )
    .infra_species("Rana", "maculosa", Rank::InfraspecificName, "chayuensis")
    .comb_authors(Some("1977"), &["Ye"])
    .published_in("Sichuan Institute of Biology Herpetology Department, 1977")
    .published_in_year(Some(1977))
    .nothing_else();
    // inside the bracket the note would take the basionym author with it: left as it was
    // FIXME(review): a note inside the basionym bracket is still read as its author
    assert_name_auth("Thelastoma bulhoesi", "(Mag of Dollfus 1952)")
        .species("Thelastoma", "bulhoesi")
        .bas_authors(Some("1952"), &["Mag of Dollfus"])
        .code(NomCode::Zoological)
        .nothing_else();
    // an emendation after the basionym's year is the note, not a second author
    assert_name_auth(
        "Praeskinnerella tamanouchiensis",
        "(Sakagami, 1956 Em. Chisaka, 1960)",
    )
    .species("Praeskinnerella", "tamanouchiensis")
    .bas_authors(Some("1956"), &["Sakagami"])
    .sensu("Em. Chisaka, 1960")
    .code(NomCode::Zoological)
    .nothing_else();
    // after the basionym it is the note
    assert_name_auth(
        "Pseudohaliotrema platicephali",
        "(Yin & Sproston, 1948) of Young (1968)",
    )
    .species("Pseudohaliotrema", "platicephali")
    .bas_authors(Some("1948"), &["Yin", "Sproston"])
    .sensu("of Young (1968)")
    .code(NomCode::Zoological)
    .nothing_else();
}

#[test]
fn a_page_reference_starts_after_the_last_author() {
    // the commas inside the basionym bracket are the authors'
    assert_name_auth(
        "Rhaphiolepis daduheensis",
        "(H. Z. Zhang ex W. B. Liao, Q. Fan & M. Y. Ding) B. B. Liu & J. Wen, Front. Plant Sci. 10 - 1731: 10. 2020",
    )
    .species("Rhaphiolepis", "daduheensis")
        .comb_authors(Some("2020"), &["B.B.Liu", "J.Wen"])
        .bas_authors(None, &["W.B.Liao", "Q.Fan", "M.Y.Ding"])
        .bas_ex_authors(None, &["H.Z.Zhang"])
        .published_in("Front. Plant Sci. 10 - 1731: 10. 2020")
        .published_in_year(Some(2020))
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth(
        "Hevansia ovalongata",
        "Luangsa-ard, Hywel-Jones, and Spatafora, IMA Fungus 8: 349 (2017). 2017",
    )
    .species("Hevansia", "ovalongata")
    .comb_authors(Some("2017"), &["Luangsa-ard", "Hywel-Jones", "Spatafora"])
    .published_in("IMA Fungus 8: 349 (2017). 2017")
    .published_in_year(Some(2017))
    .nothing_else();
    assert_name_auth("Herminium josephi", "Rchb. f., Flora 55: 276. 1872.")
        .species("Herminium", "josephi")
        .comb_authors(Some("1872"), &["Rchb.f."])
        .published_in("Flora 55: 276. 1872")
        .published_in_year(Some(1872))
        .code(NomCode::Botanical)
        .nothing_else();
    // the title's own "&"
    assert_name_auth(
        "Aus bus",
        "Kerr, Trans. & Proc. Bot. Soc. Edinburgh 30: 12. 1930",
    )
    .species("Aus", "bus")
    .comb_authors(Some("1930"), &["Kerr"])
    .published_in("Trans. & Proc. Bot. Soc. Edinburgh 30: 12. 1930")
    .published_in_year(Some(1930))
    .nothing_else();
}

#[test]
fn an_approved_lists_citation_before_an_emendation() {
    // 301 CLB rows
    assert_name_auth(
        "Actinosporangium",
        "Lechevalier & Lechevalier 1970 (Approved Lists 1980) emend. Nouioui et al. 2018",
    )
    .monomial("Actinosporangium")
    .comb_authors(Some("1970"), &["Lechevalier", "Lechevalier"])
    .sensu("emend. Nouioui et al. 2018")
    .code(NomCode::Bacterial)
    .nothing_else();
}

#[test]
fn a_manuscript_marker_before_the_year_in_a_letter() {
    // a diatom, coded by the caller
    assert_name_hinted(
        "Navicula vidovichii",
        Some("Grunow in litteris, 1863"),
        Some(Rank::Species),
        Some(NomCode::Botanical),
    )
    .species("Navicula", "vidovichii")
    .comb_authors(Some("1863"), &["Grunow"])
    .nom_note("in litteris")
    .manuscript()
    .code(NomCode::Botanical)
    .nothing_else();
}

#[test]
fn a_stray_lower_case_word_ends_the_authorship() {
    // an epithet written without its rank marker, after the species author
    assert_name("Loranthus incanus Schumach. & Thonn. sessilis Sprague")
        .species("Loranthus", "incanus")
        .comb_authors(None, &["Schumach.", "Thonn."])
        .partial("sessilis Sprague")
        .nothing_else();
    assert_name("Polypodium pectinatum (L. f.) typica Rosent")
        .species("Polypodium", "pectinatum")
        .bas_authors(None, &["L.f."])
        .partial("typica Rosent")
        .nothing_else();
    // prose and notes the parser does not know
    assert_name("Acinos clinopodiifacie Gilib., opus utique oppr.")
        .species("Acinos", "clinopodiifacie")
        .comb_authors(None, &["Gilib."])
        .partial("opus utique oppr.")
        .nothing_else();
    // the same in a separately supplied authorship
    assert_name_auth(
        "Abildgaardia baeothryon",
        "A.St.-Hil., provisionally listed as a synonym.",
    )
    .species("Abildgaardia", "baeothryon")
    .comb_authors(None, &["A.St.-Hil."])
    .partial("provisionally listed as a synonym.")
    .nothing_else();
}

#[test]
fn lower_case_words_an_author_carries_stay() {
    assert_name("Cryptopleura farlowiana (J.Agardh) ver Steeg & Jossly")
        .species("Cryptopleura", "farlowiana")
        .comb_authors(None, &["ver Steeg", "Jossly"])
        .bas_authors(None, &["J.Agardh"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Schizonema crinoideum Crouan frat., 1867")
        .species("Schizonema", "crinoideum")
        .comb_authors(Some("1867"), &["Crouan frat."])
        .nothing_else();
    // right after another word: a surname written in lower case, or cut by a broken character
    assert_name("Desmodora scaldensis De man, 1889")
        .species("Desmodora", "scaldensis")
        .comb_authors(Some("1889"), &["De man"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Amelanchier cretica Hal csy")
        .species("Amelanchier", "cretica")
        .comb_authors(None, &["Hal csy"])
        .nothing_else();
}

#[test]
fn an_ex_with_no_author_after_it_keeps_the_authors_before() {
    // the validating author is unknown or missing: the cited one is all there is
    assert_name("Bembicia uniflora (H.Perrier) Capuron ex ?")
        .species("Bembicia", "uniflora")
        .comb_authors(None, &["Capuron"])
        .bas_authors(None, &["H.Perrier"])
        .code(NomCode::Botanical)
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
    assert_name("Adenophora manshurica Nakai ex")
        .species("Adenophora", "manshurica")
        .comb_authors(None, &["Nakai"])
        .nothing_else();
}

#[test]
fn a_new_sentence_after_the_authorship_is_no_author() {
    // after the dot ending a year: a reference or prose
    assert_name("Gephyrella Mello-Leitão 1918. Rev. Soc. Brasil. Sci.")
        .monomial("Gephyrella")
        .comb_authors(Some("1918"), &["Mello-Leitão"])
        .partial("Rev. Soc. Brasil. Sci.")
        .code(NomCode::Zoological)
        .nothing_else();
    // after the dot ending the epithet, prose
    assert_name("Negalasa fumalis. Next sentence")
        .species("Negalasa", "fumalis")
        .partial("Next sentence")
        .nothing_else();
    // an author after the stray dot stays one
    assert_name("Amphiporus microocelli. Kajihara et al,2008")
        .species("Amphiporus", "microocelli")
        .comb_authors(Some("2008"), &["Kajihara", "al"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn an_abbreviated_author_glued_to_the_epithet() {
    assert_name("Pentapanax angelicifoliusGriseb.")
        .species("Pentapanax", "angelicifolius")
        .comb_authors(None, &["Griseb."])
        .nothing_else();
}
