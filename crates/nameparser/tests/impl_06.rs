// SPDX-License-Identifier: Apache-2.0
//! Ported from Java NameParserImplTest (methods on lines 2410-2970).
mod common;
use common::*;
use nameparser::model::warnings;
use nameparser::model::{NamePart, NameType, NomCode, Rank};

/// https://github.com/gbif/name-parser/issues/49
#[test]
fn unparsable_authors() {
    assert_authorship("Allemão", &[])
        .comb_authors(None, &["Allemão"])
        .nothing_else();
    // the lost ex-author: on the name string too, "ex" is no epithet
    assert_authorship("ex DC.", &["DC."])
        .comb_authors(None, &["DC."])
        .nothing_else();
}

#[test]
fn extinct_names() {
    assert_name("Sicyoniidae † Ortmann, 1898")
        .monomial("Sicyoniidae")
        .comb_authors(Some("1898"), &["Ortmann"])
        .extinct()
        .code(NomCode::Zoological)
        .nothing_else();

    assert_name("†Titanoptera")
        .monomial("Titanoptera")
        .extinct()
        .nothing_else();

    assert_name("†††Titanoptera")
        .monomial("Titanoptera")
        .extinct()
        .nothing_else();

    assert_name("† Tuarangiida MacKinnon, 1982")
        .monomial("Tuarangiida")
        .comb_authors(Some("1982"), &["MacKinnon"])
        .extinct()
        .code(NomCode::Zoological)
        .nothing_else();
}

// namesWithAuthorFile, otherFile, hybridsFile, placeholderFile: see corpus_files.rs.

/// Expect empty unparsable results for nothing or whitespace
#[test]
fn empty() {
    // Java `assertNoName(null)` has no Rust equivalent — `&str` cannot represent a null
    // reference; the immediately-following empty-string case exercises the same input-less path.
    assert_no_name("");
    assert_no_name(" ");
    assert_no_name("\t");
    assert_no_name("\n");
    assert_no_name("\t\n");
    assert_no_name("\"");
    assert_no_name("'");
}

/// Avoid nPEs and other exceptions for very short non names and other extremes found in occurrences.
#[test]
fn avoid_npe() {
    // https://github.com/gbif/portal-feedback/issues/5326#issuecomment-2107283007
    assert_name("Foa fo")
        .species("Foa", "fo")
        .type_(NameType::Scientific)
        .nothing_else();

    assert_no_name("\\");
    assert_no_name(".");
    assert_no_name("@");
    assert_no_name("&nbsp;");
    assert_no_name("X");
    assert_no_name("a");
    assert_no_name("-,.#");
    assert_no_name(" .");
}

#[test]
fn informal() {
    assert_name("Trisulcus aff. nana  (Popofsky, 1913), Petrushevskaya, 1971")
        .species("Trisulcus", "nana")
        .bas_authors(Some("1913"), &["Popofsky"])
        .comb_authors(Some("1971"), &["Petrushevskaya"])
        .type_(NameType::Informal)
        .qualifiers(&[(NamePart::Specific, "aff.")])
        .nothing_else();

    assert_name("Cerapachys mayeri cf. var. brachynodus")
        .infra_species("Cerapachys", "mayeri", Rank::Variety, "brachynodus")
        .type_(NameType::Informal)
        .qualifiers(&[(NamePart::Infraspecific, "cf.")])
        .nothing_else();

    assert_name("Solenopsis cf fugax")
        .species("Solenopsis", "fugax")
        .type_(NameType::Informal)
        .qualifiers(&[(NamePart::Specific, "cf.")])
        .nothing_else();
}

#[test]
fn abbreviated() {
    assert_name("N. giraldo")
        .species("N.", "giraldo")
        .type_(NameType::Informal)
        .warning(&[warnings::ABBREVIATED_GENUS])
        .nothing_else();

    // 5.0.0: a bare abbreviated genus with no epithet is a supraspecific anchor with no designation
    // → an Informal result. The ABBREVIATED_GENUS warning the raw ParsedName carried is not part of
    // the lean Informal type (taxon/taxonRank/rank/phrase/code); the "N. giraldo" case above still
    // pins that warning on a Parsed name.
    assert_informal("B.")
        .taxon("B.")
        .taxon_rank(Rank::Unranked)
        .rank(Rank::Unranked)
        .no_phrase()
        .nothing_else();
}

#[test]
fn string_index_out_of_bounds_exception() {
    assert_name("Amblyomma americanum (Linnaeus, 1758)")
        .species("Amblyomma", "americanum")
        .bas_authors(Some("1758"), &["Linnaeus"])
        .code(NomCode::Zoological)
        .nothing_else();
    assert_name("Salix taiwanalpina var. chingshuishanensis (S.S.Ying) F.Y.Lu, C.H.Ou, Y.C.Chen, Y.S.Chi, K.C.Lu & Y.H.Tseng ")
        .infra_species("Salix", "taiwanalpina", Rank::Variety, "chingshuishanensis")
        .comb_authors(None, &["F.Y.Lu", "C.H.Ou", "Y.C.Chen", "Y.S.Chi", "K.C.Lu", "Y.H.Tseng"])
        .bas_authors(None, &["S.S.Ying"])
        .code(NomCode::Botanical)
        .nothing_else();
    // FIXME(review): "& amp" is an HTML entity residue, not part of an author
    assert_name("Salix taiwanalpina var. chingshuishanensis (S.S.Ying) F.Y.Lu, C.H.Ou, Y.C.Chen, Y.S.Chi, K.C.Lu & amp  Y.H.Tseng ")
        .infra_species("Salix", "taiwanalpina", Rank::Variety, "chingshuishanensis")
        .comb_authors(None, &["F.Y.Lu", "C.H.Ou", "Y.C.Chen", "Y.S.Chi", "K.C.Lu", "amp Y.H.Tseng"])
        .bas_authors(None, &["S.S.Ying"])
        .code(NomCode::Botanical)
        .nothing_else();
    // FIXME(review): "& amp;" is an HTML entity residue, not an author
    assert_name("Salix morrisonicola var. takasagoalpina (Koidz.) F.Y.Lu, C.H.Ou, Y.C.Chen, Y.S.Chi, K.C.Lu & amp; Y.H.Tseng")
        .infra_species("Salix", "morrisonicola", Rank::Variety, "takasagoalpina")
        .comb_authors(None, &["F.Y.Lu", "C.H.Ou", "Y.C.Chen", "Y.S.Chi", "K.C.Lu", "amp", "Y.H.Tseng"])
        .bas_authors(None, &["Koidz."])
        .code(NomCode::Botanical)
        .nothing_else();
    // FIXME(review): "& amp;" is an HTML entity residue, not an author
    assert_name(
        "Ficus ernanii Carauta, Pederneir., P.P.Souza, A.F.P.Machado, M.D.M.Vianna & amp; Romaniuc",
    )
    .species("Ficus", "ernanii")
    .comb_authors(
        None,
        &[
            "Carauta",
            "Pederneir.",
            "P.P.Souza",
            "A.F.P.Machado",
            "M.D.M.Vianna",
            "amp",
            "Romaniuc",
        ],
    )
    .nothing_else();
}

#[test]
fn nom_notes() {
    assert_name("Anthurium lanceum Engl., nom. illeg., non. A. lancea.")
        .species("Anthurium", "lanceum")
        .comb_authors(None, &["Engl."])
        .nom_note("nom. illeg.")
        .sensu("non. A.lancea.")
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Combretum Loefl. (1758), nom. cons. [= Grislea L. 1753].")
        .monomial("Combretum")
        .comb_authors(Some("1758"), &["Loefl."])
        .nom_note("nom. cons.")
        .doubtful()
        .partial("[= Grislea L. 1753].")
        .code(NomCode::Botanical)
        .nothing_else();

    assert_name("Anthurium lanceum Engl. nom.illeg.")
        .species("Anthurium", "lanceum")
        .comb_authors(None, &["Engl."])
        .nom_note("nom. illeg.")
        .code(NomCode::Botanical)
        .nothing_else();
}

/// A code-exclusive nomenclatural status settles the code even without any other cue, and
/// even against a year that would otherwise read as a zoological author-year. Statuses that
/// exist in both codes (nom. nud., etc.) stay code-neutral.
#[test]
fn nom_note_code() {
    // ICN-only status → BOTANICAL, overriding the year → zoological heuristic
    assert_name("Polygala vulgaris L., 1753, nom. cons.")
        .species("Polygala", "vulgaris")
        .comb_authors(Some("1753"), &["L."])
        .nom_note("nom. cons.")
        .code(NomCode::Botanical)
        .nothing_else();
    // ICZN-only status → ZOOLOGICAL, with no year present
    assert_name("Aus bus Smith, nomen oblitum")
        .species("Aus", "bus")
        .comb_authors(None, &["Smith"])
        .nom_note("nomen oblitum")
        .code(NomCode::Zoological)
        .nothing_else();
    // shared status carries no code signal
    assert_name("Aus bus Smith, nom. nud.")
        .species("Aus", "bus")
        .comb_authors(None, &["Smith"])
        .nom_note("nom. nud.")
        .nothing_else();
}

/// HTML tags and entities are stripped/decoded, each flagged with its own warning.
#[test]
fn html_tags_and_entities() {
    // tag only
    assert_name("<i>Abies alba</i> Mill.")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill."])
        .warning(&[warnings::XML_TAGS])
        .nothing_else();
    // entity only
    assert_name("Abies alba Mill. &amp; Rohe")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill.", "Rohe"])
        .warning(&[warnings::HTML_ENTITIES])
        .nothing_else();
    // both a tag and an entity
    assert_name("<i>Abies alba</i> Mill. &amp; Rohe")
        .species("Abies", "alba")
        .comb_authors(None, &["Mill.", "Rohe"])
        .warning(&[warnings::XML_TAGS, warnings::HTML_ENTITIES])
        .nothing_else();
}

/// Open-nomenclature uncertainty in the authorship is flagged doubtful.
#[test]
fn uncertain_authorship() {
    // trailing standalone "?" — dropped, name flagged doubtful
    assert_name("Uroleptopsis viridis (Perejaslawzewa, 1886) ?")
        .species("Uroleptopsis", "viridis")
        .bas_authors(Some("1886"), &["Perejaslawzewa"])
        .code(NomCode::Zoological)
        .doubtful()
        .warning(&[warnings::QUESTION_MARKS_REMOVED])
        .nothing_else();
    // "?" glued to a trailing author
    assert_name("Abies alba Smith?")
        .species("Abies", "alba")
        .comb_authors(None, &["Smith"])
        .doubtful()
        .warning(&[warnings::UNCERTAIN_AUTHORSHIP])
        .nothing_else();
    // alternative authors joined by "/" — the slash is retained in the author string
    assert_name("Abies alba Smith/Jones")
        .species("Abies", "alba")
        .comb_authors(None, &["Smith/Jones"])
        .doubtful()
        .warning(&[warnings::UNCERTAIN_AUTHORSHIP])
        .nothing_else();
}

/// A comb-author list carrying a "de" particle must not be mistaken for a publishedIn ref.
#[test]
fn author_list_with_particle() {
    assert_name("Leptographium conplurium M.L. Yin, Z.W. de Beer & M.J. Wingf.")
        .species("Leptographium", "conplurium")
        .comb_authors(None, &["M.L.Yin", "Z.W.de Beer", "M.J.Wingf."])
        .nothing_else();
}

#[test]
fn test_authorteam() {
    assert_authorship("Jarocki or Schinz", &["Jarocki or Schinz"])
        .comb_authors(None, &["Jarocki or Schinz"])
        .doubtful()
        .warning(&[warnings::UNCERTAIN_AUTHORSHIP])
        .nothing_else();
    assert_authorship("van der Wulp", &["van der Wulp"])
        .comb_authors(None, &["van der Wulp"])
        .nothing_else();
    assert_authorship(
        "Balsamo M Fregni E Tongiorgi MA",
        &["M.Balsamo", "E.Fregni", "M.A.Tongiorgi"],
    )
    .comb_authors(None, &["M.Balsamo", "E.Fregni", "M.A.Tongiorgi"])
    .nothing_else();
    assert_authorship("Walker, F.", &["F.Walker"])
        .comb_authors(None, &["F.Walker"])
        .nothing_else();
    assert_authorship("Walker, F", &["F.Walker"])
        .comb_authors(None, &["F.Walker"])
        .nothing_else();
    assert_authorship("Walker F", &["F.Walker"])
        .comb_authors(None, &["F.Walker"])
        .nothing_else();
    assert_authorship("YJ Wang & ZQ Liu", &["YJ Wang", "ZQ Liu"])
        .comb_authors(None, &["YJ Wang", "ZQ Liu"])
        .nothing_else();
    assert_authorship("Y.-j. Wang & Z.-q. Liu", &["Y.-j.Wang", "Z.-q.Liu"])
        .comb_authors(None, &["Y.-j.Wang", "Z.-q.Liu"])
        .nothing_else();
    assert_authorship("Petzold & G.Kirchn.", &["Petzold", "G.Kirchn."])
        .comb_authors(None, &["Petzold", "G.Kirchn."])
        .nothing_else();
    assert_authorship(
        "Britton, Sterns, & Poggenb.",
        &["Britton", "Sterns", "Poggenb."],
    )
    .comb_authors(None, &["Britton", "Sterns", "Poggenb."])
    .nothing_else();
    assert_authorship("Van Heurck & Müll. Arg.", &["Van Heurck", "Müll.Arg."])
        .comb_authors(None, &["Van Heurck", "Müll.Arg."])
        .nothing_else();
    assert_authorship("Gruber-Vodicka", &["Gruber-Vodicka"])
        .comb_authors(None, &["Gruber-Vodicka"])
        .nothing_else();
    assert_authorship("Gruber-Vodicka et al.", &["Gruber-Vodicka", "al."])
        .comb_authors(None, &["Gruber-Vodicka", "al."])
        .nothing_else();
    assert_single_author("L.")
        .comb_authors(None, &["L."])
        .nothing_else();
    assert_single_author("Lin.")
        .comb_authors(None, &["Lin."])
        .nothing_else();
    assert_single_author("Linné")
        .comb_authors(None, &["Linné"])
        .nothing_else();
    assert_single_author("DC.")
        .comb_authors(None, &["DC."])
        .nothing_else();
    assert_single_author("de Chaudoir")
        .comb_authors(None, &["de Chaudoir"])
        .nothing_else();
    assert_single_author("Hilaire")
        .comb_authors(None, &["Hilaire"])
        .nothing_else();
    assert_authorship("St. Hilaire", &["St.Hilaire"])
        .comb_authors(None, &["St.Hilaire"])
        .nothing_else();
    assert_authorship("Geoffroy St. Hilaire", &["Geoffroy St.Hilaire"])
        .comb_authors(None, &["Geoffroy St.Hilaire"])
        .nothing_else();
    assert_single_author("Acev.-Rodr.")
        .comb_authors(None, &["Acev.-Rodr."])
        .nothing_else();
    assert_authorship(
        "Steyerm., Aristeg. & Wurdack",
        &["Steyerm.", "Aristeg.", "Wurdack"],
    )
    .comb_authors(None, &["Steyerm.", "Aristeg.", "Wurdack"])
    .nothing_else();
    assert_authorship("Du Puy & Labat", &["Du Puy", "Labat"])
        .comb_authors(None, &["Du Puy", "Labat"])
        .nothing_else();
    assert_single_author("Baum.-Bod.")
        .comb_authors(None, &["Baum.-Bod."])
        .nothing_else();
    assert_authorship("Engl. & v. Brehmer", &["Engl.", "v.Brehmer"])
        .comb_authors(None, &["Engl.", "v.Brehmer"])
        .nothing_else();
    assert_authorship("F. v. Muell.", &["F.v.Muell."])
        .comb_authors(None, &["F.v.Muell."])
        .nothing_else();
    assert_authorship("W.J.de Wilde & Duyfjes", &["W.J.de Wilde", "Duyfjes"])
        .comb_authors(None, &["W.J.de Wilde", "Duyfjes"])
        .nothing_else();
    assert_single_author("C.E.M.Bicudo")
        .comb_authors(None, &["C.E.M.Bicudo"])
        .nothing_else();
    assert_single_author("Alves-da-Silva")
        .comb_authors(None, &["Alves-da-Silva"])
        .nothing_else();
    assert_authorship(
        "Alves-da-Silva & C.E.M.Bicudo",
        &["Alves-da-Silva", "C.E.M.Bicudo"],
    )
    .comb_authors(None, &["Alves-da-Silva", "C.E.M.Bicudo"])
    .nothing_else();
    assert_single_author("Kingdon-Ward")
        .comb_authors(None, &["Kingdon-Ward"])
        .nothing_else();
    assert_authorship("Merr. & L.M.Perry", &["Merr.", "L.M.Perry"])
        .comb_authors(None, &["Merr.", "L.M.Perry"])
        .nothing_else();
    assert_authorship(
        "Calat., Nav.-Ros. & Hafellner",
        &["Calat.", "Nav.-Ros.", "Hafellner"],
    )
    .comb_authors(None, &["Calat.", "Nav.-Ros.", "Hafellner"])
    .nothing_else();
    assert_single_author("Barboza du Bocage")
        .comb_authors(None, &["Barboza du Bocage"])
        .nothing_else();
    assert_authorship("Payri & P.W.Gabrielson", &["Payri", "P.W.Gabrielson"])
        .comb_authors(None, &["Payri", "P.W.Gabrielson"])
        .nothing_else();
    assert_authorship(
        "N'Yeurt, Payri & P.W.Gabrielson",
        &["N'Yeurt", "Payri", "P.W.Gabrielson"],
    )
    .comb_authors(None, &["N'Yeurt", "Payri", "P.W.Gabrielson"])
    .nothing_else();
    assert_single_author("VanLand.")
        .comb_authors(None, &["VanLand."])
        .nothing_else();
    assert_single_author("MacLeish")
        .comb_authors(None, &["MacLeish"])
        .nothing_else();
    // Java kept "ms." in a separate authorship's author; the name string always stripped it as a
    // manuscript marker ("Aus bus Monterosato ms."), and now both do.
    assert_authorship("Monterosato ms.", &["Monterosato"])
        .manuscript()
        .comb_authors(None, &["Monterosato"])
        .nom_note("ms.")
        .nothing_else();
    // FIXME(review): "ms." is glued onto Arnott; stripped as for "Monterosato ms.", it reads Arn.
    // ex Grunow
    assert_authorship("Arn. ms., Grunow", &["Arn.ms.", "Grunow"])
        .comb_authors(None, &["Arn.ms.", "Grunow"])
        .nothing_else();
    assert_authorship(
        "Choi,J.H.; Im,W.T.; Yoo,J.S.; Lee,S.M.; Moon,D.S.; Kim,H.J.; Rhee,S.K.; Roh,D.H.",
        &[
            "J.H.Choi", "W.T.Im", "J.S.Yoo", "S.M.Lee", "D.S.Moon", "H.J.Kim", "S.K.Rhee",
            "D.H.Roh",
        ],
    )
    .comb_authors(
        None,
        &[
            "J.H.Choi", "W.T.Im", "J.S.Yoo", "S.M.Lee", "D.S.Moon", "H.J.Kim", "S.K.Rhee",
            "D.H.Roh",
        ],
    )
    .nothing_else();
    assert_authorship("da Costa Lima", &["da Costa Lima"])
        .comb_authors(None, &["da Costa Lima"])
        .nothing_else();
    assert_authorship(
        "Krapov., W.C.Greg. & C.E.Simpson",
        &["Krapov.", "W.C.Greg.", "C.E.Simpson"],
    )
    .comb_authors(None, &["Krapov.", "W.C.Greg.", "C.E.Simpson"])
    .nothing_else();
    assert_authorship("de Jussieu", &["de Jussieu"])
        .comb_authors(None, &["de Jussieu"])
        .nothing_else();
    assert_authorship("van-der Land", &["van-der Land"])
        .comb_authors(None, &["van-der Land"])
        .nothing_else();
    assert_authorship("van der Land", &["van der Land"])
        .comb_authors(None, &["van der Land"])
        .nothing_else();
    assert_authorship("van Helmsick", &["van Helmsick"])
        .comb_authors(None, &["van Helmsick"])
        .nothing_else();
    assert_authorship("Xing, Yan & Yin", &["Xing", "Yan", "Yin"])
        .comb_authors(None, &["Xing", "Yan", "Yin"])
        .nothing_else();
    assert_authorship("Xiao & Knoll", &["Xiao", "Knoll"])
        .comb_authors(None, &["Xiao", "Knoll"])
        .nothing_else();
    // FIXME(review): "Wang, Yuwen" is one person written surname-first (Yuwen Wang), as "Wang,
    // Y.-j." already is
    assert_authorship(
        "Wang, Yuwen & Xian-wei Liu",
        &["Wang", "Yuwen", "Xian-wei Liu"],
    )
    .comb_authors(None, &["Wang", "Yuwen", "Xian-wei Liu"])
    .nothing_else();
    // FIXME(review): "Liu, Xian-wei" is one person written surname-first (Xian-wei Liu), not two
    assert_authorship(
        "Liu, Xian-wei, Z. Zheng & G. Xi",
        &["Liu", "Xian-wei", "Z.Zheng", "G.Xi"],
    )
    .comb_authors(None, &["Liu", "Xian-wei", "Z.Zheng", "G.Xi"])
    .nothing_else();
    assert_authorship(
        "Clayton, D.H.; Price, R.D.; Page, R.D.M.",
        &["D.H.Clayton", "R.D.Price", "R.D.M.Page"],
    )
    .comb_authors(None, &["D.H.Clayton", "R.D.Price", "R.D.M.Page"])
    .nothing_else();
    assert_authorship("Michiel de Ruyter", &["Michiel de Ruyter"])
        .comb_authors(None, &["Michiel de Ruyter"])
        .nothing_else();
    assert_authorship("DeFilipps", &["DeFilipps"])
        .comb_authors(None, &["DeFilipps"])
        .nothing_else();
    assert_authorship("Henk 't Hart", &["Henk 't Hart"])
        .comb_authors(None, &["Henk 't Hart"])
        .nothing_else();
    assert_authorship("P.E.Berry & Reg.B.Miller", &["P.E.Berry", "Reg.B.Miller"])
        .comb_authors(None, &["P.E.Berry", "Reg.B.Miller"])
        .nothing_else();
    // forename + spaced middle initial + surname is one author, not a surname-first flip
    assert_authorship("Calder & Roy L. Taylor", &["Calder", "Roy L.Taylor"])
        .comb_authors(None, &["Calder", "Roy L.Taylor"])
        .nothing_else();
    assert_authorship("'t Hart", &["'t Hart"])
        .comb_authors(None, &["'t Hart"])
        .nothing_else();
    assert_authorship("Abdallah & Sa'ad", &["Abdallah", "Sa'ad"])
        .comb_authors(None, &["Abdallah", "Sa'ad"])
        .nothing_else();
    assert_single_author("Linnaeus filius")
        .comb_authors(None, &["Linnaeus filius"])
        .nothing_else();
    assert_authorship(
        "Bollmann, M.Y.Cortés, Kleijne, J.B.Østerg. & Jer.R.Young",
        &[
            "Bollmann",
            "M.Y.Cortés",
            "Kleijne",
            "J.B.Østerg.",
            "Jer.R.Young",
        ],
    )
    .comb_authors(
        None,
        &[
            "Bollmann",
            "M.Y.Cortés",
            "Kleijne",
            "J.B.Østerg.",
            "Jer.R.Young",
        ],
    )
    .nothing_else();
    assert_authorship(
        "Branco, M.T.P.Azevedo, Sant'Anna & Komárek",
        &["Branco", "M.T.P.Azevedo", "Sant'Anna", "Komárek"],
    )
    .comb_authors(None, &["Branco", "M.T.P.Azevedo", "Sant'Anna", "Komárek"])
    .nothing_else();
    assert_single_author("Janick Hendrik van Kinsbergen")
        .comb_authors(None, &["Janick Hendrik van Kinsbergen"])
        .nothing_else();
    assert_single_author("Jan Hendrik van Kinsbergen")
        .comb_authors(None, &["Jan Hendrik van Kinsbergen"])
        .nothing_else();
    assert_single_author("Sainte-Claire Deville")
        .comb_authors(None, &["Sainte-Claire Deville"])
        .nothing_else();
    assert_single_author("Semenov-Tian-Shanskij")
        .comb_authors(None, &["Semenov-Tian-Shanskij"])
        .nothing_else();
    assert_authorship(
        "Semenov-Tian-Shanskij, Sainte-Claire Deville, Janick Hendrik van Kinsbergen",
        &[
            "Semenov-Tian-Shanskij",
            "Sainte-Claire Deville",
            "Janick Hendrik van Kinsbergen",
        ],
    )
    .comb_authors(
        None,
        &[
            "Semenov-Tian-Shanskij",
            "Sainte-Claire Deville",
            "Janick Hendrik van Kinsbergen",
        ],
    )
    .nothing_else();
    assert_single_author("Scotto la Massese")
        .comb_authors(None, &["Scotto la Massese"])
        .nothing_else();
    assert_single_author("An der Lan")
        .comb_authors(None, &["An der Lan"])
        .nothing_else();
    assert_authorship("Bor & s'Jacob", &["Bor", "s'Jacob"])
        .comb_authors(None, &["Bor", "s'Jacob"])
        .nothing_else();
    assert_single_author("Brunner von Wattenwyl v.W.")
        .comb_authors(None, &["Brunner von Wattenwyl v.W."])
        .nothing_else();
    // FIXME(review): "Martinez y Saez" is one person (Martínez y Sáez, a Spanish double surname),
    // not two; "y" is no "et" here (cf. "Da Silva e Castro" below)
    assert_authorship("Martinez y Saez", &["Martinez", "Saez"])
        .comb_authors(None, &["Martinez", "Saez"])
        .nothing_else();
    // not two separate names — a compound surname (family name), common in Portuguese-speaking cultures like Portugal and Brazil.
    assert_single_author("Da Silva e Castro")
        .comb_authors(None, &["Da Silva e Castro"])
        .nothing_else();
    assert_authorship("LafuenteRoca & Carbonell", &["LafuenteRoca", "Carbonell"])
        .comb_authors(None, &["LafuenteRoca", "Carbonell"])
        .nothing_else();
    assert_authorship("Mas-ComaBargues & Esteban", &["Mas-ComaBargues", "Esteban"])
        .comb_authors(None, &["Mas-ComaBargues", "Esteban"])
        .nothing_else();
    assert_single_author("Hondt d")
        .comb_authors(None, &["Hondt d"])
        .nothing_else();
    assert_single_author("Abou-El-Naga")
        .comb_authors(None, &["Abou-El-Naga"])
        .nothing_else();
    assert_authorship(
        "Yong Wang bis, Y. Song, K. Geng & K.D. Hyde",
        &["Yong Wang bis", "Y.Song", "K.Geng", "K.D.Hyde"],
    )
    .comb_authors(None, &["Yong Wang bis", "Y.Song", "K.Geng", "K.D.Hyde"])
    .nothing_else();
    assert_authorship(
        "Sh. Kumar, R. Singh ter, Gond & Saini",
        &["Sh.Kumar", "R.Singh ter", "Gond", "Saini"],
    )
    .comb_authors(None, &["Sh.Kumar", "R.Singh ter", "Gond", "Saini"])
    .nothing_else();
    assert_single_author("R.Singh bis")
        .comb_authors(None, &["R.Singh bis"])
        .nothing_else();
    assert_authorship("zur Strassen", &["zur Strassen"])
        .comb_authors(None, &["zur Strassen"])
        .nothing_else();
    // Malformed input with stray "(" at the end — preserved verbatim as ex-authorship form.
    assert_ex_authorship("Wedd. ex Sch. Bip. (", Some("Wedd."), &["Sch.Bip."])
        .comb_authors(None, &["Sch.Bip."])
        .comb_ex_authors(&["Wedd."])
        .nothing_else();
    assert_ex_authorship("Plesn¡k ex F.Ritter", Some("Plesnik"), &["F.Ritter"])
        .comb_authors(None, &["F.Ritter"])
        .comb_ex_authors(&["Plesnik"])
        .warning(&[warnings::HOMOGLYHPS])
        .nothing_else();
    assert_authorship(
        "Britton, Sterns, & Poggenb.",
        &["Britton", "Sterns", "Poggenb."],
    )
    .comb_authors(None, &["Britton", "Sterns", "Poggenb."])
    .nothing_else();
    assert_authorship("Van Heurck & Müll. Arg.", &["Van Heurck", "Müll.Arg."])
        .comb_authors(None, &["Van Heurck", "Müll.Arg."])
        .nothing_else();
    assert_authorship("Gruber-Vodicka", &["Gruber-Vodicka"])
        .comb_authors(None, &["Gruber-Vodicka"])
        .nothing_else();
    assert_authorship("Gruber-Vodicka et al.", &["Gruber-Vodicka", "al."])
        .comb_authors(None, &["Gruber-Vodicka", "al."])
        .nothing_else();
    assert_single_author("L.")
        .comb_authors(None, &["L."])
        .nothing_else();
    assert_single_author("Lin.")
        .comb_authors(None, &["Lin."])
        .nothing_else();
    assert_single_author("Linné")
        .comb_authors(None, &["Linné"])
        .nothing_else();
    assert_single_author("DC.")
        .comb_authors(None, &["DC."])
        .nothing_else();
    assert_single_author("de Chaudoir")
        .comb_authors(None, &["de Chaudoir"])
        .nothing_else();
    assert_single_author("Hilaire")
        .comb_authors(None, &["Hilaire"])
        .nothing_else();
    assert_single_author("G.Don fil.")
        .comb_authors(None, &["G.Don fil."])
        .nothing_else();
    assert_authorship("St. Hilaire", &["St.Hilaire"])
        .comb_authors(None, &["St.Hilaire"])
        .nothing_else();
    assert_authorship("Geoffroy St. Hilaire", &["Geoffroy St.Hilaire"])
        .comb_authors(None, &["Geoffroy St.Hilaire"])
        .nothing_else();
    assert_single_author("Acev.-Rodr.")
        .comb_authors(None, &["Acev.-Rodr."])
        .nothing_else();
    assert_authorship(
        "Steyerm., Aristeg. & Wurdack",
        &["Steyerm.", "Aristeg.", "Wurdack"],
    )
    .comb_authors(None, &["Steyerm.", "Aristeg.", "Wurdack"])
    .nothing_else();
    assert_authorship("Du Puy & Labat", &["Du Puy", "Labat"])
        .comb_authors(None, &["Du Puy", "Labat"])
        .nothing_else();
    assert_single_author("Baum.-Bod.")
        .comb_authors(None, &["Baum.-Bod."])
        .nothing_else();
    assert_authorship("Engl. & v. Brehmer", &["Engl.", "v.Brehmer"])
        .comb_authors(None, &["Engl.", "v.Brehmer"])
        .nothing_else();
    assert_authorship("F. v. Muell.", &["F.v.Muell."])
        .comb_authors(None, &["F.v.Muell."])
        .nothing_else();
    assert_authorship("W.J.de Wilde & Duyfjes", &["W.J.de Wilde", "Duyfjes"])
        .comb_authors(None, &["W.J.de Wilde", "Duyfjes"])
        .nothing_else();
    assert_single_author("C.E.M.Bicudo")
        .comb_authors(None, &["C.E.M.Bicudo"])
        .nothing_else();
    assert_single_author("Alves-da-Silva")
        .comb_authors(None, &["Alves-da-Silva"])
        .nothing_else();
    assert_authorship(
        "Alves-da-Silva & C.E.M.Bicudo",
        &["Alves-da-Silva", "C.E.M.Bicudo"],
    )
    .comb_authors(None, &["Alves-da-Silva", "C.E.M.Bicudo"])
    .nothing_else();
    assert_single_author("Kingdon-Ward")
        .comb_authors(None, &["Kingdon-Ward"])
        .nothing_else();
    assert_authorship("Merr. & L.M.Perry", &["Merr.", "L.M.Perry"])
        .comb_authors(None, &["Merr.", "L.M.Perry"])
        .nothing_else();
    assert_authorship(
        "Calat., Nav.-Ros. & Hafellner",
        &["Calat.", "Nav.-Ros.", "Hafellner"],
    )
    .comb_authors(None, &["Calat.", "Nav.-Ros.", "Hafellner"])
    .nothing_else();
    assert_ex_authorship("Arv.-Touv. ex Dörfl.", Some("Arv.-Touv."), &["Dörfl."])
        .comb_authors(None, &["Dörfl."])
        .comb_ex_authors(&["Arv.-Touv."])
        .nothing_else();
    assert_authorship("Payri & P.W.Gabrielson", &["Payri", "P.W.Gabrielson"])
        .comb_authors(None, &["Payri", "P.W.Gabrielson"])
        .nothing_else();
    assert_authorship(
        "N'Yeurt, Payri & P.W.Gabrielson",
        &["N'Yeurt", "Payri", "P.W.Gabrielson"],
    )
    .comb_authors(None, &["N'Yeurt", "Payri", "P.W.Gabrielson"])
    .nothing_else();
    assert_single_author("VanLand.")
        .comb_authors(None, &["VanLand."])
        .nothing_else();
    assert_single_author("MacLeish")
        .comb_authors(None, &["MacLeish"])
        .nothing_else();
    // Java kept "ms." in a separate authorship's author; the name string always stripped it as a
    // manuscript marker ("Aus bus Monterosato ms."), and now both do.
    assert_authorship("Monterosato ms.", &["Monterosato"])
        .manuscript()
        .comb_authors(None, &["Monterosato"])
        .nom_note("ms.")
        .nothing_else();
    // FIXME(review): "ms." is glued onto Arnott; stripped as for "Monterosato ms.", it reads Arn.
    // ex Grunow
    assert_authorship("Arn. ms., Grunow", &["Arn.ms.", "Grunow"])
        .comb_authors(None, &["Arn.ms.", "Grunow"])
        .nothing_else();
    assert_ex_authorship("Griseb. ex. Wedd.", Some("Griseb."), &["Wedd."])
        .comb_authors(None, &["Wedd."])
        .comb_ex_authors(&["Griseb."])
        .nothing_else();
    assert_authorship(
        "Castellano, S.L.Mill., L.Singh bis & T.N.Lakh.",
        &["Castellano", "S.L.Mill.", "L.Singh bis", "T.N.Lakh."],
    )
    .comb_authors(
        None,
        &["Castellano", "S.L.Mill.", "L.Singh bis", "T.N.Lakh."],
    )
    .nothing_else();
    // in litteris: an unpublished name
    assert_authorship("Blüthgen i.l.", &["Blüthgen"])
        .comb_authors(None, &["Blüthgen"])
        .nom_note("i.l.")
        .manuscript()
        .nothing_else();
    assert_authorship("Y.-j. Wang", &["Y.-j.Wang"])
        .comb_authors(None, &["Y.-j.Wang"])
        .nothing_else();
    assert_single_author("Z.-q.Liu")
        .comb_authors(None, &["Z.-q.Liu"])
        .nothing_else();
    assert_single_author("Van den heede")
        .comb_authors(None, &["Van den heede"])
        .nothing_else();
    assert_single_author("zur Strassen")
        .comb_authors(None, &["zur Strassen"])
        .nothing_else();
}
