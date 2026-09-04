# CRAN submission comments — nameparser

Build the tarball this file accompanies with `scripts/build-r-tarball.sh --check`
(repo root), then submit at <https://cran.r-project.org/submit.html>.

## Test environments

- macOS 26 (arm64), R 4.6.1, rustc 1.97.0 — `R CMD check --as-cran`

## R CMD check results

Two notes, both expected:

* **New submission.** This is the first release of `nameparser`.

* **`installed size is ~5.8Mb`, `libs ~5.7Mb`.** The package compiles and statically links a Rust
  scientific-name parser. The size is compiled code, not debug information — an explicit
  `strip -S` recovers only ~0.1Mb, and the object is ~2.4Mb of `__text` plus ~0.9Mb of read-only
  tables (the parser embeds a large set of compiled regular expressions and reference
  vocabularies: rank markers, author particles, culture-collection codes, blacklisted epithets).
  We are not aware of a way to reduce this materially without dropping parser functionality.

No ERRORs or WARNINGs.

## Rust / `SystemRequirements`

Per the CRAN policy on packages using Rust:

* `SystemRequirements: Cargo (Rust's package manager), rustc` is declared, and
  `tools/msrv.R` (run from `configure`) verifies both are present and reports their versions
  during configuration.

* **The build is fully offline.** All third-party crates are vendored into
  `src/rust/vendor.tar.xz`; `src/Makevars.in` and `src/Makevars.win.in` unpack it, point
  `CARGO_HOME` at it, and invoke `cargo build --offline`. Nothing is fetched from the network at
  build time. The parser core itself is bundled as ordinary package sources under
  `src/rust/nameparser-core/`.

* **`LICENSE.note`** enumerates the name, repository, authors and licence of every bundled Rust
  crate. It is generated from `cargo metadata` (`rextendr::write_license_note()`), so it cannot
  drift from what is actually vendored.

* The build respects `-j 2` on CRAN (`--offline -j 2`) and uses `--release`.

## Upstream

`nameparser` is the R binding to the Rust port of the GBIF scientific name parser. Sources for
the whole project, including the Java, Python and command-line bindings that share this engine,
are at <https://github.com/gbif/name-parser-rust>.
