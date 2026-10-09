// SPDX-License-Identifier: Apache-2.0
//! Taxonomic notes (`non …`, `auct.`, `auctorum`, `sensu …`) that `org.gbif:name-parser` 4.2.0
//! lost or misread as authors. All of these are deliberate changes against Java parity.
//!
//! Most come from ChecklistBank, which passes the authorship separately and often repeats the
//! note there: `Heliotropium aff. wagneri [non Vierh.]` + `[non Vierh.]`. The separate
//! authorship lacked the name string's bracketed-note step, so the brackets were dropped and the
//! note reached the authorship parser as an author.

mod common;
use common::*;
use nameparser::model::{NamePart, NameType, NomCode, Rank};

#[test]
fn bracketed_non_note_as_separate_authorship() {
    // As CLB passes the record: the note repeated in the separate authorship field, plus a rank
    // hint. The bracketed note must not become a combination author "non Vierh.".
    assert_name_hinted(
        "Heliotropium aff. wagneri [non Vierh.]",
        Some("[non Vierh.]"),
        Some(Rank::Species),
        None,
    )
    .species("Heliotropium", "wagneri")
    .qualifiers(&[(NamePart::Specific, "aff.")])
    .sensu("non Vierh.")
    .type_(NameType::Informal)
    .nothing_else();
    // a real author before the bracket is kept
    assert_name_auth("Rubus fragrans", "Focke [non Salisb.]")
        .species("Rubus", "fragrans")
        .comb_authors(None, &["Focke"])
        .sensu("non Salisb.")
        .nothing_else();
    // a note in front of the bracket keeps its place
    assert_name_auth("Fossombronia pusilla", "auct. amer. [non (L.) Nees]")
        .species("Fossombronia", "pusilla")
        .sensu("auct. amer. non (L.) Nees")
        .nothing_else();
}

#[test]
fn a_bare_note_in_front_of_a_bracketed_one_keeps_both() {
    // The bracket is stripped first; the bare note in front of it used to overwrite it.
    assert_name("Fossombronia pusilla auct. amer. [non (L.) Nees]")
        .species("Fossombronia", "pusilla")
        .sensu("auct. amer. non (L.) Nees")
        .nothing_else();
    assert_name("Andrena labiata auct. (nec Fabricius, 1781)")
        .species("Andrena", "labiata")
        .sensu("auct. nec Fabricius, 1781")
        .nothing_else();
    assert_name("Boletus chioneus sensu auct. [non Fr.]")
        .species("Boletus", "chioneus")
        .sensu("sensu auct. non Fr.")
        .nothing_else();
}

#[test]
fn auctorum_is_a_note() {
    // the spelled-out "auctorum" ("of authors") is auct., not an author
    assert_name("Astacilla bonnieri Auctorum")
        .species("Astacilla", "bonnieri")
        .sensu("auctorum")
        .nothing_else();
    // ... and not an infraspecific epithet after a species
    assert_name("Cucullia ledereri auctorum")
        .species("Cucullia", "ledereri")
        .sensu("auctorum")
        .nothing_else();
    assert_name("Caradrina (Boursinidrina) jacobsi auctorum")
        .species_ig("Caradrina", "Boursinidrina", "jacobsi")
        .sensu("auctorum")
        .nothing_else();
    // in a separate authorship, bare, bracketed or followed by a homonym citation
    for authorship in ["auctorum", "Auctorum", "[auctorum]"] {
        assert_name_auth("Idotea viridis", authorship)
            .species("Idotea", "viridis")
            .sensu("auctorum")
            .nothing_else();
    }
    assert_name_auth("Bombyx religiosae", "auctorum non Westwood, 1847")
        .species("Bombyx", "religiosae")
        .sensu("auctorum non Westwood, 1847")
        .nothing_else();
    assert_name_auth("Minuspio", "[auctorum]")
        .monomial("Minuspio")
        .sensu("auctorum")
        .nothing_else();
}

#[test]
fn auctorum_stays_an_epithet() {
    // real species: right after the genus, or after a rank marker, it is the epithet
    assert_name("Lobotes auctorum Günther, 1859")
        .species("Lobotes", "auctorum")
        .comb_authors(Some("1859"), &["Günther"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Pannaria auctorum Bory")
        .species("Pannaria", "auctorum")
        .comb_authors(None, &["Bory"])
        .nothing_else();
    assert_name("Harnischia (Cryptocladopelma) viridula subsp. auctorum")
        .infra_species("Harnischia", "viridula", Rank::Subspecies, "auctorum")
        .infrageneric("Cryptocladopelma")
        .nothing_else();
    // ... unless a genus rank hint says the name is a monomial
    assert_name_rank("Colobodus auctorum", Rank::Genus)
        .monomial_rank("Colobodus", Rank::Genus)
        .sensu("auctorum")
        .nothing_else();
}

#[test]
fn a_bracketed_note_keyword_takes_its_author_along() {
    // "[sensu] Schmidt, 1878": only the keyword is bracketed, the author follows outside
    assert_name_auth("Coscinodiscus asteromphalus", "[sensu] Schmidt, 1878")
        .species("Coscinodiscus", "asteromphalus")
        .sensu("sensu Schmidt, 1878")
        .nothing_else();
    assert_name("Tubularia penicillus [sensu] Müller, 1776")
        .species("Tubularia", "penicillus")
        .sensu("sensu Müller, 1776")
        .nothing_else();
}

#[test]
fn not_is_a_note_like_non() {
    assert_name_auth("Eurytoma maculipes", "Ashmead 1887 not Motschulsky 1863")
        .species("Eurytoma", "maculipes")
        .comb_authors(Some("1887"), &["Ashmead"])
        .sensu("not Motschulsky 1863")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Apseudes minutus Brown, 1956 not Claus, 1888")
        .species("Apseudes", "minutus")
        .comb_authors(Some("1956"), &["Brown"])
        .sensu("not Claus, 1888")
        .code(NomCode::Zoological)
        .nothing_else();
    // in square brackets, a homonym citation or a remark
    assert_name(
        "Amphisbetia pulchella (Thompson, 1879) [not Amphisbetia pulchella Vannucci-Mendes 1954]",
    )
    .species("Amphisbetia", "pulchella")
    .bas_authors(Some("1879"), &["Thompson"])
    .sensu("not Amphisbetia pulchella Vannucci-Mendes 1954")
    .code(NomCode::Zoological)
    .nothing_else();
    assert_name("Aotus lemurinus hirsutus (J. E. Gray, 1871) [not used as valid]")
        .infra_species("Aotus", "lemurinus", Rank::Subspecies, "hirsutus")
        .bas_authors(Some("1871"), &["J.E.Gray"])
        .sensu("not used as valid")
        .code(NomCode::Zoological)
        .nothing_else();
    // ... but Erwin's Agra not is a species
    assert_name("Agra not Erwin, 2002")
        .species("Agra", "not")
        .comb_authors(Some("2002"), &["Erwin"])
        .code(NomCode::Zoological)
        .warning(&[nameparser::model::warnings::BLACKLISTED_EPITHET])
        .doubtful()
        .nothing_else();
}

#[test]
fn a_parenthesised_homonym_citation_after_the_author() {
    assert_name_auth("Rubus fragrans", "Focke (non Salisb.)")
        .species("Rubus", "fragrans")
        .comb_authors(None, &["Focke"])
        .sensu("non Salisb.")
        .nothing_else();
    assert_name_auth("Prionus heros", "Fall, 1905 (nec Semenov, 1900)")
        .species("Prionus", "heros")
        .comb_authors(Some("1905"), &["Fall"])
        .sensu("nec Semenov, 1900")
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn a_note_repeated_in_the_separate_authorship_is_kept_once() {
    // sources repeat the note in both columns, not always identically
    assert_name_auth(
        "Hemicycla gaudryi auctt. (non d'Orbigny, 1839)",
        "auctt. (non d'Orbigny, 1839)",
    )
    .species("Hemicycla", "gaudryi")
    .sensu("auctt. non d'Orbigny, 1839")
    .nothing_else();
    assert_name_auth(
        "Centropyge fisheri (non Snyder, 1904)",
        "(non Snyder, 1904)",
    )
    .species("Centropyge", "fisheri")
    .sensu("non Snyder, 1904")
    .nothing_else();
    // a shorter copy is already part of the name's note
    assert_name_auth(
        "Aulicus episcopalis sensu Blackburn, 1900 (not Spinola, 1844)",
        "sensu Blackburn",
    )
    .species("Aulicus", "episcopalis")
    .sensu("sensu Blackburn, 1900 not Spinola, 1844")
    .nothing_else();
    // two different notes within one authorship both stay
    assert_name_auth("Boletus circinans", "sensu Pers. [non sensu Pers.]")
        .species("Boletus", "circinans")
        .sensu("sensu Pers. non sensu Pers.")
        .nothing_else();
}

#[test]
fn sensu_stricto_or_lato_in_brackets_is_a_note_wherever_it_stands() {
    // after the authors, before them, and between two epithets
    assert_name("Amaurorhinus bewichianus (Wollaston,1860) (s.str.)")
        .species("Amaurorhinus", "bewichianus")
        .bas_authors(Some("1860"), &["Wollaston"])
        .sensu("s.str.")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Aus bus (s.str.) Smith")
        .species("Aus", "bus")
        .comb_authors(None, &["Smith"])
        .sensu("s.str.")
        .nothing_else();
    assert_name("Ammodramus caudacutus (s.s.) diversus")
        .infra_species(
            "Ammodramus",
            "caudacutus",
            Rank::InfraspecificName,
            "diversus",
        )
        .sensu("s.s.")
        .nothing_else();
    // an author's capital initials are no note
    assert_name("Aus bus (S. L. Schultes)")
        .species("Aus", "bus")
        .bas_authors(None, &["S.L.Schultes"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn a_note_keyword_with_a_stray_dot_is_still_the_note() {
    assert_name("Aus bus (Smith, 1900) sensu. Dworkin and Foster 1956")
        .species("Aus", "bus")
        .bas_authors(Some("1900"), &["Smith"])
        .sensu("sensu. Dworkin and Foster 1956")
        .code(NomCode::Zoological)
        .nothing_else();
    // "nomen nudum" in title case, as 15 ChecklistBank rows write it
    assert_name("Akeratidae Nomen Nudum")
        .monomial("Akeratidae")
        .nom_note("Nomen Nudum")
        .nothing_else();
}

#[test]
fn nova_abbreviated_with_n_is_a_note() {
    // "n. sp." / "sp. n." spell "sp. nov." too
    assert_name("Anomia atacamensis n.sp. HERM 1969")
        .species("Anomia", "atacamensis")
        .comb_authors(Some("1969"), &["Herm"])
        .nom_note("n. sp.")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Cryptopimpla carinifacialis Sheng, sp. n.")
        .species("Cryptopimpla", "carinifacialis")
        .comb_authors(None, &["Sheng"])
        .nom_note("sp. n.")
        .nothing_else();
    // a provisional name keeps it in its phrase
    assert_informal("Heteropriapulus sp. n. AAA-2017")
        .taxon("Heteropriapulus")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. n. AAA-2017")
        .nothing_else();
}

#[test]
fn more_note_spellings() {
    // pro parte without its comma, and as "pro max. parte": flagged doubtful only, as with it
    assert_name("Aconitum gracile Rchb. pro parte")
        .species("Aconitum", "gracile")
        .comb_authors(None, &["Rchb."])
        .doubtful()
        .nothing_else();
    assert_name("Collema alpinum Th.Fr. pro max. parte")
        .species("Collema", "alpinum")
        .comb_authors(None, &["Th.Fr."])
        .doubtful()
        .nothing_else();
    // "sens. str." for "s. str."
    assert_name("Rubus fruticosus L. sens.str.")
        .species("Rubus", "fruticosus")
        .comb_authors(None, &["L."])
        .sensu("sens.str.")
        .nothing_else();
    // "ampl.", the amplified circumscription, like "emend."
    assert_name("Cerastium octandrum Hochst. ex A.Rich. ampl. Möschl")
        .species("Cerastium", "octandrum")
        .comb_authors(None, &["A.Rich."])
        .comb_ex_authors(&["Hochst."])
        .sensu("ampl. Möschl")
        .code(NomCode::Botanical)
        .nothing_else();
    // bracketed "ined." and "in sched." are nomenclatural notes
    assert_name("Leveillula catalpae U. Braun (ined.)")
        .species("Leveillula", "catalpae")
        .comb_authors(None, &["U.Braun"])
        .nom_note("ined.")
        .manuscript()
        .nothing_else();
    assert_name("Crypsis alpicola Hochst. (in sched. rite publ.)")
        .species("Crypsis", "alpicola")
        .comb_authors(None, &["Hochst."])
        .nom_note("in sched. rite publ.")
        .nothing_else();
}

#[test]
fn a_bracketed_quoted_spelling_is_a_nomenclatural_note() {
    // the spelling it was published or also cited in, never part of the author
    assert_name("Heterosperma depressa Griseb. (\"depressum\")")
        .species("Heterosperma", "depressa")
        .comb_authors(None, &["Griseb."])
        .nom_note("\"depressum\"")
        .nothing_else();
    assert_name("Xerochlorella olmiae ('olmae')")
        .species("Xerochlorella", "olmiae")
        .nom_note("'olmae'")
        .nothing_else();
}

/// "not of X" is one note: the "of X" was taken first and "not" stayed behind as an author.
#[test]
fn not_of_is_one_note() {
    assert_name_hinted(
        "Amblodon",
        Some("not of Rafinesque, 1819"),
        Some(Rank::Genus),
        Some(NomCode::Zoological),
    )
    .monomial_rank("Amblodon", Rank::Genus)
    .sensu("not of Rafinesque, 1819")
    .code(NomCode::Zoological)
    .nothing_else();
    assert_name_auth("Amblodon", "Agassiz, 1829 not of Rafinesque, 1819")
        .monomial("Amblodon")
        .comb_authors(Some("1829"), &["Agassiz"])
        .sensu("not of Rafinesque, 1819")
        .code(NomCode::Zoological)
        .nothing_else();
}

/// A bracketed homonym note before the year: `Lea (non Faust), 1913`. The note was read as part
/// of the author, "Lea non Faust".
#[test]
fn a_bracketed_note_before_the_year() {
    assert_name_auth("Mechistocerus similis", "Lea (non Faust), 1913")
        .species("Mechistocerus", "similis")
        .comb_authors(Some("1913"), &["Lea"])
        .sensu("non Faust")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_auth("Mechistocerus similis", "Lea [non Faust] 1913")
        .species("Mechistocerus", "similis")
        .comb_authors(Some("1913"), &["Lea"])
        .sensu("non Faust")
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_auth("Rhyssomatus parvulus", "Champion (nec Casey), 1902")
        .species("Rhyssomatus", "parvulus")
        .comb_authors(Some("1902"), &["Champion"])
        .sensu("nec Casey")
        .code(NomCode::Zoological)
        .nothing_else();
}

/// A lone "auct." with a stray comma is still the note, not an author "auctt.".
#[test]
fn auct_with_a_trailing_comma() {
    assert_name_auth("Arixyleborus rugosipes", "auctt.,")
        .species("Arixyleborus", "rugosipes")
        .sensu("auctt.")
        .nothing_else();
    assert_name_auth("Amblodon", "auct.,")
        .monomial("Amblodon")
        .sensu("auct.")
        .nothing_else();
}

/// A bracketed note opening with "lapsus" is a note as a whole: `[lapsus, non-existent
/// combination, not Kolmer, 1985]` gave the authors "lapsus" and "non-existent combination" and the
/// note `not Kolmer, 1985]`, half a bracket.
#[test]
fn a_bracketed_lapsus_note() {
    assert_name_auth(
        "Flabelligera biscayensis",
        "[lapsus, non-existent combination, not Kolmer, 1985]",
    )
    .species("Flabelligera", "biscayensis")
    .sensu("lapsus, non-existent combination, not Kolmer, 1985")
    .nothing_else();
}

/// A remark and then a homonym note, in one bracket: the bracket is the note.
#[test]
fn a_bracketed_remark_with_a_homonym_note() {
    assert_name_auth(
        "Anthicus elegans",
        "(Lea, 1895) [junior secondary homonym, nec Anthicus elegans Steven, 1806]",
    )
    .species("Anthicus", "elegans")
    .bas_authors(Some("1895"), &["Lea"])
    .sensu("junior secondary homonym, nec Anthicus elegans Steven, 1806")
    .code(NomCode::Zoological)
    .nothing_else();
}

/// Two alternative (sub)genera in the bracket after the genus are a note, never the basionym
/// author and never a subgenus.
#[test]
fn alternative_subgenera_in_brackets_are_a_note() {
    assert_name("Cyclostoma (Cyclophorus vel Leptopoma) thersites Shuttleworth 1852")
        .species("Cyclostoma", "thersites")
        .comb_authors(Some("1852"), &["Shuttleworth"])
        .sensu("Cyclophorus vel Leptopoma")
        .code(NomCode::Zoological)
        .nothing_else();
    // one word stays the subgenus
    assert_name("Cyclostoma (Cyclophorus) thersites Shuttleworth 1852")
        .species_ig("Cyclostoma", "Cyclophorus", "thersites")
        .comb_authors(Some("1852"), &["Shuttleworth"])
        .code(NomCode::Zoological)
        .nothing_else();
}
