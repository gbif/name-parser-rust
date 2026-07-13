// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserImplTest (methods on lines 3494-3991).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NameType, NomCode, Rank};

#[test]
fn botanical_code_from_separate_recombination_authorship() {
    // A separately supplied botanical recombination authorship "(Basionym) Combination" (no year)
    // must infer BOTANICAL, just like the same authorship embedded in the name string does.
    assert_name_hinted(
        "Cerastium ligusticum subsp. granulatum",
        Some("(Huter et al.) P. D. Sell & Whitehead"),
        Some(Rank::Subspecies),
        None,
    )
    .infra_species("Cerastium", "ligusticum", Rank::Subspecies, "granulatum")
    .bas_authors(None, &["Huter", "al."])
    .comb_authors(None, &["P.D.Sell", "Whitehead"])
    .code(NomCode::Botanical)
    .nothing_else();
}

#[test]
fn standalone_manuscript_authorship() {
    // a standalone "ined." / "ms." supplied as the whole authorship is a manuscript marker,
    // not an author (whereas "Author ms." glues the suffix onto the author).
    assert_name_hinted("Eucnidoideae", Some("ined."), Some(Rank::Superfamily), None)
        .monomial_rank("Eucnidoideae", Rank::Superfamily)
        .manuscript()
        .nothing_else();
    assert_name_hinted("Eucnidoideae", Some("ms."), Some(Rank::Superfamily), None)
        .monomial_rank("Eucnidoideae", Rank::Superfamily)
        .manuscript()
        .nothing_else();
}

#[test]
fn virus_false_positive_animals() {
    assert_name_hinted(
        "Aspilota vector",
        Some("Belokobylskij, 2007"),
        Some(Rank::Species),
        Some(NomCode::Zoological),
    )
    .species("Aspilota", "vector")
    .comb_authors(Some("2007"), &["Belokobylskij"])
    .code(NomCode::Zoological)
    .nothing_else();

    assert_name("Euragallia prion")
        .species("Euragallia", "prion")
        .nothing_else();

    assert_name_hinted(
        "Cryptops (Cryptops) vector",
        Some("Chamberlin, 1939"),
        Some(Rank::Species),
        Some(NomCode::Zoological),
    )
    .species_ig("Cryptops", "Cryptops", "vector")
    .comb_authors(Some("1939"), &["Chamberlin"])
    .code(NomCode::Zoological)
    .nothing_else();

    assert_name("Prion").monomial("Prion").nothing_else();

    assert_name_hinted(
        "Exochus virus",
        Some("Gauld & Sithole, 2002"),
        Some(Rank::Species),
        Some(NomCode::Zoological),
    )
    .species("Exochus", "virus")
    .comb_authors(Some("2002"), &["Gauld", "Sithole"])
    .code(NomCode::Zoological)
    .nothing_else();

    assert_unparsable_code("Acara virus", NameType::Other, NomCode::Virus);

    // A soft virus-word (prion/vector/particle/replicon/rna) sitting as the leading Title-cased
    // GENUS is a real animal genus, not a virus — Prion (Lacépède, 1799) is a petrel genus. The
    // reject must only fire when a genuinely viral token is also present. Previously the binomial
    // and authored-monomial forms were wrongly thrown as OTHER + VIRUS (only bare "Prion" survived).
    assert_name("Prion vittatus")
        .species("Prion", "vittatus")
        .nothing_else();
    assert_name("Prion Lacépède, 1799")
        .monomial("Prion")
        .comb_authors(Some("1799"), &["Lacépède"])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// The Preflight `ZOOLOGICAL_BINOMIAL` regex used to be an overlapping-alternation ReDoS —
/// on an input that triggers the virus gate but has no trailing year it could backtrack
/// exponentially. The parser has no execution timeout, so this guards the hardened (possessive)
/// pattern: an adversarial string must be classified in linear time, not hang the thread.
///
/// Java's `@Test(timeout = 3000)` enforced a 3s-per-test wall-clock limit; Rust has no built-in
/// per-test timeout attribute, so this ports as a plain `#[test]` — a ReDoS regression here
/// would show up as a hang, not a timed failure.
#[test]
fn virus_gate_no_catastrophic_backtracking() {
    let adversarial = format!("Rnavirus bus {}", "Aa.-".repeat(30));
    match nameparser::parse(&adversarial, None, None, None) {
        Ok(_) => {}
        Err(_) => {
            // expected — the point is that the classification returns fast, not that it parses.
        }
    }
}

#[test]
fn virus_caller_code_override() {
    // caller asserts a non-virus code → bucket-A name parses under that code
    assert_name_code("Tobamovirus tabaci", NomCode::Zoological)
        .species("Tobamovirus", "tabaci")
        .code(NomCode::Zoological);
    // caller forces VIRUS on a legacy bare-virus binomial → unparsable OTHER + VIRUS
    assert_unparsable_code("Acara virus", NameType::Other, NomCode::Virus);
}

#[test]
fn apostrophe_epithets() {
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
        .nothing_else();
}

/// https://github.com/gbif/name-parser/issues/28
///
/// A roman-numeral generational suffix ("Loeblich III" = Loeblich the third) stays behind the
/// surname as an upper-case suffix — it must NOT be read as the initials "I.I.I." and flipped in
/// front of the surname. Both the all-caps and title-case input forms normalise to "III".
#[test]
fn generational_suffix() {
    assert_authorship("Loeblich III", &["Loeblich III"]);
    assert_authorship("Loeblich Iii", &["Loeblich III"]);
    assert_name("Ceratium hirundinella (Paulsen) Loeblich III, 1969")
        .species("Ceratium", "hirundinella")
        .bas_authors(None, &["Paulsen"])
        .comb_authors(Some("1969"), &["Loeblich III"])
        .nothing_else();
    assert_name("Ceratium hirundinella (Paulsen) Loeblich Iii, 1969")
        .species("Ceratium", "hirundinella")
        .bas_authors(None, &["Paulsen"])
        .comb_authors(Some("1969"), &["Loeblich III"])
        .nothing_else();
}

#[test]
fn initials_after_surname() {
    assert_name("Purana guttularis (Walker, F., 1858)")
        .species("Purana", "guttularis")
        .bas_authors(Some("1858"), &["F.Walker"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Physomerinus septemfoveolatus Schaufuss, L. W.")
        .species("Physomerinus", "septemfoveolatus")
        .comb_authors(None, &["L.W.Schaufuss"])
        .nothing_else();

    assert_name("Physomerinus septemfoveolatus Schaufuss, L. W., 1877")
        .species("Physomerinus", "septemfoveolatus")
        .comb_authors(Some("1877"), &["L.W.Schaufuss"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Euplectus cavicollis LeConte, J. L., 1878")
        .species("Euplectus", "cavicollis")
        .comb_authors(Some("1878"), &["J.L.LeConte"])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// The parser accepts name and authorship separately,
/// but the authorship can also be provided as part of the name - which happens a lot.
/// Make sure we can handle both cases.
#[test]
fn redundant_authorship() {
    assert_name_auth(
        "Euplectus cavicollis LeConte, J. L., 1878",
        "LeConte, J. L., 1878",
    )
    .species("Euplectus", "cavicollis")
    .comb_authors(Some("1878"), &["J.L.LeConte"])
    .code(NomCode::Zoological)
    .nothing_else();

    // Java calls this as `assertName(rawName, (String)null, canonical)` — the cast
    // disambiguates method overload resolution only (a bare `null` is ambiguous between the
    // String/Rank/NomCode 2nd-arg overloads); behaviourally identical to the no-authorship
    // 2-arg form used here.
    assert_name("Abies alba Mill.")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .nothing_else();

    assert_name_auth("Abies alba", "Mill.")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .nothing_else();

    assert_name_auth("Abies alba Mill.", "Mill.")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .nothing_else();

    assert_name_auth("Abies alba  Mill", "Mill.")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .nothing_else();

    assert_name_auth("Puma concolor (Linnaeus, 1771)", "(Linnaeus, 1771)")
        .species("Puma", "concolor")
        .bas_authors(Some("1771"), &["Linnaeus"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name_auth("Puma concolor", "(Linnaeus, 1771)")
        .species("Puma", "concolor")
        .bas_authors(Some("1771"), &["Linnaeus"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name_auth("Puma concolor ( Linnaeus, 1771 )", "(Linnaeus 1771)")
        .species("Puma", "concolor")
        .bas_authors(Some("1771"), &["Linnaeus"])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// https://github.com/gbif/name-parser/issues/45
#[test]
fn bold_placeholder() {
    // 5.0.0: BOLD/placeholder suprageneric-provisional names ("<Taxon><CODE>") are Informal
    // results — the supraspecific anchor + a placeholder phrase, no species epithet. (Java surfaced
    // them as Informal-typed ParsedNames; the three-way surfaces them as `Informal`.)
    assert_informal_hinted("OdontellidaeGEN", None, Some(Rank::Genus), None)
        .taxon("Odontellidae")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Genus)
        .phrase("GEN")
        .nothing_else();

    assert_informal_hinted("EusiridaeNZD", None, None, Some(NomCode::Zoological))
        .taxon("Eusiridae")
        .taxon_rank(Rank::Family)
        .rank(Rank::Family)
        .phrase("NZD")
        .code(NomCode::Zoological)
        .nothing_else();

    assert_informal("Blattellinae_SB")
        .taxon("Blattellinae")
        .taxon_rank(Rank::Unranked)
        .rank(Rank::Unranked)
        .phrase("SB")
        .nothing_else();

    assert_informal("GenusANIC_3")
        .taxon("Genus")
        .taxon_rank(Rank::Unranked)
        .rank(Rank::Unranked)
        .phrase("ANIC_3")
        .nothing_else();
}

/// http://dev.gbif.org/issues/browse/POR-3069
#[test]
fn null_name_parts() {
    assert_name("Austrorhynchus pectatus null pectatus")
        .infra_species(
            "Austrorhynchus",
            "pectatus",
            Rank::InfraspecificName,
            "pectatus",
        )
        .doubtful()
        .warning(&[warnings::NULL_EPITHET])
        .nothing_else();

    //assertName("Poa pratensis null proles (L.) Rouy, 1913", "Poa pratensis proles")
    //    .infraSpecies("Poa", "pratensis", Rank.PROLES, "proles")
    //    .basAuthors(null, "L.")
    //    .combAuthors("1913", "Rouy")
    //    .nothingElse();

    // should the infrasubspecific epithet kewensis be removed from the parsed name?
    //assertParsedParts("Poa pratensis kewensis proles", NameType.INFORMAL, "Poa", "pratensis", "kewensis", Rank.PROLES, null);
    //assertParsedParts("Poa pratensis kewensis proles (L.) Rouy, 1913", NameType.INFORMAL, "Poa", "pratensis", null, Rank.PROLES, "Rouy", "1913", "L.", null);
}

#[test]
fn r_na_names() {
    assert_name("Calathus (Lindrothius) KURNAKOV 1961")
        .infrageneric_at("Calathus", Rank::InfragenericName, "Lindrothius")
        .comb_authors(Some("1961"), &["Kurnakov"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert!(is_viral_name("Ustilaginoidea virens RNA virus"));
    assert!(is_viral_name("Rhizoctonia solani dsRNA virus 2"));

    assert_name("Candida albicans RNA_CTR0-3")
        .species("Candida", "albicans")
        .phrase("RNA_CTR0-3")
        .type_(NameType::Informal)
        .nothing_else();

    assert_name("Alpha proteobacterium RNA12")
        .species("Alpha", "proteobacterium")
        .phrase("RNA12")
        .type_(NameType::Informal)
        .nothing_else();

    assert_name("Armillaria ostoyae RNA1")
        .species("Armillaria", "ostoyae")
        .phrase("RNA1")
        .type_(NameType::Informal)
        .nothing_else();
}

#[test]
fn indet_names() {
    // `.species(genus, null)` — the DSL's species()/infra_species()/indet() helpers take the
    // epithet as a plain `&str` (no way to express "epithet explicitly absent"), so the fully
    // indeterminate ("spec."/"indet."/"sp.") cases below are asserted directly against the
    // parsed fields (same gap and pattern as `Aster indet.` in impl_03.rs) — only the fields
    // the Java `.species(genus, null).type(...).warning(...)` chain actually touches are
    // checked. The infraspecific-epithet-null cases further down instead reuse the
    // general-purpose public `binomial(genus, infrageneric, epithet, rank)`, which already
    // hardcodes infraspecific_epithet == None and takes an arbitrary rank, so it reproduces
    // Java's `.infraSpecies(genus, epithet, RANK, null)` exactly and can still finish with
    // `nothing_else()`.
    let n = nameparser::parse("Trametes spec.", None, None, None)
        .expect("`Trametes spec.` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Trametes"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Species);
    assert_eq!(n.type_, NameType::Informal);
    assert_eq!(n.warnings, vec![warnings::INDETERMINED.to_string()]);

    let n = nameparser::parse("Trametes indet.", None, None, None)
        .expect("`Trametes indet.` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Trametes"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Species);
    assert_eq!(n.type_, NameType::Informal);
    assert_eq!(n.warnings, vec![warnings::INDETERMINED.to_string()]);

    let n = nameparser::parse("Camillina indet", None, None, None)
        .expect("`Camillina indet` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Camillina"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Species);
    assert_eq!(n.type_, NameType::Informal);
    assert_eq!(n.warnings, vec![warnings::INDETERMINED.to_string()]);

    // Indeterminate infraspecific names keep the authorship trailing the rank marker.
    assert_name("Nitzschia sinuata var. (Grunow) Lange-Bert.")
        .binomial("Nitzschia", None, "sinuata", Rank::Variety)
        .bas_authors(None, &["Grunow"])
        .comb_authors(None, &["Lange-Bert."])
        .code(NomCode::Botanical)
        .type_(NameType::Informal)
        .warning(&[warnings::INDETERMINED])
        .nothing_else();

    assert_name("Canis lupus subsp. Linnaeus, 1758")
        .binomial("Canis", None, "lupus", Rank::Subspecies)
        .comb_authors(Some("1758"), &["Linnaeus"])
        .code(NomCode::Zoological)
        .type_(NameType::Informal)
        .warning(&[warnings::INDETERMINED])
        .nothing_else();

    //    assertName("Aphaenogaster (Ichnomyrmex) Schwammerdami var. spinipes", "Aphaenogaster var. spinipes")
    //        .infraSpecies("Aphaenogaster", null, Rank.VARIETY, "spinipes")
    //        .infraGeneric("Ichnomyrmex")
    //        .type(NameType.INFORMAL)
    //        .nothingElse();
    //
    //    assertName("Ocymyrmex Weitzaeckeri subsp. arnoldi", "Ocymyrmex subsp. arnoldi")
    //        .infraSpecies("Ocymyrmex", null, Rank.SUBSPECIES, "arnoldi")
    //        .type(NameType.INFORMAL)
    //        .nothingElse();
    //
    //    assertName("Navicula var. fasciata", "Navicula var. fasciata")
    //        .infraSpecies("Navicula", null, Rank.VARIETY, "fasciata")
    //        .type(NameType.INFORMAL)
    //        .nothingElse();

    let n = nameparser::parse("Polygonum spec.", None, None, None)
        .expect("`Polygonum spec.` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Polygonum"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Species);
    assert_eq!(n.type_, NameType::Informal);
    assert_eq!(n.warnings, vec![warnings::INDETERMINED.to_string()]);

    assert_name("Polygonum vulgaris ssp.")
        .binomial("Polygonum", None, "vulgaris", Rank::Subspecies)
        .type_(NameType::Informal)
        .warning(&[warnings::INDETERMINED])
        .nothing_else();

    let n = nameparser::parse("Mesocricetus sp.", None, None, None)
        .expect("`Mesocricetus sp.` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Mesocricetus"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Species);
    assert_eq!(n.type_, NameType::Informal);
    assert_eq!(n.warnings, vec![warnings::INDETERMINED.to_string()]);

    // dont treat these authorships as forms
    assert_name_code("Dioscoreales Hooker f.", NomCode::Botanical)
        .monomial_rank("Dioscoreales", Rank::Order)
        .comb_authors(None, &["Hooker f."])
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Melastoma vacillans Blume var.")
        .binomial("Melastoma", None, "vacillans", Rank::Variety)
        .type_(NameType::Informal)
        .warning(&[warnings::INDETERMINED])
        .nothing_else();

    let n = nameparser::parse("Lepidoptera Hooker", None, Some(Rank::Species), None)
        .expect("`Lepidoptera Hooker` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Lepidoptera"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Species);
    assert_eq!(n.type_, NameType::Informal);
    assert_eq!(n.warnings, vec![warnings::INDETERMINED.to_string()]);

    assert_name_rank("Lepidoptera alba DC.", Rank::Subspecies)
        .binomial("Lepidoptera", None, "alba", Rank::Subspecies)
        .comb_authors(None, &["DC."])
        .type_(NameType::Informal)
        .warning(&[warnings::INDETERMINED])
        .nothing_else();
}

#[test]
fn rank_mismatch() {
    // interpret as indetermined names if rank is below genus
    //
    // `.cultivar(genus, null)` — the DSL's cultivar()/cultivar_rank() take the cultivar
    // epithet as a plain `&str` (no way to express "cultivar epithet explicitly absent"), so
    // this indeterminate-cultivar case is asserted directly against the parsed fields.
    let n = nameparser::parse("Polygonum", None, Some(Rank::Cultivar), None)
        .expect("`Polygonum` (rank=CULTIVAR) should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Polygonum"));
    assert!(n.specific_epithet.is_none());
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert!(n.cultivar_epithet.is_none());
    assert_eq!(n.rank, Rank::Cultivar);
    assert_eq!(n.code, Some(NomCode::Cultivars));
    assert_eq!(n.type_, NameType::Informal);
    assert_eq!(n.warnings, vec![warnings::INDETERMINED.to_string()]);

    // `.indet(genus, null, RANK)` — again a null epithet; ParsedName also has no ported
    // `isIndetermined()` (Java's own extra check here), so that one assertion is dropped.
    let n = nameparser::parse("Polygonum", None, Some(Rank::Subspecies), None)
        .expect("`Polygonum` (rank=SUBSPECIES) should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Polygonum"));
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Subspecies);
    assert_eq!(n.type_, NameType::Informal);
    assert_eq!(n.warnings, vec![warnings::INDETERMINED.to_string()]);

    // conflict
    assert_name_rank("Polygonum alba", Rank::Genus)
        .binomial("Polygonum", None, "alba", Rank::Genus)
        .type_(NameType::Informal)
        .doubtful()
        .warning(&[warnings::RANK_MISMATCH])
        .nothing_else();
}

/// https://github.com/gbif/name-parser/issues/5
#[test]
fn zoo_subspecies() {
    // zoological trinomials are by default subspecies, not INFRASPECIFIC_NAME !!!
    assert_name_code("Vulpes vulpes silaceus Miller, 1907", NomCode::Zoological)
        .infra_species("Vulpes", "vulpes", Rank::Subspecies, "silaceus")
        .comb_authors(Some("1907"), &["Miller"])
        .code(NomCode::Zoological)
        .nothing_else();

    // inferred code
    assert_name("Vulpes vulpes silaceus Miller, 1907")
        .infra_species("Vulpes", "vulpes", Rank::Subspecies, "silaceus")
        .comb_authors(Some("1907"), &["Miller"])
        .code(NomCode::Zoological)
        .nothing_else();

    // sp likely misspelled ssp for subspecies
    assert_name("Vulpes vulpes sp. silaceus Miller, 1907")
        .infra_species("Vulpes", "vulpes", Rank::Subspecies, "silaceus")
        .comb_authors(Some("1907"), &["Miller"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn microbial_ranks2() {
    assert_name("Puccinia graminis f. sp. avenae")
        .infra_species("Puccinia", "graminis", Rank::FormaSpecialis, "avenae")
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn chinese_authors() {
    assert_name("Abaxisotima acuminata (Wang & Liu, 1996)")
        .species("Abaxisotima", "acuminata")
        .bas_authors(Some("1996"), &["Wang", "Liu"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Abaxisotima acuminata (Wang, Yuwen & Xian-wei Liu, 1996)")
        .species("Abaxisotima", "acuminata")
        .bas_authors(Some("1996"), &["Wang", "Yuwen", "Xian-wei Liu"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Abaxisotima bicolor (Liu, Xian-wei, Z. Zheng & G. Xi, 1991)")
        .species("Abaxisotima", "bicolor")
        .bas_authors(Some("1991"), &["Liu", "Xian-wei", "Z.Zheng", "G.Xi"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn etal() {
    assert_authorship("Hernández-García et. al., 2023", &[])
        .comb_authors(Some("2023"), &["Hernández-García", "al."])
        .nothing_else();

    assert_authorship(
        "Fischer-Le Saux et al., 1999 emend. Akhurst et al., 2004",
        &[],
    )
    .comb_authors(Some("1999"), &["Fischer-Le Saux", "al."])
    .sensu("emend. Akhurst et al., 2004")
    .nothing_else();
}
