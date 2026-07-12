// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserImplTest (methods on lines 1284-1969).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NamePart, NameType, NomCode, Rank};

#[test]
fn unparsable_placeholders() {
    assert_unparsable("Iteaphila-group", NameType::Informal);
    assert_unparsable("Bartonella group", NameType::Informal);
}

/// "-lineage" labels are informal phylogenetic group names that, like the "-group" /
/// "-complex" aggregates, can refer to any rank and so are treated as INFORMAL.
/// Unlike those, the stem is often an OTU-/strain-like code with digits or a lowercase
/// start (NC12A-lineage, he2-lineage).
#[test]
fn lineage_informal() {
    assert_unparsable("Vermistella-lineage", NameType::Informal);
    assert_unparsable("Flamella-lineage", NameType::Informal);
    assert_unparsable("Pessonella-lineage", NameType::Informal);
    assert_unparsable("NC12A-lineage", NameType::Informal);
    assert_unparsable("he2-lineage", NameType::Informal);
}

/// Regression tests graduated from TODO-names.txt: names that the parser now handles
/// correctly. The still-unsupported entries from that file are documented (with their
/// desired parse) in `todo_names_unsupported()` below.
#[test]
fn todo_names() {
    assert_name("Pseudoleptomesochrella incerta (Chap. and Delam. -deb., 1956)")
        .species("Pseudoleptomesochrella", "incerta")
        .bas_authors(Some("1956"), &["Chap.", "Delam.-deb."])
        .code(NomCode::Zoological)
        .nothing_else();

    // homoglyph "?" in the author abbreviation cannot be recovered, so it is stripped
    assert_name("Rosa intermedia Cr?p.")
        .species("Rosa", "intermedia")
        .comb_authors(None, &["Crp."])
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();

    assert_name("Rosa alpestris D?s?gl.")
        .species("Rosa", "alpestris")
        .comb_authors(None, &["Dsgl"])
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();

    assert_name("Digitaria sanguinea Weber, orth. var.")
        .species("Digitaria", "sanguinea")
        .comb_authors(None, &["Weber"])
        .nom_note("orth. var.")
        .nothing_else();

    assert_name("Quercus aquifolia Kotschy ex A.DC., nom. subnud.")
        .species("Quercus", "aquifolia")
        .comb_ex_authors(&["Kotschy"])
        .comb_authors(None, &["A.DC."])
        .nom_note("nom. subnud.")
        .nothing_else();

    assert_name("Spermacoce lanceolata Frank ex C.Presl, pro syn.")
        .species("Spermacoce", "lanceolata")
        .comb_ex_authors(&["Frank"])
        .comb_authors(None, &["C.Presl"])
        .nom_note("pro syn.")
        .nothing_else();

    assert_name("Cavendishia polyantha H?rold, pro syn.")
        .species("Cavendishia", "polyantha")
        .comb_authors(None, &["Hrold"])
        .nom_note("pro syn.")
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();

    assert_name("Leucopogon veillonii (Virot) comb. ined.")
        .species("Leucopogon", "veillonii")
        .bas_authors(None, &["Virot"])
        .nom_note("comb. ined.")
        .manuscript()
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Vernoniastrum musofense var. miamensis (S. Moore) comb. ined.")
        .infra_species("Vernoniastrum", "musofense", Rank::Variety, "miamensis")
        .bas_authors(None, &["S.Moore"])
        .nom_note("comb. ined.")
        .manuscript()
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Spermacoce tenuis Sess? & Moc., orth. var.")
        .species("Spermacoce", "tenuis")
        .comb_authors(None, &["Sess", "Moc."])
        .nom_note("orth. var.")
        .doubtful()
        .warning(&[warnings::UNCERTAIN_AUTHORSHIP])
        .nothing_else();

    assert_name("Quercus serra Liebm., non Unger (1845), fossil name.")
        .species("Quercus", "serra")
        .comb_authors(None, &["Liebm."])
        .sensu("non Unger (1845), fossil name.")
        .nothing_else();

    assert_name("Ceratellopsis acuminata sensu Bourdot & Galzin (1927); fide Checklist of Basidiomycota of Great Britain and Ireland (2005)")
        .species("Ceratellopsis", "acuminata")
        .sensu("sensu Bourdot & Galzin (1927); fide Checklist of Basidiomycota of Great Britain and Ireland (2005)")
        .nothing_else();

    assert_name("Agaricus rosellus sensu Withering [Bot. Arr. Brit. Pl. 1: 237 (1787)]; fide Checklist of Basidiomycota of Great Britai")
        .species("Agaricus", "rosellus")
        .sensu("sensu Withering [Bot. Arr. Brit. Pl. 1: 237 (1787)]; fide Checklist of Basidiomycota of Great Britai")
        .nothing_else();

    assert_name("Roridomyces albororidus (Maas Geest. & de Meijer) anon., 2010")
        .species("Roridomyces", "albororidus")
        .bas_authors(None, &["Maas Geest.", "de Meijer"])
        .comb_authors(Some("2010"), &["anon."])
        .nothing_else();

    assert_name("Collybia mephitica sensu Rea (1922); fide Checklist of Basidiomycota of Great Britain and Ireland (2005)")
        .species("Collybia", "mephitica")
        .sensu("sensu Rea (1922); fide Checklist of Basidiomycota of Great Britain and Ireland (2005)")
        .nothing_else();

    assert_name("Russula amoena sensu A.A. Pearson [Naturalist (1948)]; fide Checklist of Basidiomycota of Great Britain and Ireland")
        .species("Russula", "amoena")
        .sensu("sensu A.A. Pearson [Naturalist (1948)]; fide Checklist of Basidiomycota of Great Britain and Ireland")
        .nothing_else();

    assert_name("Puccinia veronicarum sensu Grove (1913) p.p.; fide Checklist of Basidiomycota of Great Britain and Ireland (2005)")
        .species("Puccinia", "veronicarum")
        .sensu("sensu Grove (1913) p.p.; fide Checklist of Basidiomycota of Great Britain and Ireland (2005)")
        .nothing_else();

    assert_name("Uroleptopsis viridis (Perejaslawzewa, 1886) ?")
        .species("Uroleptopsis", "viridis")
        .bas_authors(Some("1886"), &["Perejaslawzewa"])
        .code(NomCode::Zoological)
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();

    assert_name("Apatura iris junonina (Lambillion) & Cabeau, 1910")
        .infra_species("Apatura", "iris", Rank::InfraspecificName, "junonina")
        .bas_authors(None, &["Lambillion"])
        .comb_authors(Some("1910"), &["Cabeau"])
        .nothing_else();

    assert_name("Scleropogon kelloggi (Wilcox, 137)")
        .species("Scleropogon", "kelloggi")
        .bas_authors(Some("137"), &["Wilcox"])
        .code(NomCode::Zoological)
        .doubtful()
        .warning(&[warnings::UNLIKELY_YEAR])
        .nothing_else();

    assert_name("Ospriocerus arizonensis (Bromley, 193k7)")
        .species("Ospriocerus", "arizonensis")
        .bas_authors(Some("193"), &["Bromley"])
        .code(NomCode::Zoological)
        .doubtful()
        .warning(&[warnings::UNLIKELY_YEAR])
        .nothing_else();

    assert_name("Lepidanthrax coquilletti Evenhuis and Hall, 0000")
        .species("Lepidanthrax", "coquilletti")
        .comb_authors(Some("0000"), &["Evenhuis", "Hall"])
        .code(NomCode::Zoological)
        .doubtful()
        .warning(&[warnings::UNLIKELY_YEAR])
        .nothing_else();

    assert_name("Scopula cajanderi (Herz, 1903), 1903-01-01")
        .species("Scopula", "cajanderi")
        .bas_authors(Some("1903"), &["Herz"])
        .comb_authors(Some("1903"), &[])
        .code(NomCode::Zoological)
        .warning(&[warnings::YEAR_INTERPRETED])
        .nothing_else();

    assert_name("Gnathamitermes tubiformans (Buckley, 1862), 1862-01-01")
        .species("Gnathamitermes", "tubiformans")
        .bas_authors(Some("1862"), &["Buckley"])
        .comb_authors(Some("1862"), &[])
        .code(NomCode::Zoological)
        .warning(&[warnings::YEAR_INTERPRETED])
        .nothing_else();

    assert_name("Cyclanthera explodens var. intermedia Cogn. in Kuntze ex Kuntze")
        .infra_species("Cyclanthera", "explodens", Rank::Variety, "intermedia")
        .comb_authors(None, &["Cogn."])
        .published_in("Kuntze ex Kuntze")
        .nothing_else();

    assert_name("Pseudostenophylax clavatus Tian & Li in Tian, Li, Yang & Sun, in Chen, editor, 1993")
        .species("Pseudostenophylax", "clavatus")
        .comb_authors(Some("1993"), &["Tian", "Li"])
        .published_in("Tian, Li, Yang & Sun, in Chen, editor, 1993")
        .nothing_else();

    assert_name("Kanimia nitida (DC:) Baker")
        .species("Kanimia", "nitida")
        .bas_authors(None, &["DC"])
        .comb_authors(None, &["Baker"])
        .code(NomCode::Botanical)
        .nothing_else();
}

/// Names from TODO-names.txt that the parser still gets wrong. Each assertion encodes the
/// desired parse; the inline comment describes the current bug. The method is @Ignore'd so
/// the build stays green — remove the annotation once these are fixed (and graduate the
/// fixed cases into `todo_names()`). nothingElse() is intentionally omitted: only the
/// specific corrected fields are pinned, since the full target state of some fields (note
/// disposition, publishedIn cleanup) is still open.
#[test]
#[ignore = "desired parse for still-unsupported TODO-names.txt entries — not yet implemented"]
fn todo_names_unsupported() {
    // "ex <Author>" with nothing before "ex": currently "ex" becomes an infraspecific epithet.
    assert_name("Pitcairnia cinnagarina ex D. Dietr.")
        .species("Pitcairnia", "cinnagarina")
        .comb_authors(None, &["D.Dietr."]);

    assert_name("Grielum obtusifolium ex Harv.")
        .species("Grielum", "obtusifolium")
        .comb_authors(None, &["Harv."]);

    // Latin nomenclatural notes are currently absorbed as a second author.
    assert_name("Quercus ovalis Gand., opus utique oppr.")
        .species("Quercus", "ovalis")
        .comb_authors(None, &["Gand."])
        .nom_note("opus utique oppr.");

    assert_name("Quercus meridionalis Gand., opus utique oppr.")
        .species("Quercus", "meridionalis")
        .comb_authors(None, &["Gand."])
        .nom_note("opus utique oppr.");

    assert_name("Basanacantha spinosa var. typica K.Schum., not validly publ.")
        .infra_species("Basanacantha", "spinosa", Rank::Variety, "typica")
        .comb_authors(None, &["K.Schum."])
        .nom_note("not validly publ.");

    assert_name("Fraxinus humilior Garsault, opus utique oppr.")
        .species("Fraxinus", "humilior")
        .comb_authors(None, &["Garsault"])
        .nom_note("opus utique oppr.");

    // "Later homonym of a fossil name." should be stripped, not appended as a comb author.
    assert_name("Diospyros oblongifolia (Thwaites) Kosterm., Later homonym of a fossil name.")
        .species("Diospyros", "oblongifolia")
        .bas_authors(None, &["Thwaites"])
        .comb_authors(None, &["Kosterm."])
        .code(NomCode::Botanical);

    assert_name("Euclea rufescens E.Mey., not validly publ.")
        .species("Euclea", "rufescens")
        .comb_authors(None, &["E.Mey."])
        .nom_note("not validly publ.");

    assert_name("Lecythis subbiflora Ruiz & Pav., no type indicated.")
        .species("Lecythis", "subbiflora")
        .comb_authors(None, &["Ruiz", "Pav."])
        .nom_note("no type indicated.");

    // Trailing editorial remark should be stripped, not absorbed as an author.
    assert_name("Menestoria tocoyenae DC., provisionally listed as a synonym.")
        .species("Menestoria", "tocoyenae")
        .comb_authors(None, &["DC."]);

    assert_name("Begonia hatacoa var. viridifolia Golding & Rekha Morris, without type.")
        .infra_species("Begonia", "hatacoa", Rank::Variety, "viridifolia")
        .comb_authors(None, &["Golding", "Rekha Morris"])
        .nom_note("without type.");

    // Unclosed "(" should still yield a basionym, not a plain combination author.
    assert_name("Spilogona acuticornis (Malloch, 1920")
        .species("Spilogona", "acuticornis")
        .bas_authors(Some("1920"), &["Malloch"])
        .code(NomCode::Zoological);

    assert_name("Cerodontha lonicerae (Robineau-desvoidy, 1851")
        .species("Cerodontha", "lonicerae")
        .bas_authors(Some("1851"), &["Robineau-desvoidy"])
        .code(NomCode::Zoological);

    // Trailing edition letter must not become a forename initial ("A.Monod" / "D.Nunomura").
    assert_name("Caecognathia regalis (Monod, 1926A)")
        .species("Caecognathia", "regalis")
        .bas_authors(Some("1926"), &["Monod"])
        .code(NomCode::Zoological);

    assert_name("Caecognathia saikaiensis (Nunomura, 1992D)")
        .species("Caecognathia", "saikaiensis")
        .bas_authors(Some("1992"), &["Nunomura"])
        .code(NomCode::Zoological);

    // "ap. Syr." (apud) is a publication pointer and must not be glued onto "Maxim.".
    assert_name("Geranium sanguineum var. majus Maxim. ap. Syr. & Petunn. in Syr.")
        .infra_species("Geranium", "sanguineum", Rank::Variety, "majus")
        .comb_authors(None, &["Maxim.", "Petunn."])
        .published_in("Syr.");

    // Should be a basionym with year 1902; the stray ")" and date suffix must be cleaned off.
    assert_name("Fidicina aldegondae (Kuhlgatz in Kuhlgatz and Melichar, 1902), 1902-01-01")
        .species("Fidicina", "aldegondae")
        .bas_authors(Some("1902"), &["Kuhlgatz"])
        .code(NomCode::Zoological);

    // "ins Econ. Taxon. Bot. …" is the publication ref, not part of the "Anand Kumar" author.
    assert_name("Primula chamaejasme (Wulfen) K.K. Khanna & Anand Kumar ins Econ. Taxon. Bot., 22(1): 237 (1998), isonym")
        .species("Primula", "chamaejasme")
        .bas_authors(None, &["Wulfen"])
        .comb_authors(None, &["K.K.Khanna", "Anand Kumar"])
        .nom_note("isonym")
        .code(NomCode::Botanical);

    // "Fl. Brit. W. I. 147. 1859" is the publication ref, not a second author with year 147.
    assert_name("Ilex montana var. lanceolata (Macfad.) Griseb., Fl. Brit. W. I. 147. 1859")
        .infra_species("Ilex", "montana", Rank::Variety, "lanceolata")
        .bas_authors(None, &["Macfad."])
        .comb_authors(None, &["Griseb."])
        .code(NomCode::Botanical);

    // basionym Grunow (publ. in Cleve & Müller), combination D.G. Mann (in Round et al., 1990).
    assert_name("Tryblionella marginulata (Grunow in Cleve & M?ller) D.G. Mann in Round et al., 1990")
        .species("Tryblionella", "marginulata")
        .bas_authors(None, &["Grunow"])
        .comb_authors(None, &["D.G.Mann"]);

    // The trailing "?" must not survive inside the year ("1978?").
    assert_name("Psilopteryx psorosa subsp. retezatica Botosaneanu & ?Schneider, 1978?")
        .infra_species("Psilopteryx", "psorosa", Rank::Subspecies, "retezatica")
        .comb_authors(Some("1978"), &["Botosaneanu", "Schneider"])
        .code(NomCode::Zoological)
        .doubtful();

    // "(auct.)" should become the taxonomic note, not be left unparsed (state PARTIAL).
    assert_name("Osmanthus ilicifolius f. variegatus (auct.) Rehder")
        .infra_species("Osmanthus", "ilicifolius", Rank::Form, "variegatus")
        .comb_authors(None, &["Rehder"])
        .sensu("auct.");

    // "?" is a homoglyph for the hybrid sign "×" → nothospecies, not an INFORMAL "?" placeholder.
    assert_name("Magnolia ?soulangeana Hamel (pro sp.)")
        .species("Magnolia", "soulangeana")
        .notho(&[NamePart::Specific])
        .comb_authors(None, &["Hamel"])
        .nom_note("pro sp.");
}

#[test]
fn rank_explicit() {
    assert_name_rank("Achillea millefolium L.", Rank::Species)
        .species("Achillea", "millefolium")
        .comb_authors(None, &["L."])
        .nothing_else();

    assert_name_rank("Achillea millefolium L.", Rank::SpeciesAggregate)
        .binomial("Achillea", None, "millefolium", Rank::SpeciesAggregate)
        .comb_authors(None, &["L."])
        .nothing_else();

    // higher ranks should be marked as doubtful
    for r in Rank::ALL {
        if r.other_or_unranked() || r.is_species_or_below() || r.get_major_rank() == Rank::DivisionZoology {
            continue;
        }
        let mut ass = assert_name_rank("Achillea millefolium L.", r)
            .binomial("Achillea", None, "millefolium", r)
            .comb_authors(None, &["L."])
            .type_(NameType::Informal)
            .doubtful();
        if let Some(code) = r.is_restricted_to_code() {
            ass = ass.code(code);
        }
        ass = ass.warning(&[warnings::RANK_MISMATCH]);
        ass.nothing_else();
    }
}

#[test]
fn candidatus() {
    assert_name("\"Candidatus Endowatersipora\" Anderson and Haygood, 2007")
        .monomial("Endowatersipora")
        .candidatus()
        .comb_authors(Some("2007"), &["Anderson", "Haygood"])
        .nothing_else();

    assert_name("Candidatus Phytoplasma allocasuarinae")
        .species("Phytoplasma", "allocasuarinae")
        .candidatus()
        .nothing_else();

    assert_name("Ca. Phytoplasma allocasuarinae")
        .species("Phytoplasma", "allocasuarinae")
        .candidatus()
        .nothing_else();

    assert_name("Ca. Phytoplasma")
        .monomial("Phytoplasma")
        .candidatus()
        .nothing_else();

    assert_name("'Candidatus Nicolleia'")
        .monomial("Nicolleia")
        .candidatus()
        .nothing_else();

    assert_name("\"Candidatus Riegeria\" Gruber-Vodicka et al., 2011")
        .monomial("Riegeria")
        .comb_authors(Some("2011"), &["Gruber-Vodicka", "al."])
        .candidatus()
        .nothing_else();

    assert_name("Candidatus Endobugula")
        .monomial("Endobugula")
        .candidatus()
        .nothing_else();

    assert_name("Candidatus Liberibacter solanacearum")
        .species("Liberibacter", "solanacearum")
        .candidatus()
        .nothing_else();

    // not candidate names
    assert_name("Centropogon candidatus Lammers")
        .species("Centropogon", "candidatus")
        .comb_authors(None, &["Lammers"])
        .nothing_else();
}

#[test]
fn odd_fungi_ranks() {
    assert_name("Cyphelium disseminatum ⍺ subsessile")
        .infra_species("Cyphelium", "disseminatum", Rank::InfraspecificName, "subsessile")
        .nothing_else();

    assert_name("Capitularia fimbriata *** carpophora")
        .infra_species("Capitularia", "fimbriata", Rank::InfraspecificName, "carpophora")
        .nothing_else();

    assert_name("Cyphelium disseminatum c subsessile")
        .infra_species("Cyphelium", "disseminatum", Rank::InfraspecificName, "subsessile")
        .nothing_else();

    assert_name("Cyphelium disseminatum g subsessile")
        .infra_species("Cyphelium", "disseminatum", Rank::InfraspecificName, "subsessile")
        .nothing_else();
}

#[test]
#[ignore = "very odd names - rare and no priority"]
fn odd_fungi_ranks_unsupported() {
    assert_name("Capitularia fimbriata ⍺ vulgaris 3 tubaeformis *** carpophora")
        .infra_species("Capitularia", "fimbriata", Rank::InfraspecificName, "carpophora")
        .nothing_else();

    assert_name("Capitularia pyxidata ß longipes H. carpophora Floerke")
        .infra_species("Capitularia", "pyxidata", Rank::InfraspecificName, "carpophora")
        .nothing_else();
}

#[test]
fn strains() {
    // Java: `.species("Endobugula", null)` — a null specific epithet; `species()` requires a
    // real epithet `&str`, so this one asserts the fields directly instead of chaining through
    // the builder — the same DSL-gap workaround as `unparsable_placeholder`'s "Aster indet."
    // case in impl_03.rs.
    let n = nameparser::parse("Endobugula sp. JYr4", None, None, None)
        .expect("`Endobugula sp. JYr4` should parse");
    assert!(n.uninomial.is_none());
    assert_eq!(n.genus.as_deref(), Some("Endobugula"));
    assert!(n.infrageneric_epithet.is_none());
    assert!(n.specific_epithet.is_none());
    assert!(n.infraspecific_epithet.is_none());
    assert_eq!(n.rank, Rank::Species);
    assert_eq!(n.phrase.as_deref(), Some("JYr4"));
    assert_eq!(n.type_, NameType::Informal);

    // Java: `assertPhraseName(...).species("Lepidoptera", null)` — again a null specific
    // epithet, so this reproduces assert_phrase_name's own checks (canonical, phrase, rank,
    // type) plus the genus/epithet-absent assertions directly, the same workaround as above.
    let n2 = nameparser::parse("Lepidoptera sp. JGP0404", None, None, None)
        .expect("`Lepidoptera sp. JGP0404` should parse");
    assert_eq!(n2.canonical_name().as_deref(), Some("Lepidoptera sp. JGP0404"));
    assert_eq!(n2.rank, Rank::Species);
    assert_eq!(n2.type_, NameType::Informal);
    assert!(n2.uninomial.is_none());
    assert_eq!(n2.genus.as_deref(), Some("Lepidoptera"));
    assert!(n2.infrageneric_epithet.is_none());
    assert!(n2.specific_epithet.is_none());
    assert!(n2.infraspecific_epithet.is_none());
    assert_eq!(n2.phrase.as_deref(), Some("JGP0404"));

    // avoid author & year to be accepted as strain
    assert_name("Anniella nigra FISCHER 1885")
        .species("Anniella", "nigra")
        .comb_authors(Some("1885"), &["Fischer"])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// A quoted strain designation introduced by a "str."/"strain" marker must be kept intact as the
/// phrase of an informal STRAIN name — not mangled into fake authorship ("K.2.'Aph & '") by the
/// messy "str ." spacing and glued ".'Aph" quote, and not reinterpreted as a cultivar epithet
/// (which silently dropped the requested STRAIN rank). See parser-handover-strain.md / CoL
/// HomotypicConsolidatorIT.
#[test]
fn strain_designations() {
    // glued dot before the quote, with a redundant leading "strain" marker
    assert_name_rank("Aphanizomenon flos-aquae strain str .'Aph K2'", Rank::Strain)
        .binomial("Aphanizomenon", None, "flos-aquae", Rank::Strain)
        .phrase("Aph K2")
        .type_(NameType::Informal)
        .nothing_else();

    // "str ." spaced dot + glued quote
    assert_name_rank("Aphanizomenon flos-aquae str .'Aph K2'", Rank::Strain)
        .binomial("Aphanizomenon", None, "flos-aquae", Rank::Strain)
        .phrase("Aph K2")
        .type_(NameType::Informal)
        .nothing_else();

    // clean spacing must NOT become a CULTIVAR
    assert_name_rank("Aphanizomenon flos-aquae str. 'Aph K2'", Rank::Strain)
        .binomial("Aphanizomenon", None, "flos-aquae", Rank::Strain)
        .phrase("Aph K2")
        .type_(NameType::Informal)
        .nothing_else();

    // designation with digits and slashes stays verbatim
    assert_name_rank("Aphanizomenon gracile str .'Heaney 1986/Camb140 1/1'", Rank::Strain)
        .binomial("Aphanizomenon", None, "gracile", Rank::Strain)
        .phrase("Heaney 1986/Camb140 1/1")
        .type_(NameType::Informal)
        .nothing_else();
}

/// A name carrying an explicit infraspecific rank marker keeps that specific rank (SUBSPECIES,
/// VARIETY, …) and is never downgraded to the unspecific INFRASPECIFIC_NAME — including the
/// indeterminate forms where the terminal epithet is absent and only authorship trails the marker.
#[test]
fn explicit_marker_keeps_specific_rank() {
    assert_eq!(
        nameparser::parse("Canis lupus subsp. Linnaeus, 1758", None, None, None).unwrap().rank,
        Rank::Subspecies
    );
    assert_eq!(
        nameparser::parse("Nitzschia sinuata var. (Grunow) Lange-Bert.", None, None, None).unwrap().rank,
        Rank::Variety
    );
    assert_eq!(
        nameparser::parse("Aphelocoma californica subsp.", None, None, None).unwrap().rank,
        Rank::Subspecies
    );
    assert_eq!(
        nameparser::parse("Aus bus var.", None, None, None).unwrap().rank,
        Rank::Variety
    );
}

/// Surname-first authors with comma-separated hyphenated given-name initials invert to the
/// canonical `<initials>.<surname>` form, keeping the hyphen and the input case of a
/// lower-case follow-up letter. Previously the comma form produced two authors ("Wang & Y.-j.")
/// or dropped a dot ("C-K.Yang"). (A2)
#[test]
fn comma_hyphenated_initials() {
    assert_authorship("Wang, Y.-j.", &["Y.-j.Wang"]);
    assert_authorship("Yang, C.-K.", &["C.-K.Yang"]);
    assert_authorship("Wang, Y.-j. & Liu, Z.-q.", &["Y.-j.Wang", "Z.-q.Liu"]);
    // the space form was already correct and stays so
    assert_authorship("Y.-j. Wang", &["Y.-j.Wang"]);
}

/// A bare trailing "sp."/"spec." after a complete binomial is a redundant leftover marker: it is
/// dropped and the name stays a SPECIES, instead of "sp" becoming an infraspecific epithet at the
/// unspecific INFRASPECIFIC_NAME rank. (A3)
#[test]
fn trailing_sp_marker() {
    assert_name("Vulpes vulpes sp.")
        .species("Vulpes", "vulpes")
        .nothing_else();
    assert_name("Vulpes vulpes sp")
        .species("Vulpes", "vulpes")
        .nothing_else();
}

/// "Genus (Word)" alone is a subgenus, not a monomial + parenthesised basionym author (a genus
/// cannot carry a parenthesised basionym author). A genuine species recombination keeps its
/// parenthesised basionym and votes ZOOLOGICAL, with or without a year. (A5)
#[test]
fn parenthesised_subgenus_not_basionym() {
    assert_name("Arrhoges (Antarctohoges)")
        .infrageneric_at("Arrhoges", Rank::InfragenericName, "Antarctohoges")
        .nothing_else();
    assert_name("Abies alba (Smith)")
        .species("Abies", "alba")
        .bas_authors(None, &["Smith"])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// A genus (monomial) parenthesised token disambiguates subgenus vs. basionym author by the
/// year: (1) a botanical genus recombination "Genus (BasAuthor) CombAuthor" (no year, a
/// combination author follows) is a basionym; (2) a zoological "Genus (Author, year)" with the
/// year INSIDE the brackets is a basionym; (3) a "Genus (Subgenus) Author, year" with the year
/// OUTSIDE the brackets is a subgenus + authorship; (4) "Genus (Word)" alone is a subgenus.
#[test]
fn genus_basionym_versus_subgenus() {
    // (1) botanical genus recombination — parenthesised basionym author + combination author
    assert_name("Kyphocarpa (Fenzl) Lopr.")
        .monomial("Kyphocarpa")
        .bas_authors(None, &["Fenzl"])
        .comb_authors(None, &["Lopr."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Thliphthisa (Griseb.) P.Caputo & Del Guacchio")
        .monomial("Thliphthisa")
        .bas_authors(None, &["Griseb."])
        .comb_authors(None, &["P.Caputo", "Del Guacchio"])
        .code(NomCode::Botanical)
        .nothing_else();
    // an ABBREVIATED parenthesised word is always an author, never a subgenus (subgenera are a
    // single unabbreviated capitalised word) — so it is a basionym even with a trailing year,
    // which would otherwise (for an unabbreviated word) signal the "Genus (Subgenus) Author, year"
    // subgenus form.
    assert_name("Foa (Grev.) Kutz. 1849")
        .monomial("Foa")
        .bas_authors(None, &["Grev."])
        .comb_authors(Some("1849"), &["Kutz."])
        .nothing_else();
    // (2) zoological genus basionym — year inside the brackets, no combination author
    assert_name("Heptacyclus (Vasileyev, 1939)")
        .monomial("Heptacyclus")
        .bas_authors(Some("1939"), &["Vasileyev"])
        .code(NomCode::Zoological)
        .nothing_else();
    // (3) genus + subgenus + authorship — year OUTSIDE the brackets (ZooBank mixed format)
    assert_name("Dicromita (Pterodicromita) Fowler, 1925")
        .infrageneric_at("Dicromita", Rank::InfragenericName, "Pterodicromita")
        .comb_authors(Some("1925"), &["Fowler"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Tenthredo (Macrophya) Dahlbom, 1835")
        .infrageneric_at("Tenthredo", Rank::InfragenericName, "Macrophya")
        .comb_authors(Some("1835"), &["Dahlbom"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Caranx (Usa) Whitley, 1927")
        .infrageneric_at("Caranx", Rank::InfragenericName, "Usa")
        .comb_authors(Some("1927"), &["Whitley"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Oligota (Logiota) Mulsant & Rey 1873")
        .infrageneric_at("Oligota", Rank::InfragenericName, "Logiota")
        .comb_authors(Some("1873"), &["Mulsant", "Rey"])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// ZooBank-style "Author in Author & Author, year" citation: the leading author is the name's
/// author, the "in …" part is the publication reference and its year is the authorship year.
#[test]
fn in_authors_citation() {
    assert_name("Gnatholigota Sharp in Sharp & Scott, 1908")
        .monomial("Gnatholigota")
        .comb_authors(Some("1908"), &["Sharp"])
        .published_in("Sharp & Scott, 1908")
        .nothing_else();
}

/// The end-anchored sensu-lato marker is case-sensitive: lower-case "s.lat."/"s.l." is a
/// taxonomic note, but uppercase trailing author initials ("… S.L.") are NOT swept into the
/// taxonomicNote. (A6)
#[test]
fn sensu_lato_not_eating_uppercase_initials() {
    assert_name("Asplenium trichomanes L. s.lat.")
        .species("Asplenium", "trichomanes")
        .comb_authors(None, &["L."])
        .sensu("s.lat.")
        .nothing_else();
    // uppercase "S.L." is author initials, not a sensu marker
    assert!(
        nameparser::parse("Quercus robur Author S.L.", None, None, None)
            .unwrap()
            .taxonomic_note
            .is_none()
    );
}

/// A particle-led input ("van Berg", "del Rosario Author") is an author name, not a lowercase
/// epithet whose genus went missing — it must NOT be turned into a "? van Berg" PLACEHOLDER with
/// a MISSING_GENUS warning. (A7)
#[test]
fn missing_genus_not_particle() {
    let n = nameparser::parse("van Berg", None, None, None)
        .expect("`van Berg` should parse (not a placeholder)");
    assert_ne!(n.type_, NameType::Placeholder);
    assert!(!n.warnings.contains(&warnings::MISSING_GENUS.to_string()));

    let n2 = nameparser::parse("del Rosario Author", None, None, None)
        .expect("`del Rosario Author` should parse (not a placeholder)");
    assert_ne!(n2.type_, NameType::Placeholder);
    assert!(!n2.warnings.contains(&warnings::MISSING_GENUS.to_string()));
}
