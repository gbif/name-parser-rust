# Releasing

Every artifact in this repo shares **one version** and releases **in lockstep**, so a given
version number means the **same underlying Rust engine** everywhere.

**Versioning model** (see [DISTRIBUTION.md](DISTRIBUTION.md) §2.2 for the rationale):

- The Cargo **`[workspace.package]` version** (root `Cargo.toml`) is the *engine version*. The
  core crate, CLI, Python wheel, R package, and the Java FFM binding all carry this same number.
- `org.gbif:name-parser-api` (currently **4.x**) is the stable Java **contract** — an independently
  versioned **dependency**, *not* part of this version. The Java binding implements it but versions
  with the engine (an impl versioning independently from its api is normal).
- The bindings sit at **`0.x`** while new and gathering real-use feedback. Once stable they
  **graduate to the product's `4.x` line** — a deliberate one-time re-baseline, in lockstep.

| Channel | Artifact | Registry | Trigger | Status |
|---|---|---|---|---|
| Java FFM binding | `org.gbif.nameparser:name-parser-rust` | GBIF Nexus | Jenkins | ✅ ready |
| CLI | `nameparser-cli-<target>` archives | GitHub Releases | `cli-v*` tag | ✅ ready |
| Python | `gbif-name-parser` | PyPI | `py-v*` tag | ✅ ready (one-time PyPI setup) |
| Rust engine | `gbif-name-parser` | crates.io | `cargo publish` | ⚠️ not yet wired |
| R | `nameparser` | CRAN | manual submission | ⚠️ not yet wired |

---

## 0. One-time setup

Do these once (per registry / per person with release rights).

- **PyPI (Trusted Publishing — no token stored).** On <https://pypi.org> add a Trusted Publisher
  for project `gbif-name-parser`: owner `gbif`, repo `name-parser-rust`, workflow
  `python-release.yml`, **environment `pypi`**. Create a **`pypi`** environment under the repo's
  GitHub *Settings → Environments* (optionally require a reviewer to gate each release).
  - *Dry-run channel (recommended before the first real publish):* do the same on
    <https://test.pypi.org> with **environment `testpypi`**, and create a `testpypi` GitHub
    environment.
- **Jenkins (Java).** The Multibranch job already deploys snapshots. Release credentials
  (`gbif-release` / `gbif-snapshot`) live only in the Jenkins-managed `settings.xml` — never in
  the repo.
- **crates.io** *(when enabling — see §2)*: a crates.io API token (`cargo login`), and flip the
  core crate to publishable.
- **CRAN** *(when enabling — see §2)*: a maintainer email + the manual submission form.

---

## 1. Bump the version (always first)

```sh
scripts/bump-version.sh 0.2.0     # sets Cargo workspace + pyproject + DESCRIPTION + pom (X-SNAPSHOT)
git diff                          # sanity-check: only the version fields changed
```

Test, then commit and push:

```sh
cargo test --workspace --exclude nameparser-py       # py needs maturin, not plain cargo
cargo build -p nameparser-ffi --release              # the cdylib the Java tests load
mvn -f bindings/java/pom.xml test                    # parity 11,302/0 + smoke
git add -A && git commit -m "Release 0.2.0" && git push
```

> The Java pom carries `0.2.0-**SNAPSHOT**` (Maven dev-version convention); the Jenkins release job
> strips `-SNAPSHOT` to `0.2.0` at release time, so it lands on the same number as the others.

---

## 2. Release each channel

The channels are independent — release any subset. For a full coordinated release, do them all at
the bumped version.

### Java → GBIF Nexus (Jenkins)

- **Snapshots:** every push to `main` auto-deploys `X-SNAPSHOT` (parity runs in CI). Nothing to do.
- **Release `X`:** run the Jenkins job with **`RELEASE=true`** on `master` (optionally set
  `RELEASE_VERSION` / `DEVELOPMENT_VERSION`). It runs `release:prepare release:perform`, tagging
  `vX` and deploying the release + per-arch classifier JARs. **Run the first release as a dry-run**
  (`-DdryRun=true`) to confirm end to end — see the `Jenkinsfile` release stage.

### CLI → GitHub Releases

```sh
git tag cli-v0.2.0 && git push origin cli-v0.2.0
```

`.github/workflows/cli-release.yml` builds `nameparser-cli` natively on 5 targets (linux
x86_64/aarch64, macOS x86_64/aarch64, windows x64) and attaches per-platform archives + SHA-256 to
the `cli-v0.2.0` release.

### Python → PyPI

**Dry-run first** (recommended): *Actions → "Publish Python" → Run workflow* from `main` with the
*"Dry run … TestPyPI"* box checked → builds all wheels + sdist and publishes to TestPyPI. Verify:

```sh
pip install -i https://test.pypi.org/simple/ gbif-name-parser
```

Then the real release:

```sh
git tag py-v0.2.0 && git push origin py-v0.2.0
```

`.github/workflows/python-release.yml` builds the wheels (abi3 → one per platform, CPython 3.9+) +
sdist and publishes to PyPI via Trusted Publishing. A guard fails the run if the tag doesn't match
`pyproject.toml`'s version (PyPI uploads are irreversible). Result: `pip install gbif-name-parser`.

### Rust engine → crates.io

The core crate `gbif-name-parser` is crates.io-ready (`cargo publish --dry-run -p gbif-name-parser`
passes). **This must precede a CRAN release** — the R package vendors the core *from* crates.io.

1. `cargo login <token>` once (a crates.io API token).
2. `cargo publish -p gbif-name-parser` at the bumped version. (`nameparser-cli`/`-ffi`/`-py` stay
   `publish = false` — they are not library crates.)

### R → CRAN — *needs crates.io first*

CRAN is source-based and human-reviewed (no auto-publish). **Prerequisite: the core on crates.io**
(above) — the R package isn't self-contained until it depends on `gbif-name-parser` *by version*
and can vendor it (`cargo vendor` skips local path deps). Then:

1. Point `bindings/r/src/rust/Cargo.toml`'s `nameparser_core` dependency at the crates.io version.
2. `Rscript -e 'rextendr::vendor_pkgs("bindings/r")'` → bundles the core + all deps into
   `src/rust/vendor.tar.xz` (the `Makevars` already builds offline from it).
3. Add the vendored-crate license inventory (`inst/AUTHORS`) CRAN requires for bundled sources.
4. `R CMD check --as-cran` until clean, then submit the source tarball via
   <https://cran.r-project.org/submit.html>.

---

## Release checklist (copy per release)

```
[ ] scripts/bump-version.sh X   → review git diff → test → commit → push
[ ] Java:   Jenkins job (RELEASE=true) — or snapshot-only if not cutting a release
[ ] CLI:    git tag cli-vX && git push origin cli-vX     → verify the GitHub release assets
[ ] Python: TestPyPI dry-run → git tag py-vX && git push → verify `pip install gbif-name-parser`
[ ] crates.io / CRAN: when wired (see §2)
[ ] Confirm all published artifacts report version X (same engine everywhere)
```
