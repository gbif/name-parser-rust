// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserGnaTest (methods on lines 31-606).
mod common;
use common::*;
use nameparser::model::{NomCode, Rank};

#[test]
fn uninomials_without_authorship() {
    // group: Uninomials without authorship
    assert_name("Pseudocercospora").monomial("Pseudocercospora");
}

#[test]
fn uninomials_with_authorship() {
    // group: Uninomials with authorship — author whitespace around dots is collapsed
    // ("M.T. Lucas" → "M.T.Lucas"), ligatures/diacritics kept verbatim, year-bearing
    // trinomials become SUBSPECIES (clearly zoological).
    assert_name("Tremoctopus violaceus Delle Chiaje, 1830")
        .species("Tremoctopus", "violaceus")
        .comb_authors(Some("1830"), &["Delle Chiaje"]);
    assert_name("Protis hydrothermica ten Hove & Zibrowius, 1986")
        .species("Protis", "hydrothermica")
        .comb_authors(Some("1986"), &["ten Hove", "Zibrowius"]);
    assert_name("Cladoniicola staurospora Diederich, van den Boom & Aptroot 2001")
        .species("Cladoniicola", "staurospora")
        .comb_authors(Some("2001"), &["Diederich", "van den Boom", "Aptroot"]);
    assert_name("Stagonospora polyspora M.T. Lucas & Sousa da Câmara 1934")
        .species("Stagonospora", "polyspora")
        .comb_authors(Some("1934"), &["M.T.Lucas", "Sousa da Câmara"]);
    assert_name("Stagonospora polyspora M.T. Lucas et Sousa da Câmara 1934")
        .species("Stagonospora", "polyspora")
        .comb_authors(Some("1934"), &["M.T.Lucas", "Sousa da Câmara"]);
    assert_name("Pseudocercospora dendrobii U. Braun & Crous 2003")
        .species("Pseudocercospora", "dendrobii")
        .comb_authors(Some("2003"), &["U.Braun", "Crous"]);
    assert_name("Abaxisotima acuminata (Wang, Yuwen & Xiangwei Liu 1996)")
        .species("Abaxisotima", "acuminata")
        .bas_authors(Some("1996"), &["Wang", "Yuwen", "Xiangwei Liu"]);
    assert_name("Aboilomimus sichuanensis ornatus Liu, Xiang-wei, M. Zhou, W Bi & L. Tang, 2009")
        .infra_species("Aboilomimus", "sichuanensis", Rank::Subspecies, "ornatus")
        .comb_authors(
            Some("2009"),
            &["Liu", "Xiang-wei", "M.Zhou", "W.Bi", "L.Tang"],
        )
        .code(NomCode::Zoological);
    assert_name("Pseudocercospora Speg.")
        .monomial("Pseudocercospora")
        .comb_authors(None, &["Speg."]);
    // "(synonym)" tail is currently parsed as an extra author, not stripped.
    assert_name("Döringina Ihering 1929 (synonym)")
        .monomial("Döringina")
        .comb_authors(Some("1929"), &["Ihering", "synonym"]);
    assert_name("Pseudocercospora Speg., Francis Jack.-Drake.")
        .monomial("Pseudocercospora")
        .comb_authors(None, &["Speg.", "Francis Jack.-Drake."]);
    assert_name("Aaaba de Laubenfels, 1936")
        .monomial("Aaaba")
        .comb_authors(Some("1936"), &["de Laubenfels"]);
    assert_name("Abbottia F. von Mueller, 1875")
        .monomial("Abbottia")
        .comb_authors(Some("1875"), &["F.von Mueller"]);
    assert_name("Abella von Heyden, 1826")
        .monomial("Abella")
        .comb_authors(Some("1826"), &["von Heyden"]);
    assert_name("Micropleura v Linstow 1906")
        .monomial("Micropleura")
        .comb_authors(Some("1906"), &["v Linstow"]);
    assert_name("Pseudocercospora Speg. 1910")
        .monomial("Pseudocercospora")
        .comb_authors(Some("1910"), &["Speg."]);
    assert_name("Pseudocercospora Spegazzini, 1910")
        .monomial("Pseudocercospora")
        .comb_authors(Some("1910"), &["Spegazzini"]);
    assert_name("Rhynchonellidae d'Orbigny 1847")
        .monomial("Rhynchonellidae")
        .comb_authors(Some("1847"), &["d'Orbigny"]);
    assert_name("Rhynchonellidae d‘Orbigny 1847")
        .monomial("Rhynchonellidae")
        .comb_authors(Some("1847"), &["d'Orbigny"]);
    assert_name("Rhynchonellidae d’Orbigny 1847")
        .monomial("Rhynchonellidae")
        .comb_authors(Some("1847"), &["d'Orbigny"]);
    assert_name("Ataladoris Iredale & O'Donoghue 1923")
        .monomial("Ataladoris")
        .comb_authors(Some("1923"), &["Iredale", "O'Donoghue"]);
    assert_name("Anteplana le Renard 1995")
        .monomial("Anteplana")
        .comb_authors(Some("1995"), &["le Renard"]);
    assert_name("Candinia le Renard, Sabelli & Taviani 1996")
        .monomial("Candinia")
        .comb_authors(Some("1996"), &["le Renard", "Sabelli", "Taviani"]);
    // "le-sourdianum" is parsed as the species epithet, "Fourn." as the comb author.
    assert_name("Polypodium le-sourdianum Fourn.")
        .species("Polypodium", "le-sourdianum")
        .comb_authors(None, &["Fourn."]);
}

#[test]
fn two_letter_genus_names_legacy_genera_not_allowed_anymore() {
    // group: Two-letter genus names (legacy genera, not allowed anymore)
    assert_name("Ca Dyar 1914")
        .monomial("Ca")
        .comb_authors(Some("1914"), &["Dyar"]);
    assert_name("Ea Distant 1911")
        .monomial("Ea")
        .comb_authors(Some("1911"), &["Distant"]);
    assert_name("Do").monomial("Do");
    assert_name("Ge Nicéville 1895")
        .monomial("Ge")
        .comb_authors(Some("1895"), &["Nicéville"]);
    assert_name("Ia Thomas 1902")
        .monomial("Ia")
        .comb_authors(Some("1902"), &["Thomas"]);
    assert_name("Io Lea 1831")
        .monomial("Io")
        .comb_authors(Some("1831"), &["Lea"]);
    assert_name("Io Blanchard 1852")
        .monomial("Io")
        .comb_authors(Some("1852"), &["Blanchard"]);
    assert_name("Ix Bergroth 1916")
        .monomial("Ix")
        .comb_authors(Some("1916"), &["Bergroth"]);
    assert_name("Lo Seale 1906")
        .monomial("Lo")
        .comb_authors(Some("1906"), &["Seale"]);
    assert_name("Oa Girault 1929")
        .monomial("Oa")
        .comb_authors(Some("1929"), &["Girault"]);
    assert_name("Oo").monomial("Oo");
    assert_name("Nu").monomial("Nu");
    assert_name("Ra Whitley 1931")
        .monomial("Ra")
        .comb_authors(Some("1931"), &["Whitley"]);
    assert_name("Ty Bory de St. Vincent 1827")
        .monomial("Ty")
        .comb_authors(Some("1827"), &["Bory de St.Vincent"]);
    assert_name("Ua Girault 1929")
        .monomial("Ua")
        .comb_authors(Some("1929"), &["Girault"]);
    assert_name("Aa Baker 1940")
        .monomial("Aa")
        .comb_authors(Some("1940"), &["Baker"]);
    assert_name("Ja Uéno 1955")
        .monomial("Ja")
        .comb_authors(Some("1955"), &["Uéno"]);
    assert_name("Zu Walters & Fitch 1960")
        .monomial("Zu")
        .comb_authors(Some("1960"), &["Walters", "Fitch"]);
    assert_name("La Bleszynski 1966")
        .monomial("La")
        .comb_authors(Some("1966"), &["Bleszynski"]);
    assert_name("Qu Durkoop")
        .monomial("Qu")
        .comb_authors(None, &["Durkoop"]);
    assert_name("As Slipinski 1982")
        .monomial("As")
        .comb_authors(Some("1982"), &["Slipinski"]);
    assert_name("Ba Solem 1983")
        .monomial("Ba")
        .comb_authors(Some("1983"), &["Solem"]);
}

#[test]
fn binomials_without_authorship() {
    // group: Binomials without authorship
    assert_name("Notopholia corrusca").species("Notopholia", "corrusca");
    assert_name("Cyathicula scelobelonium").species("Cyathicula", "scelobelonium");
    assert_name("Pseudocercospora     dendrobii").species("Pseudocercospora", "dendrobii");
    assert_name("Cucurbita pepo").species("Cucurbita", "pepo");
    assert_name("Hirsutëlla male").species("Hirsutëlla", "male");
    assert_name("Aëtosaurus ferratus").species("Aëtosaurus", "ferratus");
    assert_name("Remera cvancarai").species("Remera", "cvancarai");
}

#[test]
fn binomials_with_authorship() {
    // group: Binomials with authorship
    assert_name("Gazella farasani Thouless, al Bassri, 1991")
        .species("Gazella", "farasani")
        .comb_authors(Some("1991"), &["Thouless", "al Bassri"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Anomalurus laticeps Aguilar-Amat i Banús, 1922")
        .species("Anomalurus", "laticeps")
        .comb_authors(Some("1922"), &["Aguilar-Amat i Banús"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Glis wagneri Đulić & Tortić, 1960")
        .species("Glis", "wagneri")
        .comb_authors(Some("1960"), &["Đulić", "Tortić"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Mico rondoni Ferrari, Sena, M. P. C. Schneider, & e Silva Júnior, 2010")
        .species("Mico", "rondoni")
        .comb_authors(
            Some("2010"),
            &["Ferrari", "Sena", "M.P.C.Schneider", "e Silva Júnior"],
        )
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Trachypithecus caudalis (Đào Văn Tiến, 1977)")
        .species("Trachypithecus", "caudalis")
        .bas_authors(Some("1977"), &["Đào Văn Tiến"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Cymatium raderi D’Attilio & Myers, 1984")
        .species("Cymatium", "raderi")
        .comb_authors(Some("1984"), &["D'Attilio", "Myers"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Melania testudinaria Von dem Busch, 1842")
        .species("Melania", "testudinaria")
        .comb_authors(Some("1842"), &["Von dem Busch"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Cryptopleura farlowiana (J.Agardh) ver Steeg & Jossly")
        .species("Cryptopleura", "farlowiana")
        .comb_authors(None, &["ver Steeg", "Jossly"])
        .bas_authors(None, &["J.Agardh"])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Pyxilla caput avis J.-J.Brun")
        .infra_species("Pyxilla", "caput", Rank::InfraspecificName, "avis")
        .comb_authors(None, &["J.-J.Brun"])
        .nothing_else();

    assert_name("Muscicapa randi Amadon & duPont, 1970")
        .species("Muscicapa", "randi")
        .comb_authors(Some("1970"), &["Amadon", "duPont"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Scytalopus alvarezlopezi Stiles, Laverde-R. & Cadena 2017")
        .species("Scytalopus", "alvarezlopezi")
        .comb_authors(Some("2017"), &["Stiles", "Laverde-R.", "Cadena"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Carabus (Tanaocarabus) hendrichsi Bolvar y Pieltain, Rotger & Coronado 1967")
        .species_ig("Carabus", "Tanaocarabus", "hendrichsi")
        .comb_authors(Some("1967"), &["Bolvar", "Pieltain", "Rotger", "Coronado"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Nemcia epacridoides (Meissner)Crisp")
        .species("Nemcia", "epacridoides")
        .comb_authors(None, &["Crisp"])
        .bas_authors(None, &["Meissner"])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Pseudocercospora dendrobii Goh & W.H. Hsieh 1990")
        .species("Pseudocercospora", "dendrobii")
        .comb_authors(Some("1990"), &["Goh", "W.H.Hsieh"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Pseudocercospora dendrobii Goh and W.H. Hsieh 1990")
        .species("Pseudocercospora", "dendrobii")
        .comb_authors(Some("1990"), &["Goh", "W.H.Hsieh"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Pseudocercospora dendrobii Goh et W.H. Hsieh 1990")
        .species("Pseudocercospora", "dendrobii")
        .comb_authors(Some("1990"), &["Goh", "W.H.Hsieh"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Schottera nicaeënsis (J.V. Lamouroux ex Duby) Guiry & Hollenberg")
        .species("Schottera", "nicaeënsis")
        .comb_authors(None, &["Guiry", "Hollenberg"])
        .bas_ex_authors(None, &["J.V.Lamouroux"])
        .bas_authors(None, &["Duby"])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Laevapex vazi dos Santos, 1989")
        .species("Laevapex", "vazi")
        .comb_authors(Some("1989"), &["dos Santos"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Periclimenaeus aurae dos Santos, Calado & Araújo, 2008")
        .species("Periclimenaeus", "aurae")
        .comb_authors(Some("2008"), &["dos Santos", "Calado", "Araújo"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Nototriton matama Boza-Oviedo, Rovito, Chaves, García-Rodríguez, Artavia, Bolaños, and Wake, 2012")
        .species("Nototriton", "matama")
        .comb_authors(Some("2012"), &["Boza-Oviedo", "Rovito", "Chaves", "García-Rodríguez", "Artavia", "Bolaños", "Wake"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Architectonica offlexa Iredale, 1931")
        .species("Architectonica", "offlexa")
        .comb_authors(Some("1931"), &["Iredale"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Maracanda amoena Mc'Lach")
        .species("Maracanda", "amoena")
        .comb_authors(None, &["Mc'Lach"])
        .nothing_else();

    assert_name("Maracanda amoena Mc’Lach")
        .species("Maracanda", "amoena")
        .comb_authors(None, &["Mc'Lach"])
        .nothing_else();

    assert_name("Tridentella tangeroae Bruce, 198?")
        .species("Tridentella", "tangeroae")
        .comb_authors(Some("198?"), &["Bruce"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Calobota acanthoclada (Dinter) Boatwr. & B.-E.van Wyk")
        .species("Calobota", "acanthoclada")
        .comb_authors(None, &["Boatwr.", "B.-E.van Wyk"])
        .bas_authors(None, &["Dinter"])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Zanthopsis bispinosa M'Coy, 1849")
        .species("Zanthopsis", "bispinosa")
        .comb_authors(Some("1849"), &["M'Coy"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Scilla rupestris v.d. Merwe")
        .species("Scilla", "rupestris")
        .comb_authors(None, &["v.d.Merwe"])
        .nothing_else();

    assert_name("Bembix bidentata v.d.L.")
        .species("Bembix", "bidentata")
        .comb_authors(None, &["v.d.L."])
        .nothing_else();

    assert_name("Pompilus cinctellus v. d. L.")
        .species("Pompilus", "cinctellus")
        .comb_authors(None, &["v.d.L."])
        .nothing_else();

    assert_name("Setaphis viridis v. d.G.")
        .species("Setaphis", "viridis")
        .comb_authors(None, &["v.d.G."])
        .nothing_else();

    assert_name("Coleophora mendica Baldizzone & v. d.Wolf 2000")
        .species("Coleophora", "mendica")
        .comb_authors(Some("2000"), &["Baldizzone", "v.d.Wolf"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Psoronaias semigranosa von dem Busch in Philippi, 1845")
        .species("Psoronaias", "semigranosa")
        .comb_authors(Some("1845"), &["von dem Busch"])
        .published_in("Philippi, 1845")
        .nothing_else();

    assert_name("Phora sororcula v d Wulp 1871")
        .species("Phora", "sororcula")
        .comb_authors(Some("1871"), &["v d Wulp"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Aeolothrips andalusiacus zur Strassen 1973")
        .species("Aeolothrips", "andalusiacus")
        .comb_authors(Some("1973"), &["zur Strassen"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Orthosia kindermannii Fischer v. Roslerstamm, 1837")
        .species("Orthosia", "kindermannii")
        .comb_authors(Some("1837"), &["Fischer v.Roslerstamm"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Boreophilia nomensis (Casey, 1910)")
        .species("Boreophilia", "nomensis")
        .bas_authors(Some("1910"), &["Casey"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Nereidavus kulkovi Kul'kov in Kul'kov & Obut, 1973")
        .species("Nereidavus", "kulkovi")
        .comb_authors(Some("1973"), &["Kul'kov"])
        .published_in("Kul'kov & Obut, 1973")
        .nothing_else();

    assert_name("Xylaria potentillae A S. Xu")
        .species("Xylaria", "potentillae")
        .comb_authors(None, &["A.S.Xu"])
        .nothing_else();

    assert_name("Pseudocyrtopora el Hajjaji 1987")
        .monomial("Pseudocyrtopora")
        .comb_authors(Some("1987"), &["el Hajjaji"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Geositta poeciloptera (zu Wied-Neuwied, 1830)")
        .species("Geositta", "poeciloptera")
        .bas_authors(Some("1830"), &["zu Wied-Neuwied"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Abacetus laevicollis de Chaudoir, 1869")
        .species("Abacetus", "laevicollis")
        .comb_authors(Some("1869"), &["de Chaudoir"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Gastrosericus eremorum von Beaumont 1955")
        .species("Gastrosericus", "eremorum")
        .comb_authors(Some("1955"), &["von Beaumont"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Agaricus squamula Berk. & M.A. Curtis 1860")
        .species("Agaricus", "squamula")
        .comb_authors(Some("1860"), &["Berk.", "M.A.Curtis"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Peltula coriacea Büdel, Henssen & Wessels 1986")
        .species("Peltula", "coriacea")
        .comb_authors(Some("1986"), &["Büdel", "Henssen", "Wessels"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Tuber liui A S. Xu 1999")
        .species("Tuber", "liui")
        .comb_authors(Some("1999"), &["A.S.Xu"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Lecanora wetmorei Śliwa 2004")
        .species("Lecanora", "wetmorei")
        .comb_authors(Some("2004"), &["Śliwa"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Vachonobisium troglophilum Vitali-di Castri, 1963")
        .species("Vachonobisium", "troglophilum")
        .comb_authors(Some("1963"), &["Vitali-di Castri"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Hyalesthes angustula Horvßth, 1909")
        .species("Hyalesthes", "angustula")
        .comb_authors(Some("1909"), &["Horvßth"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Platypus bicaudatulus Schedl (1935h)")
        .species("Platypus", "bicaudatulus")
        .comb_authors(Some("1935"), &["Schedl"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Platypus bicaudatulus Schedl (1935)")
        .species("Platypus", "bicaudatulus")
        .comb_authors(Some("1935"), &["Schedl"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Platypus bicaudatulus Schedl 1935")
        .species("Platypus", "bicaudatulus")
        .comb_authors(Some("1935"), &["Schedl"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Platypus bicaudatulus Schedl, 1935h")
        .species("Platypus", "bicaudatulus")
        .comb_authors(Some("1935"), &["Schedl"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Rotalina cultrata d'Orb. 1840")
        .species("Rotalina", "cultrata")
        .comb_authors(Some("1840"), &["d'Orb."])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Stylosanthes guianensis (Aubl.) Sw. var. robusta L.'t Mannetje")
        .infra_species("Stylosanthes", "guianensis", Rank::Variety, "robusta")
        .comb_authors(None, &["L.'t Mannetje"])
        .nothing_else();

    assert_name("Doxander vittatus entropi (Man in 't Veld & Visser, 1993)")
        .infra_species("Doxander", "vittatus", Rank::Subspecies, "entropi")
        .bas_authors(Some("1993"), &["Man in 't Veld", "Visser"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Elaeagnus triflora Roxb. var. brevilimbatus E.'t Hart")
        .infra_species("Elaeagnus", "triflora", Rank::Variety, "brevilimbatus")
        .comb_authors(None, &["E.'t Hart"])
        .nothing_else();

    assert_name("Laevistrombus guidoi (Man in't Veld & De Turck, 1998)")
        .species("Laevistrombus", "guidoi")
        .bas_authors(Some("1998"), &["Man in't Veld", "De Turck"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Strombus guidoi Man in't Veld & De Turck, 1998")
        .species("Strombus", "guidoi")
        .comb_authors(Some("1998"), &["Man in't Veld", "De Turck"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Strombus vittatus entropi Man in't Veld & Visser, 1993")
        .infra_species("Strombus", "vittatus", Rank::Subspecies, "entropi")
        .comb_authors(Some("1993"), &["Man in't Veld", "Visser"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Velutina haliotoides (Linnaeus, 1758),")
        .species("Velutina", "haliotoides")
        .bas_authors(Some("1758"), &["Linnaeus"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Hennediella microphylla (R.Br.bis) Paris")
        .species("Hennediella", "microphylla")
        .comb_authors(None, &["Paris"])
        .bas_authors(None, &["R.Br.bis"])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Pseudocercosporella endophytica Crous & H. Sm. ter")
        .species("Pseudocercosporella", "endophytica")
        .comb_authors(None, &["Crous", "H.Sm.ter"])
        .nothing_else();

    assert_name("Kudoa amazonica Velasco, Sindeaux Neto, Videira, de Cássia Silva do Nascimento, Gonçalves & Matos, 2019")
        .species("Kudoa", "amazonica")
        .comb_authors(Some("2019"), &["Velasco", "Sindeaux Neto", "Videira", "de Cássia Silva do Nascimento", "Gonçalves", "Matos"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Branchinecta papillata Rogers, de los Rios & Zuniga, 2008")
        .species("Branchinecta", "papillata")
        .comb_authors(Some("2008"), &["Rogers", "de los Rios", "Zuniga"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Gerrhonotus lazcanoi Banda-Leal, Manuel Nevárez-de los Reyes and Bryson, 2017")
        .species("Gerrhonotus", "lazcanoi")
        .comb_authors(
            Some("2017"),
            &["Banda-Leal", "Manuel Nevárez-de los Reyes", "Bryson"],
        )
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name(
        "Lynceus huentelauquensis  Sigvardt, Rogers, De los Ríos, Palero, and Olesen, 2019",
    )
    .species("Lynceus", "huentelauquensis")
    .comb_authors(
        Some("2019"),
        &["Sigvardt", "Rogers", "De los Ríos", "Palero", "Olesen"],
    )
    .code(NomCode::Zoological)
    .nothing_else();

    assert_name("Echiophis brunneus (Castro-Aguirre & Suárez de los Cobos, 1983)")
        .species("Echiophis", "brunneus")
        .bas_authors(Some("1983"), &["Castro-Aguirre", "Suárez de los Cobos"])
        .code(NomCode::Zoological)
        .nothing_else();
}
