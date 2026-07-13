// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserImplTest (methods on lines 3103-3493).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NameType, NomCode, Rank};

#[test]
fn flag_bad_authorship() {
    assert_name("Cynoglossus aurolineatus Not applicable")
        .species("Cynoglossus", "aurolineatus")
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();

    assert_name("Asellus major Not given")
        .species("Asellus", "major")
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();

    assert_name("Doradidae <Unspecified Agent>")
        .monomial("Doradidae")
        .warning(&[warnings::AUTHORSHIP_REMOVED, warnings::UNUSUAL_CHARACTERS])
        .doubtful()
        .nothing_else();
}

/// The longest known real, valid name (~860 chars): Homo naledi with its 47-author
/// describing team plus an equally long `sec.` concept reference. It must parse
/// cleanly — not get rejected by the 1000-char DoS cap — and, because it exceeds 250
/// chars, carry the LONG_NAME warning. A genuinely over-long input is still rejected.
#[test]
fn long_name() {
    let authors = "Berger, Hawks, de Ruiter, Churchill, Schmid, Delezene, Kivell, Garvin, Williams, DeSilva, Skinner, Musiba, Cameron, Holliday, Harcourt-Smith, Ackermann, Bastir, Bogin, Bolter, Brophy, Cofran, Congdon, Deane, Dembo, Drapeau, Elliott, Feuerriegel, Garcia-Martinez, Green, Gurtov, Irish, Kruger, Laird, Marchi, Meyer, Nalla, Negash, Orr, Radovcic, Schroeder, Scott, Throckmorton, Tocheri, VanSickle, Walker, Wei & Zipfel";
    let homo_naledi = format!("Homo naledi {authors}, 2015 sec. {authors}");
    assert!(homo_naledi.len() > 250 && homo_naledi.len() < 1000);

    assert_name(&homo_naledi)
        .species("Homo", "naledi")
        .comb_authors(
            Some("2015"),
            &[
                "Berger",
                "Hawks",
                "de Ruiter",
                "Churchill",
                "Schmid",
                "Delezene",
                "Kivell",
                "Garvin",
                "Williams",
                "DeSilva",
                "Skinner",
                "Musiba",
                "Cameron",
                "Holliday",
                "Harcourt-Smith",
                "Ackermann",
                "Bastir",
                "Bogin",
                "Bolter",
                "Brophy",
                "Cofran",
                "Congdon",
                "Deane",
                "Dembo",
                "Drapeau",
                "Elliott",
                "Feuerriegel",
                "Garcia-Martinez",
                "Green",
                "Gurtov",
                "Irish",
                "Kruger",
                "Laird",
                "Marchi",
                "Meyer",
                "Nalla",
                "Negash",
                "Orr",
                "Radovcic",
                "Schroeder",
                "Scott",
                "Throckmorton",
                "Tocheri",
                "VanSickle",
                "Walker",
                "Wei",
                "Zipfel",
            ],
        )
        .sensu(&format!("sec. {authors}"))
        .warning(&[warnings::LONG_NAME])
        .code(NomCode::Zoological)
        .nothing_else();

    // A long-but-valid authorship (~420 chars) supplied on its own via parseAuthorship
    // still parses fine — the cap only rejects beyond 1000 chars.
    // Direct-parse fallback for Java's `parser.parseAuthorship(auth, code)`, which the DSL's
    // own `assert_ex_authorship` documents as `parse("Abies alba", auth, SPECIES, code)`.
    let pa = nameparser::parse(
        "Abies alba",
        Some(&format!("{authors}, 2015")),
        Some(Rank::Species),
        None,
    )
    .unwrap_or_else(|e| panic!("expected long authorship to parse: {e:?}"));
    assert_eq!(pa.combination_authorship.year.as_deref(), Some("2015"));
    assert_eq!(pa.combination_authorship.authors.len(), 47);
    assert_eq!(pa.combination_authorship.authors[0], "Berger");
    assert_eq!(pa.combination_authorship.authors[46], "Zipfel");

    // Beyond the 1000-char cap the input is rejected rather than parsed (DoS guard).
    let mut too_long = String::from("Homo naledi ");
    while too_long.len() <= 1000 {
        too_long.push_str("Berger, ");
    }
    too_long.push_str("2015");
    assert_unparsable(&too_long, NameType::Other);

    // The same cap guards the separately supplied authorship argument.
    let mut long_authorship = String::new();
    while long_authorship.len() <= 1000 {
        long_authorship.push_str("Berger, ");
    }
    long_authorship.push_str("2015");
    // Direct-parse fallback for Java's `parser.parseAuthorship(...)` try/catch expecting an
    // UnparsableNameException with type OTHER.
    match nameparser::parse(
        "Abies alba",
        Some(&long_authorship),
        Some(Rank::Species),
        None,
    ) {
        Err(e) => assert_eq!(e.type_, NameType::Other),
        Ok(pn) => panic!("expected over-long authorship to be rejected, got: {pn:?}"),
    }
}

#[test]
fn taxonomic_notes() {
    // bacteria
    assert_name("Achromobacter Yabuuchi and Yano, 1981 emend. Yabuuchi et al., 1998")
        .monomial("Achromobacter")
        .comb_authors(Some("1981"), &["Yabuuchi", "Yano"])
        .sensu("emend. Yabuuchi et al., 1998")
        .code(NomCode::Zoological)
        .nothing_else();

    // FishBase https://github.com/CatalogueOfLife/backend/issues/1067
    assert_name("Centropyge fisheri (non Snyder, 1904)")
        .species("Centropyge", "fisheri")
        .sensu("non Snyder, 1904")
        .nothing_else();

    assert_name("Centropyge fisheri non (Snyder, 1904)")
        .species("Centropyge", "fisheri")
        .sensu("non (Snyder, 1904)")
        .nothing_else();

    assert_name("Centropyge fisheri (not Snyder, 1904)")
        .species("Centropyge", "fisheri")
        .sensu("not Snyder, 1904")
        .nothing_else();

    // https://github.com/CatalogueOfLife/data/issues/146#issuecomment-649095386
    assert_name("Vittaria auct.")
        .monomial("Vittaria")
        .sensu("auct.")
        .nothing_else();

    // from Dyntaxa
    assert_name("Pycnophyes Auctt., non Zelinka, 1907")
        .monomial("Pycnophyes")
        .sensu("auctt., non Zelinka, 1907")
        .nothing_else();

    assert_name("Dyadobacter (Chelius & Triplett, 2000) emend. Reddy & Garcia-Pichel, 2005")
        .monomial("Dyadobacter")
        .bas_authors(Some("2000"), &["Chelius", "Triplett"])
        .sensu("emend. Reddy & Garcia-Pichel, 2005")
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Thalassiosira praeconvexa Burckle emend Gersonde & Schrader, 1984")
        .species("Thalassiosira", "praeconvexa")
        .comb_authors(None, &["Burckle"])
        .sensu("emend Gersonde & Schrader, 1984")
        .nothing_else();

    assert_name("Amphora gracilis f. exilis Gutwinski according to Hollerback & Krasavina, 1971")
        .infra_species("Amphora", "gracilis", Rank::Form, "exilis")
        .comb_authors(None, &["Gutwinski"])
        .sensu("according to Hollerback & Krasavina, 1971")
        .nothing_else();

    assert_sensu(
        "Vespa emarginata Linnaeus, 1758: Fabricius, 1793",
        "Fabricius, 1793",
    );
    assert_sensu("Trifolium repens sensu Baker f.", "sensu Baker f.");
    assert_sensu("Achillea millefolium sensu latu", "sensu latu");
    assert_sensu("Achillea millefolium s.str.", "s.str.");
    assert_sensu(
        "Achillea millefolium sec. Greuter 2009",
        "sec. Greuter 2009",
    );
    assert_sensu(
        "Globularia cordifolia L. excl. var. (emend. Lam.)",
        "excl. var. (emend. Lam.)",
    );

    assert_name("Ramaria subbotrytis (Coker) Corner 1950 ss. auct. europ.")
        .species("Ramaria", "subbotrytis")
        .bas_authors(None, &["Coker"])
        .comb_authors(Some("1950"), &["Corner"])
        .sensu("ss. auct. europ.")
        .nothing_else();

    assert_name("Thelephora cuticularis Berk. ss. auct. europ.")
        .species("Thelephora", "cuticularis")
        .comb_authors(None, &["Berk."])
        .sensu("ss. auct. europ.")
        .nothing_else();

    assert_name("Handmannia austriaca f. elliptica Handmann fide Hustedt, 1922")
        .infra_species("Handmannia", "austriaca", Rank::Form, "elliptica")
        .comb_authors(None, &["Handmann"])
        .sensu("fide Hustedt, 1922")
        .nothing_else();

    // authorship-level sensu cases (sensu.txt)
    assert_authorship("Miller sensu Busch, 1930", &["Miller"]).sensu("sensu Busch, 1930");

    // "(Author, year) sensu …": basionym authorship, sensu trails as the taxonomic note
    assert_authorship("(Mereschkowsky, 1878) sensu Jankowski, 1992", &[])
        .bas_authors(Some("1878"), &["Mereschkowsky"])
        .sensu("sensu Jankowski, 1992");

    assert_name("Latrodectus marikitates sensu Whittaker")
        .species("Latrodectus", "marikitates")
        .sensu("sensu Whittaker")
        .nothing_else();

    // pure taxonomic note supplied as the authorship, no author preceding it (sensu.txt)
    assert_authorship("sensu Turcz., p.p.", &[])
        .comb_authors(None, &[])
        .sensu("sensu Turcz., p.p.");
}

#[test]
fn non_names() {
    // the entire name ends up as a taxonomic note, consider this as unparsed...
    assert_unparsable_rank(
        "non  Ramaria fagetorum Maas Geesteranus 1976 nomen nudum = Ramaria subbotrytis sensu auct. europ.",
        Rank::Species,
        NameType::Other,
    );

    assert_name("Hebeloma album Peck 1900 non ss. auct. europ.")
        .species("Hebeloma", "album")
        .comb_authors(Some("1900"), &["Peck"])
        .sensu("non ss. auct. europ.")
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Nitocris (Nitocris) similis Breuning, 1956 (nec Gahan, 1893)")
        .binomial("Nitocris", Some("Nitocris"), "similis", Rank::Species)
        .comb_authors(Some("1956"), &["Breuning"])
        .sensu("nec Gahan, 1893")
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Bartlingia Brongn. non Rchb. 1824 nec F.Muell. 1882")
        .monomial("Bartlingia")
        .comb_authors(None, &["Brongn."])
        .sensu("non Rchb. 1824 nec F.Muell. 1882")
        .nothing_else();

    assert_name("Lindera Thunb. non Adans. 1763")
        .monomial("Lindera")
        .comb_authors(None, &["Thunb."])
        .sensu("non Adans. 1763")
        .nothing_else();

    assert_name("Chorististium maculatum (non Bloch 1790)")
        .species("Chorististium", "maculatum")
        .sensu("non Bloch 1790")
        .nothing_else();

    assert_name("Puntius arulius subsp. tambraparniei (non Silas 1954)")
        .infra_species("Puntius", "arulius", Rank::Subspecies, "tambraparniei")
        .sensu("non Silas 1954")
        .nothing_else();
}

#[test]
fn misapplied() {
    assert_name("Ficus exasperata auct. non Vahl")
        .species("Ficus", "exasperata")
        .sensu("auct. non Vahl")
        .nothing_else();

    assert_name("Mentha rotundifolia auct. non (L.) Huds. 1762")
        .species("Mentha", "rotundifolia")
        .sensu("auct. non (L.) Huds. 1762")
        .nothing_else();

    assert_name("Mentha rotundifolia auct. nec Zeller, 1877")
        .species("Mentha", "rotundifolia")
        .sensu("auct. nec Zeller, 1877")
        .nothing_else();

    assert_authorship("auct. nec Zeller, 1877", &[]).sensu("auct. nec Zeller, 1877");

    assert_name("Latrodectus marikitates auct. nec Whittaker")
        .species("Latrodectus", "marikitates")
        .sensu("auct. nec Whittaker")
        .nothing_else();
}

/// Unicode apostrophe / quote variants are normalised to ASCII (' and ") in the parsed output,
/// on both the scientific name and the separately supplied authorship.
#[test]
fn quote_normalisation() {
    assert_name("Abies alba O’Brien")
        .species("Abies", "alba")
        .comb_authors(None, &["O'Brien"])
        .nothing_else();

    assert_authorship("O’Brien", &["O'Brien"]); // U+2019 right single quotation mark
    assert_authorship("OʼBrien", &["O'Brien"]); // U+02BC modifier letter apostrophe
    assert_authorship("L´Hér.", &["L'Hér."]); // U+00B4 acute accent used as apostrophe

    // In zoological nomenclature, names written like:
    //
    //'Prosthète' Hesse, 1861
    //
    //often indicate that the word is not available as a scientific name.
    // The quotation marks signal that it was published but is not recognized as a valid nomenclatural act.
    //
    // 'Prosthète' Hesse, 1861 is not a valid scientific genus name.
    //
    // It was a French vernacular (common-language) name introduced by the French zoologist Eugène Hesse in 1861 for an isopod crustacean.
    // The name was later ruled to be a vernacular term rather than an available zoological name under the ICZN.
    // The Official Index of Zoological Names explicitly lists 'Prosthète' Hesse, 1861 as "a vernacular name."
    assert_name("'Prosthète' Hesse 1861")
        .monomial("'Prosthète'")
        .doubtful() // because the quotes indicate it is not a valid scientific name
        .comb_authors(Some("1861"), &["Hesse"])
        .code(NomCode::Zoological)
        .nothing_else();

    // the curly-quoted input parses identically to the ASCII-quoted one (‘Prosthète’ Hesse 1861)
    // Direct-parse fallback: Java compares two `parser.parse(...)` results for structural
    // equality; `ParsedName` derives `PartialEq` so `assert_eq!` reproduces it directly.
    assert_eq!(
        nameparser::parse("'Prosthète' Hesse 1861", None, None, None).unwrap(),
        nameparser::parse("‘Prosthète’ Hesse 1861", None, None, None).unwrap()
    );
    assert_eq!(
        nameparser::parse("\"Prosthète\" Hesse 1861", None, None, None).unwrap(),
        nameparser::parse("“Prosthète” Hesse 1861", None, None, None).unwrap()
    );
}

#[test]
fn stray_char_in_epithet() {
    // a stray "!" inside an epithet (OCR/typo artefact for "pulchra") is kept as part of the
    // epithet, not split off into the authorship
    assert_name("Lamprostiba pu!chra Pace, 2014")
        .species("Lamprostiba", "pu!chra")
        .comb_authors(Some("2014"), &["Pace"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn viral_names() {
    assert!(is_viral_name("Cactus virus 2"));
    assert!(is_viral_name("Vibrio phage 149 (type IV)"));
    assert!(is_viral_name("Cactus virus 2"));
    assert!(is_viral_name("Suid herpesvirus 3 Ictv"));
    assert!(is_viral_name("Tomato yellow leaf curl Mali virus Ictv"));
    assert!(is_viral_name("Not Sapovirus MC10"));
    assert!(is_viral_name("Diolcogaster facetosa bracovirus"));
    assert!(is_viral_name("Human papillomavirus"));
    assert!(is_viral_name("Sapovirus Hu/GI/Nsc, 150/PA/Bra/, 1993"));
    assert!(is_viral_name("Aspergillus mycovirus, 1816"));
    assert!(is_viral_name("Hantavirus sdp2 Yxl-, 2008"));
    assert!(is_viral_name(
        "Norovirus Nizhny Novgorod /, 2461 / Rus /, 2007"
    ));
    assert!(is_viral_name("Carrot carlavirus WM-, 2008"));
    assert!(is_viral_name("C2-like viruses"));
    assert!(is_viral_name("C1 bacteriophage"));
    assert!(is_viral_name("C-terminal Gfp fusion vector pUG23"));
    assert!(is_viral_name("C-terminal Gfp fusion vector"));
    assert!(is_viral_name("CMVd3 Flexi Vector pFN24K (HaloTag 7)"));
    assert!(is_viral_name("bacteriophage, 315.6"));
    assert!(is_viral_name("bacteriophages"));
    assert!(is_viral_name("\"T1-like viruses\""));
    // http://dev.gbif.org/issues/browse/PF-2574
    assert!(is_viral_name("Inachis io NPV"));
    assert!(is_viral_name("Hyloicus pinastri NPV"));
    assert!(is_viral_name("Dictyoploca japonica NPV"));
    assert!(is_viral_name("Apocheima pilosaria NPV"));
    assert!(is_viral_name("Lymantria xylina NPV"));
    assert!(is_viral_name("Feltia subterranea GV"));
    assert!(is_viral_name("Dionychopus amasis GV"));

    assert!(!is_viral_name("Forcipomyia flavirustica Remm, 1968"));

    assert_name("Crassatellites janus Hedley, 1906")
        .species("Crassatellites", "janus")
        .comb_authors(Some("1906"), &["Hedley"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Ypsolophus satellitella")
        .species("Ypsolophus", "satellitella")
        .nothing_else();

    assert_name("Nephodia satellites")
        .species("Nephodia", "satellites")
        .nothing_else();

    // ICTV binomials that now parse as proper scientific names with code=VIRUS
    assert_name("Lausannevirus")
        .monomial("Lausannevirus")
        .code(NomCode::Virus)
        .nothing_else();

    assert_name("Clecrusatellite")
        .monomial("Clecrusatellite")
        .code(NomCode::Virus)
        .nothing_else();

    assert_name("Marseillevirus marseillevirus")
        .species("Marseillevirus", "marseillevirus")
        .code(NomCode::Virus)
        .nothing_else();

    // SKIPPED: the trailing viruses.txt loop (`resourceReader("viruses.txt")`, asserting
    // `isViralName(line)` for every non-comment/non-blank line) — reads a resource corpus
    // file, covered by the golden/cross-val harness.
}

#[test]
fn virus_binomials_parse() {
    assert_name("Tobamovirus tabaci")
        .species("Tobamovirus", "tabaci")
        .code(NomCode::Virus)
        .nothing_else();

    assert_name("Orthoebolavirus zairense")
        .species("Orthoebolavirus", "zairense")
        .code(NomCode::Virus)
        .nothing_else();

    assert_name("Lausannevirus")
        .monomial("Lausannevirus")
        .code(NomCode::Virus)
        .nothing_else();

    assert_name("Coronaviridae")
        .monomial_rank("Coronaviridae", Rank::Family)
        .code(NomCode::Virus)
        .nothing_else();

    // legacy vernacular → unparsable OTHER + code VIRUS
    assert_unparsable_code("Tobacco mosaic virus", NameType::Other, NomCode::Virus);
    assert_unparsable_code("Human papillomavirus", NameType::Other, NomCode::Virus);
    assert_unparsable_code("Acara virus", NameType::Other, NomCode::Virus);
}
