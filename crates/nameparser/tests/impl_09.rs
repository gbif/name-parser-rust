// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserImplTest (methods on lines 3992-4438).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NomCode, Rank};

#[test]
fn authorship_only_notes() {
    // "(auct.) Author": the parens mark a note, not a basionym → author + taxonomic note
    assert_authorship_note("(auct.) Rolfe", None, &["Rolfe"], "auct.", None);

    assert_authorship_note("(auct.) auct.", None, &[], "auct.", None);

    // taxonomic note + nomenclatural note are split into their own fields
    assert_authorship_note(
        "auct., nom. subnud.",
        None,
        &[],
        "auct.",
        Some("nom. subnud."),
    );

    // a parenthesised "(sensu …)" is the taxonomic note; the trailing name is the author
    assert_authorship_note(
        "(sensu Mereschkowsky, 1878) Jankowski, 1992",
        Some("1992"),
        &["Jankowski"],
        "sensu Mereschkowsky, 1878",
        None,
    );

    // a leading parenthesised homonym citation makes the whole string a taxonomic note
    assert_authorship_note(
        "(non Scacchi, 1836) sensu Zibrowius, 1968",
        None,
        &[],
        "(non Scacchi, 1836) sensu Zibrowius, 1968",
        None,
    );

    assert_authorship_note(
        "Fischer-Le Saux et al., 1999 emend. Akhurst et al., 2004",
        Some("1999"),
        &["Fischer-Le Saux", "al."],
        "emend. Akhurst et al., 2004",
        None,
    );

    assert_authorship_note(
        "Trautv. & Meyer sensu lato",
        None,
        &["Trautv.", "Meyer"],
        "sensu lato",
        None,
    );

    assert_authorship_note("Mill. non Parolly", None, &["Mill."], "non Parolly", None);
}

#[test]
fn authorship_only() {
    assert_authorship("1771", &[])
        .comb_authors(Some("1771"), &[])
        .nothing_else();

    assert_authorship("Pallas, 1771", &[])
        .comb_authors(Some("1771"), &["Pallas"])
        .nothing_else();

    // https://github.com/CatalogueOfLife/data/issues/176
    assert_authorship("Maas & He", &[])
        .comb_authors(None, &["Maas", "He"])
        .nothing_else();

    assert_authorship("Yang & Wu", &[])
        .comb_authors(None, &["Yang", "Wu"])
        .nothing_else();

    assert_authorship("Freytag & Ma", &[])
        .comb_authors(None, &["Freytag", "Ma"])
        .nothing_else();

    // Direct-parse fallback: the shared `assert_ex_authorship` DSL only forwards comb/bas/sanct
    // authorship onto its returned `NameAssertion`, dropping the nomenclatural note this
    // authorship string itself carries — checked directly against the full parse.
    let n = nameparser::parse_name(
        "Abies alba",
        Some("(Ristorcelli & Van ty) Wedd. ex Sch. Bip. (nom. nud.)"),
        Some(Rank::Species),
        None,
    )
    .unwrap_or_else(|e| panic!("authorship should parse: {e:?}"));
    assert_eq!(n.basionym_authorship.year, None);
    assert_eq!(
        n.basionym_authorship.authors,
        vec!["Ristorcelli".to_string(), "Van ty".to_string()]
    );
    assert_eq!(n.combination_authorship.year, None);
    assert_eq!(
        n.combination_authorship.authors,
        vec!["Sch.Bip.".to_string()]
    );
    assert_eq!(
        n.combination_authorship.ex_authors,
        vec!["Wedd.".to_string()]
    );
    assert_eq!(n.nomenclatural_note.as_deref(), Some("nom. nud."));

    assert_authorship("(Wang & Liu, 1996)", &[])
        .bas_authors(Some("1996"), &["Wang", "Liu"])
        .nothing_else();

    assert_authorship("(Wang, Yuwen & Xian-wei Liu, 1996)", &[])
        .bas_authors(Some("1996"), &["Wang", "Yuwen", "Xian-wei Liu"])
        .nothing_else();

    assert_authorship("(Liu, Xian-wei, Z. Zheng & G. Xi, 1991)", &[])
        .bas_authors(Some("1991"), &["Liu", "Xian-wei", "Z.Zheng", "G.Xi"])
        .nothing_else();

    assert_authorship("(Ristorcelli & Van ty, 1941)", &[])
        .bas_authors(Some("1941"), &["Ristorcelli", "Van ty"])
        .nothing_else();

    assert_authorship("FISCHER 1885", &[])
        .comb_authors(Some("1885"), &["Fischer"])
        .nothing_else();

    assert_authorship("(Walker, F., 1858)", &[])
        .bas_authors(Some("1858"), &["F.Walker"])
        .nothing_else();

    assert_authorship("Schaufuss, L. W.", &[])
        .comb_authors(None, &["L.W.Schaufuss"])
        .nothing_else();

    assert_authorship("Schaufuss, L. W., 1877", &[])
        .comb_authors(Some("1877"), &["L.W.Schaufuss"])
        .nothing_else();

    assert_authorship("LeConte, J. L., 1878", &[])
        .comb_authors(Some("1878"), &["J.L.LeConte"])
        .nothing_else();

    assert_authorship("Jian Wang ter & A.R.Bean", &[])
        .comb_authors(None, &["Jian Wang ter", "A.R.Bean"])
        .nothing_else();

    assert_authorship("A.Murray bis", &[])
        .comb_authors(None, &["A.Murray bis"])
        .nothing_else();

    assert_authorship("(Gordon) A.Murray bis", &[])
        .bas_authors(None, &["Gordon"])
        .comb_authors(None, &["A.Murray bis"])
        .nothing_else();

    assert_authorship(
        "Castellano, S.L. Mill., L. Singh bis & T.N. Lakh. 2012",
        &[],
    )
    .comb_authors(
        Some("2012"),
        &["Castellano", "S.L.Mill.", "L.Singh bis", "T.N.Lakh."],
    )
    .nothing_else();

    assert_authorship("(Beurm., Gougerot & Vaucher bis) M. Ota", &[])
        .bas_authors(None, &["Beurm.", "Gougerot", "Vaucher bis"])
        .comb_authors(None, &["M.Ota"])
        .nothing_else();

    // van der
    assert_authorship("(van der Wulp, 1885)", &[])
        .bas_authors(Some("1885"), &["van der Wulp"])
        .nothing_else();

    // https://www.ipni.org/a/40285-1
    assert_authorship("Viane & Van den heede", &[])
        .comb_authors(None, &["Viane", "Van den heede"])
        .nothing_else();

    assert_authorship("van den Brink", &[])
        .comb_authors(None, &["van den Brink"])
        .nothing_else();

    assert_authorship("Van de Kerckh.", &[])
        .comb_authors(None, &["Van de Kerckh."])
        .nothing_else();

    assert_authorship("Van de Putte", &[])
        .comb_authors(None, &["Van de Putte"])
        .nothing_else();

    assert_authorship("Van Dersal", &[])
        .comb_authors(None, &["Van Dersal"])
        .nothing_else();

    // turkish chars
    assert_authorship("Ilçim, Çenet & Dadandi", &[])
        .comb_authors(None, &["Ilçim", "Çenet", "Dadandi"])
        .nothing_else();

    assert_authorship("S. Yildirimli", &[])
        .comb_authors(None, &["S.Yildirimli"])
        .nothing_else();

    assert_authorship("Şahin, Koca & Yildirim, 2012", &[])
        .comb_authors(Some("2012"), &["Şahin", "Koca", "Yildirim"])
        .nothing_else();

    assert_authorship("L.f", &[])
        .comb_authors(None, &["L.f"])
        .nothing_else();

    assert_authorship("(L.) G. Don filius", &[])
        .bas_authors(None, &["L."])
        .comb_authors(None, &["G.Don filius"])
        .nothing_else();

    assert_authorship("(L.) G. Don fil.", &[])
        .bas_authors(None, &["L."])
        .comb_authors(None, &["G.Don fil."])
        .nothing_else();

    assert_authorship("d'Urv.", &[])
        .comb_authors(None, &["d'Urv."])
        .nothing_else();

    assert_authorship("Balsamo M Fregni E Tongiorgi P", &[])
        .comb_authors(None, &["M.Balsamo", "E.Fregni", "P.Tongiorgi"])
        .nothing_else();

    assert_authorship("Balsamo M Todaro MA", &[])
        .comb_authors(None, &["M.Balsamo", "M.A.Todaro"])
        .nothing_else();

    assert_authorship("Cushman Em. Sellier de Civrieux, 1976", &[])
        .comb_authors(Some("1976"), &["Cushman Em.Sellier de Civrieux"])
        .nothing_else();

    // http://dev.gbif.org/issues/browse/POR-101
    assert_authorship("la Croix & P.J.Cribb", &[])
        .comb_authors(None, &["la Croix", "P.J.Cribb"])
        .nothing_else();

    assert_authorship("le Croix & P.J.Cribb", &[])
        .comb_authors(None, &["le Croix", "P.J.Cribb"])
        .nothing_else();

    assert_authorship("de la Croix & le P.J.Cribb", &[])
        .comb_authors(None, &["de la Croix", "le P.J.Cribb"])
        .nothing_else();

    // Direct-parse fallback: `.doubtful()`/`.warning(...)` read fields the shared
    // `assert_authorship` DSL doesn't forward onto its returned `NameAssertion`.
    let n = nameparser::parse_name(
        "Abies alba",
        Some("Istv?nffi, 1898"),
        Some(Rank::Species),
        None,
    )
    .unwrap_or_else(|e| panic!("authorship should parse: {e:?}"));
    assert_eq!(n.combination_authorship.year.as_deref(), Some("1898"));
    assert_eq!(
        n.combination_authorship.authors,
        vec!["Istvnffi".to_string()]
    );
    assert!(n.doubtful);
    assert_eq!(
        n.warnings,
        vec![warnings::QUESTION_MARKS_REMOVED.to_string()]
    );

    assert_authorship("F.S.Castracane degli Antelminelli", &[])
        .comb_authors(None, &["F.S.Castracane degli Antelminelli"])
        .nothing_else();

    assert_authorship("De la Soie", &[])
        .comb_authors(None, &["De la Soie"])
        .nothing_else();

    assert_ex_authorship("Hort. ex Vilmorin", Some("hort."), &[])
        .comb_authors(None, &["Vilmorin"])
        .comb_ex_authors(&["hort."])
        .nothing_else();

    assert_ex_authorship("hortusa ex K. Koch", Some("hort."), &[])
        .comb_authors(None, &["K.Koch"])
        .comb_ex_authors(&["hort."])
        .nothing_else();

    assert_ex_authorship("hortus ex K. Koch", Some("hort."), &[])
        .comb_authors(None, &["K.Koch"])
        .comb_ex_authors(&["hort."])
        .nothing_else();

    assert_authorship("(Thunberg) A.P.de Candolle", &[])
        .bas_authors(None, &["Thunberg"])
        .comb_authors(None, &["A.P.de Candolle"])
        .nothing_else();

    assert_authorship(
        "(Huguet del Villar) S. Rivas-Martínez, F. Fernández González & D. Sánchez-Mata",
        &[],
    )
    .bas_authors(None, &["Huguet del Villar"])
    .comb_authors(
        None,
        &["S.Rivas-Martínez", "F.Fernández González", "D.Sánchez-Mata"],
    )
    .nothing_else();

    assert_authorship("(H. da C. Monteiro Filho) H. da C. Monteiro Filho", &[])
        .bas_authors(None, &["H.da C.Monteiro Filho"])
        .comb_authors(None, &["H.da C.Monteiro Filho"])
        .nothing_else();
}

#[test]
fn test_phrase_names() {
    assert_phrase_name(
        "Pultenaea sp. 'Olinda' (Coveny 6616)",
        "Pultenaea sp. 'Olinda' (Coveny 6616)",
        Some(Rank::Species),
        "'Olinda' (Coveny 6616)",
    );
    assert_phrase_name(
        "Marsilea sp. Neutral Junction (D.E.Albrecht 9192)",
        "Marsilea sp. Neutral Junction (D.E.Albrecht 9192)",
        Some(Rank::Species),
        "Neutral Junction (D.E.Albrecht 9192)",
    );
    assert_phrase_name(
        "Dampiera sp. Central Wheatbelt (L.W.Sage, F.Hort, C.A.Hollister LWS2321)",
        "Dampiera sp. Central Wheatbelt (L.W.Sage, F.Hort, C.A.Hollister LWS2321)",
        Some(Rank::Species),
        "Central Wheatbelt (L.W.Sage, F.Hort, C.A.Hollister LWS2321)",
    );
    assert_phrase_name(
        "Baeckea ssp. 2 (LJM 2019)",
        "Baeckea subsp. 2 (LJM 2019)",
        Some(Rank::Subspecies),
        "2 (LJM 2019)",
    );
    assert_phrase_name(
        "Baeckea var 2 (LJM 2019)",
        "Baeckea var. 2 (LJM 2019)",
        Some(Rank::Variety),
        "2 (LJM 2019)",
    );
    assert_phrase_name(
        "Baeckea sp. Bunney Road (S.Patrick 4059)",
        "Baeckea sp. Bunney Road (S.Patrick 4059)",
        Some(Rank::Species),
        "Bunney Road (S.Patrick 4059)",
    );
    assert_phrase_name(
        "Prostanthera sp. Bundjalung Nat. Pk. (B.J.Conn 3471)",
        "Prostanthera sp. Bundjalung Nat. Pk. (B.J.Conn 3471)",
        Some(Rank::Species),
        "Bundjalung Nat. Pk. (B.J.Conn 3471)",
    );
    assert_phrase_name(
        "Toechima sp. East Alligator (J.Russell-Smith 8418) NT Herbarium",
        "Toechima sp. East Alligator (J.Russell-Smith 8418) NT Herbarium",
        Some(Rank::Species),
        "East Alligator (J.Russell-Smith 8418) NT Herbarium",
    );
    assert_phrase_name(
        "Goodenia sp. Bachsten Creek (M.D. Barrett 685) WA Herbarium",
        "Goodenia sp. Bachsten Creek (M.D. Barrett 685) WA Herbarium",
        Some(Rank::Species),
        "Bachsten Creek (M.D. Barrett 685) WA Herbarium",
    );
    assert_phrase_name(
        "Baeckea sp. Beringbooding (AR Main 11/9/1957)",
        "Baeckea sp. Beringbooding (AR Main 11/9/1957)",
        Some(Rank::Species),
        "Beringbooding (AR Main 11/9/1957)",
    );
    assert_phrase_name(
        "Sida sp. Walhallow Station (C.Edgood 28/Oct/94)",
        "Sida sp. Walhallow Station (C.Edgood 28/Oct/94)",
        Some(Rank::Species),
        "Walhallow Station (C.Edgood 28/Oct/94)",
    );
    assert_phrase_name(
        "Elaeocarpus sp. Rocky Creek (Hunter s.n., 16 Sep 1993)",
        "Elaeocarpus sp. Rocky Creek (Hunter s.n., 16 Sep 1993)",
        Some(Rank::Species),
        "Rocky Creek (Hunter s.n., 16 Sep 1993)",
    );
    assert_phrase_name(
        "Sida sp. B (C.Dunlop 1739)",
        "Sida sp. B (C.Dunlop 1739)",
        Some(Rank::Species),
        "B (C.Dunlop 1739)",
    );
    assert_phrase_name(
        "Grevillea brachystylis subsp. Busselton (G.J.Keighery s.n. 28/8/1985)",
        "Grevillea brachystylis ssp. Busselton (G.J.Keighery s.n. 28/8/1985)",
        Some(Rank::Subspecies),
        "Busselton (G.J.Keighery s.n. 28/8/1985)",
    );
    assert_phrase_name(
        "Baeckea sp. Calingiri (F.Hort 1710)",
        "Baeckea sp. Calingiri (F.Hort 1710)",
        Some(Rank::Species),
        "Calingiri (F.Hort 1710)",
    );
    assert_phrase_name(
        "Baeckea sp. East Yuna (R Spjut & C Edson 7077)",
        "Baeckea sp. East Yuna (R Spjut & C Edson 7077)",
        Some(Rank::Species),
        "East Yuna (R Spjut & C Edson 7077)",
    );
    assert_phrase_name(
        "Acacia sp. Goodlands (BR Maslin 7761) [aff. resinosa]",
        "Acacia sp. Goodlands (BR Maslin 7761) [aff. resinosa]",
        Some(Rank::Species),
        "Goodlands (BR Maslin 7761) [aff. resinosa]",
    );
    assert_phrase_name(
        "Acacia sp. Manmanning (BR Maslin 7711) [aff. multispicata]",
        "Acacia sp. Manmanning (BR Maslin 7711) [aff. multispicata]",
        Some(Rank::Species),
        "Manmanning (BR Maslin 7711) [aff. multispicata]",
    );
    let na = assert_phrase_name(
        "Atrichornis (Rahcinta) sp Glory (BR Maslin 7711)",
        "Atrichornis sp. Glory (BR Maslin 7711)",
        Some(Rank::Species),
        "Glory (BR Maslin 7711)",
    );
    na.infrageneric("Rahcinta");
    assert_phrase_name(
        "Acacia mutabilis subsp. Young River (G.F.Craig 2052)",
        "Acacia mutabilis ssp. Young River (G.F.Craig 2052)",
        Some(Rank::Subspecies),
        "Young River (G.F.Craig 2052)",
    );
    assert_phrase_name(
        "Acacia mutabilis Maslin subsp. Young River (G.F.Craig 2052)",
        "Acacia mutabilis ssp. Young River (G.F.Craig 2052)",
        Some(Rank::Subspecies),
        "Young River (G.F.Craig 2052)",
    )
    .comb_authors(None, &["Maslin"]);
    assert_phrase_name(
        "Acacia sp. \"Morning Glory\"",
        "Acacia sp. \"Morning Glory\"",
        Some(Rank::Species),
        "\"Morning Glory\"",
    );
}

#[test]
fn test_nomenclatural_notes_pattern() {
    // author only
    // Java calls `parser.parseAuthorship("nom. illeg.", null)` here — parsing JUST the
    // authorship text (no name at all). The Rust engine has no `parseAuthorship` entry point
    // (only `parse(name, authorship, rank, code)`), so this reuses the shared DSL's own
    // "Abies alba" placeholder-name approximation. Java's `na.type(null)` assertion — the fresh
    // `ParsedName`'s `type` field, left at Java's bare-field `null` default since
    // `ParsedAuthorship.copy()` never touches `type` — has no Rust equivalent (`NameType` is a
    // plain, non-nullable enum here), so that one check is dropped; the nomenclatural-note check
    // it chains is preserved below.
    let n = nameparser::parse_name("Abies alba", Some("nom. illeg."), Some(Rank::Species), None)
        .unwrap_or_else(|e| panic!("authorship `nom. illeg.` should parse: {e:?}"));
    assert_eq!(n.nomenclatural_note.as_deref(), Some("nom. illeg."));

    assert_nom_note(
        "nom. illeg.",
        "Vaucheria longicaulis var. bengalensis Islam, nom. illeg.",
    );
    assert_nom_note("nom. correct", "Dorataspidae nom. correct");
    assert_nom_note("nom. transf.", "Ethmosphaeridae nom. transf.");
    assert_nom_note("nom. ambig.", "Fucus ramosissimus Oeder, nom. ambig.");
    assert_nom_note("nom. nov.", "Myrionema majus Foslie, nom. nov.");
    assert_nom_note(
        "nom. utique rej.",
        "Corydalis bulbosa (L.) DC., nom. utique rej.",
    );
    assert_nom_note(
        "nom. cons. prop.",
        "Anthoceros agrestis var. agrestis Paton nom. cons. prop.",
    );
    assert_nom_note(
        "nom. superfl.",
        "Lithothamnion glaciale forma verrucosum (Foslie) Foslie, nom. superfl.",
    );
    assert_nom_note(
        "nom. rejic.",
        "Pithecellobium montanum var. subfalcatum (Zoll. & Moritzi)Miq., nom.rejic.",
    );
    assert_nom_note(
        "nom. inval.",
        "Fucus vesiculosus forma volubilis (Goodenough & Woodward) H.T. Powell, nom. inval",
    );
    assert_nom_note(
        "nom. nud.",
        "Sao hispanica R. & E. Richter nom. nud. in Sampelayo 1935",
    );
    assert_nom_note("nom. illeg.", "Hallo (nom.illeg.)");
    assert_nom_note(
        "nom. super.",
        "Calamagrostis cinnoides W. Bart. nom. super.",
    );
    assert_nom_note(
        "nom. nud.",
        "Iridaea undulosa var. papillosa Bory de Saint-Vincent, nom. nud.",
    );
    assert_nom_note(
        "nom. inval.",
        "Sargassum angustifolium forma filiforme V. Krishnamurthy & H. Joshi, nom. inval",
    );
    assert_nom_note("nomen nudum", "Solanum bifidum Vell. ex Dunal, nomen nudum");
    assert_nom_note(
        "nomen invalid",
        "Schoenoplectus ×scheuchzeri (Bruegger) Palla ex Janchen, nomen invalid.",
    );
    assert_nom_note(
        "nom. nud.",
        "Cryptomys \"Kasama\" Kawalika et al., 2001, nom. nud. (Kasama, Zambia) .",
    );
    assert_nom_note(
        "nom. super.",
        "Calamagrostis cinnoides W. Bart. nom. super.",
    );
    assert_nom_note("nom. dub.", "Pandanus odorifer (Forssk.) Kuntze, nom. dub.");
    assert_nom_note("nom. rejic.", "non Clarisia Abat, 1792, nom. rejic.");
    assert_nom_note(
        "nom. cons.",
        "Yersinia pestis (Lehmann and Neumann, 1896) van Loghem, 1944 (Approved Lists, 1980) , nom. cons",
    );
    assert_nom_note(
        "nom. rejic.",
        "\"Pseudomonas denitrificans\" (Christensen, 1903) Bergey et al., 1923, nom. rejic.",
    );
    assert_nom_note("nom. nov.", "Tipula rubiginosa Loew, 1863, nom. nov.");
    assert_nom_note(
        "nom. prov.",
        "Amanita pruittii A.H.Sm. ex Tulloss & J.Lindgr., nom. prov.",
    );
    assert_nom_note("nom. cons.", "Ramonda Rich., nom. cons.");
    assert_nom_note(
        "nom. cons.",
        "Kluyver and van Niel, 1936 emend. Barker, 1956 (Approved Lists, 1980) , nom. cons., emend. Mah and Kuhn, 1984",
    );
    assert_nom_note(
        "nom. superfl.",
        "Coccocypselum tontanea (Aubl.) Kunth, nom. superfl.",
    );
    assert_nom_note(
        "nom. ambig.",
        "Lespedeza bicolor var. intermedia Maxim. , nom. ambig.",
    );
    assert_nom_note(
        "nom. praeoccup.",
        "Erebia aethiops uralensis Goltz, 1930 nom. praeoccup.",
    );
    assert_nom_note(
        "comb. nov. ined.",
        "Ipomopsis tridactyla (Rydb.) Wilken, comb. nov. ined.",
    );
    assert_nom_note(
        "sp. nov. ined.",
        "Orobanche riparia Collins, sp. nov. ined.",
    );
    assert_nom_note(
        "gen. nov.",
        "Anchimolgidae gen. nov. New Caledonia-Rjh-, 2004",
    );
    assert_nom_note("gen. nov. ined.", "Stebbinsoseris gen. nov. ined.");
    assert_nom_note("var. nov.", "Euphorbia rossiana var. nov. Steinmann, 1199");
}

/// http://dev.gbif.org/issues/browse/POR-2454
#[test]
fn fungus_names() {
    assert_name("Merulius lacrimans (Wulfen : Fr.) Schum.")
        .species("Merulius", "lacrimans")
        .comb_authors(None, &["Schum."])
        .bas_authors(None, &["Wulfen"])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Merulius lacrimans (Wulfen) Schum. : Fr.")
        .species("Merulius", "lacrimans")
        .comb_authors(None, &["Schum."])
        .bas_authors(None, &["Wulfen"])
        .sanct_author("Fr.")
        .code(NomCode::Botanical)
        .nothing_else();

    //assertParsedParts("", null, "Merulius", "lacrimans", null, null, "Schum.", null, "Wulfen : Fr.", null);
    //assertParsedParts("Aecidium berberidis Pers. ex J.F. Gmel.", null, "Aecidium", "berberidis", null, null, "Pers. ex J.F. Gmel.", null, null, null);
    //assertParsedParts("Roestelia penicillata (O.F. Müll.) Fr.", null, "Roestelia", "penicillata", null, null, "Fr.", null, "O.F. Müll.", null);
    //
    //assertParsedParts("Mycosphaerella eryngii (Fr. Duby) ex Oudem., 1897", null, "Mycosphaerella", "eryngii", null, null, "ex Oudem.", "1897", "Fr. Duby", null);
    //assertParsedParts("Mycosphaerella eryngii (Fr.ex Duby) ex Oudem. 1897", null, "Mycosphaerella", "eryngii", null, null, "ex Oudem.", "1897", "Fr.ex Duby", null);
    //assertParsedParts("Mycosphaerella eryngii (Fr. ex Duby) Johanson ex Oudem. 1897", null, "Mycosphaerella", "eryngii", null, null, "Johanson ex Oudem.", "1897", "Fr. ex Duby", null);
}

#[test]
fn year_variations() {
    // The bracketed [1912] is the imprint year; the author span has no nominal
    // publication year, so code can't be inferred from authorship and the trinomial
    // stays INFRASPECIFIC_NAME.
    assert_name("Deudorix epijarbas turbo Fruhstorfer, [1912]")
        .infra_species("Deudorix", "epijarbas", Rank::InfraspecificName, "turbo")
        .comb_authors(None, &["Fruhstorfer"])
        .imprint_year("1912")
        .nothing_else();
}

// ---- local helpers: one DSL gap not covered by `common::` --------------------------------------

/// `assertAuthorship(raw).combAuthors(year, authors...).sensu(sensu)[.nomNote(note)].nothingElse()`
/// — Java's real `assertAuthorship`/`assertExAuthorship` call `parser.parseAuthorship(...)`,
/// whose returned `ParsedAuthorship` carries `taxonomicNote`/`nomenclaturalNote` too (copied in
/// full by `NameAssertion(ParsedAuthorship)`). The shared `common::assert_authorship` DSL parses
/// the same way (`"Abies alba"` + the separately supplied authorship) but its returned
/// `NameAssertion` only forwards combination/basionym/sanctioning authorship, dropping notes — so
/// `authorshipOnlyNotes` (the one Java test whose whole point is a note living inside a bare
/// authorship string) reads them straight off the full parse here instead.
fn assert_authorship_note(
    raw: &str,
    year: Option<&str>,
    authors: &[&str],
    sensu: &str,
    nom_note: Option<&str>,
) {
    let n = nameparser::parse_name("Abies alba", Some(raw), Some(Rank::Species), None)
        .unwrap_or_else(|e| panic!("authorship `{raw}` should parse: {e:?}"));
    assert_eq!(
        n.combination_authorship.year.as_deref(),
        year,
        "year mismatch for `{raw}`"
    );
    assert_eq!(
        n.combination_authorship.authors,
        authors.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        "authors mismatch for `{raw}`"
    );
    assert_eq!(
        n.taxonomic_note.as_deref(),
        Some(sensu),
        "sensu mismatch for `{raw}`"
    );
    assert_eq!(
        n.nomenclatural_note.as_deref(),
        nom_note,
        "nomNote mismatch for `{raw}`"
    );
}
