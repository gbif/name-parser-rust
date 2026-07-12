// SPDX-License-Identifier: Apache-2.0
package org.gbif.nameparser.rust;

import org.gbif.nameparser.api.NameParser;
import org.gbif.nameparser.api.NomCode;
import org.gbif.nameparser.api.ParsedName;
import org.gbif.nameparser.api.Rank;
import org.gbif.nameparser.api.UnparsableNameException;

import javax.annotation.Nullable;

/**
 * {@link NameParser} implementation backed by the Rust {@code nameparser} core, called in-process
 * via FFM (Panama) downcalls into the {@code nameparser-ffi} cdylib (see {@link Ffi}). Each call
 * marshals its inputs across the FFI boundary, receives the parse result as a flat fixed-layout
 * binary struct (see {@link StructCodec} and {@code crates/nameparser-ffi/src/layout.rs}), and
 * rebuilds a Java {@link ParsedName} from it through {@code ParsedName}'s real setters.
 *
 * <p>This binary struct is the single wire format the binding uses. It was the faster of the two
 * formats benchmarked during Phase 3 (~13% over a JSON/Gson path — see {@code ParseBench}), and
 * that JSON path was dropped at cdylib ABI version 2, which also shed the {@code gson} runtime
 * dependency; {@link StructCodec}'s class doc covers how the setter-based rebuild reproduces the
 * exact field values the Rust core computed.
 *
 * <p>Only the one non-default {@link NameParser} method is implemented; the two {@code @Deprecated}
 * {@code parse} overloads and {@code parseAuthorship} are inherited {@code default} methods that
 * delegate back into this one. The HEAD {@code NameParser} interface has no {@code close()} (it
 * does not extend {@code AutoCloseable}), so none is declared here either.
 */
public class NameParserRust implements NameParser {

  @Override
  public ParsedName parse(String scientificName, @Nullable String authorship, @Nullable Rank rank, @Nullable NomCode code)
      throws UnparsableNameException {
    return Ffi.callParseStruct(scientificName,
        authorship, rank == null ? null : rank.name(), code == null ? null : code.name());
  }
}
