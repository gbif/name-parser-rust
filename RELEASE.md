# Releasing

Every artifact in this repo shares **one version** and releases **in lockstep**, so a given
version number means the **same underlying Rust engine** everywhere.

**Versioning model** (see [DISTRIBUTION.md](DISTRIBUTION.md) §2.2 for the rationale):

- The Cargo **`[workspace.package]` version** (root `Cargo.toml`) is the *engine version*. The
  core crate, CLI, Python wheel, R package, and the Java FFM binding all carry this same number.
- `org.gbif:name-parser-api` (currently **`5.0.1`**) is the stable Java **contract** — an
  independently versioned **dependency**, *not* part of this version. The Java binding implements it
  but versions with the engine (an impl versioning independently from its api is normal).
- The bindings sit at **`0.x`** while new and gathering real-use feedback. Once stable they
  **graduate to the product's `5.x` line** — a deliberate one-time re-baseline, in lockstep.

| Channel | Artifact | Registry | Trigger | Status |
|---|---|---|---|---|
| Java FFM binding | `org.gbif.nameparser:name-parser-rust` | GBIF Nexus | Jenkins | ✅ `0.2.0` released |
| CLI | `nameparser-cli-<target>` archives | GitHub Releases | `cli-v*` tag | ✅ `0.2.0` released |
| Python | `gbif-name-parser` | PyPI | `py-v*` tag | ✅ `0.2.0` published (setup done); later versions via tag |
| Rust engine | `gbif-name-parser` | crates.io | `crate-v*` tag | ✅ `0.2.0` published (setup done); later versions via tag |
| R | `nameparser` | CRAN | `scripts/build-r-tarball.sh` + manual submission | 🟡 tarball builds & checks clean; awaiting first submission |

---

## 0. One-time setup

Do these once (per registry / per person with release rights).

- **PyPI (Trusted Publishing — no token stored). ✅ DONE.** The GitHub `pypi` and `testpypi`
  environments exist (repo *Settings → Environments*; `pypi` is gated behind a required reviewer),
  the Trusted Publisher is registered (owner `gbif`, repo `name-parser-rust`, workflow
  `python-release.yml`, **environment `pypi`**), and `gbif-name-parser` `0.1.0` is on PyPI — so
  later versions publish token-free from a `py-v*` tag.
  - *Dry-run channel:* the same registration on <https://test.pypi.org> with **environment
    `testpypi`**, driven by the workflow's "Dry run … TestPyPI" input.
- **crates.io (Trusted Publishing — no token stored). ✅ DONE.** The GitHub `crates-io` environment
  exists (gated behind a required reviewer), `gbif-name-parser` `0.1.0` is published, and its Trusted
  Publisher is registered (owner `gbif`, repo `name-parser-rust`, workflow `crate-release.yml`,
  environment `crates-io`) — so later versions publish token-free from the workflow.
  - *How the first publish was bootstrapped* (for the record — unlike PyPI, **crates.io has no
    pending-publisher flow**, so the crate must exist before a Trusted Publisher can be attached):
    create a short-lived scoped API token (crates.io → *Account Settings → API Tokens*, scope
    `publish-new`), run `CARGO_REGISTRY_TOKEN=<token> cargo publish -p gbif-name-parser` once to
    create the crate, add the Trusted Publisher on the now-existing crate's *Settings*, then revoke
    the token.
- **Jenkins (Java).** The Multibranch job already deploys snapshots. Release credentials
  (`gbif-release` / `gbif-snapshot`) live only in the Jenkins-managed `settings.xml` — never in
  the repo.
- **CRAN** *(when enabling — see §2)*: a maintainer email + the manual submission form.

---

## 1. Bump the version (always first)

```sh
scripts/bump-version.sh 0.2.0     # Cargo workspace + pyproject + DESCRIPTION + R crate + pom (X-SNAPSHOT)
                                  # + the pom's <rust.engine.version> (stamped into the JAR manifest)
git diff                          # sanity-check: only the version fields changed
```

Test, then commit and push:

```sh
cargo test --workspace --exclude nameparser-py       # py needs maturin, not plain cargo
cargo build -p nameparser-ffi --release              # the cdylib the Java tests load
mvn -f bindings/java/pom.xml test                    # ParityTest 8,017/0 + smoke
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
- **Release `X`:** run the Jenkins job with **`RELEASE=true`** on `main` (optionally set
  `RELEASE_VERSION` / `DEVELOPMENT_VERSION`). It runs `release:prepare release:perform`, tagging
  `vX` and deploying the release + per-arch classifier JARs. **Run the first release as a dry-run**
  (`-DdryRun=true`) to confirm end to end — see the `Jenkinsfile` release stage.
- **Afterwards:** maven-release-plugin bumps `bindings/java/pom.xml` to the next dev version but
  does **not** touch `bindings/java/jmh/pom.xml`, which is a standalone module outside its reactor.
  Point that module's `name-parser-rust` dependency at the new dev version — the snapshot it named
  before is consumed by the release and 404s on Nexus.

### CLI → GitHub Releases

```sh
git tag cli-v0.2.0 && git push origin cli-v0.2.0
```

`.github/workflows/cli-release.yml` builds `nameparser-cli` natively on 4 targets (linux
x86_64/aarch64, macOS arm64, windows x64) and attaches per-platform archives + SHA-256 to
the `cli-v0.2.0` release. (Intel macOS is not built — GitHub is retiring the `macos-13` Intel
runners; Intel-Mac users build from source.)

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

`gbif-name-parser` **`0.1.0` is published** (that first version was bootstrapped manually — see §0);
subsequent versions publish via Trusted Publishing (OIDC — no stored token). **crates.io must precede
a CRAN release** — the R package vendors the core *from* crates.io.

For any subsequent release, after the §1 version bump — **dry-run first** (recommended):
*Actions → "Publish crate" → Run workflow* → runs `cargo publish --dry-run` (packages + verifies,
never publishes). Then the real release:

```sh
git tag crate-v0.2.0 && git push origin crate-v0.2.0   # NOT crate-v0.1.0 — 0.1.0 is already published
```

`.github/workflows/crate-release.yml` guards the tag against the engine version (root `Cargo.toml`
`[workspace.package]`), authenticates via OIDC (`rust-lang/crates-io-auth-action`), and runs
`cargo publish -p gbif-name-parser`. The `crates-io` GitHub environment is gated behind a required
reviewer, so the run pauses for approval before the (irreversible) upload.
(`nameparser-cli`/`-ffi`/`-py` stay `publish = false` — they are not library crates.) The one-time
crates.io Trusted-Publisher registration is already done (§0).

### R → CRAN

CRAN is source-based and human-reviewed (no auto-publish), and it compiles on machines with **no
network**. `scripts/build-r-tarball.sh` produces a tarball that satisfies both:

```sh
scripts/build-r-tarball.sh --check     # build + R CMD check --as-cran   (~2.1 MB tarball)
```

It works in a staging copy under `target/r-cran/`, never touching `bindings/r`, and it:

1. **Bundles the core crate** into `src/rust/nameparser-core/`, rewriting the binding's
   `path = "../../../../crates/nameparser"` to point at it. This is exactly what `maturin sdist`
   already does for the Python binding — see the script header for why bundling beats switching the
   binding onto the crates.io release.
2. **Vendors the registry crates** (`rextendr::vendor_crates()`, *not* the deprecated
   `vendor_pkgs()`) into `src/rust/vendor.tar.xz`. Both `Makevars` already unpack it and build
   `--offline`; the check log should read `Building for CRAN` → `Using offline vendor tarball`.
3. **Regenerates `LICENSE.note`** from `cargo metadata` — the licence inventory CRAN requires for
   bundled sources — and refreshes `bindings/r/LICENSE.note` so the repo copy stays in step.

The checked-in `bindings/r/src/rust/Cargo.lock` is carried over rather than regenerated, so CRAN
compiles the dependency versions the test suite ran against. If it has drifted, the script stops
and tells you to update it deliberately.

Then submit the tarball via <https://cran.r-project.org/submit.html>. Expect a "New submission"
NOTE — that one is unavoidable on a first submission.

> **No crates.io prerequisite.** Because the core travels inside the tarball, a CRAN release never
> waits on a crates.io release, and the R parity test always validates the engine in *this* working
> tree. (A crates.io *version* dependency would do the opposite: `main` is routinely ahead of the
> last published core, and the R tests would then check a published engine against re-baselined
> goldens.)

Two local tools only affect check completeness, not the package: `checkbashisms` (else `checking
top-level files` WARNs) and a LaTeX install (else the PDF-manual check errors — pass `--no-manual`
to skip). CRAN's own machines have both.

---

## Release checklist (copy per release)

```
[ ] scripts/bump-version.sh X   → review git diff → test → commit → push
[ ] Java:   Jenkins job (RELEASE=true) — or snapshot-only if not cutting a release
[ ] CLI:    git tag cli-vX && git push origin cli-vX     → verify the GitHub release assets
[ ] Python: TestPyPI dry-run → git tag py-vX && git push → verify `pip install gbif-name-parser`
[ ] crates.io: dry-run → git tag crate-vX && git push
[ ] R/CRAN: scripts/build-r-tarball.sh --check → submit the tarball (manual; not tag-triggered)
[ ] Confirm all published artifacts report version X (same engine everywhere)
[ ] Flip the "published at <version>" claims to X: README.md (status banner + binding table),
    DISTRIBUTION.md §2 table. Everything else is bumped by scripts/bump-version.sh at §1.
```
