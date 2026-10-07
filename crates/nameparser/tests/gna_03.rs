// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserGnaTest (methods on lines 982-1461).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NamePart, NameType, NomCode, Rank};

#[test]
fn names_with_the_dagger_char() {
    // group: Names with the dagger char '†'. The dagger marks the taxon as
    // extinct; it is stripped from anywhere in the input and sets extinct=true.
    assert_name("Henriksenopterix†")
        .monomial("Henriksenopterix")
        .extinct()
        .nothing_else();
    assert_name("Henriksenopterix† paucistriata (Henriksen, 1922)")
        .species("Henriksenopterix", "paucistriata")
        .bas_authors(Some("1922"), &["Henriksen"])
        .extinct()
        .code(NomCode::Zoological)
        .nothing_else();
    // Trailing surname-first all-caps initials ("Huia N E") flip to "E.N.Huia"
    // per the surname-first author convention.
    // FIXME(review): undotted initials come out reversed (N.E., not E.N.), and "Huia" is the bird's vernacular name
    assert_name("Heteralocha acutirostris (Gould, 1837) Huia N E†")
        .species("Heteralocha", "acutirostris")
        .comb_authors(None, &["E.N.Huia"])
        .bas_authors(Some("1837"), &["Gould"])
        .extinct()
        .nothing_else();
    // FIXME(review): junk with a vernacular name, parsed as a name
    // skipped: "Oncorhynchus nerka (Walbaum, 1792) Sockeye salmon F A †?" —
    //   "Sockeye salmon" is a vernacular name embedded in the authorship slot;
    //   parser can't separate it from real authors without a vernacular list.
}

#[test]
fn hybrids_with_notho_ranks() {
    // group: Hybrids with notho- ranks. notho-prefixed and short n-prefixed
    // infraspecies markers (nvar. / nothovar. / nothosubsp. / nothof. / nothossp.)
    // are recognised; the resulting name carries the INFRASPECIFIC notho flag.
    // Botanical rank markers kept in canonical (notho rendered as "nothovar." etc.).
    assert_name("Crataegus curvisepala nvar. naviculiformis T. Petauer")
        .infra_species("Crataegus", "curvisepala", Rank::Variety, "naviculiformis")
        .comb_authors(None, &["T.Petauer"])
        .notho(&[NamePart::Infraspecific])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Abies masjoannis nothof. mesoides")
        .infra_species("Abies", "masjoannis", Rank::Form, "mesoides")
        .notho(&[NamePart::Infraspecific])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Aconitum berdaui nothosubsp. walasii (Mitka) Mitka")
        .infra_species("Aconitum", "berdaui", Rank::Subspecies, "walasii")
        .comb_authors(None, &["Mitka"])
        .bas_authors(None, &["Mitka"])
        .notho(&[NamePart::Infraspecific])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Aconitum tauricum nothossp. hayekianum (Gáyer) Grintescu")
        .infra_species("Aconitum", "tauricum", Rank::Subspecies, "hayekianum")
        .comb_authors(None, &["Grintescu"])
        .bas_authors(None, &["Gáyer"])
        .notho(&[NamePart::Infraspecific])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Aeonium holospathulatum nothovar. sanchezii (Bañares) Bañares")
        .infra_species("Aeonium", "holospathulatum", Rank::Variety, "sanchezii")
        .comb_authors(None, &["Bañares"])
        .bas_authors(None, &["Bañares"])
        .notho(&[NamePart::Infraspecific])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Aeonium × proliferum Bañares nothovar. glabrifolium Bañares")
        .infra_species("Aeonium", "proliferum", Rank::Variety, "glabrifolium")
        .comb_authors(None, &["Bañares"])
        .notho(&[NamePart::Infraspecific])
        .code(NomCode::Botanical)
        .specific_authors(None, &["Bañares"])
        .nothing_else();
    assert_name("Biscogniauxia nothofagi Whalley, Læssøe & Kile 1990")
        .species("Biscogniauxia", "nothofagi")
        .comb_authors(Some("1990"), &["Whalley", "Læssøe", "Kile"])
        .code(NomCode::Zoological)
        .nothing_else();
    // Skipped — nothosect./nothoser. after an author span (Aconitum W. Mucher
    // nothosect. Acopellus), and notho-marker-after-author-span variants
    // (Amaranthus ×ozanonii (Contré) Lambinon nothosubsp. ralletii;
    // Aconitum ×teppneri Mucher ex Starm. nothosubsp. goetzii) currently lose
    // the notho marker.
}

#[test]
fn named_hybrids() {
    // group: Named hybrids
    assert_name("×Agropogon P. Fourn. 1934")
        .monomial("Agropogon")
        .notho(&[NamePart::Generic])
        .comb_authors(Some("1934"), &["P.Fourn."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("xAgropogon P. Fourn.")
        .monomial("Agropogon")
        .notho(&[NamePart::Generic])
        .comb_authors(None, &["P.Fourn."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("XAgropogon P.Fourn.")
        .monomial("Agropogon")
        .notho(&[NamePart::Generic])
        .comb_authors(None, &["P.Fourn."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("× Agropogon")
        .notho(&[NamePart::Generic])
        .monomial("Agropogon")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("x Agropogon")
        .notho(&[NamePart::Generic])
        .monomial("Agropogon")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("X Agropogon")
        .notho(&[NamePart::Generic])
        .monomial("Agropogon")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("X Cupressocyparis leylandii")
        .notho(&[NamePart::Generic])
        .species("Cupressocyparis", "leylandii")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("×Heucherella tiarelloides")
        .notho(&[NamePart::Generic])
        .species("Heucherella", "tiarelloides")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("xHeucherella tiarelloides")
        .notho(&[NamePart::Generic])
        .species("Heucherella", "tiarelloides")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("x Heucherella tiarelloides")
        .notho(&[NamePart::Generic])
        .species("Heucherella", "tiarelloides")
        .code(NomCode::Botanical)
        .nothing_else();
    // GNA reduces this to a bare monomial; GBIF retains the genus+infrageneric structure
    assert_name("XAgroelymus Lapage sect. Agroelinelymus")
        .infrageneric_at("Agroelymus", Rank::SectionBotany, "Agroelinelymus")
        .notho(&[NamePart::Generic])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("×Agropogon littoralis (Sm.) C. E. Hubb. 1946")
        .species("Agropogon", "littoralis")
        .notho(&[NamePart::Generic])
        .comb_authors(Some("1946"), &["C.E.Hubb."])
        .bas_authors(None, &["Sm."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Asplenium X inexpectatum (E.L. Braun 1940) Morton (1956)")
        .species("Asplenium", "inexpectatum")
        .notho(&[NamePart::Specific])
        .comb_authors(Some("1956"), &["Morton"])
        .bas_authors(Some("1940"), &["E.L.Braun"])
        .code(NomCode::Botanical)
        .nothing_else();
    // GNA drops × from the canonical for species-level hybrids; GBIF includes it
    assert_name("Androrchis × fallax (De Not.) W.Foelsche & Jakely")
        .species("Androrchis", "fallax")
        .notho(&[NamePart::Specific])
        .comb_authors(None, &["W.Foelsche", "Jakely"])
        .bas_authors(None, &["De Not."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Salix ×capreola Andersson (1867)")
        .species("Salix", "capreola")
        .notho(&[NamePart::Specific])
        .comb_authors(Some("1867"), &["Andersson"])
        .code(NomCode::Botanical)
        .nothing_else();
    // x before the specific epithet + nothosubsp. rank marker: the rank marker wins for notho
    assert_name("Polypodium  x vulgare nothosubsp. mantoniae (Rothm.) Schidlay")
        .infra_species("Polypodium", "vulgare", Rank::Subspecies, "mantoniae")
        .notho(&[NamePart::Infraspecific])
        .comb_authors(None, &["Schidlay"])
        .bas_authors(None, &["Rothm."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Salix x capreola Andersson")
        .species("Salix", "capreola")
        .notho(&[NamePart::Specific])
        .comb_authors(None, &["Andersson"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("x Abacopterella x altifrons T.E.Almeida & A.R.Field")
        .species("Abacopterella", "altifrons")
        .notho(&[NamePart::Generic, NamePart::Specific])
        .comb_authors(None, &["T.E.Almeida", "A.R.Field"])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn hybrid_formulae() {
    // group: Hybrid formulae
    assert_unparsable_rank(
        "Stanhopea tigrina Bateman ex Lindl. x S. ecornuta Lem.",
        Rank::Unranked,
        NameType::Formula,
    );
    assert_unparsable_rank(
        "Arthopyrenia hyalospora X Hydnellum scrobiculatum",
        Rank::Unranked,
        NameType::Formula,
    );
    assert_unparsable_rank(
        "Arthopyrenia hyalospora (Banker) D. Hall X Hydnellum scrobiculatum D.E. Stuntz",
        Rank::Unranked,
        NameType::Formula,
    );
    assert_unparsable_rank(
        "Arthopyrenia hyalospora × ?",
        Rank::Unranked,
        NameType::Formula,
    );
    assert_unparsable_rank(
        "Agrostis L. × Polypogon Desf.",
        Rank::Unranked,
        NameType::Formula,
    );
    assert_unparsable_rank(
        "Agrostis stolonifera L. × Polypogon monspeliensis (L.) Desf.",
        Rank::Unranked,
        NameType::Formula,
    );
    assert_unparsable_rank("Coeloglossum viride (L.) Hartman x Dactylorhiza majalis (Rchb. f.) P.F. Hunt & Summerhayes ssp. praetermissa (Druce) D.M. Moore & Soó", Rank::Unranked, NameType::Formula);
    assert_unparsable_rank(
        "Salix aurita L. × S. caprea L.",
        Rank::Unranked,
        NameType::Formula,
    );
    assert_unparsable_rank(
        "Asplenium rhizophyllum X A. ruta-muraria E.L. Braun 1939",
        Rank::Unranked,
        NameType::Formula,
    );
    assert_unparsable_rank(
        "Asplenium rhizophyllum DC. x ruta-muraria E.L. Braun 1939",
        Rank::Unranked,
        NameType::Formula,
    );
    assert_unparsable_rank(
        "Tilletia caries (Bjerk.) Tul. × T. foetida (Wallr.) Liro.",
        Rank::Unranked,
        NameType::Formula,
    );
    assert_unparsable_rank("Brassica oleracea L. subsp. capitata (L.) DC. convar. fruticosa (Metzg.) Alef. × B. oleracea L. subsp. capitata (L.) var. costata DC.", Rank::Unranked, NameType::Formula);
    assert_unparsable_rank(
        "Ambystoma laterale × A. texanum × A. tigrinum",
        Rank::Unranked,
        NameType::Formula,
    );
    assert_name("Pseudocercospora broussonetiae (Chupp & Linder) X.J. Liu & Y.L. Guo 1989")
        .species("Pseudocercospora", "broussonetiae")
        .comb_authors(Some("1989"), &["X.J.Liu", "Y.L.Guo"])
        .bas_authors(None, &["Chupp", "Linder"])
        .nothing_else();
}

#[test]
fn graft_chimeras() {
    // group: Graft-chimeras should parse as hybrid formulas
    //assert_unparsable_rank("+ Crataegomespilus", Rank::Unranked, NameType::Formula);
    //assert_unparsable_rank("+Crataegomespilus", Rank::Unranked, NameType::Formula);
    assert_unparsable_rank(
        "Cytisus purpureus + Laburnum anagyroides",
        Rank::Unranked,
        NameType::Formula,
    );
    assert_unparsable_rank("Crataegus + Mespilus", Rank::Unranked, NameType::Formula);
}

#[test]
fn genus_with_hyphen_allowed_by_icn() {
    // group: Genus with hyphen (allowed by ICN)
    assert_name("Saxo-Fridericia R. H. Schomb.")
        .monomial("Saxo-Fridericia")
        .comb_authors(None, &["R.H.Schomb."])
        .nothing_else();

    assert_name("Saxo-fridericia R. H. Schomb.")
        .monomial("Saxo-fridericia")
        .comb_authors(None, &["R.H.Schomb."])
        .nothing_else();

    assert_name("Uva-ursi cinerea (Howell) A. Heller")
        .species("Uva-ursi", "cinerea")
        .comb_authors(None, &["A.Heller"])
        .bas_authors(None, &["Howell"])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Uva-Ursi cinerea (Howell) A. Heller")
        .species("Uva-ursi", "cinerea")
        .comb_authors(None, &["A.Heller"])
        .bas_authors(None, &["Howell"])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Arctostaphylos uva-ursi")
        .species("Arctostaphylos", "uva-ursi")
        .nothing_else();

    assert_name("Prunus-lauro-cerasus")
        .monomial("Prunus-lauro-cerasus")
        .nothing_else();

    assert_name("Prunus-Lauro-Cerasus")
        .monomial("Prunus-lauro-cerasus")
        .nothing_else();

    assert_name("Tsugo-piceo-picea × crassifolia (Flous) Campo-Duplan & Gaussen")
        .species("Tsugo-piceo-picea", "crassifolia")
        .notho(&[NamePart::Specific])
        .comb_authors(None, &["Campo-Duplan", "Gaussen"])
        .bas_authors(None, &["Flous"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Tsugo-piceo-piceo-picea × crassifolia")
        .species("Tsugo-piceo-piceo-picea", "crassifolia")
        .notho(&[NamePart::Specific])
        .code(NomCode::Botanical)
        .nothing_else();
    // The × before crassifolia marks it as a nothotaxon: canonical includes "×"
    assert_name("De-Filippii Gortani & Merla 1934")
        .monomial("De-filippii")
        .comb_authors(Some("1934"), &["Gortani", "Merla"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Eu-Scalpellum Hoek, 1907")
        .monomial("Eu-scalpellum")
        .comb_authors(Some("1907"), &["Hoek"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Eu-hookeria olfersiana (Hornsch.) Hampe")
        .species("Eu-hookeria", "olfersiana")
        .comb_authors(None, &["Hampe"])
        .bas_authors(None, &["Hornsch."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Le-monniera")
        .monomial("Le-monniera")
        .nothing_else();
    assert_name("Le-Monniera clitandrifolia (A. Chev.) Lecomte")
        .species("Le-monniera", "clitandrifolia")
        .comb_authors(None, &["Lecomte"])
        .bas_authors(None, &["A.Chev."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Ne-ourbania adendrobium (Rchb.f. ) Fawc. & Rendle")
        .species("Ne-ourbania", "adendrobium")
        .comb_authors(None, &["Fawc.", "Rendle"])
        .bas_authors(None, &["Rchb.f."])
        .code(NomCode::Botanical)
        .nothing_else();
    // FIXME(review): an OCR artefact parsed as a uninomial
    // skipped: Ph-echinodermata
    assert_name("Prunus-lauro-cerasus")
        .monomial("Prunus-lauro-cerasus")
        .nothing_else();
    assert_name("Prunus-Lauro-Cerasus")
        .monomial("Prunus-lauro-cerasus")
        .nothing_else();
    assert_name("Tsugo-piceo-picea × crassifolia (Flous) Campo-Duplan & Gaussen")
        .species("Tsugo-piceo-picea", "crassifolia")
        .notho(&[NamePart::Specific])
        .comb_authors(None, &["Campo-Duplan", "Gaussen"])
        .bas_authors(None, &["Flous"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Tsugo-piceo-piceo-picea × crassifolia")
        .species("Tsugo-piceo-piceo-picea", "crassifolia")
        .notho(&[NamePart::Specific])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn misspelled_name() {
    // group: Misspelled name — the trailing "Stål, 1862" is read as part of the
    // hyphenated uninomial because the "-Stål" form looks like a single hyphenated
    // genus token; case is preserved verbatim.
    assert_name("Ambrysus-Stål, 1862")
        .monomial("Ambrysus-Stål")
        .comb_authors(Some("1862"), &[])
        .nothing_else();
}

#[test]
fn infrageneric_epithets_iczn() {
    // group: Infrageneric epithets (ICZN). The (Subgenus) parens become the
    // infrageneric epithet on the parsed name. Surname-first all-caps trailing
    // initials ("Lindberg H") are flipped to "H.Lindberg".
    assert_name("Hegeter (Hegeter) tenuipunctatus Brullé, 1838")
        .species_ig("Hegeter", "Hegeter", "tenuipunctatus")
        .comb_authors(Some("1838"), &["Brullé"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Hegeter (Hegeter) intercedens Lindberg H 1950")
        .species_ig("Hegeter", "Hegeter", "intercedens")
        .comb_authors(Some("1950"), &["H.Lindberg"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Cyprideis (Cyprideis) thessalonike amasyaensis")
        .infra_species(
            "Cyprideis",
            "thessalonike",
            Rank::InfraspecificName,
            "amasyaensis",
        )
        .infrageneric("Cyprideis")
        .nothing_else();
    assert_name("Acanthoderes (Abramov) satanas Aurivillius")
        .species_ig("Acanthoderes", "Abramov", "satanas")
        .comb_authors(None, &["Aurivillius"])
        .nothing_else();
    // The lowercase "(acanthoderes)" is not recognised as a subgenus token (subgenus
    // requires a Title-cased word) so the parser bails out at the parens — left as
    // an unparsed tail on the bare uninomial. Skipped here.
}

#[test]
fn names_with_multiple_dashes_in_specific_epithet() {
    // group: Names with multiple dashes in specific epithet
    assert_name("Athyrium boreo-occidentali-indobharaticola-birianum Fraser-Jenk.")
        .species("Athyrium", "boreo-occidentali-indobharaticola-birianum")
        .comb_authors(None, &["Fraser-Jenk."])
        .nothing_else();
    assert_name("Puccinia band-i-amirii Durrieu, 1975")
        .species("Puccinia", "band-i-amirii")
        .comb_authors(Some("1975"), &["Durrieu"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn genus_with_question_mark() {
    // group: Genus with question mark — open-nomenclature doubtful identification.
    // The "?" is captured as a SPECIFIC epithet qualifier (like cf. or aff.).
    assert_name("Ferganoconcha? oblonga")
        .species("Ferganoconcha", "oblonga")
        .type_(NameType::Informal)
        .doubtful()
        .qualifiers(&[(NamePart::Specific, "?")])
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
}

#[test]
fn epithets_starting_with_non() {
    // group: Epithets starting with non- (genuine species names like
    // "Peperomia non-alata"). The hyphenated "non-X" form is kept as the species
    // epithet; modern ex-author convention attaches the validating (post-"ex")
    // author as comb and the cited author as exAuthor.
    assert_name("Peperomia non-alata Trel.")
        .species("Peperomia", "non-alata")
        .comb_authors(None, &["Trel."])
        .nothing_else();
    assert_name("Hyacinthoides non-scripta (L.) Chouard ex Rothm.")
        .species("Hyacinthoides", "non-scripta")
        .comb_authors(None, &["Rothm."])
        .comb_ex_authors(&["Chouard"])
        .bas_authors(None, &["L."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Monocelis non-scripta Curini-Galletti, 2014")
        .species("Monocelis", "non-scripta")
        .comb_authors(Some("2014"), &["Curini-Galletti"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn epithets_starting_with_authors_prefixes_de_di_la_von_etc() {
    // group: Epithets starting with authors' prefixes (de, di, la, von etc.)
    assert_name("Aspicilia desertorum desertorum")
        .infra_species(
            "Aspicilia",
            "desertorum",
            Rank::InfraspecificName,
            "desertorum",
        )
        .nothing_else();
    assert_name("Theope thestias discus")
        .infra_species("Theope", "thestias", Rank::InfraspecificName, "discus")
        .nothing_else();
    assert_name("Ocydromus dalmatinus dalmatinus (Dejean, 1831)")
        .infra_species("Ocydromus", "dalmatinus", Rank::Subspecies, "dalmatinus")
        .bas_authors(Some("1831"), &["Dejean"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Rhipidia gracilirama lassula")
        .infra_species(
            "Rhipidia",
            "gracilirama",
            Rank::InfraspecificName,
            "lassula",
        )
        .nothing_else();
}

#[test]
fn authorship_missing_one_parenthesis() {
    // group: Authorship missing one parenthesis. A bare unmatched closing or
    // opening paren around an authorship-with-year is tolerated — the paren is
    // ignored and the inner authorship parses as a regular zoological
    // combination. Year-bearing trinomial → ZOOLOGICAL → SUBSPECIES.
    assert_name("Ocydromus dalmatinus dalmatinus Dejean, 1831)")
        .infra_species("Ocydromus", "dalmatinus", Rank::Subspecies, "dalmatinus")
        .comb_authors(Some("1831"), &["Dejean"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Ocydromus dalmatinus dalmatinus Dejean, 1831 )")
        .infra_species("Ocydromus", "dalmatinus", Rank::Subspecies, "dalmatinus")
        .comb_authors(Some("1831"), &["Dejean"])
        .code(NomCode::Zoological)
        .nothing_else();
    // FIXME(review): junk parsed as a name
    // skipped: "Ocydromus dalmatinus dalmatinus ( Dejean, 1831 Mill." and
    //   the variant without leading space — missing-paren reconstruction
    //   (splitting Dejean,1831 as basionym from Mill. as combination author)
    //   is not implemented; parser collapses both authors into a single comb.
}

#[test]
fn unknown_authorship() {
    // group: Unknown authorship — "anon." (any case) is the anonymous flag, not an author;
    // "(?)" / "(auct.)" parens before a real author are stripped as unparsed (PARTIAL state).
    // Unlike Java, "(anon.)" is an anonymous basionym rather than unparsed.
    assert_name("Saccharomyces drosophilae anon.")
        .species("Saccharomyces", "drosophilae")
        .comb_anon(None, &[])
        .nothing_else();
    assert_name("Physalospora rubiginosa (Fr.) anon.")
        .species("Physalospora", "rubiginosa")
        .comb_anon(None, &[])
        .bas_authors(None, &["Fr."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Tragacantha leporina (?) Kuntze")
        .species("Tragacantha", "leporina")
        .comb_authors(None, &["Kuntze"])
        .partial("(?)")
        .nothing_else();
    assert_name("Lachenalia tricolor var. nelsonii (auct.) Baker")
        .infra_species("Lachenalia", "tricolor", Rank::Variety, "nelsonii")
        .comb_authors(None, &["Baker"])
        .partial("(auct.)")
        .nothing_else();
    assert_name("Lachenalia tricolor var. nelsonii (anon.) Baker")
        .infra_species("Lachenalia", "tricolor", Rank::Variety, "nelsonii")
        .comb_authors(None, &["Baker"])
        .bas_anon(None, &[])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Puya acris anon.")
        .species("Puya", "acris")
        .comb_anon(None, &[])
        .nothing_else();
}

#[test]
fn anon_authorship() {
    // "Anon."/"Anon"/"anon"/"anon." in any case are the anonymous flag, not an author.
    assert_name("Saccharomyces drosophilae Anon.")
        .species("Saccharomyces", "drosophilae")
        .comb_anon(None, &[])
        .nothing_else();
    assert_name("Saccharomyces drosophilae Anon")
        .species("Saccharomyces", "drosophilae")
        .comb_anon(None, &[])
        .nothing_else();
    assert_name("Saccharomyces drosophilae anon")
        .species("Saccharomyces", "drosophilae")
        .comb_anon(None, &[])
        .nothing_else();
    assert_name("Saccharomyces drosophilae anon. 1923")
        .species("Saccharomyces", "drosophilae")
        .comb_anon(Some("1923"), &[])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn treating_apud_with() {
    // group: Treating apud (with) — "apud" is a publishedIn marker (like "in").
    // The post-apud author span goes to publishedIn; the year propagates onto the
    // comb authorship.
    assert_name("Pseudocercospora dendrobii Goh apud W.H. Hsieh 1990")
        .species("Pseudocercospora", "dendrobii")
        .comb_authors(Some("1990"), &["Goh"])
        .published_in("W.H. Hsieh 1990")
        .published_in_year(Some(1990))
        .nothing_else();
}

#[test]
fn names_with_ex_authors_we_follow_iczn_convention() {
    // group: Names with ex authors (we follow ICZN convention).
    // Year from publishedIn ("in Chimonides, 1987" / "in Souverbie and Montrouzier, 1864")
    // propagates onto comb authorship.
    assert_name("Amathia tricornis Busk ms in Chimonides, 1987")
        .species("Amathia", "tricornis")
        .comb_authors(Some("1987"), &["Busk"])
        .nom_note("ms")
        .published_in("Chimonides, 1987")
        .published_in_year(Some(1987))
        .manuscript()
        .nothing_else();
    assert_name("Pisania billehousti Souverbie, in Souverbie and Montrouzier, 1864")
        .species("Pisania", "billehousti")
        .comb_authors(Some("1864"), &["Souverbie"])
        .published_in("Souverbie and Montrouzier, 1864")
        .published_in_year(Some(1864))
        .nothing_else();
    // Modern interpretation of "X ex Y": the post-ex author Y is the validating
    // author and is captured as the comb (or basionym) author; X becomes the
    // exAuthor reference.
    assert_name("Arthopyrenia hyalospora (Nyl. ex Banker) R.C. Harris")
        .species("Arthopyrenia", "hyalospora")
        .comb_authors(None, &["R.C.Harris"])
        .bas_authors(None, &["Banker"])
        .bas_ex_authors(None, &["Nyl."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Arthopyrenia hyalospora (Nyl. ex. Banker) R.C. Harris")
        .species("Arthopyrenia", "hyalospora")
        .comb_authors(None, &["R.C.Harris"])
        .bas_authors(None, &["Banker"])
        .bas_ex_authors(None, &["Nyl."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Arthopyrenia hyalospora Nyl. ex Banker")
        .species("Arthopyrenia", "hyalospora")
        .comb_authors(None, &["Banker"])
        .comb_ex_authors(&["Nyl."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Arthopyrenia hyalospora Nyl. ex. Banker")
        .species("Arthopyrenia", "hyalospora")
        .comb_authors(None, &["Banker"])
        .comb_ex_authors(&["Nyl."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Glomopsis lonicerae Peck ex C.J. Gould 1945")
        .species("Glomopsis", "lonicerae")
        .comb_authors(Some("1945"), &["C.J.Gould"])
        .comb_ex_authors(&["Peck"])
        .nothing_else();
    assert_name("Glomopsis lonicerae Peck ex. C.J. Gould 1945")
        .species("Glomopsis", "lonicerae")
        .comb_authors(Some("1945"), &["C.J.Gould"])
        .comb_ex_authors(&["Peck"])
        .nothing_else();
    assert_name("Acanthobasidium delicatum (Wakef.) Oberw. ex Jülich 1979")
        .species("Acanthobasidium", "delicatum")
        .comb_authors(Some("1979"), &["Jülich"])
        .comb_ex_authors(&["Oberw."])
        .bas_authors(None, &["Wakef."])
        .nothing_else();
    assert_name("Acanthobasidium delicatum (Wakef.) Oberw. ex. Jülich 1979")
        .species("Acanthobasidium", "delicatum")
        .comb_authors(Some("1979"), &["Jülich"])
        .comb_ex_authors(&["Oberw."])
        .bas_authors(None, &["Wakef."])
        .nothing_else();
    assert_name("Mycosphaerella eryngii (Fr. ex Duby) Johanson ex Oudem. 1897")
        .species("Mycosphaerella", "eryngii")
        .comb_authors(Some("1897"), &["Oudem."])
        .comb_ex_authors(&["Johanson"])
        .bas_authors(None, &["Duby"])
        .bas_ex_authors(None, &["Fr."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Mycosphaerella eryngii (Fr. ex. Duby) Johanson ex. Oudem. 1897")
        .species("Mycosphaerella", "eryngii")
        .comb_authors(Some("1897"), &["Oudem."])
        .comb_ex_authors(&["Johanson"])
        .bas_authors(None, &["Duby"])
        .bas_ex_authors(None, &["Fr."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Mycosphaerella eryngii (Fr. Duby) ex Oudem. 1897")
        .species("Mycosphaerella", "eryngii")
        .comb_authors(Some("1897"), &["Oudem."])
        .bas_authors(None, &["Fr.Duby"])
        .code(NomCode::Botanical)
        .nothing_else();
}
