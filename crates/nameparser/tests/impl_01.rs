// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserImplTest (methods on lines 37-384).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NameType, NomCode, Rank};

/// https://github.com/gbif/name-parser/issues/33
/// https://github.com/gbif/name-parser/issues/66
/// https://www.ncbi.nlm.nih.gov/books/NBK8808/#A431
#[test]
fn sic() {
    assert_name("Ameiva plei Rosicky, 1955")
        .species("Ameiva", "plei")
        .comb_authors(Some("1955"), &["Rosicky"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Ameiva plei (sic) Duméril & Bibron, 1839")
        .species("Ameiva", "plei")
        .comb_authors(Some("1839"), &["Duméril", "Bibron"])
        .sic()
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("†Amnicola kushixiaensis [sic]")
        .species("Amnicola", "kushixiaensis")
        .extinct()
        .sic()
        .nothing_else();

    assert_name("†Amnicola (Amnicola) dubrueilliana [sic]")
        .species_ig("Amnicola", "Amnicola", "dubrueilliana")
        .extinct()
        .sic()
        .nothing_else();

    assert_name("Anabathron (Scrobs) elongatus [sic]")
        .species_ig("Anabathron", "Scrobs", "elongatus")
        .sic()
        .nothing_else();

    assert_name("Scaphander lignarius var. brittanica [sic]")
        .infra_species("Scaphander", "lignarius", Rank::Variety, "brittanica")
        .sic()
        .nothing_else();

    assert_name("†Tulotoma bifarcinata var. contiqua [sic]")
        .infra_species("Tulotoma", "bifarcinata", Rank::Variety, "contiqua")
        .extinct()
        .sic()
        .nothing_else();

    assert_name("Scincus homolocephalus (sic) Wiegmann, 1828")
        .species("Scincus", "homolocephalus")
        .comb_authors(Some("1828"), &["Wiegmann"])
        .sic()
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Cochlostyla (Dryocochlias) satyrus palawanensis [sic]")
        .infra_species(
            "Cochlostyla",
            "satyrus",
            Rank::InfraspecificName,
            "palawanensis",
        )
        .infrageneric("Dryocochlias")
        .sic()
        .nothing_else();

    assert_name("Clionella sinuata borni [sic]")
        .infra_species("Clionella", "sinuata", Rank::InfraspecificName, "borni")
        .sic()
        .nothing_else();

    assert_name("†Melanopsis (Melanopsis) pterochyla pterochyla [sic]")
        .infra_species(
            "Melanopsis",
            "pterochyla",
            Rank::InfraspecificName,
            "pterochyla",
        )
        .infrageneric("Melanopsis")
        .extinct()
        .sic()
        .nothing_else();

    assert_name("Melanella hollandri [sic] var. detrita Kucik")
        .infra_species("Melanella", "hollandri", Rank::Variety, "detrita")
        .comb_authors(None, &["Kucik"])
        .sic()
        .nothing_else();

    assert_name("Alnetoidia (Alnella) [sic] sudzhuchenica Sohi, 1998")
        .species_ig("Alnetoidia", "Alnella", "sudzhuchenica")
        .comb_authors(Some("1998"), &["Sohi"])
        .sic()
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Flavobacterium branchiophila (sic) Wakabayashi et al. 1989")
        .species("Flavobacterium", "branchiophila")
        .comb_authors(Some("1989"), &["Wakabayashi", "al."])
        .sic()
        .code(NomCode::Zoological)
        .nothing_else();

    // https://lpsn.dsmz.de/text/glossary#corrigendum
    assert_name("Campylobacter lari corrig. Benjamin et al. 1984")
        .species("Campylobacter", "lari")
        .comb_authors(Some("1984"), &["Benjamin", "al."])
        .corrig()
        .code(NomCode::Zoological)
        .nothing_else();

    // parenthesised corrig., like (sic) above
    assert_name("Campylobacter lari (corrig.) Benjamin et al. 1984")
        .species("Campylobacter", "lari")
        .comb_authors(Some("1984"), &["Benjamin", "al."])
        .corrig()
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Campylobacter laridis (sic) Benjamin et al. 1984")
        .species("Campylobacter", "laridis")
        .comb_authors(Some("1984"), &["Benjamin", "al."])
        .sic()
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Firmicutes corrig. Gibbons & Murray, 1978")
        .monomial("Firmicutes")
        .comb_authors(Some("1978"), &["Gibbons", "Murray"])
        .corrig()
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Barleeidae [sic!]")
        .monomial("Barleeidae")
        .sic()
        .nothing_else();

    assert_name("†Sainshandiidae [sic]")
        .monomial("Sainshandiidae")
        .extinct()
        .sic()
        .nothing_else();

    assert_name("Turbo porphyrites [sic, porphyria]")
        .species("Turbo", "porphyrites")
        .partial("(sic,porphyria)") // not ideal , but hey
        .sic()
        .nothing_else();
}

#[test]
fn digit_epithets_leading_numeral() {
    assert_name("Coccinella 11-punctata Linnaeus, 1758")
        .species("Coccinella", "11-punctata")
        .comb_authors(Some("1758"), &["Linnaeus"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Coccinella 2-pustulata")
        .species("Coccinella", "2-pustulata")
        .nothing_else();
}

#[test]
fn square_genera() {
    assert_name("[Acontia] chia Holland, 1894")
        .species("Acontia", "chia")
        .comb_authors(Some("1894"), &["Holland"])
        .code(NomCode::Zoological)
        .doubtful()
        .warning(&[warnings::DOUBTFUL_GENUS])
        .nothing_else();

    assert_name("[Dexia]")
        .monomial("Dexia")
        .doubtful()
        .warning(&[warnings::DOUBTFUL_GENUS])
        .nothing_else();

    assert_name("[Diomea] orbicularis Walker, 1858")
        .species("Diomea", "orbicularis")
        .comb_authors(Some("1858"), &["Walker"])
        .code(NomCode::Zoological)
        .doubtful()
        .warning(&[warnings::DOUBTFUL_GENUS])
        .nothing_else();

    // bracketed genus with a parenthesised basionym — brackets stripped, name kept SCIENTIFIC
    // but flagged doubtful because of them.
    assert_name("[Ablabesmyia] aurea (Johannsen, 1907)")
        .species("Ablabesmyia", "aurea")
        .bas_authors(Some("1907"), &["Johannsen"])
        .code(NomCode::Zoological)
        .doubtful()
        .warning(&[warnings::DOUBTFUL_GENUS])
        .nothing_else();
}

#[test]
fn tinfr() {
    assert_name("Hieracium vulgatum t.infr. arrectariicaule Sudre")
        .infra_species(
            "Hieracium",
            "vulgatum",
            Rank::InfraspecificName,
            "arrectariicaule",
        )
        .comb_authors(None, &["Sudre"])
        .nothing_else();
}

/// https://github.com/gbif/name-parser/issues/82
#[test]
fn error() {
    assert_name("Agama annectans Blanford, 1870 [orth. error]")
        .species("Agama", "annectans")
        .comb_authors(Some("1870"), &["Blanford"])
        .nom_note("orth. error")
        .code(NomCode::Zoological)
        .nothing_else();
}

/// https://github.com/gbif/name-parser/issues/80
#[test]
fn ck_yang() {
    assert_name("Nemopistha sinica C.-k. Yang, 1986")
        .species("Nemopistha", "sinica")
        .comb_authors(Some("1986"), &["C.-k.Yang"])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// https://github.com/gbif/name-parser/issues/75
#[test]
fn oosterveldii() {
    assert_name("Taraxacum piet-oosterveldii H. Øllg. in press")
        .species("Taraxacum", "piet-oosterveldii")
        .comb_authors(None, &["H.Øllg."])
        .nom_note("in press")
        .manuscript()
        .nothing_else();
}

/// https://github.com/gbif/name-parser/issues/72
#[test]
fn zur_strassen() {
    assert_name("Jezzinothrips cretacicus zur Strassen, 1973")
        .species("Jezzinothrips", "cretacicus")
        .comb_authors(Some("1973"), &["zur Strassen"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Jezzinothrips cretacicus amazur Strassen, 1973")
        .infra_species("Jezzinothrips", "cretacicus", Rank::Subspecies, "amazur")
        .comb_authors(Some("1973"), &["Strassen"])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// https://github.com/gbif/name-parser/issues/70
#[test]
fn nom_superfl() {
    assert_name("Agrostis compressa Willd., nom. superfl.")
        .species("Agrostis", "compressa")
        .comb_authors(None, &["Willd."])
        .nom_note("nom. superfl.")
        .code(NomCode::Botanical)
        .nothing_else();
}

/// https://github.com/gbif/name-parser/issues/68
#[test]
fn clade_names() {
    // keyword clade to throw unparsable
    assert_unparsable("Amauropeltoid clade", NameType::Informal);
    // Java also passes an explicit `Rank.UNRANKED` hint here — the same default the 2-arg
    // `assertUnparsable`/`assert_unparsable` overload already parses with, so the two calls
    // are behaviourally identical.
    assert_unparsable("Cyanobacteriota/Melainabacteria clade", NameType::Informal);
    // no clades
    assert_name("Endococcus cladiae Zhurb. & Pino-Bodas")
        .species("Endococcus", "cladiae")
        .comb_authors(None, &["Zhurb.", "Pino-Bodas"])
        .nothing_else();

    assert_name("Clada tricostata clada (Clada) Pascoe, 1887")
        .infra_species("Clada", "tricostata", Rank::InfraspecificName, "clada")
        .comb_authors(Some("1887"), &["Pascoe"])
        .bas_authors(None, &["Clada"])
        .nothing_else();
}

/// https://github.com/gbif/name-parser/issues/67
#[test]
fn double_hyphen_epithet() {
    assert_name("Grammitis friderici-et-pauli (Christ) Copel.")
        .species("Grammitis", "friderici-et-pauli")
        .comb_authors(None, &["Copel."])
        .bas_authors(None, &["Christ"])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn nom_cons() {
    assert_name("Polygala vulgaris L., 1753 [nom. et typ. cons.]")
        .species("Polygala", "vulgaris")
        .comb_authors(Some("1753"), &["L."])
        .nom_note("nom. & typ. cons.")
        .code(NomCode::Botanical)
        .nothing_else();
}

/// https://github.com/gbif/name-parser/issues/62
#[test]
fn des_loges() {
    assert_name("Desbrochers des Loges, 1881")
        .monomial("Desbrochers")
        .comb_authors(Some("1881"), &["des Loges"])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// https://github.com/gbif/name-parser/issues/60
#[test]
fn subg() {
    assert_name("Centaurea subg. Jacea")
        .infrageneric_at("Centaurea", Rank::Subgenus, "Jacea")
        .nothing_else();

    // Author placed before the infrageneric marker is captured as the genus (generic) author.
    assert_name("Centaurea L. subg. Jacea")
        .infrageneric_at("Centaurea", Rank::Subgenus, "Jacea")
        .generic_authors(None, &["L."])
        .nothing_else();

    // not a series: https://github.com/gbif/checklistbank/issues/200
    assert_name("Mergus merganser Linnaeus, 1758")
        .species("Mergus", "merganser")
        .comb_authors(Some("1758"), &["Linnaeus"])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// https://github.com/gbif/name-parser/issues/59
#[test]
fn von_den() {
    assert_name("Gyalidea minuta van den Boom & Vezda")
        .species("Gyalidea", "minuta")
        .comb_authors(None, &["van den Boom", "Vezda"])
        .nothing_else();
}
