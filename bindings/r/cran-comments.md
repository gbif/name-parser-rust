## Test environments

* macOS 26 (arm64), R 4.6.1, rustc 1.97.0 -- R CMD check --as-cran
* Ubuntu 24.04 (x86_64), R release, rustc stable -- R CMD check
* Windows Server 2022 (x86_64), R release, rustc stable -- R CMD check
* win-builder (R-devel)

## R CMD check results

There were no ERRORs or WARNINGs.

One NOTE, expected for a first submission:

* New submission. This is the first release of nameparser.

  The same NOTE reports "Possibly misspelled words in DESCRIPTION:
  nomenclatural". This is correct usage: a nomenclatural code (ICZN, ICN,
  ICNP, ICTV) is the body of rules under which a scientific name is
  governed, and it is one of the fields the parser returns.

Depending on the platform, a second NOTE may appear for the installed size
("installed size is ~5.8Mb", "libs ~5.7Mb"). The package compiles and
statically links a scientific name parser written in Rust. That size is
compiled code, not debug information: an explicit "strip -S" recovers only
about 0.1Mb, and the object is roughly 2.4Mb of machine code plus 0.9Mb of
read-only tables (the parser embeds a large set of compiled regular
expressions and reference vocabularies -- rank markers, author particles,
culture-collection codes, blacklisted epithets). We are not aware of a way
to reduce this materially without removing parser functionality.

## Rust

Per the CRAN policy on packages using Rust:

* SystemRequirements declares "Cargo (Rust's package manager), rustc", and
  tools/msrv.R (run from configure) verifies both are present and reports
  their versions during configuration.

* The build is fully offline. All third-party crates are vendored into
  src/rust/vendor.tar.xz; src/Makevars.in and src/Makevars.win.in unpack it,
  point CARGO_HOME at it, and invoke "cargo build --offline". Nothing is
  fetched from the network at build time. The parser core itself is bundled
  as ordinary package sources under src/rust/nameparser-core/.

* LICENSE.note enumerates the name, repository, authors and licence of every
  bundled Rust crate. It is generated from "cargo metadata", so it cannot
  drift from what is actually vendored.

* The build respects CRAN's parallelism limit ("--offline -j 2") and uses
  the release profile.

## Upstream

nameparser is the R binding to the Rust port of the scientific name parser
developed by the Global Biodiversity Information Facility. Sources for the
whole project, including the Java, Python and command-line bindings that
share this engine, are at <https://github.com/gbif/name-parser-rust>.

<!--
Maintainer notes (not part of the submission comment):
build this tarball with `scripts/build-r-tarball.sh --check` from the repo
root, then submit at <https://cran.r-project.org/submit.html>. Paste
everything above the HTML comment into the "Optional comment" box as plain
text. See RELEASE.md section 2, "R -> CRAN".
-->
