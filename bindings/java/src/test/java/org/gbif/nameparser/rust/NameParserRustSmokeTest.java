// SPDX-License-Identifier: Apache-2.0
package org.gbif.nameparser.rust;

import org.gbif.nameparser.api.NamePart;
import org.gbif.nameparser.api.NameType;
import org.gbif.nameparser.api.NomCode;
import org.gbif.nameparser.api.ParseResult;
import org.gbif.nameparser.api.ParsedName;
import org.gbif.nameparser.api.Rank;
import org.gbif.nameparser.api.UnparsableNameException;
import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Map;
import java.util.Set;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
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
 * <p>Against the 5.0.0 exceptionless API {@link NameParserRust#parse} returns a {@link ParseResult}
 * — {@link ParseResult.Parsed}, {@link ParseResult.Informal} or {@link ParseResult.Unparsable} —
 * and never throws for an unparsable name. The informal cases below also lock the Java-side 3-way
 * split ({@link StructCodec#toParseResult}) against the Rust-authoritative values from the corpus
 * study.
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

  // ---- 5.0.0 informal / semistructured band (StructCodec.toParseResult, mirror of Rust) ----

  @Test
  void molecularProvisionalSpeciesIsAnInformalResult() {
    // Genus sp. <specimen tag> — the dominant informal shape (~99.8% genus-anchored, SPECIES rank).
    ParseResult result = parser.parse("Serratia sp. RE1-2a", null, null, null);

    ParseResult.Informal inf = assertInstanceOf(ParseResult.Informal.class, result);
    assertEquals("Serratia", inf.taxon());
    assertEquals(Rank.GENUS, inf.taxonRank());
    assertEquals(Rank.SPECIES, inf.rank());
    assertEquals("sp. RE1-2a", inf.phrase());
    assertEquals(NameType.INFORMAL, inf.type());
    // Informal carries no ParsedName.
    assertFalse(result.isParsable());
    assertTrue(result.parsed().isEmpty());
    assertThrows(UnparsableNameException.class, result::orElseThrow);
  }

  @Test
  void multiTokenSpecimenTagIsCapturedAsThePhrase() {
    // The 5.0.0 tag-capture enhancement (rescues the ~382k "tag not captured" corpus rows): the
    // whole verbatim tail becomes the phrase rather than being misread as an author.
    ParseResult.Informal inf =
        assertInstanceOf(ParseResult.Informal.class, parser.parse("Rhizobium sp. RMCC TR1811", null, null, null));
    assertEquals("Rhizobium", inf.taxon());
    assertEquals(Rank.GENUS, inf.taxonRank());
    assertEquals("sp. RMCC TR1811", inf.phrase());
  }

  @Test
  void bareGenusSpCarriesTheBareMarkerAsPhrase() {
    // A bare "Genus sp." carries the verbatim marker as its phrase ("sp.") — uniform taxon+phrase,
    // matching the core/CLI/Python/R (it is still flagged INDETERMINED internally).
    ParseResult.Informal inf =
        assertInstanceOf(ParseResult.Informal.class, parser.parse("Rhizobium sp.", null, null, null));
    assertEquals("Rhizobium", inf.taxon());
    assertEquals(Rank.SPECIES, inf.rank());
    assertEquals("sp.", inf.phrase());
  }

  @Test
  void cfBinomialStaysParsedNotInformal() throws UnparsableNameException {
    // A species epithet is present (a binomial core), so it stays Parsed — the qualifier is an
    // annotation in epithetQualifier, and specificAuthorship (unrepresentable by a flat anchor) is kept.
    ParseResult result = parser.parse("Salicornia cf. patula", null, null, null);

    ParseResult.Parsed parsed = assertInstanceOf(ParseResult.Parsed.class, result);
    assertEquals("patula", parsed.name().getSpecificEpithet());
    assertEquals("cf.", parsed.name().getEpithetQualifier(NamePart.SPECIFIC));
    assertTrue(result.isParsable());
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
    assertTrue(pn.getCombinationAuthorship().getAuthors().isEmpty(),
        "the accession must not be parsed as an author, got " + pn.getCombinationAuthorship().getAuthors());
  }
}
