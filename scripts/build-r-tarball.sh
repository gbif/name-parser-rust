#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Build the CRAN source tarball for the R binding (`nameparser`).
#
#   scripts/build-r-tarball.sh [--out DIR] [--check] [--keep-stage]
#
# CRAN is the only channel that ships the parser as *source* and compiles it on machines with
# no network. That imposes two requirements the in-repo package does not meet on its own:
#
#   1. The core crate must travel *inside* the tarball. In the repo, `bindings/r` reaches the
#      core with `path = "../../../../crates/nameparser"`, which points outside the R package —
#      fine for a checkout, meaningless once the package is extracted somewhere else.
#   2. Every third-party crate must travel with it too, because `cargo` cannot reach crates.io
#      during a CRAN build.
#
# This script does both, in a staging copy, leaving the checked-in package untouched:
#
#   bindings/r/                          crates/nameparser/
#        |                                     |
#        +---------------> stage <-------------+
#                            |
#                  rewrite path dep to the bundled core
#                  vendor the registry crates -> src/rust/vendor.tar.xz
#                  regenerate LICENSE.note from `cargo metadata`
#                            |
#                        R CMD build -> nameparser_<version>.tar.gz
#
# (1) is exactly what `maturin sdist` already does for the Python binding: it copies
# `crates/nameparser/**` into the sdist and keeps the path dependency, rather than switching the
# binding onto the crates.io release. Doing the same here keeps every binding in this repo
# building the core *in this working tree* — so `bindings/r`'s parity test validates the engine
# you are about to ship, not the last one that happened to be published — and it means a CRAN
# release never has to wait on a crates.io release.
#
# (2) is R-specific and handled by `rextendr::vendor_crates()`, which `src/Makevars.in` and
# `src/Makevars.win.in` already know how to build from (they unpack `vendor.tar.xz` and point
# CARGO_HOME at it). Note it is `vendor_crates()`, not the `vendor_pkgs()` alias — deprecated
# since rextendr 0.4.0.
#
# See RELEASE.md ("R -> CRAN") for where this sits in the release flow.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

out="$root/target/r-cran"
run_check=0
keep_stage=0

while [ $# -gt 0 ]; do
  case "$1" in
    --out)        out="$2"; shift 2 ;;
    --check)      run_check=1; shift ;;
    --keep-stage) keep_stage=1; shift ;;
    -h|--help)    sed -n '3,6p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *)            echo "error: unknown argument '$1'" >&2; exit 2 ;;
  esac
done

pkg="$root/bindings/r"
core="$root/crates/nameparser"
version="$(awk '/^Version:/ {print $2; exit}' "$pkg/DESCRIPTION")"
stage="$out/nameparser"

if [ -z "$version" ]; then
  echo "error: no Version: field in bindings/r/DESCRIPTION" >&2
  exit 1
fi

for cmd in cargo Rscript R tar; do
  command -v "$cmd" >/dev/null 2>&1 || { echo "error: '$cmd' not found on PATH" >&2; exit 1; }
done
Rscript -e 'if (!requireNamespace("rextendr", quietly = TRUE)) quit(status = 1)' \
  || { echo "error: the 'rextendr' R package is required (install.packages(\"rextendr\"))" >&2; exit 1; }

echo "==> Building nameparser $version"
echo "    out:   $out"
echo "    stage: $stage"

# ---------------------------------------------------------------------------------------------
# 1. Stage the R package.
# ---------------------------------------------------------------------------------------------
rm -rf "$stage"
mkdir -p "$stage"
cp -R "$pkg/." "$stage/"

# Build leftovers a checkout accumulates. `src/rust/vendor` and `src/Makevars*` are regenerated
# below / by `configure`; the rest are compiler output. (`.Rbuildignore` also lists most of these,
# but dropping them here keeps the staged tree honest and the tarball reproducible.)
rm -rf "$stage/src/rust/target" "$stage/src/rust/vendor" "$stage/src/.cargo" \
       "$stage/.Rproj.user" "$stage/src/Makevars" "$stage/src/Makevars.win"
find "$stage/src" -maxdepth 1 \( -name '*.o' -o -name '*.so' -o -name '*.dll' \) -delete

# ---------------------------------------------------------------------------------------------
# 2. Bundle the core crate.
# ---------------------------------------------------------------------------------------------
# `tests/` and `benches/` are deliberately left behind: they are half a megabyte, they read
# fixtures from `testdata/` at the repo root (absent from the tarball), and CRAN never runs them.
# The Rust unit tests that live *inside* `src/` come along and still compile.
echo "==> Bundling the core crate into src/rust/nameparser-core"
mkdir -p "$stage/src/rust/nameparser-core"
cp -R "$core/src" "$core/resources" "$stage/src/rust/nameparser-core/"
cp "$core/Cargo.toml" "$core/README.md" "$core/LICENSE" "$stage/src/rust/nameparser-core/"

core_manifest="$stage/src/rust/nameparser-core/Cargo.toml"

# The core inherits its version from the repo's root workspace (`version.workspace = true`).
# That root does not exist in the tarball, and the R binding's own manifest is the workspace root
# there, so pin the literal version instead. It is the same number by construction:
# scripts/bump-version.sh sets the workspace version and DESCRIPTION together.
perl -i -pe 's/^version\.workspace = true$/version = "'"$version"'"/' "$core_manifest"
grep -q "^version = \"$version\"\$" "$core_manifest" \
  || { echo "error: failed to pin the core crate version in $core_manifest" >&2; exit 1; }

# Drop `[dev-dependencies]` and `[[bench]]`. Once the core sits inside `src/rust/` it becomes a
# member of the binding's workspace, so cargo would resolve — and `cargo vendor` would bundle —
# its dev-dependency tree (criterion and ~40 transitive crates) into a tarball that never runs a
# benchmark. `[[bench]]` goes too: its `benches/parse.rs` was not copied.
perl -i -ne 'if (/^\[/) { $skip = /^\[dev-dependencies\]/ || /^\[\[bench\]\]/ } print unless $skip' \
  "$core_manifest"

# ---------------------------------------------------------------------------------------------
# 3. Point the binding at the bundled core.
# ---------------------------------------------------------------------------------------------
# Only the `path` changes. The `package = "gbif-name-parser"` rename and the `nameparser_core`
# alias stay exactly as they are — the alias is load-bearing for the `[[bin]] document` target
# (see the comment above the dependency in the checked-in manifest).
binding_manifest="$stage/src/rust/Cargo.toml"
perl -i -pe 's{path = "\.\./\.\./\.\./\.\./crates/nameparser"}{path = "nameparser-core"}' "$binding_manifest"
grep -q 'path = "nameparser-core"' "$binding_manifest" \
  || { echo "error: failed to rewrite the core path dependency in $binding_manifest" >&2; exit 1; }

# The checked-in lockfile is carried over deliberately, NOT regenerated: it pins the exact
# dependency versions the repo's test suite runs against, and `rextendr::vendor_crates()` vendors
# `--locked`, so whatever is pinned here is what CRAN compiles. Regenerating at packaging time
# would quietly ship dependency versions nobody has tested.
#
# Rewriting the `path` above does not invalidate it: a path dependency is recorded by name and
# version with no `source`, and the core's identity is unchanged. Stripping its dev-dependencies
# matters here too — as a workspace member it would otherwise want criterion in the lock.
if ! cargo metadata --locked --format-version=1 --manifest-path "$binding_manifest" >/dev/null; then
  echo "error: bindings/r/src/rust/Cargo.lock is out of date with its Cargo.toml." >&2
  echo "       Update it deliberately and re-run the R tests before packaging:" >&2
  echo "         cargo update --manifest-path bindings/r/src/rust/Cargo.toml" >&2
  exit 1
fi

# ---------------------------------------------------------------------------------------------
# 4. Vendor the third-party crates + write the license inventory.
# ---------------------------------------------------------------------------------------------
echo "==> Vendoring registry crates"
Rscript -e 'rextendr::vendor_crates(commandArgs(TRUE)[1], clean = TRUE)' "$stage"

for f in src/rust/vendor.tar.xz src/rust/vendor-config.toml; do
  [ -f "$stage/$f" ] || { echo "error: vendoring did not produce $f" >&2; exit 1; }
done

# CRAN requires the licences of all bundled sources to be enumerated. `write_license_note()`
# derives LICENSE.note from `cargo metadata`, so it lists exactly what was vendored (plus the
# bundled core) and cannot drift from it by hand-editing.
echo "==> Writing LICENSE.note"
Rscript -e 'rextendr::write_license_note(commandArgs(TRUE)[1])' "$stage"
[ -f "$stage/LICENSE.note" ] || { echo "error: LICENSE.note was not written" >&2; exit 1; }

# Keep the checked-in copy in step, so the repo shows what a build would bundle.
if ! cmp -s "$stage/LICENSE.note" "$pkg/LICENSE.note" 2>/dev/null; then
  cp "$stage/LICENSE.note" "$pkg/LICENSE.note"
  echo "    refreshed bindings/r/LICENSE.note (commit it)"
fi

# ---------------------------------------------------------------------------------------------
# 5. Build the tarball.
# ---------------------------------------------------------------------------------------------
echo "==> R CMD build"
( cd "$out" && R CMD build --no-build-vignettes nameparser )

tarball="$out/nameparser_$version.tar.gz"
[ -f "$tarball" ] || { echo "error: expected $tarball" >&2; exit 1; }

if [ "$run_check" -eq 1 ]; then
  # The PDF-manual check needs a LaTeX install. Its absence is an environment gap, not a package
  # defect (CRAN's machines have one), and it surfaces as a hard ERROR that would bury the real
  # findings — so skip that one check rather than fail on it, and say so.
  check_args=""
  if ! command -v pdflatex >/dev/null 2>&1; then
    check_args="--no-manual"
    echo "note: pdflatex not found -- checking with --no-manual (the PDF manual is NOT validated)."
  fi
  # `checkbashisms` is the same kind of gap: without it, 'checking top-level files' WARNs.
  command -v checkbashisms >/dev/null 2>&1 \
    || echo "note: checkbashisms not found -- 'checking top-level files' will WARN for that reason alone."

  echo "==> R CMD check --as-cran"
  mkdir -p "$out/check"
  ( cd "$out" && R CMD check --as-cran $check_args -o check "$tarball" )
fi

if [ "$keep_stage" -eq 0 ]; then
  rm -rf "$stage"
fi

echo
echo "Built: $tarball  ($(du -h "$tarball" | cut -f1 | tr -d ' '))"
if [ "$run_check" -eq 0 ]; then
  echo "Next:  scripts/build-r-tarball.sh --check     # R CMD check --as-cran"
fi
echo "Then:  submit at https://cran.r-project.org/submit.html  (manual, human-reviewed)"
