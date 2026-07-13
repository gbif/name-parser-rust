// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserImplTest (methods on lines 4439-4854).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NameType, NomCode, Rank};

#[test]
fn imprint_years() {
    assert_name("Ophidocampa tapacumae Ehrenberg, 1870, 1869")
        .species("Ophidocampa", "tapacumae")
        .comb_authors(Some("1870"), &["Ehrenberg"])
        .imprint_year("1869")
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name(
        "Brachyspira Hovind-Hougen, Birch-Andersen, Henrik-Nielsen, Orholm, Pedersen, Teglbjaerg & Thaysen, 1983, 1982",
    )
    .monomial("Brachyspira")
    .comb_authors(
        Some("1983"),
        &[
            "Hovind-Hougen",
            "Birch-Andersen",
            "Henrik-Nielsen",
            "Orholm",
            "Pedersen",
            "Teglbjaerg",
            "Thaysen",
        ],
    )
    .imprint_year("1982")
    .code(NomCode::Zoological)
    .nothing_else();

    assert_name("Gyrosigma angulatum var. gamma Griffith & Henfrey, 1860, 1856")
        .infra_species("Gyrosigma", "angulatum", Rank::Variety, "gamma")
        .comb_authors(Some("1860"), &["Griffith", "Henfrey"])
        .imprint_year("1856")
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Ctenotus alacer Storr, 1970 [\"1969\"]")
        .species("Ctenotus", "alacer")
        .comb_authors(Some("1970"), &["Storr"])
        .imprint_year("1969")
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Ctenotus alacer Storr, 1970 (imprint 1969)")
        .species("Ctenotus", "alacer")
        .comb_authors(Some("1970"), &["Storr"])
        .imprint_year("1969")
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Ctenotus alacer Storr, 1887 (\"1886-1888\")")
        .species("Ctenotus", "alacer")
        .comb_authors(Some("1887"), &["Storr"])
        .imprint_year("1886-1888")
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Melanargia halimede menetriesi Wagener, 1959 & 1961")
        .infra_species("Melanargia", "halimede", Rank::Subspecies, "menetriesi")
        .comb_authors(Some("1959"), &["Wagener"])
        .imprint_year("1961")
        .code(NomCode::Zoological)
        .nothing_else();
}

/// The four equivalent imprint-year forms from the ICZN Article 22 example:
/// <a href="https://code.iczn.org/date-of-publication/article-22-citation-of-date/">code.iczn.org/date-of-publication/article-22-citation-of-date</a>.
/// Anomalopus truncatus carries an imprint year inside the basionym brackets.
#[test]
fn iczn_imprint() {
    assert_name("Ctenotus alacer Storr, 1970 (\"1969\")")
        .species("Ctenotus", "alacer")
        .comb_authors(Some("1970"), &["Storr"])
        .imprint_year("1969")
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Ctenotus alacer Storr, 1970 [\"1969\"]")
        .species("Ctenotus", "alacer")
        .comb_authors(Some("1970"), &["Storr"])
        .imprint_year("1969")
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Ctenotus alacer Storr, 1970 (imprint 1969)")
        .species("Ctenotus", "alacer")
        .comb_authors(Some("1970"), &["Storr"])
        .imprint_year("1969")
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Ctenotus alacer Storr, 1970 (not 1969)")
        .species("Ctenotus", "alacer")
        .comb_authors(Some("1970"), &["Storr"])
        .imprint_year("1969")
        .code(NomCode::Zoological)
        .nothing_else();

    // unquoted bracketed imprint year
    assert_name("Ctenotus alacer Storr, 1970 [1969]")
        .species("Ctenotus", "alacer")
        .comb_authors(Some("1970"), &["Storr"])
        .imprint_year("1969")
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Anomalopus truncatus (Peters, 1876 [\"1877\"])")
        .species("Anomalopus", "truncatus")
        .bas_authors(Some("1876"), &["Peters"])
        .imprint_year("1877")
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn lower_case_names() {
    assert_name("abies alba Mill.")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .type_(NameType::Scientific)
        .nothing_else();
}

#[test]
fn manuscript_names() {
    assert_name("Abrodictyum caespifrons (C. Chr.) comb. ined.")
        .species("Abrodictyum", "caespifrons")
        .bas_authors(None, &["C.Chr."])
        .type_(NameType::Scientific)
        .nom_note("comb. ined.")
        .manuscript()
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Acranthera virescens (Ridl.) ined.")
        .species("Acranthera", "virescens")
        .bas_authors(None, &["Ridl."])
        .type_(NameType::Scientific)
        .nom_note("ined.")
        .manuscript()
        .code(NomCode::Zoological)
        .nothing_else();

    // real authorship is Siliç
    assert_name("Micromeria cristata subsp. kosaninii ( ilic) ined.")
        .infra_species("Micromeria", "cristata", Rank::Subspecies, "kosaninii")
        //.bas_authors(None, &["ilic"])
        .partial("(ilic)")
        .type_(NameType::Scientific)
        .nom_note("ined.")
        .manuscript()
        .nothing_else();

    assert_name("Genoplesium vernalis D.L.Jones ms.")
        .species("Genoplesium", "vernalis")
        .comb_authors(None, &["D.L.Jones"])
        .type_(NameType::Scientific)
        .manuscript()
        .nom_note("ms.")
        .nothing_else();

    // Java: `assertPhraseName(...).species("Verticordia", null)` — a null specific epithet;
    // `species()` requires a real epithet `&str`, so this reproduces assert_phrase_name's own
    // checks (canonical, phrase, rank, type) plus the genus/epithet-absent assertions directly
    // — the same DSL-gap workaround as impl_04's "Lepidoptera sp. JGP0404" case.
    assert_phrase_name(
        "Verticordia sp.1",
        "Verticordia sp. 1",
        Some(Rank::Species),
        "1",
    );
    let n = nameparser::parse("Verticordia sp.1", None, None, None)
        .expect("`Verticordia sp.1` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Verticordia"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());

    // Spelled-out "species N" placeholder keeps the verbatim marker word in the phrase and
    // renders it as-is ("Allium species 1"), not collapsed to the synthetic "sp." marker.
    assert_phrase_name(
        "Allium species 1",
        "Allium species 1",
        Some(Rank::Species),
        "species 1",
    );
    let n = nameparser::parse("Allium species 1", None, None, None)
        .expect("`Allium species 1` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Allium"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());

    assert_phrase_name("Bryozoan sp. E", "Bryozoan sp. E", Some(Rank::Species), "E");
    let n = nameparser::parse("Bryozoan sp. E", None, None, None)
        .expect("`Bryozoan sp. E` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Bryozoan"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
}

#[test]
fn phrase_names() {
    // Java: `assertName(name, expectedCanonicalWithoutAuthors).phraseIndetName(genus, phrase,
    // RANK).nothingElse()`. Per rule 2 the outer `expectedCanonicalWithoutAuthors` is dropped
    // like every other `assertName(...)` call — so, unlike `manuscriptNames` above, this does
    // NOT use `assert_phrase_name`: that helper mirrors the *different* top-level
    // `assertPhraseName` Java method and checks the WITH-authorship `canonicalName()`; reusing
    // it here would check a with-authorship string against the without-authorship value Java
    // actually asserts, which would be wrong (and would fail outright on the "Maslin" case
    // below, which has real authorship). `phraseIndetName` itself is a `NameAssertion` instance
    // method with no DSL equivalent, so each case below is a direct parse + field assertions
    // instead — the same DSL-gap workaround as impl_04's "Lepidoptera sp. JGP0404" case.
    let n = nameparser::parse(
        "Prostanthera sp. Somersbey (B.J.Conn 4024)",
        None,
        None,
        None,
    )
    .expect("`Prostanthera sp. Somersbey (B.J.Conn 4024)` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Prostanthera"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Species);
    assert_eq!(n.phrase.as_deref(), Some("Somersbey (B.J.Conn 4024)"));
    assert_eq!(n.type_, NameType::Informal);

    let n2 = nameparser::parse("Pultenaea sp. 'Olinda' (Coveny 6616)", None, None, None)
        .expect("`Pultenaea sp. 'Olinda' (Coveny 6616)` should parse");
    assert!(n2.uninomial.is_none());
    assert_eq!(n2.genus.as_deref(), Some("Pultenaea"));
    assert!(n2.infrageneric_epithet.is_none());
    assert!(n2.specific_epithet.is_none());
    assert!(n2.infraspecific_epithet.is_none());
    assert_eq!(n2.rank, Rank::Species);
    assert_eq!(n2.phrase.as_deref(), Some("'Olinda' (Coveny 6616)"));
    assert_eq!(n2.type_, NameType::Informal);

    let n3 = nameparser::parse(
        "Pterostylis sp. Sandheath (D.Murfet 3190) R.J.Bates",
        None,
        None,
        None,
    )
    .expect("`Pterostylis sp. Sandheath (D.Murfet 3190) R.J.Bates` should parse");
    assert!(n3.uninomial.is_none());
    assert_eq!(n3.genus.as_deref(), Some("Pterostylis"));
    assert!(n3.infrageneric_epithet.is_none());
    assert!(n3.specific_epithet.is_none());
    assert!(n3.infraspecific_epithet.is_none());
    assert_eq!(n3.rank, Rank::Species);
    assert_eq!(
        n3.phrase.as_deref(),
        Some("Sandheath (D.Murfet 3190) R.J.Bates")
    );
    assert_eq!(n3.type_, NameType::Informal);

    // Check to make sure base name is parsed before haring off into the wilderness
    assert_name("Acacia mutabilis Maslin")
        .species("Acacia", "mutabilis")
        .comb_authors(None, &["Maslin"])
        .nothing_else();

    assert_name("Acacia mutabilis Maslin subsp. Young River (G.F. Craig 2052)")
        .comb_authors(None, &["Maslin"]);
    let n4 = nameparser::parse(
        "Acacia mutabilis Maslin subsp. Young River (G.F. Craig 2052)",
        None,
        None,
        None,
    )
    .expect("`Acacia mutabilis Maslin subsp. Young River (G.F. Craig 2052)` should parse");
    assert!(n4.uninomial.is_none());
    assert_eq!(n4.genus.as_deref(), Some("Acacia"));
    assert!(n4.infrageneric_epithet.is_none());
    // Java's `assertPhrase` only null-checks specificEpithet for ranks other than
    // SUBSPECIES/VARIETY/FORM, so — unlike the SPECIES-rank cases above/below — it asserts
    // nothing about specificEpithet at all here (not null, not a value); left unchecked to
    // match.
    assert!(n4.infraspecific_epithet.is_none());
    assert_eq!(n4.rank, Rank::Subspecies);
    assert_eq!(n4.phrase.as_deref(), Some("Young River (G.F. Craig 2052)"));
    assert_eq!(n4.type_, NameType::Informal);

    let n5 = nameparser::parse(
        "Dampiera sp. Central Wheatbelt (L.W.Sage, F.Hort, C.A.Hollister LWS2321)",
        None,
        None,
        None,
    )
    .expect(
        "`Dampiera sp. Central Wheatbelt (L.W.Sage, F.Hort, C.A.Hollister LWS2321)` should parse",
    );
    assert!(n5.uninomial.is_none());
    assert_eq!(n5.genus.as_deref(), Some("Dampiera"));
    assert!(n5.infrageneric_epithet.is_none());
    assert!(n5.specific_epithet.is_none());
    assert!(n5.infraspecific_epithet.is_none());
    assert_eq!(n5.rank, Rank::Species);
    assert_eq!(
        n5.phrase.as_deref(),
        Some("Central Wheatbelt (L.W.Sage, F.Hort, C.A.Hollister LWS2321)")
    );
    assert_eq!(n5.type_, NameType::Informal);

    let n6 = nameparser::parse(
        "Dampiera     sp    Central  Wheatbelt (L.W.Sage,   F.Hort,   C.A.Hollister   LWS2321)",
        None,
        None,
        None,
    )
    .expect(
        "`Dampiera     sp    Central  Wheatbelt (L.W.Sage,   F.Hort,   C.A.Hollister   LWS2321)` should parse",
    );
    assert!(n6.uninomial.is_none());
    assert_eq!(n6.genus.as_deref(), Some("Dampiera"));
    assert!(n6.infrageneric_epithet.is_none());
    assert!(n6.specific_epithet.is_none());
    assert!(n6.infraspecific_epithet.is_none());
    assert_eq!(n6.rank, Rank::Species);
    assert_eq!(
        n6.phrase.as_deref(),
        Some("Central Wheatbelt (L.W.Sage, F.Hort, C.A.Hollister LWS2321)")
    );
    assert_eq!(n6.type_, NameType::Informal);

    let n7 = nameparser::parse(
        "Toechima sp. East Alligator (J.Russell-Smith 8418) NT Herbarium",
        None,
        None,
        None,
    )
    .expect("`Toechima sp. East Alligator (J.Russell-Smith 8418) NT Herbarium` should parse");
    assert!(n7.uninomial.is_none());
    assert_eq!(n7.genus.as_deref(), Some("Toechima"));
    assert!(n7.infrageneric_epithet.is_none());
    assert!(n7.specific_epithet.is_none());
    assert!(n7.infraspecific_epithet.is_none());
    assert_eq!(n7.rank, Rank::Species);
    assert_eq!(
        n7.phrase.as_deref(),
        Some("East Alligator (J.Russell-Smith 8418) NT Herbarium")
    );
    assert_eq!(n7.type_, NameType::Informal);

    let n8 = nameparser::parse(
        "Acacia sp. Mount Hilditch (M.E. Trudgen 19134)",
        None,
        None,
        None,
    )
    .expect("`Acacia sp. Mount Hilditch (M.E. Trudgen 19134)` should parse");
    assert!(n8.uninomial.is_none());
    assert_eq!(n8.genus.as_deref(), Some("Acacia"));
    assert!(n8.infrageneric_epithet.is_none());
    assert!(n8.specific_epithet.is_none());
    assert!(n8.infraspecific_epithet.is_none());
    assert_eq!(n8.rank, Rank::Species);
    assert_eq!(
        n8.phrase.as_deref(),
        Some("Mount Hilditch (M.E. Trudgen 19134)")
    );
    assert_eq!(n8.type_, NameType::Informal);
}

#[test]
fn all_caps_authors_with_page() {
    assert_name(" Anolis marmoratus girafus LAZELL 1964: 377")
        .infra_species("Anolis", "marmoratus", Rank::Subspecies, "girafus")
        .comb_authors(Some("1964"), &["Lazell"])
        .published_in_page("377")
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn test_cultivar_pattern() {
    assert_name("Abutilon 'Kentish Belle'").cultivar("Abutilon", "Kentish Belle");
    assert_name("Abutilon 'Nabob'").cultivar("Abutilon", "Nabob");
    assert_name("Abutilon \"Dall\"").cultivar("Abutilon", "Dall");
    assert_name("Arachis pintoi cv. 'Belmonte'").cultivar_sp("Arachis", "pintoi", "Belmonte");
    assert_name("Sorbus hupehensis C.K.Schneid. cv. 'November pink'")
        .cultivar_sp("Sorbus", "hupehensis", "November pink")
        .comb_authors(None, &["C.K.Schneid."]);
    assert_name("Symphoricarpos albus (L.) S.F.Blake cv. 'Turesson'")
        .cultivar_sp("Symphoricarpos", "albus", "Turesson")
        .bas_authors(None, &["L."])
        .comb_authors(None, &["S.F.Blake"]);
    assert_name("Symphoricarpos sp. cv. 'mother of pearl'")
        .cultivar("Symphoricarpos", "mother of pearl");
}

#[test]
fn test_nom_status_remarks() {
    // parser expects a dot or a space as done by the string normalizer
    assert_name("Aster megaformis sp.nov.")
        .species("Aster", "megaformis")
        .nom_note("sp. nov.");
    assert_name("Aster vulgaris Spec nov")
        .species("Aster", "vulgaris")
        .nom_note("Spec nov.");
    assert_name("Asteraceae Fam.nov.")
        .monomial_rank("Asteraceae", Rank::Family)
        .nom_note("Fam. nov.");
    assert_name("Aster Gen.nov.")
        .monomial_rank("Aster", Rank::Genus)
        .nom_note("Gen. nov.");
    // our test only catches the first match, real parsing both!
    assert_name("Perugia gruela Gen. nov. sp. nov")
        .species("Perugia", "gruela")
        .nom_note("Gen. nov. sp. nov.");
    assert_name("Abies keralia spec. nov.")
        .species("Abies", "keralia")
        .nom_note("spec. nov.");

    // Java: `.species("Abies", null)` — a null specific epithet; `species()` requires a real
    // epithet `&str`, so this one is asserted directly against the parsed fields, the same
    // DSL-gap workaround as impl_03's "Aster indet." case.
    let n = nameparser::parse("Abies sp. nov.", None, None, None)
        .expect("`Abies sp. nov.` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Abies"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Species);
    assert_eq!(n.type_, NameType::Informal);
    assert_eq!(n.nomenclatural_note.as_deref(), Some("sp. nov."));
}

/// From https://www.gbif.org/species/search?dataset_key=da38f103-4410-43d1-b716-ea6b1b92bbac&origin=SOURCE&issue=PARTIALLY_PARSABLE&advanced=1
#[test]
fn senior_epithet() {
    assert_name("Mesotrichia senior (Vachal)")
        .species("Mesotrichia", "senior")
        .bas_authors(None, &["Vachal"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Xylocopa (Koptortosoma) senior clitelligera Friese")
        .infra_species(
            "Xylocopa",
            "senior",
            Rank::InfraspecificName,
            "clitelligera",
        )
        .infrageneric("Koptortosoma")
        .comb_authors(None, &["Friese"])
        .nothing_else();
}

/// A sensu-lato marker followed by trailing junk ("L. s.lat. - Asplen trich") still
/// captures "s.lat." as the taxonomic note; the dangling remainder is parked as unparsed.
#[test]
fn sensu_lato_with_trailing_junk() {
    assert_name("Asplenium trichomanes L. s.lat. - Asplen trich")
        .species("Asplenium", "trichomanes")
        .comb_authors(None, &["L."])
        .sensu("s.lat.")
        .partial("- Asplen trich")
        .nothing_else();
}

/// "s.s." (and spaced "s. s.") is the abbreviation of sensu stricto and becomes the
/// taxonomic note — but only in its lower-case form, so uppercase author initials
/// ("S.S.Ying") are never mistaken for it.
#[test]
fn sensu_stricto_ss() {
    assert_name("Achillea millefolium s.s.")
        .species("Achillea", "millefolium")
        .sensu("s.s.")
        .nothing_else();
    assert_name("Achillea millefolium s. s.")
        .species("Achillea", "millefolium")
        .sensu("s.s.")
        .nothing_else();
    assert_name("Achillea millefolium L. s.s.")
        .species("Achillea", "millefolium")
        .comb_authors(None, &["L."])
        .sensu("s.s.")
        .nothing_else();
    assert_name("Achillea millefolium L. s.s. - junk here")
        .species("Achillea", "millefolium")
        .comb_authors(None, &["L."])
        .sensu("s.s.")
        .partial("- junk here")
        .nothing_else();
    // uppercase "S.S." initials must stay part of the author
    assert_name("Amitostigma formosana (S.S.Ying) S.S.Ying")
        .species("Amitostigma", "formosana")
        .comb_authors(None, &["S.S.Ying"])
        .bas_authors(None, &["S.S.Ying"])
        .code(NomCode::Botanical)
        .nothing_else();
}

/// A parenthesised subgenus written in lower case ("(acanthoderes)") is a malformed
/// infrageneric name — capitalise it and flag the name doubtful, but still parse the
/// surrounding species.
#[test]
fn lowercase_subgenus() {
    assert_name("Acanthoderes (acanthoderes) satanas Aurivillius, 1923")
        .species_ig("Acanthoderes", "Acanthoderes", "satanas")
        .comb_authors(Some("1923"), &["Aurivillius"])
        .code(NomCode::Zoological)
        .doubtful()
        .nothing_else();
}

/// PR2 names with underscores
/// https://www.dev.checklistbank.org/dataset/303326
#[test]
fn pr2() {
    assert_unparsable("Basal_Cryptophyceae-1", NameType::Other);
}

#[test]
fn digit_epithets_trailing() {
    assert_name("Simplexvirus humanalpha1")
        .species("Simplexvirus", "humanalpha1")
        .code(NomCode::Virus)
        .nothing_else();
    assert_name("Simplexvirus humanalpha2")
        .species("Simplexvirus", "humanalpha2")
        .code(NomCode::Virus)
        .nothing_else();
    assert_name("Lentivirus humimdef1")
        .species("Lentivirus", "humimdef1")
        .code(NomCode::Virus)
        .nothing_else();
}

/// A trailing bracketed comment introduced by a taxonomic-concept keyword
/// ("[auctt. misspelling for Eunoe]") is not part of the name — the whole bracket
/// content is captured as the taxonomic note and the leading monomial parses cleanly.
#[test]
fn bracketed_auct_note() {
    assert_name("Eunoa [auctt. misspelling for Eunoe]")
        .monomial("Eunoa")
        .sensu("auctt. misspelling for Eunoe")
        .nothing_else();
}

/// Old lichenological/botanical floras subdivide a species informally with letters
/// ("a.", "b.", "a.a.", "a.b."). Such a letter marker between the species (with or
/// without its author) and a following epithet is an infraspecific rank marker of the
/// unmappable rank OTHER — "pulverulenta" is the infraspecific epithet.
#[test]
fn letter_subdivision_rank() {
    assert_name("Graphis scripta L. a.b pulverulenta")
        .infra_species("Graphis", "scripta", Rank::Other, "pulverulenta")
        .nothing_else();
    assert_name("Graphis scripta L. a.b. pulverulenta")
        .infra_species("Graphis", "scripta", Rank::Other, "pulverulenta")
        .nothing_else();
    assert_name("Graphis scripta a.b pulverulenta")
        .infra_species("Graphis", "scripta", Rank::Other, "pulverulenta")
        .nothing_else();
    assert_name("Graphis scripta a. pulverulenta")
        .infra_species("Graphis", "scripta", Rank::Other, "pulverulenta")
        .nothing_else();
}

/// atypical hyphens, https://github.com/CatalogueOfLife/backend/issues/1178
#[test]
fn hyphens() {
    assert_name("Minilimosina v-atrum (Villeneuve, 1917)")
        .species("Minilimosina", "v-atrum")
        .bas_authors(Some("1917"), &["Villeneuve"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Aelurillus v-insignitus")
        .species("Aelurillus", "v-insignitus")
        .nothing_else();

    assert_name("Desmometopa m-nigrum")
        .species("Desmometopa", "m-nigrum")
        .nothing_else();

    assert_name("Chloroclystis v-ata")
        .species("Chloroclystis", "v-ata")
        .nothing_else();

    assert_name("Cortinarius moenne-loccozii Bidaud")
        .species("Cortinarius", "moenne-loccozii")
        .comb_authors(None, &["Bidaud"])
        .nothing_else();

    assert_name("Asarum sieboldii f. non-maculatum (Y.N.Lee) M. Kim")
        .infra_species("Asarum", "sieboldii", Rank::Form, "non-maculatum")
        .comb_authors(None, &["M.Kim"])
        .bas_authors(None, &["Y.N.Lee"])
        .code(NomCode::Botanical)
        .nothing_else();

    // atypical hyphens, https://github.com/CatalogueOfLife/backend/issues/1178
    assert_name("Passalus (Pertinax) gaboi Jiménez‑Ferbans & Reyes‑Castillo, 2022")
        .species_ig("Passalus", "Pertinax", "gaboi")
        .infrageneric("Pertinax")
        .comb_authors(Some("2022"), &["Jiménez-Ferbans", "Reyes-Castillo"])
        .code(NomCode::Zoological)
        .warning(&[warnings::HOMOGLYHPS])
        .nothing_else();
}
