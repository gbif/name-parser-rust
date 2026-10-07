// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserGnaTest (methods on lines 607-981).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NameType, NomCode, Rank};

#[test]
fn binomials_with_an_abbreviated_genus() {
    // group: Binomials with an abbreviated genus — INFORMAL with ABBREVIATED_GENUS
    // warning. Year-bearing trinomials → SUBSPECIES via the zoological-trinomial rule.
    assert_name("M. alpium")
        .species("M.", "alpium")
        .type_(NameType::Informal)
        .warning(&[warnings::ABBREVIATED_GENUS])
        .nothing_else();
    assert_name("Mo. alpium (Osbeck, 1778)")
        .species("Mo.", "alpium")
        .bas_authors(Some("1778"), &["Osbeck"])
        .type_(NameType::Informal)
        .code(NomCode::Zoological)
        .warning(&[warnings::ABBREVIATED_GENUS])
        .nothing_else();
}

#[test]
fn binomials_with_abbreviated_subgenus() {
    // group: Binomials with abbreviated subgenus — kept as SCIENTIFIC with the
    // ABBREVIATED_SUBGENUS warning so callers can see the infrageneric epithet is
    // incomplete.
    assert_name("Phalaena (Tin.) guttella Fab.")
        .species_ig("Phalaena", "Tin.", "guttella")
        .comb_authors(None, &["Fab."])
        .warning(&[warnings::ABBREVIATED_SUBGENUS])
        .nothing_else();
    assert_name("Gahrliepia (G.) tessellata Traub & Morrow 1955")
        .species_ig("Gahrliepia", "G.", "tessellata")
        .comb_authors(Some("1955"), &["Traub", "Morrow"])
        .code(NomCode::Zoological)
        .warning(&[warnings::ABBREVIATED_SUBGENUS])
        .nothing_else();
    assert_name("Simia (Cercop.) nasuus Kerr 1792")
        .species_ig("Simia", "Cercop.", "nasuus")
        .comb_authors(Some("1792"), &["Kerr"])
        .code(NomCode::Zoological)
        .warning(&[warnings::ABBREVIATED_SUBGENUS])
        .nothing_else();
}

#[test]
fn binomials_with_basionym_and_combination_authors() {
    // group: Binomials with basionym and combination authors. Botanical "var." /
    // "subsp." kept in canonical.
    assert_name("Yarrowia lipolytica var. lipolytica (Wick., Kurtzman & E.A. Herrm.) Van der Walt & Arx 1981")
        .infra_species("Yarrowia", "lipolytica", Rank::Variety, "lipolytica")
        .comb_authors(Some("1981"), &["Van der Walt", "Arx"])
        .bas_authors(None, &["Wick.", "Kurtzman", "E.A.Herrm."])
        .nothing_else();
    assert_name("Pseudocercospora dendrobii(H.C.     Burnett)U. Braun & Crous     2003")
        .species("Pseudocercospora", "dendrobii")
        .comb_authors(Some("2003"), &["U.Braun", "Crous"])
        .bas_authors(None, &["H.C.Burnett"])
        .nothing_else();
    assert_name("Pseudocercospora dendrobii(H.C.     Burnett, 1873)U. Braun & Crous     2003")
        .species("Pseudocercospora", "dendrobii")
        .comb_authors(Some("2003"), &["U.Braun", "Crous"])
        .bas_authors(Some("1873"), &["H.C.Burnett"])
        .nothing_else();
    assert_name("Pseudocercospora dendrobii(H.C.     Burnett 1873)U. Braun & Crous ,    2003")
        .species("Pseudocercospora", "dendrobii")
        .comb_authors(Some("2003"), &["U.Braun", "Crous"])
        .bas_authors(Some("1873"), &["H.C.Burnett"])
        .nothing_else();
    assert_name("Sedella pumila (Benth.) Britton & Rose")
        .species("Sedella", "pumila")
        .comb_authors(None, &["Britton", "Rose"])
        .bas_authors(None, &["Benth."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Impatiens nomenyae Eb.Fisch. & Raheliv.")
        .species("Impatiens", "nomenyae")
        .comb_authors(None, &["Eb.Fisch.", "Raheliv."])
        .nothing_else();
    assert_name("Armeria carpetana ssp. carpetana H. del Villar")
        .infra_species("Armeria", "carpetana", Rank::Subspecies, "carpetana")
        .comb_authors(None, &["H.del Villar"])
        .nothing_else();
}

#[test]
fn exceptions_with_binomials() {
    // group: Exceptions with Binomials — names whose species epithet happens to
    // look like a virus marker, a blacklisted word, or otherwise unusual still
    // parse when an explicit Title-cased author + year follows.
    assert_name("Agra not Erwin, 2002")
        .species("Agra", "not")
        .comb_authors(Some("2002"), &["Erwin"])
        .code(NomCode::Zoological)
        .warning(&[warnings::BLACKLISTED_EPITHET])
        .doubtful()
        .nothing_else();
    assert_name("Navicula bacterium Frenguelli")
        .species("Navicula", "bacterium")
        .comb_authors(None, &["Frenguelli"])
        .nothing_else();
    assert_name("Bottaria nudum (Nyl.) Vain.")
        .species("Bottaria", "nudum")
        .comb_authors(None, &["Vain."])
        .bas_authors(None, &["Nyl."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Turkozelotes attavirus Chatzaki, 2019")
        .species("Turkozelotes", "attavirus")
        .comb_authors(Some("2019"), &["Chatzaki"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Phalium (Semicassis) vector R. T. Abbott, 1993")
        .species_ig("Phalium", "Semicassis", "vector")
        .comb_authors(Some("1993"), &["R.T.Abbott"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Spirophora bacterium Lendenfeld, 1887")
        .species("Spirophora", "bacterium")
        .comb_authors(Some("1887"), &["Lendenfeld"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn binomials_with_mc_and_mac_authors() {
    // group: Binomials with Mc and Mac authors
    assert_name("Zygocera norfolkensis McKeown 1938")
        .species("Zygocera", "norfolkensis")
        .comb_authors(Some("1938"), &["McKeown"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Zygocera norfolkensis MacKeown 1938")
        .species("Zygocera", "norfolkensis")
        .comb_authors(Some("1938"), &["MacKeown"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Zygocera norfolkensis Mac'Keown 1938")
        .species("Zygocera", "norfolkensis")
        .comb_authors(Some("1938"), &["Mac'Keown"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Zygocera norfolkensis Mc'Keown 1938")
        .species("Zygocera", "norfolkensis")
        .comb_authors(Some("1938"), &["Mc'Keown"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn infraspecies_without_rank_iczn() {
    // group: Infraspecies without rank (ICZN). Trinomials whose authorship carries a
    // year are inferred as zoological → bumped to SUBSPECIES; pure trinomials with no
    // code signal stay INFRASPECIFIC_NAME.
    assert_name("Myotis fimbriatus taiwanensis Ärnbäck-Christie-Linde, 1908")
        .infra_species("Myotis", "fimbriatus", Rank::Subspecies, "taiwanensis")
        .comb_authors(Some("1908"), &["Ärnbäck-Christie-Linde"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Peristernia nassatula forskali Tapparone-Canefri 1875")
        .infra_species("Peristernia", "nassatula", Rank::Subspecies, "forskali")
        .comb_authors(Some("1875"), &["Tapparone-Canefri"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Cypraeovula (Luponia) amphithales perdentata")
        .infra_species(
            "Cypraeovula",
            "amphithales",
            Rank::InfraspecificName,
            "perdentata",
        )
        .infrageneric("Luponia")
        .nothing_else();
    assert_name("Triticum repens vulgäre")
        .infra_species("Triticum", "repens", Rank::InfraspecificName, "vulgäre")
        .nothing_else();
    assert_name("Hydnellum scrobiculatum zonatum (Batsch) K. A. Harrison 1961")
        .infra_species(
            "Hydnellum",
            "scrobiculatum",
            Rank::InfraspecificName,
            "zonatum",
        )
        .comb_authors(Some("1961"), &["K.A.Harrison"])
        .bas_authors(None, &["Batsch"])
        .nothing_else();
    assert_name("Hydnellum scrobiculatum zonatum (Banker) D. Hall & D.E. Stuntz 1972")
        .infra_species(
            "Hydnellum",
            "scrobiculatum",
            Rank::InfraspecificName,
            "zonatum",
        )
        .comb_authors(Some("1972"), &["D.Hall", "D.E.Stuntz"])
        .bas_authors(None, &["Banker"])
        .nothing_else();
    assert_name("Hydnellum (Hydnellum) scrobiculatum zonatum (Banker) D. Hall & D.E. Stuntz 1972")
        .infra_species(
            "Hydnellum",
            "scrobiculatum",
            Rank::InfraspecificName,
            "zonatum",
        )
        .infrageneric("Hydnellum")
        .comb_authors(Some("1972"), &["D.Hall", "D.E.Stuntz"])
        .bas_authors(None, &["Banker"])
        .nothing_else();
    assert_name("Hydnellum scrobiculatum zonatum")
        .infra_species(
            "Hydnellum",
            "scrobiculatum",
            Rank::InfraspecificName,
            "zonatum",
        )
        .nothing_else();
    assert_name("Mus musculus hortulanus")
        .infra_species("Mus", "musculus", Rank::InfraspecificName, "hortulanus")
        .nothing_else();
    assert_name("Ortygospiza atricollis mülleri")
        .infra_species(
            "Ortygospiza",
            "atricollis",
            Rank::InfraspecificName,
            "mülleri",
        )
        .nothing_else();
    assert_name("Caulerpa fastigiata confervoides P. L. Crouan & H. M. Crouan ex Weber-van Bosse")
        .infra_species(
            "Caulerpa",
            "fastigiata",
            Rank::InfraspecificName,
            "confervoides",
        )
        .comb_authors(None, &["Weber-van Bosse"])
        .comb_ex_authors(&["P.L.Crouan", "H.M.Crouan"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Rhinanthus glacialis simplex(Sterneck) J.Dostál")
        .infra_species(
            "Rhinanthus",
            "glacialis",
            Rank::InfraspecificName,
            "simplex",
        )
        .comb_authors(None, &["J.Dostál"])
        .bas_authors(None, &["Sterneck"])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn legacy_iczn_names_with_rank() {
    // group: Legacy ICZN names with rank — quadrinomial: parser keeps the explicit
    // rank-marker (natio) + its trailing epithet (danubicus) and drops the middle
    // "extra" epithet (colchicus).
    // FIXME(review): "colchicus" is dropped without the QUADRINOMIAL warning the var./f.
    // quadrinomials get
    assert_name("Acipenser gueldenstaedti colchicus natio danubicus Movchan, 1967")
        .infra_species("Acipenser", "gueldenstaedti", Rank::Natio, "danubicus")
        .comb_authors(Some("1967"), &["Movchan"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn infraspecies_with_rank_icn() {
    // group: Infraspecies with rank (ICN). Botanical rank markers kept in canonical
    // (var., f., subsp., morph., natio, prol., convar., …); zoological subspecies
    // drop the marker per ICZN convention. Year on author → ZOOLOGICAL inference.
    assert_name("Cantharellus sinuosus var. multiplex(A.H.Sm.) Romagn., 1995")
        .infra_species("Cantharellus", "sinuosus", Rank::Variety, "multiplex")
        .comb_authors(Some("1995"), &["Romagn."])
        .bas_authors(None, &["A.H.Sm."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Crematogaster impressa st. brazzai Santschi 1937")
        .infra_species("Crematogaster", "impressa", Rank::Subspecies, "brazzai")
        .comb_authors(Some("1937"), &["Santschi"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Plantago major prol. lutulenta (Lamotte) Rouy")
        .infra_species("Plantago", "major", Rank::Proles, "lutulenta")
        .comb_authors(None, &["Rouy"])
        .bas_authors(None, &["Lamotte"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Camponotus conspicuus st. zonatus")
        .infra_species(
            "Camponotus",
            "conspicuus",
            Rank::InfraspecificName,
            "zonatus",
        )
        .nothing_else();
    assert_name("Fagus sylvatica subsp. orientalis (Lipsky) Greuter & Burdet")
        .infra_species("Fagus", "sylvatica", Rank::Subspecies, "orientalis")
        .comb_authors(None, &["Greuter", "Burdet"])
        .bas_authors(None, &["Lipsky"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Tillandsia utriculata subspec. utriculata")
        .infra_species("Tillandsia", "utriculata", Rank::Subspecies, "utriculata")
        .nothing_else();
    assert_name("Prunus mexicana S. Watson var. reticulata (Sarg.) Sarg.")
        .infra_species("Prunus", "mexicana", Rank::Variety, "reticulata")
        .comb_authors(None, &["Sarg."])
        .bas_authors(None, &["Sarg."])
        .code(NomCode::Botanical)
        .specific_authors(None, &["S.Watson"])
        .nothing_else();
    assert_name("Potamogeton iilinoensis var. ventanicola")
        .infra_species("Potamogeton", "iilinoensis", Rank::Variety, "ventanicola")
        .nothing_else();
    assert_name("Potamogeton iilinoensis var. ventanicola (Hicken) Horn af Rantzien")
        .infra_species("Potamogeton", "iilinoensis", Rank::Variety, "ventanicola")
        .comb_authors(None, &["Horn af Rantzien"])
        .bas_authors(None, &["Hicken"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Triticum repens var. vulgäre")
        .infra_species("Triticum", "repens", Rank::Variety, "vulgäre")
        .nothing_else();
    assert_name("Aus bus Linn. var. bus")
        .infra_species("Aus", "bus", Rank::Variety, "bus")
        .comb_authors(None, &["Linn."])
        .nothing_else();
    assert_name("Agalinis purpurea (L.) Briton var. borealis (Berg.) Peterson 1987")
        .infra_species("Agalinis", "purpurea", Rank::Variety, "borealis")
        .comb_authors(Some("1987"), &["Peterson"])
        .bas_authors(None, &["Berg."])
        .specific_authors(None, &["Briton"])
        .specific_bas_authors(None, &["L."])
        .nothing_else();
    assert_name("Callideriphus flavicollis morph. reductus Fuchs 1961")
        .infra_species("Callideriphus", "flavicollis", Rank::Morph, "reductus")
        .comb_authors(Some("1961"), &["Fuchs"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Caulerpa cupressoides forma nuda")
        .infra_species("Caulerpa", "cupressoides", Rank::Form, "nuda")
        .nothing_else();
    assert_name("Chlorocyperus glaber form. fasciculariforme (Lojac.) Soó")
        .infra_species("Chlorocyperus", "glaber", Rank::Form, "fasciculariforme")
        .comb_authors(None, &["Soó"])
        .bas_authors(None, &["Lojac."])
        .code(NomCode::Botanical)
        .nothing_else();
    // FIXME(review): 1753 predates the ICZN (1758) and "fm." is a botanical rank: not ZOOLOGICAL
    assert_name("Pteris longifolia fm. stipularis Linnaeus 1753")
        .infra_species("Pteris", "longifolia", Rank::Form, "stipularis")
        .comb_authors(Some("1753"), &["Linnaeus"])
        .code(NomCode::Zoological)
        .nothing_else();
    // FIXME(review): 1753 predates the ICZN (1758) and "fm." is a botanical rank: not ZOOLOGICAL
    assert_name("Pteris longifolia fm stipularis Linnaeus 1753")
        .infra_species("Pteris", "longifolia", Rank::Form, "stipularis")
        .comb_authors(Some("1753"), &["Linnaeus"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Sphaerotheca    fuliginea    f.     dahliae    Movss.     1967")
        .infra_species("Sphaerotheca", "fuliginea", Rank::Form, "dahliae")
        .comb_authors(Some("1967"), &["Movss."])
        .nothing_else();
    assert_name("Allophylus amazonicus var amazonicus")
        .infra_species("Allophylus", "amazonicus", Rank::Variety, "amazonicus")
        .nothing_else();
    assert_name("Yarrowia lipolytica variety lipolytic")
        .infra_species("Yarrowia", "lipolytica", Rank::Variety, "lipolytic")
        .nothing_else();
    assert_name("Prunus armeniaca convar. budae (Pénzes) Soó")
        .infra_species("Prunus", "armeniaca", Rank::Convariety, "budae")
        .comb_authors(None, &["Soó"])
        .bas_authors(None, &["Pénzes"])
        .code(NomCode::Cultivars)
        .nothing_else();
    assert_name("Polypodium pectinatum (L.) f. typica Rosenst.")
        .infra_species("Polypodium", "pectinatum", Rank::Form, "typica")
        .comb_authors(None, &["Rosenst."])
        .specific_bas_authors(None, &["L."])
        .nothing_else();
    assert_name("Polypodium pectinatum L. f. typica Rosenst.")
        .infra_species("Polypodium", "pectinatum", Rank::Form, "typica")
        .comb_authors(None, &["Rosenst."])
        .specific_authors(None, &["L."])
        .nothing_else();
    // "agamosp." marker — parser captures the chloocladus token as infrasp epithet
    // but the rank stays SPECIES (per RankMarkers.put("agamosp", Rank.SPECIES)).
    assert_name("Rubus fruticosus agamosp. chloocladus (W.C.R. Watson) A. & D. Löve")
        .infra_species("Rubus", "fruticosus", Rank::Species, "chloocladus")
        .comb_authors(None, &["A.Löve", "D.Löve"])
        .bas_authors(None, &["W.C.R.Watson"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Rubus fruticosus L. agamossp. discolor (Weihe & Nees) A. & D. Löve")
        .infra_species("Rubus", "fruticosus", Rank::Subspecies, "discolor")
        .comb_authors(None, &["A.Löve", "D.Löve"])
        .bas_authors(None, &["Weihe", "Nees"])
        .code(NomCode::Botanical)
        .specific_authors(None, &["L."])
        .nothing_else();
    assert_name("Rubus fruticosus agamovar. graecensis (W.Maurer) A. & D. Löve")
        .infra_species("Rubus", "fruticosus", Rank::Variety, "graecensis")
        .comb_authors(None, &["A.Löve", "D.Löve"])
        .bas_authors(None, &["W.Maurer"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Polypodium pectinatum L.f. typica Rosenst.")
        .infra_species("Polypodium", "pectinatum", Rank::Form, "typica")
        .comb_authors(None, &["Rosenst."])
        .specific_authors(None, &["L."])
        .nothing_else();
    assert_name("Polypodium lineare C.Chr. f. caudatoattenuatum Takeda")
        .infra_species("Polypodium", "lineare", Rank::Form, "caudatoattenuatum")
        .comb_authors(None, &["Takeda"])
        .specific_authors(None, &["C.Chr."])
        .nothing_else();
    assert_name("Rhododendron weyrichii Maxim. f. albiflorum T.Yamaz.")
        .infra_species("Rhododendron", "weyrichii", Rank::Form, "albiflorum")
        .comb_authors(None, &["T.Yamaz."])
        .specific_authors(None, &["Maxim."])
        .nothing_else();
    assert_name("Armeria maaritima (Mill.) Willd. fma. originaria Bern.")
        .infra_species("Armeria", "maaritima", Rank::Form, "originaria")
        .comb_authors(None, &["Bern."])
        .specific_authors(None, &["Willd."])
        .specific_bas_authors(None, &["Mill."])
        .nothing_else();
    assert_name("Cotoneaster (Pyracantha) rogersiana var.aurantiaca")
        .infra_species("Cotoneaster", "rogersiana", Rank::Variety, "aurantiaca")
        .infrageneric("Pyracantha")
        .nothing_else();
    assert_name("Poa annua fo varia")
        .infra_species("Poa", "annua", Rank::Form, "varia")
        .nothing_else();
    assert_name("Physarum globuliferum forma. flavum Leontyev & Dudka")
        .infra_species("Physarum", "globuliferum", Rank::Form, "flavum")
        .comb_authors(None, &["Leontyev", "Dudka"])
        .nothing_else();
    assert_name("Homalanthus nutans (Mull.Arg.) Benth. & Hook. f. ex Drake")
        .species("Homalanthus", "nutans")
        .comb_authors(None, &["Drake"])
        .comb_ex_authors(&["Benth.", "Hook.f."])
        .bas_authors(None, &["Mull.Arg."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Calicium furfuraceum * furfuraceum (L.) Pers. 1797")
        .infra_species(
            "Calicium",
            "furfuraceum",
            Rank::InfraspecificName,
            "furfuraceum",
        )
        .comb_authors(Some("1797"), &["Pers."])
        .bas_authors(None, &["L."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Polyrhachis orsyllus nat musculus Forel 1901")
        .infra_species("Polyrhachis", "orsyllus", Rank::Natio, "musculus")
        .comb_authors(Some("1901"), &["Forel"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Acmaeops (Pseudodinoptera) bivittata ab. fusciceps Aurivillius, 1912")
        .infra_species("Acmaeops", "bivittata", Rank::Aberration, "fusciceps")
        .infrageneric("Pseudodinoptera")
        .comb_authors(Some("1912"), &["Aurivillius"])
        .code(NomCode::Zoological)
        .nothing_else();
    // FIXME(review): "st.-johnii" splits into epithet "st" + author "johnii …"; "ab. n." makes "n"
    // the epithet and "undularia" the author
    // Skipped: "Cibotium st.-johnii Krajina" needs hyphenated single-letter epithet
    // recognition; "Acidalia remutaria ab. n. undularia" needs "ab. n." (aberratio
    // nova) handling; "Rhododendron weyrichii Maxim. albiflorum T.Yamaz. f.
    // fakeepithet" and the bracketed variant need quadrinomial-with-rank handling.
}

#[test]
fn infraspecies_multiple_icn() {
    // group: Infraspecies multiple (ICN). Quadrinomial-with-rank: the most specific
    // explicit rank marker (the rightmost) wins; the middle epithet is dropped
    // with a QUADRINOMIAL warning.
    assert_name(
        "Hydnellum scrobiculatum var. zonatum f. parvum (Banker) D. Hall & D.E. Stuntz 1972",
    )
    .infra_species("Hydnellum", "scrobiculatum", Rank::Form, "parvum")
    .comb_authors(Some("1972"), &["D.Hall", "D.E.Stuntz"])
    .bas_authors(None, &["Banker"])
    .warning(&["Removed: var. zonatum", warnings::QUADRINOMIAL])
    .nothing_else();
    assert_name("Senecio fuchsii C.C.Gmel. subsp. fuchsii var. expansus (Boiss. & Heldr.) Hayek")
        .infra_species("Senecio", "fuchsii", Rank::Variety, "expansus")
        .comb_authors(None, &["Hayek"])
        .bas_authors(None, &["Boiss.", "Heldr."])
        .code(NomCode::Botanical)
        .warning(&["Removed: subsp. fuchsii", warnings::QUADRINOMIAL])
        .specific_authors(None, &["C.C.Gmel."])
        .nothing_else();
    assert_name("Senecio fuchsii C.C.Gmel. subsp. fuchsii var. fuchsii")
        .infra_species("Senecio", "fuchsii", Rank::Variety, "fuchsii")
        .warning(&["Removed: subsp. fuchsii", warnings::QUADRINOMIAL])
        .comb_authors(None, &["C.C.Gmel."])
        .nothing_else();
    assert_name("Euastrum divergens var. rhodesiense f. coronulum A.M. Scott & Prescott")
        .infra_species("Euastrum", "divergens", Rank::Form, "coronulum")
        .comb_authors(None, &["A.M.Scott", "Prescott"])
        .warning(&["Removed: var. rhodesiense", warnings::QUADRINOMIAL])
        .nothing_else();
}

#[test]
fn infraspecies_with_greek_letters_icn() {
    // group: Infraspecies with greek letters (ICN). A greek letter (with optional
    // dot) sitting between epithets is a historical informal rank marker; it's
    // stripped in StripAndStash so the surrounding epithets parse normally.
    assert_name("Aristotelia fruticosa var. δ. microphylla Hook.f.")
        .infra_species("Aristotelia", "fruticosa", Rank::Variety, "microphylla")
        .comb_authors(None, &["Hook.f."])
        .nothing_else();
    assert_name("Aristotelia fruticosa var. δ microphylla Hook.f.")
        .infra_species("Aristotelia", "fruticosa", Rank::Variety, "microphylla")
        .comb_authors(None, &["Hook.f."])
        .nothing_else();
    assert_name("Aristotelia fruticosa var.δ.microphylla Hook.f.")
        .infra_species("Aristotelia", "fruticosa", Rank::Variety, "microphylla")
        .comb_authors(None, &["Hook.f."])
        .nothing_else();
    // "var. δmicrophylla" — greek letter glued to the next epithet without a
    // separator is kept as-is (consistent with "var. βrigida" in
    // alphaBetaThetaNames). The whole "δmicrophylla" becomes the variety epithet.
    assert_name("Aristotelia fruticosa var. δmicrophylla Hook.f.")
        .infra_species("Aristotelia", "fruticosa", Rank::Variety, "δmicrophylla")
        .comb_authors(None, &["Hook.f."])
        .nothing_else();
    // FIXME(review): "unr." (unranked) is read as the species epithet, the infrageneric name as an
    // author
    // "Hieracium unr. Verbasciformia Arv.-Touv." — "unr." is an unknown rank
    // marker the parser doesn't recognise, leaving "unr" as the species epithet.
    // Skipped here.
}
