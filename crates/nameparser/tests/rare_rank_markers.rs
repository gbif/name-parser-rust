// SPDX-License-Identifier: Apache-2.0
//! Rank markers rarer than subsp./var./f. and sect./subg.: they used to be read as an epithet or
//! end up inside the authorship.

mod common;
use common::*;
use nameparser::model::{warnings, NamePart, NomCode, Rank};

#[test]
fn rare_infraspecific_markers() {
    assert_name("Scilla vindobonensis lus. alba (Beck) Kereszty")
        .infra_species("Scilla", "vindobonensis", Rank::Lusus, "alba")
        .comb_authors(None, &["Kereszty"])
        .bas_authors(None, &["Beck"])
        .code(NomCode::Botanical)
        .nothing_else();
    // Sudre's microgène of Rubus
    assert_name("Rubus hirtus Rchb. microg pectinatus Sudre & Gavet")
        .infra_species("Rubus", "hirtus", Rank::InfraspecificName, "pectinatus")
        .comb_authors(None, &["Sudre", "Gavet"])
        .specific_authors(None, &["Rchb."])
        .nothing_else();
    // the "sssp." typo of "ssp."
    assert_name("Centaurea subciliaris Boiss. & Heldr. sssp. acarnanica Matthas")
        .infra_species("Centaurea", "subciliaris", Rank::Subspecies, "acarnanica")
        .comb_authors(None, &["Matthas"])
        .specific_authors(None, &["Boiss.", "Heldr."])
        .nothing_else();
    // a marker word where the epithet belongs is the epithet
    assert_name("Haliotis cracherodii var. lusus Finlay, 1927")
        .infra_species("Haliotis", "cracherodii", Rank::Variety, "lusus")
        .comb_authors(Some("1927"), &["Finlay"])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// `r.` is the race of older zoological literature (Oberthür, Forel, Verity), a geographic race
/// that is read as a subspecies today. It used to be taken for the infraspecific epithet, and the
/// real epithet for an author.
#[test]
fn race_marker() {
    assert_name("Lycaena rippertii r. fabressei Oberthür, 1910")
        .infra_species("Lycaena", "rippertii", Rank::Subspecies, "fabressei")
        .comb_authors(Some("1910"), &["Oberthür"])
        .code(NomCode::Zoological)
        .nothing_else();
    // written out, too: Pic's beetles, and the races of French floras (a proles in botany)
    assert_name("Leptura erratica race bottcheri Pic, 1911")
        .infra_species("Leptura", "erratica", Rank::Subspecies, "bottcheri")
        .comb_authors(Some("1911"), &["Pic"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Zygorhynchus vuilleminii race agamus Namysl.")
        .infra_species(
            "Zygorhynchus",
            "vuilleminii",
            Rank::InfraspecificName,
            "agamus",
        )
        .comb_authors(None, &["Namysl."])
        .nothing_else();
    // the numbered physiological race of a plant pathogen is no rank
    assert_name("Fusarium oxysporum f. sp. cubense race 4")
        .infra_species("Fusarium", "oxysporum", Rank::FormaSpecialis, "cubense")
        .code(NomCode::Botanical)
        .partial("race")
        .nothing_else();
    // with no epithet after it, "r." ends an abbreviated author name
    assert_name("Rubus deznensis Ny r.")
        .species("Rubus", "deznensis")
        .comb_authors(None, &["Ny r."])
        .nothing_else();
    // or, before an author, is the epithet itself, abbreviated like the repeated one of a
    // nominotypical subspecies
    assert_name("Diacamma rugosum r. Karavajev & Karawajew 1935")
        .infra_species("Diacamma", "rugosum", Rank::Subspecies, "r")
        .comb_authors(Some("1935"), &["Karavajev", "Karawajew"])
        .code(NomCode::Zoological)
        .nothing_else();
}

/// Markers below the subspecies in older literature: `m.` (morpha, monstrositas or modificatio),
/// `monstr.`, `mod.`, `morpha`. `m.` used to be glued onto the species author as an initial or
/// read as the epithet, the real epithet as an author. A zoological `m.` is the morpha.
#[test]
fn infrasubspecific_markers_of_older_literature() {
    // the stray "?." is dropped
    assert_name("Phalaris canariensis L. m. ?. bracteata Jansen & Wacht.")
        .infra_species(
            "Phalaris",
            "canariensis",
            Rank::InfrasubspecificName,
            "bracteata",
        )
        .specific_authors(None, &["L."])
        .comb_authors(None, &["Jansen", "Wacht."])
        .nothing_else();
    // Breuning's morphae: under the zoological code "m." is the morpha
    assert_name("Eunidia simplex m. bifuscomaculata Breuning, 1957")
        .infra_species("Eunidia", "simplex", Rank::Morph, "bifuscomaculata")
        .comb_authors(Some("1957"), &["Breuning"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name_code("Proetus midas m. elongatus", NomCode::Zoological)
        .infra_species("Proetus", "midas", Rank::Morph, "elongatus")
        .code(NomCode::Zoological)
        .nothing_else();
    // with no code it stays the shared rank
    assert_name("Proetus midas m. elongatus")
        .infra_species(
            "Proetus",
            "midas",
            Rank::InfrasubspecificName,
            "elongatus",
        )
        .nothing_else();
    assert_name("Aquilegia vulgaris monstr. corniculata (Vill.) Graebn. & P.Graebn.")
        .infra_species(
            "Aquilegia",
            "vulgaris",
            Rank::InfrasubspecificName,
            "corniculata",
        )
        .comb_authors(None, &["Graebn.", "P.Graebn."])
        .bas_authors(None, &["Vill."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Cladonia gracilis mod. albinea Sandst.")
        .infra_species(
            "Cladonia",
            "gracilis",
            Rank::InfrasubspecificName,
            "albinea",
        )
        .comb_authors(None, &["Sandst."])
        .nothing_else();
    assert_name("Arvicola terrestris morpha subalpina")
        .infra_species("Arvicola", "terrestris", Rank::Morph, "subalpina")
        .code(NomCode::Zoological)
        .nothing_else();
    // a capital M. stays an initial
    assert_name("Abies alba M. Smith")
        .species("Abies", "alba")
        .comb_authors(None, &["M.Smith"])
        .nothing_else();
}

#[test]
fn rare_infrageneric_markers() {
    // Fries's tribus within a genus
    assert_name("Agaricus tr. Hypholoma Fr.")
        .infrageneric_at("Agaricus", Rank::InfragenericName, "Hypholoma")
        .comb_authors(None, &["Fr."])
        .nothing_else();
    assert_name("Hieracium unr. Verbasciformia Arv.-Touv.")
        .infrageneric_at("Hieracium", Rank::InfragenericName, "Verbasciformia")
        .comb_authors(None, &["Arv.-Touv."])
        .nothing_else();
    // a notho marker after the genus author
    assert_name("Aconitum W. Mucher nothosect. Acopellus")
        .infrageneric_at("Aconitum", Rank::SectionBotany, "Acopellus")
        .notho(&[NamePart::Infrageneric])
        .generic_authors(None, &["W.Mucher"])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn an_unranked_marker_marks_an_unranked_name() {
    // #98: IPNI's "[unranked]" put the epithet into the authorship
    assert_name("Aetheorhiza bulbosa [unranked] montana (Willk.) Gand.")
        .infra_species("Aetheorhiza", "bulbosa", Rank::InfraspecificName, "montana")
        .bas_authors(None, &["Willk."])
        .comb_authors(None, &["Gand."])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Viola [unranked] Mirabiles Nyman")
        .infrageneric_at("Viola", Rank::InfragenericName, "Mirabiles")
        .comb_authors(None, &["Nyman"])
        .nothing_else();
    assert_name_auth("Abelia unranked Biflorae", "Zabel")
        .infrageneric_at("Abelia", Rank::InfragenericName, "Biflorae")
        .comb_authors(None, &["Zabel"])
        .nothing_else();
    assert_name("Poa (unranked) Arctophila")
        .infrageneric_at("Poa", Rank::InfragenericName, "Arctophila")
        .nothing_else();
    // the marker the formatter writes for it reads back
    assert_name("Bromus hordeaceus infrasp. ferronii (Mabille) P.M.Sm.")
        .infra_species("Bromus", "hordeaceus", Rank::InfraspecificName, "ferronii")
        .bas_authors(None, &["Mabille"])
        .comb_authors(None, &["P.M.Sm."])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn an_infrageneric_marker_inside_the_brackets() {
    // #88: the epithet was lost, the bracket read as the basionym author "sect.Auriculella"
    assert_name("Achatinella (sect. Auriculella) Pfeiffer 1854")
        .infrageneric_at("Achatinella", Rank::SectionBotany, "Auriculella")
        .comb_authors(Some("1854"), &["Pfeiffer"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Zygopetalum (sect. Cheiradenia)", "(Lindl.) Kuntze")
        .infrageneric_at("Zygopetalum", Rank::SectionBotany, "Cheiradenia")
        .bas_authors(None, &["Lindl."])
        .comb_authors(None, &["Kuntze"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name_auth("Begonia (sect. Diploclinium) labordei", "H. Lev.")
        .species_ig("Begonia", "Diploclinium", "labordei")
        .comb_authors(None, &["H.Lev."])
        .nothing_else();
}

#[test]
fn a_superspecies_is_a_species_aggregate() {
    // #87: the marker became the epithet, and all superspecies of a genus one name
    assert_name_auth("Eosembia supersp. thoracica", "(Ross, 2007)")
        .binomial("Eosembia", None, "thoracica", Rank::SpeciesAggregate)
        .bas_authors(Some("2007"), &["Ross"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Acestrura heliodor superspecies")
        .binomial("Acestrura", None, "heliodor", Rank::SpeciesAggregate)
        .nothing_else();
}

#[test]
fn a_lowercase_infrageneric_epithet_after_its_marker_is_capitalised() {
    // #89: it became a species epithet
    assert_name("Hygrocybe sect. obtusae (A.H. Sm. & Hesler) Bon")
        .infrageneric_at("Hygrocybe", Rank::SectionBotany, "Obtusae")
        .bas_authors(None, &["A.H.Sm.", "Hesler"])
        .comb_authors(None, &["Bon"])
        .code(NomCode::Botanical)
        .nothing_else();
    assert_name("Limacinia subgen limacinula Saccardo & D.Saccardo")
        .infrageneric_at("Limacinia", Rank::Subgenus, "Limacinula")
        .comb_authors(None, &["Saccardo", "D.Saccardo"])
        .nothing_else();
    // an author abbreviated like a marker is none
    assert_name("Pocockia Ser. ex DC.")
        .monomial("Pocockia")
        .comb_authors(None, &["DC."])
        .comb_ex_authors(&["Ser."])
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn a_greek_letter_in_brackets_numbers_an_infraspecific_name() {
    // #90: written out in brackets, the letter dropped the epithet; as β it was understood
    assert_name_auth("Achnanthes brevipes [beta] salina", "Kützing")
        .infra_species("Achnanthes", "brevipes", Rank::InfraspecificName, "salina")
        .comb_authors(None, &["Kützing"])
        .nothing_else();
    assert_name_auth(
        "Batrachospermum moniliforme (epsilon)viridis",
        "(Bory) J.E. Duby",
    )
    .infra_species(
        "Batrachospermum",
        "moniliforme",
        Rank::InfraspecificName,
        "viridis",
    )
    .bas_authors(None, &["Bory"])
    .comb_authors(None, &["J.E.Duby"])
    .code(NomCode::Botanical)
    .nothing_else();
    // after a rank marker it is a mere label
    assert_name_auth("Epithemia gibba var. .(gamma)parallela", "Grunow")
        .infra_species("Epithemia", "gibba", Rank::Variety, "parallela")
        .comb_authors(None, &["Grunow"])
        .nothing_else();
}

#[test]
fn a_lowercase_cultivar_and_an_abbreviated_cultivar_group() {
    // #92: "cv" became the infraspecific epithet, "Queen Gp" a basionym author
    assert_name_auth("Vanilla planifolia cv. tahiti", "Costantin & Bois, 1916")
        .cultivar_sp("Vanilla", "planifolia", "Tahiti")
        .comb_authors(Some("1916"), &["Costantin", "Bois"])
        .nothing_else();
    assert_name("Lactuca sativa (Capitata Gp)")
        .cultivar_sp_rank("Lactuca", "sativa", Rank::CultivarGroup, "Capitata")
        .nothing_else();
}

#[test]
fn an_unquoted_cultivar_epithet() {
    // #118: "cv" became the infraspecific epithet, the cultivar epithet its author
    assert_name("Abies alba cv. Orfei")
        .cultivar_sp("Abies", "alba", "Orfei")
        .nothing_else();
    assert_name("Abies alba, cv. Orfei")
        .cultivar_sp("Abies", "alba", "Orfei")
        .nothing_else();
    assert_name("Origanum laevigatum cv Aurea")
        .cultivar_sp("Origanum", "laevigatum", "Aurea")
        .nothing_else();
    // it may run over several words
    assert_name("Triticum dicoccum cv. Large White Emmer")
        .cultivar_sp("Triticum", "dicoccum", "Large White Emmer")
        .nothing_else();
    assert_name("Saccharum officinarum cultivar CP 96-1252")
        .cultivar_sp("Saccharum", "officinarum", "CP 96-1252")
        .nothing_else();
    // the author's first initial ends it
    assert_name("Bambusa textilis cv. Maculata L. C. Chia")
        .cultivar_sp("Bambusa", "textilis", "Maculata")
        .comb_authors(None, &["L.C.Chia"])
        .nothing_else();
    assert_name("Bambusa multiplex cv. Golden Goddess L.C.Chia & C.Y.Sia")
        .cultivar_sp("Bambusa", "multiplex", "Golden Goddess")
        .comb_authors(None, &["L.C.Chia", "C.Y.Sia"])
        .nothing_else();
    // or a bracketed basionym author
    assert_name("Weigela praecox cv. Floreal (Lemoine) L.H.Bailey")
        .cultivar_sp("Weigela", "praecox", "Floreal")
        .bas_authors(None, &["Lemoine"])
        .comb_authors(None, &["L.H.Bailey"])
        .nothing_else();
    // and Linnaeus and de Candolle, abbreviated without initials
    assert_name("Abies alba cv. Orfei L.")
        .cultivar_sp("Abies", "alba", "Orfei")
        .comb_authors(None, &["L."])
        .nothing_else();
    assert_name("Abies alba cv. Orfei DC.")
        .cultivar_sp("Abies", "alba", "Orfei")
        .comb_authors(None, &["DC."])
        .nothing_else();
    assert_name("Abies alba cv. Large Orfei L. ex DC.")
        .cultivar_sp("Abies", "alba", "Large Orfei")
        .comb_authors(None, &["DC."])
        .comb_ex_authors(&["L."])
        .nothing_else();
}

/// #93: a stirps with a capitalised epithet, as MycoBank has Bidaud et al.'s Cortinarius atlas,
/// is an unranked infrageneric name; the model has no stirps rank.
#[test]
fn a_stirps_with_a_capitalised_epithet_is_an_unranked_infrageneric_name() {
    assert_name("Cortinarius stirps Balteatus (Bidaud, Moënne-Loccoz & Reumaux) Bidaud, Moënne-Loccoz & Reumaux")
        .infrageneric_at("Cortinarius", Rank::InfragenericName, "Balteatus")
        .bas_authors(None, &["Bidaud", "Moënne-Loccoz", "Reumaux"])
        .comb_authors(None, &["Bidaud", "Moënne-Loccoz", "Reumaux"])
        .code(NomCode::Botanical)
        .nothing_else();
    // ChecklistBank passes the rank stirps as PROLES; the marker wins
    assert_name_hinted(
        "Cortinarius stirps Caesiostraminei",
        None,
        Some(Rank::Proles),
        Some(NomCode::Botanical),
    )
    .infrageneric_at("Cortinarius", Rank::InfragenericName, "Caesiostraminei")
    .code(NomCode::Botanical)
    .nothing_else();
    // the species stirps and its author
    assert_name("Astraea stirps Laws, 1932")
        .species("Astraea", "stirps")
        .comb_authors(Some("1932"), &["Laws"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn ser_is_a_serovar_in_salmonella_and_under_the_bacterial_code() {
    // "ser." abbreviates the serovar of Salmonella, by far the largest genus of serovars (Brenner
    // et al. 2000, Salmonella nomenclature), not the botanical series
    assert_name("Salmonella enterica ser. Typhimurium")
        .infra_species("Salmonella", "enterica", Rank::Serovar, "Typhimurium")
        .code(NomCode::Bacterial)
        .nothing_else();
    assert_name("Salmonella enterica subsp. enterica ser. Typhimurium")
        .infra_species("Salmonella", "enterica", Rank::Serovar, "Typhimurium")
        .code(NomCode::Bacterial)
        .warning(&["Removed: subsp. enterica", "name was quadrinomial"])
        .nothing_else();
    // a serovar of the genus, as the short form "Salmonella Typhimurium" means
    assert_name("Salmonella ser. Typhimurium")
        .infra_under_genus("Salmonella", Rank::Serovar, "Typhimurium")
        .code(NomCode::Bacterial)
        .nothing_else();
    assert_name("Salmonella serovar Typhimurium")
        .infra_under_genus("Salmonella", Rank::Serovar, "Typhimurium")
        .code(NomCode::Bacterial)
        .nothing_else();
    // under a bacterial code hint in any genus
    assert_name_code("Leptospira interrogans ser. Pomona", NomCode::Bacterial)
        .infra_species("Leptospira", "interrogans", Rank::Serovar, "Pomona")
        .code(NomCode::Bacterial)
        .nothing_else();
    assert_name_code("Salmonella ser. Typhimurium", NomCode::Bacterial)
        .infra_under_genus("Salmonella", Rank::Serovar, "Typhimurium")
        .code(NomCode::Bacterial)
        .nothing_else();
    // a series elsewhere
    assert_name("Carex ser. Acutae")
        .infrageneric_at("Carex", Rank::SeriesBotany, "Acutae")
        .code(NomCode::Botanical)
        .nothing_else();
}

#[test]
fn a_misspelt_rank_marker_is_the_marker_it_is_one_letter_off() {
    // #115: "va." (Index Nominum Algarum) became the epithet and the epithet an author, or was lost
    // with the authorship given separately
    assert_name("Aulacodiscus margaritaceus va. distincta J. Rattray")
        .infra_species("Aulacodiscus", "margaritaceus", Rank::Variety, "distincta")
        .comb_authors(None, &["J.Rattray"])
        .nothing_else();
    assert_name_auth("Aulacodiscus spectabilis va. depressa", "J. Rattray")
        .infra_species("Aulacodiscus", "spectabilis", Rank::Variety, "depressa")
        .comb_authors(None, &["J.Rattray"])
        .nothing_else();
    assert_name("Campanula balcanica subfo. umbrosa Hruby")
        .infra_species("Campanula", "balcanica", Rank::Subform, "umbrosa")
        .comb_authors(None, &["Hruby"])
        .nothing_else();
    assert_name("Unio kungurensis varr. chlebnikowi")
        .infra_species("Unio", "kungurensis", Rank::Variety, "chlebnikowi")
        .nothing_else();
    assert_name("Hieracium incisum subs. sprecheri Zahn")
        .infra_species("Hieracium", "incisum", Rank::Subspecies, "sprecheri")
        .comb_authors(None, &["Zahn"])
        .nothing_else();
    // without its dot a word may be an epithet
    assert_name("Zacla va")
        .species("Zacla", "va")
        .nothing_else();
}

#[test]
fn a_cultivar_marker_without_its_epithet_is_an_indetermined_cultivar() {
    // #118: `cv` or `cultivar` became the epithet, or an author
    assert_name("Acacia latifolia cv.")
        .indet("Acacia", "latifolia", Rank::Cultivar)
        .code(NomCode::Cultivars)
        .warning(&[warnings::INDETERMINED])
        .nothing_else();
    assert_informal("Hibiscus cv.")
        .taxon("Hibiscus")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Cultivar)
        .no_phrase()
        .code(NomCode::Cultivars)
        .nothing_else();
    // `hybrid` is the epithet, as in "Rosa hybrid 'Peace'", unless there is one
    assert_name("Rosa hybrid cultivar")
        .indet("Rosa", "hybrid", Rank::Cultivar)
        .code(NomCode::Cultivars)
        .warning(&[warnings::INDETERMINED])
        .nothing_else();
    assert_name("Aspidistra mushaensis hybrid cultivar")
        .indet("Aspidistra", "mushaensis", Rank::Cultivar)
        .code(NomCode::Cultivars)
        .warning(&[warnings::INDETERMINED])
        .nothing_else();
}

#[test]
fn a_lower_case_cultivar_epithet_of_several_words() {
    // #118: only the last word was the epithet, of the rank "cv"
    assert_name("Vanilla planifolia cv. oreja de burro")
        .cultivar_sp("Vanilla", "planifolia", "Oreja de Burro")
        .nothing_else();
    assert_name("Phyllostachys viridis cv robert young McClure")
        .cultivar_sp("Phyllostachys", "viridis", "Robert Young")
        .comb_authors(None, &["McClure"])
        .nothing_else();
    assert_name("Bambusa blumeana cv wei-fang lin (W.C. Lin) Chia")
        .cultivar_sp("Bambusa", "blumeana", "Wei-fang Lin")
        .bas_authors(None, &["W.C.Lin"])
        .comb_authors(None, &["Chia"])
        .nothing_else();
    // an author's particle stays with the author
    assert_name("Abies alba cv. alba de Vries")
        .cultivar_sp("Abies", "alba", "Alba")
        .comb_authors(None, &["de Vries"])
        .nothing_else();
    // a quoted epithet before a bracketed author
    assert_name("Cyathea cooperi cv. 'Highland Lace' (Hook. ex F.Muell.) Domin")
        .cultivar_sp("Cyathea", "cooperi", "Highland Lace")
        .bas_authors(None, &["F.Muell."])
        .bas_ex_authors(None, &["Hook."])
        .comb_authors(None, &["Domin"])
        .nothing_else();
}

#[test]
fn misspelt_cultivar_markers() {
    // #118: the plural, a doubled dot or no space after the marker
    assert_name("Malva verticillata cvs. Ikva")
        .cultivar_sp("Malva", "verticillata", "Ikva")
        .nothing_else();
    assert_name("Cyperus esculentus cv. s. Faraon-FKO")
        .cultivar_sp("Cyperus", "esculentus", "Faraon-FKO")
        .nothing_else();
    assert_name("Cotoneaster horizontales cv. . Hodginsii")
        .cultivar_sp("Cotoneaster", "horizontales", "Hodginsii")
        .nothing_else();
    assert_name("Arachis hypogaea cv.Tigrovyi")
        .cultivar_sp("Arachis", "hypogaea", "Tigrovyi")
        .nothing_else();
    // a Cultivar Group in lower case
    assert_name("Oryza sativa (japonica cultivar-group)")
        .cultivar_sp_rank("Oryza", "sativa", Rank::CultivarGroup, "Japonica")
        .nothing_else();
    assert_name("Oryza sativa tropical japonica cultivar-group")
        .cultivar_sp_rank("Oryza", "sativa", Rank::CultivarGroup, "Tropical Japonica")
        .nothing_else();
    assert_name("Cucumis sativus cv.-gr. pickling cucumber")
        .cultivar_sp_rank(
            "Cucumis",
            "sativus",
            Rank::CultivarGroup,
            "Pickling Cucumber",
        )
        .nothing_else();
}
