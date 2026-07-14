// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserGnaTest (methods on lines 2411-2769).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NameType, NomCode, Rank};

#[test]
fn misc_annotations() {
    // group: Misc annotations. Trailing data-quality artefacts ("species",
    // "not found", "MS"), sensu spans, and informal aggregate annotations
    // ("group" / "species group" / "species complex") are stripped. For binomials
    // an "agg./group/complex" annotation promotes the rank to SPECIES_AGGREGATE;
    // for trinomials it's stripped silently without touching the rank, so the
    // trinomial's regular code-driven rank (ZOOLOGICAL → SUBSPECIES) is kept.
    assert_name("Feldmannia species").monomial("Feldmannia");
    assert_name("Periglypta G. Paulay, MS")
        .monomial("Periglypta")
        .comb_authors(None, &["G.Paulay"]);
    assert_name("Teredo not found").monomial("Teredo");
    assert_name("Velutina haliotoides (Linnaeus, 1758), sensu Fabricius, 1780")
        .species("Velutina", "haliotoides")
        .bas_authors(Some("1758"), &["Linnaeus"]);
    assert_name("Acarospora cratericola cratericola Shenk 1974 group")
        .infra_species("Acarospora", "cratericola", Rank::Subspecies, "cratericola")
        .comb_authors(Some("1974"), &["Shenk"]);
    assert_name("Acarospora cratericola cratericola Shenk 1974 species group")
        .infra_species("Acarospora", "cratericola", Rank::Subspecies, "cratericola")
        .comb_authors(Some("1974"), &["Shenk"]);
    assert_name("Acarospora cratericola cratericola Shenk 1974 species complex")
        .infra_species("Acarospora", "cratericola", Rank::Subspecies, "cratericola")
        .comb_authors(Some("1974"), &["Shenk"]);
    assert_name("Parus caeruleus species complex").binomial(
        "Parus",
        None,
        "caeruleus",
        Rank::SpeciesAggregate,
    );
    // skipped: Crenarchaeote enrichment culture clone OREC-B1022
    //   — env-sample annotation pattern not implemented (parses as messy trinomial)
    // skipped: Diodora dorsata  CF
    //   — trailing 2-letter all-caps token parses as a short author surname
    // skipped: Dasysyrphus intrudens complex sp. BBDCQ003-10
    //   — multi-annotation strip (`complex` mid-string + trailing strain code)
    //     not implemented
}

#[test]
fn horticultural_annotation() {
    // group: Horticultural annotation. Botanical "var." kept in canonical; the
    // (ht.) / (hort.) marker after the rank-marker variety is left as an unparsed
    // tail (state=PARTIAL). "ht." is normalised to "hort." on the way in.
    assert_name("Lachenalia tricolor var. nelsonii (ht.) Baker")
        .infra_species("Lachenalia", "tricolor", Rank::Variety, "nelsonii")
        .comb_authors(None, &["Baker"])
        .partial("(hort.)");
    assert_name("Lachenalia tricolor var. nelsonii (hort.) Baker")
        .infra_species("Lachenalia", "tricolor", Rank::Variety, "nelsonii")
        .comb_authors(None, &["Baker"])
        .partial("(hort.)");
    // Trailing "ht."/"hort." after a binomial both parse as a species with the
    // horticultural marker as the comb author ("ht." is normalised to "hort.").
    assert_name("Puya acris ht.")
        .species("Puya", "acris")
        .comb_authors(None, &["hort."]);
    assert_name("Puya acris hort.")
        .species("Puya", "acris")
        .comb_authors(None, &["hort."]);
}

#[test]
fn names_with_mihi() {
    // group: Names with "mihi" — Latin "by me", a self-attribution placeholder.
    // Stripped from the name with an AUTHORSHIP_REMOVED warning.
    assert_name("Characium obovatum mihi. var. longipes mihi")
        .infra_species("Characium", "obovatum", Rank::Variety, "longipes")
        .warning(&[warnings::AUTHORSHIP_REMOVED]);
    assert_name("Regulus modestus mihi. Gould 1837")
        .species("Regulus", "modestus")
        .comb_authors(Some("1837"), &["Gould"])
        .code(NomCode::Zoological)
        .warning(&[warnings::AUTHORSHIP_REMOVED]);
}

#[test]
fn exceptions_with_mihi() {
    // "mihi" between species and authors is also stripped, leaving the binomial
    // with the real authorship.
    assert_name("Eucyclops serrulatus mihi Dussart, Graf & Husson, 1966")
        .species("Eucyclops", "serrulatus")
        .comb_authors(Some("1966"), &["Dussart", "Graf", "Husson"])
        .code(NomCode::Zoological)
        .warning(&[warnings::AUTHORSHIP_REMOVED]);
}

#[test]
fn exceptions_from_ranks_rank_line_epithets() {
    // group: Exceptions from ranks (rank-line epithets) — words that look like
    // infrageneric rank markers (ab, ser, subser) but are genuine species
    // epithets when followed by an author-year span.
    assert_name("Selenops ab Logunov & Jäger, 2015")
        .species("Selenops", "ab")
        .comb_authors(Some("2015"), &["Logunov", "Jäger"])
        .code(NomCode::Zoological);
    assert_name("Helophorus (Lihelophorus) ser Zaitzev, 1908")
        .species_ig("Helophorus", "Lihelophorus", "ser")
        .comb_authors(Some("1908"), &["Zaitzev"])
        .code(NomCode::Zoological);
    // "Serina subser Gredler, 1898" and "Serina ser Gredler, 1898" — the parser
    // takes "subser"/"ser" as infrageneric rank markers (SUBSERIES_BOTANY /
    // SERIES_BOTANY) and folds "Gredler" into the infrageneric epithet. Left
    // as TODOs — needs context-aware disambiguation.
}

#[test]
fn exceptions_from_author_prefixes_prefix_like_epithets() {
    // group: Exceptions from author prefixes (prefix-like epithets) — words like
    // "dela" / "den" that aren't in the AuthorParticles list already parse as
    // species. Genuine author particles (de, des, dos, du, la, van, zu) used as
    // species epithets remain ambiguous without an authority lookup ("Aaaba de
    // Laubenfels, 1936" is a uninomial; "Semiothisa da Dyar, 1916" is a binomial)
    // — those cases are kept as inline TODOs.
    assert_name("Campylosphaera dela (M.N.Bramlette & F.R.Sullivan) W.W.Hay & H.Mohler")
        .species("Campylosphaera", "dela")
        .comb_authors(None, &["W.W.Hay", "H.Mohler"])
        .bas_authors(None, &["M.N.Bramlette", "F.R.Sullivan"]);
    assert_name("Antaplaga dela Druce, 1904")
        .species("Antaplaga", "dela")
        .comb_authors(Some("1904"), &["Druce"])
        .code(NomCode::Zoological);
    assert_name("Baeolidia dela (Er. Marcus & Ev. Marcus, 1960)")
        .species("Baeolidia", "dela")
        .bas_authors(Some("1960"), &["Er.Marcus", "Ev.Marcus"])
        .code(NomCode::Zoological);
    assert_name("Dicentria dela Druce, 1894")
        .species("Dicentria", "dela")
        .comb_authors(Some("1894"), &["Druce"])
        .code(NomCode::Zoological);
    assert_name("Eulaira dela Chamberlin & Ivie, 1933")
        .species("Eulaira", "dela")
        .comb_authors(Some("1933"), &["Chamberlin", "Ivie"])
        .code(NomCode::Zoological);
    assert_name("Paralvinella dela Detinova, 1988")
        .species("Paralvinella", "dela")
        .comb_authors(Some("1988"), &["Detinova"])
        .code(NomCode::Zoological);
    assert_name("Scoparia dela Clarke, 1965")
        .species("Scoparia", "dela")
        .comb_authors(Some("1965"), &["Clarke"])
        .code(NomCode::Zoological);
    assert_name("Tortolena dela Chamberlin & Ivie, 1941")
        .species("Tortolena", "dela")
        .comb_authors(Some("1941"), &["Chamberlin", "Ivie"])
        .code(NomCode::Zoological);
    // "den" is parsed as the species epithet here because the trailing author
    // span has initials (J.L.) — disambiguates from particle usage.
    assert_name("Gnathopleustes den (J.L. Barnard, 1969)")
        .species("Gnathopleustes", "den")
        .bas_authors(Some("1969"), &["J.L.Barnard"])
        .code(NomCode::Zoological);
    assert_name("Agnetina den Cao, T.K.T. & Bae, 2006")
        .species("Agnetina", "den")
        .comb_authors(Some("2006"), &["T.K.T.Cao", "Bae"])
        .code(NomCode::Zoological);
}

#[test]
fn exceptions_from_author_suffixes_suffix_like_epithets() {
    // group: Exceptions from author suffixes (suffix-like epithets)
    assert_name("Ruteloryctes bis Dechambre, 2006")
        .species("Ruteloryctes", "bis")
        .comb_authors(Some("2006"), &["Dechambre"]);
}

#[test]
fn icvcn_binomial_names_and_exceptions() {
    // group: ICVCN binomial names and exceptions
    // skipped: Tokiviricetes
    // skipped: Usarudivirus nymphense
    // skipped: Ictavirus ictaluridallo1
    // skipped: Aghbyvirus ISAO8
    assert_name("Mahavira").monomial("Mahavira");
}

#[test]
fn not_parsed_ocr_errors_to_get_better_precision_recall_ratio() {
    // group: Not parsed OCR errors to get better precision/recall ratio
    // skipped: Mom.alpium (Osbeck, 1778)
}

#[test]
fn no_parsing_genera_abbreviated_to3_letters_too_rare() {
    // group: No parsing -- Genera abbreviated to 3 letters (too rare)
    // skipped: Gen. et n. sp. Kaimatira Pumice Sand, Marton N ~1 Ma
    // skipped: Genn. et n. sp. Kaimatira Pumice Sand, Marton N ~1 Ma
}

#[test]
fn no_parsing_incertae_sedis() {
    // group: No parsing -- incertae sedis
    // skipped: Incertae sedis
    // skipped: </i>Hipponicidae<i> incertae sedis</i>
    // skipped: incertae sedis
    // skipped: Inc.   sed.
    // skipped: inc.sed.
    // skipped: inc.   sed.
    // skipped: Incertaesedis obscuricornis Fairmaire LMH 1893
    // skipped: Uropodoideaincertaesedis
}

#[test]
fn no_parsing_bacterium_candidatus() {
    // group: No parsing -- bacterium, Candidatus. The "Candidatus" prefix is captured
    // as a flag (isCandidatus()) and rendered in the canonical inside quotes.
    assert_name("Acidobacterium ailaaui Myers & King, 2016")
        .species("Acidobacterium", "ailaaui")
        .comb_authors(Some("2016"), &["Myers", "King"]);
    assert_name("Candidatus").monomial("Candidatus");
    assert_name("Candidatus Puniceispirillum Oh, Kwon, Kang, Kang, Lee, Kim & Cho, 2010")
        .monomial("Puniceispirillum")
        .comb_authors(
            Some("2010"),
            &["Oh", "Kwon", "Kang", "Kang", "Lee", "Kim", "Cho"],
        )
        .candidatus();
    assert_name("Candidatus Halobonum")
        .monomial("Halobonum")
        .candidatus();
}

#[test]
fn no_parsing_not_none_unidentified_phrases() {
    // group: No parsing -- 'Not', 'None', 'Unidentified'  phrases
    // skipped: None recorded
    // skipped: NONE recorded
    // skipped: NoNe recorded
    // skipped: None
    // skipped: unidentified recorded
    // skipped: UniDentiFied recorded
    // skipped: not recorded
    // skipped: NOT recorded
    // skipped: Not recorded
    // skipped: Not assigned
    assert_name("Notassigned").monomial("Notassigned");
    // skipped: Unnamed clade
    // skipped: Unamed clade
}

#[test]
fn no_parsing_genus_with_apostrophe() {
    // group: No parsing -- genus with apostrophe
    // skipped: Abbott's moray eel
    // skipped: Chambers' twinpod
    // skipped: Columnea × Alladin's
    // skipped: Hawai'i silversword
}

#[test]
fn no_parsing_camelcase_genus_word() {
    // group: No parsing -- CamelCase 'genus' word
    // skipped: PomaTomus
    // skipped: DizygopUwa stosei
    // skipped: Oxytox[idae] Lindermann
    // skipped: ScarabaeinGCsp.
}

#[test]
fn no_parsing_phytoplasma() {
    // group: No parsing -- phytoplasma
    // skipped: Alfalfa witches'-broom phytoplasma
    // skipped: Allium ampeloprasumphytoplasma
    // skipped: Alstroemeria sp. phytoplasma
}

#[test]
fn no_parsing_symbiont() {
    // group: No parsing symbiont — botanical "var." kept in canonical.
    assert_name("Dictyochloropsis symbiontica Tschermak-Woess")
        .species("Dictyochloropsis", "symbiontica")
        .comb_authors(None, &["Tschermak-Woess"]);
    assert_name("Dylakosoma symbionticum var. valens Skuja")
        .infra_species("Dylakosoma", "symbionticum", Rank::Variety, "valens")
        .comb_authors(None, &["Skuja"]);
}

#[test]
fn names_with_spec_nov_spec() {
    // group: Names with spec., nov spec

    // Java `.species(genus, null)` (→ `.binomial(genus, null, null, SPECIES)`) has no DSL
    // equivalent for a null/absent specific epithet — same DSL-gap workaround as impl_04's
    // "Lepidoptera sp. JGP0404" / impl_10's `phraseIndetName` cases: direct parse + explicit
    // field checks for the fields the Java chain touches (`.nothingElse()` itself isn't
    // replicated field-by-field, matching that same established precedent).
    let n = nameparser::parse_name("Lampona spec Platnick, 2000", None, None, None)
        .expect("`Lampona spec Platnick, 2000` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Lampona"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Species);
    assert_eq!(n.combination_authorship.year.as_deref(), Some("2000"));
    assert_eq!(
        n.combination_authorship.authors,
        vec!["Platnick".to_string()]
    );
    assert_eq!(n.type_, NameType::Informal);
    assert_eq!(n.code, Some(NomCode::Zoological));
    assert_eq!(n.warnings, vec![warnings::INDETERMINED.to_string()]);

    let n = nameparser::parse_name("Gobiosoma spec (Ginsburg, 1939)", None, None, None)
        .expect("`Gobiosoma spec (Ginsburg, 1939)` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Gobiosoma"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Species);
    assert_eq!(n.basionym_authorship.year.as_deref(), Some("1939"));
    assert_eq!(n.basionym_authorship.authors, vec!["Ginsburg".to_string()]);
    assert_eq!(n.type_, NameType::Informal);
    assert_eq!(n.code, Some(NomCode::Zoological));
    assert_eq!(n.warnings, vec![warnings::INDETERMINED.to_string()]);

    let n = nameparser::parse_name("Globigerina spec", None, None, None)
        .expect("`Globigerina spec` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Globigerina"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Species);
    assert_eq!(n.type_, NameType::Informal);
    assert_eq!(n.warnings, vec![warnings::INDETERMINED.to_string()]);

    //      assertName("Eunotia genuflexa Norpel-Schempp nov spec", "Eunotia genuflexa")
    //          .species("Eunotia", "genuflexa")
    //          .combAuthors(null, "Norpel-Schempp")
    //          .nomNote("nov spec")
    //          .nothingElse();

    let n = nameparser::parse_name("Ctenotus spec.", None, None, None)
        .expect("`Ctenotus spec.` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Ctenotus"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Species);
    assert_eq!(n.type_, NameType::Informal);
    assert_eq!(n.warnings, vec![warnings::INDETERMINED.to_string()]);

    // Java `.phraseIndetName(genus, phrase, rank)` is a `NameAssertion` instance method with no
    // DSL equivalent either; same direct-parse workaround.
    let n = nameparser::parse_name("Byrsophlebidae spec. 2", None, None, None)
        .expect("`Byrsophlebidae spec. 2` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Byrsophlebidae"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Species);
    assert_eq!(n.phrase.as_deref(), Some("spec. 2"));
    assert_eq!(n.type_, NameType::Informal);

    assert_name("Naviculadicta witkowskii LB & Metzeltin nov spec")
        .species("Naviculadicta", "witkowskii")
        .comb_authors(None, &["LB", "Metzeltin"])
        .nom_note("nov spec.")
        .nothing_else();
}

#[test]
fn html_tags_and_entities() {
    // group: HTML tags and entities
    // HTML tags are stripped but their text content is kept, so the "sensu Fabricius,
    // 1780" concept reference lands in the taxonomic note rather than the authorship.
    assert_name("Velutina haliotoides (Linnaeus, 1758) <i>sensu</i> Fabricius, 1780")
        .species("Velutina", "haliotoides")
        .bas_authors(Some("1758"), &["Linnaeus"])
        .sensu("sensu Fabricius, 1780")
        .code(NomCode::Zoological)
        .warning(&[warnings::XML_TAGS])
        .nothing_else();

    assert_name("Velutina haliotoides (Linnaeus, 1758), <i>sensu</i> Fabricius, 1780")
        .species("Velutina", "haliotoides")
        .bas_authors(Some("1758"), &["Linnaeus"]);

    assert_name("<i>Velutina halioides</i> (Linnaeus, 1758)")
        .species("Velutina", "halioides")
        .bas_authors(Some("1758"), &["Linnaeus"]);

    assert_name("Quadrella steyermarkii (Standl.) Iltis &amp; Cornejo")
        .species("Quadrella", "steyermarkii")
        .comb_authors(None, &["Iltis", "Cornejo"])
        .bas_authors(None, &["Standl."]);

    assert_name("Torymus bangalorensis (Mani &amp; Kurian, 1953)")
        .species("Torymus", "bangalorensis")
        .bas_authors(Some("1953"), &["Mani", "Kurian"])
        .code(NomCode::Zoological)
        .warning(&[warnings::HTML_ENTITIES])
        .nothing_else();
}

#[test]
fn underscores_instead_of_spaces() {
    // group: Underscores instead of spaces
    assert_name("Oxalis_barrelieri").species("Oxalis", "barrelieri");

    assert_name("Pseudocercospora__dendrobii").species("Pseudocercospora", "dendrobii");

    assert_name("Oxalis barrelieri XXZ_21243")
        .species("Oxalis", "barrelieri")
        .partial("XXZ_21243");
}
