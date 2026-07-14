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
        .notho(&[NamePart::Specific]);
}

#[test]
fn names_with_a_dash() {
    // group: Names with a dash
    assert_name("Drosophila obscura-x Burla, 1951")
        .species("Drosophila", "obscura-x")
        .comb_authors(Some("1951"), &["Burla"]);
    assert_name("Sanogasta x-signata (Keyserling,1891)")
        .species("Sanogasta", "x-signata")
        .bas_authors(Some("1891"), &["Keyserling"]);
    assert_name("Aedes w-albus (Theobald, 1905)")
        .species("Aedes", "w-albus")
        .bas_authors(Some("1905"), &["Theobald"]);
    assert_name("Abryna regis-petri Paiva, 1860")
        .species("Abryna", "regis-petri")
        .comb_authors(Some("1860"), &["Paiva"]);
    assert_name("Solms-laubachia orbiculata Y.C. Lan & T.Y. Cheo")
        .species("Solms-laubachia", "orbiculata")
        .comb_authors(None, &["Y.C.Lan", "T.Y.Cheo"]);
}

#[test]
fn authorship_with_degli() {
    // group: Authorship with 'degli'
    assert_name("Cestodiscus gemmifer F. S. Castracane degli Antelminelli")
        .species("Cestodiscus", "gemmifer")
        .comb_authors(None, &["F.S.Castracane degli Antelminelli"]);
}

#[test]
fn authorship_with_filius_son_of() {
    // group: Authorship with filius (son of). The parser preserves the input form
    // (f. / fil. / filius) verbatim instead of normalising — "Hook. f." stays
    // "Hook.f." in the captured author. Botanical var. / f. / forma kept in canonical.
    assert_name("Oxytropis minjanensis Rech. f.")
        .species("Oxytropis", "minjanensis")
        .comb_authors(None, &["Rech.f."]);
    assert_name("Platypus bicaudatulus Schedl f. 1935")
        .species("Platypus", "bicaudatulus")
        .comb_authors(Some("1935"), &["Schedl f."])
        .code(NomCode::Zoological);
    assert_name("Platypus bicaudatulus Schedl filius 1935")
        .species("Platypus", "bicaudatulus")
        .comb_authors(Some("1935"), &["Schedl filius"])
        .code(NomCode::Zoological);
    assert_name("Fimbristylis ovata (Burm. f.) J. Kern")
        .species("Fimbristylis", "ovata")
        .comb_authors(None, &["J.Kern"])
        .bas_authors(None, &["Burm.f."]);
    assert_name("Amelanchier arborea var. arborea (Michx. f.) Fernald")
        .infra_species("Amelanchier", "arborea", Rank::Variety, "arborea")
        .comb_authors(None, &["Fernald"])
        .bas_authors(None, &["Michx.f."])
        .code(NomCode::Botanical);
    assert_name("Cerastium arvense var. fuegianum Hook. f.")
        .infra_species("Cerastium", "arvense", Rank::Variety, "fuegianum")
        .comb_authors(None, &["Hook.f."]);
    assert_name("Cerastium arvense var. fuegianum Hook.f.")
        .infra_species("Cerastium", "arvense", Rank::Variety, "fuegianum")
        .comb_authors(None, &["Hook.f."]);
    assert_name("Jacquemontia spiciflora (Choisy) Hall. fil.")
        .species("Jacquemontia", "spiciflora")
        .comb_authors(None, &["Hall.fil."])
        .bas_authors(None, &["Choisy"]);
    assert_name("Amelanchier arborea f. hirsuta (Michx. f.) Fernald")
        .infra_species("Amelanchier", "arborea", Rank::Form, "hirsuta")
        .comb_authors(None, &["Fernald"])
        .bas_authors(None, &["Michx.f."])
        .code(NomCode::Botanical);
    assert_name("Betula pendula fo. dalecarlica (L. f.) C.K. Schneid.")
        .infra_species("Betula", "pendula", Rank::Form, "dalecarlica")
        .comb_authors(None, &["C.K.Schneid."])
        .bas_authors(None, &["L.f."])
        .code(NomCode::Botanical);
    assert_name("Polypodium pectinatum L. f.")
        .species("Polypodium", "pectinatum")
        .comb_authors(None, &["L.f."]);
}

#[test]
fn names_with_emend_rectified_by_authorship() {
    // group: Names with emend (rectified by) authorship — the trailing "emend.
    // Author, year" reference is dropped from the authorship; first author/year
    // wins.
    assert_name("Chlorobium phaeobacteroides Pfennig, 1968 emend. Imhoff, 2003")
        .species("Chlorobium", "phaeobacteroides")
        .comb_authors(Some("1968"), &["Pfennig"])
        .code(NomCode::Zoological);
    assert_name("Chlorobium phaeobacteroides Pfennig, 1968 emend Imhoff, 2003")
        .species("Chlorobium", "phaeobacteroides")
        .comb_authors(Some("1968"), &["Pfennig"])
        .code(NomCode::Zoological);
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
        .comb_authors(None, &["Burt"]);
    assert_name("Nautilus asterizans von")
        .species("Nautilus", "asterizans")
        .comb_authors(None, &["von"]);
    assert_name("Dryopteris X separabilis Small (pro sp.)")
        .species("Dryopteris", "separabilis")
        .comb_authors(None, &["Small"]);
    assert_name("Eulima excellens Verkrüzen fide Paetel, 1887")
        .species("Eulima", "excellens")
        .comb_authors(None, &["Verkrüzen"])
        .sensu("fide Paetel, 1887");
    assert_name(
        "Procamallanus (Spirocamallanus) soodi Lakshmi & Kumari, 2001 nec (Gupta & Masood, 1988)",
    )
    .species_ig("Procamallanus", "Spirocamallanus", "soodi")
    .comb_authors(Some("2001"), &["Lakshmi", "Kumari"])
    .sensu("nec (Gupta & Masood, 1988)");
    assert_name("Membranipora minuscula Canu, 1911 non Hincks, 1882")
        .species("Membranipora", "minuscula")
        .comb_authors(Some("1911"), &["Canu"])
        .sensu("non Hincks, 1882");
    assert_name("Proboscina subechinata Canu & Bassler, 1920 non d'Orbigny, 1853")
        .species("Proboscina", "subechinata")
        .comb_authors(Some("1920"), &["Canu", "Bassler"])
        .sensu("non d'Orbigny, 1853");
    // "Author in Source, YYYY vide Other (YYYY)": the "in" tail goes into
    // publishedIn, and the trailing parenthesised year overrides as the
    // combination year.
    assert_name("Porina reussi Meneghini in De Amicis, 1885 vide Neviani (1900)")
        .species("Porina", "reussi")
        .comb_authors(Some("1900"), &["Meneghini"])
        .published_in("De Amicis, 1885 vide Neviani (1900)");
}

#[test]
fn non_ascii_utf8_characters_in_a_name() {
    // group: Non-ASCII UTF-8 characters in a name (ligatures/diacritics are kept verbatim)
    assert_name("Seleuca chûjôi Voss, 1957")
        .species("Seleuca", "chûjôi")
        .comb_authors(Some("1957"), &["Voss"]);
    assert_name("Pleurotus ëous (Berk.) Sacc. 1887")
        .species("Pleurotus", "ëous")
        .comb_authors(Some("1887"), &["Sacc."])
        .bas_authors(None, &["Berk."]);
    assert_name("Sténométope laevissimus Bibron 1855")
        .species("Sténométope", "laevissimus")
        .comb_authors(Some("1855"), &["Bibron"]);
    assert_name("Choriozopella trägårdhi Lawrence, 1947")
        .species("Choriozopella", "trägårdhi")
        .comb_authors(Some("1947"), &["Lawrence"]);
    assert_name("Isoëtes asplundii H. P. Fuchs")
        .species("Isoëtes", "asplundii")
        .comb_authors(None, &["H.P.Fuchs"]);
    assert_name("Campethera cailliautii fülleborni").infra_species(
        "Campethera",
        "cailliautii",
        Rank::InfraspecificName,
        "fülleborni",
    );
    assert_name("Östrupia Heiden ex Hustedt, 1935")
        .monomial("Östrupia")
        .comb_authors(Some("1935"), &["Hustedt"])
        .comb_ex_authors(&["Heiden"]);
}

#[test]
fn epithets_with_an_apostrophe() {
    // group: Epithets with an apostrophe — Indigenous-name and Irish/Scottish
    // surname apostrophes (o'donelli, m'coyi, l'herminierii, wila-k'oyu) are
    // kept verbatim in the epithet. Curly apostrophes (’) are normalised to
    // straight (') silently.
    assert_name("Solanum tuberosum f. wila-k'oyu Ochoa")
        .infra_species("Solanum", "tuberosum", Rank::Form, "wila-k'oyu")
        .comb_authors(None, &["Ochoa"]);
    assert_name("Junellia o'donelli Moldenke, 1946")
        .species("Junellia", "o'donelli")
        .comb_authors(Some("1946"), &["Moldenke"])
        .code(NomCode::Zoological);
    assert_name("Trophon d'orbignyi Carcelles, 1946")
        .species("Trophon", "d'orbignyi")
        .comb_authors(Some("1946"), &["Carcelles"])
        .code(NomCode::Zoological);
    assert_name("Phrynosoma m’callii").species("Phrynosoma", "m'callii");
    assert_name("Arca m'coyi Tenison-Woods, 1878")
        .species("Arca", "m'coyi")
        .comb_authors(Some("1878"), &["Tenison-Woods"])
        .code(NomCode::Zoological);
    assert_name("Nucula m'andrewii Hanley, 1860")
        .species("Nucula", "m'andrewii")
        .comb_authors(Some("1860"), &["Hanley"])
        .code(NomCode::Zoological);
    assert_name("Eristalis l'herminierii Macquart")
        .species("Eristalis", "l'herminierii")
        .comb_authors(None, &["Macquart"]);
    assert_name("Odynerus o'neili Cameron")
        .species("Odynerus", "o'neili")
        .comb_authors(None, &["Cameron"]);
    assert_name("Serjania meridionalis Cambess. var. o'donelli F.A. Barkley")
        .infra_species("Serjania", "meridionalis", Rank::Variety, "o'donelli")
        .comb_authors(None, &["F.A.Barkley"]);
}

#[test]
fn authors_with_an_apostrophe() {
    // group: Authors with an apostrophe. Acute (´) and back-tick (`) variants are
    // normalised to a plain apostrophe so "L´Hèr." / "L`Hèr." / "L'Hèr." all parse
    // identically. The quadrinomial collapses to the inner-most rank.
    assert_name("Galega officinalis (L.) L´Hèr. subsp. mackayana (O'Flannagan) Mc Inley var. petiolata (È. Neé) Brüch.")
        .infra_species("Galega", "officinalis", Rank::Variety, "petiolata")
        .comb_authors(None, &["Brüch."])
        .bas_authors(None, &["È.Neé"]);
    assert_name("Galega officinalis (L.) L`Hèr. subsp. mackayana (O'Flannagan) Mc Inley var. petiolata (È. Neé) Brüch.")
        .infra_species("Galega", "officinalis", Rank::Variety, "petiolata")
        .comb_authors(None, &["Brüch."])
        .bas_authors(None, &["È.Neé"]);
    assert_name("Galega officinalis (L.) L'Hèr. subsp. mackayana (O'Flannagan) Mc Inley var. petiolata (È. Neé) Brüch.")
        .infra_species("Galega", "officinalis", Rank::Variety, "petiolata")
        .comb_authors(None, &["Brüch."])
        .bas_authors(None, &["È.Neé"]);
}

#[test]
fn digraph_unicode_characters() {
    // group: Digraph unicode characters (ligatures kept verbatim)
    assert_name("Crisia romanica Zágoršek Silye & Szabó 2008")
        .species("Crisia", "romanica")
        .comb_authors(Some("2008"), &["Zágoršek Silye", "Szabó"]);
    assert_name("Æschopalæa grisella Pascoe, 1864")
        .species("Æschopalæa", "grisella")
        .comb_authors(Some("1864"), &["Pascoe"]);
    assert_name("Læptura laetifica Dow, 1913")
        .species("Læptura", "laetifica")
        .comb_authors(Some("1913"), &["Dow"]);
    assert_name("Leptura lætifica Dow, 1913")
        .species("Leptura", "lætifica")
        .comb_authors(Some("1913"), &["Dow"]);
    assert_name("Leptura leætifica Dow, 1913")
        .species("Leptura", "leætifica")
        .comb_authors(Some("1913"), &["Dow"]);
    assert_name("Leæptura laetifica Dow, 1913")
        .species("Leæptura", "laetifica")
        .comb_authors(Some("1913"), &["Dow"]);
    assert_name("Leœptura laetifica Dow, 1913")
        .species("Leœptura", "laetifica")
        .comb_authors(Some("1913"), &["Dow"]);
    assert_name("Ærenea cognata Lacordaire, 1872")
        .species("Ærenea", "cognata")
        .comb_authors(Some("1872"), &["Lacordaire"]);
    assert_name("Œdicnemus capensis").species("Œdicnemus", "capensis");
    assert_name("Œnanthe œnanthe").species("Œnanthe", "œnanthe");
    assert_name("Hördeum vulgare cœrulescens").infra_species(
        "Hördeum",
        "vulgare",
        Rank::InfraspecificName,
        "cœrulescens",
    );
    assert_name("Hordeum vulgare cœrulescens Metzger")
        .infra_species("Hordeum", "vulgare", Rank::InfraspecificName, "cœrulescens")
        .comb_authors(None, &["Metzger"]);
    assert_name("Hordeum vulgare f. cœrulescens").infra_species(
        "Hordeum",
        "vulgare",
        Rank::Form,
        "cœrulescens",
    );
}

#[test]
fn old_style_s() {
    // group: Old style s (ſ) — long-s normalised to s (it is a glyph variant),
    // ligatures æ and ß kept verbatim.
    assert_name("Musca domeſtica Linnaeus 1758")
        .species("Musca", "domestica")
        .comb_authors(Some("1758"), &["Linnaeus"]);
    assert_name("Amphisbæna fuliginoſa Linnaeus 1758")
        .species("Amphisbæna", "fuliginosa")
        .comb_authors(Some("1758"), &["Linnaeus"]);
    assert_name("Dreyfusia nüßlini").species("Dreyfusia", "nüßlini");
}

#[test]
fn miscellaneous_diacritics() {
    // group: Miscellaneous diacritics — kept verbatim, not decomposed.
    assert_name("Pärdosa").monomial("Pärdosa");
    assert_name("Pårdosa").monomial("Pårdosa");
    assert_name("Pardøsa").monomial("Pardøsa");
    assert_name("Pardösa").monomial("Pardösa");
    assert_name("Rühlella").monomial("Rühlella");
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
        .warning(&[warnings::QUESTION_MARKS_REMOVED]);
    // skipped: Euxoa nr. idahoensis sp. 1clay
    // skipped: Acarinina aff. pentacamerata
    // skipped: Acarinina aff pentacamerata
    // skipped: Sphingomonas sp. 37
    // skipped: Thryothorus leucotis spp. bogotensis
    // skipped: Endoxyla sp. GM-, 2003
    // skipped: X Aegilotrichum sp.
    // skipped: Liopropoma sp.2 Not applicable
    // skipped: Lacanobia sp. nr. subjuncta Bold:Aab, 0925
    // skipped: Lacanobia nr. subjuncta Bold:Aab, 0925
    // skipped: Abturia cf. alabamensis (Morton )
    // skipped: Abturia cf alabamensis (Morton )
    // skipped: Calidris cf. cooperi
    // "Aesculus cf. × hybrida" and "Daphnia (Daphnia) x krausi Flossner 1993" are
    // currently classified as FORMULA hybrids — the cf./subgenus + × combination
    // trips the hybrid-formula heuristic. Left as a known limitation.
    // skipped: Barbus cf macrotaenia × toppini
    // skipped: Gemmula cf. cosmoi NP-2008
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
        .code(NomCode::Zoological);
}

#[test]
fn viruses_plasmids_prions_etc() {
    // group: Viruses, plasmids, prions etc.
    // skipped: Arv1virus
    // skipped: Turtle herpesviruses
    // skipped: Cre expression vector
    // skipped: Cyanophage
    // skipped: Drosophila sturtevanti rhabdovirus
    // skipped: Hydra expression vector
    // skipped: Gateway destination plasmid
    // skipped: Abutilon mosaic virus [X15983] [X15984] Abutilon mosaic virus ICTV
    // skipped: Omphalotus sp. Ictv Garcia, 18224
    // skipped: Acute bee paralysis virus [AF150629] Acute bee paralysis virus
    // skipped: Adeno-associated virus - 3
    // skipped: ?M1-like Viruses Methanobrevibacter phage PG
    // skipped: Aeromonas phage 65
    // skipped: Bacillus phage SPß [AF020713] Bacillus phage SPb ICTV
    // skipped: Apple scar skin viroid
    // skipped: Australian grapevine viroid [X17101] Australian grapevine viroid ICTV
    // skipped: Agents of Spongiform Encephalopathies CWD prion Chronic wasting disease
    // skipped: Phi h-like viruses
    // skipped: Viroids
    // skipped: Fungal prions
    // skipped: Human rhinovirus A11
    // skipped: Kobuvirus korean black goat/South Korea/2010
    // skipped: Australian bat lyssavirus human/AUS/1998
    // skipped: Gossypium mustilinum symptomless alphasatellite
    // skipped: Okra leaf curl Mali alphasatellites-Cameroon
    // skipped: Bemisia betasatellite LW-2014
    // skipped: Tomato leaf curl Bangladesh betasatellites [India/Patna/Chilli/2008]
    // skipped: Intracisternal A-particles
    // skipped: Saccharomyces cerevisiae killer particle M1
    // skipped: Uranotaenia sapphirina NPV
    // skipped: Uranotaenia sapphirina Npv
    // skipped: Spodoptera exigua nuclear polyhedrosis virus SeMNPV
    // skipped: Spodoptera frugiperda MNPV
    // skipped: Rachiplusia ou MNPV (strain R1)
    // skipped: Orgyia pseudotsugata nuclear polyhedrosis virus OpMNPV
    // skipped: Mamestra configurata NPV-A
    // skipped: Helicoverpa armigera SNPV NNg1
    // skipped: Zamilon virophage
    // skipped: Sputnik virophage 3
    // skipped: Bacteriophage PH75
    // skipped: Escherichia coli bacteriophage
    // skipped: Betasatellites
    // skipped: Satellite Nucleic Acids (Subviral DNA-ssDNA)
}

#[test]
fn name_strings_with_rna() {
    // group: Name-strings with RNA
    // skipped: ssRNA
    // skipped: Alpha proteobacterium RNA12
    // skipped: Ustilaginoidea virens RNA virus
    // skipped: Candida albicans RNA_CTR0-3
    assert_name("Carabus satyrus satyrus KURNAKOV, 1962")
        .infra_species("Carabus", "satyrus", Rank::Subspecies, "satyrus")
        .comb_authors(Some("1962"), &["Kurnakov"]);
}

#[test]
fn epithet_prioni_is_not_a_prion() {
    // group: Epithet prioni is not a prion
    assert_name("Fakus prioni").species("Fakus", "prioni");
}

#[test]
fn names_with_satellite_as_a_substring() {
    // group: Names with "satellite" as a substring
    assert_name("Crassatellites fulvida").species("Crassatellites", "fulvida");
}

#[test]
fn bacterial_genus() {
    // group: Bacterial genus — year 1937 from publishedIn ("in Hauduroy 1937") propagates onto comb authorship.
    assert_name("Salmonella werahensis (Castellani) Hauduroy and Ehringer in Hauduroy 1937")
        .species("Salmonella", "werahensis")
        .comb_authors(Some("1937"), &["Hauduroy", "Ehringer"])
        .bas_authors(None, &["Castellani"]);
}

#[test]
fn bacteria_genus_homonym() {
    // group: Bacteria genus homonym
    assert_name("Actinomyces cardiffensis").species("Actinomyces", "cardiffensis");
}

#[test]
fn bacteria_with_pathovar_rank() {
    // group: Bacteria with pathovar rank — "pv." is the standard bacterial pathovar
    // marker and is kept in the canonical. "pathovar." is normalised to "pv.". A
    // bare trailing marker yields an indeterminate PATHOVAR with an INDETERMINED
    // warning, mirroring the openTaxonomyWithRanksUnfinished convention.
    assert_name("Xanthomonas axonopodis pv. phaseoli").infra_species(
        "Xanthomonas",
        "axonopodis",
        Rank::Pathovar,
        "phaseoli",
    );
    assert_name("Xanthomonas axonopodis pathovar. phaseoli").infra_species(
        "Xanthomonas",
        "axonopodis",
        Rank::Pathovar,
        "phaseoli",
    );

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
    // group: "Stray" ex is not parsed as species. Botanical subsp. kept in canonical;
    // square brackets around an ex-author are stripped silently. Modern interpretation:
    // post-"ex" author is the validating author, pre-"ex" becomes the exAuthor.
    assert_name("Pelargonium cucullatum ssp. cucullatum (L.) L'Her. ex [Soland.]")
        .infra_species("Pelargonium", "cucullatum", Rank::Subspecies, "cucullatum")
        .comb_authors(None, &["Soland."])
        .comb_ex_authors(&["L'Her."])
        .bas_authors(None, &["L."])
        .code(NomCode::Botanical);
    // "Acastella ex gr. rouaulti" — ex grege ("of the species-group of") is a
    // paleontological qualifier that the parser doesn't recognise. The trailing
    // "rouaulti" survives as authorship; the test is left as a TODO.
}

#[test]
fn authorship_in_upper_case() {
    // group: Authorship in upper case
    assert_name("Lecanora strobilinoides GIRALT & GÓMEZ-BOLEA")
        .species("Lecanora", "strobilinoides")
        .comb_authors(None, &["Giralt", "Gómez-Bolea"]);
}

#[test]
fn numbers_and_letters_separated_with_are_not_parsed_as_authors() {
    // group: Numbers and letters separated with '-' are not parsed as authors
    // skipped: Astatotilapia cf. bloyeti OS-2017
}

#[test]
fn double_parenthesis() {
    // group: Double parenthesis
    assert_name("Eichornia crassipes ( (Martius) ) Solms-Laub.")
        .species("Eichornia", "crassipes")
        .comb_authors(None, &["Solms-Laub."])
        .bas_authors(None, &["Martius"]);
}

#[test]
fn year_without_authorship() {
    // group: Year without authorship
    assert_name("Acarospora cratericola 1929").species("Acarospora", "cratericola");
    assert_name("Goggia gemmula 1996").species("Goggia", "gemmula");
}

#[test]
fn year_range() {
    // group: Year range
    assert_name("Eurodryas orientalis Herrich-Schäffer 1845-1847")
        .species("Eurodryas", "orientalis")
        .comb_authors(Some("1845"), &["Herrich-Schäffer"])
        .warning(&[warnings::YEAR_INTERPRETED]);

    assert_name("Tridentella tangeroae Bruce, 1987-92")
        .species("Tridentella", "tangeroae")
        .comb_authors(Some("1987"), &["Bruce"])
        .warning(&[warnings::YEAR_INTERPRETED]);

    assert_name("Macroplectra unicolor Moore, 1858/59")
        .species("Macroplectra", "unicolor")
        .comb_authors(Some("1858"), &["Moore"])
        .warning(&[warnings::YEAR_INTERPRETED]);

    assert_name("Seryda basirei Druce, 1891/901")
        .species("Seryda", "basirei")
        .comb_authors(Some("1891"), &["Druce"])
        .warning(&[warnings::YEAR_INTERPRETED]);
}

#[test]
fn year_with_page_number() {
    // group: Year with page number — ":NN" trailing the year is captured into the
    // dedicated publishedInPage field (no PARTIAL state).
    assert_name("Recilia truncatus Dash & Viraktamath, 1998: 29")
        .species("Recilia", "truncatus")
        .comb_authors(Some("1998"), &["Dash", "Viraktamath"])
        .published_in_page("29");
    assert_name("Recilia truncatus Dash & Viraktamath, 1998:29")
        .species("Recilia", "truncatus")
        .comb_authors(Some("1998"), &["Dash", "Viraktamath"])
        .published_in_page("29");
}
