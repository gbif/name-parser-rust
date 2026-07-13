// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserImplTest (methods on lines 1970-2409).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NamePart, NameType, NomCode, Rank};

// The publication year is additionally extracted from the publishedIn reference into
// `ParsedName::published_in_year` (an `Option<i32>`), while the reference string keeps the
// year verbatim. When several year-shaped numbers are present the trailing one is taken, since
// a reference lists page numbers (which can fall in the year range) before the publication year.
#[test]
fn published_in_year() {
    use nameparser::parse_name as parse;

    // trailing year 1988 even though the page range "1658-1662" earlier looks year-shaped
    let n = parse(
        "Passiflora eglandulosa J.M. MacDougal. Annals of the Missouri Botanical Garden 75: 1658-1662. figs 1, 2B, and 3. 1988. Figs 36-37",
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        n.published_in.as_deref(),
        Some("Annals of the Missouri Botanical Garden 75: 1658-1662. figs 1, 2B, and 3. 1988. Figs 36-37")
    );
    assert_eq!(n.published_in_year, Some(1988));

    // parenthesised year, kept in the reference
    let n2 = parse(
        "Samyda arborea Rich., Actes Soc. Hist. Nat. Paris 1: 109 (1792).",
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        n2.published_in.as_deref(),
        Some("Actes Soc. Hist. Nat. Paris 1: 109 (1792)")
    );
    assert_eq!(n2.published_in_year, Some(1792));

    // a reference without a year → null
    let n3 = parse(
        "Xolisma turquini Small apud Britton & Wilson",
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(n3.published_in.as_deref(), Some("Britton & Wilson"));
    assert_eq!(n3.published_in_year, None);
}

// An "in <publication>" citation INSIDE the parenthesised basionym: the year is the
// basionym's and the "in …" tail is the publishedIn reference.
// "Hypsicera femoralis (Geoffroy in Fourcroy, 1785)" → basionym "Geoffroy, 1785" (ZOOLOGICAL),
// publishedIn "Fourcroy, 1785".
#[test]
fn in_author_inside_basionym() {
    assert_name("Hypsicera femoralis (Geoffroy in Fourcroy, 1785)")
        .species("Hypsicera", "femoralis")
        .bas_authors(Some("1785"), &["Geoffroy"])
        .code(NomCode::Zoological)
        .published_in("Fourcroy, 1785")
        .published_in_year(Some(1785))
        .nothing_else();
}

#[test]
fn norwegian_radiolaria() {
    assert_name("Actinomma leptodermum longispinum Cortese & Bjørklund 1998")
        .infra_species("Actinomma", "leptodermum", Rank::Subspecies, "longispinum")
        .comb_authors(Some("1998"), &["Cortese", "Bjørklund"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Arachnosphaera dichotoma  Jørgensen, 1900")
        .species("Arachnosphaera", "dichotoma")
        .comb_authors(Some("1900"), &["Jørgensen"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Hexaconthium pachydermum forma legitime Cortese & Bjørklund 1998")
        .infra_species("Hexaconthium", "pachydermum", Rank::Form, "legitime")
        .comb_authors(Some("1998"), &["Cortese", "Bjørklund"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Hexaconthium pachydermum form A Cortese & Bjørklund 1998")
        .infra_species("Hexaconthium", "pachydermum", Rank::Form, "A")
        .comb_authors(Some("1998"), &["Cortese", "Bjørklund"])
        .type_(NameType::Informal)
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Tripodiscium gephyristes  (Hülseman, 1963) BJ&KR-Atsdatabanken")
        .species("Tripodiscium", "gephyristes")
        .bas_authors(Some("1963"), &["Hülseman"])
        .comb_authors(None, &["BJ", "KR-Atsdatabanken"])
        .nothing_else();

    assert_name("Protocystis xiphodon  (Haeckel, 1887), Borgert, 1901")
        .species("Protocystis", "xiphodon")
        .bas_authors(Some("1887"), &["Haeckel"])
        .comb_authors(Some("1901"), &["Borgert"])
        .nothing_else();

    assert_name("Acrosphaera lappacea  (Haeckel, 1887) Takahashi, 1991")
        .species("Acrosphaera", "lappacea")
        .bas_authors(Some("1887"), &["Haeckel"])
        .comb_authors(Some("1991"), &["Takahashi"])
        .nothing_else();
}

// cv. Abbreviation of cultivar
// A formal category under the cultivated plant code (ICNCP), not the botanical code
// Must follow strict formatting rules:
//  - not italicized
//  - capitalized
//  - usually in quotes
//
// Correct style: Abies brevifolia 'Short Needle'
// (or historically: Abies brevifolia cv. 'Short Needle')
#[test]
fn cultivars() {
    assert_name("Abutilon 'Kentish Belle'")
        .cultivar("Abutilon", "Kentish Belle")
        .nothing_else();

    assert_name("Acer campestre L. cv. 'nanum'")
        .cultivar_sp("Acer", "campestre", "nanum")
        .comb_authors(None, &["L."])
        .nothing_else();

    assert_name("Verpericola megasoma \"Dall\" Pils.")
        .cultivar_sp("Verpericola", "megasoma", "Dall")
        .comb_authors(None, &["Pils."])
        .nothing_else();

    assert_name("Abutilon 'Kentish Belle'")
        .cultivar("Abutilon", "Kentish Belle")
        .nothing_else();

    assert_name("Abutilon 'Nabob'")
        .cultivar("Abutilon", "Nabob")
        .nothing_else();

    assert_name("Sorbus americana Marshall cv. 'Belmonte'")
        .cultivar_sp("Sorbus", "americana", "Belmonte")
        .comb_authors(None, &["Marshall"])
        .nothing_else();

    assert_name("Sorbus hupehensis C.K.Schneid. cv. 'November pink'")
        .cultivar_sp("Sorbus", "hupehensis", "November pink")
        .comb_authors(None, &["C.K.Schneid."])
        .nothing_else();

    assert_name("Symphoricarpos albus (L.) S.F.Blake cv. 'Turesson'")
        .cultivar_sp_rank("Symphoricarpos", "albus", Rank::Cultivar, "Turesson")
        .bas_authors(None, &["L."])
        .comb_authors(None, &["S.F.Blake"])
        .nothing_else();

    assert_name("Symphoricarpos sp. cv. 'mother of pearl'")
        .cultivar_rank("Symphoricarpos", Rank::Cultivar, "mother of pearl")
        .nothing_else();

    // The cultivar name's author is the cultivar author (Broerse); the preceding species
    // author (L.) is captured as the specific authorship, not merged into "L. & Broerse".
    assert_name("Acer campestre L. cv. 'Elsrijk' Broerse")
        .cultivar_sp("Acer", "campestre", "Elsrijk")
        .comb_authors(None, &["Broerse"])
        .specific_authors(None, &["L."])
        .nothing_else();

    assert_name("Primula Border Auricula Group")
        .cultivar_rank("Primula", Rank::CultivarGroup, "Border Auricula")
        .nothing_else();

    assert_name("Rhododendron boothii Mishmiense Group")
        .cultivar_sp_rank("Rhododendron", "boothii", Rank::CultivarGroup, "Mishmiense")
        .nothing_else();

    assert_name("Paphiopedilum Sorel grex")
        .cultivar_rank("Paphiopedilum", Rank::Grex, "Sorel")
        .nothing_else();

    assert_name("Cattleya Prince John gx")
        .cultivar_rank("Cattleya", Rank::Grex, "Prince John")
        .nothing_else();
}

// An unclosed leading cultivar quote (single or double) trailing a name — common in
// aquarium/horticultural trade lists — must be treated like the properly closed form
// rather than being mistaken for a combination author.
#[test]
fn unclosed_cultivar_quote() {
    assert_name("Labeotropheus trewavasae 'albino")
        .cultivar_sp("Labeotropheus", "trewavasae", "albino")
        .nothing_else();

    assert_name("Labeotropheus trewavasae \"albino")
        .cultivar_sp("Labeotropheus", "trewavasae", "albino")
        .nothing_else();
}

// "ht." is an occasional abbreviation of the horticultural marker "hort." written
// directly on the author span. It must parse like its spelled-out twin
// ("Gymnogramma alstoni hort.Birkenh.; Gard.") instead of leaking "ht" in as a bogus
// infraspecific epithet.
#[test]
fn hort_abbreviation() {
    assert_name("Gymnogramma alstoni ht.Birkenh.; Gard.")
        .species("Gymnogramma", "alstoni")
        .comb_authors(None, &["hort.Birkenh.", "Gard."])
        .nothing_else();
}

#[test]
fn hybrid_formulas() {
    assert_name("Polypodium  x vulgare nothosubsp. mantoniae (Rothm.) Schidlay")
        .infra_species("Polypodium", "vulgare", Rank::Subspecies, "mantoniae")
        .bas_authors(None, &["Rothm."])
        .comb_authors(None, &["Schidlay"])
        .notho(&[NamePart::Infraspecific])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_hybrid_formula("Asplenium rhizophyllum DC. x ruta-muraria E.L. Braun 1939");
    assert_hybrid_formula("Arthopyrenia hyalospora X Hydnellum scrobiculatum");
    assert_hybrid_formula(
        "Arthopyrenia hyalospora (Banker) D. Hall X Hydnellum scrobiculatum D.E. Stuntz",
    );
    assert_hybrid_formula("Arthopyrenia hyalospora × ? ");
    assert_hybrid_formula("Agrostis L. × Polypogon Desf. ");
    assert_hybrid_formula("Agrostis stolonifera L. × Polypogon monspeliensis (L.) Desf. ");
    assert_hybrid_formula("Asplenium rhizophyllum X A. ruta-muraria E.L. Braun 1939");
    assert_hybrid_formula("Asplenium rhizophyllum DC. x ruta-muraria E.L. Braun 1939");
    assert_hybrid_formula("Asplenium rhizophyllum x ruta-muraria");
    assert_hybrid_formula("Salix aurita L. × S. caprea L.");
    assert_hybrid_formula("Mentha aquatica L. × M. arvensis L. × M. spicata L.");
    assert_hybrid_formula("Polypodium vulgare subsp. prionodes (Asch.) Rothm. × subsp. vulgare");
    assert_hybrid_formula("Tilletia caries (Bjerk.) Tul. × T. foetida (Wallr.) Liro.");
    assert_hybrid_formula("Cirsium acaulon x arvense");
    assert_hybrid_formula("Juncus effusus × inflexus");
    assert_hybrid_formula("Symphytum caucasicum x uplandicum");
}

#[test]
fn o_tu() {
    // https://github.com/gbif/name-parser/issues/74
    // 5.0.0: Java's `.phraseName(monomial, phrase)` is a supraspecific-provisional name → an
    // `Informal` result. The anchor is a suprageneric uninomial (a GTDB-style phylum), so its
    // `taxon_rank` is UNRANKED (the parser cannot rank it), not GENUS.
    assert_informal("Desulfobacterota_B")
        .taxon("Desulfobacterota")
        .taxon_rank(Rank::Unranked)
        .rank(Rank::Unranked)
        .phrase("B")
        .nothing_else();

    // unparsable identifiers
    assert_unparsable("UBA3054", NameType::Other);
    assert_unparsable("F0040", NameType::Other);
    assert_unparsable("AABM5-125-24", NameType::Other);
    assert_unparsable("B130-G9", NameType::Other);
    assert_unparsable("BMS3Abin14", NameType::Other);
    assert_unparsable("4572-55", NameType::Other);
    assert_unparsable("T1SED10-198M", NameType::Other);
    assert_unparsable("BMS3Abin14", NameType::Other);
    assert_unparsable("UBA11359_C", NameType::Other);
    assert_unparsable("01-FULL-45-15b", NameType::Other);
    assert_unparsable("E44-bin80", NameType::Other);
    assert_unparsable("E2", NameType::Other);
    assert_unparsable("9FT-COMBO-53-11", NameType::Other);
    assert_unparsable("AqS3", NameType::Other);
    assert_unparsable("Gp7-AA8", NameType::Other);
    assert_unparsable("0-14-0-10-38-17 sp002774085", NameType::Other);
    assert_unparsable("01-FULL-45-15b sp001822655", NameType::Other);
    assert_unparsable("18JY21-1 sp004344915", NameType::Other);
    assert_unparsable("SH1508347.08FU", NameType::Other);
    assert_unparsable("SH19186714.17FU", NameType::Other);
    assert_unparsable("SH191814.08FU", NameType::Other);
    assert_unparsable("SH191814.04FU", NameType::Other);
    assert_unparsable("BOLD:ACW2100", NameType::Other);
    assert_unparsable("BOLD:ACW2100", NameType::Other);
    assert_unparsable_name(
        " BOLD:ACW2100 ",
        Rank::Unranked,
        NameType::Other,
        "BOLD:ACW2100",
    );
    assert_unparsable_name(
        "Festuca sp. BOLD:ACW2100",
        Rank::Unranked,
        NameType::Other,
        "BOLD:ACW2100",
    );
    assert_unparsable_name(
        "sh460441.07fu",
        Rank::Unranked,
        NameType::Other,
        "SH460441.07FU",
    );

    // no OTU names
    assert_name("Boldenaria")
        .monomial("Boldenaria")
        .nothing_else();

    assert_name("Boldea").monomial("Boldea").nothing_else();

    assert_name("Boldiaceae")
        .monomial_rank("Boldiaceae", Rank::Family)
        .nothing_else();

    assert_name("Boldea vulgaris")
        .species("Boldea", "vulgaris")
        .nothing_else();
}

#[test]
fn hybrid_alike_names() {
    assert_name("Huaiyuanella Xing, Yan & Yin, 1984")
        .monomial("Huaiyuanella")
        .comb_authors(Some("1984"), &["Xing", "Yan", "Yin"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Caveasphaera Xiao & Knoll, 2000")
        .monomial("Caveasphaera")
        .comb_authors(Some("2000"), &["Xiao", "Knoll"])
        .code(NomCode::Zoological)
        .nothing_else();
}

// https://github.com/CatalogueOfLife/data/issues/1079
#[test]
fn case_sensitive() {
    assert_name("CHIONE elevata")
        .species("Chione", "elevata")
        .nothing_else();

    assert_name("CHIONE ELEVATA")
        .species("Chione", "elevata")
        .nothing_else();

    assert_name("CHIONE ELEVATA VULGARIS")
        .infra_species("Chione", "elevata", Rank::InfraspecificName, "vulgaris")
        .nothing_else();

    // A diacritic in an all-caps epithet is treated like its ASCII twin (CHIONE ELEVATA).
    assert_name("CHIONE ELEVÄTA")
        .species("Chione", "eleväta")
        .nothing_else();

    assert_name("CHIONE ELEV.")
        .monomial("Chione")
        .comb_authors(None, &["Elev."])
        .nothing_else();

    assert_name("chione elevata")
        .species("Chione", "elevata")
        .nothing_else();

    assert_name("chione elevata vulgaris")
        .infra_species("Chione", "elevata", Rank::InfraspecificName, "vulgaris")
        .nothing_else();
}

#[test]
fn alpha_beta_theta_names() {
    assert_name("Euchlanis dilatata β-larga")
        .infra_species("Euchlanis", "dilatata", Rank::InfraspecificName, "β-larga")
        .nothing_else();

    assert_name("Trianosperma ficifolia var. βrigida Cogn.")
        .infra_species("Trianosperma", "ficifolia", Rank::Variety, "βrigida")
        .comb_authors(None, &["Cogn."])
        .nothing_else();

    assert_name("Agaricus collinitus β mucosus (Bull.) Fr.")
        .infra_species("Agaricus", "collinitus", Rank::InfraspecificName, "mucosus")
        .comb_authors(None, &["Fr."])
        .bas_authors(None, &["Bull."])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Cyclotus amethystinus var. β Guppy, 1868")
        .infra_species("Cyclotus", "amethystinus", Rank::Variety, "β")
        .comb_authors(Some("1868"), &["Guppy"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn hybrid_names() {
    assert_name("+ Pyrocrataegus willei L.L.Daniel")
        .species("Pyrocrataegus", "willei")
        .comb_authors(None, &["L.L.Daniel"])
        .notho(&[NamePart::Generic])
        .nothing_else();

    assert_name("×Pyrocrataegus willei L.L. Daniel")
        .species("Pyrocrataegus", "willei")
        .comb_authors(None, &["L.L.Daniel"])
        .notho(&[NamePart::Generic])
        .nothing_else();

    assert_name(" × Pyrocrataegus willei  L. L. Daniel")
        .species("Pyrocrataegus", "willei")
        .comb_authors(None, &["L.L.Daniel"])
        .notho(&[NamePart::Generic])
        .nothing_else();

    assert_name(" X Pyrocrataegus willei L. L. Daniel")
        .species("Pyrocrataegus", "willei")
        .comb_authors(None, &["L.L.Daniel"])
        .notho(&[NamePart::Generic])
        .nothing_else();

    assert_name("Pyrocrataegus ×willei L. L. Daniel")
        .species("Pyrocrataegus", "willei")
        .comb_authors(None, &["L.L.Daniel"])
        .notho(&[NamePart::Specific])
        .nothing_else();

    assert_name("Pyrocrataegus × willei L. L. Daniel")
        .species("Pyrocrataegus", "willei")
        .comb_authors(None, &["L.L.Daniel"])
        .notho(&[NamePart::Specific])
        .nothing_else();

    assert_name("Pyrocrataegus x willei L. L. Daniel")
        .species("Pyrocrataegus", "willei")
        .comb_authors(None, &["L.L.Daniel"])
        .notho(&[NamePart::Specific])
        .nothing_else();

    assert_name("Pyrocrataegus X willei L. L. Daniel")
        .species("Pyrocrataegus", "willei")
        .comb_authors(None, &["L.L.Daniel"])
        .notho(&[NamePart::Specific])
        .nothing_else();

    assert_name("Pyrocrataegus willei ×libidi  L.L.Daniel")
        .infra_species("Pyrocrataegus", "willei", Rank::InfraspecificName, "libidi")
        .comb_authors(None, &["L.L.Daniel"])
        .notho(&[NamePart::Infraspecific])
        .nothing_else();

    assert_name("Pyrocrataegus willei nothosubsp. libidi  L.L.Daniel")
        .infra_species("Pyrocrataegus", "willei", Rank::Subspecies, "libidi")
        .comb_authors(None, &["L.L.Daniel"])
        .notho(&[NamePart::Infraspecific])
        .nothing_else();

    assert_name("+ Pyrocrataegus willei nothosubsp. libidi  L.L.Daniel")
        .infra_species("Pyrocrataegus", "willei", Rank::Subspecies, "libidi")
        .comb_authors(None, &["L.L.Daniel"])
        .notho(&[NamePart::Infraspecific])
        .nothing_else();
}

#[test]
fn author_variations() {
    // Van den heede works only if given as separate authorship

    // assertName("Asplenium cyprium Viane & Van den heede", "Asplenium cyprium")
    //     .species("Asplenium", "cyprium")
    //     .combAuthors(null, "Viane", "Van den heede")
    //     .nothingElse();

    // bis and ter as author suffix
    // https://github.com/Sp2000/colplus-backend/issues/591
    assert_name("Lagenophora queenslandica Jian Wang ter & A.R.Bean")
        .species("Lagenophora", "queenslandica")
        .comb_authors(None, &["Jian Wang ter", "A.R.Bean"])
        .nothing_else();

    assert_name("Abies arctica A.Murray bis")
        .species("Abies", "arctica")
        .comb_authors(None, &["A.Murray bis"])
        .nothing_else();

    assert_name("Abies lowiana (Gordon) A.Murray bis")
        .species("Abies", "lowiana")
        .bas_authors(None, &["Gordon"])
        .comb_authors(None, &["A.Murray bis"])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Trappeindia Castellano, S.L. Mill., L. Singh bis & T.N. Lakh. 2012")
        .monomial("Trappeindia")
        .comb_authors(
            Some("2012"),
            &["Castellano", "S.L.Mill.", "L.Singh bis", "T.N.Lakh."],
        )
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Trichosporon cutaneum (Beurm., Gougerot & Vaucher bis) M. Ota")
        .species("Trichosporon", "cutaneum")
        .bas_authors(None, &["Beurm.", "Gougerot", "Vaucher bis"])
        .comb_authors(None, &["M.Ota"])
        .code(NomCode::Botanical)
        .nothing_else();

    // van der
    assert_name("Megistocera tenuis (van der Wulp, 1885)")
        .species("Megistocera", "tenuis")
        .bas_authors(Some("1885"), &["van der Wulp"])
        .code(NomCode::Zoological)
        .nothing_else();

    // turkish chars
    assert_name("Stachys marashica Ilçim, Çenet & Dadandi")
        .species("Stachys", "marashica")
        .comb_authors(None, &["Ilçim", "Çenet", "Dadandi"])
        .nothing_else();

    // cedilla inside an epithet must not split the word (gbif/name-parser#104)
    assert_name("Euphrasia mendonçae")
        .species("Euphrasia", "mendonçae")
        .nothing_else();

    assert_name("Viola bocquetiana S. Yildirimli")
        .species("Viola", "bocquetiana")
        .comb_authors(None, &["S.Yildirimli"])
        .nothing_else();

    assert_name("Anatolidamnicola gloeri gloeri Şahin, Koca & Yildirim, 2012")
        .infra_species("Anatolidamnicola", "gloeri", Rank::Subspecies, "gloeri")
        .comb_authors(Some("2012"), &["Şahin", "Koca", "Yildirim"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Modiola caroliniana L.f")
        .species("Modiola", "caroliniana")
        .comb_authors(None, &["L.f"])
        .nothing_else();

    assert_name("Modiola caroliniana (L.) G. Don filius")
        .species("Modiola", "caroliniana")
        .bas_authors(None, &["L."])
        .comb_authors(None, &["G.Don filius"])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Modiola caroliniana (L.) G. Don fil.")
        .species("Modiola", "caroliniana")
        .bas_authors(None, &["L."])
        .comb_authors(None, &["G.Don fil."])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Cirsium creticum d'Urv.")
        .species("Cirsium", "creticum")
        .comb_authors(None, &["d'Urv."])
        .nothing_else();

    // Autonym authors are the species authors (ICN Art. 22.1/26.1): the autonym's final
    // epithet carries no author, but the species author "d'Urv." is captured and rendered
    // after the species epithet.
    assert_name("Cirsium creticum d'Urv. subsp. creticum")
        .infra_species("Cirsium", "creticum", Rank::Subspecies, "creticum")
        .comb_authors(None, &["d'Urv."])
        .autonym()
        .nothing_else();

    assert_name("Cirsium creticum Balsamo M Fregni E Tongiorgi P")
        .species("Cirsium", "creticum")
        .comb_authors(None, &["M.Balsamo", "E.Fregni", "P.Tongiorgi"])
        .nothing_else();

    assert_name("Cirsium creticum Balsamo M Todaro MA")
        .species("Cirsium", "creticum")
        .comb_authors(None, &["M.Balsamo", "M.A.Todaro"])
        .nothing_else();

    assert_name("Bolivina albatrossi Cushman Em. Sellier de Civrieux, 1976")
        .species("Bolivina", "albatrossi")
        .comb_authors(Some("1976"), &["Cushman Em.Sellier de Civrieux"])
        .code(NomCode::Zoological)
        .nothing_else();

    // http://dev.gbif.org/issues/browse/POR-101
    assert_name("Cribbia pendula la Croix & P.J.Cribb")
        .species("Cribbia", "pendula")
        .comb_authors(None, &["la Croix", "P.J.Cribb"])
        .nothing_else();

    assert_name("Cribbia pendula le Croix & P.J.Cribb")
        .species("Cribbia", "pendula")
        .comb_authors(None, &["le Croix", "P.J.Cribb"])
        .nothing_else();

    assert_name("Cribbia pendula de la Croix & le P.J.Cribb")
        .species("Cribbia", "pendula")
        .comb_authors(None, &["de la Croix", "le P.J.Cribb"])
        .nothing_else();

    assert_name("Cribbia pendula Croix & de le P.J.Cribb")
        .species("Cribbia", "pendula")
        .comb_authors(None, &["Croix", "de le P.J.Cribb"])
        .nothing_else();

    assert_name("Navicula ambigua f. craticularis Istv?nffi, 1898, 1897")
        .infra_species("Navicula", "ambigua", Rank::Form, "craticularis")
        .comb_authors(Some("1898"), &["Istvnffi"])
        .imprint_year("1897")
        .doubtful()
        .code(NomCode::Zoological)
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();

    assert_name("Cestodiscus gemmifer F.S.Castracane degli Antelminelli")
        .species("Cestodiscus", "gemmifer")
        .comb_authors(None, &["F.S.Castracane degli Antelminelli"])
        .nothing_else();

    assert_name("Hieracium scorzoneraefolium De la Soie")
        .species("Hieracium", "scorzoneraefolium")
        .comb_authors(None, &["De la Soie"])
        .nothing_else();

    assert_name("Sepidium capricorne des Desbrochers des Loges, 1881")
        .species("Sepidium", "capricorne")
        .comb_authors(Some("1881"), &["des Desbrochers des Loges"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Sepidium capricorne Desbrochers des Loges, 1881")
        .species("Sepidium", "capricorne")
        .comb_authors(Some("1881"), &["Desbrochers des Loges"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Calycostylis aurantiaca Hort. ex Vilmorin")
        .species("Calycostylis", "aurantiaca")
        .comb_authors(None, &["Vilmorin"])
        .comb_ex_authors(&["hort."])
        .nothing_else();

    assert_name("Pourretia magnispatha hortusa ex K. Koch")
        .species("Pourretia", "magnispatha")
        .comb_authors(None, &["K.Koch"])
        .comb_ex_authors(&["hort."])
        .nothing_else();

    assert_name("Pitcairnia pruinosa hortus ex K. Koch")
        .species("Pitcairnia", "pruinosa")
        .comb_authors(None, &["K.Koch"])
        .comb_ex_authors(&["hort."])
        .nothing_else();

    assert_name("Platycarpha glomerata (Thunberg) A.P.de Candolle")
        .species("Platycarpha", "glomerata")
        .bas_authors(None, &["Thunberg"])
        .comb_authors(None, &["A.P.de Candolle"])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name(
        "Abies alba (Huguet del Villar) S. Rivas-Martínez, F. Fernández González & D. Sánchez-Mata",
    )
    .species("Abies", "alba")
    .bas_authors(None, &["Huguet del Villar"])
    .comb_authors(
        None,
        &["S.Rivas-Martínez", "F.Fernández González", "D.Sánchez-Mata"],
    )
    .code(NomCode::Botanical)
    .nothing_else();

    assert_name(
        "Sida kohautiana var. corchorifolia (H. da C. Monteiro Filho) H. da C. Monteiro Filho",
    )
    .infra_species("Sida", "kohautiana", Rank::Variety, "corchorifolia")
    .bas_authors(None, &["H.da C.Monteiro Filho"])
    .comb_authors(None, &["H.da C.Monteiro Filho"])
    .code(NomCode::Botanical)
    .nothing_else();

    assert_name("Stenosigma humerale Giordani Soika, 1990")
        .species("Stenosigma", "humerale")
        .comb_authors(Some("1990"), &["Giordani Soika"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Errantia Audouin & H Milne Edwards, 1832")
        .monomial("Errantia")
        .comb_authors(Some("1832"), &["Audouin", "H.Milne Edwards"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Thuja pyramidalis var. stricta John M.Mill.")
        .infra_species("Thuja", "pyramidalis", Rank::Variety, "stricta")
        .comb_authors(None, &["John M.Mill."])
        .nothing_else();

    assert_name("Tetralobus flabellicornis subsp. flabellicornis (Linnaeus, 1767) Linnaeus 1767")
        .infra_species(
            "Tetralobus",
            "flabellicornis",
            Rank::Subspecies,
            "flabellicornis",
        )
        .bas_authors(Some("1767"), &["Linnaeus"])
        .comb_authors(Some("1767"), &["Linnaeus"])
        .nothing_else();

    assert_name("Parachipteria van der Hammen, 1952")
        .monomial("Parachipteria")
        .comb_authors(Some("1952"), &["van der Hammen"])
        .code(NomCode::Zoological)
        .nothing_else();
}

// ---- local helpers: one DSL gap not covered by `common::` --------------------------------------

/// Java's `protected void assertHybridFormula(String name)` (`NameParserImplTest.java:2206-2208`)
/// — a hybrid-formula input must be unparsable with `NameType.FORMULA`.
fn assert_hybrid_formula(name: &str) {
    assert_unparsable(name, NameType::Formula);
}
