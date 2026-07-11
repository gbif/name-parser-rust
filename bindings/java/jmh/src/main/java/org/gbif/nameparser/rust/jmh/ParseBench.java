// SPDX-License-Identifier: Apache-2.0
package org.gbif.nameparser.rust.jmh;

import org.gbif.nameparser.NameParserImpl;
import org.gbif.nameparser.api.UnparsableNameException;
import org.gbif.nameparser.rust.NameParserRust;

import org.openjdk.jmh.annotations.Benchmark;
import org.openjdk.jmh.annotations.BenchmarkMode;
import org.openjdk.jmh.annotations.Fork;
import org.openjdk.jmh.annotations.Measurement;
import org.openjdk.jmh.annotations.Mode;
import org.openjdk.jmh.annotations.OutputTimeUnit;
import org.openjdk.jmh.annotations.Scope;
import org.openjdk.jmh.annotations.Setup;
import org.openjdk.jmh.annotations.State;
import org.openjdk.jmh.annotations.Warmup;
import org.openjdk.jmh.infra.Blackhole;

import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.TimeUnit;

/**
 * Single-name, in-process parse latency, three arms: {@link NameParserImpl} (the Java {@code
 * 4.2.0-SNAPSHOT} oracle, {@code javaImpl}) vs {@link NameParserRust} over its JSON wire format
 * ({@code rustJson}) vs {@link NameParserRust} over its flat fixed-layout struct wire format
 * ({@code rustStruct}, {@code org.gbif.nameparser.rust.StructCodec}). This is the JMH half of
 * the Phase-3 gate ("&ge;2&times; in JMH") -- see the Phase-3 plan's Tasks 4/6 and its design doc
 * &sect;5 model (in-process &asymp; 2-2.5&times;, capped by the Java-object-build floor) -- and
 * Task 6's A/B: whichever of {@code rustJson}/{@code rustStruct} wins (and by how much) is the
 * evidence used to decide which wire format ships. The numbers this benchmark produces are the
 * empirical answer, recorded verbatim in {@code results-jmh-ab.json} and the task report -- this
 * class does not assert a pass/fail threshold.
 *
 * <p>Each {@code @Benchmark} method loops the WHOLE sample array once per invocation (so "one
 * op" = one full pass over the sample, not one name); JMH's {@link Mode#AverageTime} then reports
 * average time per op. Divide by {@link #names}{@code .length} for the per-name figure -- the
 * ratios between arms are identical either way, since all three share the same sample and loop
 * shape. All three parsers are called the same way the parity test calls them:
 * {@code parse(name, null, null, null)} (name-only, no explicit authorship/rank/code).
 *
 * <p>Run (from the repo root, after {@code cargo build -p nameparser-ffi --release} and building
 * this module's shaded jar -- see {@code ../README.md}):
 * <pre>
 * java --enable-native-access=ALL-UNNAMED \
 *      -Dnameparser.ffi.lib=$PWD/target/release/libnameparser_ffi.dylib \
 *      -jar bindings/java/jmh/target/benchmarks.jar -rf json -rff bindings/java/jmh/results-jmh-ab.json
 * </pre>
 * The {@code --enable-native-access}/{@code -Dnameparser.ffi.lib} flags are given to the HOST
 * java process above, not to a {@code -jvmArgs} JMH option -- JMH's default fork behaviour (see
 * {@code @Fork} below) is to launch each forked measurement JVM with the same input arguments the
 * host JVM itself was launched with, so both flags reach the forked JVMs that actually run {@link
 * #rustJson}/{@link #rustStruct} without this class needing to hardcode a path into a {@code
 * @Fork} annotation (annotation values must be compile-time constants, and the absolute cdylib
 * path is not one).
 */
@State(Scope.Benchmark)
@BenchmarkMode(Mode.AverageTime)
@OutputTimeUnit(TimeUnit.MICROSECONDS)
@Warmup(iterations = 5, time = 1)
@Measurement(iterations = 5, time = 1)
@Fork(2)
public class ParseBench {

  /** Stable slice size -- "the first ~2,000 names of testdata/benchmark-data.txt" per the brief. */
  static final int SAMPLE_SIZE = 2000;

  private static final String CORPUS_FILE_NAME = "benchmark-data.txt";

  /** Optional override: {@code -Dnameparser.testdata.dir=<repo>/testdata}, the same property
   *  name/convention {@code bindings/java}'s {@code ParityTest} already uses. Not wired in by
   *  the documented run command above (no Maven surefire argLine here to inject it), but honored
   *  first if a caller does pass it, before falling back to the relative-path guesses below. */
  private static final String TESTDATA_DIR_PROPERTY = "nameparser.testdata.dir";

  /** Candidate directories to find {@code testdata/} under, tried in order, relative to whatever
   *  the JVM's working directory happens to be. The documented run command above is launched
   *  from the repo root (`cd` there first, per the task instructions), so plain {@code "testdata"}
   *  is the expected hit -- the others are defensive fallbacks for e.g. an IDE run configuration
   *  whose working directory defaults to this module's own {@code basedir}. */
  private static final String[] RELATIVE_TESTDATA_DIRS = {
      "testdata",           // cwd = repo root (the documented `java -jar .../benchmarks.jar` invocation)
      "../../../testdata",  // cwd = bindings/java/jmh (this module's own basedir)
      "../../testdata",     // cwd = bindings/java
      "../testdata",        // cwd = bindings
  };

  String[] names;
  NameParserImpl javaParser;
  NameParserRust rustJsonParser;
  NameParserRust rustStructParser;

  @Setup
  public void setup() throws IOException {
    names = loadSample(resolveCorpusFile());
    javaParser = new NameParserImpl();
    rustJsonParser = new NameParserRust();
    rustStructParser = new NameParserRust(NameParserRust.WireFormat.STRUCT);
  }

  @Benchmark
  public void javaImpl(Blackhole bh) {
    for (String name : names) {
      try {
        bh.consume(javaParser.parse(name, null, null, null));
      } catch (UnparsableNameException e) {
        bh.consume(e);
      }
    }
  }

  @Benchmark
  public void rustJson(Blackhole bh) {
    for (String name : names) {
      try {
        bh.consume(rustJsonParser.parse(name, null, null, null));
      } catch (UnparsableNameException e) {
        bh.consume(e);
      }
    }
  }

  /** Same shape as {@link #rustJson}, using the flat fixed-layout struct wire format ({@code
   *  org.gbif.nameparser.rust.StructCodec}) instead of JSON -- the Task 6 A/B arm this
   *  benchmark exists to add. */
  @Benchmark
  public void rustStruct(Blackhole bh) {
    for (String name : names) {
      try {
        bh.consume(rustStructParser.parse(name, null, null, null));
      } catch (UnparsableNameException e) {
        bh.consume(e);
      }
    }
  }

  // ---------------------------------------------------------------------------------------
  // Sample loading -- name = text before the first TAB, blank/#-comment lines skipped, exactly
  // the convention `extract_name` (crates/nameparser-cli/src/main.rs) and ParityTest.extractName
  // use, so this benchmark's sample is drawn from the identical name set those already validate.
  // benchmark-data.txt today has no TABs and one leading `#` comment line, so in practice this
  // reads lines 2..2001, but the general rule is kept for robustness against future edits.
  // ---------------------------------------------------------------------------------------

  private static String[] loadSample(File corpus) throws IOException {
    List<String> sample = new ArrayList<>(SAMPLE_SIZE);
    for (String raw : Files.readAllLines(corpus.toPath(), StandardCharsets.UTF_8)) {
      if (sample.size() >= SAMPLE_SIZE) {
        break;
      }
      String name = extractName(raw);
      if (name != null) {
        sample.add(name);
      }
    }
    if (sample.isEmpty()) {
      throw new IllegalStateException("loaded 0 names from " + corpus.getAbsolutePath());
    }
    return sample.toArray(new String[0]);
  }

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

  private static File resolveCorpusFile() {
    String override = System.getProperty(TESTDATA_DIR_PROPERTY);
    if (override != null && !override.isBlank()) {
      File f = new File(override, CORPUS_FILE_NAME);
      if (f.isFile()) {
        return f;
      }
      throw new IllegalStateException("-D" + TESTDATA_DIR_PROPERTY + "=" + override
          + " does not contain " + CORPUS_FILE_NAME + " (looked at " + f.getAbsolutePath() + ")");
    }
    for (String dir : RELATIVE_TESTDATA_DIRS) {
      File f = new File(dir, CORPUS_FILE_NAME);
      if (f.isFile()) {
        return f;
      }
    }
    throw new IllegalStateException(CORPUS_FILE_NAME + " not found via any of "
        + Arrays.toString(RELATIVE_TESTDATA_DIRS) + " (cwd=" + new File("").getAbsolutePath()
        + ") -- pass -D" + TESTDATA_DIR_PROPERTY + "=<repo>/testdata explicitly");
  }
}
