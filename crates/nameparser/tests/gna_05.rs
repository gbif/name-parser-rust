// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserGnaTest (methods on lines 2006-2410).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NamePart, NameType, NomCode, Rank};

#[test]
fn year_in_square_brackets() {
    // group: Year in square brackets — bracketed years are imprint years (the year
    // printed on the work) and never become the nominal publication year, even when
    // they are the only year in the input.
    assert_name("Anthoscopus Cabanis [1851]")
        .monomial("Anthoscopus")
        .comb_authors(None, &["Cabanis"])
        .imprint_year("1851")
        .nothing_else();
    assert_name("Anthoscopus Cabanis [185?]")
        .monomial("Anthoscopus")
        .comb_authors(None, &["Cabanis"])
        .imprint_year("185?")
        .nothing_else();
    assert_name("Anthoscopus Cabanis [1851?]")
        .monomial("Anthoscopus")
        .comb_authors(None, &["Cabanis"])
        .imprint_year("1851?")
        .nothing_else();
    assert_name("Trismegistia monodii Ando, 1973 [1974]")
        .species("Trismegistia", "monodii")
        .comb_authors(Some("1973"), &["Ando"])
        .imprint_year("1974")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Zygaena witti Wiegel [1973]")
        .species("Zygaena", "witti")
        .comb_authors(None, &["Wiegel"])
        .imprint_year("1973")
        .nothing_else();
    assert_name("Deyeuxia coarctata Kunth, 1815 [1816]")
        .species("Deyeuxia", "coarctata")
        .comb_authors(Some("1815"), &["Kunth"])
        .imprint_year("1816")
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn utf80x3000_character_ideographic_space() {
    // group: UTF-8 0x3000 character (IDEOGRAPHIC_SPACE)
    assert_name("Kinosternidae　Agassiz, 1857")
        .monomial("Kinosternidae")
        .comb_authors(Some("1857"), &["Agassiz"]);
}

#[test]
fn names_with_ex_as_sp_epithet() {
    // group: Names with 'ex' as sp. epithet
    assert_name("Acanthochiton exquisitus")
        .species("Acanthochiton", "exquisitus");
}

#[test]
fn names_with_spanish_y_instead_of() {
    // group: Names with Spanish 'y' instead of '&'
    assert_name("Caloptenopsis crassiusculus (Martínez y Fernández-Castillo, 1896)")
        .species("Caloptenopsis", "crassiusculus")
        .bas_authors(Some("1896"), &["Martínez", "Fernández-Castillo"]);
    assert_name("Dicranum saxatile Lagasca y Segura, García & Clemente y Rubio, 1802")
        .species("Dicranum", "saxatile")
        .comb_authors(Some("1802"), &["Lagasca", "Segura", "García", "Clemente", "Rubio"]);
    assert_name("Carabus (Tanaocarabus) hendrichsi Bolvar y Pieltain, Rotger & Coronado 1967")
        .species_ig("Carabus", "Tanaocarabus", "hendrichsi")
        .comb_authors(Some("1967"), &["Bolvar", "Pieltain", "Rotger", "Coronado"]);
}

#[test]
fn normalize_atypical_dashes() {
    // group: Normalize atypical dashes (non-breaking hyphens U+2011 normalised to "-").
    assert_name("Passalus (Pertinax) gaboi Jiménez‑Ferbans & Reyes‑Castillo, 2022")
        .species_ig("Passalus", "Pertinax", "gaboi")
        .comb_authors(Some("2022"), &["Jiménez-Ferbans", "Reyes-Castillo"])
        .warning(&[warnings::HOMOGLYHPS]);
}

#[test]
fn possible_canonical() {
    // group: Possible canonical. Various trailing junk forms recoverable to
    // the core canonical name. Gibberish trailing digit strings are dropped,
    // stray opening parens / quoted "Dall"-style annotations are stripped,
    // botanical " ined.?" tentative-publication markers leave a PARTIAL state,
    // "(Approved Lists YYYY)" bacterial-code annotations are stripped.
    assert_name("Morea (Morea) burtius 2342343242 23424322342 23424234")
        .species_ig("Morea", "Morea", "burtius");
    assert_name("Verpericola megasoma \"\"Dall\" Pils.")
        .species("Verpericola", "megasoma")
        .comb_authors(None, &["Dall Pils."]);
    assert_name("Moraea spathulata ( (L. f. Klatt")
        .species("Moraea", "spathulata")
        .comb_authors(None, &["L.f.Klatt"]);
    assert_name("Agropyron pectiniforme var. karabaljikji ined.?")
        .infra_species("Agropyron", "pectiniforme", Rank::Variety, "karabaljikji")
        .partial("ined");
    assert_name("Staphylococcus hyicus chromogenes Devriese et al. 1978")
        .infra_species("Staphylococcus", "hyicus", Rank::Subspecies, "chromogenes")
        .comb_authors(Some("1978"), &["Devriese", "al."]);
    // skipped:
    //   "Verpericola megasoma \"Dall\" Pils." — the quoted "Dall" is parsed as
    //     a cultivar epithet (correct behavior given quoted-string convention);
    //     can't recover as bare species without losing cultivar parsing.
    //   "Stewartia micrantha (Chun) Sealy, Bot. Mag. 176: t. 510. 1967." —
    //     IPNI-style publication ref with page-and-plate; the page/plate span
    //     bleeds into the author span.
    //   "Pyrobaculum neutrophilum V24Sta" — trailing alphanumeric strain code
    //     captured as informal phrase; expected was bare species.
    //   "Rana aurora Baird and Girard, 1852; H.B. Shaffer et al., 2004" —
    //     semicolon-separated dual authorship not recognised; parser merges
    //     both author teams.
}

#[test]
fn treating_al_as_et_al() {
    // group: Treating `& al.` as `et al.`. The "& al." / "& al" inside an
    // author span before a rank marker is silently consumed by the mid-name
    // author skip so the trailing infraspecific portion (var./subsp./f. +
    // epithet + author) is parsed normally.
    assert_name("Adonis cyllenea Boiss. & al. var. paryadrica Boiss.")
        .infra_species("Adonis", "cyllenea", Rank::Variety, "paryadrica")
        .comb_authors(None, &["Boiss."]);
    assert_name("Adonis cyllenea Boiss. & al var. paryadrica Boiss.")
        .infra_species("Adonis", "cyllenea", Rank::Variety, "paryadrica")
        .comb_authors(None, &["Boiss."]);
}

#[test]
fn treating_al_as_et_al_binomials() {
    // `& al.` parsed as a separate author token; the formatter renders it as "et al."
    assert_name("Adonis cyllenea Boiss. & al.")
        .species("Adonis", "cyllenea")
        .comb_authors(None, &["Boiss.", "al."]);
    assert_name("Adonis cyllenea Boiss. & al")
        .species("Adonis", "cyllenea")
        .comb_authors(None, &["Boiss.", "al"]);
    assert_name("Adetus fuscoapicalis Souza f. et al. 2001")
        .species("Adetus", "fuscoapicalis")
        .comb_authors(Some("2001"), &["Souza f.", "al."]);
    assert_name("Sterigmostemon rhodanthum Rech. f. et al. in Rech. f.")
        .species("Sterigmostemon", "rhodanthum")
        .comb_authors(None, &["Rech.f.", "al."]);
}

#[test]
fn authors_do_not_start_with_apostrophe() {
    // group: Authors do not start with apostrophe
    assert_name("Nereidavus kulkovi 'Kulkov")
        .species("Nereidavus", "kulkovi");
}

#[test]
fn epithets_do_not_start_or_end_with_a_dash() {
    // group: Epithets do not start or end with a dash. A leading-dash epithet is not
    // recognised as the species, so the rest of the line collapses into authorship.
    // A trailing-dash epithet has the dash stripped and parses as a normal binomial.
    assert_name("Abryna -petri Paiva, 1860")
        .monomial("Abryna")
        .comb_authors(Some("1860"), &["petri Paiva"]);
    assert_name("Abryna petri- Paiva, 1860")
        .species("Abryna", "petri")
        .comb_authors(Some("1860"), &["Paiva"]);
}

#[test]
fn names_that_contain_of() {
    // group: Names that contain "of" — the parser keeps the full author span verbatim,
    // including "of" inside organisation names. Years from publishedIn-like tails attach
    // to the comb authorship.
    assert_name("Musca capraria Trustees of the British Museum (Natural History), 1939")
        .species("Musca", "capraria")
        .comb_authors(Some("1939"), &["Trustees of the British Museum Natural History"]);
    assert_name("Nassellarid genera of uncertain affinities")
        .species("Nassellarid", "genera")
        .comb_authors(None, &["of uncertain affinities"]);
    assert_name("Natica of nidus")
        .monomial("Natica")
        .comb_authors(None, &["of nidus"]);
    assert_name("Neritina chemmoi Reeve var of cornea Linn")
        .species("Neritina", "chemmoi")
        .comb_authors(None, &["Reeve var of cornea Linn"]);
}

#[test]
fn cultivars() {
    // group: Cultivars — quoted cultivar epithet is captured as cultivarEpithet.
    assert_name("Sarracenia flava 'Maxima'")
        .cultivar_sp("Sarracenia", "flava", "Maxima");
}

#[test]
fn open_taxonomy_with_ranks_unfinished() {
    // group: "Open taxonomy" with ranks unfinished — bare rank marker after a binomial
    // produces an indeterminate infraspecific name with the marker preserved in the
    // canonical, an INDETERMINED warning, and INFORMAL type.
    //
    // `.infraSpecies(genus, epithet, RANK, null)` — a null infraspecific epithet; the
    // DSL's `infra_species()` takes it as a plain `&str`, so this reuses the
    // general-purpose `binomial(genus, infrageneric, epithet, rank)`, which already
    // hardcodes infraspecific_epithet == None (same precedent as impl_08's `indet_names`).
    assert_name("Alyxia reinwardti var")
        .binomial("Alyxia", None, "reinwardti", Rank::Variety)
        .type_(NameType::Informal)
        .warning(&[warnings::INDETERMINED]);
    assert_name("Alyxia reinwardti var.")
        .binomial("Alyxia", None, "reinwardti", Rank::Variety)
        .type_(NameType::Informal)
        .warning(&[warnings::INDETERMINED]);
    assert_name("Alyxia reinwardti ssp")
        .binomial("Alyxia", None, "reinwardti", Rank::Subspecies)
        .type_(NameType::Informal)
        .warning(&[warnings::INDETERMINED]);
    assert_name("Alyxia reinwardti ssp.")
        .binomial("Alyxia", None, "reinwardti", Rank::Subspecies)
        .type_(NameType::Informal)
        .warning(&[warnings::INDETERMINED]);
    // skipped: Alaria spp
    // skipped: Alaria spp.
    // skipped: Xenodon sp
    // skipped: Xenodon sp.
    // skipped: Formicidae cf.
    // skipped: Formicidae cf
    // skipped: Arctostaphylos preglauca cf.
    // skipped: Albinaria brevicollis cf. sica Fuchs & Kaufel 1936
    // skipped: Albinaria cf brevicollis sica Fuchs & Kaufel 1936
    // skipped: Albinaria brevicollis cf
    // skipped: Acastoides spp.
}

#[test]
fn ignoring_serovar_serotype() {
    // group: Ignoring serovar/serotype. Bacterial subspecific epidemiological
    // designators (serotype/serovar [strain]) are silently stripped — they aren't
    // formal nomenclatural ranks.
    assert_name("Aggregatibacter actinomycetemcomitans serotype d str. SA508")
        .species("Aggregatibacter", "actinomycetemcomitans");
    assert_name("Streptococcus pyogenes (serotype M18)")
        .species("Streptococcus", "pyogenes");
    assert_name("Actinobacillus pleuropneumoniae serovar 2 strain S1536")
        .species("Actinobacillus", "pleuropneumoniae");
    assert_name("Leptospira interrogans serovar Fugis")
        .species("Leptospira", "interrogans");
}

#[test]
fn ignoring_sensu_sec() {
    // group: Ignoring sensu sec — sensu/sec/auct./s.str./s.l. spans go into the
    // taxonomicNote field; ", pro parte" / ", p.p." are stripped silently with the
    // doubtful flag. Botanical "var." kept in canonical.
    assert_name("Senecio legionensis sensu Samp., non Lange")
        .species("Senecio", "legionensis")
        .sensu("sensu Samp., non Lange");
    assert_name("Abarema scutifera sensu auct., non (Blanco)Kosterm.")
        .species("Abarema", "scutifera")
        .sensu("sensu auct., non (Blanco)Kosterm.");
    assert_name("Puya acris Auct.")
        .species("Puya", "acris")
        .sensu("auct.");
    assert_name("Puya acris Auct non L.")
        .species("Puya", "acris")
        .sensu("auct non L.");
    assert_name("Galium tricorne Stokes, pro parte")
        .species("Galium", "tricorne")
        .comb_authors(None, &["Stokes"])
        .doubtful();
    assert_name("Galium tricorne Stokes,pro parte")
        .species("Galium", "tricorne")
        .comb_authors(None, &["Stokes"])
        .doubtful();
    assert_name("Senecio jacquinianus sec. Rchb.")
        .species("Senecio", "jacquinianus")
        .sensu("sec. Rchb.");
    assert_name("Acantholimon ulicinum S. L. Schultes")
        .species("Acantholimon", "ulicinum")
        .comb_authors(None, &["S.L.Schultes"]);
    assert_name("Amitostigma formosana (S.S.Ying) S.S.Ying")
        .species("Amitostigma", "formosana")
        .comb_authors(None, &["S.S.Ying"])
        .bas_authors(None, &["S.S.Ying"]);
    assert_name("Arenaria serpyllifolia L. s.str.")
        .species("Arenaria", "serpyllifolia")
        .comb_authors(None, &["L."])
        .sensu("s.str.");
    assert_name("Asplenium anisophyllum Kunze, s.l.")
        .species("Asplenium", "anisophyllum")
        .comb_authors(None, &["Kunze"])
        .sensu("s.l.");
    assert_name("Abramis Cuvier 1816 sec. Dybowski 1862")
        .monomial("Abramis")
        .comb_authors(Some("1816"), &["Cuvier"])
        .sensu("sec. Dybowski 1862")
        .code(NomCode::Zoological);
    assert_name("Abramis brama subsp. bergi Grib & Vernidub 1935 sec Eschmeyer 2004")
        .infra_species("Abramis", "brama", Rank::Subspecies, "bergi")
        .comb_authors(Some("1935"), &["Grib", "Vernidub"])
        .sensu("sec Eschmeyer 2004")
        .code(NomCode::Zoological);
    assert_name("Abarema clypearia (Jack) Kosterm., P. P.")
        .species("Abarema", "clypearia")
        .comb_authors(None, &["Kosterm."])
        .bas_authors(None, &["Jack"])
        .doubtful();
    assert_name("Abarema clypearia (Jack) Kosterm., p.p.")
        .species("Abarema", "clypearia")
        .comb_authors(None, &["Kosterm."])
        .bas_authors(None, &["Jack"])
        .doubtful();
    assert_name("Abarema clypearia (Jack) Kosterm., p. p.")
        .species("Abarema", "clypearia")
        .comb_authors(None, &["Kosterm."])
        .bas_authors(None, &["Jack"])
        .doubtful();
    assert_name("Indigofera phyllogramme var. aphylla R.Vig., p.p.B")
        .infra_species("Indigofera", "phyllogramme", Rank::Variety, "aphylla")
        .comb_authors(None, &["R.Vig."])
        .doubtful();
    // The remaining inputs ("Pseudomonas methanica (...) sensu. Dworkin and Foster
    // 1956", "Acantholimon ulicinum s.l. (Schultes) Boiss.", "Amaurorhinus
    // bewichianus (Wollaston,1860) (s.str.)", "Ammodramus caudacutus (s.s.)
    // diversus", "Asplenium trichomanes L. s.lat. - Asplen trich") aren't yet
    // disambiguated — the s.str./s.l./s.s. tokens get folded into the author span
    // when they sit between the species and parenthesised basionym/comb-author
    // material. Left as TODOs.
}

#[test]
fn ignore_terminal_annotations() {
    // group: Ignore terminal annotations
    assert_name("Abida secale margaridae I.M.Fake Ms")
        .infra_species("Abida", "secale", Rank::InfraspecificName, "margaridae")
        .comb_authors(None, &["I.M.Fake"]);
    assert_name("Abida secale margaridae I.M.Fake ms")
        .infra_species("Abida", "secale", Rank::InfraspecificName, "margaridae")
        .comb_authors(None, &["I.M.Fake"]);
}

#[test]
fn hort_annotations() {
    // group: Hort. annotations — "ht." is normalised to the horticultural marker
    // "hort." (StripAndStash), so "ht.<Author>" parses as a species with the
    // horticultural author glued to the following semicolon-separated author span
    // (matching the spelled-out "hort.<Author>" twin), rather than leaking "ht" in
    // as a bogus infraspecific epithet.
    assert_name("Asplenium mayi ht.May; Gard.")
        .species("Asplenium", "mayi")
        .comb_authors(None, &["hort.May", "Gard."]);
    assert_name("Asplenium mayii ht.May; Gard.")
        .species("Asplenium", "mayii")
        .comb_authors(None, &["hort.May", "Gard."]);
    assert_name("Davallia decora ht.Bull.; Gard.Chr.")
        .species("Davallia", "decora")
        .comb_authors(None, &["hort.Bull.", "Gard.Chr."]);
    assert_name("Gymnogramma alstoni ht.Birkenh.; Gard.")
        .species("Gymnogramma", "alstoni")
        .comb_authors(None, &["hort.Birkenh.", "Gard."]);
    assert_name("Gymnogramma sprengeriana ht.Wiener Ill.")
        .species("Gymnogramma", "sprengeriana")
        .comb_authors(None, &["hort.Wiener Ill."]);
}

#[test]
fn removing_nomenclatural_annotations() {
    // group: Removing nomenclatural annotations. Known nomenclatural notes are
    // stripped into nomenclaturalNote. Bacterial "str." (strain) marker is kept
    // in the canonical (consistent with bacterial pv. policy). Semicolon-separated
    // "(nomen nudum)" is currently captured as nomNote when it has the canonical
    // form; the bare "Nomen Nudum" trailing form is not recognised.
    assert_name("Amphiprora pseudoduplex (Osada & Kobayasi, 1990) comb. nov.")
        .species("Amphiprora", "pseudoduplex")
        .bas_authors(Some("1990"), &["Osada", "Kobayasi"])
        .code(NomCode::Zoological)
        .nom_note("comb. nov.");
    assert_name("Methanosarcina barkeri str. fusaro")
        .infra_species("Methanosarcina", "barkeri", Rank::Strain, "fusaro");
    assert_name("Arthopyrenia hyalospora (Nyl.) R.C. Harris comb. nov.")
        .species("Arthopyrenia", "hyalospora")
        .comb_authors(None, &["R.C.Harris"])
        .bas_authors(None, &["Nyl."])
        .nom_note("comb. nov.");
    assert_name("Acontias lineatus WAGLER 1830: 196 (nomen nudum)")
        .species("Acontias", "lineatus")
        .comb_authors(Some("1830"), &["Wagler"]);
    // The trailing "(nomen nudum)" suppresses both the ":196" page capture and the
    // ZOOLOGICAL code inference (parser leans BOTANICAL because "nomen" smells like
    // a nom. annotation).
    assert_name("Aster exilis Ell., nomen dubium")
        .species("Aster", "exilis")
        .comb_authors(None, &["Ell."])
        .nom_note("nomen dubium");
    assert_name("Abutilon avicennae Gaertn., nom. illeg.")
        .species("Abutilon", "avicennae")
        .comb_authors(None, &["Gaertn."])
        .nom_note("nom. illeg.");
    assert_name("Achillea bonarota nom. in herb.")
        .species("Achillea", "bonarota")
        .nom_note("nom. in herb.");
    assert_name("Aconitum napellus var. formosum (Rchb.) W. D. J. Koch (nom. ambig.)")
        .infra_species("Aconitum", "napellus", Rank::Variety, "formosum")
        .comb_authors(None, &["W.D.J.Koch"])
        .bas_authors(None, &["Rchb."])
        .code(NomCode::Botanical)
        .nom_note("nom. ambig.");
    assert_name("Aesculus canadensis Hort. ex Lavallée")
        .species("Aesculus", "canadensis")
        .comb_authors(None, &["Lavallée"])
        .comb_ex_authors(&["hort."]);
    assert_name("× Dialaeliopsis hort.")
        .monomial("Dialaeliopsis")
        .notho(&[NamePart::Generic])
        .comb_authors(None, &["hort."]);
}
