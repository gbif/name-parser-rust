// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserImplTest (methods on lines 385-719).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NameType, NomCode, Rank};

/// https://github.com/gbif/name-parser/issues/58
#[test]
fn vd_authors() {
    assert_name("Taraxacum dunense v. Soest")
        .species("Taraxacum", "dunense")
        .comb_authors(None, &["v.Soest"])
        .nothing_else();

    assert_name("Rubus planus v. d. Beek")
        .species("Rubus", "planus")
        .comb_authors(None, &["v.d.Beek"])
        .nothing_else();
}

/// https://github.com/gbif/name-parser/issues/56
#[test]
fn rechf() {
    assert_name("Salix repens L. subsp. galeifolia Neumann ex Rech. f.")
        .infra_species("Salix", "repens", Rank::Subspecies, "galeifolia")
        .comb_ex_authors(&["Neumann"])
        .comb_authors(None, &["Rech.f."])
        .code(NomCode::Botanical)
        .nothing_else();
}

/// The "f." / "fil." / "filius" suffix is the regulated botanical convention for the
/// son of a same-named author, but it does also appear in older zoological literature
/// to distinguish father and son authorities. Those zoological cases always carry a
/// year, so the year-on-author-span signal correctly classifies them as ZOOLOGICAL.
#[test]
fn filius_zoological() {
    assert_name("Lacerta agilis Linnaeus f., 1789")
        .species("Lacerta", "agilis")
        .comb_authors(Some("1789"), &["Linnaeus f."])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Testudo graeca Linnaeus f., 1789")
        .species("Testudo", "graeca")
        .comb_authors(Some("1789"), &["Linnaeus f."])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Raja batis Forster f., 1781")
        .species("Raja", "batis")
        .comb_authors(Some("1781"), &["Forster f."])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// "L. f." is Carl Linnaeus the Younger (filius), rendered in the glued IPNI form
/// "L.f." — a single author, distinct from the second author "Rosenst." (Rosenstock).
/// The filius suffix marks the name as botanical.
#[test]
fn filius_two_authors() {
    assert_name("Polypodium pectinatum L. f., Rosenst.")
        .species("Polypodium", "pectinatum")
        .comb_authors(None, &["L.f.", "Rosenst."])
        .code(NomCode::Botanical)
        .nothing_else();
}

/// "filius" directly after the genus is the species epithet, not a filius author suffix:
/// "Gyracanthus filius Snyder, 2011" is the species Gyracanthus filius authored by Snyder.
#[test]
fn filius_as_epithet() {
    assert_name("Gyracanthus filius Snyder, 2011")
        .species("Gyracanthus", "filius")
        .comb_authors(Some("2011"), &["Snyder"])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// https://github.com/gbif/name-parser/issues/49
#[test]
fn wfo_authors() {
    assert_name("Taraxacum vulgaris Backer ex K.Heyne")
        .species("Taraxacum", "vulgaris")
        .comb_authors(None, &["K.Heyne"])
        .comb_ex_authors(&["Backer"])
        .nothing_else();

    // unpublished = manuscript name
    assert_name("Taraxacum oosterveldii Petrasiak & Johansen, unpublished")
        .species("Taraxacum", "oosterveldii")
        .comb_authors(None, &["Petrasiak", "Johansen"])
        .nom_note("unpublished")
        .manuscript()
        .nothing_else();
}

/// https://github.com/gbif/portal-feedback/issues/3535
#[test]
fn no_hybrids() {
    assert_name("Lepidodens similis Zhang F & Pan Z-X in Zhang, F, Pan, Z-X, Wu, J, Ding, Y-H, Yu, D-Y & Wang, B-X, 2016")
        .species("Lepidodens", "similis")
        .comb_authors(Some("2016"), &["F.Zhang", "Z-X.Pan"])
        .published_in("Zhang, F, Pan, Z-X, Wu, J, Ding, Y-H, Yu, D-Y & Wang, B-X, 2016")
        .nothing_else();
}

/// https://github.com/gbif/checklistbank/issues/87
#[test]
fn nom_refs() {
    assert_name("Passiflora plumosa Feuillet & Cremers, Proceedings of the Koninklijke Nederlandse Akademie van Wetenschappen, Series C: Biological and Medical Sciences 87(3): 381, f. 2. 1984. Fig. 2I, J")
        .species("Passiflora", "plumosa")
        .comb_authors(None, &["Feuillet", "Cremers"])
        .published_in("Proceedings of the Koninklijke Nederlandse Akademie van Wetenschappen, Series C: Biological and Medical Sciences 87(3): 381, f. 2. 1984. Fig. 2I, J")
        .warning(&[warnings::NOMENCLATURAL_REFERENCE])
        .nothing_else();

    assert_name("Passiflora jussieui Feuillet, Journal of the Botanical Research Institute of Texas 4(2): 611, f. 1. 2010. Figs 2E, F, 3E, F")
        .species("Passiflora", "jussieui")
        .comb_authors(None, &["Feuillet"])
        .published_in("Journal of the Botanical Research Institute of Texas 4(2): 611, f. 1. 2010. Figs 2E, F, 3E, F")
        .warning(&[warnings::NOMENCLATURAL_REFERENCE])
        .nothing_else();

    assert_name("Passiflora eglandulosa J.M. MacDougal. Annals of the Missouri Botanical Garden 75: 1658-1662. figs 1, 2B, and 3. 1988. Figs 36-37")
        .species("Passiflora", "eglandulosa")
        .comb_authors(None, &["J.M.MacDougal"])
        .published_in("Annals of the Missouri Botanical Garden 75: 1658-1662. figs 1, 2B, and 3. 1988. Figs 36-37")
        .warning(&[warnings::NOMENCLATURAL_REFERENCE])
        .nothing_else();

    assert_name("Passiflora eglandulosa J.M. MacDougal. Lingua franca de Missouri Botanical Garden 75: 1658-1662. figs 1, 2B, and 3. 1988. Figs 36-37")
        .species("Passiflora", "eglandulosa")
        .comb_authors(None, &["J.M.MacDougal"])
        .published_in("Lingua franca de Missouri Botanical Garden 75: 1658-1662. figs 1, 2B, and 3. 1988. Figs 36-37")
        .warning(&[warnings::NOMENCLATURAL_REFERENCE])
        .nothing_else();
}

/// https://github.com/gbif/checklistbank/issues/87
#[test]
fn blacklisted() {
    assert_name("Abies null Hood")
        .species("Abies", "null")
        .comb_authors(None, &["Hood"])
        .doubtful()
        .warning(&[warnings::NULL_EPITHET])
        .nothing_else();

    // a literal "Null" genus is a data artefact too — flag doubtful like a null epithet
    assert_name("Null bactus")
        .species("Null", "bactus")
        .doubtful()
        .warning(&[warnings::NULL_EPITHET])
        .nothing_else();

    assert_name("Null")
        .monomial("Null")
        .doubtful()
        .warning(&[warnings::NULL_EPITHET])
        .nothing_else();

    assert_unparsable("Unidentified unidentified Hood", NameType::Placeholder);

    assert_unparsable("Abies unidentified", NameType::Placeholder);

    assert_name("Passiflora possible Müller")
        .species("Passiflora", "possible")
        .comb_authors(None, &["Müller"])
        .doubtful()
        .warning(&[warnings::BLACKLISTED_EPITHET])
        .nothing_else();
}

#[test]
fn author_with_publication() {
    // in publications often give the year the name was published. We propagate that to the authorship instance
    // and for botanical names we simply don't show it in the NameFormatter

    // these are difficult to find the cutoff between author and the publishedIn reference
    assert_name("Passiflora eglandulosa J.M. MacDougal. Annals of the Missouri Botanical Garden 75. figs 1, 2B, and 3. 1988. Figs 36-37")
        .species("Passiflora", "eglandulosa")
        .comb_authors(Some("1988"), &["J.M.MacDougal"])
        .published_in("Annals of the Missouri Botanical Garden 75. figs 1, 2B, and 3. 1988. Figs 36-37")
        .nothing_else();

    // IPNI botanical style, reference after a comma behind the (abbreviated?) author
    assert_name("Samyda arborea Rich., Actes Soc. Hist. Nat. Paris 1: 109 (1792).")
        .species("Samyda", "arborea")
        .comb_authors(Some("1792"), &["Rich."])
        .published_in("Actes Soc. Hist. Nat. Paris 1: 109 (1792)")
        .nothing_else();

    assert_name("Casearia arborea Urb., Symb. Antill. (Urban). 4(3): 421 (1910).")
        .species("Casearia", "arborea")
        .comb_authors(Some("1910"), &["Urb."])
        .published_in("Symb. Antill. (Urban). 4(3): 421 (1910)")
        .nothing_else();

    assert_name("Abuta candicans Rich. in DC., Syst. Nat. 1: 543 (1817).")
        .species("Abuta", "candicans")
        .comb_authors(Some("1817"), &["Rich."])
        .published_in("DC., Syst. Nat. 1: 543 (1817)")
        .nothing_else();

    assert_name("Antacanthus Rich. ex DC., Prodr. 4: 484 (1830), pro syn.")
        .monomial("Antacanthus")
        .comb_authors(Some("1830"), &["DC."])
        .comb_ex_authors(&["Rich."])
        .published_in("Prodr. 4: 484 (1830)")
        .nom_note("pro syn.")
        .nothing_else();

    assert_name(
        "Aegiphila pyramidata Rich. ex Moldenke, Phytologia 1: 204, in obs., pro syn. (1937).",
    )
    .species("Aegiphila", "pyramidata")
    .comb_authors(Some("1937"), &["Moldenke"])
    .comb_ex_authors(&["Rich."])
    .published_in("Phytologia 1: 204, (1937)")
    .nom_note("in obs., pro syn.")
    .nothing_else();

    assert_name("Amplexoididae Wang, Guang-Xu in Wang, He, Tang & Percival, 2018")
        .monomial("Amplexoididae")
        .comb_authors(Some("2018"), &["Wang", "Guang-Xu"])
        .published_in("Wang, He, Tang & Percival, 2018")
        .nothing_else();

    assert_name(
        "Roelofinae St Laurent & Kawahara in St Laurent, Mielke, Herbin, Dexter & Kawahara 2020",
    )
    .monomial("Roelofinae")
    .comb_authors(Some("2020"), &["St Laurent", "Kawahara"])
    .published_in("St Laurent, Mielke, Herbin, Dexter & Kawahara 2020")
    .nothing_else();

    assert_name("Charlottea Whalen & Carter in Carter, Whalen & Guex, 1998")
        .monomial("Charlottea")
        .comb_authors(Some("1998"), &["Whalen", "Carter"])
        .published_in("Carter, Whalen & Guex, 1998")
        .nothing_else();
}

#[test]
fn zoobank() {
    assert_name("Euplexauridae McFadden, van Ofwegen & Quattrini, 2022")
        .monomial("Euplexauridae")
        .comb_authors(Some("2022"), &["McFadden", "van Ofwegen", "Quattrini"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Culexlineatascinciina Hoser, 2015")
        .monomial("Culexlineatascinciina")
        .comb_authors(Some("2015"), &["Hoser"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn species() {
    assert_name("Diodia teres Walter")
        .species("Diodia", "teres")
        .comb_authors(None, &["Walter"])
        .nothing_else();

    assert_name("Dysponetus bulbosus Hartmann-Schroder 1982")
        .species("Dysponetus", "bulbosus")
        .comb_authors(Some("1982"), &["Hartmann-Schroder"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Zophosis persis (Chatanay 1914)")
        .species("Zophosis", "persis")
        .bas_authors(Some("1914"), &["Chatanay"])
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("Abies alba Mill.")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .nothing_else();

    assert_name("Alstonia vieillardii Van Heurck & Müll.Arg.")
        .species("Alstonia", "vieillardii")
        .comb_authors(None, &["Van Heurck", "Müll.Arg."])
        .nothing_else();

    assert_name("Angiopteris d'urvilleana de Vriese")
        .species("Angiopteris", "d'urvilleana")
        .comb_authors(None, &["de Vriese"])
        .nothing_else();

    assert_name("Agrostis hyemalis (Walter) Britton, Sterns, & Poggenb.")
        .species("Agrostis", "hyemalis")
        .comb_authors(None, &["Britton", "Sterns", "Poggenb."])
        .bas_authors(None, &["Walter"])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn species_with_subgenus() {
    assert_name("Passalus (Pertinax) gaboi Jiménez-Maxim & Reyes, 2022")
        .species_ig("Passalus", "Pertinax", "gaboi")
        .comb_authors(Some("2022"), &["Jiménez-Maxim", "Reyes"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn special_epithets() {
    assert_name("Gracillaria v-flava Haworth, 1828")
        .species("Gracillaria", "v-flava")
        .comb_authors(Some("1828"), &["Haworth"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn capital_authors() {
    assert_name("Anniella nigra FISCHER 1885")
        .species("Anniella", "nigra")
        .comb_authors(Some("1885"), &["Fischer"])
        .code(NomCode::Zoological)
        .nothing_else();
}
