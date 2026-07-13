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
    //assertAuthorship("ex DC.")
    //    .combAuthors(null, "DC.")
    //    .nothingElse();

    //TODO: https://github.com/gbif/name-parser/issues/49
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

// SKIPPED: namesWithAuthorFile — reads a resource corpus file, covered by the golden/cross-val harness.
// SKIPPED: otherFile — reads a resource corpus file, covered by the golden/cross-val harness.
// SKIPPED: hybridsFile — reads a resource corpus file, covered by the golden/cross-val harness.
// SKIPPED: placeholderFile — reads a resource corpus file, covered by the golden/cross-val harness.

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
    assert_name("Amblyomma americanum (Linnaeus, 1758)");
    assert_name("Salix taiwanalpina var. chingshuishanensis (S.S.Ying) F.Y.Lu, C.H.Ou, Y.C.Chen, Y.S.Chi, K.C.Lu & Y.H.Tseng ");
    assert_name("Salix taiwanalpina var. chingshuishanensis (S.S.Ying) F.Y.Lu, C.H.Ou, Y.C.Chen, Y.S.Chi, K.C.Lu & amp  Y.H.Tseng ");
    assert_name("Salix morrisonicola var. takasagoalpina (Koidz.) F.Y.Lu, C.H.Ou, Y.C.Chen, Y.S.Chi, K.C.Lu & amp; Y.H.Tseng");
    assert_name(
        "Ficus ernanii Carauta, Pederneir., P.P.Souza, A.F.P.Machado, M.D.M.Vianna & amp; Romaniuc",
    );
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

    //TODO: pro syn.
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
    assert_authorship("Jarocki or Schinz", &["Jarocki or Schinz"]);
    assert_authorship("van der Wulp", &["van der Wulp"]);
    assert_authorship(
        "Balsamo M Fregni E Tongiorgi MA",
        &["M.Balsamo", "E.Fregni", "M.A.Tongiorgi"],
    );
    assert_authorship("Walker, F.", &["F.Walker"]);
    assert_authorship("Walker, F", &["F.Walker"]);
    assert_authorship("Walker F", &["F.Walker"]);
    assert_authorship("YJ Wang & ZQ Liu", &["YJ Wang", "ZQ Liu"]);
    assert_authorship("Y.-j. Wang & Z.-q. Liu", &["Y.-j.Wang", "Z.-q.Liu"]);
    assert_authorship("Petzold & G.Kirchn.", &["Petzold", "G.Kirchn."]);
    assert_authorship(
        "Britton, Sterns, & Poggenb.",
        &["Britton", "Sterns", "Poggenb."],
    );
    assert_authorship("Van Heurck & Müll. Arg.", &["Van Heurck", "Müll.Arg."]);
    assert_authorship("Gruber-Vodicka", &["Gruber-Vodicka"]);
    assert_authorship("Gruber-Vodicka et al.", &["Gruber-Vodicka", "al."]);
    assert_single_author("L.");
    assert_single_author("Lin.");
    assert_single_author("Linné");
    assert_single_author("DC.");
    assert_single_author("de Chaudoir");
    assert_single_author("Hilaire");
    assert_authorship("St. Hilaire", &["St.Hilaire"]);
    assert_authorship("Geoffroy St. Hilaire", &["Geoffroy St.Hilaire"]);
    assert_single_author("Acev.-Rodr.");
    assert_authorship(
        "Steyerm., Aristeg. & Wurdack",
        &["Steyerm.", "Aristeg.", "Wurdack"],
    );
    assert_authorship("Du Puy & Labat", &["Du Puy", "Labat"]);
    assert_single_author("Baum.-Bod.");
    assert_authorship("Engl. & v. Brehmer", &["Engl.", "v.Brehmer"]);
    assert_authorship("F. v. Muell.", &["F.v.Muell."]);
    assert_authorship("W.J.de Wilde & Duyfjes", &["W.J.de Wilde", "Duyfjes"]);
    assert_single_author("C.E.M.Bicudo");
    assert_single_author("Alves-da-Silva");
    assert_authorship(
        "Alves-da-Silva & C.E.M.Bicudo",
        &["Alves-da-Silva", "C.E.M.Bicudo"],
    );
    assert_single_author("Kingdon-Ward");
    assert_authorship("Merr. & L.M.Perry", &["Merr.", "L.M.Perry"]);
    assert_authorship(
        "Calat., Nav.-Ros. & Hafellner",
        &["Calat.", "Nav.-Ros.", "Hafellner"],
    );
    assert_single_author("Barboza du Bocage");
    assert_authorship("Payri & P.W.Gabrielson", &["Payri", "P.W.Gabrielson"]);
    assert_authorship(
        "N'Yeurt, Payri & P.W.Gabrielson",
        &["N'Yeurt", "Payri", "P.W.Gabrielson"],
    );
    assert_single_author("VanLand.");
    assert_single_author("MacLeish");
    assert_single_author("Monterosato ms.");
    assert_authorship("Arn. ms., Grunow", &["Arn.ms.", "Grunow"]);
    assert_authorship(
        "Choi,J.H.; Im,W.T.; Yoo,J.S.; Lee,S.M.; Moon,D.S.; Kim,H.J.; Rhee,S.K.; Roh,D.H.",
        &[
            "J.H.Choi", "W.T.Im", "J.S.Yoo", "S.M.Lee", "D.S.Moon", "H.J.Kim", "S.K.Rhee",
            "D.H.Roh",
        ],
    );
    assert_authorship("da Costa Lima", &["da Costa Lima"]);
    assert_authorship(
        "Krapov., W.C.Greg. & C.E.Simpson",
        &["Krapov.", "W.C.Greg.", "C.E.Simpson"],
    );
    assert_authorship("de Jussieu", &["de Jussieu"]);
    assert_authorship("van-der Land", &["van-der Land"]);
    assert_authorship("van der Land", &["van der Land"]);
    assert_authorship("van Helmsick", &["van Helmsick"]);
    assert_authorship("Xing, Yan & Yin", &["Xing", "Yan", "Yin"]);
    assert_authorship("Xiao & Knoll", &["Xiao", "Knoll"]);
    assert_authorship(
        "Wang, Yuwen & Xian-wei Liu",
        &["Wang", "Yuwen", "Xian-wei Liu"],
    );
    assert_authorship(
        "Liu, Xian-wei, Z. Zheng & G. Xi",
        &["Liu", "Xian-wei", "Z.Zheng", "G.Xi"],
    );
    assert_authorship(
        "Clayton, D.H.; Price, R.D.; Page, R.D.M.",
        &["D.H.Clayton", "R.D.Price", "R.D.M.Page"],
    );
    assert_authorship("Michiel de Ruyter", &["Michiel de Ruyter"]);
    assert_authorship("DeFilipps", &["DeFilipps"]);
    assert_authorship("Henk 't Hart", &["Henk 't Hart"]);
    assert_authorship("P.E.Berry & Reg.B.Miller", &["P.E.Berry", "Reg.B.Miller"]);
    // forename + spaced middle initial + surname is one author, not a surname-first flip
    assert_authorship("Calder & Roy L. Taylor", &["Calder", "Roy L.Taylor"]);
    assert_authorship("'t Hart", &["'t Hart"]);
    assert_authorship("Abdallah & Sa'ad", &["Abdallah", "Sa'ad"]);
    assert_single_author("Linnaeus filius");
    assert_authorship(
        "Bollmann, M.Y.Cortés, Kleijne, J.B.Østerg. & Jer.R.Young",
        &[
            "Bollmann",
            "M.Y.Cortés",
            "Kleijne",
            "J.B.Østerg.",
            "Jer.R.Young",
        ],
    );
    assert_authorship(
        "Branco, M.T.P.Azevedo, Sant'Anna & Komárek",
        &["Branco", "M.T.P.Azevedo", "Sant'Anna", "Komárek"],
    );
    assert_single_author("Janick Hendrik van Kinsbergen");
    assert_single_author("Jan Hendrik van Kinsbergen");
    assert_single_author("Sainte-Claire Deville");
    assert_single_author("Semenov-Tian-Shanskij");
    assert_authorship(
        "Semenov-Tian-Shanskij, Sainte-Claire Deville, Janick Hendrik van Kinsbergen",
        &[
            "Semenov-Tian-Shanskij",
            "Sainte-Claire Deville",
            "Janick Hendrik van Kinsbergen",
        ],
    );
    assert_single_author("Scotto la Massese");
    assert_single_author("An der Lan");
    assert_authorship("Bor & s'Jacob", &["Bor", "s'Jacob"]);
    assert_single_author("Brunner von Wattenwyl v.W.");
    // spanish "et"
    assert_authorship("Martinez y Saez", &["Martinez", "Saez"]);
    // not two separate names — a compound surname (family name), common in Portuguese-speaking cultures like Portugal and Brazil.
    assert_single_author("Da Silva e Castro");
    assert_authorship("LafuenteRoca & Carbonell", &["LafuenteRoca", "Carbonell"]);
    assert_authorship("Mas-ComaBargues & Esteban", &["Mas-ComaBargues", "Esteban"]);
    assert_single_author("Hondt d");
    assert_single_author("Abou-El-Naga");
    assert_authorship(
        "Yong Wang bis, Y. Song, K. Geng & K.D. Hyde",
        &["Yong Wang bis", "Y.Song", "K.Geng", "K.D.Hyde"],
    );
    assert_authorship(
        "Sh. Kumar, R. Singh ter, Gond & Saini",
        &["Sh.Kumar", "R.Singh ter", "Gond", "Saini"],
    );
    assert_single_author("R.Singh bis");
    assert_authorship("zur Strassen", &["zur Strassen"]);
    // Malformed input with stray "(" at the end — preserved verbatim as ex-authorship form.
    assert_ex_authorship("Wedd. ex Sch. Bip. (", Some("Wedd."), &["Sch.Bip."]);
    assert_ex_authorship("Plesn¡k ex F.Ritter", Some("Plesnik"), &["F.Ritter"]);
    assert_authorship(
        "Britton, Sterns, & Poggenb.",
        &["Britton", "Sterns", "Poggenb."],
    );
    assert_authorship("Van Heurck & Müll. Arg.", &["Van Heurck", "Müll.Arg."]);
    assert_authorship("Gruber-Vodicka", &["Gruber-Vodicka"]);
    assert_authorship("Gruber-Vodicka et al.", &["Gruber-Vodicka", "al."]);
    assert_single_author("L.");
    assert_single_author("Lin.");
    assert_single_author("Linné");
    assert_single_author("DC.");
    assert_single_author("de Chaudoir");
    assert_single_author("Hilaire");
    assert_single_author("G.Don fil.");
    assert_authorship("St. Hilaire", &["St.Hilaire"]);
    assert_authorship("Geoffroy St. Hilaire", &["Geoffroy St.Hilaire"]);
    assert_single_author("Acev.-Rodr.");
    assert_authorship(
        "Steyerm., Aristeg. & Wurdack",
        &["Steyerm.", "Aristeg.", "Wurdack"],
    );
    assert_authorship("Du Puy & Labat", &["Du Puy", "Labat"]);
    assert_single_author("Baum.-Bod.");
    assert_authorship("Engl. & v. Brehmer", &["Engl.", "v.Brehmer"]);
    assert_authorship("F. v. Muell.", &["F.v.Muell."]);
    assert_authorship("W.J.de Wilde & Duyfjes", &["W.J.de Wilde", "Duyfjes"]);
    assert_single_author("C.E.M.Bicudo");
    assert_single_author("Alves-da-Silva");
    assert_authorship(
        "Alves-da-Silva & C.E.M.Bicudo",
        &["Alves-da-Silva", "C.E.M.Bicudo"],
    );
    assert_single_author("Kingdon-Ward");
    assert_authorship("Merr. & L.M.Perry", &["Merr.", "L.M.Perry"]);
    assert_authorship(
        "Calat., Nav.-Ros. & Hafellner",
        &["Calat.", "Nav.-Ros.", "Hafellner"],
    );
    assert_ex_authorship("Arv.-Touv. ex Dörfl.", Some("Arv.-Touv."), &["Dörfl."]);
    assert_authorship("Payri & P.W.Gabrielson", &["Payri", "P.W.Gabrielson"]);
    assert_authorship(
        "N'Yeurt, Payri & P.W.Gabrielson",
        &["N'Yeurt", "Payri", "P.W.Gabrielson"],
    );
    assert_single_author("VanLand.");
    assert_single_author("MacLeish");
    assert_single_author("Monterosato ms.");
    assert_authorship("Arn. ms., Grunow", &["Arn.ms.", "Grunow"]);
    assert_ex_authorship("Griseb. ex. Wedd.", Some("Griseb."), &["Wedd."]);
    assert_authorship(
        "Castellano, S.L.Mill., L.Singh bis & T.N.Lakh.",
        &["Castellano", "S.L.Mill.", "L.Singh bis", "T.N.Lakh."],
    );
    assert_authorship("Blüthgen i.l.", &["Blüthgen i.l."]);
    assert_authorship("Y.-j. Wang", &["Y.-j.Wang"]);
    assert_single_author("Z.-q.Liu");
    assert_single_author("Van den heede");
    assert_single_author("zur Strassen");
}
