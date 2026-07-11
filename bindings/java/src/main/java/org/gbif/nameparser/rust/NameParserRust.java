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

import java.util.Objects;

import javax.annotation.Nullable;

/**
 * {@link NameParser} implementation backed by the Rust {@code nameparser} core, called
 * in-process via FFM (Panama) downcalls into the {@code nameparser-ffi} cdylib (see
 * {@link Ffi}). Carries TWO wire formats, selected by {@link WireFormat} at construction time:
 *
 * <ul>
 *   <li>{@link WireFormat#JSON} (the default, {@link #NameParserRust()}) -- each call marshals a
 *       JSON string across the FFI boundary and rebuilds a Java {@link ParsedName} from it with
 *       Gson;
 *   <li>{@link WireFormat#STRUCT} -- each call marshals a flat fixed-layout binary struct
 *       instead (see {@link StructCodec}), avoiding JSON's text-marshalling cost.
 * </ul>
 *
 * <p>Both paths read every field off the exact same Rust-side {@code ParsedName}, so they are
 * expected to always agree; {@code ParityTest} is parametrized over both formats specifically to
 * prove that (0 diffs against the Java oracle for each), and {@code ParseBench}'s {@code
 * rustJson}/{@code rustStruct} benchmark arms measure which is faster in practice.
 *
 * <p><b>gson round-trip, not setters (JSON path only).</b> {@code GSON.fromJson(json,
 * ParsedName.class)} populates {@code ParsedName}'s fields by reflection, bypassing every
 * side-effecting setter (e.g. {@code setGenus} stripping a leading {@code ×} and calling {@code
 * addNotho}, or {@code setPublishedIn} auto-deriving {@code publishedInYear}). That is
 * intentional and correct here: the Rust JSON already carries final field values computed by the
 * same logic the setters exist to reproduce, so re-running the setters' side effects on top
 * would be redundant at best and wrong at worst (e.g. double-stripping a marker that Rust
 * already stripped). The STRUCT path, by contrast, has no reflective rebuild available and must
 * use the real setters -- see {@link StructCodec}'s class doc for how it navigates their side
 * effects.
 *
 * <p>Only the one non-default {@link NameParser} method is implemented; the two
 * {@code @Deprecated} {@code parse} overloads and {@code parseAuthorship} are inherited {@code
 * default} methods that delegate back into this one. The HEAD (4.2.0-SNAPSHOT) {@code
 * NameParser} interface has no {@code close()} (it does not extend {@code AutoCloseable}), so
 * none is declared here either.
 */
public class NameParserRust implements NameParser {

  /** Selects which wire format {@link #parse} marshals a parse result across the FFM boundary
   *  with -- see the class doc for what each entails. */
  public enum WireFormat {
    JSON,
    STRUCT
  }

  /**
   * Shared, stateless, thread-safe (Gson instances are safe for concurrent use once built).
   * Package-private so {@code NameParserRustSmokeTest} can re-serialize a {@code ParsedName}
   * through the exact same instance for its round-trip assertions.
   */
  static final Gson GSON = new GsonBuilder().create();

  private final WireFormat wireFormat;

  /** Equivalent to {@code new NameParserRust(WireFormat.JSON)}. */
  public NameParserRust() {
    this(WireFormat.JSON);
  }

  public NameParserRust(WireFormat wireFormat) {
    this.wireFormat = Objects.requireNonNull(wireFormat, "wireFormat");
  }

  @Override
  public ParsedName parse(String scientificName, @Nullable String authorship, @Nullable Rank rank, @Nullable NomCode code)
      throws UnparsableNameException {
    return switch (wireFormat) {
      case JSON -> parseJson(scientificName, authorship, rank, code);
      case STRUCT -> Ffi.callParseStruct(scientificName,
          authorship, rank == null ? null : rank.name(), code == null ? null : code.name());
    };
  }

  private ParsedName parseJson(String scientificName, @Nullable String authorship, @Nullable Rank rank, @Nullable NomCode code)
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
