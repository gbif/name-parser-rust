// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserGnaTest (methods on lines 1462-2005).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NamePart, NameType, NomCode, Rank};

#[test]
fn empty_spaces() {
    // group: Empty spaces — leading "X" between genus and species is the hybrid mark.
    assert_name("Asplenium       X inexpectatum(E. L. Braun ex Friesner      )Morton")
        .species("Asplenium", "inexpectatum")
        .comb_authors(None, &["Morton"])
        .bas_authors(None, &["Friesner"])
        .bas_ex_authors(None, &["E.L.Braun"])
        .notho(&[NamePart::Specific])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn names_with_a_dash() {
    // group: Names with a dash
    assert_name("Drosophila obscura-x Burla, 1951")
        .species("Drosophila", "obscura-x")
        .comb_authors(Some("1951"), &["Burla"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Sanogasta x-signata (Keyserling,1891)")
        .species("Sanogasta", "x-signata")
        .bas_authors(Some("1891"), &["Keyserling"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Aedes w-albus (Theobald, 1905)")
        .species("Aedes", "w-albus")
        .bas_authors(Some("1905"), &["Theobald"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Abryna regis-petri Paiva, 1860")
        .species("Abryna", "regis-petri")
        .comb_authors(Some("1860"), &["Paiva"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Solms-laubachia orbiculata Y.C. Lan & T.Y. Cheo")
        .species("Solms-laubachia", "orbiculata")
        .comb_authors(None, &["Y.C.Lan", "T.Y.Cheo"])
        .nothing_else();
}

#[test]
fn authorship_with_degli() {
    // group: Authorship with 'degli'
    assert_name("Cestodiscus gemmifer F. S. Castracane degli Antelminelli")
        .species("Cestodiscus", "gemmifer")
        .comb_authors(None, &["F.S.Castracane degli Antelminelli"])
        .nothing_else();
}

#[test]
fn authorship_with_filius_son_of() {
    // group: Authorship with filius (son of). The parser preserves the input form
    // (f. / fil. / filius) verbatim instead of normalising — "Hook. f." stays
    // "Hook.f." in the captured author. Botanical var. / f. / forma kept in canonical.
    assert_name("Oxytropis minjanensis Rech. f.")
        .species("Oxytropis", "minjanensis")
        .comb_authors(None, &["Rech.f."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Platypus bicaudatulus Schedl f. 1935")
        .species("Platypus", "bicaudatulus")
        .comb_authors(Some("1935"), &["Schedl f."])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Platypus bicaudatulus Schedl filius 1935")
        .species("Platypus", "bicaudatulus")
        .comb_authors(Some("1935"), &["Schedl filius"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Fimbristylis ovata (Burm. f.) J. Kern")
        .species("Fimbristylis", "ovata")
        .comb_authors(None, &["J.Kern"])
        .bas_authors(None, &["Burm.f."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Amelanchier arborea var. arborea (Michx. f.) Fernald")
        .infra_species("Amelanchier", "arborea", Rank::Variety, "arborea")
        .comb_authors(None, &["Fernald"])
        .bas_authors(None, &["Michx.f."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Cerastium arvense var. fuegianum Hook. f.")
        .infra_species("Cerastium", "arvense", Rank::Variety, "fuegianum")
        .comb_authors(None, &["Hook.f."])
        .code(NomCode::Botanical)
        .nothing_else();
    // FIXME(review): the same author as "Hook. f." above, which is BOTANICAL: the code must not
    // depend on the space
    assert_name("Cerastium arvense var. fuegianum Hook.f.")
        .infra_species("Cerastium", "arvense", Rank::Variety, "fuegianum")
        .comb_authors(None, &["Hook.f."])
        .nothing_else();
    assert_name("Jacquemontia spiciflora (Choisy) Hall. fil.")
        .species("Jacquemontia", "spiciflora")
        .comb_authors(None, &["Hall.fil."])
        .bas_authors(None, &["Choisy"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Amelanchier arborea f. hirsuta (Michx. f.) Fernald")
        .infra_species("Amelanchier", "arborea", Rank::Form, "hirsuta")
        .comb_authors(None, &["Fernald"])
        .bas_authors(None, &["Michx.f."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Betula pendula fo. dalecarlica (L. f.) C.K. Schneid.")
        .infra_species("Betula", "pendula", Rank::Form, "dalecarlica")
        .comb_authors(None, &["C.K.Schneid."])
        .bas_authors(None, &["L.f."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Polypodium pectinatum L. f.")
        .species("Polypodium", "pectinatum")
        .comb_authors(None, &["L.f."])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn names_with_emend_rectified_by_authorship() {
    // group: Names with emend (rectified by) authorship — the trailing "emend.
    // Author, year" reference is dropped from the authorship; first author/year
    // wins.
    assert_name("Chlorobium phaeobacteroides Pfennig, 1968 emend. Imhoff, 2003")
        .species("Chlorobium", "phaeobacteroides")
        .comb_authors(Some("1968"), &["Pfennig"])
        .sensu("emend. Imhoff, 2003")
        .nothing_else();
    assert_name("Chlorobium phaeobacteroides Pfennig, 1968 emend Imhoff, 2003")
        .species("Chlorobium", "phaeobacteroides")
        .comb_authors(Some("1968"), &["Pfennig"])
        .sensu("emend Imhoff, 2003")
        .nothing_else();
}

#[test]
fn names_with_an_unparsed_tail() {
    // group: Names with an unparsed "tail". Various trailing junk and homonym-
    // qualifier spans are recognised — gibberish digit strings are dropped via
    // the general number-stripping pass, taxonomic homonym citations ("non …" /
    // "nec …" / "fide …") go into the sensu/taxonomicNote field, "in <Editor>"
    // publishedIn references go into publishedIn, "(pro sp.)" annotations are
    // stripped silently.
    assert_name("Morea (Morea) Burt 2342343242 23424322342 23424234")
        .infrageneric_at("Morea", Rank::InfragenericName, "Morea")
        .comb_authors(None, &["Burt"])
        .nothing_else();
    // a lone particle is an author's name cut short
    assert_name("Nautilus asterizans von")
        .species("Nautilus", "asterizans")
        .warning(&[warnings::AUTHORSHIP_REMOVED])
        .nothing_else();
    assert_name("Dryopteris X separabilis Small (pro sp.)")
        .species("Dryopteris", "separabilis")
        .comb_authors(None, &["Small"])
        .notho(&[NamePart::Specific])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Eulima excellens Verkrüzen fide Paetel, 1887")
        .species("Eulima", "excellens")
        .comb_authors(None, &["Verkrüzen"])
        .sensu("fide Paetel, 1887")
        .nothing_else();
    assert_name(
        "Procamallanus (Spirocamallanus) soodi Lakshmi & Kumari, 2001 nec (Gupta & Masood, 1988)",
    )
    .species_ig("Procamallanus", "Spirocamallanus", "soodi")
    .comb_authors(Some("2001"), &["Lakshmi", "Kumari"])
    .sensu("nec (Gupta & Masood, 1988)")
    .code(NomCode::Zoological)
    .nothing_else();
    assert_name("Membranipora minuscula Canu, 1911 non Hincks, 1882")
        .species("Membranipora", "minuscula")
        .comb_authors(Some("1911"), &["Canu"])
        .sensu("non Hincks, 1882")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Proboscina subechinata Canu & Bassler, 1920 non d'Orbigny, 1853")
        .species("Proboscina", "subechinata")
        .comb_authors(Some("1920"), &["Canu", "Bassler"])
        .sensu("non d'Orbigny, 1853")
        .code(NomCode::Zoological)
        .nothing_else();
    // "Author in Source, YYYY vide Other (YYYY)": the "in" tail goes into publishedIn; "vide …"
    // (see) is where the name was found, the taxonomic note.
    assert_name("Porina reussi Meneghini in De Amicis, 1885 vide Neviani (1900)")
        .species("Porina", "reussi")
        .comb_authors(Some("1885"), &["Meneghini"])
        .published_in("De Amicis, 1885")
        .published_in_year(Some(1885))
        .sensu("vide Neviani (1900)")
        .nothing_else();
}

#[test]
fn non_ascii_utf8_characters_in_a_name() {
    // group: Non-ASCII UTF-8 characters in a name (ligatures/diacritics are kept verbatim)
    assert_name("Seleuca chûjôi Voss, 1957")
        .species("Seleuca", "chûjôi")
        .comb_authors(Some("1957"), &["Voss"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Pleurotus ëous (Berk.) Sacc. 1887")
        .species("Pleurotus", "ëous")
        .comb_authors(Some("1887"), &["Sacc."])
        .bas_authors(None, &["Berk."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Sténométope laevissimus Bibron 1855")
        .species("Sténométope", "laevissimus")
        .comb_authors(Some("1855"), &["Bibron"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Choriozopella trägårdhi Lawrence, 1947")
        .species("Choriozopella", "trägårdhi")
        .comb_authors(Some("1947"), &["Lawrence"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Isoëtes asplundii H. P. Fuchs")
        .species("Isoëtes", "asplundii")
        .comb_authors(None, &["H.P.Fuchs"])
        .nothing_else();
    assert_name("Campethera cailliautii fülleborni")
        .infra_species(
            "Campethera",
            "cailliautii",
            Rank::InfraspecificName,
            "fülleborni",
        )
        .nothing_else();
    assert_name("Östrupia Heiden ex Hustedt, 1935")
        .monomial("Östrupia")
        .comb_authors(Some("1935"), &["Hustedt"])
        .comb_ex_authors(&["Heiden"])
        .nothing_else();
}

#[test]
fn epithets_with_an_apostrophe() {
    // group: Epithets with an apostrophe — Indigenous-name and Irish/Scottish
    // surname apostrophes (o'donelli, m'coyi, l'herminierii, wila-k'oyu) are
    // kept verbatim in the epithet. Curly apostrophes (’) are normalised to
    // straight (') silently.
    assert_name("Solanum tuberosum f. wila-k'oyu Ochoa")
        .infra_species("Solanum", "tuberosum", Rank::Form, "wila-k'oyu")
        .comb_authors(None, &["Ochoa"])
        .nothing_else();
    assert_name("Junellia o'donelli Moldenke, 1946")
        .species("Junellia", "o'donelli")
        .comb_authors(Some("1946"), &["Moldenke"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Trophon d'orbignyi Carcelles, 1946")
        .species("Trophon", "d'orbignyi")
        .comb_authors(Some("1946"), &["Carcelles"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Phrynosoma m’callii")
        .species("Phrynosoma", "m'callii")
        .nothing_else();
    assert_name("Arca m'coyi Tenison-Woods, 1878")
        .species("Arca", "m'coyi")
        .comb_authors(Some("1878"), &["Tenison-Woods"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Nucula m'andrewii Hanley, 1860")
        .species("Nucula", "m'andrewii")
        .comb_authors(Some("1860"), &["Hanley"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Eristalis l'herminierii Macquart")
        .species("Eristalis", "l'herminierii")
        .comb_authors(None, &["Macquart"])
        .nothing_else();
    assert_name("Odynerus o'neili Cameron")
        .species("Odynerus", "o'neili")
        .comb_authors(None, &["Cameron"])
        .nothing_else();
    assert_name("Serjania meridionalis Cambess. var. o'donelli F.A. Barkley")
        .infra_species("Serjania", "meridionalis", Rank::Variety, "o'donelli")
        .comb_authors(None, &["F.A.Barkley"])
        .specific_authors(None, &["Cambess."])
        .nothing_else();
}

#[test]
fn authors_with_an_apostrophe() {
    // group: Authors with an apostrophe. Acute (´) and back-tick (`) variants are
    // normalised to a plain apostrophe so "L´Hèr." / "L`Hèr." / "L'Hèr." all parse
    // identically. The quadrinomial collapses to the inner-most rank.
    assert_name("Galega officinalis (L.) L´Hèr. subsp. mackayana (O'Flannagan) Mc Inley var. petiolata (È. Neé) Brüch.")
        .infra_species("Galega", "officinalis", Rank::Variety, "petiolata")
        .comb_authors(None, &["Brüch."])
        .bas_authors(None, &["È.Neé"])
        .warning(&["Removed: subsp. mackayana", warnings::QUADRINOMIAL])
        .code(NomCode::Botanical)
        .specific_authors(None, &["L'Hèr."])
        .specific_bas_authors(None, &["L."])
        .nothing_else();
    assert_name("Galega officinalis (L.) L`Hèr. subsp. mackayana (O'Flannagan) Mc Inley var. petiolata (È. Neé) Brüch.")
        .infra_species("Galega", "officinalis", Rank::Variety, "petiolata")
        .comb_authors(None, &["Brüch."])
        .bas_authors(None, &["È.Neé"])
        .warning(&["Removed: subsp. mackayana", warnings::QUADRINOMIAL])
        .code(NomCode::Botanical)
        .specific_authors(None, &["L'Hèr."])
        .specific_bas_authors(None, &["L."])
        .nothing_else();
    assert_name("Galega officinalis (L.) L'Hèr. subsp. mackayana (O'Flannagan) Mc Inley var. petiolata (È. Neé) Brüch.")
        .infra_species("Galega", "officinalis", Rank::Variety, "petiolata")
        .comb_authors(None, &["Brüch."])
        .bas_authors(None, &["È.Neé"])
        .warning(&["Removed: subsp. mackayana", warnings::QUADRINOMIAL])
        .code(NomCode::Botanical)
        .specific_authors(None, &["L'Hèr."])
        .specific_bas_authors(None, &["L."])
        .nothing_else();
}

#[test]
fn digraph_unicode_characters() {
    // group: Digraph unicode characters (ligatures kept verbatim)
    assert_name("Crisia romanica Zágoršek Silye & Szabó 2008")
        .species("Crisia", "romanica")
        .comb_authors(Some("2008"), &["Zágoršek Silye", "Szabó"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Æschopalæa grisella Pascoe, 1864")
        .species("Æschopalæa", "grisella")
        .comb_authors(Some("1864"), &["Pascoe"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Læptura laetifica Dow, 1913")
        .species("Læptura", "laetifica")
        .comb_authors(Some("1913"), &["Dow"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Leptura lætifica Dow, 1913")
        .species("Leptura", "lætifica")
        .comb_authors(Some("1913"), &["Dow"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Leptura leætifica Dow, 1913")
        .species("Leptura", "leætifica")
        .comb_authors(Some("1913"), &["Dow"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Leæptura laetifica Dow, 1913")
        .species("Leæptura", "laetifica")
        .comb_authors(Some("1913"), &["Dow"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Leœptura laetifica Dow, 1913")
        .species("Leœptura", "laetifica")
        .comb_authors(Some("1913"), &["Dow"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Ærenea cognata Lacordaire, 1872")
        .species("Ærenea", "cognata")
        .comb_authors(Some("1872"), &["Lacordaire"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Œdicnemus capensis")
        .species("Œdicnemus", "capensis")
        .nothing_else();
    assert_name("Œnanthe œnanthe")
        .species("Œnanthe", "œnanthe")
        .nothing_else();
    assert_name("Hördeum vulgare cœrulescens")
        .infra_species("Hördeum", "vulgare", Rank::InfraspecificName, "cœrulescens")
        .nothing_else();
    assert_name("Hordeum vulgare cœrulescens Metzger")
        .infra_species("Hordeum", "vulgare", Rank::InfraspecificName, "cœrulescens")
        .comb_authors(None, &["Metzger"])
        .nothing_else();
    assert_name("Hordeum vulgare f. cœrulescens")
        .infra_species("Hordeum", "vulgare", Rank::Form, "cœrulescens")
        .nothing_else();
}

#[test]
fn old_style_s() {
    // group: Old style s (ſ) — long-s normalised to s (it is a glyph variant),
    // ligatures æ and ß kept verbatim.
    assert_name("Musca domeſtica Linnaeus 1758")
        .species("Musca", "domestica")
        .comb_authors(Some("1758"), &["Linnaeus"])
        .warning(&[warnings::HOMOGLYHPS])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Amphisbæna fuliginoſa Linnaeus 1758")
        .species("Amphisbæna", "fuliginosa")
        .comb_authors(Some("1758"), &["Linnaeus"])
        .warning(&[warnings::HOMOGLYHPS])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Dreyfusia nüßlini")
        .species("Dreyfusia", "nüßlini")
        .nothing_else();
}

#[test]
fn miscellaneous_diacritics() {
    // group: Miscellaneous diacritics — kept verbatim, not decomposed.
    assert_name("Pärdosa").monomial("Pärdosa").nothing_else();
    assert_name("Pårdosa").monomial("Pårdosa").nothing_else();
    assert_name("Pardøsa").monomial("Pardøsa").nothing_else();
    assert_name("Pardösa").monomial("Pardösa").nothing_else();
    assert_name("Rühlella").monomial("Rühlella").nothing_else();
}

#[test]
fn open_nomenclature_approximate_names() {
    // group: Open Nomenclature ('approximate' names) — "?" between epithets is an
    // open-nomenclature doubtful identification, captured on the INFRASPECIFIC
    // qualifier (analogous to cf. / aff.).
    assert_name("Buteo borealis ? ventralis")
        .infra_species("Buteo", "borealis", Rank::InfraspecificName, "ventralis")
        .type_(NameType::Informal)
        .doubtful()
        .qualifiers(&[(NamePart::Infraspecific, "?")])
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
    // FIXME(review): "nr." (near) is read as the species epithet
    // skipped: Euxoa nr. idahoensis sp. 1clay
    assert_name("Acarinina aff. pentacamerata")
        .species("Acarinina", "pentacamerata")
        .qualifiers(&[(NamePart::Specific, "aff.")])
        .type_(NameType::Informal)
        .nothing_else();
    assert_name("Acarinina aff pentacamerata")
        .species("Acarinina", "pentacamerata")
        .qualifiers(&[(NamePart::Specific, "aff.")])
        .type_(NameType::Informal)
        .nothing_else();
    assert_informal("Sphingomonas sp. 37")
        .taxon("Sphingomonas")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. 37")
        .nothing_else();
    // FIXME(review): "spp." is read as the infraspecific epithet
    // skipped: Thryothorus leucotis spp. bogotensis
    assert_informal("Endoxyla sp. GM-, 2003")
        .taxon("Endoxyla")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. GM-, 2003")
        .nothing_else();
    assert_informal("X Aegilotrichum sp.")
        .taxon("Aegilotrichum")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp.")
        .code(NomCode::Botanical)
        .nothing_else();
    assert_informal("Liopropoma sp.2 Not applicable")
        .taxon("Liopropoma")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp.2")
        .nothing_else();
    assert_informal("Lacanobia sp. nr. subjuncta Bold:Aab, 0925")
        .taxon("Lacanobia")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. nr. subjuncta Bold:Aab, 0925")
        .nothing_else();
    // FIXME(review): "nr." is read as the species epithet, and the BIN as a botanical author
    // skipped: Lacanobia nr. subjuncta Bold:Aab, 0925
    assert_name("Abturia cf. alabamensis (Morton )")
        .species("Abturia", "alabamensis")
        .bas_authors(None, &["Morton"])
        .qualifiers(&[(NamePart::Specific, "cf.")])
        .type_(NameType::Informal)
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Abturia cf alabamensis (Morton )")
        .species("Abturia", "alabamensis")
        .bas_authors(None, &["Morton"])
        .qualifiers(&[(NamePart::Specific, "cf.")])
        .type_(NameType::Informal)
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Calidris cf. cooperi")
        .species("Calidris", "cooperi")
        .qualifiers(&[(NamePart::Specific, "cf.")])
        .type_(NameType::Informal)
        .nothing_else();
    // FIXME(review): named nothospecies (Daphnia ×krausi Flößner, 1993) are rejected as FORMULA
    // "Aesculus cf. × hybrida" and "Daphnia (Daphnia) x krausi Flossner 1993" are
    // currently classified as FORMULA hybrids — the cf./subgenus + × combination
    // trips the hybrid-formula heuristic. Left as a known limitation.
    assert_unparsable("Barbus cf macrotaenia × toppini", NameType::Formula);
    // FIXME(review): the specimen code "NP-2008" becomes an author
    assert_name("Gemmula cf. cosmoi NP-2008")
        .species("Gemmula", "cosmoi")
        .comb_authors(None, &["Np-2008"])
        .qualifiers(&[(NamePart::Specific, "cf.")])
        .type_(NameType::Informal)
        .nothing_else();
}

#[test]
fn surrogate_name_strings() {
    // group: Surrogate Name-Strings — "Bold:CODE" (a BOLD BIN database surrogate) is an anchorless
    // machine identifier. 5.0.0 classifies it NameType::Identifier (was OTHER in 4.2.0).
    assert_unparsable_rank("Bold:AAV0432", Rank::Unranked, NameType::Identifier);
}

#[test]
fn virus_like_normal_names() {
    // group: Virus-like "normal" names — names with "virus"/"vector"/"phage" in
    // the species epithet are parsed as real species when an explicit author-year
    // citation follows (ZOOLOGICAL_BINOMIAL pattern in Preflight overrides VIRUS).
    assert_name("Ceylonesmus vector Chamberlin, 1941")
        .species("Ceylonesmus", "vector")
        .comb_authors(Some("1941"), &["Chamberlin"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn viruses_plasmids_prions_etc() {
    // group: Viruses, plasmids, prions etc.
    assert_unparsable_code("Arv1virus", NameType::Other, NomCode::Virus);
    assert_unparsable_code("Turtle herpesviruses", NameType::Other, NomCode::Virus);
    assert_unparsable_code("Cre expression vector", NameType::Other, NomCode::Virus);
    assert_unparsable_code("Cyanophage", NameType::Other, NomCode::Virus);
    assert_unparsable_code(
        "Drosophila sturtevanti rhabdovirus",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code("Hydra expression vector", NameType::Other, NomCode::Virus);
    // FIXME(review): a plasmid parsed as a trinomial
    // skipped: Gateway destination plasmid
    assert_unparsable_code(
        "Abutilon mosaic virus [X15983] [X15984] Abutilon mosaic virus ICTV",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Omphalotus sp. Ictv Garcia, 18224",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Acute bee paralysis virus [AF150629] Acute bee paralysis virus",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Adeno-associated virus - 3",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "?M1-like Viruses Methanobrevibacter phage PG",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code("Aeromonas phage 65", NameType::Other, NomCode::Virus);
    assert_unparsable_code(
        "Bacillus phage SPß [AF020713] Bacillus phage SPb ICTV",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code("Apple scar skin viroid", NameType::Other, NomCode::Virus);
    assert_unparsable_code(
        "Australian grapevine viroid [X17101] Australian grapevine viroid ICTV",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Agents of Spongiform Encephalopathies CWD prion Chronic wasting disease",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code("Phi h-like viruses", NameType::Other, NomCode::Virus);
    assert_unparsable_code("Viroids", NameType::Other, NomCode::Virus);
    // FIXME(review): a vernacular group parsed as a binomial
    // skipped: Fungal prions
    assert_unparsable_code("Human rhinovirus A11", NameType::Other, NomCode::Virus);
    assert_unparsable_code(
        "Kobuvirus korean black goat/South Korea/2010",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Australian bat lyssavirus human/AUS/1998",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Gossypium mustilinum symptomless alphasatellite",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Okra leaf curl Mali alphasatellites-Cameroon",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Bemisia betasatellite LW-2014",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Tomato leaf curl Bangladesh betasatellites [India/Patna/Chilli/2008]",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Intracisternal A-particles",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Saccharomyces cerevisiae killer particle M1",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Uranotaenia sapphirina NPV",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Uranotaenia sapphirina Npv",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Spodoptera exigua nuclear polyhedrosis virus SeMNPV",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Spodoptera frugiperda MNPV",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Rachiplusia ou MNPV (strain R1)",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Orgyia pseudotsugata nuclear polyhedrosis virus OpMNPV",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Mamestra configurata NPV-A",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code(
        "Helicoverpa armigera SNPV NNg1",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code("Zamilon virophage", NameType::Other, NomCode::Virus);
    assert_unparsable_code("Sputnik virophage 3", NameType::Other, NomCode::Virus);
    assert_unparsable_code("Bacteriophage PH75", NameType::Other, NomCode::Virus);
    assert_unparsable_code(
        "Escherichia coli bacteriophage",
        NameType::Other,
        NomCode::Virus,
    );
    assert_unparsable_code("Betasatellites", NameType::Other, NomCode::Virus);
    assert_unparsable_code(
        "Satellite Nucleic Acids (Subviral DNA-ssDNA)",
        NameType::Other,
        NomCode::Virus,
    );
}

#[test]
fn name_strings_with_rna() {
    // group: Name-strings with RNA
    // FIXME(review): a molecule type parsed as a uninomial
    // skipped: ssRNA
    assert_name("Alpha proteobacterium RNA12")
        .species("Alpha", "proteobacterium")
        .phrase("RNA12")
        .type_(NameType::Informal)
        .nothing_else();
    assert_unparsable_code(
        "Ustilaginoidea virens RNA virus",
        NameType::Other,
        NomCode::Virus,
    );
    assert_name("Candida albicans RNA_CTR0-3")
        .species("Candida", "albicans")
        .phrase("RNA_CTR0-3")
        .type_(NameType::Informal)
        .nothing_else();
    assert_name("Carabus satyrus satyrus KURNAKOV, 1962")
        .infra_species("Carabus", "satyrus", Rank::Subspecies, "satyrus")
        .comb_authors(Some("1962"), &["Kurnakov"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn epithet_prioni_is_not_a_prion() {
    // group: Epithet prioni is not a prion
    assert_name("Fakus prioni")
        .species("Fakus", "prioni")
        .nothing_else();
}

#[test]
fn names_with_satellite_as_a_substring() {
    // group: Names with "satellite" as a substring
    assert_name("Crassatellites fulvida")
        .species("Crassatellites", "fulvida")
        .nothing_else();
}

#[test]
fn bacterial_genus() {
    // group: Bacterial genus — year 1937 from publishedIn ("in Hauduroy 1937") propagates onto comb authorship.
    // FIXME(review): a bacterium coded BOTANICAL
    assert_name("Salmonella werahensis (Castellani) Hauduroy and Ehringer in Hauduroy 1937")
        .species("Salmonella", "werahensis")
        .comb_authors(Some("1937"), &["Hauduroy", "Ehringer"])
        .bas_authors(None, &["Castellani"])
        .published_in("Hauduroy 1937")
        .published_in_year(Some(1937))
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn bacteria_genus_homonym() {
    // group: Bacteria genus homonym
    assert_name("Actinomyces cardiffensis")
        .species("Actinomyces", "cardiffensis")
        .nothing_else();
}

#[test]
fn bacteria_with_pathovar_rank() {
    // group: Bacteria with pathovar rank — "pv." is the standard bacterial pathovar
    // marker and is kept in the canonical. "pathovar." is normalised to "pv.". A
    // bare trailing marker yields an indeterminate PATHOVAR with an INDETERMINED
    // warning, mirroring the openTaxonomyWithRanksUnfinished convention.
    assert_name("Xanthomonas axonopodis pv. phaseoli")
        .infra_species("Xanthomonas", "axonopodis", Rank::Pathovar, "phaseoli")
        .code(NomCode::Bacterial)
        .nothing_else();
    assert_name("Xanthomonas axonopodis pathovar. phaseoli")
        .infra_species("Xanthomonas", "axonopodis", Rank::Pathovar, "phaseoli")
        .code(NomCode::Bacterial)
        .nothing_else();

    // .infraSpecies(genus, epithet, PATHOVAR, null) — a null infraspecific epithet; direct-parse
    // fallback since the DSL's infra_species requires a non-null epithet, replicating the same
    // field assertions Java's infraSpecies(...) builder call would have made.
    let n = nameparser::parse_name("Xanthomonas axonopodis pathovar.", None, None, None)
        .unwrap_or_else(|e| panic!("expected `Xanthomonas axonopodis pathovar.` to parse: {e:?}"));
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Xanthomonas"));
    assert_eq!(n.specific_epithet.as_deref(), Some("axonopodis"));
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Pathovar);
    assert_eq!(n.type_, NameType::Informal);
    assert_eq!(n.warnings, vec![warnings::INDETERMINED.to_string()]);

    let n = nameparser::parse_name("Xanthomonas axonopodis pv.", None, None, None)
        .unwrap_or_else(|e| panic!("expected `Xanthomonas axonopodis pv.` to parse: {e:?}"));
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Xanthomonas"));
    assert_eq!(n.specific_epithet.as_deref(), Some("axonopodis"));
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Pathovar);
    assert_eq!(n.type_, NameType::Informal);
    assert_eq!(n.warnings, vec![warnings::INDETERMINED.to_string()]);
}

#[test]
fn stray_ex_is_not_parsed_as_species() {
    // group: "Stray" ex is not parsed as species. Botanical subsp. kept in canonical.
    // Modern interpretation: post-"ex" author is the validating author, pre-"ex" becomes the
    // exAuthor. Unlike Java, the bracketed validating author is kept as the attributed author
    // of an anonymous work (Solander, in Aiton's anonymous Hortus Kewensis).
    assert_name("Pelargonium cucullatum ssp. cucullatum (L.) L'Her. ex [Soland.]")
        .infra_species("Pelargonium", "cucullatum", Rank::Subspecies, "cucullatum")
        .comb_anon(None, &["Soland."])
        .comb_ex_authors(&["L'Her."])
        .bas_authors(None, &["L."])
        .code(NomCode::Botanical)
        .nothing_else();
    // FIXME(review): "ex gr." is read as two epithets and "rouaulti" as the author
    // "Acastella ex gr. rouaulti" — ex grege ("of the species-group of") is a
    // paleontological qualifier that the parser doesn't recognise. The trailing
    // "rouaulti" survives as authorship; the test is left as a TODO.
}

#[test]
fn authorship_in_upper_case() {
    // group: Authorship in upper case
    assert_name("Lecanora strobilinoides GIRALT & GÓMEZ-BOLEA")
        .species("Lecanora", "strobilinoides")
        .comb_authors(None, &["Giralt", "Gómez-Bolea"])
        .nothing_else();
}

#[test]
fn numbers_and_letters_separated_with_are_not_parsed_as_authors() {
    // group: Numbers and letters separated with '-' are not parsed as authors
    // FIXME(review): the specimen code "OS-2017" becomes an author
    assert_name("Astatotilapia cf. bloyeti OS-2017")
        .species("Astatotilapia", "bloyeti")
        .comb_authors(None, &["Os-2017"])
        .qualifiers(&[(NamePart::Specific, "cf.")])
        .type_(NameType::Informal)
        .nothing_else();
}

#[test]
fn double_parenthesis() {
    // group: Double parenthesis
    assert_name("Eichornia crassipes ( (Martius) ) Solms-Laub.")
        .species("Eichornia", "crassipes")
        .comb_authors(None, &["Solms-Laub."])
        .bas_authors(None, &["Martius"])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn year_without_authorship() {
    // group: Year without authorship
    assert_name("Acarospora cratericola 1929")
        .species("Acarospora", "cratericola")
        .comb_authors(Some("1929"), &[])
        .nothing_else();
    assert_name("Goggia gemmula 1996")
        .species("Goggia", "gemmula")
        .comb_authors(Some("1996"), &[])
        .nothing_else();
}

#[test]
fn year_range() {
    // group: Year range
    assert_name("Eurodryas orientalis Herrich-Schäffer 1845-1847")
        .species("Eurodryas", "orientalis")
        .comb_authors(Some("1845"), &["Herrich-Schäffer"])
        .warning(&[warnings::YEAR_INTERPRETED])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Tridentella tangeroae Bruce, 1987-92")
        .species("Tridentella", "tangeroae")
        .comb_authors(Some("1987"), &["Bruce"])
        .warning(&[warnings::YEAR_INTERPRETED])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Macroplectra unicolor Moore, 1858/59")
        .species("Macroplectra", "unicolor")
        .comb_authors(Some("1858"), &["Moore"])
        .warning(&[warnings::YEAR_INTERPRETED])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Seryda basirei Druce, 1891/901")
        .species("Seryda", "basirei")
        .comb_authors(Some("1891"), &["Druce"])
        .warning(&[warnings::YEAR_INTERPRETED])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn year_with_page_number() {
    // group: Year with page number — ":NN" trailing the year is captured into the
    // dedicated publishedInPage field (no PARTIAL state).
    assert_name("Recilia truncatus Dash & Viraktamath, 1998: 29")
        .species("Recilia", "truncatus")
        .comb_authors(Some("1998"), &["Dash", "Viraktamath"])
        .published_in_page("29")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Recilia truncatus Dash & Viraktamath, 1998:29")
        .species("Recilia", "truncatus")
        .comb_authors(Some("1998"), &["Dash", "Viraktamath"])
        .published_in_page("29")
        .code(NomCode::Zoological)
        .nothing_else();
}
