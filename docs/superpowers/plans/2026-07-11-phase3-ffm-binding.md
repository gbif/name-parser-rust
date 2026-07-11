# Phase 3 — Java FFM (Panama) binding

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use `- [ ]`.

**Goal:** Prove the design's end-state Java path — an in-process Java caller getting a real `ParsedName` from the Rust core via `java.lang.foreign` (Panama/FFM) — by building `nameparser-ffi` (a C-ABI cdylib) and a self-contained `NameParserRust implements NameParser`, then measuring it against the Java parser. Per the confirmed decision, build **both** wire formats (JSON string, then the design's flat fixed-layout struct), A/B them in JMH, and ship the winner.

**Architecture:** A new Rust crate `crates/nameparser-ffi` wraps `nameparser::parse` behind a stable `extern "C"` surface — first `np_parse_json` (returns the proven Phase-1/2 JSON), later `np_parse_struct` (fixed binary layout). A new **standalone** Maven project `bindings/java/` (NOT part of the GBIF name-parser reactor — that reactor is `--release 17` and forbids FFM) implements `org.gbif.nameparser.api.NameParser` by downcalling the cdylib via FFM, rebuilding a Java `ParsedName`. Two JUnit gates: a **parity** test (`NameParserRust` vs `NameParserImpl` over the corpora → zero diffs) and a **JMH** benchmark (single-name, in-process → assess ≥2×).

**Tech Stack:** Rust 2021 (`nameparser` + `serde_json`; cdylib). Java 25 (`25.0.3-librca` — the only locally-installed JDK ≥22, where `java.lang.foreign` is finalized), compiled `--release 22`. Maven (standalone POM, no GBIF parent). Deps: `org.gbif:name-parser-api:4.2.0-SNAPSHOT` (interface + model + enums + exception), `org.gbif:name-parser:4.2.0-SNAPSHOT` (test-scope oracle), `com.google.code.gson:gson`, JUnit 5, `org.openjdk.jmh`.

**Recon (read before Task 1):** `/private/tmp/claude-501/-Users-markus-code-gbif-name-parser-rust/dbbfb8b2-eb56-450c-a20a-0774ef659766/scratchpad/phase3-java-recon.md` — the authoritative map of the Java API surface (interface signatures, every `ParsedName`/`ParsedAuthorship` setter + its side effects, `UnparsableNameException` constructors, enum FQNs/counts, module layout, JDK inventory). Java source lives at `/Users/markus/code/gbif/name-parser/`.

## Global Constraints

- **Target the HEAD (4.2.0-SNAPSHOT) interface, not 3.16.0.** Implement `org.gbif.nameparser.api.NameParser` as it exists at the checked-out HEAD: the one non-default method `ParsedName parse(String scientificName, @Nullable String authorship, @Nullable Rank rank, @Nullable NomCode code) throws UnparsableNameException`. The interface does **not** extend `AutoCloseable`; there is **no** `close()` and **no** `InterruptedException`. The two `@Deprecated` `parse` overloads and `parseAuthorship` are `default` methods — inherited free, do not re-implement. (The CoL backend still runs 3.16.0 with a different-shaped interface; that upgrade is out of Phase-3 scope — record it as a Phase-5 prerequisite in the status doc.)
- **Enum mapping key = `.name()` (the constant identifier), never `ordinal()`, for the JSON path.** For the flat-struct path enums cross as `i32` ordinals, but ONLY behind an ABI-version guard plus a startup consistency check (Task 6) — because Rust and Java declare these enums in the same order (Rank 117 confirmed on both sides). The five enums: `Rank`, `NomCode`, `NameType` (5 constants at HEAD: SCIENTIFIC, FORMULA, INFORMAL, PLACEHOLDER, OTHER), `ParsedName.State` (nested), `NamePart`.
- **The FFI boundary must never unwind.** Wrap every `extern "C"` body in `std::panic::catch_unwind`; on panic return a null pointer (JSON path) or a negative status (struct path). Rust owns every heap pointer it returns — Java must hand it back via `np_free` (JSON) and never free native memory itself.
- **gson round-trip is the JSON rebuild.** `GSON.fromJson(json, ParsedName.class)` reconstructs fields by reflection, bypassing the side-effecting setters (× → addNotho, setPublishedIn → auto year) — which is correct, because the Rust JSON already carries final field values. The parity test (re-serialize both sides through one `Gson`, set-compare) is the guard. If gson cannot round-trip `EnumSet<NamePart> notho` / `EnumMap<NamePart,String> epithetQualifier` / `HashSet<String> warnings`, register minimal `TypeAdapter`s — do not hand-map via setters on the JSON path.
- **Self-contained + reversible.** All Java lives under `bindings/java/` in THIS repo (`name-parser-rust`). Do not modify `/Users/markus/code/gbif/name-parser/` or the CoL backend. The module depends on the published `name-parser-api`/`name-parser` artifacts from `~/.m2`.
- **cdylib discovery for the proof:** load the native library from an explicit path — the system property `-Dnameparser.ffi.lib=<abs path>`, else env `NAMEPARSER_FFI_LIB`, else the repo-relative default `crates/../target/release/libnameparser_ffi.{dylib,so}`. Extract-from-jar (the netty/sqlite-jdbc pattern) is Phase-5 packaging — out of scope; note it deferred.
- SPDX header on every new source file; Rust workspace crate version `0.0.0`. The core `nameparser` crate stays untouched except a tiny additive `from_name` enum helper if one is not already present (Task 1). No new dependency in the core.
- **Working dir** `/Users/markus/code/gbif/name-parser-rust/`. **Preamble for every bash invocation:**
  ```
  export PATH="$HOME/.cargo/bin:$PATH"
  export JAVA_HOME="$HOME/.sdkman/candidates/java/25.0.3-librca"
  export PATH="$JAVA_HOME/bin:$PATH"
  ```
  Verify once with `java -version` (expect `25.0.3`) and `cargo --version`.

## File structure

```
crates/nameparser-ffi/
  Cargo.toml                     # cdylib + rlib; deps: nameparser (path), serde_json
  src/lib.rs                     # np_abi_version, np_parse_json, np_free  (Task 1)
                                 #   + np_parse_struct, layout consts       (Task 5)
  src/layout.rs                  # flat-struct field offsets + string-table codec (Task 5)
  tests/ffi_json.rs              # Rust-side C-ABI tests for the JSON entry   (Task 1)
  tests/ffi_struct.rs            # Rust-side C-ABI tests for the struct entry  (Task 5)
bindings/java/
  pom.xml                        # standalone; --release 22; no GBIF parent   (Task 2)
  README.md                      # how to build the cdylib + run tests/JMH    (Task 2)
  src/main/java/org/gbif/nameparser/rust/
    NameParserRust.java          # implements NameParser; JSON path           (Task 2)
                                 #   + WireFormat.STRUCT path                 (Task 6)
    Ffi.java                     # FFM handle setup, lib loading, downcalls    (Task 2/6)
    StructCodec.java             # VarHandle reads of the flat struct          (Task 6)
  src/test/java/org/gbif/nameparser/rust/
    NameParserRustSmokeTest.java # single-name end-to-end (both formats)       (Task 2/6)
    ParityTest.java              # corpora vs NameParserImpl, zero diffs       (Task 3/6)
  jmh/                           # JMH sub-tree (own pom or profile)
    pom.xml
    src/main/java/.../ParseBench.java                                          (Task 4/6)
Cargo.toml                       # workspace: add crates/nameparser-ffi member (Task 1)
```

---

## Task 1: `nameparser-ffi` crate — JSON C ABI

**Files:** Create `crates/nameparser-ffi/{Cargo.toml,src/lib.rs}`, `crates/nameparser-ffi/tests/ffi_json.rs`; modify the workspace `Cargo.toml` (add member). Possibly modify `crates/nameparser/src/model/enums.rs` (additive `from_name` if absent).

**Interfaces produced (the C ABI other tasks bind):**
- `extern "C" fn np_abi_version() -> u32` — returns `1` (bump on any ABI change).
- `extern "C" fn np_parse_json(name: *const c_char, authorship: *const c_char, rank: *const c_char, code: *const c_char) -> *mut c_char` — `name` required; the other three nullable (null ptr = `None`). `rank`/`code` are the Java enum `.name()` strings. Returns a heap `CString` (caller frees via `np_free`): on success the ParsedName JSON (identical to `nameparser`'s serialization); on an unparsable name the error object `{"error":{"type":"...","code":"...|null","message":"..."}}`; on internal panic, a null pointer.
- `extern "C" fn np_free(ptr: *mut c_char)` — retakes and drops a pointer previously returned by `np_parse_json`; null-safe.

- [ ] **Step 1: Crate + workspace wiring.** `crates/nameparser-ffi/Cargo.toml`:
  ```toml
  [package]
  name = "nameparser-ffi"
  version = "0.0.0"
  edition = "2021"
  license = "Apache-2.0"

  [lib]
  crate-type = ["cdylib", "rlib"]   # cdylib for Java; rlib so tests/ can call the extern fns

  [dependencies]
  nameparser = { path = "../nameparser" }
  serde_json = "1"
  ```
  Add `"crates/nameparser-ffi"` to the workspace `members` in the root `Cargo.toml`.

- [ ] **Step 2: enum name→value helper.** The FFI must turn a Java `Rank.name()` / `NomCode.name()` string into the core's enum. First `grep -rn "fn from_name\|fn from_str_name\|FromStr for Rank" crates/nameparser/src/`. If a name-lookup already exists, use it. Otherwise add to `crates/nameparser/src/model/enums.rs`, for BOTH `Rank` and `NomCode` (they derive `serde::Deserialize` with the by-name representation used for serialization):
  ```rust
  impl Rank {
      /// Look up a variant by its serialized SCREAMING_SNAKE name (Java `Rank.name()`).
      pub fn from_name(name: &str) -> Option<Rank> {
          serde_json::from_value(serde_json::Value::String(name.to_string())).ok()
      }
  }
  ```
  (Same for `NomCode`. If `enums.rs` must not depend on `serde_json`, instead generate an explicit `match` over the variants — either is fine; a unit test in Step 4 pins it.) Keep it additive; do not touch existing code.

- [ ] **Step 3: `src/lib.rs`.** SPDX header. Implement the three functions. Key skeleton (fill in faithfully):
  ```rust
  use std::ffi::{c_char, CStr, CString};
  use std::panic::catch_unwind;

  #[no_mangle]
  pub extern "C" fn np_abi_version() -> u32 { 1 }

  /// SAFETY: `p` is null or a valid C string for the duration of the call.
  unsafe fn opt_str<'a>(p: *const c_char) -> Option<&'a str> {
      if p.is_null() { None } else { CStr::from_ptr(p).to_str().ok() }
  }

  #[no_mangle]
  pub extern "C" fn np_parse_json(
      name: *const c_char, authorship: *const c_char,
      rank: *const c_char, code: *const c_char,
  ) -> *mut c_char {
      let result = catch_unwind(|| unsafe {
          let name = match opt_str(name) { Some(s) => s, None => return err_json("NO_NAME", None, "null name") };
          let authorship = opt_str(authorship);
          let rank = opt_str(rank).and_then(nameparser::Rank::from_name);
          let code = opt_str(code).and_then(nameparser::NomCode::from_name);
          match nameparser::parse(name, authorship, rank, code) {
              Ok(pn)  => serde_json::to_string(&pn).unwrap_or_else(|_| err_json_str("OTHER", None, "serialize failed")),
              Err(e)  => unparsable_json(&e),   // build {"error":{type,code,message}} from the error
          }
      });
      match result {
          Ok(s)  => CString::new(s).map(CString::into_raw).unwrap_or(std::ptr::null_mut()),
          Err(_) => std::ptr::null_mut(),       // never unwind into Java
      }
  }

  #[no_mangle]
  pub extern "C" fn np_free(ptr: *mut c_char) {
      if !ptr.is_null() { unsafe { drop(CString::from_raw(ptr)); } }
  }
  ```
  Match the exact call signature of `nameparser::parse` and the exact `Err` type in the core (`grep -n "pub fn parse" crates/nameparser/src/lib.rs`; inspect the error/`UnparsableName` shape). The error JSON's `type`/`code` must be the same `NameType`/`NomCode` `.name()` strings the Phase-2 CLI emits for unparsable names (compare against `crates/nameparser-cli/src/main.rs`'s error-row logic so both agree). Reuse that logic; do not invent a new error shape.

- [ ] **Step 4: Rust-side C-ABI tests** (`tests/ffi_json.rs`). Call the extern fns directly (the rlib exposes them). Use a helper `fn cstr(s:&str)->CString`. Assert:
  - A parsable name (`"Vulpes vulpes silaceus Miller, 1907"`, all-null rank/authorship/code) → free-standing: read the returned `*mut c_char` back to a `String`, `serde_json` it, assert `genus=="Vulpes"`, `specificEpithet=="vulpes"`, `infraspecificEpithet=="silaceus"`, `combinationAuthorship.year=="1907"`. Then `np_free` it (no crash).
  - The SAME name compared byte-for-byte against `serde_json::to_string(&nameparser::parse(...))` — the FFI adds nothing.
  - An unparsable name (`"Tobacco mosaic virus"`) → returned JSON has an `error` object with the expected `type`/`code`.
  - `np_parse_json(null, …)` → returns non-null error JSON (not a crash); `np_free(null)` is a no-op.
  - `Rank::from_name("SPECIES")==Some(SPECIES)`, `NomCode::from_name("BOTANICAL")==Some(BOTANICAL)`, `Rank::from_name("NONSENSE")==None`.

- [ ] **Step 5: Build the cdylib + verify the symbol.** Run:
  - `cargo test -p nameparser-ffi` → all green.
  - `cargo build -p nameparser-ffi --release` → produces `target/release/libnameparser_ffi.dylib`.
  - `nm -gU target/release/libnameparser_ffi.dylib | grep -E 'np_parse_json|np_free|np_abi_version'` → all three symbols exported (unmangled).

- [ ] **Step 6: Commit** `Phase 3: nameparser-ffi crate + np_parse_json C ABI`.

---

## Task 2: `bindings/java/` module + `NameParserRust` (JSON path) via FFM

**Files:** Create `bindings/java/pom.xml`, `bindings/java/README.md`, `bindings/java/src/main/java/org/gbif/nameparser/rust/{NameParserRust.java,Ffi.java}`, `bindings/java/src/test/java/org/gbif/nameparser/rust/NameParserRustSmokeTest.java`.

**Interfaces produced:** `org.gbif.nameparser.rust.NameParserRust implements org.gbif.nameparser.api.NameParser`, constructible `new NameParserRust()` (JSON wire format) — Task 6 adds a `WireFormat` selector.

- [ ] **Step 1: Ensure the Java oracle artifacts are in `~/.m2`.** The binding depends on `name-parser-api` + `name-parser` at the checkout's version. Read the version: `grep -m1 "<version>" /Users/markus/code/gbif/name-parser/pom.xml` (expect `4.2.0-SNAPSHOT`). Install from source so `~/.m2` is guaranteed current and matches the Rust oracle:
  ```
  (cd /Users/markus/code/gbif/name-parser && mvn -q -DskipTests -pl name-parser-api,name-parser -am install)
  ```
  Confirm: `ls ~/.m2/repository/org/gbif/name-parser-api/4.2.0-SNAPSHOT/` and `.../name-parser/4.2.0-SNAPSHOT/` both contain a jar. Use whatever version string the pom reported in the `<dependency>` blocks below.

- [ ] **Step 2: `pom.xml`** — standalone (no `<parent>`; the GBIF motherpom enforces `--release 17` and would reject FFM). Group `org.gbif.nameparser`, artifact `name-parser-rust-binding`, version `0.0.0`. Set `<maven.compiler.release>22</maven.compiler.release>`. Dependencies: `org.gbif:name-parser-api:4.2.0-SNAPSHOT` (compile), `com.google.code.gson:gson:2.11.0` (compile), `org.gbif:name-parser:4.2.0-SNAPSHOT` (test), `org.junit.jupiter:junit-jupiter:5.10.2` (test). Configure `maven-surefire-plugin` with `<argLine>--enable-native-access=ALL-UNNAMED -Dnameparser.ffi.lib=${nameparser.ffi.lib}</argLine>` and a property `<nameparser.ffi.lib>${project.basedir}/../../target/release/libnameparser_ffi.dylib</nameparser.ffi.lib>` (a `.so` fallback profile for Linux is fine to add; macOS is the dev target). `README.md`: the two-step build (`cargo build -p nameparser-ffi --release`, then `mvn -f bindings/java/pom.xml test`), the lib-path override property, and the JDK-25 requirement.

- [ ] **Step 3: `Ffi.java`** — the FFM plumbing, isolated from the parser logic:
  - Static `load()`: resolve the lib path (`System.getProperty("nameparser.ffi.lib")` → `System.getenv("NAMEPARSER_FFI_LIB")` → the repo-relative default). `SymbolLookup lib = SymbolLookup.libraryLookup(Path.of(path), Arena.global());`
  - `Linker linker = Linker.nativeLinker();`
  - Build downcall handles:
    - `np_abi_version` : `FunctionDescriptor.of(JAVA_INT)`
    - `np_parse_json`  : `FunctionDescriptor.of(ADDRESS, ADDRESS, ADDRESS, ADDRESS, ADDRESS)` — mark the returned `ADDRESS` unbounded on read via `.reinterpret(...)`.
    - `np_free`        : `FunctionDescriptor.ofVoid(ADDRESS)`
  - Verify `np_abi_version() == 1` at load; throw `ExceptionInInitializerError` otherwise (fail fast on desync).
  - Helper `String callParseJson(String name, String authorship, String rank, String code)`: in a `try (Arena a = Arena.ofConfined())`, encode each non-null arg with `a.allocateFrom(s)` (JDK 22+ UTF-8, null-terminated) and pass `MemorySegment.NULL` for nulls; invoke; the result `MemorySegment` is zero-length — `res.reinterpret(Long.MAX_VALUE)` then `res.getString(0)` (UTF-8) to copy into a Java `String`; then `np_free(res)` in a `finally`. Return the string (or null if `res` is `NULL`).

- [ ] **Step 4: `NameParserRust.java`** — `implements NameParser`. A shared `static final Gson GSON = new GsonBuilder().create();` (add `TypeAdapter`s only if Step 6 tests show a round-trip gap). Implement only:
  ```java
  @Override
  public ParsedName parse(String scientificName, String authorship, Rank rank, NomCode code)
      throws UnparsableNameException {
    String json = Ffi.callParseJson(scientificName,
        authorship, rank == null ? null : rank.name(), code == null ? null : code.name());
    if (json == null) throw new UnparsableNameException(NameType.OTHER, scientificName, "native parse returned null");
    JsonObject o = JsonParser.parseString(json).getAsJsonObject();
    if (o.has("error")) {
      JsonObject e = o.getAsJsonObject("error");
      NameType t = NameType.valueOf(e.get("type").getAsString());
      NomCode c = e.has("code") && !e.get("code").isJsonNull() ? NomCode.valueOf(e.get("code").getAsString()) : null;
      throw new UnparsableNameException(t, c, scientificName);
    }
    return GSON.fromJson(o, ParsedName.class);
  }
  ```
  The inherited `default` methods cover the rest. (Do NOT implement `close()` — the HEAD interface has none.)

- [ ] **Step 5: `NameParserRustSmokeTest.java`** — end-to-end over the real FFM boundary:
  - `new NameParserRust().parse("Vulpes vulpes silaceus Miller, 1907", null, null, null)` → assert `getGenus()=="Vulpes"`, `getSpecificEpithet()=="vulpes"`, `getInfraspecificEpithet()=="silaceus"`, `getRank()==Rank.SUBSPECIES`, `getCombinationAuthorship().getYear()=="1907"`, authors contains `"Miller"`.
  - A name with an explicit authorship arg (`parse("Abies alba", "Mill.", Rank.SPECIES, NomCode.BOTANICAL)`) → assert the authorship attaches (proves rank/code/authorship marshalling).
  - `parse("Tobacco mosaic virus", null, null, null)` → `assertThrows(UnparsableNameException.class, ...)`; assert `getType()`/`getName()` on the caught exception.
  - Round-trip fidelity for the trickier fields: parse a hybrid (`"Salix ×capreola"` or a corpus name with notho) and a name with multiple authors + a warning, then re-serialize with the same `GSON` and assert the collections survived (`notho`, `warnings`, `authors`). If any field is dropped/garbled, add the minimal gson `TypeAdapter` in Step 4 and note it.

- [ ] **Step 6: Run + commit.** `cargo build -p nameparser-ffi --release` then `mvn -q -f bindings/java/pom.xml test` → green. Commit `Phase 3: bindings/java NameParserRust JSON path over FFM`.

---

## Task 3: Parity test — `NameParserRust` (JSON) vs `NameParserImpl` over the corpora

**Files:** Create `bindings/java/src/test/java/org/gbif/nameparser/rust/ParityTest.java`.

**Realizes the gate's "zero diffs on real data" half — now over the in-process FFM path.**

- [ ] **Step 1: Corpus discovery + a re-serialize-and-set-compare helper.** The corpora are this repo's `testdata/` (relative to the module: `${project.basedir}/../../testdata/`): `benchmark-data.txt` (8017) + `names-with-authors.txt`, `hybrids.txt`, `other.txt`, `otu.txt`, `placeholder.txt`, `viruses.txt`. Read each line the same way the Java/Rust CLIs do (name = text before the first TAB; skip blank lines and `#` comments). Write a comparator that, given two `ParsedName`s (or two thrown `UnparsableNameException`s), reports equal/not: serialize both via one shared `Gson`, then compare as `JsonObject`s where `warnings`, `notho`, and `epithetQualifier` are compared **order-insensitively** (as sets/maps) and all other fields byte-for-byte — the exact rule the Phase-2 `compare` command uses (mirror `UNORDERED_FIELD_KEYS` from `crates/nameparser-cli/src/main.rs`). For unparsable names, "equal" = both threw with the same `NameType` and `NomCode`.

- [ ] **Step 2: The parametrized test.** Instantiate one `NameParserRust rust = new NameParserRust();` and one `NameParserImpl java = new NameParserImpl();`. For every corpus, for every name: parse with both (catching `UnparsableNameException` on each side), compare with the helper, tally mismatches, and collect up to 20 example diffs. Assert **0 mismatches total** across all corpora. On failure, print the examples (name, both JSONs) so triage is immediate. Expected: 0 — Rust already cross-validated 11,302/11,302 vs the Java CLI in Phase 2; this re-proves it through FFM + gson round-trip. Any mismatch here is an FFM-marshalling or round-trip bug (not a core parser bug), so investigate the binding, not the core.

- [ ] **Step 3: Run + commit.** `mvn -q -f bindings/java/pom.xml test` (rebuild the cdylib first if the core changed). Commit `Phase 3: parity test NameParserRust vs NameParserImpl — zero diffs on corpora`.

---

## Task 4: JMH — `NameParserRust` (JSON) vs `NameParserImpl`, single-name

**Files:** Create `bindings/java/jmh/pom.xml`, `bindings/java/jmh/src/main/java/org/gbif/nameparser/rust/jmh/ParseBench.java`.

**Realizes the gate's "≥2× in JMH" half. Deliverable = an honest measurement + analysis, assessed against the design's ≥2× target — not a build-blocking assertion (the design's own §5 model predicts in-process ≈2–2.5×, capped by the Java-object-build floor; the true ratio is the empirical question this task answers).**

- [ ] **Step 1: JMH module.** Standalone `bindings/java/jmh/pom.xml`, `--release 22`, deps: the binding module (`org.gbif.nameparser:name-parser-rust-binding:0.0.0`), `org.gbif:name-parser:4.2.0-SNAPSHOT`, `org.openjdk.jmh:jmh-core:1.37`, `org.openjdk.jmh:jmh-generator-annprocess:1.37` (as an `annotationProcessorPaths` entry). Shade into `target/benchmarks.jar` with `Main-Class: org.openjdk.jmh.Main` (standard JMH shade config). The cdylib path passes via `-Dnameparser.ffi.lib=…` on the `java -jar` line (document it in the module README).

- [ ] **Step 2: `ParseBench.java`.** `@State(Scope.Benchmark)` loads a fixed representative sample into a `String[]` at `@Setup` — read the first ~2,000 names of `testdata/benchmark-data.txt` (a stable slice; JMH warms the JIT itself). One `@State`-held `NameParserRust` and one `NameParserImpl`. Three `@Benchmark`s, each iterating the sample and consuming results via a `Blackhole` (swallow `UnparsableNameException`):
  - `javaImpl` — `NameParserImpl.parse`
  - `rustJson` — `NameParserRust.parse` (JSON path)
  - (Task 6 adds `rustStruct`)
  Config: `@BenchmarkMode(Mode.AverageTime)`, `@OutputTimeUnit(TimeUnit.MICROSECONDS)`, `@Warmup(iterations=5)`, `@Measurement(iterations=5)`, `@Fork(2)`.

- [ ] **Step 3: Run + record.** `cargo build -p nameparser-ffi --release`; build the JMH jar; `java --enable-native-access=ALL-UNNAMED -Dnameparser.ffi.lib=$PWD/target/release/libnameparser_ffi.dylib -jar bindings/java/jmh/target/benchmarks.jar -rf json -rff bindings/java/jmh/results-json.json`. Capture avg µs/op for `javaImpl` and `rustJson` and the ratio into the task report. Commit `Phase 3: JMH single-name benchmark — NameParserRust(JSON) vs NameParserImpl` (include the results json).

---

## Task 5: `nameparser-ffi` — flat fixed-layout struct C ABI

**Files:** Modify `crates/nameparser-ffi/src/lib.rs`; create `crates/nameparser-ffi/src/layout.rs`, `crates/nameparser-ffi/tests/ffi_struct.rs`.

**Interface produced:** `extern "C" fn np_parse_struct(name, authorship, rank, code: *const c_char, out: *mut u8, out_cap: usize) -> i64`. Return convention: `>= 0` = number of bytes written (success); `-1` = unparsable (still writes the `status`+`type`+`code` header fields so Java can throw); `-2` = panic/internal error; if the needed size exceeds `out_cap`, return `-(needed as i64 + 3)` so Java decodes `needed = -ret - 3` and retries with a larger buffer. Document this convention in `layout.rs` and mirror it in `StructCodec.java` (Task 6).

- [ ] **Step 1: Define the layout (`layout.rs`).** A single canonical description both languages read. A fixed header of scalar fields at explicit byte offsets, followed by a string-table region. Concretely:
  - `abi_version: u32` at offset 0 (must equal `np_abi_version()`), `status: i32`, then the enums as `i32` (`rank`, `code`, `name_type`, `state`; `-1` = absent/null), flags as `u8` (`candidatus`, `doubtful`, `manuscript`, `extinct`, `original_spelling` as `0/1/2` for false/true/unknown), `published_in_year: i32` (`-1` absent), and a `notho_bits: u8` (bitset over the 4 `NamePart` ordinals).
  - A **string table**: a `u32 count` of entries then `count` × (`u32 offset`, `u32 len`) pairs into a trailing UTF-8 blob, addressed by a fixed enum of slot indices (`UNINOMIAL=0, GENUS=1, INFRAGENERIC=2, SPECIFIC=3, INFRASPECIFIC=4, CULTIVAR=5, PHRASE=6, TAXONOMIC_NOTE=7, NOMENCLATURAL_NOTE=8, PUBLISHED_IN=9, PUBLISHED_IN_PAGE=10, UNPARSED=11, SANCTIONING_AUTHOR=12, YEAR_COMB=13, YEAR_BAS=14`). Repeated groups — `combination` authors, `basionym` authors, ex-authors of each, `warnings`, `epithetQualifier` entries — encode as a slot holding a `count` plus a contiguous run (define `AUTHORS_COMB`, `EXAUTHORS_COMB`, `AUTHORS_BAS`, `EXAUTHORS_BAS`, `WARNINGS`, `EPITHET_QUALIFIER` as run-slots: `u32 count` then `count` string refs; `epithetQualifier` runs as `count` × (`u32 namepart_ordinal`, string ref)). Keep offsets in named `const`s. This is the intricate part — the design's "scalar fields at known offsets + string table" made concrete.

- [ ] **Step 2: `np_parse_struct`.** `catch_unwind`-wrapped. Parse (same input handling as `np_parse_json`). On success, serialize the `ParsedName` into a `Vec<u8>` per the layout, then, if it fits `out_cap`, `copy_nonoverlapping` into `out` and return the length; else return the negative-needed code. On unparsable, write only the header (`status=-1`, `name_type`, `code`) and return `-1`. Read every field off the SAME `ParsedName` the JSON path uses — do not re-derive.

- [ ] **Step 3: Rust-side tests (`ffi_struct.rs`).** Encode a known name into a `Vec<u8>` buffer, then decode it back **in the test** (write a small decoder mirroring the layout) and assert every field equals `nameparser::parse(...)` — genus/epithets, authors/ex-authors/year on both authorships, warnings set, notho bits, the flags, published-in-year. Cover: an autonym, a hybrid (notho bits), a name with warnings, a name with ex-authors, and the overflow path (pass `out_cap=0` → assert the return is `-(needed+3)` and `needed>0`; then a buffer of exactly `needed` succeeds). Assert `abi_version` in the header equals `np_abi_version()`.

- [ ] **Step 4: Build + commit.** `cargo test -p nameparser-ffi` green; `cargo build -p nameparser-ffi --release`; `nm -gU …/libnameparser_ffi.dylib | grep np_parse_struct` present. Commit `Phase 3: nameparser-ffi np_parse_struct flat-layout C ABI`.

---

## Task 6: `NameParserRust` flat-struct path + JMH A/B → pick the winner

**Files:** Modify `NameParserRust.java`, `Ffi.java`, `NameParserRustSmokeTest.java`, `ParityTest.java`, `bindings/java/jmh/.../ParseBench.java`; create `bindings/java/src/main/java/org/gbif/nameparser/rust/StructCodec.java`.

- [ ] **Step 1: `WireFormat` selector.** Add `enum WireFormat { JSON, STRUCT }` and constructors `NameParserRust()` (defaults to `JSON`) and `NameParserRust(WireFormat)`. `parse(...)` dispatches on the format.

- [ ] **Step 2: Enum-ordinal consistency guard.** Because the struct path maps enums by `i32` ordinal, add a one-time startup check in `Ffi` (or `StructCodec`): assert `np_abi_version()==1` AND that the Java enum ordinals still match what the ABI was built against. Concretely, verify `Rank.values().length == 117`, `NameType.values().length == 5`, `NamePart.values().length == 4`, and spot-check a few identities the codec relies on (`Rank.SPECIES.ordinal()`, `NameType.OTHER.ordinal()`, each `NamePart` ordinal) against constants recorded from the Rust side. On mismatch, throw with a clear "Rust/Java enum ABI desync — rebuild the cdylib" message. (This is the design's "layout-version field prevents silent desync", enforced.)

- [ ] **Step 3: `Ffi.callParseStruct` + `StructCodec`.** `callParseStruct(...)`: allocate a confined `Arena` scratch segment (start e.g. 4 KiB), downcall `np_parse_struct` with `out`+`out_cap`; if the return is the negative-needed code, re-allocate `needed` and retry once; on `-1` read the header and throw `UnparsableNameException`; on `-2` throw. `StructCodec.decode(MemorySegment, ParsedName)` reads scalars via `VarHandle`s at the `layout.rs` offsets and the string table via slot indices, then **populates the `ParsedName` using the correct setters** — this is where the recon's setter semantics are load-bearing:
  - Use `setGenus/setUninomial/setInfragenericEpithet/setSpecificEpithet/setInfraspecificEpithet` — but note these strip a leading `×` and call `addNotho(...)`. The Rust struct already carries the de-`×`'d epithet + the notho bits, so **set the notho set explicitly from `notho_bits` AFTER setting epithets** (loop `addNotho(NamePart)` per set bit), so the final `notho` equals Rust's regardless of setter side effects. (Or set epithets, then `setNotho`/`addNotho` to the exact Rust set.)
  - `setPublishedIn` auto-derives `publishedInYear`; then call `setPublishedInYear(Integer)` with the struct's value to pin it exactly.
  - `warnings`: `addWarning(String...)` (additive) for each.
  - `epithetQualifier`: `setEpithetQualifier(NamePart, String)` per entry.
  - Authorships: build `Authorship` via `setAuthors(List)`, `setExAuthors(List)`, `setYear(String)` (do NOT use `addAuthor`/`addExAuthor` — recon flags them as inverted-blank-check no-ops), then `setCombinationAuthorship`/`setBasionymAuthorship`/`setSanctioningAuthor`; the nullable `genericAuthorship`/`specificAuthorship` only if present.
  - `setState`, `setType`, `setCode`, `setRank`, the boolean flags, `setOriginalSpelling(Boolean)` tri-state, `setCandidatus`, `setPhrase`, `setTaxonomicNote`, `setNomenclaturalNote`, `setUnparsed`.

- [ ] **Step 4: Extend the smoke + parity tests to STRUCT.** Parametrize `NameParserRustSmokeTest` and `ParityTest` over both `WireFormat`s. **The parity test must show 0 diffs for STRUCT too** — this is the correctness gate for the flat codec (it is the highest-risk code in the phase; the parity corpus is its proof). Fix the codec until zero.

- [ ] **Step 5: JMH A/B + decision.** Add the `rustStruct` `@Benchmark`. Re-run the JMH jar → a three-row table (`javaImpl`, `rustJson`, `rustStruct`) in µs/op with ratios vs `javaImpl`. Record which wire format wins and whether either clears the ≥2× target. This is the evidence the "build both" decision exists to produce.

- [ ] **Step 6: Commit** `Phase 3: NameParserRust flat-struct path + JMH A/B (JSON vs struct vs Java)`.

---

## Task 7: Phase 3 status doc + wire-format decision record

**Files:** Create `docs/superpowers/findings/2026-07-11-phase3-ffm-status.md`.

- [ ] Record, honestly and concisely (the tone of the prior findings docs):
  - **Deliverable:** in-process Java→Rust parsing works over FFM; `NameParserRust implements NameParser` (HEAD 4.2.0 interface); self-contained `bindings/java/`, cdylib from `nameparser-ffi`.
  - **Parity:** zero diffs vs `NameParserImpl` over N names, for BOTH wire formats (the gate's data half).
  - **JMH table:** `javaImpl` / `rustJson` / `rustStruct` µs/op + ratios; which won; whether ≥2× was met; interpret against the design's §5 object-build-floor prediction (in-process ≈2–2.5×).
  - **Wire-format decision:** which format ships and why (evidence-based, resolving design §4 decision #3 — the flat-struct-vs-JSON call the spike deferred).
  - **Deferred / next:** cdylib jar-bundling + extract-and-load (Phase-5 packaging); grafting `NameParserRust` into the production `name-parser` repo (Phase 5); the **CoL backend 3.16.0→4.2.0 upgrade prerequisite** and the breaks it entails (interface reshape: `AutoCloseable`/`InterruptedException`/timeout ctor gone; `NameType.NO_NAME`/`OTU`/`VIRUS` gone, `HYBRID_FORMULA`→`FORMULA`, `OTHER` new; `ParsedName.getVoucher()`/`getNominatingParty()` removed → `life.catalogue.parser.NameParser.fromParsedName()` must change; `Rank.DIVISION`→`DIVISION_ZOOLOGY`+`DIVISION_BOTANY`) — cite `name-parser/README.md` "Migrating from 3.x to 4.x"; the enum-ordinal guard as a release-time check; Phase 4 (PyO3 + extendr).
  - Commit `Phase 3: FFM binding status + wire-format decision record`.

## Self-Review

Realizes design §6.3 (the FFM boundary) and §8 Phase 3 (cdylib + `NameParserRust`; gate = zero diffs + ≥2× JMH). The "build both, A/B" decision is Tasks 1–4 (JSON) then 5–6 (struct), with Task 6's parity extension as the flat-codec correctness gate and Task 6 Step 5 as the decision point. Self-contained per the location decision (Global Constraints). Deferred + stated: jar packaging, production-repo graft, the backend 3.16→4.2 upgrade (Phase 5), Phase 4 bindings. Type consistency: `np_parse_json`/`np_free`/`np_abi_version` (Task 1) are consumed by `Ffi` (Task 2); `np_parse_struct` (Task 5) by `StructCodec` (Task 6); `ParityTest`'s comparator (Task 3) is reused for STRUCT (Task 6); the JMH `@Benchmark`s (Task 4) gain `rustStruct` (Task 6). Risk called out where it lives: the flat codec (Task 6 Step 3) is the highest-risk code and is gated by the parity corpus; the FFI never unwinds (`catch_unwind`); enum desync is guarded (Task 6 Step 2).
