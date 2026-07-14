<!-- SPDX-License-Identifier: Apache-2.0 -->
# Distribution & Release

How each of this project's artifacts is (or will be) built, published, and consumed, and
how to wire the whole thing into CI. One Rust core, four delivery channels — there is **no
single "deploy"**, because each binding targets a different package ecosystem.

> **Status (2026-07):** CI is in place. `.github/workflows/` builds + tests the engine and every
> binding (`ci.yml`) and publishes on tags (`crate-release.yml` → crates.io, `cli-release.yml` →
> GitHub Releases, `python-release.yml` → PyPI); the `Jenkinsfile` deploys the Java FFM binding to
> GBIF Nexus. The Java module bundles the native library into a **self-contained JAR** (§3), so it
> is consumable off-checkout. The Java `0.1.0-SNAPSHOT` already auto-deploys to GBIF Nexus on every
> push to `main`; the public registries (crates.io / PyPI / CRAN) are wired but awaiting their first
> release tag (see [`RELEASE.md`](RELEASE.md)). The pre-existing *pure-Java* parser
> (`org.gbif:name-parser*`) remains available and is what this project will eventually back or replace.

---

## 1. Artifact status at a glance

| Artifact | Path | Channel | Coordinates / name | Status |
|---|---|---|---|---|
| Rust core library | `crates/nameparser` | crates.io | `gbif-name-parser` (lib `nameparser`) | `0.1.0`, unpublished |
| Native CLI | `crates/nameparser-cli` | GitHub Releases | `nameparser-cli` binaries | none built |
| **Java FFM binding** | `bindings/java` | **repository.gbif.org** (Jenkins) | `org.gbif.nameparser:name-parser-rust` (+ per-arch classifier JARs) | **LIVE** — auto-deployed `0.1.0-SNAPSHOT` |
| Python binding | `crates/nameparser-py` | PyPI | dist `gbif-name-parser`, import `nameparser` | CI ready (needs PyPI trusted-publisher setup) |
| R binding | `bindings/r` | GitHub (`install_github`), later CRAN | pkg `nameparser` | in progress |

Every binding except the pure-Rust CLI wraps the **`nameparser-ffi` cdylib**
(`crates/nameparser-ffi`) — so packaging that native library correctly is the cross-cutting
problem, addressed in §3.

---

## 2. Per-channel distribution

### 2.1 Rust core + CLI

- **Library** → `cargo publish` to crates.io. The crate is package **`gbif-name-parser`**
  `0.1.0` (Apache-2.0) with lib name `nameparser` (so dependents keep `use nameparser::`). The
  manifest already carries `description`/`repository`/`keywords`/`categories` and is publishable
  (`cargo publish --dry-run -p gbif-name-parser` passes); a `crate-v*` tag publishes it via
  `crate-release.yml` (OIDC Trusted Publishing — no stored token).
- **CLI** → build `cargo build --release -p nameparser-cli` per target and attach the
  stripped binaries to a **GitHub Release** (`nameparser-cli-<version>-<target>.tar.gz`,
  `.zip` on Windows). No package manager needed; users download and run.

### 2.2 Java FFM binding — the one that matters for GBIF/CoL

The module is a **deliberately standalone** Maven project (see the comment atop
`bindings/java/pom.xml`): it does **not** inherit `org.gbif:motherpom` because motherpom
enforces `maven.compiler.release=17`, and `java.lang.foreign` (FFM/Panama) needs the
compiler release at **22**. Current coordinates:

```
org.gbif.nameparser:name-parser-rust:0.1.0-SNAPSHOT   (packaging: jar)
```

It compiles `org.gbif.nameparser.rust.NameParserRust implements org.gbif.nameparser.api.NameParser`,
so it is a **drop-in** for any code already written against the `NameParser` interface — the
whole point of the FFM binding, and the basis for the Phase-5 backend cutover.

**Prerequisites to be a real Maven dependency:**

1. **Bundle the native library** (§3) — ✅ DONE. `Ffi.resolveLibPath()` now extracts the cdylib
   from a JAR resource (`native/${os.detected.classifier}/`) when no `-Dnameparser.ffi.lib` /
   `$NAMEPARSER_FFI_LIB` override is set, so `mvn package` produces a self-contained JAR (verified
   loading the bundled lib with no override). The `-D`/env dev escape hatches still work.
2. **Java 22+ at runtime.** FFM downcalls are a restricted method; the module already opts in
   with `--enable-native-access=ALL-UNNAMED`, but the *language* level requires JDK 22+. A
   Java 17 service cannot load this JAR until it upgrades (tracked as the Phase-5 cutover
   prerequisite).

**Release-readiness — done, and the versioning model:**

- ✅ `name-parser-api` pinned to the released **`5.0.0`**; a GBIF Nexus `<repositories>` block
  resolves it (this standalone POM has no motherpom to supply it). The Java `name-parser`
  reference-impl / oracle was removed at 5.0.0 (api-only), so it is no longer a test dependency.
  The api is an independently versioned **dependency** — the stable contract — **not** this
  module's own version.
- ✅ **Version = `0.1.0-SNAPSHOT`** — the Java FFM binding **shares the Rust engine's version**
  (the Cargo `[workspace.package]` version at the repo root), released in lockstep with the
  CLI/Python/R bindings: **one version across every binding ⇒ the same engine**. It is *not* tied
  to the `name-parser-api` version it implements (an implementation versioning independently from
  its api is normal — cf. logback-classic vs slf4j-api). The bindings sit at **`0.x`** while new
  and gathering real-use feedback, and will **graduate to the product's `5.x` line once stable**
  (a deliberate one-time re-baseline, in lockstep). The JAR manifest also stamps
  `Rust-Engine-Version` + the git SHA, so the exact engine is always readable from the artifact.
- ✅ `<distributionManagement>` for `repository.gbif.org` (interim, until the reactor move). The
  `<server>` credential ids in the deployer's `~/.m2/settings.xml` must match `gbif-release` /
  `gbif-snapshot` — adjust to your actual ids.
- Remaining before an actual `mvn deploy`: confirm those credential ids; optionally add the
  `maven-source-plugin` / `maven-javadoc-plugin` / `maven-gpg-plugin` + Sonatype/Central
  publishing plugin **if** Maven Central sync is wanted (GBIF-internal consumers resolve from
  Nexus, so this is skippable).

### 2.3 Python binding

- **CI: `.github/workflows/python-release.yml`** (uses **[PyO3/maturin-action](https://github.com/PyO3/maturin-action)**)
  builds `manylinux` (x86_64 + aarch64), macOS (x86_64 + arm64), and Windows wheels plus an sdist,
  each embedding the compiled Rust extension. Because the crate builds an **abi3** (`abi3-py39`)
  wheel, one wheel per platform covers CPython 3.9+ — no per-version matrix.
- Publishes to **PyPI** under distribution name **`gbif-name-parser`** (import stays `nameparser`;
  matches the Rust crate on crates.io) via **Trusted Publishing (OIDC)** — no PyPI token is stored
  anywhere. A tag-vs-`pyproject.toml`-version guard blocks a mismatched (and irreversible) upload.
- Trigger: push a `py-v<version>` tag. **One-time setup** before the first release: register the
  PyPI Trusted Publisher (owner `gbif`, repo `name-parser-rust`, workflow `python-release.yml`,
  environment `pypi`) and create the `pypi` GitHub environment. See the workflow header for details.

### 2.4 R binding

- Distribute from **GitHub** first: `remotes::install_github("gbif/name-parser-rust", subdir = "bindings/r")`.
  This compiles the embedded Rust crate on the user's machine (they need a Rust toolchain;
  `SystemRequirements: Cargo` in `DESCRIPTION` declares it).
- **CRAN** later: CRAN requires the Rust dependencies to be **vendored** into the source
  tarball (`cargo vendor` under `src/rust/vendor/`) and an offline, network-free build — a
  separate hardening step, deferred like the Python PyPI publish.

---

## 3. Packaging the native library (the cross-cutting problem)

Three bindings need `libnameparser_ffi.*` on the target machine. Java is the hard case because it
loads over FFM from a path. `Ffi.resolveLibPath()` extracts the cdylib from a **classpath
resource** `native/${os.detected.classifier}/<libname>` to a temp file and hands *that* to
`SymbolLookup.libraryLookup(...)` — with the `-Dnameparser.ffi.lib` / `$NAMEPARSER_FFI_LIB`
overrides and a repo-relative dev path as fallbacks. That resource works whether it sits in the
main JAR or (as we ship it) a per-platform classifier JAR on the consumer's classpath.

**✅ IMPLEMENTED — classifier JARs (netty-tcnative style), for small JARs.** The module publishes:

- a **thin main JAR** `name-parser-rust-<ver>.jar` (~22 KB, Java classes only) whose manifest is
  stamped with `Implementation-Version`, `Rust-Engine-Version` (the `gbif-name-parser` crate
  version), and `Rust-Engine-Git-Revision` (the exact source SHA — see the versioning note in §2.2);
- one **per-platform classifier JAR** `name-parser-rust-<ver>-<classifier>.jar` (~1–1.6 MB, just
  that platform's cdylib under `native/<classifier>/`) for `linux-x86_64`, `linux-aarch_64`,
  `osx-x86_64`, `osx-aarch_64` (Apple Silicon), and `windows-x86_64`.

A consumer depends on the main JAR + their platform's classifier (via `os-maven-plugin`'s
`${os.detected.classifier}`), so nobody downloads five architectures they won't use (§5).

The cdylibs are **cross-compiled with `cargo-zigbuild`** (`ci/build-cdylib.sh`): one Linux CI agent
builds every target — zig cross-compiles cleanly to macOS/Windows because `nameparser-ffi` is pure
Rust (no C deps). The script bootstraps rustup + zig (self-contained tarball, no brew needed) and
stages each cdylib into `native-staging/<classifier>/`; the pom's `maven-jar-plugin` executions
package them with `skipIfEmpty`, so an agent that manages only its host platform still deploys a
valid subset. Verified end-to-end locally: 6 JARs, correct architectures (incl. Mach-O arm64), and
a stamped manifest.

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
    maturin-action (manylinux + macOS + Windows wheels + sdist)  → gh-action-pypi-publish (OIDC)  → PyPI

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

**The Rust-backed FFM binding** — a drop-in `NameParser` on **JDK 22+**. Add the thin main JAR
plus your platform's native classifier JAR (via `os-maven-plugin`, §3). `0.1.0-SNAPSHOT` deploys
on every push to `main`; `0.1.0` once released:

```xml
<build><extensions>
  <!-- sets ${os.detected.classifier}: linux-x86_64, osx-aarch_64, windows-x86_64, … -->
  <extension>
    <groupId>kr.motd.maven</groupId><artifactId>os-maven-plugin</artifactId><version>1.7.1</version>
  </extension>
</extensions></build>

<dependencies>
  <dependency>                               <!-- thin main JAR: Java + FFM loader -->
    <groupId>org.gbif.nameparser</groupId>
    <artifactId>name-parser-rust</artifactId>
    <version>0.1.0</version>
  </dependency>
  <dependency>                               <!-- your platform's native cdylib -->
    <groupId>org.gbif.nameparser</groupId>
    <artifactId>name-parser-rust</artifactId>
    <version>0.1.0</version>
    <classifier>${os.detected.classifier}</classifier>
  </dependency>
</dependencies>
```

```java
// same interface as the pure-Java parser — swap the implementation, keep the callers.
NameParser parser = new org.gbif.nameparser.rust.NameParserRust();
ParsedName pn = parser.parse("Abies alba Mill.", Rank.SPECIES);
```

### Python

```bash
pip install gbif-name-parser   # once published to PyPI
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

- [x] Java native-lib packaging — DONE. Per-arch **classifier JARs** (thin main + `linux-x86_64` /
      `linux-aarch_64` / `osx-x86_64` / `osx-aarch_64` / `windows-x86_64`), cross-compiled via
      cargo-zigbuild; main-JAR manifest stamped with version + `Rust-Engine-Version`/`-Git-Revision`.
- [x] Java deploy + Jenkins — **LIVE**. A Multibranch pipeline auto-deploys `0.1.0-SNAPSHOT` to
      `repository.gbif.org` on every push to `main` (parity 11,302/0 in CI). The `release:perform` stage is
      now complete — `<scm>` is in place and the classifier JARs read `${native.staging.dir}`, which
      the Jenkinsfile points at the outer workspace's staged cdylibs — pending a first dry-run.
      Optionally add the Central sources/javadoc/GPG plugins for Maven Central sync.
- [x] Rust CLI — GitHub Releases workflow (`.github/workflows/cli-release.yml`): a `cli-v*` tag
      builds + attaches per-platform archives (5 targets) + sha256. Pending the first tag.
- [x] Python — PyPI workflow (`.github/workflows/python-release.yml`): a `py-v*` tag builds all
      wheels + sdist and publishes via Trusted Publishing. Pending the one-time PyPI trusted-publisher
      + `pypi` environment setup, then the first tag.
- [ ] R: `cargo vendor` for a CRAN-ready, network-free source build.
- [x] Wire-format decision — RESOLVED: struct-only. The flat-struct wire (~13% faster than the
      JSON/Gson path in the Phase-3 JMH A/B) is the single format; the JSON path was dropped at ABI
      version 2, which also removed the `gson` runtime dependency (now test-scope only).
- [ ] Phase 5: backend cutover (swap `NameParserRust` in behind the `NameParser` interface; Java 22+).
- [x] Rust engine + CLI — DONE. The crate carries full metadata and is publishable via
      `crate-release.yml` (a `crate-v*` tag → crates.io, OIDC Trusted Publishing); the CLI ships
      per-platform archives via `cli-release.yml` (a `cli-v*` tag → GitHub Releases). Pending only
      the first release tags.
- [ ] Decide whether the Rust FFM binding ships **alongside** the pure-Java parser or eventually
      **replaces** it behind the same `org.gbif:name-parser` coordinates (Phase-5 cutover decision).
