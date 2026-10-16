// SPDX-License-Identifier: Apache-2.0
package org.gbif.nameparser.rust;

import org.gbif.nameparser.api.NamePart;
import org.gbif.nameparser.api.NameType;
import org.gbif.nameparser.api.NomCode;
import org.gbif.nameparser.api.ParseResult;
import org.gbif.nameparser.api.ParsedName;
import org.gbif.nameparser.api.Rank;
import org.gbif.nameparser.api.UnparsableNameException;
import org.gbif.nameparser.api.Warnings;
import org.gbif.nameparser.util.NameFormatter;
import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Map;
import java.util.Set;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * End-to-end tests of {@link NameParserRust} over the real FFM boundary: every assertion here
 * exercises the actual {@code nameparser-ffi} cdylib (built by {@code cargo build -p
 * nameparser-ffi --release}), not a mock. Run via {@code mvn -f bindings/java/pom.xml test} (the
 * cdylib path and {@code --enable-native-access} are wired in by the surefire {@code argLine} in
 * {@code pom.xml}).
 *
 * <p>The binding carries a single wire format — the flat fixed-layout binary struct decoded by
 * {@link StructCodec} — so these tests double as a check that the collection-typed fields
 * ({@code notho}, {@code epithetQualifier}, {@code warnings}, author lists) survive that decode
 * and its setter-based rebuild intact.
 *
 * <p>Against the exceptionless API {@link NameParserRust#parse} returns a {@link ParseResult} —
 * {@link ParseResult.Parsed}, informal names included, or {@link ParseResult.Unparsable} — and never
 * throws for an unparsable name. The informal cases below also check {@link ParsedName#anchor()}
 * on the decoded names.
 */
class NameParserRustSmokeTest {

  private final NameParserRust parser = new NameParserRust();

  @Test
  void parsesAVulpesSubspeciesWithCombinationAuthorship() throws UnparsableNameException {
    ParsedName pn = parser.parse("Vulpes vulpes silaceus Miller, 1907", null, null, null).orElseThrow();

    assertEquals("Vulpes", pn.getGenus());
    assertEquals("vulpes", pn.getSpecificEpithet());
    assertEquals("silaceus", pn.getInfraspecificEpithet());
    assertEquals(Rank.SUBSPECIES, pn.getRank());
    assertNotNull(pn.getCombinationAuthorship());
    assertEquals("1907", pn.getCombinationAuthorship().getYear());
    assertTrue(pn.getCombinationAuthorship().getAuthors().contains("Miller"),
        "combination authors should contain Miller, got " + pn.getCombinationAuthorship().getAuthors());
  }

  @Test
  void explicitAuthorshipRankAndCodeArgumentsAreMarshalledAndAttached() throws UnparsableNameException {
    ParsedName pn = parser.parse("Abies alba", "Mill.", Rank.SPECIES, NomCode.BOTANICAL).orElseThrow();

    assertEquals("Abies", pn.getGenus());
    assertEquals("alba", pn.getSpecificEpithet());
    assertEquals(Rank.SPECIES, pn.getRank());
    assertEquals(NomCode.BOTANICAL, pn.getCode());
    assertTrue(pn.getCombinationAuthorship().hasAuthors(),
        "explicit authorship 'Mill.' should have attached as a combination author");
    assertTrue(pn.getCombinationAuthorship().getAuthors().contains("Mill."),
        "got authors " + pn.getCombinationAuthorship().getAuthors());
  }

  @Test
  void anonymousAuthorshipCrossesTheWireAndRendersPerCode() throws UnparsableNameException {
    // ABI 5: Authorship.anonymous, attributed authors in square brackets (ICZN Rec. 51D)
    ParsedName pn = parser.parse("Rhinobatos typus (Anonymous [Bennett], 1830)", null, null, null).orElseThrow();
    assertTrue(pn.getBasionymAuthorship().isAnonymous());
    assertEquals(List.of("Bennett"), pn.getBasionymAuthorship().getAuthors());
    assertEquals("([Bennett], 1830)", pn.authorshipComplete());

    ParsedName bot = parser.parse("Physalospora rubiginosa (Fr.) anon.", null, null, null).orElseThrow();
    assertTrue(bot.getCombinationAuthorship().isAnonymous());
    assertFalse(bot.getCombinationAuthorship().hasAuthors());
    assertEquals(NomCode.BOTANICAL, bot.getCode());
    assertEquals("(Fr.) anon.", bot.authorshipComplete());
  }

  @Test
  void bracketedYearsCrossTheWire() throws UnparsableNameException {
    // ABI 7: Authorship.bracketedYear, a year from external evidence (ICZN Rec. 22A.2.3)
    ParsedName pn = parser.parse("Phyllomacromia", "Westwood, [1851]", null, null).orElseThrow();
    assertTrue(pn.getCombinationAuthorship().isBracketedYear());
    assertEquals("1851", pn.getCombinationAuthorship().getYear());
    assertEquals("Westwood, [1851]", pn.authorshipComplete());

    ParsedName bas = parser.parse("Acleris aspersana (Hübner, [1819])", null, null, null).orElseThrow();
    assertTrue(bas.getBasionymAuthorship().isBracketedYear());
    assertEquals("(Hübner, [1819])", bas.authorshipComplete());

    ParsedName imprint = parser.parse("Ctenotus alacer Storr, 1970 [1969]", null, null, null).orElseThrow();
    assertFalse(imprint.getCombinationAuthorship().isBracketedYear());
    assertEquals("1969", imprint.getCombinationAuthorship().getImprintYear());
  }

  @Test
  void inAuthorsCrossTheWire() throws UnparsableNameException {
    // ABI 8: Authorship.inAuthors, the authors of the work a name was published in
    ParsedName pn = parser.parse("Aus bus (Smith in Jones, 1900) Brown in Miller & Lee, 1920", null, null, null)
        .orElseThrow();
    assertEquals(List.of("Jones"), pn.getBasionymAuthorship().getInAuthors());
    assertEquals(List.of("Miller", "Lee"), pn.getCombinationAuthorship().getInAuthors());
    assertEquals("Miller & Lee, 1920", pn.getPublishedIn());
    assertEquals("(Smith in Jones, 1900) Brown in Miller & Lee, 1920", pn.authorshipComplete());

    ParsedName sep = parser.parse("Atriplex prostrata", "Boucher ex DC. in Lam. & DC.", null, null).orElseThrow();
    assertEquals(List.of("Lam.", "DC."), sep.getCombinationAuthorship().getInAuthors());
    assertEquals("Boucher ex DC. in Lam. & DC.", sep.authorshipComplete());

    // a reference has no in-authors, and a name without any allocates no list
    ParsedName ref = parser.parse("Abies alba Mill. in Gard. Dict. ed. 8", null, null, null).orElseThrow();
    assertNull(ref.getCombinationAuthorship().getInAuthors());
    assertNull(ref.getBasionymAuthorship().getInAuthors());
    assertEquals("Gard. Dict. ed. 8", ref.getPublishedIn());
  }

  @Test
  void emptyAuthorListsStayNull() {
    // gbif/name-parser#109: an author list the name does not have is never allocated
    ParsedName pn = parser.parse("Abies alba Mill.", null, null, null).orElseThrow();
    assertEquals(List.of("Mill."), pn.getCombinationAuthorship().getAuthors());
    assertNull(pn.getCombinationAuthorship().getExAuthors());
    assertNull(pn.getCombinationAuthorship().getInAuthors());
    assertNull(pn.getBasionymAuthorship().getAuthors());
    assertNull(pn.getBasionymAuthorship().getExAuthors());
    assertFalse(pn.getBasionymAuthorship().exists());

    ParsedName ex = parser.parse("Acacia aneura F.Muell. ex Benth.", null, null, null).orElseThrow();
    assertEquals(List.of("F.Muell."), ex.getCombinationAuthorship().getExAuthors());
  }

  @Test
  void sanctioningAuthorsCrossTheWireOnEachAuthorship() throws UnparsableNameException {
    // ABI 6: the basionym's sanctioning author is kept inside its brackets (ICN Art. 15)
    ParsedName pn = parser.parse("Merulius lacrimans (Wulfen : Fr.) Schum.", null, null, null).orElseThrow();
    assertEquals("Fr.", pn.getBasionymAuthorship().getSanctioningAuthor());
    assertNull(pn.getCombinationAuthorship().getSanctioningAuthor());
    assertEquals(NomCode.BOTANICAL, pn.getCode());
    assertEquals("(Wulfen : Fr.) Schum.", pn.authorshipComplete());

    ParsedName comb = parser.parse("Boletus versicolor L. : Fr.", null, null, null).orElseThrow();
    assertEquals("Fr.", comb.getCombinationAuthorship().getSanctioningAuthor());
    assertNull(comb.getBasionymAuthorship().getSanctioningAuthor());
    assertEquals("L. : Fr.", comb.authorshipComplete());
  }

  @Test
  void unparsableVirusNameIsAnUnparsableResultCarryingTypeCodeAndName() {
    // 5.0.0: no throw — the result IS an Unparsable variant carrying the classification.
    ParseResult result = parser.parse("Tobacco mosaic virus", null, null, null);

    ParseResult.Unparsable u = assertInstanceOf(ParseResult.Unparsable.class, result);
    assertEquals(NameType.OTHER, u.type());
    assertEquals(NomCode.VIRUS, u.code());
    assertEquals("Tobacco mosaic virus", u.name());
    assertFalse(result.isParsable());
    assertTrue(result.parsed().isEmpty());
    // orElseThrow() is the opt-in fail-fast bridge for callers that want the exception style.
    assertThrows(UnparsableNameException.class, result::orElseThrow);
  }

  @Test
  void unparsableOtuNameIsReturnedInItsCanonicalUppercaseForm() {
    // The Rust core canonicalizes a UNITE species-hypothesis id to uppercase, and that canonicalized
    // ParseError.name survives the FFI — so the binding returns "SH..." for a lowercase "sh..."
    // input. 5.0.0 also reclassifies it OTHER -> IDENTIFIER (an anchorless machine identifier).
    ParseResult result = parser.parse("sh19186714.17fu", null, null, null);

    ParseResult.Unparsable u = assertInstanceOf(ParseResult.Unparsable.class, result);
    assertEquals(NameType.IDENTIFIER, u.type());
    assertEquals("SH19186714.17FU", u.name());
  }

  @Test
  void hybridNothoSetIsDecodedFromTheStructWire() throws UnparsableNameException {
    ParsedName pn = parser.parse("Salix ×capreola", null, null, null).orElseThrow();

    assertEquals("Salix", pn.getGenus());
    assertEquals("capreola", pn.getSpecificEpithet());
    assertNotNull(pn.getNotho(), "notho must not be null for a hybrid name");
    // EnumSet<NamePart> — a collection-typed field StructCodec decodes and repopulates via setters.
    assertEquals(Set.of(NamePart.SPECIFIC), pn.getNotho());
  }

  @Test
  void epithetQualifierIsDecodedFromTheStructWire() throws UnparsableNameException {
    // "aff." (affinis) lives in the EnumMap<NamePart, String> epithetQualifier field — another
    // collection-typed field the struct wire must carry and StructCodec must repopulate.
    ParsedName pn = parser.parse("Turritella aff. adulterata Deshayes 1820-1851", null, null, null).orElseThrow();

    assertEquals("Turritella", pn.getGenus());
    assertEquals("adulterata", pn.getSpecificEpithet());
    assertEquals("aff.", pn.getEpithetQualifier(NamePart.SPECIFIC));
    assertEquals(Map.of(NamePart.SPECIFIC, "aff."), pn.getEpithetQualifier());
  }

  @Test
  void multiAuthorNameWithAWarningIsDecodedFromTheStructWire() throws UnparsableNameException {
    ParsedName pn = parser.parse("Gahrliepia (G.) tessellata Traub & Morrow 1955", null, null, null).orElseThrow();

    assertEquals("Gahrliepia", pn.getGenus());
    assertEquals("tessellata", pn.getSpecificEpithet());
    assertEquals(List.of("Traub", "Morrow"), pn.getCombinationAuthorship().getAuthors());
    assertEquals("1955", pn.getCombinationAuthorship().getYear());
    // warnings is a Set<String> — must not be dropped or truncated by the struct round trip.
    assertFalse(pn.getWarnings().isEmpty(), "expected at least one warning");
    assertTrue(pn.getWarnings().contains("abbreviated subgenus name"),
        "got warnings " + pn.getWarnings());
  }

  // ---- informal names: parsed, their anchor derived from the decoded ParsedName ----

  @Test
  void molecularProvisionalSpeciesIsAParsedInformalName() {
    // Genus sp. <specimen tag> — the dominant informal shape (~99.8% genus-anchored, SPECIES rank).
    ParseResult result = parser.parse("Serratia sp. RE1-2a", null, null, null);

    ParsedName pn = assertInstanceOf(ParseResult.Parsed.class, result).name();
    assertEquals(NameType.INFORMAL, result.type());
    assertEquals("Serratia", pn.getGenus());
    assertEquals(Rank.SPECIES, pn.getRank());
    assertEquals("sp. RE1-2a", pn.getPhrase());
    assertTrue(pn.isIndetermined());
    ParsedName anchor = pn.anchor().orElseThrow();
    assertEquals("Serratia", anchor.getUninomial());
    assertEquals(Rank.GENUS, anchor.getRank());
    assertEquals(NameType.SCIENTIFIC, anchor.getType());
  }

  @Test
  void multiTokenSpecimenTagIsCapturedAsThePhrase() {
    // The 5.0.0 tag-capture enhancement (rescues the ~382k "tag not captured" corpus rows): the
    // whole verbatim tail becomes the phrase rather than being misread as an author.
    ParsedName pn = parser.parse("Rhizobium sp. RMCC TR1811", null, null, null).orElseThrow();
    assertEquals("Rhizobium", pn.getGenus());
    assertEquals("sp. RMCC TR1811", pn.getPhrase());
    assertFalse(pn.hasAuthorship());
  }

  @Test
  void bareGenusSpCarriesTheBareMarkerAsPhrase() {
    // A bare "Genus sp." carries the verbatim marker as its phrase ("sp."), flagged INDETERMINED.
    ParsedName pn = parser.parse("Rhizobium sp.", null, null, null).orElseThrow();
    assertEquals("Rhizobium", pn.getGenus());
    assertEquals(Rank.SPECIES, pn.getRank());
    assertEquals("sp.", pn.getPhrase());
    assertTrue(pn.getWarnings().contains(Warnings.INDETERMINED));
    assertFalse(pn.isCandidatus());
  }

  @Test
  void informalKeepsCandidatus() {
    // #76: Candidatus is a status of the name the provisional designation belongs to
    ParsedName pn = parser.parse("Candidatus Pelagibacter sp. HTCC7211", null, null, null).orElseThrow();
    assertEquals("Pelagibacter", pn.getGenus());
    assertEquals("sp. HTCC7211", pn.getPhrase());
    assertTrue(pn.isCandidatus());
    assertTrue(pn.anchor().orElseThrow().isCandidatus());
    assertEquals("\"Candidatus Pelagibacter\" sp. HTCC7211", NameFormatter.canonical(pn));
  }

  @Test
  void phraseNameOfASpeciesIsAnchoredOnTheSpecies() {
    // #134: an Australian phrase name below a species
    ParsedName pn = parser.parse("Acacia mutabilis Maslin subsp. Young River (G.F.Craig 2052)", null, null, null)
        .orElseThrow();
    assertEquals(NameType.INFORMAL, pn.getType());
    assertEquals(Rank.SUBSPECIES, pn.getRank());
    assertEquals("Young River (G.F.Craig 2052)", pn.getPhrase());
    ParsedName anchor = pn.anchor().orElseThrow();
    assertEquals("Acacia mutabilis Maslin", anchor.canonicalName());
    assertEquals(Rank.SPECIES, anchor.getRank());
  }

  @Test
  void cfBinomialKeepsItsQualifiedEpithet() throws UnparsableNameException {
    // The qualifier is an annotation in epithetQualifier; the anchor stops above the qualified epithet.
    ParseResult result = parser.parse("Salicornia cf. patula", null, null, null);

    ParseResult.Parsed parsed = assertInstanceOf(ParseResult.Parsed.class, result);
    assertEquals("patula", parsed.name().getSpecificEpithet());
    assertEquals("cf.", parsed.name().getEpithetQualifier(NamePart.SPECIFIC));
    assertEquals("Salicornia", parsed.name().anchor().orElseThrow().getUninomial());
    assertTrue(result.isParsable());
  }

  @Test
  void infraspecificEpithetUnderBareGenusKeepsItsAuthor() throws UnparsableNameException {
    // The species is missing (INFORMAL); the epithet keeps its author.
    ParseResult result = parser.parse("Navicula var. fasciata Grunow", null, null, null);

    ParseResult.Parsed parsed = assertInstanceOf(ParseResult.Parsed.class, result);
    assertEquals("Navicula", parsed.name().getGenus());
    assertEquals("fasciata", parsed.name().getInfraspecificEpithet());
    assertEquals(Rank.VARIETY, parsed.name().getRank());
    assertEquals(NameType.INFORMAL, parsed.name().getType());
    assertEquals("Grunow", parsed.name().getCombinationAuthorship().getAuthors().get(0));
  }

  // ---- 5.0.0 NameType.IDENTIFIER band (Part A) + trailing culture accession (Part B) ----------

  @Test
  void boldBinIsAnUnparsableIdentifierResult() {
    // 5.0.0: an anchorless machine identifier (BOLD BIN) is NameType.IDENTIFIER, carried by the
    // Unparsable variant across the ABI-4 struct wire (was OTHER pre-5.0). Exercises the new
    // ordinal (IDENTIFIER=4, OTHER shifted to 5) round-tripping through StructCodec.
    ParseResult result = parser.parse("BOLD:AAA0001", null, null, null);

    ParseResult.Unparsable u = assertInstanceOf(ParseResult.Unparsable.class, result);
    assertEquals(NameType.IDENTIFIER, u.type());
    assertEquals("BOLD:AAA0001", u.name());
    assertFalse(result.isParsable());
  }

  @Test
  void standaloneCultureCollectionAccessionIsAnIdentifier() {
    ParseResult.Unparsable u =
        assertInstanceOf(ParseResult.Unparsable.class, parser.parse("DSM 10", null, null, null));
    assertEquals(NameType.IDENTIFIER, u.type());
  }

  @Test
  void trailingCultureCollectionAccessionBecomesThePhrase() throws UnparsableNameException {
    // "DSM 19832" is a strain annotation, not an author — captured verbatim (acronym included) as
    // the phrase; the binomial core stays Parsed with type INFORMAL, and "DSM" is NOT an author.
    ParsedName pn = parser.parse("Aquimarina muelleri DSM 19832", null, null, null).orElseThrow();

    assertEquals("Aquimarina", pn.getGenus());
    assertEquals("muelleri", pn.getSpecificEpithet());
    assertEquals("DSM 19832", pn.getPhrase());
    assertEquals(NameType.INFORMAL, pn.getType());
    assertFalse(pn.getCombinationAuthorship().hasAuthors(),
        "the accession must not be parsed as an author, got " + pn.getCombinationAuthorship().getAuthors());
  }
}
