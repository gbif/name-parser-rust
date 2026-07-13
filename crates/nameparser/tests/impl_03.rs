// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserImplTest (methods on lines 720-1283).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NamePart, NameType, NomCode, Rank};

#[test]
fn capital_monomial() {
    // https://github.com/CatalogueOfLife/checklistbank/issues/1262
    assert_name("XENACOELOMORPHA")
        .monomial("Xenacoelomorpha")
        .nothing_else();
}

#[test]
fn infra_species() {
    // bad rank given
    assert_name_rank(
        "Poa pratensis subsp. anceps (Gaudin) Dumort., 1824",
        Rank::Species,
    )
    .infra_species("Poa", "pratensis", Rank::Subspecies, "anceps")
    .bas_authors(None, &["Gaudin"])
    .comb_authors(Some("1824"), &["Dumort."])
    .warning(&[warnings::SUBSPECIES_ASSIGNED])
    .nothing_else();

    assert_name("Abies alba ssp. alpina Mill.")
        .infra_species("Abies", "alba", Rank::Subspecies, "alpina")
        .comb_authors(None, &["Mill."])
        .nothing_else();

    assert_name("Festuca ovina L. subvar. gracilis Hackel")
        .infra_species("Festuca", "ovina", Rank::Subvariety, "gracilis")
        .comb_authors(None, &["Hackel"])
        .nothing_else();

    assert_name("Pseudomonas syringae pv. aceris (Ark, 1939) Young, Dye & Wilkie, 1978")
        .infra_species("Pseudomonas", "syringae", Rank::Pathovar, "aceris")
        .comb_authors(Some("1978"), &["Young", "Dye", "Wilkie"])
        .bas_authors(Some("1939"), &["Ark"])
        .code(NomCode::Bacterial)
        .nothing_else();

    assert_name("Agaricus compactus sarcocephalus (Fr.) Fr. ")
        .infra_species(
            "Agaricus",
            "compactus",
            Rank::InfraspecificName,
            "sarcocephalus",
        )
        .comb_authors(None, &["Fr."])
        .bas_authors(None, &["Fr."])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Baccharis microphylla Kunth var. rhomboidea Wedd. ex Sch. Bip. (nom. nud.)")
        .infra_species("Baccharis", "microphylla", Rank::Variety, "rhomboidea")
        .comb_authors(None, &["Sch.Bip."])
        .comb_ex_authors(&["Wedd."])
        .nom_note("nom. nud.")
        .nothing_else();

    // Warnings.REMOVED_PREFIX + "subsp. pallidotegula B.Boivin"
    let removed_prefix_msg = format!("{}subsp. pallidotegula B.Boivin", warnings::REMOVED_PREFIX);
    assert_name("Achillea millefolium subsp. pallidotegula B. Boivin var. pallidotegula")
        .infra_species("Achillea", "millefolium", Rank::Variety, "pallidotegula")
        .warning(&[warnings::QUADRINOMIAL, removed_prefix_msg.as_str()])
        .nothing_else();

    assert_name_rank(
        "Achillea millefolium var. pallidotegula",
        Rank::InfraspecificName,
    )
    .infra_species("Achillea", "millefolium", Rank::Variety, "pallidotegula")
    .nothing_else();

    assert_name_rank("Monograptus turriculatus mut. minor", Rank::Mutatio)
        .infra_species("Monograptus", "turriculatus", Rank::Mutatio, "minor")
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn ex_authors() {
    assert_name("Acacia truncata (Burm. f.) hort. ex Hoffmanns.")
        .species("Acacia", "truncata")
        .bas_authors(None, &["Burm.f."])
        .comb_ex_authors(&["hort."])
        .comb_authors(None, &["Hoffmanns."])
        .code(NomCode::Botanical)
        .nothing_else();

    // In botany (99% of ex author use) the ex author comes first, see https://en.wikipedia.org/wiki/Author_citation_(botany)#Usage_of_the_term_.22ex.22
    assert_name("Gymnocalycium eurypleurumn Plesn¡k ex F.Ritter")
        .species("Gymnocalycium", "eurypleurumn")
        .comb_authors(None, &["F.Ritter"])
        .comb_ex_authors(&["Plesnik"])
        .warning(&[warnings::HOMOGLYHPS]) // the ¡ in Plesn¡k is not a regular i
        .nothing_else();

    assert_name("Abutilon bastardioides Baker f. ex Rose")
        .species("Abutilon", "bastardioides")
        .comb_authors(None, &["Rose"])
        .comb_ex_authors(&["Baker f."])
        .nothing_else();

    assert_name("Baccharis microphylla Kunth var. rhomboidea Wedd. ex Sch. Bip. (nom. nud.)")
        .infra_species("Baccharis", "microphylla", Rank::Variety, "rhomboidea")
        .comb_authors(None, &["Sch.Bip."])
        .comb_ex_authors(&["Wedd."])
        .nom_note("nom. nud.")
        .nothing_else();

    // hort. = from hortulanorum (“of gardens”), the name was used in cultivation (nurseries, gardens, horticultural trade)
    //         often without valid scientific publication sometimes applied loosely or incorrectly
    // ex Dallim. = William Dallimore later validly published the name
    assert_name("Abies brevifolia hort. ex Dallim.")
        .species("Abies", "brevifolia")
        .comb_ex_authors(&["hort."])
        .comb_authors(None, &["Dallim."])
        .nothing_else();

    assert_name("Abies brevifolia cv. ex Dallim.")
        .species("Abies", "brevifolia")
        .comb_ex_authors(&["hort."])
        .comb_authors(None, &["Dallim."])
        .nothing_else();

    assert_name("Abutilon ×hybridum cv. ex Voss")
        .species("Abutilon", "hybridum")
        .notho(&[NamePart::Specific])
        .comb_ex_authors(&["hort."])
        .comb_authors(None, &["Voss"])
        .nothing_else();

    // "Abutilon bastardioides Baker f. ex Rose"
    // "Aukuba ex Koehne 'Thunb'   "
    // "Crepinella subgen. Marchal ex Oliver  "
    // "Echinocereus sect. Triglochidiata ex Bravo"
    // "Hadrolaelia sect. Sophronitis ex Chiron & V.P.Castro"
}

#[test]
fn four_parted_names() {
    assert_name("Poa pratensis kewensis primula (L.) Rouy, 1913")
        .infra_species("Poa", "pratensis", Rank::InfrasubspecificName, "primula")
        .comb_authors(Some("1913"), &["Rouy"])
        .bas_authors(None, &["L."])
        .nothing_else();

    assert_name("Bombus sichelii alticola latofasciatus")
        .infra_species(
            "Bombus",
            "sichelii",
            Rank::InfrasubspecificName,
            "latofasciatus",
        )
        .nothing_else();

    assert_name("Acipenser gueldenstaedti colchicus natio danubicus Movchan, 1967")
        .infra_species("Acipenser", "gueldenstaedti", Rank::Natio, "danubicus")
        .comb_authors(Some("1967"), &["Movchan"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Cymbella cistula var. sinus regis")
        .infra_species("Cymbella", "cistula", Rank::Variety, "sinus")
        .partial("regis")
        .nothing_else();
}

#[test]
fn monomial() {
    assert_name("Animalia").monomial("Animalia").nothing_else();
    assert_name("Polychaeta")
        .monomial("Polychaeta")
        .nothing_else();
    assert_name("Chrysopetalidae")
        .monomial("Chrysopetalidae")
        .nothing_else();
    assert_name_code("Chrysopetalidae", NomCode::Zoological)
        .monomial_rank("Chrysopetalidae", Rank::Family)
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Acripeza Guérin-Ménéville 1838")
        .monomial("Acripeza")
        .comb_authors(Some("1838"), &["Guérin-Ménéville"])
        .code(NomCode::Zoological)
        .nothing_else();
    // https://github.com/gbif/name-parser/issues/98
    assert_name("Salmonidae Jarocki or Schinz, 1822")
        .monomial("Salmonidae")
        .comb_authors(Some("1822"), &["Jarocki or Schinz"])
        .code(NomCode::Zoological)
        .doubtful()
        .warning(&[warnings::UNCERTAIN_AUTHORSHIP])
        .nothing_else();
}

#[test]
fn in_references() {
    assert_name("Amathia tricornis Busk ms in Chimonides, 1987")
        .species("Amathia", "tricornis")
        .comb_authors(Some("1987"), &["Busk"])
        .published_in("Chimonides, 1987")
        .nom_note("ms")
        .manuscript()
        .nothing_else();

    assert_name("Xolisma turquini Small apud Britton & Wilson")
        .species("Xolisma", "turquini")
        .comb_authors(None, &["Small"])
        .published_in("Britton & Wilson")
        .nothing_else();

    assert_name("Negundo aceroides var. violaceum G.Kirchn. in Petzold & G.Kirchn.")
        .infra_species("Negundo", "aceroides", Rank::Variety, "violaceum")
        .comb_authors(None, &["G.Kirchn."])
        .published_in("Petzold & G.Kirchn.")
        .nothing_else();

    assert_name("Abies denheyeri Eghbalian, Khanjani and Ueckermann in Eghbalian, Khanjani & Ueckermann, 2017")
        .species("Abies", "denheyeri")
        .comb_authors(Some("2017"), &["Eghbalian", "Khanjani", "Ueckermann"])
        .published_in("Eghbalian, Khanjani & Ueckermann, 2017")
        .nothing_else();

    assert_name("Mica Budde-Lund in Voeltzkow, 1908")
        .monomial("Mica")
        .comb_authors(Some("1908"), &["Budde-Lund"])
        .published_in("Voeltzkow, 1908")
        .nothing_else();
}

#[test]
fn supra_generic_ipni() {
    assert_name("Poaceae subtrib. Scolochloinae Soreng")
        .monomial_rank("Scolochloinae", Rank::Subtribe)
        .comb_authors(None, &["Soreng"])
        .nothing_else();

    assert_name("subtrib. Scolochloinae Soreng")
        .monomial_rank("Scolochloinae", Rank::Subtribe)
        .comb_authors(None, &["Soreng"])
        .nothing_else();
}

#[test]
fn infra_generic() {
    // default to botanical ranks for sections
    assert_name("Pinus suprasect. Taeda")
        .infrageneric_at("Pinus", Rank::SupersectionBotany, "Taeda")
        .code(NomCode::Botanical)
        .nothing_else();

    // Authorship placed BEFORE the infrageneric marker is the genus author, captured as the
    // generic authorship; "Salimori" is the (unauthored) sectional epithet, not an author.
    assert_name("Cordia (Adans.) Kuntze sect. Salimori")
        .infrageneric_at("Cordia", Rank::SectionBotany, "Salimori")
        .generic_bas_authors(None, &["Adans."])
        .generic_authors(None, &["Kuntze"])
        .code(NomCode::Botanical)
        .nothing_else();

    // "div." between a botanical genus and a capitalised epithet is the botanical
    // divisio infrageneric rank (not the zoological suprageneric division).
    assert_name("Rosa div. Caninae Lindl.")
        .infrageneric_at("Rosa", Rank::DivisionBotany, "Caninae")
        .comb_authors(None, &["Lindl."])
        .code(NomCode::Botanical)
        .nothing_else();

    // with zoological code it is an impossible name! The zoological section is above family rank and cannot be enclosed in a genus
    assert_name_code("Pinus suprasect. Taeda", NomCode::Zoological)
        .infrageneric_at("Pinus", Rank::SupersectionBotany, "Taeda")
        .code(NomCode::Botanical)
        .warning(&[warnings::CODE_MISMATCH])
        .nothing_else();

    // result in the zoological rank equivalent if the code is given!
    assert_name_code("sect. Taeda", NomCode::Zoological)
        .monomial_rank("Taeda", Rank::SectionZoology)
        .code(NomCode::Zoological)
        .nothing_else();

    // IPNI notho ranks: https://github.com/gbif/name-parser/issues/15
    assert_name("Aeonium nothosect. Leugalonium")
        .infrageneric_at("Aeonium", Rank::SectionBotany, "Leugalonium")
        .notho(&[NamePart::Infrageneric])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Narcissus nothoser. Dubizettae")
        .infrageneric_at("Narcissus", Rank::SeriesBotany, "Dubizettae")
        .notho(&[NamePart::Infrageneric])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Serapias nothosubsect. Pladiopetalae")
        .infrageneric_at("Serapias", Rank::SubsectionBotany, "Pladiopetalae")
        .notho(&[NamePart::Infrageneric])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Rubus nothosubgen. Cylarubus")
        .infrageneric_at("Rubus", Rank::Subgenus, "Cylarubus")
        .notho(&[NamePart::Infrageneric])
        .nothing_else();

    assert_name_rank("Arrhoges (Antarctohoges)", Rank::Subgenus)
        .infrageneric_at("Arrhoges", Rank::Subgenus, "Antarctohoges")
        .nothing_else();

    // Java: `.infraGeneric(null, Rank.SUBGENUS, "Polygonum")` — a null genus. The DSL's
    // `infrageneric_at` requires a real genus `&str`, so the null-genus case is reproduced by
    // combining the single-arg epithet check with an explicit rank check; `nothing_else()`
    // then still confirms genus/uninomial/specific/infraspecific are all absent.
    assert_name_rank("Polygonum", Rank::Subgenus)
        .infrageneric("Polygonum")
        .rank(Rank::Subgenus)
        .nothing_else();

    assert_name("subgen. Trematostoma Sacc.")
        .infrageneric("Trematostoma")
        .rank(Rank::Subgenus)
        .comb_authors(None, &["Sacc."])
        .nothing_else();

    assert_name("Echinocereus sect. Triglochidiata Bravo")
        .infrageneric_at("Echinocereus", Rank::SectionBotany, "Triglochidiata")
        .comb_authors(None, &["Bravo"])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Zignoella subgen. Trematostoma Sacc.")
        .infrageneric_at("Zignoella", Rank::Subgenus, "Trematostoma")
        .comb_authors(None, &["Sacc."])
        .nothing_else();

    assert_name("Polygonum subgen. Bistorta (L.) Zernov")
        .infrageneric_at("Polygonum", Rank::Subgenus, "Bistorta")
        .comb_authors(None, &["Zernov"])
        .bas_authors(None, &["L."])
        .code(NomCode::Botanical)
        .nothing_else();

    // "Arrhoges (Antarctohoges)" without an explicit rank: the parenthesised single word is a
    // subgenus (a genus can't carry a parenthesised basionym author), not authorship. Parses as
    // genus + infrageneric epithet at the generic infrageneric rank.
    assert_name("Arrhoges (Antarctohoges)")
        .infrageneric_at("Arrhoges", Rank::InfragenericName, "Antarctohoges")
        .nothing_else();

    assert_name("Festuca subg. Schedonorus (P. Beauv. ) Peterm.")
        .infrageneric_at("Festuca", Rank::Subgenus, "Schedonorus")
        .comb_authors(None, &["Peterm."])
        .bas_authors(None, &["P.Beauv."])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Catapodium subg.Agropyropsis  Trab.")
        .infrageneric_at("Catapodium", Rank::Subgenus, "Agropyropsis")
        .comb_authors(None, &["Trab."])
        .nothing_else();

    assert_name(" Gnaphalium subg. Laphangium Hilliard & B. L. Burtt")
        .infrageneric_at("Gnaphalium", Rank::Subgenus, "Laphangium")
        .comb_authors(None, &["Hilliard", "B.L.Burtt"])
        .nothing_else();

    assert_name("Woodsiaceae (Hooker) Herter")
        .monomial_rank("Woodsiaceae", Rank::Family)
        .comb_authors(None, &["Herter"])
        .bas_authors(None, &["Hooker"])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn not_names() {
    assert_name("Diatrypella favacea var. favacea (Fr.) Ces. & De Not.")
        .infra_species("Diatrypella", "favacea", Rank::Variety, "favacea")
        .comb_authors(None, &["Ces.", "De Not."])
        .bas_authors(None, &["Fr."])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Protoventuria rosae (De Not.) Berl. & Sacc.")
        .species("Protoventuria", "rosae")
        .comb_authors(None, &["Berl.", "Sacc."])
        .bas_authors(None, &["De Not."])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Hormospora De Not.")
        .monomial("Hormospora")
        .comb_authors(None, &["De Not."])
        .nothing_else();
}

// autonymAuthorship (Java lines 1108-1141). Not a DSL test — it calls `parser.parse(...)`
// directly and checks exact strings from `NameFormatter.authorshipComplete(...)` / `.canonical(...)`,
// now ported to `ParsedName::authorship_complete` / `::canonical_name` (see `src/format.rs`).
// http://dev.gbif.org/issues/browse/POR-2459,
// https://www.iapt-taxon.org/icbn/frameset/0026Ch3Sec3a022.htm (ICN Art. 26),
// https://code.iczn.org/types-in-the-species-group/article-72-general-provisions/ (ICZN Art. 72).
#[test]
fn autonym_authorship() {
    use nameparser::parse_name as parse;

    // botanical: species author after the species epithet, none after the autonym
    let acer = parse(
        "Acer rubrum L. var. rubrum",
        None,
        None,
        Some(NomCode::Botanical),
    )
    .unwrap();
    assert_eq!(acer.specific_epithet.as_deref(), Some("rubrum"));
    assert_eq!(acer.infraspecific_epithet.as_deref(), Some("rubrum"));
    assert!(acer.is_autonym());
    assert_eq!(acer.authorship_complete().as_deref(), Some("L."));
    assert_eq!(
        acer.canonical_name().as_deref(),
        Some("Acer rubrum L. var. rubrum")
    );

    // botanical recombination with basionym + combination author before the marker
    let trim = parse(
        "Trimezia spathata (Klatt) Baker subsp. spathata",
        None,
        None,
        None,
    )
    .unwrap();
    assert!(trim.is_autonym());
    assert_eq!(trim.specific_epithet.as_deref(), Some("spathata"));
    assert_eq!(trim.infraspecific_epithet.as_deref(), Some("spathata"));
    assert_eq!(trim.infrageneric_epithet, None);
    assert_eq!(trim.code, Some(NomCode::Botanical));
    assert_eq!(trim.authorship_complete().as_deref(), Some("(Klatt) Baker"));
    assert_eq!(
        trim.canonical_name().as_deref(),
        Some("Trimezia spathata (Klatt) Baker subsp. spathata")
    );

    // zoological autonym: author at the very end, no rank marker
    let vul = parse(
        "Vulpes vulpes vulpes Linnaeus, 1758",
        None,
        None,
        Some(NomCode::Zoological),
    )
    .unwrap();
    assert!(vul.is_autonym());
    assert_eq!(
        vul.canonical_name().as_deref(),
        Some("Vulpes vulpes vulpes Linnaeus, 1758")
    );

    // botanical autonym with no author renders cleanly without one
    let bare = parse(
        "Acer rubrum var. rubrum",
        None,
        None,
        Some(NomCode::Botanical),
    )
    .unwrap();
    assert!(bare.is_autonym());
    assert_eq!(
        bare.canonical_name().as_deref(),
        Some("Acer rubrum var. rubrum")
    );
}

#[test]
fn unparsable_placeholder() {
    assert_unparsable("Mollusca not assigned", NameType::Placeholder);
    assert_unparsable("[unassigned] Cladobranchia", NameType::Placeholder);
    assert_unparsable("Biota incertae sedis", NameType::Placeholder);
    assert_unparsable("Unaccepted", NameType::Placeholder);
    assert_unparsable(
        "uncultured Verrucomicrobiales bacterium",
        NameType::Placeholder,
    );
    assert_unparsable("uncultured Vibrio sp.", NameType::Placeholder);
    assert_unparsable("uncultured virus", NameType::Placeholder);
    // ITIS placeholders:
    assert_unparsable("Temp dummy name", NameType::Placeholder);
    // https://de.wikipedia.org/wiki/N._N.
    assert_unparsable("N.N.", NameType::Placeholder);
    assert_unparsable("N.N. (e.g., Breoghania)", NameType::Placeholder);
    assert_unparsable("N.N. (Chitinivorax)", NameType::Placeholder);
    assert_unparsable("N.n. (Chitinivorax)", NameType::Placeholder);

    // https://github.com/gbif/checklistbank/issues/48
    assert_unparsable("Gen.nov. sp.nov.", NameType::Other);
    assert_unparsable("Gen.nov.", NameType::Other);

    // "Aster indet." parses to genus "Aster" with a missing (indeterminate) specific epithet.
    // The DSL's species()/indet() helpers take the epithet as a plain `&str` (no way to express
    // "epithet explicitly absent"), so this one case is asserted directly against the parsed
    // fields instead of through the assert_name(...).nothing_else() chain — mirrors Java's
    // `.species("Aster", null).type(NameType.INFORMAL).warning(Warnings.INDETERMINED)`.
    let aster = nameparser::parse_name("Aster indet.", None, None, None)
        .expect("`Aster indet.` should parse");
    assert!(aster.uninomial.is_none());
    assert_eq!(aster.genus.as_deref(), Some("Aster"));
    assert!(aster.infrageneric_epithet.is_none());
    assert!(aster.specific_epithet.is_none());
    assert!(aster.infraspecific_epithet.is_none());
    assert_eq!(aster.rank, Rank::Species);
    assert_eq!(aster.type_, NameType::Informal);
    assert_eq!(aster.warnings, vec![warnings::INDETERMINED.to_string()]);

    assert_unparsable("Asteraceae incertae sedis", NameType::Placeholder);
    assert_unparsable("unassigned Abies", NameType::Placeholder);
    assert_unparsable("Unident-Boraginaceae", NameType::Placeholder);
    assert_unparsable("Unident", NameType::Placeholder);
    assert_unparsable("IncertaeSedis justi", NameType::Placeholder);
    // IPNI underscore-joined placeholder, https://github.com/CatalogueOfLife (name-parser v4 item 3)
    assert_unparsable("Incertae_sedis", NameType::Placeholder);
    assert_unparsable_rank("Incertae_sedis", Rank::Family, NameType::Placeholder);
}

#[test]
fn placeholder() {
    assert_name(
        "denheyeri Eghbalian, Khanjani and Ueckermann in Eghbalian, Khanjani & Ueckermann, 2017",
    )
    .species("?", "denheyeri")
    .comb_authors(Some("2017"), &["Eghbalian", "Khanjani", "Ueckermann"])
    .type_(NameType::Placeholder)
    .published_in("Eghbalian, Khanjani & Ueckermann, 2017")
    .warning(&[warnings::MISSING_GENUS])
    .nothing_else();

    assert_name("\"? gryphoidis")
        .species("?", "gryphoidis")
        .type_(NameType::Placeholder)
        .nothing_else();

    assert_name("\"? gryphoidis (Bourguignat 1870) Schoepf. 1909")
        .species("?", "gryphoidis")
        .bas_authors(Some("1870"), &["Bourguignat"])
        .comb_authors(Some("1909"), &["Schoepf."])
        .type_(NameType::Placeholder)
        .nothing_else();

    assert_name("Missing penchinati Bourguignat, 1870")
        .species("?", "penchinati")
        .comb_authors(Some("1870"), &["Bourguignat"])
        .type_(NameType::Placeholder)
        .code(NomCode::Zoological)
        .nothing_else();

    // A leading double question mark is not a "? epithet" missing-genus placeholder — the
    // whole thing is junk, not a name. It must be fully unparsable (OTHER), not coerced into
    // a doubtful "? ? not a name" INFORMAL result.
    assert_unparsable("?? not a name 12345 ##", NameType::Other);
    assert_unparsable("?? not a name", NameType::Other);
}

#[test]
fn sanctioned() {
    // sanctioning authors not supported
    // https://github.com/GlobalNamesArchitecture/gnparser/issues/409
    assert_name("Boletus versicolor L. : Fr.")
        .species("Boletus", "versicolor")
        .comb_authors(None, &["L."])
        .sanct_author("Fr.")
        .nothing_else();

    assert_name("Agaricus compactus sarcocephalus (Fr. : Fr.) Fr. ")
        .infra_species(
            "Agaricus",
            "compactus",
            Rank::InfraspecificName,
            "sarcocephalus",
        )
        .comb_authors(None, &["Fr."])
        .bas_authors(None, &["Fr."])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Agaricus compactus sarcocephalus (Fr. : Fr.) Fr. ")
        .infra_species(
            "Agaricus",
            "compactus",
            Rank::InfraspecificName,
            "sarcocephalus",
        )
        .comb_authors(None, &["Fr."])
        .bas_authors(None, &["Fr."])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Agaricus ericetorum Pers. : Fr.")
        .species("Agaricus", "ericetorum")
        .comb_authors(None, &["Pers."])
        .sanct_author("Fr.")
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn nothotaxa() {
    // https://github.com/GlobalNamesArchitecture/gnparser/issues/410
    assert_name("Iris germanica nothovar. florentina")
        .infra_species("Iris", "germanica", Rank::Variety, "florentina")
        .notho(&[NamePart::Infraspecific])
        .nothing_else();

    assert_name("Abies alba var. ×alpina L.")
        .infra_species("Abies", "alba", Rank::Variety, "alpina")
        .notho(&[NamePart::Infraspecific])
        .comb_authors(None, &["L."])
        .nothing_else();
}

#[test]
fn aggregates() {
    // see https://github.com/gbif/checklistbank/issues/69
    assert_name("Achillea millefolium agg. L.")
        .binomial("Achillea", None, "millefolium", Rank::SpeciesAggregate)
        .comb_authors(None, &["L."])
        .nothing_else();

    assert_name("Strumigenys koningsbergeri-group")
        .binomial(
            "Strumigenys",
            None,
            "koningsbergeri",
            Rank::SpeciesAggregate,
        )
        .nothing_else();

    assert_name("Selenophorus parumpunctatus species group")
        .binomial(
            "Selenophorus",
            None,
            "parumpunctatus",
            Rank::SpeciesAggregate,
        )
        .nothing_else();

    assert_name("Monomorium monomorium group")
        .binomial("Monomorium", None, "monomorium", Rank::SpeciesAggregate)
        .nothing_else();
}

// ---- local helpers: one DSL gap not covered by `common::` --------------------------------------

/// `assertUnparsable(name, rank, type)` — Java's rank-hinted overload (`NameParserImplTest.java`
/// private helper, distinct from the `(name, type, code)` overload). The shared DSL only exposes
/// the no-hint `assert_unparsable` and the type+code `assert_unparsable_code`, so this one is
/// reproduced locally rather than added to the shared `common` module.
fn assert_unparsable_rank(input: &str, rank: Rank, type_: NameType) {
    match nameparser::parse_name(input, None, Some(rank), None) {
        Err(e) => assert_eq!(
            e.type_, type_,
            "`{input}` (rank {rank:?}) unparsable as expected but with type {:?}, expected {type_:?}",
            e.type_
        ),
        Ok(pn) => panic!(
            "expected `{input}` (rank {rank:?}) to be unparsable ({type_:?}) but it parsed: {pn:?}"
        ),
    }
}
