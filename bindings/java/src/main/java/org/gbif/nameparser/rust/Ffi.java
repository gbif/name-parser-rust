// SPDX-License-Identifier: Apache-2.0
package org.gbif.nameparser.rust;

import org.gbif.nameparser.api.ParseResult;

import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SymbolLookup;
import java.lang.invoke.MethodHandle;
import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;

import static java.lang.foreign.ValueLayout.ADDRESS;
import static java.lang.foreign.ValueLayout.JAVA_INT;
import static java.lang.foreign.ValueLayout.JAVA_LONG;

/**
 * FFM (Panama, {@code java.lang.foreign}) plumbing for the {@code nameparser-ffi} Rust cdylib
 * (built by {@code cargo build -p nameparser-ffi --release} from {@code crates/nameparser-ffi}
 * in this repo). Isolated from {@link NameParserRust}'s parsing/model logic so that class only
 * ever deals with Java types (a {@code String} in, a {@code ParsedName} or exception out):
 * {@link #callParseStruct} hands back an already-decoded {@link ParsedName} (or throws {@link
 * UnparsableNameException}), delegating the actual byte-level decode work to {@link StructCodec}
 * so this class stays limited to the arena/downcall/retry mechanics.
 *
 * <p>The C ABI bound here (see {@code crates/nameparser-ffi/src/lib.rs} for the authoritative
 * doc comments):
 * <pre>
 *   u32          np_abi_version()
 *   i64          np_parse_struct(name, authorship, rank, code: const char *, out: u8 *, out_cap: usize)
 * </pre>
 * {@code name} is required; {@code authorship}/{@code rank}/{@code code} are nullable
 * ({@link MemorySegment#NULL} = absent). {@code rank}/{@code code} are the Java enum
 * {@code .name()} strings (e.g. {@code "SPECIES"}, {@code "BOTANICAL"}). {@code np_parse_struct}
 * writes into a caller-owned buffer and returns a byte count/status code -- see {@link
 * #callParseStruct} for its retry-on-overflow protocol. (The former JSON wire path,
 * {@code np_parse_json}/{@code np_free}, was removed at ABI version 2.)
 */
final class Ffi {

  /**
   * The ABI version this Java binding was written against. Bumped in lockstep with
   * {@code nameparser-ffi}'s own {@code np_abi_version()} on any change to the extern "C"
   * surface itself.
   */
  private static final int EXPECTED_ABI_VERSION = 4;

  private static final Linker LINKER = Linker.nativeLinker();
  private static final MethodHandle ABI_VERSION;
  private static final MethodHandle PARSE_STRUCT;

  static {
    SymbolLookup lib = SymbolLookup.libraryLookup(resolveLibPath(), Arena.global());
    ABI_VERSION = LINKER.downcallHandle(
        findOrThrow(lib, "np_abi_version"),
        FunctionDescriptor.of(JAVA_INT));
    PARSE_STRUCT = LINKER.downcallHandle(
        findOrThrow(lib, "np_parse_struct"),
        FunctionDescriptor.of(JAVA_LONG, ADDRESS, ADDRESS, ADDRESS, ADDRESS, ADDRESS, JAVA_LONG));

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
   * Resolves the cdylib to load, in priority order:
   * <ol>
   *   <li>{@code -Dnameparser.ffi.lib} system property, else {@code NAMEPARSER_FFI_LIB} env var —
   *       an explicit override for dev or a deployment that ships the cdylib separately;</li>
   *   <li>a cdylib <b>bundled in this JAR</b> ({@code /native/<classifier>/<libname>}), extracted
   *       to a temp file — the distributable path that makes the JAR self-contained;</li>
   *   <li>the repo-relative build output ({@code ../../target/release/}) — the in-tree dev
   *       fallback when neither an override nor a bundled resource is present.</li>
   * </ol>
   */
  private static Path resolveLibPath() {
    String path = System.getProperty("nameparser.ffi.lib");
    if (path == null || path.isBlank()) {
      path = System.getenv("NAMEPARSER_FFI_LIB");
    }
    if (path != null && !path.isBlank()) {
      return Path.of(path);
    }
    Path bundled = extractBundledLib();
    if (bundled != null) {
      return bundled;
    }
    return Path.of("../../target/release/" + defaultLibFileName());
  }

  /**
   * Extracts the platform's cdylib bundled in this JAR (resource {@code /native/<classifier>/
   * <libname>}, {@code <classifier>} matching os-maven-plugin's {@code ${os.detected.classifier}})
   * to a temp file deleted on JVM exit, returning its path — or {@code null} when no such resource
   * is bundled (a plain {@code cargo}/dev build), letting {@link #resolveLibPath} fall through.
   */
  private static Path extractBundledLib() {
    String libName = defaultLibFileName();
    String resource = "/native/" + osDetectedClassifier() + "/" + libName;
    try (InputStream in = Ffi.class.getResourceAsStream(resource)) {
      if (in == null) {
        return null;
      }
      int dot = libName.lastIndexOf('.');
      Path tmp = Files.createTempFile("nameparser_ffi", dot >= 0 ? libName.substring(dot) : ".lib");
      tmp.toFile().deleteOnExit();
      Files.copy(in, tmp, StandardCopyOption.REPLACE_EXISTING);
      return tmp;
    } catch (IOException e) {
      throw new IllegalStateException(
          "failed to extract bundled nameparser-ffi native library " + resource, e);
    }
  }

  /**
   * The os-maven-plugin {@code ${os.detected.classifier}} for the running JVM — e.g.
   * {@code osx-aarch_64}, {@code linux-x86_64}, {@code windows-x86_64} — so this lookup and the
   * pom's resource copy agree on the {@code /native/<classifier>/} directory name.
   */
  private static String osDetectedClassifier() {
    String os = System.getProperty("os.name", "").toLowerCase();
    String arch = System.getProperty("os.arch", "").toLowerCase();
    String o;
    if (os.startsWith("mac") || os.contains("os x") || os.contains("darwin")) {
      o = "osx";
    } else if (os.contains("win")) {
      o = "windows";
    } else if (os.contains("linux")) {
      o = "linux";
    } else {
      o = os.replaceAll("[^a-z0-9]+", "");
    }
    String a;
    if (arch.equals("aarch64") || arch.equals("arm64")) {
      a = "aarch_64";
    } else if (arch.equals("x86_64") || arch.equals("amd64")) {
      a = "x86_64";
    } else {
      a = arch.replaceAll("[^a-z0-9]+", "_");
    }
    return o + "-" + a;
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

  /** Initial scratch buffer size for {@link #callParseStruct}: comfortably above the format's
   *  own minimum successful-encode size (208 bytes, an empty {@code ParsedName}) and every
   *  ordinary name/authorship, so the overflow-retry path below is only actually exercised for
   *  unusually long inputs. Verified end-to-end (retry logic exercised on literally every parse
   *  in the 11,302-name corpus, still 0 diffs) with this constant temporarily set as low as 64.
   *
   *  <p><b>Must stay &ge; {@link StructCodec#HEADER_SIZE} (36).</b> The unparsable path (native
   *  return {@code -1}) writes the header followed by the error name (ABI 3) and — like the success
   *  path — overflow-codes and is retried when the whole buffer doesn't fit (see {@code layout.rs}'s
   *  "Return convention"). At 4096 that retry is only ever hit for an unusually long name; the floor
   *  keeps even the first write from truncating the header. This invariant is enforced at class-init
   *  time by the {@code static} block below (not merely documented). */
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
              + "): the unparsable path's header+name write would otherwise be truncated"));
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
   * Calls {@code np_parse_struct} over the FFM boundary and turns its flat binary result into the
   * 5.0.0 {@link ParseResult} (never throwing for an unparsable name): a {@code -1} header becomes
   * {@link StructCodec#unparsableResult}, and a success buffer is decoded via
   * {@link StructCodec#decode} then split into {@code Parsed}/{@code Informal} by
   * {@link StructCodec#toParseResult}. {@code authorship}/{@code rank}/{@code code} are each
   * marshalled as {@link MemorySegment#NULL} when null.
   *
   * <p>Implements the overflow-retry protocol {@code np_parse_struct} documents (see {@code
   * layout.rs}): a first attempt against a {@link #INITIAL_STRUCT_BUFFER_BYTES}-byte scratch
   * buffer, and -- only if that overflows -- exactly one retry against a buffer sized to the
   * reported {@code needed} count. Both calls happen inside the same confined {@link Arena}, and
   * {@link StructCodec#decode} runs before that arena closes (it copies every string it needs
   * into plain Java {@code String}s, so the returned result holds no live reference into native
   * memory once this method returns).
   */
  static ParseResult callParseStruct(String name, String authorship, String rank, String code) {
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
        // -1 (unparsable — now variable-length, so it too can overflow the first buffer) is a
        // valid retry outcome; only a caught panic (-2) or a second overflow is a real failure.
        if (ret < -1) {
          throw new IllegalStateException("nameparser-ffi: np_parse_struct still failed (ret=" + ret
              + ") after retrying with the exact reported size (" + needed + " bytes) for name '" + name + "'");
        }
      }

      if (ret == -2) {
        throw new IllegalStateException(
            "nameparser-ffi internal error: np_parse_struct returned -2 (caught panic) for name '" + name + "'");
      }
      if (ret == -1) {
        return StructCodec.unparsableResult(out, name);
      }
      return StructCodec.toParseResult(StructCodec.decode(out, (int) ret));
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
