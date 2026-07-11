# `bindings/java` — `NameParserRust` over FFM (Panama)

`org.gbif.nameparser.rust.NameParserRust` implements `org.gbif.nameparser.api.NameParser`
(the HEAD `4.2.0-SNAPSHOT` interface) by downcalling the `nameparser-ffi` Rust cdylib
in-process via `java.lang.foreign` (FFM/Panama, stable since JDK 22 — no
`--enable-preview` needed). This module carries the JSON wire format only: each `parse()`
call marshals a JSON string across the FFI boundary and rebuilds a `ParsedName` from it with
Gson (`GSON.fromJson(json, ParsedName.class)`, reflective field population — see
`NameParserRust`'s class doc for why that, rather than the setters, is the correct rebuild).

This module is deliberately **standalone**: it has no `<parent>` (the GBIF `name-parser`
reactor's motherpom pins `--release 17`, which rejects `java.lang.foreign`), and it does not
modify `/Users/markus/code/gbif/name-parser/` or any other repo. It depends on the
already-published `org.gbif:name-parser-api:4.2.0-SNAPSHOT` / `org.gbif:name-parser:4.2.0-SNAPSHOT`
artifacts from the local `~/.m2` repository (built from that repo's HEAD — see below).

## Requirements

- **JDK 22 or newer** to compile and run (`java.lang.foreign` is finalized in 22; this repo's
  dev/CI JDK is `25.0.3-librca`). Set `JAVA_HOME` accordingly before running Maven:
  ```sh
  export JAVA_HOME="$HOME/.sdkman/candidates/java/25.0.3-librca"
  export PATH="$JAVA_HOME/bin:$PATH"
  ```
- `org.gbif:name-parser-api:4.2.0-SNAPSHOT` and `org.gbif:name-parser:4.2.0-SNAPSHOT` present
  in `~/.m2` (built from `/Users/markus/code/gbif/name-parser` HEAD on JDK 25). If Maven
  reports either unresolved, rebuild that repo's jars there (`mvn -pl name-parser-api,name-parser -am install -DskipTests`)
  — do **not** run that from this repo/module.

## Build

Two steps, always in this order — the Java tests load the cdylib at a fixed path, so it must
exist (and be current) before `mvn test` runs:

```sh
# 1. Build the Rust cdylib the JVM will dlopen.
cargo build -p nameparser-ffi --release

# 2. Compile + run the Java tests against it.
mvn -f bindings/java/pom.xml test
```

This produces `target/release/libnameparser_ffi.dylib` (macOS) at the **workspace root**
(not inside `bindings/java/`) — the module's default `nameparser.ffi.lib` property points
there via a relative path (`${project.basedir}/../../target/release/...`).

## Pointing at a different cdylib

Resolved in this order by `Ffi.resolveLibPath()`:

1. JVM system property `-Dnameparser.ffi.lib=/abs/path/to/libnameparser_ffi.dylib`
2. Environment variable `NAMEPARSER_FFI_LIB`
3. A repo-relative default (`../../target/release/libnameparser_ffi.{dylib,so,dll}`,
   extension picked from `os.name`)

`mvn test` already wires (1) via the surefire `argLine`, sourced from the `nameparser.ffi.lib`
Maven property (default: the macOS `.dylib` path above; a `linux-cdylib` profile overrides it
to the `.so` path when the build runs on Linux). Override on the command line if you built the
cdylib somewhere else:

```sh
mvn -f bindings/java/pom.xml test -Dnameparser.ffi.lib=/path/to/libnameparser_ffi.dylib
```

Running a test/benchmark JVM directly (outside Maven) needs the same native-access opt-in
FFM downcalls require:

```sh
java --enable-native-access=ALL-UNNAMED -Dnameparser.ffi.lib=$PWD/target/release/libnameparser_ffi.dylib ...
```

## What's here (Tasks 2-4 of the Phase 3 plan)

- `pom.xml` — standalone Maven module, `--release 22`.
- `src/main/java/org/gbif/nameparser/rust/Ffi.java` — the FFM plumbing only (symbol lookup,
  downcall handles, the ABI-version guard, the confined-arena marshalling helper). No parsing
  logic.
- `src/main/java/org/gbif/nameparser/rust/NameParserRust.java` — `implements NameParser`,
  JSON wire format, `new NameParserRust()`.
- `src/test/java/org/gbif/nameparser/rust/NameParserRustSmokeTest.java` — end-to-end tests
  over the real FFM boundary (no mocking): a subspecies parse, an explicit-authorship parse,
  the virus → `UnparsableNameException` case, and Gson round-trip fidelity for the
  trickier collection-typed fields (`notho`, `warnings`, multi-author lists).
- `src/test/java/org/gbif/nameparser/rust/ParityTest.java` — `NameParserRust` vs
  `NameParserImpl` (the Java 4.2.0 oracle) over all 7 corpora in `../../testdata/` (11,302
  names): 0 diffs, re-proving Phase 2's out-of-process CLI parity result in-process, through
  the FFM boundary and the Gson round trip. Prints a per-corpus + total tally to stdout, and
  up to 20 example diffs (both sides' JSON/exception) to stderr on failure.
- `jmh/` — a separate, standalone JMH module (own `pom.xml`, not part of a reactor with this
  one): `org.gbif.nameparser.rust.jmh.ParseBench` benchmarks `NameParserImpl` (`javaImpl`)
  against `NameParserRust`'s JSON path (`rustJson`), single-name, in-process, over the first
  ~2,000 names of `../../testdata/benchmark-data.txt`. See "Running the JMH benchmark" below.
  Raw results: `jmh/results-jmh.json` (committed alongside the code that produced it).

Not in scope for this module yet (a later task in the same plan, see
`docs/superpowers/plans/2026-07-11-phase3-ffm-binding.md`): the flat fixed-layout struct wire
format + a `WireFormat` selector (Tasks 5-6, which also extend `ParityTest` and `jmh/` to cover
the struct path and its own `rustStruct` benchmark).

## Running the JMH benchmark

The `jmh/` module depends on this module's own `name-parser-rust-binding` artifact from
`~/.m2` (same as `org.gbif:name-parser(-api)`; see "Requirements" above) — install it first,
then build the cdylib and the benchmark's shaded jar, then run it:

```sh
mvn -q -f bindings/java/pom.xml install          # publish name-parser-rust-binding:0.0.0 to ~/.m2
cargo build -p nameparser-ffi --release          # the cdylib the forked benchmark JVMs will dlopen
mvn -q -f bindings/java/jmh/pom.xml package       # target/benchmarks.jar (shaded, Main-Class org.openjdk.jmh.Main)

java --enable-native-access=ALL-UNNAMED -Dnameparser.ffi.lib=$PWD/target/release/libnameparser_ffi.dylib \
     -jar bindings/java/jmh/target/benchmarks.jar -rf json -rff bindings/java/jmh/results-jmh.json
```

The `--enable-native-access`/`-Dnameparser.ffi.lib` flags are given to that `java -jar` command
itself (the JMH host process), not to a JMH `-jvmArgs` option: by default JMH launches each
`@Fork`ed measurement JVM with the same input arguments the host JVM was started with, so both
flags reach the forked JVMs that actually load the cdylib and run `rustJson` without needing to
repeat them. Override `-Dnameparser.ffi.lib=...` the same way as for `mvn test` above if the
cdylib lives somewhere else.
