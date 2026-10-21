// SPDX-License-Identifier: Apache-2.0
package org.gbif.nameparser.rust;

import org.gbif.nameparser.util.UnicodeUtils;
import org.junit.jupiter.api.Test;

import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * The Rust engine's homoglyph table ({@code crates/nameparser/resources/homoglyphs.txt}) is a copy
 * of name-parser-api's {@code unicode/homoglyphs.txt}, which ChecklistBank uses to flag verbatim
 * terms ({@link UnicodeUtils#containsHomoglyphs}). The copies drifted once (#205): the Rust one was
 * fixed, the Java one kept pairing {@code e} with {@code ě}, and a WFO import got 2,834 records
 * flagged. A fix to one copy must go into the other too; this test fails until it does.
 */
class HomoglyphTableTest {

  private static final File RUST_TABLE =
      new File(System.getProperty("nameparser.resources.dir", "../../crates/nameparser/resources"),
          "homoglyphs.txt");

  @Test
  void homoglyphTableMatchesNameParserApi() throws IOException {
    String api;
    try (InputStream in = UnicodeUtils.class.getResourceAsStream("/unicode/homoglyphs.txt")) {
      assertNotNull(in, "name-parser-api has no /unicode/homoglyphs.txt resource");
      api = new String(in.readAllBytes(), StandardCharsets.UTF_8);
    }
    assertTrue(RUST_TABLE.isFile(), "homoglyph table not found: " + RUST_TABLE.getAbsolutePath()
        + " -- set -Dnameparser.resources.dir=<repo>/crates/nameparser/resources");
    String rust = Files.readString(RUST_TABLE.toPath(), StandardCharsets.UTF_8);

    assertEquals(api, rust, "crates/nameparser/resources/homoglyphs.txt differs from"
        + " name-parser-api's unicode/homoglyphs.txt -- change both copies");
  }
}
