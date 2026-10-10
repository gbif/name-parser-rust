// SPDX-License-Identifier: Apache-2.0
//! What an authorship carries besides its authors — manuscript markers, emendations, references,
//! notes, a lone particle — kept out of the author list. Frequencies are from ChecklistBank's
//! verbatim authorship column.

mod common;
use common::*;
use nameparser::model::{warnings, NamePart, NameType, NomCode, Rank};

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
    .nom_note("Approved Lists 1980")
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
fn an_of_note_needs_authors_and_no_corporate_name() {
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
    // inside the basionym's bracket it is the note too, its year the cited concept's
    assert_name_auth("Thelastoma bulhoesi", "(Mag of Dollfus 1952)")
        .species("Thelastoma", "bulhoesi")
        .bas_authors(None, &["Mag"])
        .sensu("of Dollfus 1952")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_auth(
        "Aus bus",
        "(Schellwien of Grozdylova & Lebedeva 1961) Smith, 1970",
    )
    .species("Aus", "bus")
    .bas_authors(None, &["Schellwien"])
    .comb_authors(Some("1970"), &["Smith"])
    .sensu("of Grozdylova & Lebedeva 1961")
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
    .nom_note("Approved Lists 1980")
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
    // an epithet written without its rank marker, after the species author and before its own
    assert_name("Loranthus incanus Schumach. & Thonn.  sessilis Sprague")
        .infra_species("Loranthus", "incanus", Rank::InfraspecificName, "sessilis")
        .specific_authors(None, &["Schumach.", "Thonn."])
        .comb_authors(None, &["Sprague"])
        .nothing_else();
    assert_name("Polypodium pectinatum (L. f.) typica Rosent")
        .infra_species(
            "Polypodium",
            "pectinatum",
            Rank::InfraspecificName,
            "typica",
        )
        .specific_bas_authors(None, &["L.f."])
        .comb_authors(None, &["Rosent"])
        .code(NomCode::Botanical)
        .nothing_else();
    // a note keyword is no epithet
    assert_name("Abies alba Mill. teste Smith")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .partial("teste Smith")
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

#[test]
fn a_second_name_after_an_ampersand_is_no_author() {
    assert_name("Mesalia zinkeni (Dunker 1851) & Promathildia turritella (Dunker 1851)")
        .species("Mesalia", "zinkeni")
        .bas_authors(Some("1851"), &["Dunker"])
        .partial("& Promathildia turritella (Dunker 1851)")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Sillago ciliata & Sillago maculata")
        .species("Sillago", "ciliata")
        .partial("& Sillago maculata")
        .nothing_else();
}

/// An unparsed bracket keeps the spaces of the source: Java glued its words together, so
/// `(swamp variant)` came back as `(swampvariant)` (ChecklistBank dataset 1162).
#[test]
fn an_unparsed_bracket_keeps_its_spaces() {
    assert_name("Acacia retinodes var. retinodes (swamp variant)")
        .infra_species("Acacia", "retinodes", Rank::Variety, "retinodes")
        .partial("(swamp variant)")
        .nothing_else();
    assert_name("Acacia brachybotyra (appressed hair variant)")
        .species("Acacia", "brachybotyra")
        .partial("(appressed hair variant)")
        .nothing_else();
}

/// More spellings of sic and corrig.: a dot after sic, a bare trailing "sic.", and corrig. before
/// a comma or glued to the next author.
#[test]
fn sic_and_corrig_spellings_in_a_separate_authorship() {
    assert_name_auth("Bunodeopsis strumosa", "AANDRES, 1881 sic.")
        .species("Bunodeopsis", "strumosa")
        .comb_authors(Some("1881"), &["Aandres"])
        .code(NomCode::Zoological)
        .sic()
        .nothing_else();
    assert_name_auth("Stephanauge nexilis", "(Verril, 1883) [sic.]")
        .species("Stephanauge", "nexilis")
        .bas_authors(Some("1883"), &["Verril"])
        .code(NomCode::Zoological)
        .sic()
        .nothing_else();
    assert_name_auth("Anemonia pelagica", "Quoy et Gaymard [sic.]")
        .species("Anemonia", "pelagica")
        .comb_authors(None, &["Quoy", "Gaymard"])
        .sic()
        .nothing_else();
    assert_name_auth("Abies alba", "Smith corrig., 1900")
        .species("Abies", "alba")
        .comb_authors(Some("1900"), &["Smith"])
        .code(NomCode::Zoological)
        .corrig()
        .nothing_else();
    assert_name_auth("Aus bus", "corrig.Yoon et al. 2001")
        .species("Aus", "bus")
        .comb_authors(Some("2001"), &["Yoon", "al."])
        .code(NomCode::Zoological)
        .corrig()
        .nothing_else();
}

#[test]
fn a_page_after_the_year_is_the_page() {
    // #78: 77 CLB authorships cite a page with "p." after the year; the "p." became an author and
    // the page a second year, the imprint year ("Girault & p., 1913 [244]")
    assert_name("Trichaporoidella margiventris Girault 1913, p.244")
        .species("Trichaporoidella", "margiventris")
        .comb_authors(Some("1913"), &["Girault"])
        .published_in_page("244")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_auth("Aus bus", "Smith, 1900, pp. 12-14")
        .species("Aus", "bus")
        .comb_authors(Some("1900"), &["Smith"])
        .published_in_page("12-14")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_auth(
        "Parhabdocidaris",
        "Thiery in Thiery Cuenot & Lambert 1928, p. 123",
    )
    .monomial("Parhabdocidaris")
    .comb_authors(Some("1928"), &["Thiery"])
    .published_in("Thiery Cuenot & Lambert 1928")
    .published_in_year(Some(1928))
    .published_in_page("123")
    .nothing_else();
    // brackets holding only the year are the name's own citation
    assert_name("Hieracium kolthoffianum Hyl. (1943 p. 155)")
        .species("Hieracium", "kolthoffianum")
        .comb_authors(Some("1943"), &["Hyl."])
        .published_in_page("155")
        .nothing_else();
    // no year, no page
    assert_name("Aus bus Smith p. 44")
        .species("Aus", "bus")
        .comb_authors(None, &["Smith p."])
        .nothing_else();
}

#[test]
fn a_page_in_the_basionym_brackets_is_dropped() {
    // #78: the page of the original description is no page of the name's publication; dropped as
    // the colon form in the brackets is
    for authorship in [
        "(McMurrich 1889, p. 111)",
        "(McMurrich 1889 p. 111)",
        "(McMurrich 1889: 111)",
    ] {
        assert_name_auth("Actinia bermudensis", authorship)
            .species("Actinia", "bermudensis")
            .bas_authors(Some("1889"), &["McMurrich"])
            .code(NomCode::Zoological)
            .nothing_else();
    }
    assert_name("Actinia bermudensis (McMurrich 1889, p. 111)")
        .species("Actinia", "bermudensis")
        .bas_authors(Some("1889"), &["McMurrich"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn an_implausible_year_is_no_year() {
    // #85: a voucher number after a binomial is its phrase
    assert_name("Abietinella abietina Bezgodov 116")
        .species("Abietinella", "abietina")
        .phrase("Bezgodov 116")
        .type_(NameType::Informal)
        .nothing_else();
    assert_name("Aglaia aff. spectabilis Pannell 2083")
        .species("Aglaia", "spectabilis")
        .qualifiers(&[(NamePart::Specific, "aff.")])
        .phrase("Pannell 2083")
        .type_(NameType::Informal)
        .nothing_else();
    // a truncated year is left unparsed, flagged
    assert_name_auth("Sepidium reichei", ",187 Allard")
        .species("Sepidium", "reichei")
        .comb_authors(None, &["Allard"])
        .partial("187")
        .doubtful()
        .code(NomCode::Zoological)
        .warning(&[warnings::UNLIKELY_YEAR])
        .nothing_else();
}
