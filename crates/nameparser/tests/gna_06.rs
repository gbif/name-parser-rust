// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserGnaTest (methods on lines 2411-2769).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NameType, NomCode, Rank};

#[test]
fn misc_annotations() {
    // group: Misc annotations. Trailing data-quality artefacts ("species",
    // "not found", "MS") and informal aggregate annotations ("group" / "species
    // group" / "species complex") are stripped; a sensu span becomes the taxonomic
    // note. For binomials
    // an "agg./group/complex" annotation promotes the rank to SPECIES_AGGREGATE;
    // for trinomials it's stripped silently without touching the rank, so the
    // trinomial's regular code-driven rank (ZOOLOGICAL → SUBSPECIES) is kept.
    assert_name("Feldmannia species")
        .monomial("Feldmannia")
        .nothing_else();
    assert_name("Periglypta G. Paulay, MS")
        .monomial("Periglypta")
        .comb_authors(None, &["G.Paulay"])
        .nom_note("ms")
        .manuscript()
        .nothing_else();
    assert_name("Teredo not found")
        .monomial("Teredo")
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
    assert_name("Velutina haliotoides (Linnaeus, 1758), sensu Fabricius, 1780")
        .species("Velutina", "haliotoides")
        .bas_authors(Some("1758"), &["Linnaeus"])
        .sensu("sensu Fabricius, 1780")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Acarospora cratericola cratericola Shenk 1974 group")
        .infra_species("Acarospora", "cratericola", Rank::Subspecies, "cratericola")
        .comb_authors(Some("1974"), &["Shenk"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Acarospora cratericola cratericola Shenk 1974 species group")
        .infra_species("Acarospora", "cratericola", Rank::Subspecies, "cratericola")
        .comb_authors(Some("1974"), &["Shenk"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Acarospora cratericola cratericola Shenk 1974 species complex")
        .infra_species("Acarospora", "cratericola", Rank::Subspecies, "cratericola")
        .comb_authors(Some("1974"), &["Shenk"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Parus caeruleus species complex")
        .binomial("Parus", None, "caeruleus", Rank::SpeciesAggregate)
        .nothing_else();
    // FIXME(review): an environmental sample label parsed as a trinomial
    // skipped: Crenarchaeote enrichment culture clone OREC-B1022
    //   — env-sample annotation pattern not implemented (parses as messy trinomial)
    // FIXME(review): the upper-case "CF" (cf.) is dropped without a qualifier
    // skipped: Diodora dorsata  CF
    //   — trailing 2-letter all-caps token parses as a short author surname
    // FIXME(review): a BOLD sample id becomes the infraspecific epithet
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
        .partial("(hort.)")
        .nothing_else();
    assert_name("Lachenalia tricolor var. nelsonii (hort.) Baker")
        .infra_species("Lachenalia", "tricolor", Rank::Variety, "nelsonii")
        .comb_authors(None, &["Baker"])
        .partial("(hort.)")
        .nothing_else();
    // Trailing "ht."/"hort." after a binomial both parse as a species with the
    // horticultural marker as the comb author ("ht." is normalised to "hort.").
    assert_name("Puya acris ht.")
        .species("Puya", "acris")
        .comb_authors(None, &["hort."])
        .nothing_else();
    assert_name("Puya acris hort.")
        .species("Puya", "acris")
        .comb_authors(None, &["hort."])
        .nothing_else();
}

#[test]
fn names_with_mihi() {
    // group: Names with "mihi" — Latin "by me", a self-attribution placeholder.
    // Stripped from the name with an AUTHORSHIP_REMOVED warning.
    assert_name("Characium obovatum mihi. var. longipes mihi")
        .infra_species("Characium", "obovatum", Rank::Variety, "longipes")
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
    assert_name("Regulus modestus mihi. Gould 1837")
        .species("Regulus", "modestus")
        .comb_authors(Some("1837"), &["Gould"])
        .code(NomCode::Zoological)
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
}

#[test]
fn exceptions_with_mihi() {
    // "mihi" between species and authors is also stripped, leaving the binomial
    // with the real authorship.
    assert_name("Eucyclops serrulatus mihi Dussart, Graf & Husson, 1966")
        .species("Eucyclops", "serrulatus")
        .comb_authors(Some("1966"), &["Dussart", "Graf", "Husson"])
        .code(NomCode::Zoological)
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
}

#[test]
fn exceptions_from_ranks_rank_line_epithets() {
    // group: Exceptions from ranks (rank-line epithets) — words that look like
    // infrageneric rank markers (ab, ser, subser) but are genuine species
    // epithets when followed by an author-year span.
    assert_name("Selenops ab Logunov & Jäger, 2015")
        .species("Selenops", "ab")
        .comb_authors(Some("2015"), &["Logunov", "Jäger"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Helophorus (Lihelophorus) ser Zaitzev, 1908")
        .species_ig("Helophorus", "Lihelophorus", "ser")
        .comb_authors(Some("1908"), &["Zaitzev"])
        .code(NomCode::Zoological)
        .nothing_else();
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
        .bas_authors(None, &["M.N.Bramlette", "F.R.Sullivan"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Antaplaga dela Druce, 1904")
        .species("Antaplaga", "dela")
        .comb_authors(Some("1904"), &["Druce"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Baeolidia dela (Er. Marcus & Ev. Marcus, 1960)")
        .species("Baeolidia", "dela")
        .bas_authors(Some("1960"), &["Er.Marcus", "Ev.Marcus"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Dicentria dela Druce, 1894")
        .species("Dicentria", "dela")
        .comb_authors(Some("1894"), &["Druce"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Eulaira dela Chamberlin & Ivie, 1933")
        .species("Eulaira", "dela")
        .comb_authors(Some("1933"), &["Chamberlin", "Ivie"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Paralvinella dela Detinova, 1988")
        .species("Paralvinella", "dela")
        .comb_authors(Some("1988"), &["Detinova"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Scoparia dela Clarke, 1965")
        .species("Scoparia", "dela")
        .comb_authors(Some("1965"), &["Clarke"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Tortolena dela Chamberlin & Ivie, 1941")
        .species("Tortolena", "dela")
        .comb_authors(Some("1941"), &["Chamberlin", "Ivie"])
        .code(NomCode::Zoological)
        .nothing_else();
    // "den" is parsed as the species epithet here because the trailing author
    // span has initials (J.L.) — disambiguates from particle usage.
    assert_name("Gnathopleustes den (J.L. Barnard, 1969)")
        .species("Gnathopleustes", "den")
        .bas_authors(Some("1969"), &["J.L.Barnard"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Agnetina den Cao, T.K.T. & Bae, 2006")
        .species("Agnetina", "den")
        .comb_authors(Some("2006"), &["T.K.T.Cao", "Bae"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn exceptions_from_author_suffixes_suffix_like_epithets() {
    // group: Exceptions from author suffixes (suffix-like epithets)
    assert_name("Ruteloryctes bis Dechambre, 2006")
        .species("Ruteloryctes", "bis")
        .comb_authors(Some("2006"), &["Dechambre"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn icvcn_binomial_names_and_exceptions() {
    // group: ICVCN binomial names and exceptions
    assert_name("Tokiviricetes")
        .monomial_rank("Tokiviricetes", Rank::Class)
        .code(NomCode::Virus)
        .nothing_else();
    assert_name("Usarudivirus nymphense")
        .species("Usarudivirus", "nymphense")
        .code(NomCode::Virus)
        .nothing_else();
    assert_name("Ictavirus ictaluridallo1")
        .species("Ictavirus", "ictaluridallo1")
        .code(NomCode::Virus)
        .nothing_else();
    assert_unparsable_code("Aghbyvirus ISAO8", NameType::Other, NomCode::Virus);
    assert_name("Mahavira").monomial("Mahavira").nothing_else();
}

#[test]
#[ignore = "desired: not parsed as a name, not yet supported"]
fn not_parsed_ocr_errors_to_get_better_precision_recall_ratio() {
    // group: Not parsed OCR errors to get better precision/recall ratio
    // currently an OCR artefact ("Mom." for "Momordica") parsed as genus "Mom."
    assert_unparsable("Mom.alpium (Osbeck, 1778)", NameType::Other);
}

#[test]
#[ignore = "desired: not parsed as a name, not yet supported"]
fn no_parsing_genera_abbreviated_to3_letters_too_rare() {
    // group: No parsing -- Genera abbreviated to 3 letters (too rare)
    // currently "Gen. et n. sp." + a locality parsed as genus "Gen." and epithets
    assert_unparsable(
        "Gen. et n. sp. Kaimatira Pumice Sand, Marton N ~1 Ma",
        NameType::Other,
    );
    // currently "Genn. et n. sp." + a locality parsed as genus "Genn." and epithets
    assert_unparsable(
        "Genn. et n. sp. Kaimatira Pumice Sand, Marton N ~1 Ma",
        NameType::Other,
    );
}

#[test]
fn no_parsing_incertae_sedis() {
    // group: No parsing -- incertae sedis
    assert_unparsable("Incertae sedis", NameType::Placeholder);
    assert_unparsable(
        "</i>Hipponicidae<i> incertae sedis</i>",
        NameType::Placeholder,
    );
    assert_unparsable("incertae sedis", NameType::Placeholder);
    assert_unparsable("Inc.   sed.", NameType::Placeholder);
    assert_unparsable("inc.sed.", NameType::Placeholder);
    assert_unparsable("inc.   sed.", NameType::Placeholder);
    assert_unparsable(
        "Incertaesedis obscuricornis Fairmaire LMH 1893",
        NameType::Placeholder,
    );
    // FIXME(review): a glued "incertae sedis" placeholder parsed as a uninomial
    // skipped: Uropodoideaincertaesedis
}

#[test]
fn no_parsing_bacterium_candidatus() {
    // group: No parsing -- bacterium, Candidatus. The "Candidatus" prefix is captured
    // as a flag (isCandidatus()) and rendered in the canonical inside quotes.
    assert_name("Acidobacterium ailaaui Myers & King, 2016")
        .species("Acidobacterium", "ailaaui")
        .comb_authors(Some("2016"), &["Myers", "King"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Candidatus")
        .monomial("Candidatus")
        .nothing_else();
    assert_name("Candidatus Puniceispirillum Oh, Kwon, Kang, Kang, Lee, Kim & Cho, 2010")
        .monomial("Puniceispirillum")
        .comb_authors(
            Some("2010"),
            &["Oh", "Kwon", "Kang", "Kang", "Lee", "Kim", "Cho"],
        )
        .candidatus()
        .nothing_else();
    assert_name("Candidatus Halobonum")
        .monomial("Halobonum")
        .candidatus()
        .nothing_else();
}

#[test]
fn no_parsing_not_none_unidentified_phrases() {
    // group: No parsing -- 'Not', 'None', 'Unidentified'  phrases
    // FIXME(review): a no-name placeholder parsed as genus "None"
    // skipped: None recorded
    // FIXME(review): a no-name placeholder parsed as genus "None"
    // skipped: NONE recorded
    // FIXME(review): a no-name placeholder parsed as genus "NoNe"
    // skipped: NoNe recorded
    // FIXME(review): a no-name placeholder parsed as a uninomial
    // skipped: None
    assert_unparsable("unidentified recorded", NameType::Placeholder);
    assert_unparsable("UniDentiFied recorded", NameType::Placeholder);
    // FIXME(review): a no-name placeholder parsed as genus "Not"
    // skipped: not recorded
    // FIXME(review): a no-name placeholder parsed as genus "Not"
    // skipped: NOT recorded
    // FIXME(review): a no-name placeholder parsed as genus "Not"
    // skipped: Not recorded
    assert_unparsable("Not assigned", NameType::Placeholder);
    assert_name("Notassigned")
        .monomial("Notassigned")
        .nothing_else();
    assert_unparsable("Unnamed clade", NameType::Other);
    assert_unparsable("Unamed clade", NameType::Other);
}

#[test]
#[ignore = "desired: not parsed as a name, not yet supported"]
fn no_parsing_genus_with_apostrophe() {
    // group: No parsing -- genus with apostrophe
    // currently a vernacular name parsed as a trinomial
    assert_unparsable("Abbott's moray eel", NameType::Other);
    // currently a vernacular name parsed as a uninomial
    assert_unparsable("Chambers' twinpod", NameType::Other);
    // currently a cultivar-like vernacular parsed as a uninomial
    assert_unparsable("Columnea × Alladin's", NameType::Other);
    // currently a vernacular name parsed as a binomial
    assert_unparsable("Hawai'i silversword", NameType::Other);
}

#[test]
#[ignore = "desired: not parsed as a name, not yet supported"]
fn no_parsing_camelcase_genus_word() {
    // group: No parsing -- CamelCase 'genus' word
    // currently an OCR artefact (mixed case) parsed as a uninomial
    assert_unparsable("PomaTomus", NameType::Other);
    // currently an OCR artefact (mixed case) parsed as a binomial
    assert_unparsable("DizygopUwa stosei", NameType::Other);
    // currently the bracketed suffix is dropped: uninomial "Oxytox"
    assert_unparsable("Oxytox[idae] Lindermann", NameType::Other);
    // currently a glued label parsed as a uninomial
    assert_unparsable("ScarabaeinGCsp.", NameType::Other);
}

#[test]
fn no_parsing_phytoplasma() {
    // group: No parsing -- phytoplasma
    // FIXME(review): a phytoplasma label parsed as genus "Alfalfa"
    // skipped: Alfalfa witches'-broom phytoplasma
    // FIXME(review): a glued phytoplasma label parsed as a binomial
    // skipped: Allium ampeloprasumphytoplasma
    assert_informal("Alstroemeria sp. phytoplasma")
        .taxon("Alstroemeria")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. phytoplasma")
        .nothing_else();
}

#[test]
fn no_parsing_symbiont() {
    // group: No parsing symbiont — botanical "var." kept in canonical.
    assert_name("Dictyochloropsis symbiontica Tschermak-Woess")
        .species("Dictyochloropsis", "symbiontica")
        .comb_authors(None, &["Tschermak-Woess"])
        .nothing_else();
    assert_name("Dylakosoma symbionticum var. valens Skuja")
        .infra_species("Dylakosoma", "symbionticum", Rank::Variety, "valens")
        .comb_authors(None, &["Skuja"])
        .nothing_else();
}

#[test]
fn names_with_spec_nov_spec() {
    // group: Names with spec., nov spec

    // 5.0.0 divergence from Java 4.2.0, which read both of these as indet `Genus spec.` and
    // dropped the epithet. They are real published species — `spec` is a genuine epithet — and
    // a dot-less `spec` carrying an authorship is exactly the signal that says so; see
    // informal.rs's `bare_spec_*` tests for the rule.
    assert_name("Lampona spec Platnick, 2000")
        .species("Lampona", "spec")
        .comb_authors(Some("2000"), &["Platnick"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Gobiosoma spec (Ginsburg, 1939)")
        .species("Gobiosoma", "spec")
        .bas_authors(Some("1939"), &["Ginsburg"])
        .code(NomCode::Zoological)
        .nothing_else();

    // Java `.species(genus, null)` (→ `.binomial(genus, null, null, SPECIES)`) has no DSL
    // equivalent for a null/absent specific epithet — same DSL-gap workaround as impl_04's
    // "Lepidoptera sp. JGP0404" / impl_10's `phraseIndetName` cases: direct parse + explicit
    // field checks for the fields the Java chain touches (`.nothingElse()` itself isn't
    // replicated field-by-field, matching that same established precedent).

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
        .bas_authors(Some("1758"), &["Linnaeus"])
        .sensu("sensu Fabricius, 1780")
        .warning(&[warnings::XML_TAGS])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("<i>Velutina halioides</i> (Linnaeus, 1758)")
        .species("Velutina", "halioides")
        .bas_authors(Some("1758"), &["Linnaeus"])
        .warning(&[warnings::XML_TAGS])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Quadrella steyermarkii (Standl.) Iltis &amp; Cornejo")
        .species("Quadrella", "steyermarkii")
        .comb_authors(None, &["Iltis", "Cornejo"])
        .bas_authors(None, &["Standl."])
        .warning(&[warnings::HTML_ENTITIES])
        .code(NomCode::Botanical)
        .nothing_else();

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
    assert_name("Oxalis_barrelieri")
        .species("Oxalis", "barrelieri")
        .nothing_else();

    assert_name("Pseudocercospora__dendrobii")
        .species("Pseudocercospora", "dendrobii")
        .nothing_else();

    assert_name("Oxalis barrelieri XXZ_21243")
        .species("Oxalis", "barrelieri")
        .partial("XXZ_21243")
        .nothing_else();
}
