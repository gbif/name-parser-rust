// SPDX-License-Identifier: Apache-2.0
package org.gbif.nameparser.rust;

import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SymbolLookup;
import java.lang.invoke.MethodHandle;
import java.nio.file.Path;

import static java.lang.foreign.ValueLayout.ADDRESS;
import static java.lang.foreign.ValueLayout.JAVA_INT;

/**
 * FFM (Panama, {@code java.lang.foreign}) plumbing for the {@code nameparser-ffi} Rust cdylib
 * (built by {@code cargo build -p nameparser-ffi --release} from {@code crates/nameparser-ffi}
 * in this repo). Isolated from {@link NameParserRust}'s parsing/model logic so that class only
 * ever deals with Java types (a {@code String} in, a {@code ParsedName} or exception out).
 *
 * <p>The C ABI bound here (see {@code crates/nameparser-ffi/src/lib.rs} for the authoritative
 * doc comments):
 * <pre>
 *   u32          np_abi_version()
 *   char *       np_parse_json(name, authorship, rank, code: const char *)
 *   void         np_free(char *)
 * </pre>
 * {@code name} is required; {@code authorship}/{@code rank}/{@code code} are nullable
 * ({@link MemorySegment#NULL} = absent). {@code rank}/{@code code} are the Java enum
 * {@code .name()} strings (e.g. {@code "SPECIES"}, {@code "BOTANICAL"}). The returned pointer
 * is a heap-allocated, NUL-terminated C string that MUST be handed back to {@code np_free}
 * exactly once; a {@code NULL} return means an internal Rust panic (never an unwind across
 * the ABI boundary -- see the Rust doc comment).
 */
final class Ffi {

  /**
   * The ABI version this Java binding was written against. Bumped in lockstep with
   * {@code nameparser-ffi}'s own {@code np_abi_version()} on any change to the extern "C"
   * surface itself.
   */
  private static final int EXPECTED_ABI_VERSION = 1;

  private static final Linker LINKER = Linker.nativeLinker();
  private static final MethodHandle ABI_VERSION;
  private static final MethodHandle PARSE_JSON;
  private static final MethodHandle FREE;

  static {
    SymbolLookup lib = SymbolLookup.libraryLookup(resolveLibPath(), Arena.global());
    ABI_VERSION = LINKER.downcallHandle(
        findOrThrow(lib, "np_abi_version"),
        FunctionDescriptor.of(JAVA_INT));
    PARSE_JSON = LINKER.downcallHandle(
        findOrThrow(lib, "np_parse_json"),
        FunctionDescriptor.of(ADDRESS, ADDRESS, ADDRESS, ADDRESS, ADDRESS));
    FREE = LINKER.downcallHandle(
        findOrThrow(lib, "np_free"),
        FunctionDescriptor.ofVoid(ADDRESS));

    int actual;
    try {
      actual = (int) ABI_VERSION.invokeExact();
    } catch (Throwable t) {
      throw new ExceptionInInitializerError(
          new IllegalStateException("nameparser-ffi: np_abi_version() downcall failed", t));
    }
    // Fail fast on desync: a stale cdylib built against a different extern "C" surface must
    // never be silently misinterpreted (e.g. a changed FunctionDescriptor reading garbage).
    if (actual != EXPECTED_ABI_VERSION) {
      throw new ExceptionInInitializerError(new IllegalStateException(
          "nameparser-ffi ABI version " + actual + " != " + EXPECTED_ABI_VERSION
              + " -- rebuild the cdylib (`cargo build -p nameparser-ffi --release`) "
              + "or update this binding to match its new C ABI"));
    }
  }

  private Ffi() {
  }

  private static MemorySegment findOrThrow(SymbolLookup lib, String symbol) {
    return lib.find(symbol)
        .orElseThrow(() -> new IllegalStateException(
            "nameparser-ffi cdylib is missing expected symbol '" + symbol + "' -- "
                + "wrong/stale build at " + resolveLibPath()));
  }

  /**
   * Resolves the cdylib path: {@code -Dnameparser.ffi.lib} system property, else the
   * {@code NAMEPARSER_FFI_LIB} environment variable, else a repo-relative default (this
   * module lives at {@code bindings/java/}, and the cdylib is built to the workspace root's
   * {@code target/release/}).
   */
  private static Path resolveLibPath() {
    String path = System.getProperty("nameparser.ffi.lib");
    if (path == null || path.isBlank()) {
      path = System.getenv("NAMEPARSER_FFI_LIB");
    }
    if (path == null || path.isBlank()) {
      path = "../../target/release/" + defaultLibFileName();
    }
    return Path.of(path);
  }

  /** Best-effort default cdylib file name for the current OS (macOS is the dev target). */
  private static String defaultLibFileName() {
    String os = System.getProperty("os.name", "").toLowerCase();
    if (os.contains("win")) {
      return "nameparser_ffi.dll";
    } else if (os.contains("linux")) {
      return "libnameparser_ffi.so";
    }
    return "libnameparser_ffi.dylib";
  }

  /**
   * Calls {@code np_parse_json} over the FFM boundary and copies its result into a Java
   * {@code String}. {@code name} must be non-null (see the C ABI doc above); {@code
   * authorship}/{@code rank}/{@code code} may each be null, which is marshalled as {@link
   * MemorySegment#NULL}.
   *
   * @return the JSON text (either a {@code ParsedName} object or an {@code {"error":...}}
   *     envelope -- {@link NameParserRust} distinguishes them), or {@code null} if the native
   *     call itself returned a null pointer (an internal Rust panic).
   */
  static String callParseJson(String name, String authorship, String rank, String code) {
    try (Arena arena = Arena.ofConfined()) {
      MemorySegment n = arena.allocateFrom(name);
      MemorySegment au = authorship == null ? MemorySegment.NULL : arena.allocateFrom(authorship);
      MemorySegment r = rank == null ? MemorySegment.NULL : arena.allocateFrom(rank);
      MemorySegment c = code == null ? MemorySegment.NULL : arena.allocateFrom(code);

      MemorySegment result = (MemorySegment) PARSE_JSON.invokeExact(n, au, r, c);
      if (result.address() == 0) {
        return null;
      }
      // The returned segment has zero declared length (it's just an address returned from a
      // downcall) -- reinterpret it as unbounded so getString can walk it to find the NUL
      // terminator, then copy the bytes into a Java String before freeing the native memory.
      try {
        return result.reinterpret(Long.MAX_VALUE).getString(0);
      } finally {
        FREE.invokeExact(result);
      }
    } catch (Throwable t) {
      throw new RuntimeException("nameparser-ffi FFM downcall failed", t);
    }
  }
}
