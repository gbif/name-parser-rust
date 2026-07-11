// SPDX-License-Identifier: Apache-2.0
package org.gbif.nameparser.rust;

import org.gbif.nameparser.api.ParsedName;
import org.gbif.nameparser.api.UnparsableNameException;

import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SymbolLookup;
import java.lang.invoke.MethodHandle;
import java.nio.file.Path;

import static java.lang.foreign.ValueLayout.ADDRESS;
import static java.lang.foreign.ValueLayout.JAVA_INT;
import static java.lang.foreign.ValueLayout.JAVA_LONG;

/**
 * FFM (Panama, {@code java.lang.foreign}) plumbing for the {@code nameparser-ffi} Rust cdylib
 * (built by {@code cargo build -p nameparser-ffi --release} from {@code crates/nameparser-ffi}
 * in this repo). Isolated from {@link NameParserRust}'s parsing/model logic so that class only
 * ever deals with Java types (a {@code String} in, a {@code ParsedName} or exception out) --
 * {@link #callParseJson} hands back raw JSON text ({@link NameParserRust} does the Gson rebuild
 * and the {@code {"error":...}} envelope handling); {@link #callParseStruct} hands back an
 * already-decoded {@link ParsedName} (or throws {@link UnparsableNameException}), delegating the
 * actual byte-level decode work to {@link StructCodec} so this class stays limited to the
 * arena/downcall/retry mechanics.
 *
 * <p>The C ABI bound here (see {@code crates/nameparser-ffi/src/lib.rs} for the authoritative
 * doc comments):
 * <pre>
 *   u32          np_abi_version()
 *   char *       np_parse_json(name, authorship, rank, code: const char *)
 *   i64          np_parse_struct(name, authorship, rank, code: const char *, out: u8 *, out_cap: usize)
 *   void         np_free(char *)
 * </pre>
 * {@code name} is required; {@code authorship}/{@code rank}/{@code code} are nullable
 * ({@link MemorySegment#NULL} = absent). {@code rank}/{@code code} are the Java enum
 * {@code .name()} strings (e.g. {@code "SPECIES"}, {@code "BOTANICAL"}). {@code np_parse_json}'s
 * returned pointer is a heap-allocated, NUL-terminated C string that MUST be handed back to
 * {@code np_free} exactly once; a {@code NULL} return means an internal Rust panic (never an
 * unwind across the ABI boundary -- see the Rust doc comment). {@code np_parse_struct} instead
 * writes into a caller-owned buffer and returns a byte count/status code -- see {@link
 * #callParseStruct} for its retry-on-overflow protocol.
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
  private static final MethodHandle PARSE_STRUCT;
  private static final MethodHandle FREE;

  static {
    SymbolLookup lib = SymbolLookup.libraryLookup(resolveLibPath(), Arena.global());
    ABI_VERSION = LINKER.downcallHandle(
        findOrThrow(lib, "np_abi_version"),
        FunctionDescriptor.of(JAVA_INT));
    PARSE_JSON = LINKER.downcallHandle(
        findOrThrow(lib, "np_parse_json"),
        FunctionDescriptor.of(ADDRESS, ADDRESS, ADDRESS, ADDRESS, ADDRESS));
    PARSE_STRUCT = LINKER.downcallHandle(
        findOrThrow(lib, "np_parse_struct"),
        FunctionDescriptor.of(JAVA_LONG, ADDRESS, ADDRESS, ADDRESS, ADDRESS, ADDRESS, JAVA_LONG));
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

  /** Initial scratch buffer size for {@link #callParseStruct}: comfortably above the format's
   *  own minimum successful-encode size (208 bytes, an empty {@code ParsedName}) and every
   *  ordinary name/authorship, so the overflow-retry path below is only actually exercised for
   *  unusually long inputs. Verified end-to-end (retry logic exercised on literally every parse
   *  in the 11,302-name corpus, still 0 diffs) with this constant temporarily set as low as 64.
   *
   *  <p><b>Must stay &ge; {@link StructCodec#HEADER_SIZE} (36).</b> The unparsable path (native
   *  return {@code -1}) writes only a header, clamped to {@code min(HEADER_SIZE, out_cap)} (see
   *  {@code layout.rs}'s "Unparsable path" doc) and is never overflow-coded/retried -- so a
   *  smaller value here would truncate that header and make {@link StructCodec#unparsableException}
   *  read past the buffer for every unparsable name. This invariant is enforced at class-init time
   *  by the {@code static} block below (not merely documented). */
  private static final long INITIAL_STRUCT_BUFFER_BYTES = 4096;

  static {
    // Real runtime guard for the invariant documented on INITIAL_STRUCT_BUFFER_BYTES above.
    // StructCodec.HEADER_SIZE is a JLS compile-time constant (a `static final int` with a
    // constant initializer), so referencing it here inlines the literal 36 and does NOT trigger
    // StructCodec's own <clinit> (its enum-ordinal guard) -- this check is free of any
    // Ffi<->StructCodec class-initialization ordering dependency.
    if (INITIAL_STRUCT_BUFFER_BYTES < StructCodec.HEADER_SIZE) {
      throw new ExceptionInInitializerError(new IllegalStateException(
          "INITIAL_STRUCT_BUFFER_BYTES (" + INITIAL_STRUCT_BUFFER_BYTES
              + ") must be >= StructCodec.HEADER_SIZE (" + StructCodec.HEADER_SIZE
              + "): the unparsable path writes a header-only buffer that would otherwise be truncated"));
    }
  }

  /**
   * Raw {@code np_abi_version()} downcall, with no comparison/throwing beyond the downcall
   * failing outright -- used by {@link StructCodec}'s own startup guard. Referencing this method
   * (a real invocation, not a compile-time-constant field read) forces this class's static
   * initializer -- which independently verifies the same version against {@link
   * #EXPECTED_ABI_VERSION} and refuses to load the class at all on mismatch -- to run to
   * completion first, so by the time this method's body executes the version is already known
   * good; {@link StructCodec} re-checks it anyway as an explicit, self-documented part of its
   * own guard rather than silently relying on that ordering.
   */
  static int nativeAbiVersion() {
    try {
      return (int) ABI_VERSION.invokeExact();
    } catch (Throwable t) {
      throw new IllegalStateException("nameparser-ffi: np_abi_version() downcall failed", t);
    }
  }

  /**
   * Calls {@code np_parse_struct} over the FFM boundary and decodes its flat binary result into
   * a {@link ParsedName} via {@link StructCodec#decode}. Null handling for {@code
   * authorship}/{@code rank}/{@code code} mirrors {@link #callParseJson}'s.
   *
   * <p>Implements the overflow-retry protocol {@code np_parse_struct} documents (see {@code
   * layout.rs}): a first attempt against a {@link #INITIAL_STRUCT_BUFFER_BYTES}-byte scratch
   * buffer, and -- only if that overflows -- exactly one retry against a buffer sized to the
   * reported {@code needed} count. Both calls happen inside the same confined {@link Arena}, and
   * {@link StructCodec#decode} runs before that arena closes (it copies every string it needs
   * into plain Java {@code String}s, so the returned {@link ParsedName} holds no live reference
   * into native memory once this method returns).
   *
   * @throws UnparsableNameException translated from the native {@code -1} return (the header
   *     carries the {@code NameType}/{@code NomCode} to attach to it; see {@link
   *     StructCodec#unparsableException}).
   */
  static ParsedName callParseStruct(String name, String authorship, String rank, String code)
      throws UnparsableNameException {
    try (Arena arena = Arena.ofConfined()) {
      MemorySegment n = arena.allocateFrom(name);
      MemorySegment au = authorship == null ? MemorySegment.NULL : arena.allocateFrom(authorship);
      MemorySegment r = rank == null ? MemorySegment.NULL : arena.allocateFrom(rank);
      MemorySegment c = code == null ? MemorySegment.NULL : arena.allocateFrom(code);

      long cap = INITIAL_STRUCT_BUFFER_BYTES;
      MemorySegment out = arena.allocate(cap);
      long ret = invokeParseStruct(n, au, r, c, out, cap);

      if (ret < -2) {
        // Overflow: ret == -(needed + 3). Retry exactly once with a buffer sized to fit exactly
        // -- np_parse_struct is a pure function of its (name, authorship, rank, code) inputs, so
        // a second call with the identical inputs and a big-enough buffer cannot rationally
        // overflow again or produce a different status. `Math.max(HEADER_SIZE, needed)` is
        // belt-and-suspenders: a real overflow `needed` is the full success-encode size (>= 208),
        // never below HEADER_SIZE, but this guarantees the retry buffer can always hold the
        // header-only write the unparsable path would make even if `needed` were somehow tiny.
        long needed = -ret - 3;
        cap = Math.max(StructCodec.HEADER_SIZE, needed);
        out = arena.allocate(cap);
        ret = invokeParseStruct(n, au, r, c, out, cap);
        if (ret < 0) {
          throw new IllegalStateException("nameparser-ffi: np_parse_struct still failed (ret=" + ret
              + ") after retrying with the exact reported size (" + needed + " bytes) for name '" + name + "'");
        }
      }

      if (ret == -2) {
        throw new RuntimeException(
            "nameparser-ffi: np_parse_struct reported an internal error (caught panic) for name '" + name + "'");
      }
      if (ret == -1) {
        throw StructCodec.unparsableException(out, name);
      }
      return StructCodec.decode(out, (int) ret);
    }
  }

  private static long invokeParseStruct(MemorySegment n, MemorySegment au, MemorySegment r, MemorySegment c,
                                         MemorySegment out, long cap) {
    try {
      return (long) PARSE_STRUCT.invokeExact(n, au, r, c, out, cap);
    } catch (Throwable t) {
      throw new RuntimeException("nameparser-ffi FFM downcall failed", t);
    }
  }
}
