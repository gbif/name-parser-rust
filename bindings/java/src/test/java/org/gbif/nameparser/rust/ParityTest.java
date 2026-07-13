// SPDX-License-Identifier: Apache-2.0
package org.gbif.nameparser.rust;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonNull;
import com.google.gson.JsonObject;

import org.gbif.nameparser.api.ParseResult;
import org.junit.jupiter.api.Test;

import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Locale;
import java.util.Objects;
import java.util.Set;
import java.util.TreeSet;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * Regression-snapshot parity gate: {@link NameParserRust} (over the real FFM boundary) vs the Rust
 * golden snapshot {@code testdata/golden/expected-parse.jsonl} — the same snapshot
 * {@code parse_golden.rs} and the R binding validate against, regenerated from the Rust CLI (the Java
 * {@code NameParserImpl} is gone, so it is the current-Rust output, not a Java oracle). It
 * re-validates the whole {@link StructCodec} decode over the ~8k-name benchmark corpus (the
 * {@code NameParserRustSmokeTest} only spot-checks a handful).
 *
 * <p>The snapshot is the raw core {@code parse()} output (a {@code ParsedName}, or an error), while
 * {@link NameParserRust} returns the 5.0.0 three-way {@link ParseResult}. So each row is mapped
 * through the SAME split + clamp the binding applies — deliberate mirrors of the Rust core's
 * {@code is_informal} / {@code to_informal} / {@code ParseError::clamped_to_unparsable} — before
 * comparing. The engine output underneath is identical, so this is a 0-diff gate.
 *
 * <p>Parsed rows compare as full {@code ParsedName} JSON trees (via one {@link #GSON}), with
 * {@code warnings}/{@code notho}/{@code epithetQualifier} compared order-insensitively — the same
 * carve-out {@code parse_golden.rs} uses ({@link #UNORDERED_FIELD_KEYS}).
 */
class ParityTest {

  private static final File GOLDEN =
      new File(System.getProperty("nameparser.testdata.dir", "../../testdata"),
          "golden/expected-parse.jsonl");

  /** Java {@code HashSet} vs Rust {@code Vec} order, and enum-map order — compared as sets. Copied
   *  verbatim from {@code parse_golden.rs}'s / the CLI's {@code UNORDERED_FIELD_KEYS}. */
  private static final Set<String> UNORDERED_FIELD_KEYS =
      Set.of("warnings", "notho", "epithetQualifier");

  private static final Gson GSON = new GsonBuilder().create();
  private static final int MAX_EXAMPLES = 20;

  private final NameParserRust rust = new NameParserRust();

  @Test
  void matchesTheRustGoldenSnapshotOverTheBenchmarkCorpus() throws IOException {
    assertTrue(GOLDEN.isFile(), "golden snapshot not found: " + GOLDEN.getAbsolutePath()
        + " -- set -Dnameparser.testdata.dir=<repo>/testdata");

    List<String> lines = Files.readAllLines(GOLDEN.toPath(), StandardCharsets.UTF_8);
    int compared = 0;
    int diffs = 0;
    List<String> examples = new ArrayList<>();

    for (String line : lines) {
      if (line.isBlank()) {
        continue;
      }
      JsonObject row = GSON.fromJson(line, JsonObject.class);
      String input = row.get("input").getAsString();
      compared++;

      String mismatch = compare(rust.parse(input, null, null, null), row);
      if (mismatch != null) {
        diffs++;
        if (examples.size() < MAX_EXAMPLES) {
          examples.add("\"" + input + "\": " + mismatch);
        }
      }
    }

    System.out.println(String.format(Locale.ROOT,
        "ParityTest (NameParserRust vs Rust golden snapshot): %d compared, %d diffs", compared, diffs));
    assertTrue(compared >= 8000, "only " + compared + " rows compared — is the golden truncated?");
    if (diffs > 0) {
      System.err.println("First " + examples.size() + " of " + diffs + " diff(s):");
      examples.forEach(System.err::println);
    }
    assertEquals(0, diffs, diffs + " row(s) differ from the Rust golden snapshot — see stdout/stderr");
  }

  /**
   * Compare the binding's 5.0.0 {@link ParseResult} to one frozen oracle row (raw {@code parse()}
   * output), mapping the oracle through the same three-way split + clamp. Returns {@code null} on a
   * match, else a short diff description.
   */
  private static String compare(ParseResult got, JsonObject row) {
    boolean oracleError = row.has("error");
    JsonObject oracleParsed = row.has("parsed") ? row.getAsJsonObject("parsed") : null;

    if (got instanceof ParseResult.Unparsable u) {
      if (!oracleError) {
        return "binding Unparsable but oracle parsed";
      }
      JsonObject err = row.getAsJsonObject("error");
      String oType = clampType(err.get("type").getAsString());
      String gType = u.type().name();
      if (!Objects.equals(gType, oType)) {
        return "error type: binding=" + gType + " oracle=" + oType;
      }
      String oCode = err.has("code") ? err.get("code").getAsString() : null;
      String gCode = u.code() == null ? null : u.code().name();
      if (!Objects.equals(gCode, oCode)) {
        return "error code: binding=" + gCode + " oracle=" + oCode;
      }
      return null;
    }
    if (oracleError) {
      return "binding parsed but oracle errored";
    }

    if (got instanceof ParseResult.Informal inf) {
      if (!oracleIsInformal(oracleParsed)) {
        return "binding Informal but oracle is a plain ParsedName";
      }
      return compareInformal(inf, oracleParsed);
    }

    ParseResult.Parsed p = (ParseResult.Parsed) got;
    if (oracleIsInformal(oracleParsed)) {
      return "binding Parsed but oracle is informal-band";
    }
    JsonObject gJson = GSON.toJsonTree(p.name()).getAsJsonObject();
    return jsonEquals(gJson, oracleParsed) ? null : "ParsedName field diff (see the two JSON trees)";
  }

  /** Mirror of Rust {@code is_informal} / {@link StructCodec}'s {@code isInformal}, on the oracle
   *  JSON: an INFORMAL-typed ParsedName with a real anchor and no species epithet. */
  private static boolean oracleIsInformal(JsonObject p) {
    if (p == null) {
      return false;
    }
    String type = p.has("type") ? p.get("type").getAsString() : null;
    boolean hasSpecific = p.has("specificEpithet");
    boolean hasAnchor = p.has("genus") || p.has("uninomial") || p.has("infragenericEpithet");
    return "INFORMAL".equals(type) && !hasSpecific && hasAnchor;
  }

  /** Mirror of Rust {@code to_informal}: compare the binding's flat Informal to the anchor derived
   *  from the oracle ParsedName. */
  private static String compareInformal(ParseResult.Informal inf, JsonObject p) {
    String taxon;
    String taxonRank;
    if (p.has("genus")) {
      taxon = p.get("genus").getAsString();
      taxonRank = "GENUS";
    } else if (p.has("uninomial")) {
      taxon = p.get("uninomial").getAsString();
      taxonRank = p.get("rank").getAsString();
    } else {
      taxon = p.get("infragenericEpithet").getAsString();
      taxonRank = p.get("rank").getAsString();
    }
    String rank = p.has("rank") ? p.get("rank").getAsString() : null;
    String phrase = p.has("phrase") ? p.get("phrase").getAsString() : null;
    String code = p.has("code") ? p.get("code").getAsString() : null;

    if (!Objects.equals(inf.taxon(), taxon)) {
      return "taxon: binding=" + inf.taxon() + " oracle=" + taxon;
    }
    if (!Objects.equals(inf.taxonRank().name(), taxonRank)) {
      return "taxonRank: binding=" + inf.taxonRank() + " oracle=" + taxonRank;
    }
    if (!Objects.equals(inf.rank().name(), rank)) {
      return "rank: binding=" + inf.rank() + " oracle=" + rank;
    }
    if (!Objects.equals(inf.phrase(), phrase)) {
      return "phrase: binding=" + inf.phrase() + " oracle=" + phrase;
    }
    String gCode = inf.code() == null ? null : inf.code().name();
    if (!Objects.equals(gCode, code)) {
      return "code: binding=" + gCode + " oracle=" + code;
    }
    return null;
  }

  /** Mirror of {@code ParseError::clamped_to_unparsable}: a parsable oracle error type becomes OTHER. */
  private static String clampType(String t) {
    return ("INFORMAL".equals(t) || "SCIENTIFIC".equals(t)) ? "OTHER" : t;
  }

  // ---------------------------------------------------------------------------------------
  // JSON structural equality (verbatim from the retired live-oracle ParityTest) — the
  // order-insensitive carve-out for UNORDERED_FIELD_KEYS is applied at every nesting depth.
  // ---------------------------------------------------------------------------------------

  private static boolean jsonEquals(JsonElement a, JsonElement b) {
    boolean aNull = a == null || a.isJsonNull();
    boolean bNull = b == null || b.isJsonNull();
    if (aNull || bNull) {
      return aNull && bNull;
    }
    if (a.isJsonObject() && b.isJsonObject()) {
      JsonObject oa = a.getAsJsonObject();
      JsonObject ob = b.getAsJsonObject();
      Set<String> keys = new TreeSet<>();
      keys.addAll(oa.keySet());
      keys.addAll(ob.keySet());
      for (String key : keys) {
        JsonElement va = oa.has(key) ? oa.get(key) : JsonNull.INSTANCE;
        JsonElement vb = ob.has(key) ? ob.get(key) : JsonNull.INSTANCE;
        boolean fieldEqual = UNORDERED_FIELD_KEYS.contains(key) && va.isJsonArray() && vb.isJsonArray()
            ? unorderedArraysEqual(va.getAsJsonArray(), vb.getAsJsonArray())
            : jsonEquals(va, vb);
        if (!fieldEqual) {
          return false;
        }
      }
      return true;
    }
    if (a.isJsonArray() && b.isJsonArray()) {
      JsonArray aa = a.getAsJsonArray();
      JsonArray ab = b.getAsJsonArray();
      if (aa.size() != ab.size()) {
        return false;
      }
      for (int i = 0; i < aa.size(); i++) {
        if (!jsonEquals(aa.get(i), ab.get(i))) {
          return false;
        }
      }
      return true;
    }
    return a.equals(b);
  }

  private static boolean unorderedArraysEqual(JsonArray a, JsonArray b) {
    if (a.size() != b.size()) {
      return false;
    }
    return renderedSorted(a).equals(renderedSorted(b));
  }

  private static List<String> renderedSorted(JsonArray array) {
    List<String> rendered = new ArrayList<>(array.size());
    for (JsonElement e : array) {
      rendered.add(e.toString());
    }
    Collections.sort(rendered);
    return rendered;
  }
}
