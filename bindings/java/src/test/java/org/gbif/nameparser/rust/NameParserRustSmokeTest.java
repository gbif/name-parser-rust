// SPDX-License-Identifier: Apache-2.0
package org.gbif.nameparser.rust;

import com.google.gson.JsonObject;

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
 * nameparser-ffi --release}), not a mock. Run via {@code mvn -f bindings/java/pom.xml test}
 * (the cdylib path and {@code --enable-native-access} are wired in by the surefire
 * {@code argLine} in {@code pom.xml}).
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
  void hybridNothoSetSurvivesAGsonRoundTrip() throws UnparsableNameException {
    ParsedName pn = parser.parse("Salix ×capreola", null, null, null);

    assertEquals("Salix", pn.getGenus());
    assertEquals("capreola", pn.getSpecificEpithet());
    assertNotNull(pn.getNotho(), "notho must not be null for a hybrid name");
    assertEquals(Set.of(NamePart.SPECIFIC), pn.getNotho());

    // Re-serialize with the exact same Gson instance NameParserRust used to build `pn`, then
    // rebuild a second ParsedName from that — a full round trip, not just the one-way
    // deserialization already exercised above. This is the fidelity check Task 2's brief
    // calls for: if EnumSet<NamePart> can't survive Gson's default reflection, it would show
    // up here as a dropped/garbled `notho` on the second hop.
    JsonObject reserialized = NameParserRust.GSON.toJsonTree(pn).getAsJsonObject();
    ParsedName roundTripped = NameParserRust.GSON.fromJson(reserialized, ParsedName.class);
    assertEquals(Set.of(NamePart.SPECIFIC), roundTripped.getNotho(),
        "notho must survive a second Gson round trip identically");
  }

  @Test
  void epithetQualifierSurvivesAGsonRoundTrip() throws UnparsableNameException {
    // "aff." (affinis) is stored in the EnumMap<NamePart, String> epithetQualifier field —
    // named as a specific round-trip risk alongside notho/warnings (both abstract
    // collection-ish types Gson's reflective ConstructorConstructor might not know how to
    // instantiate on deserialization; EnumSet has a documented special case, EnumMap's
    // handling is the one this test actually pins down empirically).
    ParsedName pn = parser.parse("Turritella aff. adulterata Deshayes 1820-1851", null, null, null);

    assertEquals("Turritella", pn.getGenus());
    assertEquals("adulterata", pn.getSpecificEpithet());
    assertEquals("aff.", pn.getEpithetQualifier(NamePart.SPECIFIC));
    assertEquals(Map.of(NamePart.SPECIFIC, "aff."), pn.getEpithetQualifier());

    JsonObject reserialized = NameParserRust.GSON.toJsonTree(pn).getAsJsonObject();
    ParsedName roundTripped = NameParserRust.GSON.fromJson(reserialized, ParsedName.class);
    assertEquals(Map.of(NamePart.SPECIFIC, "aff."), roundTripped.getEpithetQualifier(),
        "epithetQualifier must survive a second Gson round trip identically");
  }

  @Test
  void multiAuthorNameWithAWarningSurvivesAGsonRoundTrip() throws UnparsableNameException {
    ParsedName pn = parser.parse("Gahrliepia (G.) tessellata Traub & Morrow 1955", null, null, null);

    assertEquals("Gahrliepia", pn.getGenus());
    assertEquals("tessellata", pn.getSpecificEpithet());
    assertEquals(List.of("Traub", "Morrow"), pn.getCombinationAuthorship().getAuthors());
    assertEquals("1955", pn.getCombinationAuthorship().getYear());
    assertFalse(pn.getWarnings().isEmpty(), "expected at least one warning");
    assertTrue(pn.getWarnings().contains("abbreviated subgenus name"),
        "got warnings " + pn.getWarnings());

    // Same full-round-trip fidelity check as the hybrid test above, this time for the
    // `authors` list (List<String>, straightforward) and `warnings` (Set<String>, a HashSet
    // in Java — order-insensitive by construction, but must not be dropped or truncated).
    JsonObject reserialized = NameParserRust.GSON.toJsonTree(pn).getAsJsonObject();
    ParsedName roundTripped = NameParserRust.GSON.fromJson(reserialized, ParsedName.class);
    assertEquals(List.of("Traub", "Morrow"), roundTripped.getCombinationAuthorship().getAuthors());
    assertEquals(pn.getWarnings(), roundTripped.getWarnings());
  }
}
