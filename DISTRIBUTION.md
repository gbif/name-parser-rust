<!-- SPDX-License-Identifier: Apache-2.0 -->
# Distribution & Release

How each of this project's artifacts is (or will be) built, published, and consumed, and
how to wire the whole thing into CI. One Rust core, four delivery channels — there is **no
single "deploy"**, because each binding targets a different package ecosystem.

> **Status honesty (2026-07):** nothing here is published yet and there is no CI in this
> repo (`.github/workflows` is empty, no `Jenkinsfile`). The Java module builds and tests
> but is **dev-only** — it loads the native library from a filesystem path, so its JAR is
> not yet consumable off-checkout. This document is the target design plus the concrete
> gaps to close. The one thing already released through normal channels is the *pure-Java*
> parser (`org.gbif:name-parser*`), which this project will eventually back or replace.

---

## 1. Artifact status at a glance

| Artifact | Path | Channel | Coordinates / name | Status |
|---|---|---|---|---|
| Rust core library | `crates/nameparser` | crates.io | `scientific-name-parser` (lib `nameparser`) | `0.1.0`, unpublished |
| Native CLI | `crates/nameparser-cli` | GitHub Releases | `nameparser-cli` binaries | none built |
| **Java FFM binding** | `bindings/java` | **repository.gbif.org → Maven Central** | `org.gbif.nameparser:name-parser-rust` | dev-only (native lib not bundled) |
| Python binding | `crates/nameparser-py` | PyPI | dist `scientific-name-parser`, import `nameparser` | deferred |
| R binding | `bindings/r` | GitHub (`install_github`), later CRAN | pkg `nameparser` | in progress |

Every binding except the pure-Rust CLI wraps the **`nameparser-ffi` cdylib**
(`crates/nameparser-ffi`) — so packaging that native library correctly is the cross-cutting
problem, addressed in §3.

---

## 2. Per-channel distribution

### 2.1 Rust core + CLI

- **Library** → `cargo publish` to crates.io. The crate is package **`scientific-name-parser`**
  `0.1.0` (Apache-2.0) with lib name `nameparser` (so dependents keep `use nameparser::`); add
  `description`/`repository` and set `publish = true` in `Cargo.toml` before the first publish.
- **CLI** → build `cargo build --release -p nameparser-cli` per target and attach the
  stripped binaries to a **GitHub Release** (`nameparser-cli-<version>-<target>.tar.gz`,
  `.zip` on Windows). No package manager needed; users download and run.

### 2.2 Java FFM binding — the one that matters for GBIF/CoL

The module is a **deliberately standalone** Maven project (see the comment atop
`bindings/java/pom.xml`): it does **not** inherit `org.gbif:motherpom` because motherpom
enforces `maven.compiler.release=17`, and `java.lang.foreign` (FFM/Panama) needs the
compiler release at **22**. Current coordinates:

```
org.gbif.nameparser:name-parser-rust:0.0.0   (packaging: jar)
```

It compiles `org.gbif.nameparser.rust.NameParserRust implements org.gbif.nameparser.api.NameParser`,
so it is a **drop-in** for any code already written against the `NameParser` interface — the
whole point of the FFM binding, and the basis for the Phase-5 backend cutover.

**Two hard prerequisites before it can be a real Maven dependency:**

1. **Bundle the native library** (§3) — today `Ffi.resolveLibPath()` resolves the cdylib from
   `-Dnameparser.ffi.lib` → `$NAMEPARSER_FFI_LIB` → the repo-relative
   `../../target/release/libnameparser_ffi.{dylib,so,dll}`. Great for `mvn test` in-tree,
   useless for a downstream consumer who has no such file.
2. **Java 22+ at runtime.** FFM downcalls are a restricted method; the module already opts in
   with `--enable-native-access=ALL-UNNAMED`, but the *language* level requires JDK 22+. A
   Java 17 service cannot load this JAR until it upgrades (tracked as the Phase-5 cutover
   prerequisite).

**Release-readiness changes to `bindings/java/pom.xml`:**

- Bump `name-parser-api` from `4.2.0-SNAPSHOT` to the released **`4.2.0`** (now available on
  repository.gbif.org / Central), and drop the test-scoped `name-parser:4.2.0-SNAPSHOT` to
  `4.2.0` as well.
- Set a real `<version>` aligned with the parser line it binds (recommend tracking the
  `name-parser` version, i.e. `4.2.0`, so the binding version tells you which model it maps).
- Add `<distributionManagement>` for `repository.gbif.org` (this module cannot get it from
  motherpom). Use motherpom's own release/snapshot repository definitions as the source of
  truth for the exact URLs; resolve dependencies through the read group
  `https://repository.gbif.org/content/groups/gbif`.
- To also sync to **Maven Central**, add the `maven-source-plugin`, `maven-javadoc-plugin`,
  `maven-gpg-plugin`, and the Sonatype/Central publishing plugin that motherpom would
  normally contribute — since this POM opts out of the parent, it must supply them itself.
  If Central is not required (GBIF-internal consumers resolve from Nexus), this can be
  skipped.

### 2.3 Python binding

- Build wheels with **[cibuildwheel](https://cibuildwheel.pypa.io/)** driving
  **maturin** — it produces a matrix of `manylinux` (x86_64 + aarch64), macOS
  (x86_64 + arm64), and Windows wheels, each embedding the compiled Rust extension. Because
  the crate builds an **abi3** wheel, one wheel per platform covers all supported CPython 3.x.
- `twine upload` (or the maturin/PyPI publish action) to **PyPI** under distribution name
  **`scientific-name-parser`** (import stays `nameparser`; matches the Rust crate on crates.io).
- Deferred items: the cibuildwheel CI config and the first PyPI publish.

### 2.4 R binding

- Distribute from **GitHub** first: `remotes::install_github("gbif/name-parser-rust", subdir = "bindings/r")`.
  This compiles the embedded Rust crate on the user's machine (they need a Rust toolchain;
  `SystemRequirements: Cargo` in `DESCRIPTION` declares it).
- **CRAN** later: CRAN requires the Rust dependencies to be **vendored** into the source
  tarball (`cargo vendor` under `src/rust/vendor/`) and an offline, network-free build — a
  separate hardening step, deferred like the Python PyPI publish.

---

## 3. Packaging the native library (the cross-cutting problem)

Three bindings need `libnameparser_ffi.*` present on the target machine. Java is the hard
case because it loads over FFM from a path. Two standard solutions:

- **(a) Bundle-and-extract — recommended.** Ship every platform's cdylib as a classpath
  resource and extract the right one at load time:
  ```
  bindings/java/src/main/resources/native/
    ├── linux-x86-64/libnameparser_ffi.so
    ├── linux-aarch64/libnameparser_ffi.so
    ├── darwin-x86-64/libnameparser_ffi.dylib
    ├── darwin-aarch64/libnameparser_ffi.dylib
    └── windows-x86-64/nameparser_ffi.dll
  ```
  Change `Ffi.resolveLibPath()` to: keep the `-Dnameparser.ffi.lib` / `$NAMEPARSER_FFI_LIB`
  overrides as dev escape hatches, but when unset, compute a classifier from
  `System.getProperty("os.name")` + `os.arch`, copy `native/<classifier>/<libname>` out of the
  JAR to a temp file, and hand *that* path to `SymbolLookup.libraryLookup(...)`. This yields a
  single self-contained fat JAR — the model used by `sqlite-jdbc`, `jansi`, `rocksdbjni`.
  Cost: JAR size grows by (cdylib size × number of platforms), a few MB total.

- **(b) Classifier artifacts** (à la `netty-tcnative`): a thin main JAR plus
  `name-parser-rust-<ver>-linux-x86-64.jar` etc.; the consumer adds the matching
  classifier via `os-maven-plugin`'s `${os.detected.classifier}`. Leaner per-consumer
  downloads, but more artifacts to build and deploy and a heavier consumer POM.

Given GBIF's deployment surface (Linux servers + developer macs), **(a)** is the simpler
choice. Either way, CI must build the cdylib for **every platform in the resource matrix
before the JAR is packaged** — which drives the pipeline shape below.

---

## 4. CI / Jenkins pipeline

Because the native libraries must exist before the JAR and wheels are assembled, the pipeline
is multi-stage with a per-platform build matrix — not a single `mvn deploy`:

```
Stage 1 — native cdylib per platform
    Jenkins matrix over agents { linux-x86_64, linux-aarch64, darwin-arm64, windows-x86_64 }
    running:  cargo build --release -p nameparser-ffi
    (or cargo-zigbuild to cross-compile the Linux/Windows targets on one Linux agent;
     macOS still wants a real mac agent). Stash each .so / .dylib / .dll.

Stage 2 — Java  (needs JDK 22+)
    Unstash all cdylibs into bindings/java/src/main/resources/native/**
    mvn -f bindings/java/pom.xml deploy      → repository.gbif.org

Stage 3 — Python
    cibuildwheel (manylinux + macOS + Windows)  → twine upload  → PyPI

Stage 4 — R
    R CMD build bindings/r ; R CMD check       → GitHub release asset

Stage 5 — CLI
    cargo build --release -p nameparser-cli per target  → GitHub release
```

**GBIF-specific notes**

- The Java stage is well-trodden ground: the sibling `github.com/gbif/name-parser` already
  releases through GBIF Jenkins → `repository.gbif.org` via `org.gbif:motherpom:61` and the
  `maven-release-plugin`. Reuse that infrastructure and the branch/tag conventions.
- This module can't inherit motherpom (the `release=17` enforcer), so it carries its own
  `<distributionManagement>` and — if Central sync is wanted — its own
  sources/javadoc/GPG/publish plugins (§2.2).
- Follow the [GBIF engineering handbook](https://informatics-docs.gbif.org/engineering-handbook)
  for versioning (semver), issue/release management, and documentation conventions — it
  applies to all `github.com/gbif/` repos, including this one.
- Trigger model: a multibranch pipeline that builds + tests on PRs, deploys `-SNAPSHOT` on
  `main`, and cuts a release on a version tag (`maven-release-plugin` for the Java module).

**Incremental path (do this first).** Get **Stage 2 — the Java artifact to
`repository.gbif.org`** working before anything else. It is the one artifact GBIF/CoL
consumers resolve through Maven, and it reuses infrastructure you already operate. The other
three channels (PyPI, GitHub/CRAN, crates.io) are independent of Nexus and can follow one at
a time.

---

## 5. Consuming the parser

### Java / Maven

**The released pure-Java parser (available today** on repository.gbif.org and Maven Central**):**

```xml
<!-- interface + ParsedName model + enums only -->
<dependency>
  <groupId>org.gbif</groupId>
  <artifactId>name-parser-api</artifactId>
  <version>4.2.0</version>
</dependency>

<!-- or the NameParserGBIF implementation -->
<dependency>
  <groupId>org.gbif</groupId>
  <artifactId>name-parser</artifactId>
  <version>4.2.0</version>
</dependency>
```

If your build does not already resolve from GBIF's Nexus, add:

```xml
<repositories>
  <repository>
    <id>gbif-all</id>
    <url>https://repository.gbif.org/content/groups/gbif</url>
  </repository>
</repositories>
```

**The Rust-backed FFM binding (once released** — coordinates as in `bindings/java/pom.xml`, a
drop-in `NameParser` on **JDK 22+**, native libs bundled per §3**):**

```xml
<dependency>
  <groupId>org.gbif.nameparser</groupId>
  <artifactId>name-parser-rust</artifactId>
  <version>4.2.0</version>
</dependency>
```

```java
// same interface as the pure-Java parser — swap the implementation, keep the callers.
NameParser parser = new org.gbif.nameparser.rust.NameParserRust();
ParsedName pn = parser.parse("Abies alba Mill.", Rank.SPECIES);
```

### Python

```bash
pip install scientific-name-parser   # once published to PyPI
```
```python
import nameparser
pn = nameparser.parse("Abies alba Mill.")
```

### R

```r
# from GitHub (compiles the Rust crate; needs a Rust toolchain):
remotes::install_github("gbif/name-parser-rust", subdir = "bindings/r")
library(nameparser)
parse_names("Abies alba Mill.")
```

### Rust CLI

```bash
# once binaries are on GitHub Releases:
curl -L .../nameparser-cli-<ver>-<target>.tar.gz | tar xz
./nameparser-cli parse --input=names.txt
```

---

## 6. Open items

- [ ] Java: implement native-lib bundling in `Ffi.java` + `src/main/resources/native/**` (§3a);
      add `<distributionManagement>` (+ Central plugins if syncing); bump deps to `4.2.0`; set version.
- [ ] `Jenkinsfile`: native build matrix (Stage 1) + Java deploy (Stage 2); then Stages 3–5.
- [ ] Python: cibuildwheel config + first PyPI publish.
- [ ] R: `cargo vendor` for a CRAN-ready, network-free source build.
- [ ] Rust: add `description`/`repository` + set `publish = true` (name/version/license already
      `scientific-name-parser` / `0.1.0` / Apache-2.0); `cargo publish`; CLI release binaries.
- [ ] Decide whether the Rust FFM binding ships **alongside** the pure-Java parser or eventually
      **replaces** it behind the same `org.gbif:name-parser` coordinates (Phase-5 cutover decision).
