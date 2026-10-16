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
    // with the issue of the volume, a page range, the year after the page without a full stop
    // (#147); the page or the issue was left unparsed
    assert_name_auth(
        "Roussoella siamensis",
        "Phook., Jian K. Liu & K. D. Hyde, Phytotaxa 181 (1): 18 2014",
    )
    .species("Roussoella", "siamensis")
    .comb_authors(Some("2014"), &["Phook.", "Jian K.Liu", "K.D.Hyde"])
    .published_in("Phytotaxa 181 (1): 18 2014")
    .published_in_year(Some(2014))
    .nothing_else();
    assert_name_auth(
        "Polylepis besseri",
        "Hieron., Bot. Jahrb. Syst. 21 (3): 312. 1896.",
    )
    .species("Polylepis", "besseri")
    .comb_authors(Some("1896"), &["Hieron."])
    .published_in("Bot. Jahrb. Syst. 21 (3): 312. 1896")
    .published_in_year(Some(1896))
    .nothing_else();
    assert_name_auth("Wisterieae", "Zhu, Cathaya 6: 115 - 124 1994")
        .monomial("Wisterieae")
        .comb_authors(Some("1994"), &["Zhu"])
        .published_in("Cathaya 6: 115 - 124 1994")
        .published_in_year(Some(1994))
        .nothing_else();
}

#[test]
fn a_journal_with_volume_and_page_after_the_year_is_the_reference() {
    // the title was read as an author, the volume and page dropped (#147)
    assert_name("Aus bus Smith, 1850 Journal of Zoology 8(5): 563")
        .species("Aus", "bus")
        .comb_authors(Some("1850"), &["Smith"])
        .published_in("Journal of Zoology 8(5): 563")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Aus bus Smith 1850 Bull. Soc. ent. Fr. 8(5): 563")
        .species("Aus", "bus")
        .comb_authors(Some("1850"), &["Smith"])
        .published_in("Bull. Soc. ent. Fr. 8(5): 563")
        .code(NomCode::Zoological)
        .nothing_else();
    // a zoologist's page after the year stays the page
    assert_name_auth("Typhlops gracilis", "STERNFELD 1910: 70")
        .species("Typhlops", "gracilis")
        .comb_authors(Some("1910"), &["Sternfeld"])
        .published_in_page("70")
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn an_abbreviated_title_after_the_authors_is_the_reference() {
    // a title without volume and page was read as one more author (#147)
    assert_name_auth("Lophocolea martiana", "Nees, Syn. Hepat.")
        .species("Lophocolea", "martiana")
        .comb_authors(None, &["Nees"])
        .published_in("Syn. Hepat.")
        .nothing_else();
    assert_name_auth("Bauhinia vahlii", "Wight & Arn., Cat. Ind. Pl.")
        .species("Bauhinia", "vahlii")
        .comb_authors(None, &["Wight", "Arn."])
        .published_in("Cat. Ind. Pl.")
        .nothing_else();
    assert_name_auth("Lepidozia cancellata", "Colenso, Trans. & Proc.")
        .species("Lepidozia", "cancellata")
        .comb_authors(None, &["Colenso"])
        .published_in("Trans. & Proc.")
        .nothing_else();
    // the year closing it is the year
    assert_name_auth(
        "Quercus texana",
        "Buckley, Proc. Acad. Nat. Sci. Philadelphia 1860",
    )
    .species("Quercus", "texana")
    .comb_authors(Some("1860"), &["Buckley"])
    .published_in("Proc. Acad. Nat. Sci. Philadelphia 1860")
    .published_in_year(Some(1860))
    .nothing_else();
    // Bull. abbreviates Bulliard too, so a Bulletin waits for a list of titles
    assert_name_auth("Bauhinia cancellata", "Ridl., Bull. Misc. Inf. Kew 1929")
        .species("Bauhinia", "cancellata")
        .comb_authors(Some("1929"), &["Ridl.", "Bull.Misc.Inf.Kew"])
        .code(NomCode::Zoological)
        .nothing_else();
    // a page with no volume, the page was left unparsed
    assert_name("Schizanthus Ruiz & Pav., Fl. Peruv. Prodr.: 4. 1794")
        .monomial("Schizanthus")
        .comb_authors(Some("1794"), &["Ruiz", "Pav."])
        .published_in("Fl. Peruv. Prodr.: 4. 1794")
        .published_in_year(Some(1794))
        .nothing_else();
    assert_name_auth("Solanum wrightii", "Benth., Fl. Hongk. 243. 1861.")
        .species("Solanum", "wrightii")
        .comb_authors(Some("1861"), &["Benth."])
        .published_in("Fl. Hongk. 243. 1861")
        .published_in_year(Some(1861))
        .nothing_else();
}

#[test]
fn a_given_name_abbreviated_like_a_title_is_an_author() {
    // Archana Singh and Ed. Navarro, not the titles Arch. and Ed.
    assert_name_auth("Stenella capparicola", "S.K. Singh, Arch. Singh & Kamal")
        .species("Stenella", "capparicola")
        .comb_authors(None, &["S.K.Singh", "Arch.Singh", "Kamal"])
        .nothing_else();
    assert_name_auth(
        "Aschersoniodoxa peruviana",
        "Al-Shehbaz, Ed.Navarro & A.Cano",
    )
    .species("Aschersoniodoxa", "peruviana")
    .comb_authors(None, &["Al-Shehbaz", "Ed.Navarro", "A.Cano"])
    .nothing_else();
    // and a given name that is no title word at all
    assert_name_auth("Genlisea barthlottii", "Porembski, Eb. Fisch. & Gemmel")
        .species("Genlisea", "barthlottii")
        .comb_authors(None, &["Porembski", "Eb.Fisch.", "Gemmel"])
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
    .comb_in_authors(&["Sichuan Institute of Biology Herpetology Department"])
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
    .comb_in_authors(&["Thiery Cuenot", "Lambert"])
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

#[test]
fn minor_authorship_shapes_of_the_reimport() {
    // #105: a second citation in the basionym bracket is a note, no co-author
    assert_name("Amphiesma parallelum (Boulenger, 1890; Kramer, 1977)")
        .species("Amphiesma", "parallelum")
        .bas_authors(Some("1890"), &["Boulenger"])
        .sensu("Kramer, 1977")
        .code(NomCode::Zoological)
        .nothing_else();
    // a basionym author written after the combination author: turned round when abbreviated,
    // never merged into one person
    assert_name_auth("Cordyceps militaris", "Fr. (L.)")
        .species("Cordyceps", "militaris")
        .bas_authors(None, &["L."])
        .comb_authors(None, &["Fr."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Penicillium mariae-crucis var. flavescens Quintanilla (Saez)")
        .infra_species("Penicillium", "mariae-crucis", Rank::Variety, "flavescens")
        .comb_authors(None, &["Quintanilla"])
        .partial("(Saez)")
        .nothing_else();
    // a hybrid sign after the genus marks the nothogenus
    assert_name_hinted("Eriochroma ×", Some("J.M.H.Shaw"), Some(Rank::Genus), None)
        .monomial_rank("Eriochroma", Rank::Genus)
        .notho(&[NamePart::Generic])
        .comb_authors(None, &["J.M.H.Shaw"])
        .code(NomCode::Botanical)
        .nothing_else();
    // "n.aff." is a new species near the epithet
    assert_name("Hoploseius n.aff.tenuis Lindquist, 1965")
        .species("Hoploseius", "tenuis")
        .qualifiers(&[(NamePart::Specific, "aff.")])
        .comb_authors(Some("1965"), &["Lindquist"])
        .code(NomCode::Zoological)
        .type_(NameType::Informal)
        .nothing_else();
    // the comment of a sic is a note
    assert_name_auth("Turbo porphyrites", "[sic, porphyria]")
        .species("Turbo", "porphyrites")
        .sic()
        .sensu("porphyria")
        .nothing_else();
}

#[test]
fn a_lowercase_bracket_after_the_authors_is_no_part_of_them() {
    // #107: a remark or an alternative epithet in brackets was glued to the last author, its space
    // removed: "Chiov.native", "Lindb.huntii"
    assert_name("Pennisetum setaceum (Forssk.) Chiov. (native)")
        .species("Pennisetum", "setaceum")
        .bas_authors(None, &["Forssk."])
        .comb_authors(None, &["Chiov."])
        .partial("(native)")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Pennisetum setaceum", "(Forssk.) Chiov. (native)")
        .species("Pennisetum", "setaceum")
        .bas_authors(None, &["Forssk."])
        .comb_authors(None, &["Chiov."])
        .partial("(native)")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Andreaea rothii ssp. falcata (Schimp.) Lindb. (huntii)")
        .infra_species("Andreaea", "rothii", Rank::Subspecies, "falcata")
        .bas_authors(None, &["Schimp."])
        .comb_authors(None, &["Lindb."])
        .partial("(huntii)")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Brachythecium curtum (Lindb.) J.Lange & C.Jens. (oedipodium)")
        .species("Brachythecium", "curtum")
        .bas_authors(None, &["Lindb."])
        .comb_authors(None, &["J.Lange", "C.Jens."])
        .partial("(oedipodium)")
        .code(NomCode::Botanical)
        .nothing_else();
    // filed like a note after the year: a status, a manuscript, "in part"
    assert_name("Angraecum carifolium H.Perrier (sphalm)")
        .species("Angraecum", "carifolium")
        .comb_authors(None, &["H.Perrier"])
        .nom_note("sphalm")
        .nothing_else();
    assert_name_auth("Nitzschia subconstricta", "Grunow (ms)")
        .species("Nitzschia", "subconstricta")
        .comb_authors(None, &["Grunow"])
        .nom_note("ms")
        .manuscript()
        .nothing_else();
    assert_name_auth("Cyzicus lobatus", "Wolf (in litteris) in Daday, 1914")
        .species("Cyzicus", "lobatus")
        .comb_authors(Some("1914"), &["Wolf"])
        .published_in("Daday, 1914")
        .published_in_year(Some(1914))
        .nom_note("in litteris")
        .manuscript()
        .comb_in_authors(&["Daday"])
        .nothing_else();
    assert_name_auth("Asterina penicillaris", "Sladen, 1889 (part)")
        .species("Asterina", "penicillaris")
        .comb_authors(Some("1889"), &["Sladen"])
        .doubtful()
        .code(NomCode::Zoological)
        .nothing_else();
    // not after a uninomial
    assert_name("Acantharea (awaiting allocation)")
        .monomial("Acantharea")
        .partial("(awaiting allocation)")
        .nothing_else();
}

#[test]
fn doubled_brackets_are_single_ones() {
    // #113: a subgenus, a basionym author or a bracketed note doubled by the source reads as the
    // single bracket. 475 CLB names have a "((", most of them a subgenus: the epithet was lost.
    assert_name("Atheta ((Datomicra)) particula")
        .species_ig("Atheta", "Datomicra", "particula")
        .nothing_else();
    assert_name("Aurotipula orion ( (Hudson, 1895) )")
        .species("Aurotipula", "orion")
        .bas_authors(Some("1895"), &["Hudson"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Aus bus Smith, 1900 [[nomen nudum]]")
        .species("Aus", "bus")
        .comb_authors(Some("1900"), &["Smith"])
        .nom_note("nomen nudum")
        .code(NomCode::Zoological)
        .nothing_else();
    // an unclosed one
    assert_name("Aganisia cyanea f. alba ((Roeth & O.Gruss) Meneguzzo")
        .infra_species("Aganisia", "cyanea", Rank::Form, "alba")
        .bas_authors(None, &["Roeth", "O.Gruss"])
        .comb_authors(None, &["Meneguzzo"])
        .code(NomCode::Botanical)
        .nothing_else();
    // a surplus closing one, in whichever column the authorship comes
    assert_name("Aus bus Smith, 1900 [nomen nudum]]")
        .species("Aus", "bus")
        .comb_authors(Some("1900"), &["Smith"])
        .nom_note("nomen nudum")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_auth("Schedonorus inermis", "(Leysser) ) Palisot de Beauvois")
        .species("Schedonorus", "inermis")
        .bas_authors(None, &["Leysser"])
        .comb_authors(None, &["Palisot de Beauvois"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Abies alba", "((L.)) Mill.")
        .species("Abies", "alba")
        .bas_authors(None, &["L."])
        .comb_authors(None, &["Mill."])
        .code(NomCode::Botanical)
        .nothing_else();
    // brackets that do enclose more stay nested
    assert_name("Ageniella serrula ((Fox), 1897)")
        .species("Ageniella", "serrula")
        .bas_authors(Some("1897"), &["Fox"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn lowercase_words_after_a_comma_are_no_authorship() {
    // #118: a population, a morph or a breed after the binomial is a note, not an author. GBIF
    // occurrences: `Columba livia, domesticated populations` alone has 319k.
    assert_name("Columba livia, domesticated populations")
        .species("Columba", "livia")
        .partial("domesticated populations")
        .nothing_else();
    assert_name("Felis catus, feral population")
        .species("Felis", "catus")
        .partial("feral population")
        .nothing_else();
    assert_name("Canis lupus, domestic")
        .species("Canis", "lupus")
        .partial("domestic")
        .nothing_else();
    assert_name("Buteo buteo, dark morph")
        .species("Buteo", "buteo")
        .partial("dark morph")
        .nothing_else();
    // a capitalised word further on does not make it an authorship
    assert_name("Cardamine gunnii, small Tasmanian variant Hewson")
        .species("Cardamine", "gunnii")
        .partial("small Tasmanian variant Hewson")
        .nothing_else();
    // added to what the name part left unparsed
    assert_name("Calamintha officinalis f. brevifrons ilic, dark morph")
        .infra_species("Calamintha", "officinalis", Rank::Form, "brevifrons")
        .partial("ilic dark morph")
        .nothing_else();
    // an author particle is an author's
    assert_name("Zodarion alba, van Bosmans")
        .species("Zodarion", "alba")
        .comb_authors(None, &["van Bosmans"])
        .nothing_else();
    // an author team written all in lower case, held together by a year or an ampersand
    assert_name("Devibursaphelenchus wangi, gu, wang & zheng, 2010")
        .species("Devibursaphelenchus", "wangi")
        .comb_authors(Some("2010"), &["gu", "wang", "zheng"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Nemophora apiciprocessa, wang et li, 2022")
        .species("Nemophora", "apiciprocessa")
        .comb_authors(Some("2022"), &["wang", "li"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn a_comma_next_to_a_rank_marker_keeps_the_infraspecific_name() {
    // #118: in front of the marker the comma is dropped
    assert_name("Felis tigris, var. montanus")
        .infra_species("Felis", "tigris", Rank::Variety, "montanus")
        .nothing_else();
    assert_name("Micronycteris megalotis, f. typica")
        .infra_species("Micronycteris", "megalotis", Rank::Form, "typica")
        .nothing_else();
    assert_name("Equus hemionus, var syriacus")
        .infra_species("Equus", "hemionus", Rank::Variety, "syriacus")
        .nothing_else();
    assert_name("Oncifelis geoffroyi, forma argentea")
        .infra_species("Oncifelis", "geoffroyi", Rank::Form, "argentea")
        .nothing_else();
    assert_name("Abies alba, ssp. alpina (L.) Mill.")
        .infra_species("Abies", "alba", Rank::Subspecies, "alpina")
        .comb_authors(None, &["Mill."])
        .bas_authors(None, &["L."])
        .code(NomCode::Botanical)
        .nothing_else();
    // right after the marker it stands for the abbreviation dot
    assert_name("Hieracium nigrescens subsp, linophyton")
        .infra_species("Hieracium", "nigrescens", Rank::Subspecies, "linophyton")
        .nothing_else();
    assert_name("Dalbergia retusa var, lineata Pittier")
        .infra_species("Dalbergia", "retusa", Rank::Variety, "lineata")
        .comb_authors(None, &["Pittier"])
        .nothing_else();
    assert_name("Abies alba, var, alpina")
        .infra_species("Abies", "alba", Rank::Variety, "alpina")
        .nothing_else();
    // no epithet behind the marker: the tail stays unparsed
    assert_name("Dasyhelea bulbosa sp, nov.")
        .species("Dasyhelea", "bulbosa")
        .partial("nov.")
        .nothing_else();
}

#[test]
fn the_epithet_repeated_in_capitals_is_no_author() {
    // 172 ChecklistBank names, most of them lactobacilli whose authorship column holds the epithet
    // and the year (#123)
    assert_name(
        "Paramormyrops gabonensis GABONENSIS TAVERNE, THYS VAN DEN AUDENAERDE, & HEYMER 1977",
    )
    .species("Paramormyrops", "gabonensis")
    .comb_authors(
        Some("1977"),
        &["Taverne", "Thys van den Audenaerde", "Heymer"],
    )
    .code(NomCode::Zoological)
    .warning(&["Removed: GABONENSIS"])
    .nothing_else();
    assert_name_auth("Loigolactobacillus binensis", "BINENSIS 2020")
        .species("Loigolactobacillus", "binensis")
        .comb_authors(Some("2020"), &[])
        .warning(&["Removed: BINENSIS"])
        .nothing_else();
    assert_name("Coryphocera affinis AFFINIS,1848")
        .species("Coryphocera", "affinis")
        .comb_authors(Some("1848"), &[])
        .warning(&["Removed: AFFINIS"])
        .nothing_else();
    // only in capitals: a capitalised epithet is put in lower case
    assert_name("Aurelia coerulea COERULEA VON LENDENFELD 1884")
        .species("Aurelia", "coerulea")
        .comb_authors(Some("1884"), &["von Lendenfeld"])
        .code(NomCode::Zoological)
        .warning(&["Removed: COERULEA"])
        .nothing_else();
}
