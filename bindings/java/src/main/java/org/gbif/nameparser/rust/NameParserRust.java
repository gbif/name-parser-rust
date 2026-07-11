// SPDX-License-Identifier: Apache-2.0
package org.gbif.nameparser.rust;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

import org.gbif.nameparser.api.NameParser;
import org.gbif.nameparser.api.NameType;
import org.gbif.nameparser.api.NomCode;
import org.gbif.nameparser.api.ParsedName;
import org.gbif.nameparser.api.Rank;
import org.gbif.nameparser.api.UnparsableNameException;

import javax.annotation.Nullable;

/**
 * {@link NameParser} implementation backed by the Rust {@code nameparser} core, called
 * in-process via FFM (Panama) downcalls into the {@code nameparser-ffi} cdylib (see
 * {@link Ffi}). This is the JSON wire-format path: each call marshals a JSON string across the
 * FFI boundary and rebuilds a Java {@link ParsedName} from it with Gson.
 *
 * <p><b>gson round-trip, not setters.</b> {@code GSON.fromJson(json, ParsedName.class)}
 * populates {@code ParsedName}'s fields by reflection, bypassing every side-effecting setter
 * (e.g. {@code setGenus} stripping a leading {@code ×} and calling {@code addNotho}, or {@code
 * setPublishedIn} auto-deriving {@code publishedInYear}). That is intentional and correct here:
 * the Rust JSON already carries final field values computed by the same logic the setters exist
 * to reproduce, so re-running the setters' side effects on top would be redundant at best and
 * wrong at worst (e.g. double-stripping a marker that Rust already stripped).
 *
 * <p>Only the one non-default {@link NameParser} method is implemented; the two
 * {@code @Deprecated} {@code parse} overloads and {@code parseAuthorship} are inherited {@code
 * default} methods that delegate back into this one. The HEAD (4.2.0-SNAPSHOT) {@code
 * NameParser} interface has no {@code close()} (it does not extend {@code AutoCloseable}), so
 * none is declared here either.
 */
public class NameParserRust implements NameParser {

  /**
   * Shared, stateless, thread-safe (Gson instances are safe for concurrent use once built).
   * Package-private so {@code NameParserRustSmokeTest} can re-serialize a {@code ParsedName}
   * through the exact same instance for its round-trip assertions.
   */
  static final Gson GSON = new GsonBuilder().create();

  @Override
  public ParsedName parse(String scientificName, @Nullable String authorship, @Nullable Rank rank, @Nullable NomCode code)
      throws UnparsableNameException {
    String json = Ffi.callParseJson(scientificName,
        authorship, rank == null ? null : rank.name(), code == null ? null : code.name());
    if (json == null) {
      throw new UnparsableNameException(NameType.OTHER, scientificName, "native parse returned null");
    }
    JsonObject o = JsonParser.parseString(json).getAsJsonObject();
    if (o.has("error")) {
      JsonObject e = o.getAsJsonObject("error");
      NameType t = NameType.valueOf(e.get("type").getAsString());
      NomCode c = e.has("code") && !e.get("code").isJsonNull() ? NomCode.valueOf(e.get("code").getAsString()) : null;
      throw new UnparsableNameException(t, c, scientificName);
    }
    return GSON.fromJson(o, ParsedName.class);
  }
}
