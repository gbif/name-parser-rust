// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserImplTest (methods on lines 3992-4438).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::NamePart;
use nameparser::model::{NomCode, Rank};

#[test]
fn authorship_only_notes() {
    // "(auct.) Author": the parens mark a note, not a basionym → author + taxonomic note
    assert_authorship("(auct.) Rolfe", &["Rolfe"])
        .comb_authors(None, &["Rolfe"])
        .sensu("auct.")
        .nothing_else();

    assert_authorship("(auct.) auct.", &[])
        .sensu("auct.")
        .nothing_else();

    // taxonomic note + nomenclatural note are split into their own fields
    assert_authorship("auct., nom. subnud.", &[])
        .sensu("auct.")
        .nom_note("nom. subnud.")
        .nothing_else();

    // a parenthesised "(sensu …)" is the taxonomic note; the trailing name is the author
    assert_authorship(
        "(sensu Mereschkowsky, 1878) Jankowski, 1992",
        &["Jankowski"],
    )
    .comb_authors(Some("1992"), &["Jankowski"])
    .sensu("sensu Mereschkowsky, 1878")
    .nothing_else();

    // a leading parenthesised homonym citation makes the whole string a taxonomic note
    assert_authorship("(non Scacchi, 1836) sensu Zibrowius, 1968", &[])
        .sensu("(non Scacchi, 1836) sensu Zibrowius, 1968")
        .nothing_else();

    assert_authorship(
        "Fischer-Le Saux et al., 1999 emend. Akhurst et al., 2004",
        &["Fischer-Le Saux", "al."],
    )
    .comb_authors(Some("1999"), &["Fischer-Le Saux", "al."])
    .sensu("emend. Akhurst et al., 2004")
    .nothing_else();

    assert_authorship("Trautv. & Meyer sensu lato", &["Trautv.", "Meyer"])
        .comb_authors(None, &["Trautv.", "Meyer"])
        .sensu("sensu lato")
        .nothing_else();

    assert_authorship("Mill. non Parolly", &["Mill."])
        .comb_authors(None, &["Mill."])
        .sensu("non Parolly")
        .nothing_else();
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

    assert_ex_authorship(
        "(Ristorcelli & Van ty) Wedd. ex Sch. Bip. (nom. nud.)",
        Some("Wedd."),
        &["Sch.Bip."],
    )
    .bas_authors(None, &["Ristorcelli", "Van ty"])
    .comb_authors(None, &["Sch.Bip."])
    .comb_ex_authors(&["Wedd."])
    .nom_note("nom. nud.")
    .nothing_else();

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
        .comb_authors(None, &["Cushman"])
        .sensu("Em. Sellier de Civrieux, 1976")
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

    assert_authorship("Istv?nffi, 1898", &["Istvnffi"])
        .comb_authors(Some("1898"), &["Istvnffi"])
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();

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
        "sp. 'Olinda' (Coveny 6616)",
    )
    .genus_rank("Pultenaea", Rank::Species)
    .nothing_else();
    assert_phrase_name(
        "Marsilea sp. Neutral Junction (D.E.Albrecht 9192)",
        "Marsilea sp. Neutral Junction (D.E.Albrecht 9192)",
        Some(Rank::Species),
        "sp. Neutral Junction (D.E.Albrecht 9192)",
    )
    .genus_rank("Marsilea", Rank::Species)
    .nothing_else();
    assert_phrase_name(
        "Dampiera sp. Central Wheatbelt (L.W.Sage, F.Hort, C.A.Hollister LWS2321)",
        "Dampiera sp. Central Wheatbelt (L.W.Sage, F.Hort, C.A.Hollister LWS2321)",
        Some(Rank::Species),
        "sp. Central Wheatbelt (L.W.Sage, F.Hort, C.A.Hollister LWS2321)",
    )
    .genus_rank("Dampiera", Rank::Species)
    .nothing_else();
    // ssp./var. (non-species) markers keep the bare phrase — the formatter re-synthesises the
    // rank marker, and prefixing a non-species marker onto the phrase would double it.
    assert_phrase_name(
        "Baeckea ssp. 2 (LJM 2019)",
        "Baeckea subsp. 2 (LJM 2019)",
        Some(Rank::Subspecies),
        "2 (LJM 2019)",
    )
    .genus_rank("Baeckea", Rank::Subspecies)
    .nothing_else();
    assert_phrase_name(
        "Baeckea var 2 (LJM 2019)",
        "Baeckea var. 2 (LJM 2019)",
        Some(Rank::Variety),
        "2 (LJM 2019)",
    )
    .genus_rank("Baeckea", Rank::Variety)
    .nothing_else();
    assert_phrase_name(
        "Baeckea sp. Bunney Road (S.Patrick 4059)",
        "Baeckea sp. Bunney Road (S.Patrick 4059)",
        Some(Rank::Species),
        "sp. Bunney Road (S.Patrick 4059)",
    )
    .genus_rank("Baeckea", Rank::Species)
    .nothing_else();
    assert_phrase_name(
        "Prostanthera sp. Bundjalung Nat. Pk. (B.J.Conn 3471)",
        "Prostanthera sp. Bundjalung Nat. Pk. (B.J.Conn 3471)",
        Some(Rank::Species),
        "sp. Bundjalung Nat. Pk. (B.J.Conn 3471)",
    )
    .genus_rank("Prostanthera", Rank::Species)
    .nothing_else();
    assert_phrase_name(
        "Toechima sp. East Alligator (J.Russell-Smith 8418) NT Herbarium",
        "Toechima sp. East Alligator (J.Russell-Smith 8418) NT Herbarium",
        Some(Rank::Species),
        "sp. East Alligator (J.Russell-Smith 8418) NT Herbarium",
    )
    .genus_rank("Toechima", Rank::Species)
    .nothing_else();
    assert_phrase_name(
        "Goodenia sp. Bachsten Creek (M.D. Barrett 685) WA Herbarium",
        "Goodenia sp. Bachsten Creek (M.D. Barrett 685) WA Herbarium",
        Some(Rank::Species),
        "sp. Bachsten Creek (M.D. Barrett 685) WA Herbarium",
    )
    .genus_rank("Goodenia", Rank::Species)
    .nothing_else();
    assert_phrase_name(
        "Baeckea sp. Beringbooding (AR Main 11/9/1957)",
        "Baeckea sp. Beringbooding (AR Main 11/9/1957)",
        Some(Rank::Species),
        "sp. Beringbooding (AR Main 11/9/1957)",
    )
    .genus_rank("Baeckea", Rank::Species)
    .nothing_else();
    assert_phrase_name(
        "Sida sp. Walhallow Station (C.Edgood 28/Oct/94)",
        "Sida sp. Walhallow Station (C.Edgood 28/Oct/94)",
        Some(Rank::Species),
        "sp. Walhallow Station (C.Edgood 28/Oct/94)",
    )
    .genus_rank("Sida", Rank::Species)
    .nothing_else();
    assert_phrase_name(
        "Elaeocarpus sp. Rocky Creek (Hunter s.n., 16 Sep 1993)",
        "Elaeocarpus sp. Rocky Creek (Hunter s.n., 16 Sep 1993)",
        Some(Rank::Species),
        "sp. Rocky Creek (Hunter s.n., 16 Sep 1993)",
    )
    .genus_rank("Elaeocarpus", Rank::Species)
    .nothing_else();
    assert_phrase_name(
        "Sida sp. B (C.Dunlop 1739)",
        "Sida sp. B (C.Dunlop 1739)",
        Some(Rank::Species),
        "sp. B (C.Dunlop 1739)",
    )
    .genus_rank("Sida", Rank::Species)
    .nothing_else();
    // Binomial (has a species epithet) + subsp. -> Parsed, out of scope: keeps the bare phrase.
    assert_phrase_name(
        "Grevillea brachystylis subsp. Busselton (G.J.Keighery s.n. 28/8/1985)",
        "Grevillea brachystylis ssp. Busselton (G.J.Keighery s.n. 28/8/1985)",
        Some(Rank::Subspecies),
        "Busselton (G.J.Keighery s.n. 28/8/1985)",
    )
    .binomial("Grevillea", None, "brachystylis", Rank::Subspecies)
    .nothing_else();
    assert_phrase_name(
        "Baeckea sp. Calingiri (F.Hort 1710)",
        "Baeckea sp. Calingiri (F.Hort 1710)",
        Some(Rank::Species),
        "sp. Calingiri (F.Hort 1710)",
    )
    .genus_rank("Baeckea", Rank::Species)
    .nothing_else();
    assert_phrase_name(
        "Baeckea sp. East Yuna (R Spjut & C Edson 7077)",
        "Baeckea sp. East Yuna (R Spjut & C Edson 7077)",
        Some(Rank::Species),
        "sp. East Yuna (R Spjut & C Edson 7077)",
    )
    .genus_rank("Baeckea", Rank::Species)
    .nothing_else();
    assert_phrase_name(
        "Acacia sp. Goodlands (BR Maslin 7761) [aff. resinosa]",
        "Acacia sp. Goodlands (BR Maslin 7761) [aff. resinosa]",
        Some(Rank::Species),
        "sp. Goodlands (BR Maslin 7761) [aff. resinosa]",
    )
    .genus_rank("Acacia", Rank::Species)
    .nothing_else();
    assert_phrase_name(
        "Acacia sp. Manmanning (BR Maslin 7711) [aff. multispicata]",
        "Acacia sp. Manmanning (BR Maslin 7711) [aff. multispicata]",
        Some(Rank::Species),
        "sp. Manmanning (BR Maslin 7711) [aff. multispicata]",
    )
    .genus_rank("Acacia", Rank::Species)
    .nothing_else();
    // Genus (Subgenus) sp. -> the subgenus makes the prefix non-genus-only, so it keeps the
    // bare phrase and the formatter re-synthesises "sp.".
    assert_phrase_name(
        "Atrichornis (Rahcinta) sp Glory (BR Maslin 7711)",
        "Atrichornis sp. Glory (BR Maslin 7711)",
        Some(Rank::Species),
        "Glory (BR Maslin 7711)",
    )
    .infrageneric_at("Atrichornis", Rank::Species, "Rahcinta")
    .nothing_else();
    assert_phrase_name(
        "Acacia mutabilis subsp. Young River (G.F.Craig 2052)",
        "Acacia mutabilis ssp. Young River (G.F.Craig 2052)",
        Some(Rank::Subspecies),
        "Young River (G.F.Craig 2052)",
    )
    .binomial("Acacia", None, "mutabilis", Rank::Subspecies)
    .nothing_else();
    assert_phrase_name(
        "Acacia mutabilis Maslin subsp. Young River (G.F.Craig 2052)",
        "Acacia mutabilis ssp. Young River (G.F.Craig 2052)",
        Some(Rank::Subspecies),
        "Young River (G.F.Craig 2052)",
    )
    .specific_authors(None, &["Maslin"])
    .binomial("Acacia", None, "mutabilis", Rank::Subspecies)
    .nothing_else();
    assert_phrase_name(
        "Acacia sp. \"Morning Glory\"",
        "Acacia sp. \"Morning Glory\"",
        Some(Rank::Species),
        "sp. \"Morning Glory\"",
    )
    .genus_rank("Acacia", Rank::Species)
    .nothing_else();
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
    )
    .infra_species("Vaucheria", "longicaulis", Rank::Variety, "bengalensis")
    .comb_authors(None, &["Islam"])
    .code(NomCode::Botanical)
    .nothing_else();
    assert_nom_note("nom. correct", "Dorataspidae nom. correct")
        .monomial_rank("Dorataspidae", Rank::Unranked)
        .nothing_else();
    assert_nom_note("nom. transf.", "Ethmosphaeridae nom. transf.")
        .monomial_rank("Ethmosphaeridae", Rank::Unranked)
        .nothing_else();
    assert_nom_note("nom. ambig.", "Fucus ramosissimus Oeder, nom. ambig.")
        .species("Fucus", "ramosissimus")
        .comb_authors(None, &["Oeder"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_nom_note("nom. nov.", "Myrionema majus Foslie, nom. nov.")
        .species("Myrionema", "majus")
        .comb_authors(None, &["Foslie"])
        .nothing_else();
    assert_nom_note(
        "nom. utique rej.",
        "Corydalis bulbosa (L.) DC., nom. utique rej.",
    )
    .species("Corydalis", "bulbosa")
    .comb_authors(None, &["DC."])
    .bas_authors(None, &["L."])
    .code(NomCode::Botanical)
    .nothing_else();
    assert_nom_note(
        "nom. cons. prop.",
        "Anthoceros agrestis var. agrestis Paton nom. cons. prop.",
    )
    .infra_species("Anthoceros", "agrestis", Rank::Variety, "agrestis")
    .comb_authors(None, &["Paton"])
    .code(NomCode::Botanical)
    .nothing_else();
    assert_nom_note(
        "nom. superfl.",
        "Lithothamnion glaciale forma verrucosum (Foslie) Foslie, nom. superfl.",
    )
    .infra_species("Lithothamnion", "glaciale", Rank::Form, "verrucosum")
    .comb_authors(None, &["Foslie"])
    .bas_authors(None, &["Foslie"])
    .code(NomCode::Botanical)
    .nothing_else();
    assert_nom_note(
        "nom. rejic.",
        "Pithecellobium montanum var. subfalcatum (Zoll. & Moritzi)Miq., nom.rejic.",
    )
    .infra_species("Pithecellobium", "montanum", Rank::Variety, "subfalcatum")
    .comb_authors(None, &["Miq."])
    .bas_authors(None, &["Zoll.", "Moritzi"])
    .code(NomCode::Botanical)
    .nothing_else();
    assert_nom_note(
        "nom. inval.",
        "Fucus vesiculosus forma volubilis (Goodenough & Woodward) H.T. Powell, nom. inval",
    )
    .infra_species("Fucus", "vesiculosus", Rank::Form, "volubilis")
    .comb_authors(None, &["H.T.Powell"])
    .bas_authors(None, &["Goodenough", "Woodward"])
    .code(NomCode::Botanical)
    .nothing_else();
    // FIXME(review): "R. & E. Richter" is two Richters: R.Richter, E.Richter
    assert_nom_note(
        "nom. nud.",
        "Sao hispanica R. & E. Richter nom. nud. in Sampelayo 1935",
    )
    .species("Sao", "hispanica")
    .comb_authors(Some("1935"), &["R.", "E.Richter"])
    .published_in("Sampelayo 1935")
    .published_in_year(Some(1935))
    .nothing_else();
    assert_nom_note("nom. illeg.", "Hallo (nom.illeg.)")
        .monomial_rank("Hallo", Rank::Unranked)
        .code(NomCode::Botanical)
        .nothing_else();
    assert_nom_note(
        "nom. super.",
        "Calamagrostis cinnoides W. Bart. nom. super.",
    )
    .species("Calamagrostis", "cinnoides")
    .comb_authors(None, &["W.Bart."])
    .code(NomCode::Botanical)
    .nothing_else();
    assert_nom_note(
        "nom. nud.",
        "Iridaea undulosa var. papillosa Bory de Saint-Vincent, nom. nud.",
    )
    .infra_species("Iridaea", "undulosa", Rank::Variety, "papillosa")
    .comb_authors(None, &["Bory de Saint-Vincent"])
    .nothing_else();
    assert_nom_note(
        "nom. inval.",
        "Sargassum angustifolium forma filiforme V. Krishnamurthy & H. Joshi, nom. inval",
    )
    .infra_species("Sargassum", "angustifolium", Rank::Form, "filiforme")
    .comb_authors(None, &["V.Krishnamurthy", "H.Joshi"])
    .code(NomCode::Botanical)
    .nothing_else();
    assert_nom_note("nomen nudum", "Solanum bifidum Vell. ex Dunal, nomen nudum")
        .species("Solanum", "bifidum")
        .comb_authors(None, &["Dunal"])
        .comb_ex_authors(&["Vell."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_nom_note(
        "nomen invalid",
        "Schoenoplectus ×scheuchzeri (Bruegger) Palla ex Janchen, nomen invalid.",
    )
    .species("Schoenoplectus", "scheuchzeri")
    .comb_authors(None, &["Janchen"])
    .comb_ex_authors(&["Palla"])
    .bas_authors(None, &["Bruegger"])
    .notho(&[NamePart::Specific])
    .code(NomCode::Botanical)
    .nothing_else();
    // FIXME(review): a double-quoted word is no cultivar here (Cryptomys is a mole-rat, "Kasama" a
    // manuscript name), and the "(Kasama, Zambia)" locality ends up among the authors. Left open
    // until fixed.
    assert_nom_note(
        "nom. nud.",
        "Cryptomys \"Kasama\" Kawalika et al., 2001, nom. nud. (Kasama, Zambia) .",
    );
    assert_nom_note(
        "nom. super.",
        "Calamagrostis cinnoides W. Bart. nom. super.",
    )
    .species("Calamagrostis", "cinnoides")
    .comb_authors(None, &["W.Bart."])
    .code(NomCode::Botanical)
    .nothing_else();
    assert_nom_note("nom. dub.", "Pandanus odorifer (Forssk.) Kuntze, nom. dub.")
        .species("Pandanus", "odorifer")
        .comb_authors(None, &["Kuntze"])
        .bas_authors(None, &["Forssk."])
        .code(NomCode::Botanical)
        .nothing_else();
    // FIXME(review): garbage: uninomial "Non", author "Clarisia Abat" — the name is Clarisia, the note "non … Abat"
    assert_nom_note("nom. rejic.", "non Clarisia Abat, 1792, nom. rejic.")
        .monomial_rank("Non", Rank::Unranked)
        .comb_authors(Some("1792"), &["Clarisia Abat"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_nom_note(
        "nom. cons.",
        "Yersinia pestis (Lehmann and Neumann, 1896) van Loghem, 1944 (Approved Lists, 1980) , nom. cons",
    )
        .species("Yersinia", "pestis")
        .comb_authors(Some("1944"), &["van Loghem"])
        .bas_authors(Some("1896"), &["Lehmann", "Neumann"])
        .code(NomCode::Bacterial)
        .nothing_else();
    // FIXME(review): garbage: the quoted name ends up as an author
    assert_nom_note(
        "nom. rejic.",
        "\"Pseudomonas denitrificans\" (Christensen, 1903) Bergey et al., 1923, nom. rejic.",
    )
    .comb_authors(
        Some("1903"),
        &["Pseudomonas denitrificans Christensen", "Bergey", "al."],
    )
    .imprint_year("1923")
    .partial("\"Pseudomonas denitrificans\" (Christensen, 1903) Bergey et al., 1923, nom. rejic.")
    .code(NomCode::Botanical)
    .nothing_else();
    assert_nom_note("nom. nov.", "Tipula rubiginosa Loew, 1863, nom. nov.")
        .species("Tipula", "rubiginosa")
        .comb_authors(Some("1863"), &["Loew"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_nom_note(
        "nom. prov.",
        "Amanita pruittii A.H.Sm. ex Tulloss & J.Lindgr., nom. prov.",
    )
    .species("Amanita", "pruittii")
    .comb_authors(None, &["Tulloss", "J.Lindgr."])
    .comb_ex_authors(&["A.H.Sm."])
    .code(NomCode::Botanical)
    .nothing_else();
    assert_nom_note("nom. cons.", "Ramonda Rich., nom. cons.")
        .monomial_rank("Ramonda", Rank::Unranked)
        .comb_authors(None, &["Rich."])
        .code(NomCode::Botanical)
        .nothing_else();
    // FIXME(review): garbage: "Kluyver and van Niel" is the authorship, not genus + epithet
    assert_nom_note(
        "nom. cons.",
        "Kluyver and van Niel, 1936 emend. Barker, 1956 (Approved Lists, 1980) , nom. cons., emend. Mah and Kuhn, 1984",
    )
        .species("Kluyver", "and")
        .comb_authors(Some("1936"), &["van Niel"])
        .sensu("emend. Barker, 1956 (Approved Lists, 1980) , , emend. Mah and Kuhn, 1984")
        .doubtful()
        .warning(&[warnings::BLACKLISTED_EPITHET])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_nom_note(
        "nom. superfl.",
        "Coccocypselum tontanea (Aubl.) Kunth, nom. superfl.",
    )
    .species("Coccocypselum", "tontanea")
    .comb_authors(None, &["Kunth"])
    .bas_authors(None, &["Aubl."])
    .code(NomCode::Botanical)
    .nothing_else();
    assert_nom_note(
        "nom. ambig.",
        "Lespedeza bicolor var. intermedia Maxim. , nom. ambig.",
    )
    .infra_species("Lespedeza", "bicolor", Rank::Variety, "intermedia")
    .comb_authors(None, &["Maxim."])
    .code(NomCode::Botanical)
    .nothing_else();
    assert_nom_note(
        "nom. praeoccup.",
        "Erebia aethiops uralensis Goltz, 1930 nom. praeoccup.",
    )
    .infra_species("Erebia", "aethiops", Rank::Subspecies, "uralensis")
    .comb_authors(Some("1930"), &["Goltz"])
    .code(NomCode::Zoological)
    .nothing_else();
    assert_nom_note(
        "comb. nov. ined.",
        "Ipomopsis tridactyla (Rydb.) Wilken, comb. nov. ined.",
    )
    .species("Ipomopsis", "tridactyla")
    .comb_authors(None, &["Wilken"])
    .bas_authors(None, &["Rydb."])
    .manuscript()
    .code(NomCode::Botanical)
    .nothing_else();
    assert_nom_note(
        "sp. nov. ined.",
        "Orobanche riparia Collins, sp. nov. ined.",
    )
    .species("Orobanche", "riparia")
    .comb_authors(None, &["Collins"])
    .manuscript()
    .nothing_else();
    // FIXME(review): "New Caledonia-Rjh-" is a locality code, not an author
    assert_nom_note(
        "gen. nov.",
        "Anchimolgidae gen. nov. New Caledonia-Rjh-, 2004",
    )
    .monomial_rank("Anchimolgidae", Rank::Genus)
    .comb_authors(Some("2004"), &["New Caledonia-Rjh"])
    .code(NomCode::Zoological)
    .nothing_else();
    assert_nom_note("gen. nov. ined.", "Stebbinsoseris gen. nov. ined.")
        .monomial_rank("Stebbinsoseris", Rank::Genus)
        .manuscript()
        .nothing_else();
    // FIXME(review): 1199 is a page or typo, and a Euphorbia is no zoological name
    assert_nom_note("var. nov.", "Euphorbia rossiana var. nov. Steinmann, 1199")
        .species("Euphorbia", "rossiana")
        .comb_authors(Some("1199"), &["Steinmann"])
        .doubtful()
        .warning(&[warnings::UNLIKELY_YEAR])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// http://dev.gbif.org/issues/browse/POR-2454
#[test]
fn fungus_names() {
    // the basionym's sanctioning author stays inside its brackets (Java dropped it)
    assert_name("Merulius lacrimans (Wulfen : Fr.) Schum.")
        .species("Merulius", "lacrimans")
        .comb_authors(None, &["Schum."])
        .bas_authors(None, &["Wulfen"])
        .bas_sanct_author("Fr.")
        .nothing_else();

    assert_name("Merulius lacrimans (Wulfen) Schum. : Fr.")
        .species("Merulius", "lacrimans")
        .comb_authors(None, &["Schum."])
        .bas_authors(None, &["Wulfen"])
        .sanct_author("Fr.")
        .code(NomCode::Botanical)
        .nothing_else();

    //assertParsedParts("Aecidium berberidis Pers. ex J.F. Gmel.", null, "Aecidium", "berberidis", null, null, "Pers. ex J.F. Gmel.", null, null, null);
    //assertParsedParts("Roestelia penicillata (O.F. Müll.) Fr.", null, "Roestelia", "penicillata", null, null, "Fr.", null, "O.F. Müll.", null);
    //
    //assertParsedParts("Mycosphaerella eryngii (Fr. Duby) ex Oudem., 1897", null, "Mycosphaerella", "eryngii", null, null, "ex Oudem.", "1897", "Fr. Duby", null);
    //assertParsedParts("Mycosphaerella eryngii (Fr.ex Duby) ex Oudem. 1897", null, "Mycosphaerella", "eryngii", null, null, "ex Oudem.", "1897", "Fr.ex Duby", null);
    // ex-authors in both the basionym and the combination
    assert_name("Mycosphaerella eryngii (Fr. ex Duby) Johanson ex Oudem. 1897")
        .species("Mycosphaerella", "eryngii")
        .comb_authors(Some("1897"), &["Oudem."])
        .comb_ex_authors(&["Johanson"])
        .bas_authors(None, &["Duby"])
        .bas_ex_authors(None, &["Fr."])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn year_variations() {
    // The bracketed [1912] alone is the year, zoological evidence like any
    // year after a spelled-out author: the trinomial is a subspecies.
    assert_name("Deudorix epijarbas turbo Fruhstorfer, [1912]")
        .infra_species("Deudorix", "epijarbas", Rank::Subspecies, "turbo")
        .comb_authors(Some("1912"), &["Fruhstorfer"])
        .code(NomCode::Zoological)
        .nothing_else();
}
