// SPDX-License-Identifier: Apache-2.0
package org.gbif.nameparser.rust;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonNull;
import com.google.gson.JsonObject;

import org.gbif.nameparser.NameParserImpl;
import org.gbif.nameparser.api.NameParser;
import org.gbif.nameparser.api.ParsedName;
import org.gbif.nameparser.api.UnparsableNameException;
import org.junit.jupiter.api.Test;

import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Locale;
import java.util.Set;
import java.util.TreeSet;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * Parity gate: {@link NameParserRust} vs {@link NameParserImpl} (the Java oracle pipeline) over
 * every name corpus in {@code testdata/}.
 *
 * <p>Phase 2 already proved 11,302/11,302 zero-diff parity between the Rust core and the Java
 * oracle out-of-process, by dumping both to JSONL and diffing with the native CLI's {@code
 * nameparser-cli compare} subcommand (see {@code cross-validation.md} and {@code
 * crates/nameparser-cli/src/main.rs}). This test re-proves the identical claim *in-process*,
 * through the FFM boundary and the binding's single flat-struct wire format ({@link StructCodec}
 * decode). A mismatch caught here is therefore an FFM-marshalling or flat-codec decode bug in the
 * binding, not a core-parser regression; the core is already gated by {@code
 * crates/nameparser/tests/parse_golden.rs}.
 *
 * <p>Both parsers are called the same way the corpora were cross-validated in Phase 2:
 * {@code parse(name, null, null, null)} — name-only, no explicit authorship/rank/code.
 *
 * <p><b>Comparison rule</b> — the exact rule the Phase-2 CLI's {@code compare} subcommand
 * uses: for a name both sides parse, serialize both {@link ParsedName}s through one {@link Gson}
 * instance (this test's own {@link #GSON} — the binding itself no longer ships Gson) and compare
 * the resulting {@code JsonObject}s field by field, where {@code warnings}/{@code notho}/{@code
 * epithetQualifier} compare order-insensitively (as sets/maps — {@link #UNORDERED_FIELD_KEYS},
 * mirrored verbatim from the Rust CLI's constant of the same name) and every other field compares
 * exactly. For a name both sides fail on, "equal" means the same {@code NameType} and {@code
 * NomCode}. One side parsing while the other throws is always a mismatch.
 */
class ParityTest {

  /**
   * This repo's name corpora (see {@code cross-validation.md}): the 8017-name benchmark corpus
   * this crate already golden-tests in-harness, plus the 6 Java {@code name-parser} test-resource
   * corpora ({@code NameParserImplTest}'s own fixtures) copied verbatim into {@code testdata/}.
   * Together these are the exact 11,302 names Phase 2 cross-validated out-of-process.
   */
  private static final List<String> CORPORA = List.of(
      "benchmark-data.txt",
      "names-with-authors.txt",
      "hybrids.txt",
      "other.txt",
      "otu.txt",
      "placeholder.txt",
      "viruses.txt");

  /**
   * Field names — the exact JSON wire spelling {@code ParsedName} serializes to — whose value is
   * backed by a Java {@code Set}/{@code Map}-shaped collection ({@code warnings}: {@code
   * HashSet<String>}; {@code notho}: {@code EnumSet<NamePart>}; {@code epithetQualifier}: {@code
   * EnumMap<NamePart,String>}) and are therefore compared order-insensitively below. Mirrored
   * verbatim from {@code UNORDERED_FIELD_KEYS} in {@code crates/nameparser-cli/src/main.rs} (see
   * that constant's doc comment for the full rationale, including why only {@code warnings} has
   * actually been observed to disagree in order in practice).
   */
  private static final Set<String> UNORDERED_FIELD_KEYS = Set.of("warnings", "notho", "epithetQualifier");

  /** Caps the diff dump on assertion failure; the aggregate compared/diff counts are never capped. */
  private static final int MAX_EXAMPLES = 20;

  /** This test's own Gson, used solely to structurally compare two {@link ParsedName}s (see the
   *  class doc's comparison rule). The binding itself no longer ships Gson; it is a test-scope
   *  dependency here. */
  private static final Gson GSON = new GsonBuilder().create();

  private final NameParserImpl oracle = new NameParserImpl();

  @Test
  void zeroDiffsAcrossAllCorpora() throws IOException {
    NameParserRust rust = new NameParserRust();
    File testdataDir = testdataDir();
    assertTrue(testdataDir.isDirectory(),
        "testdata dir not found at " + testdataDir.getAbsolutePath()
            + " -- set -Dnameparser.testdata.dir=<repo>/testdata");

    StringBuilder tally = new StringBuilder();
    List<String> examples = new ArrayList<>();
    int totalCompared = 0;
    int totalDiffs = 0;

    for (String corpus : CORPORA) {
      File file = new File(testdataDir, corpus);
      assertTrue(file.isFile(), "corpus file not found: " + file.getAbsolutePath());

      int compared = 0;
      int diffs = 0;
      for (String name : readNames(file)) {
        compared++;
        Outcome rustOutcome = parse(rust, name);
        Outcome oracleOutcome = parse(oracle, name);
        if (!outcomesMatch(rustOutcome, oracleOutcome)) {
          diffs++;
          if (examples.size() < MAX_EXAMPLES) {
            examples.add(describeDiff(corpus, name, rustOutcome, oracleOutcome));
          }
        }
      }
      tally.append(String.format(Locale.ROOT, "  %-24s %6d compared, %4d diffs%n", corpus, compared, diffs));
      totalCompared += compared;
      totalDiffs += diffs;
    }
    tally.append(String.format(Locale.ROOT, "  %-24s %6d compared, %4d diffs%n", "TOTAL", totalCompared, totalDiffs));

    System.out.println("ParityTest: NameParserRust vs NameParserImpl, per-corpus tally:");
    System.out.print(tally);

    if (totalDiffs > 0) {
      System.err.println("ParityTest: " + totalDiffs + " diff(s) out of " + totalCompared + " -- first "
          + examples.size() + " example(s) follow:");
      for (String example : examples) {
        System.err.println("----");
        System.err.println(example);
      }
    }

    assertEquals(0, totalDiffs, totalDiffs + " of " + totalCompared
        + " names differ between NameParserRust and NameParserImpl -- see stdout/stderr above"
        + " for the per-corpus tally and up to " + MAX_EXAMPLES + " example diff(s)");
  }

  // ---------------------------------------------------------------------------------------
  // Corpus discovery: name = text before the first TAB, blank/#-comment lines skipped -- the
  // exact convention `extract_name` (crates/nameparser-cli/src/main.rs) uses, so this test
  // walks the identical name set Phase 2 already cross-validated (see cross-validation.md).
  // ---------------------------------------------------------------------------------------

  private static File testdataDir() {
    return new File(System.getProperty("nameparser.testdata.dir", "../../testdata"));
  }

  private static List<String> readNames(File file) throws IOException {
    List<String> names = new ArrayList<>();
    for (String raw : Files.readAllLines(file.toPath(), StandardCharsets.UTF_8)) {
      String name = extractName(raw);
      if (name != null) {
        names.add(name);
      }
    }
    return names;
  }

  /**
   * Mirrors {@code extract_name} in {@code crates/nameparser-cli/src/main.rs} exactly, down to
   * its two edge cases: a line is a comment only when the very first (untrimmed) character is
   * {@code '#'}, and a lone {@code "scientificName"} header cell (a TSV header row, were one
   * ever present) is skipped like a blank line.
   */
  static String extractName(String raw) {
    if (raw.isEmpty() || raw.startsWith("#")) {
      return null;
    }
    int tab = raw.indexOf('\t');
    String name = (tab < 0 ? raw : raw.substring(0, tab)).trim();
    if (name.isEmpty() || name.equals("scientificName")) {
      return null;
    }
    return name;
  }

  // ---------------------------------------------------------------------------------------
  // Parsing both sides, uniformly
  // ---------------------------------------------------------------------------------------

  /**
   * One side's outcome for a single name: either a successfully-built {@link ParsedName} or a
   * thrown {@link UnparsableNameException} -- never both, never neither.
   */
  private static final class Outcome {
    final ParsedName parsed;
    final UnparsableNameException error;

    private Outcome(ParsedName parsed, UnparsableNameException error) {
      this.parsed = parsed;
      this.error = error;
    }

    static Outcome parsed(ParsedName pn) {
      return new Outcome(pn, null);
    }

    static Outcome unparsable(UnparsableNameException e) {
      return new Outcome(null, e);
    }
  }

  private static Outcome parse(NameParser parser, String name) {
    try {
      return Outcome.parsed(parser.parse(name, null, null, null));
    } catch (UnparsableNameException e) {
      return Outcome.unparsable(e);
    }
  }

  // ---------------------------------------------------------------------------------------
  // Comparison -- see the class doc for the rule; this section is the mechanical
  // implementation of it.
  // ---------------------------------------------------------------------------------------

  private static boolean outcomesMatch(Outcome a, Outcome b) {
    if (a.parsed != null && b.parsed != null) {
      JsonObject aJson = GSON.toJsonTree(a.parsed).getAsJsonObject();
      JsonObject bJson = GSON.toJsonTree(b.parsed).getAsJsonObject();
      return jsonEquals(aJson, bJson);
    }
    if (a.error != null && b.error != null) {
      return a.error.getType() == b.error.getType() && a.error.getCode() == b.error.getCode();
    }
    return false; // one side parsed, the other threw
  }

  /**
   * Structural equality between two {@link JsonElement}s, with the order-insensitive carve-out
   * for {@link #UNORDERED_FIELD_KEYS} applied at every nesting depth (mirroring the Rust
   * comparator's {@code canonicalize_for_key}, which runs on every recursive call, not just the
   * top level -- though in {@code ParsedName}'s actual shape all three keys only ever occur at
   * the top level). A JSON *object* value under one of those keys (namely {@code
   * epithetQualifier}) needs no special-casing beyond the object branch below: comparing by the
   * union of keys is already order-insensitive. A key missing on one side (Gson omits
   * null/absent fields by default) is defaulted to {@link JsonNull#INSTANCE} before comparing,
   * so "absent on one side, present on the other" is correctly a mismatch while "absent on both"
   * is correctly equal.
   */
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

  // ---------------------------------------------------------------------------------------
  // Diff reporting
  // ---------------------------------------------------------------------------------------

  private static String describeDiff(String corpus, String name, Outcome rustOutcome, Outcome oracleOutcome) {
    return "corpus=" + corpus + " name=\"" + name + "\"\n"
        + "  rust:   " + describe(rustOutcome) + "\n"
        + "  oracle: " + describe(oracleOutcome);
  }

  private static String describe(Outcome o) {
    if (o.parsed != null) {
      return GSON.toJson(o.parsed);
    }
    return "UnparsableNameException{type=" + o.error.getType() + ", code=" + o.error.getCode()
        + ", name=" + o.error.getName() + "}";
  }
}
