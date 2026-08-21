// SPDX-License-Identifier: Apache-2.0
//! The 5.0.0 informal / semistructured band, tested through the three-way `parse` via the
//! fluent `assert_informal` / `assert_name` DSL helpers. Cases are lifted from the reservoir
//! samples of the 67.5M verbatim-corpus study: overwhelmingly
//! molecular / DNA-barcoding provisional species `Genus sp. <specimen/culture/BOLD code>`.
//!
//! The design contract this file pins:
//!  * a supraspecific taxon carrying a provisional designation with NO species epithet → `Informal`,
//!    a flat `taxon` + `taxon_rank` + `rank` + `phrase` + `code`;
//!  * a name WITH a species epithet (a binomial core — incl. cf./aff. and infraspecific-indet) stays
//!    `Parsed`, so its `specific_authorship` (unrepresentable by a flat anchor) survives;
//!  * a determined monomial (`Rhizobium`) stays `Parsed`/SCIENTIFIC — not informal.

mod common;
use common::*;
use nameparser::model::{NamePart, NameType, NomCode, Rank};
use nameparser::ParseResult;

// ---- Informal: supraspecific anchor + provisional designation, no species epithet -------------

#[test]
fn molecular_provisional_species_with_a_captured_tag() {
    // ~99.8% of the band: genus-anchored, SPECIES rank, a specimen/culture/BOLD code phrase.
    assert_informal("Serratia sp. RE1-2a")
        .taxon("Serratia")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. RE1-2a")
        .nothing_else();
    assert_informal("Plasmodium sp. SYBOR9")
        .taxon("Plasmodium")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. SYBOR9")
        .nothing_else();
}

#[test]
fn multi_token_specimen_tag_is_captured_as_the_phrase() {
    // The 5.0.0 tag-capture enhancement rescues the ~382k rows whose multi-token trailing tag the
    // 4.2.0 parser dropped (or misread as an author): the whole verbatim tail becomes the phrase.
    assert_informal("Rhizobium sp. RMCC TR1811")
        .taxon("Rhizobium")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. RMCC TR1811")
        .nothing_else();
    assert_informal("Ichneumonidae sp. UAM Ento 145060")
        .taxon("Ichneumonidae") // a family, but the parser's best guess is the genus slot (not backbone-validated)
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. UAM Ento 145060")
        .nothing_else();
}

#[test]
fn species_n_with_a_trailing_note_keeps_the_whole_tail_as_the_phrase() {
    // "once a phrase starts, it runs to the end": everything after "(sp|spec|species) N" is part of
    // the phrase, verbatim — so a trailing "(=synonym)" note is NOT split off as a subgenus/epithet.
    // Surfaced by the CoL backend's dwca/17 fixture.
    assert_informal("Dichanthelium species 12 (=chrysopsidifolium)")
        .taxon("Dichanthelium")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("species 12 (=chrysopsidifolium)")
        .nothing_else();
}

// ---- The tail runs to the end, authorship included -------------------------------------------

/// An informal name is not fully parsable by definition, so the phrase takes the WHOLE verbatim
/// tail — a trailing author citation included. Until 5.0.0 a year-bearing tail was exempted and
/// routed to `combinationAuthorship` instead, on the theory that it "IS an authorship citation,
/// not a specimen tag". In the 67.5M verbatim corpus that theory holds for barely half the band:
/// of 681 informal names carrying a parsed authorship, 43% were demonstrably spurious on a
/// conservative test — impossible years (`1002`, `2483`, `2951`) or "authors" carrying digits,
/// underscores or slashes — and that undercounts, because collection acronyms like `ZRC` and
/// `MNHN` pass both tests. Worse, the exemption truncated the tag as well:
/// `Rhodococcus sp. 14-2483-1-2` kept only `sp. 14` and invented the year 2483.
///
/// So the tail is now captured whole and nothing is invented. Round-tripping beats structure
/// here: on `Amphicynodon sp. 1 Filhol, 1881` the authorship is the GENUS author anyway — an
/// undetermined species has none — and a caller who wants it can resolve `taxon`.
#[test]
fn a_trailing_author_citation_is_part_of_the_phrase() {
    // No `code` either: the zoological inference keyed off the "Author, year" authorship that no
    // longer exists. Dropping it removes wrong answers rather than a right one — it used to label
    // the plant `Aster sp. Linnaeus, 1753` ZOOLOGICAL, and the alga `Ectocarpus sp. CCAP 1310/114`
    // ZOOLOGICAL off the bogus year 1310 it read out of a culture-collection accession.
    assert_informal("Cantuaria sp. Forster, 1968")
        .taxon("Cantuaria")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. Forster, 1968")
        .nothing_else();
    assert_informal("Amphicynodon sp. 1 Filhol, 1881")
        .taxon("Amphicynodon")
        .rank(Rank::Species)
        .phrase("sp. 1 Filhol, 1881");
    assert_informal("Aster sp. Linnaeus, 1753")
        .taxon("Aster")
        .rank(Rank::Species)
        .phrase("sp. Linnaeus, 1753");
    assert_informal("Anuropus species N. Bruce, 2008")
        .taxon("Anuropus")
        .rank(Rank::Species)
        .phrase("species N. Bruce, 2008");
}

/// The misparses the old exemption produced: a digit-bearing specimen or collection code was
/// split into a bogus author + year AND had its tag truncated. Both are now kept verbatim.
#[test]
fn digit_bearing_specimen_codes_are_no_longer_split_into_a_bogus_author() {
    // was: phrase "sp. 14", combinationAuthorship year=2483
    assert_informal("Rhodococcus sp. 14-2483-1-2")
        .taxon("Rhodococcus")
        .rank(Rank::Species)
        .phrase("sp. 14-2483-1-2");
    // was: authors=["ZRC"], year=1999 — ZRC is the Zoological Reference Collection
    assert_informal("Atergatopsis sp. ZRC 1999.0472")
        .taxon("Atergatopsis")
        .rank(Rank::Species)
        .phrase("sp. ZRC 1999.0472");
    // was: authors=["Ccap"], year=1310
    assert_informal("Ectocarpus sp. CCAP 1310/114")
        .taxon("Ectocarpus")
        .rank(Rank::Species)
        .phrase("sp. CCAP 1310/114");
}

/// An underscored OTU code is a specimen tag like any other, so it belongs in the phrase.
///
/// `stashTrailingOtuCode` (StripAndStash step 16, `\s+([A-Z0-9]{3,}_\d{3,})$`) amputates such a
/// code into `unparsed` BEFORE tokenising, so the informal tail-capture never saw it: on
/// `Streptomyces sp. NBC_00448` the phrase came back as a bare `"sp."` with `NBC_00448` parked in
/// `unparsed` and the name marked PARTIAL. The underscore was the whole difference —
/// `Streptomyces sp. NBC00448` already captured `"sp. NBC00448"` and parsed COMPLETE.
///
/// That mattered because a flat `Informal` has no `unparsed` field, so for the informal band the
/// code was dropped outright at the three-way boundary. A determined name keeps its epithet and
/// stays `Parsed`, where `unparsed` survives on the `ParsedName` — so the stash is left alone
/// there and only the indet case is diverted to the phrase.
#[test]
fn underscored_otu_codes_are_captured_as_the_phrase_not_stashed_as_unparsed() {
    for (input, phrase) in [
        ("Streptomyces sp. NBC_00448", "sp. NBC_00448"),
        ("Hyalinobatrachium sp. ZSFQ_3906", "sp. ZSFQ_3906"),
        ("Salmonella sp. 2021_1741", "sp. 2021_1741"),
        ("Decapoda sp. KSA_1761", "sp. KSA_1761"),
        ("Limosilactobacillus sp. 252371_901", "sp. 252371_901"),
    ] {
        assert_informal(input).phrase(phrase).nothing_else();
    }
}

/// A DETERMINED name keeps the old `unparsed` stash: it stays `Parsed`, so the code rides along on
/// the `ParsedName` and nothing is lost. Guard against widening the diversion above.
#[test]
fn a_determined_name_still_stashes_its_otu_code_as_unparsed() {
    let pn = match nameparser::parse("Oxalis barrelieri XXZ_21243", None, None, None) {
        ParseResult::Parsed(pn) => pn,
        other => panic!("expected Parsed, got {other:?}"),
    };
    assert_eq!(pn.genus.as_deref(), Some("Oxalis"));
    assert_eq!(pn.specific_epithet.as_deref(), Some("barrelieri"));
    assert_eq!(pn.unparsed.as_deref(), Some("XXZ_21243"));
    assert_eq!(pn.phrase, None);
}

/// A catalogue number's `:<digits>` tail belongs to the phrase, not to `publishedInPage`.
///
/// `stripPublishedPage` (StripAndStash step 45, `\s*:\s*(\d+(?:[-–]\d+)?)\s*$`) exists for a
/// trailing page citation — `Anolis marmoratus girafus LAZELL 1964: 377`. Museum and culture
/// catalogue numbers wear the same shape, so `Trachipterus sp. HUMZ:220860` had `220860` filed as
/// a publication page and the phrase truncated to `"sp. HUMZ"`. The number was not lost on the
/// `ParsedName`, but a flat `Informal` has no `publishedInPage` field either, so the informal band
/// lost it at the three-way boundary — the third field to hit that projection after the authorship
/// and the OTU code.
///
/// The verbatim separator is restored, so `MIB:SASS:0006` comes back whole.
#[test]
fn a_catalogue_numbers_colon_tail_stays_in_the_phrase() {
    for (input, phrase) in [
        ("Trachipterus sp. HUMZ:220860", "sp. HUMZ:220860"),
        ("Prevotella sp. CAG:1031", "sp. CAG:1031"),
        ("Devario sp. CBM:ZF:11302", "sp. CBM:ZF:11302"),
        ("Ageratum sp. MIB:SASS:0006", "sp. MIB:SASS:0006"),
        ("Opistognathus sp. BSKU:121417", "sp. BSKU:121417"),
    ] {
        assert_informal(input).phrase(phrase).nothing_else();
    }
}

/// A strain identifier on a DETERMINED binomial is the phrase, not a page and not an author.
///
/// `Bacteroides caccae CAG21` already stashed `CAG21` as its phrase via `stashTrailingStrainCode`
/// (step 9), but the far commoner written form `CAG:21` — a co-abundance-gene-group MAG bin — was
/// not in that step's character class, so it fell through 36 steps to `stripPublishedPage` and was
/// split into a bogus author `CAG` AND a bogus page `21`. Both spellings now behave alike.
#[test]
fn a_colon_separated_strain_code_on_a_binomial_is_the_phrase() {
    for (input, phrase) in [
        ("Bacteroides caccae CAG:21", "CAG:21"),
        ("Streptococcus salivarius CAG:79", "CAG:79"),
        ("Ligilactobacillus ruminis CAG:367", "CAG:367"),
        // the pre-existing glued spelling, unchanged
        ("Bacteroides caccae CAG21", "CAG21"),
    ] {
        let pn = match nameparser::parse(input, None, None, None) {
            ParseResult::Parsed(pn) => pn,
            other => panic!("expected `{input}` to be Parsed, got {other:?}"),
        };
        assert_eq!(pn.phrase.as_deref(), Some(phrase), "phrase for {input:?}");
        assert_eq!(pn.published_in_page, None, "page for {input:?}");
        assert!(
            pn.combination_authorship.authors.is_empty(),
            "no author should be invented for {input:?}, got {:?}",
            pn.combination_authorship.authors
        );
    }
}

/// A page reference is the tail of a PUBLICATION citation, so it must follow a year. Without one,
/// a `:<digits>` tail is an identifier — `irmng:1017387` was filed as page 1017387.
#[test]
fn a_colon_digits_tail_with_no_year_is_not_a_published_page() {
    for input in ["irmng:1017387", "Bacteroides caccae CAG:21"] {
        match nameparser::parse(input, None, None, None) {
            ParseResult::Parsed(pn) => {
                assert_eq!(pn.published_in_page, None, "page for {input:?}")
            }
            other => panic!("expected `{input}` to be Parsed, got {other:?}"),
        }
    }
}

/// The mycological sanctioning colon is untouched by the widened strain-code class — it carries
/// dots and spaces the class does not admit.
#[test]
fn the_sanctioning_author_colon_still_works() {
    assert_name("Agaricus campestris L. : Fr.")
        .species("Agaricus", "campestris")
        .comb_authors(None, &["L."])
        .sanct_author("Fr.");
}

/// A DETERMINED name keeps the page strip: it stays `Parsed`, so `publishedInPage` survives on the
/// `ParsedName` and nothing is lost. Crucially this leaves GENUINE page citations alone — including
/// the tight `1933:54` form that carries no space after the colon, which no spacing rule could
/// separate from `CAG:21`; the year is what tells them apart.
#[test]
fn a_determined_name_keeps_its_published_page() {
    for (input, page) in [
        ("Anolis marmoratus girafus LAZELL 1964: 377", "377"),
        ("Anguis maculata Linnaeus, 1758: 228", "228"),
        ("Raphitydeus Thor 1933:54", "54"),
    ] {
        match nameparser::parse(input, None, None, None) {
            ParseResult::Parsed(pn) => {
                assert_eq!(
                    pn.published_in_page.as_deref(),
                    Some(page),
                    "publishedInPage for {input:?}"
                );
                assert_eq!(pn.phrase, None, "phrase for {input:?}");
            }
            other => panic!("expected `{input}` to be Parsed, got {other:?}"),
        }
    }
}

/// The point of the change: `taxon` + `" "` + `phrase` reproduces the input exactly.
#[test]
fn taxon_plus_phrase_round_trips_the_input() {
    for input in [
        "Cantuaria sp. Forster, 1968",
        "Rhodococcus sp. 14-2483-1-2",
        "Atergatopsis sp. ZRC 1999.0472",
        "Amphicynodon sp. 1 Filhol, 1881",
        "Rhizobium sp. RMCC TR1811",
        "Allium sp. 1",
        "Streptomyces sp. NBC_00448",
        "Salmonella sp. 2021_1741",
        "Trachipterus sp. HUMZ:220860",
        "Ageratum sp. MIB:SASS:0006",
    ] {
        match nameparser::parse(input, None, None, None) {
            ParseResult::Informal(inf) => {
                let round_tripped = format!(
                    "{} {}",
                    inf.taxon,
                    inf.phrase.as_deref().unwrap_or_default()
                );
                assert_eq!(round_tripped, input, "round-trip mismatch for {input:?}");
            }
            other => panic!("expected `{input}` to be Informal, got {other:?}"),
        }
    }
}

#[test]
fn australian_herbarium_locality_convention() {
    // "Genus sp. <Locality>" — the type-specimen-based convention; the locality becomes the phrase
    // instead of the 4.2.0 parser's misread "author Rocky Creek".
    assert_informal("Elaeocarpus sp. Rocky Creek")
        .taxon("Elaeocarpus")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. Rocky Creek")
        .nothing_else();
}

#[test]
fn numbered_placeholder() {
    // Phrase leading tokens are dominated by bare numbers (sp. 1, sp. 2, …).
    assert_informal("Allium sp. 1")
        .taxon("Allium")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. 1")
        .nothing_else();
}

#[test]
fn bare_genus_sp_captures_the_marker_as_phrase() {
    // A bare "Genus sp." — indeterminate, no distinguishing tag, but the verbatim marker is still
    // the phrase (uniform taxon+phrase round-trip). It stays INDETERMINED-flagged (asserted in the
    // name_tokens unit test); here we lock the phrase == the bare marker.
    assert_informal("Rhizobium sp.")
        .taxon("Rhizobium")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp.")
        .nothing_else();
}

#[test]
fn single_uppercase_letter_designator() {
    // "Genus sp. E" — a single-letter informal designator captured as the phrase.
    assert_informal("Bryozoan sp. E")
        .taxon("Bryozoan")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. E")
        .nothing_else();
}

#[test]
fn molecular_provisional_species_keep_the_whole_biological_annotation_tail() {
    // NCBI / genetic-database style: everything after "sp." is a strain / pathovar / biovar /
    // serotype / host-association annotation, NOT nomenclature — so the whole verbatim tail
    // (marker included) becomes the phrase and the anchor stays the bare genus.
    assert_informal("Solanum sp. phytoplasma")
        .taxon("Solanum")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. phytoplasma")
        .nothing_else();
    assert_informal("Citrus sp. phytoplasma")
        .taxon("Citrus")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. phytoplasma")
        .nothing_else();
    // "Alstroemeria sp. phytoplasma" is really a phytoplasma named by its host plant (host =
    // Alstroemeria sp., organism = the phytoplasma), not a species of Alstroemeria — semantically
    // distinct, but for now it parses as an Informal like the rest.
    assert_informal("Alstroemeria sp. phytoplasma")
        .taxon("Alstroemeria")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. phytoplasma")
        .nothing_else();
    // pathovar
    assert_informal("Xanthomonas sp. pv. citri")
        .taxon("Xanthomonas")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. pv. citri")
        .nothing_else();
    // biovar
    assert_informal("Pseudomonas sp. biovar 2")
        .taxon("Pseudomonas")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. biovar 2")
        .nothing_else();
    // strain designation
    assert_informal("Bacillus sp. strain ATCC 12345")
        .taxon("Bacillus")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Species)
        .phrase("sp. strain ATCC 12345")
        .nothing_else();
}

// ---- Boundary: a species epithet is present → must STAY Parsed, NOT Informal ------------------

#[test]
fn cf_binomial_stays_parsed_with_its_qualifier() {
    // A complete binomial that was only "informal" via an open-nomenclature qualifier — the
    // qualifier is an annotation (epithetQualifier), not a reclassification.
    assert_name("Salicornia cf. patula")
        .species("Salicornia", "patula")
        .type_(NameType::Informal)
        .qualifiers(&[(NamePart::Specific, "cf.")])
        .nothing_else();
}

/// A genus repeated after the qualifier is the same name written longhand: `Sorex cf. S. shinto`
/// is `Sorex cf. shinto`. The repetition — abbreviated (`S.`, `D.`) or spelled out — used to trip
/// the "capitalised word starts the authorship" boundary, so the whole binomial collapsed into a
/// uninomial with the invented author `S.shinto`, losing the epithet entirely. 83 of the 132
/// `Genus cf. Capitalised …` names in a 2.55M verbatim sample are this shape.
#[test]
fn a_genus_repeated_after_cf_is_skipped_not_read_as_an_author() {
    for (input, genus, epithet) in [
        ("Sorex cf. S. shinto", "Sorex", "shinto"),
        ("Sorex cf. Sorex shinto", "Sorex", "shinto"),
        ("Melanerpes cf. M. carolinus", "Melanerpes", "carolinus"),
        ("Diurodrilus cf. D. dohrni", "Diurodrilus", "dohrni"),
        ("Microtus cf. Microtus arvalis", "Microtus", "arvalis"),
        (
            "Archaeolagus cf. A. macrocephalus",
            "Archaeolagus",
            "macrocephalus",
        ),
        (
            "Eucytherura cf. Eucytherura complexa",
            "Eucytherura",
            "complexa",
        ),
    ] {
        assert_name(input)
            .species(genus, epithet)
            .type_(NameType::Informal)
            .qualifiers(&[(NamePart::Specific, "cf.")])
            .nothing_else();
    }
    // aff. behaves identically, keeping its own qualifier text
    assert_name("Soergelia aff. S. mayfieldi")
        .species("Soergelia", "mayfieldi")
        .type_(NameType::Informal)
        .qualifiers(&[(NamePart::Specific, "aff.")])
        .nothing_else();
}

/// Boundary: only a REPETITION of the anchor genus is skipped. A different capitalised word after
/// the qualifier is a genuinely different taxon — `Veneridae cf. Phacosoma sp` anchors on a family
/// and compares to another genus, `Onthophagus cf. Aphodius` compares two genera with no epithet at
/// all — so those keep their current reading rather than having an epithet invented for them.
#[test]
fn a_different_genus_after_cf_is_not_skipped() {
    // No species epithet is reachable, so these land in the informal band on the anchor alone —
    // unchanged by this fix, and pinned here so the skip cannot widen onto them.
    assert_informal("Onthophagus cf. Aphodius").taxon("Onthophagus");
    assert_informal("Eudoxia cf. Chelophyes contorta").taxon("Eudoxia");
}

#[test]
fn aff_binomial_with_authorship_stays_parsed() {
    // aff. on a complete binomial WITH authorship — the clearest reason it must stay Parsed: a flat
    // Informal anchor could not represent the species-level authorship.
    assert_name("Turritella aff. adulterata Deshayes 1820-1851")
        .species("Turritella", "adulterata")
        .comb_authors(Some("1820"), &["Deshayes"])
        .qualifiers(&[(NamePart::Specific, "aff.")])
        .type_(NameType::Informal);
}

#[test]
fn near_binomial_stays_parsed_with_its_qualifier() {
    // "near" is an open-nomenclature qualifier synonymous with aff. ("Poa near pratensis" = a Poa
    // near/affinis pratensis). Like cf./aff. it annotates a complete binomial, so the name stays
    // Parsed (type INFORMAL) with the qualifier in epithetQualifier — but as a full English word it
    // is stored verbatim, with NO synthesised trailing dot (unlike the abbreviations cf./aff.).
    assert_name("Poa near pratensis")
        .species("Poa", "pratensis")
        .type_(NameType::Informal)
        .qualifiers(&[(NamePart::Specific, "near")])
        .nothing_else();
}

#[test]
fn infraspecific_indeterminate_stays_parsed() {
    // "Salix alba subsp. B" has a species epithet ("alba"), so it stays Parsed — a flat Informal
    // could not hold an infraspecific-level designation hanging off a determined species.
    assert_name("Salix alba subsp. B")
        .infra_species("Salix", "alba", Rank::Subspecies, "B")
        .type_(NameType::Informal);
}

#[test]
fn binomial_with_a_trailing_annotation_currently_stays_parsed() {
    // "Persea americana phytoplasma" is a complete binomial (the host plant) + a trailing organism
    // annotation ("phytoplasma"). Ideally the annotation would be captured as a phrase like the
    // "Genus sp. phytoplasma" cases above — but with no "sp." marker the complete binomial absorbs
    // "phytoplasma" as an infraspecific epithet, so it stays SCIENTIFIC. DEFERRED: capturing a
    // trailing annotation on a bare binomial needs annotation-term recognition; this locks the
    // CURRENT behavior so the eventual change is visible in the diff.
    assert_name("Persea americana phytoplasma")
        .infra_species(
            "Persea",
            "americana",
            Rank::InfraspecificName,
            "phytoplasma",
        )
        .type_(NameType::Scientific);
}

#[test]
fn binomial_with_a_species_n_tag_stays_parsed_keeping_the_phrase() {
    // "Genus epithet species N" / "Genus epithet sp. N" — a placeholder tag appended to a binomial.
    // The species epithet is present, so it stays Parsed; the trailing tag is preserved as the phrase
    // (type INFORMAL) rather than reading "species"/"sp" as a (blacklisted) infraspecific epithet and
    // dropping the number. Surfaced by the CoL backend's dwca/17 fixture.
    assert_name("Dichanthelium chrysopsidifolium species 12")
        .species("Dichanthelium", "chrysopsidifolium")
        .type_(NameType::Informal)
        .phrase("species 12")
        .nothing_else();
    assert_name("Dichanthelium chrysopsidifolium sp. 12")
        .species("Dichanthelium", "chrysopsidifolium")
        .type_(NameType::Informal)
        .phrase("12")
        .nothing_else();
}

#[test]
fn bare_determined_genus_stays_parsed_scientific() {
    // "Rhizobium" alone is a determined SCIENTIFIC monomial — NOT informal (no provisional marker).
    assert_name("Rhizobium")
        .monomial("Rhizobium")
        .type_(NameType::Scientific)
        .nothing_else();
}

// ---- Monomial-aggregate / lineage groupings: anchored → Informal, anchorless → OTHER -----------

#[test]
fn monomial_aggregate_groups_are_rescued_to_informal() {
    // 5.0.0 rescue (see pipeline::preflight): an anchored monomial-aggregate (group/complex) or a
    // clean-genus "-lineage" becomes an Informal — the monomial is the anchor, the marker the phrase.
    assert_informal("Bartonella group")
        .taxon("Bartonella")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Unranked)
        .phrase("group")
        .nothing_else();
    assert_informal("Vermistella-lineage")
        .taxon("Vermistella")
        .taxon_rank(Rank::Genus)
        .rank(Rank::Unranked)
        .phrase("lineage")
        .nothing_else();
}

#[test]
fn anchorless_clade_and_code_labels_are_unparsable_other() {
    // Anchorless phylogenetic clade labels ("Unnamed clade") and OTU-/strain-code lineage stems
    // ("NC12A-lineage") have no clean single-taxon anchor → Unparsable(OTHER).
    for input in [
        "Amauropeltoid clade",
        "Unnamed clade",
        "NC12A-lineage",
        "he2-lineage",
    ] {
        match nameparser::parse(input, None, None, None) {
            ParseResult::Unparsable(e) => assert_eq!(
                e.type_,
                NameType::Other,
                "`{input}` should be Unparsable(OTHER)"
            ),
            other => panic!("expected `{input}` Unparsable(OTHER), got {other:?}"),
        }
    }
}

// ---- Not informal: "spec" as a genuine species epithet ---------------------------------------

#[test]
fn bare_spec_with_an_authorship_is_a_real_epithet() {
    // A handful of zoologists have actually published `spec` as a specific epithet. COL carries
    // nine of them; they are written WITHOUT the abbreviation dot and — unlike a provisional
    // `Genus spec.` — always carry an authorship. Both signals together (bare marker + a real
    // authorship) mark the word as the epithet rather than an indet marker.
    assert_name("Hemicloeina spec Platnick, 2002")
        .species("Hemicloeina", "spec")
        .comb_authors(Some("2002"), &["Platnick"])
        .code(NomCode::Zoological)
        .nothing_else();
}

#[test]
fn bare_spec_is_a_real_epithet_with_a_yearless_authorship_too() {
    // No zoological year needed — a botanical-style authorship is just as good a signal.
    assert_name("Zygonyx spec Dijkstra & Kipping")
        .species("Zygonyx", "spec")
        .comb_authors(None, &["Dijkstra", "Kipping"])
        .nothing_else();
}

#[test]
fn the_spec_epithet_rescue_needs_both_a_missing_dot_and_an_authorship() {
    // Regression guards on the two signals the rescue above hinges on — each of these fails if
    // the rule is widened. The dot makes it an abbreviation, so it stays a provisional marker —
    // and the authorship, no longer needed to mark an epithet, rides along in the phrase…
    assert_informal("Hemicloeina spec. Platnick, 2002")
        .taxon("Hemicloeina")
        .rank(Rank::Species)
        .phrase("spec. Platnick, 2002");
    // …and with no authorship there is nothing to say the word is an epithet.
    assert_informal("Globigerina spec")
        .taxon("Globigerina")
        .rank(Rank::Species)
        .phrase("spec");
    // `sp` is deliberately NOT rescued: a dot-less `sp` is overwhelmingly a sloppy `sp.`, so
    // even a real `Genus sp Author, Year` (they exist) stays indeterminate.
    assert_informal("Megakhosara sp Sharov, 1961")
        .taxon("Megakhosara")
        .rank(Rank::Species)
        .phrase("sp Sharov, 1961");
}
