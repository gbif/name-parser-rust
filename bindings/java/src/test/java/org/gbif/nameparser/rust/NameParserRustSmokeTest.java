// SPDX-License-Identifier: Apache-2.0
package org.gbif.nameparser.rust;

import org.gbif.nameparser.api.NamePart;
import org.gbif.nameparser.api.NameType;
import org.gbif.nameparser.api.NomCode;
import org.gbif.nameparser.api.ParsedName;
import org.gbif.nameparser.api.Rank;
import org.gbif.nameparser.api.UnparsableNameException;
import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Map;
import java.util.Set;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
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
 */
class NameParserRustSmokeTest {

  private final NameParserRust parser = new NameParserRust();

  @Test
  void parsesAVulpesSubspeciesWithCombinationAuthorship() throws UnparsableNameException {
    ParsedName pn = parser.parse("Vulpes vulpes silaceus Miller, 1907", null, null, null);

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
    ParsedName pn = parser.parse("Abies alba", "Mill.", Rank.SPECIES, NomCode.BOTANICAL);

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
  void unparsableVirusNameThrowsWithTypeAndName() {
    UnparsableNameException ex = assertThrows(UnparsableNameException.class,
        () -> parser.parse("Tobacco mosaic virus", null, null, null));

    assertEquals(NameType.OTHER, ex.getType());
    assertEquals(NomCode.VIRUS, ex.getCode());
    assertEquals("Tobacco mosaic virus", ex.getName());
  }

  @Test
  void hybridNothoSetIsDecodedFromTheStructWire() throws UnparsableNameException {
    ParsedName pn = parser.parse("Salix ×capreola", null, null, null);

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
    ParsedName pn = parser.parse("Turritella aff. adulterata Deshayes 1820-1851", null, null, null);

    assertEquals("Turritella", pn.getGenus());
    assertEquals("adulterata", pn.getSpecificEpithet());
    assertEquals("aff.", pn.getEpithetQualifier(NamePart.SPECIFIC));
    assertEquals(Map.of(NamePart.SPECIFIC, "aff."), pn.getEpithetQualifier());
  }

  @Test
  void multiAuthorNameWithAWarningIsDecodedFromTheStructWire() throws UnparsableNameException {
    ParsedName pn = parser.parse("Gahrliepia (G.) tessellata Traub & Morrow 1955", null, null, null);

    assertEquals("Gahrliepia", pn.getGenus());
    assertEquals("tessellata", pn.getSpecificEpithet());
    assertEquals(List.of("Traub", "Morrow"), pn.getCombinationAuthorship().getAuthors());
    assertEquals("1955", pn.getCombinationAuthorship().getYear());
    // warnings is a Set<String> — must not be dropped or truncated by the struct round trip.
    assertFalse(pn.getWarnings().isEmpty(), "expected at least one warning");
    assertTrue(pn.getWarnings().contains("abbreviated subgenus name"),
        "got warnings " + pn.getWarnings());
  }
}
