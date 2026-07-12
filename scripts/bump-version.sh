#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Set ONE version across every artifact in this repo so all bindings release in lockstep with the
# same number (= the same Rust engine). See RELEASE.md for the full release flow.
#
#   scripts/bump-version.sh 0.2.0
#
# Sets:
#   Cargo [workspace.package] version     -> 0.2.0          (core / CLI / ffi / py-crate inherit it)
#   crates/nameparser-py/pyproject.toml   -> 0.2.0          (PyPI wheel)
#   bindings/r/DESCRIPTION                -> 0.2.0          (CRAN)
#   bindings/java/pom.xml     <version>   -> 0.2.0-SNAPSHOT (Maven dev version; the Jenkins release
#   bindings/java/jmh/pom.xml dep version -> 0.2.0-SNAPSHOT  job strips -SNAPSHOT to 0.2.0 at release)
#
# It does NOT touch the org.gbif:name-parser-api / name-parser (4.x) dependency versions -- that is
# the stable contract, versioned independently of the engine (see RELEASE.md / DISTRIBUTION.md).
set -euo pipefail

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
  echo "usage: scripts/bump-version.sh <version>    e.g. 0.2.0  (or a pre-release: 0.2.0rc1)" >&2
  exit 2
fi
# X.Y.Z with an optional pre-release suffix (0.2.0, 0.2.0rc1, 0.2.0-alpha.1)
if ! printf '%s' "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+([._-]?[0-9A-Za-z][0-9A-Za-z.]*)?$'; then
  echo "error: '$VERSION' is not a valid version (expected e.g. 0.2.0 or 0.2.0rc1)" >&2
  exit 1
fi

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
echo "Setting all artifacts to version: $VERSION"

# 1. Cargo workspace version -- the only `version = "..."` at column 0 in the root manifest;
#    every crate inherits it via `version.workspace = true`.
perl -i -pe 's/^version = "[^"]*"/version = "'"$VERSION"'"/' Cargo.toml

# 2. Python wheel -- the [project] `version` (the only top-level `version = "..."`).
perl -i -pe 's/^version = "[^"]*"/version = "'"$VERSION"'"/' crates/nameparser-py/pyproject.toml

# 3. R package (CRAN).
perl -i -pe 's/^Version: .*/Version: '"$VERSION"'/' bindings/r/DESCRIPTION

# 4. Java binding + its JMH module's dependency on it -> X-SNAPSHOT (Maven dev-version convention).
#    Targets ONLY the <version> immediately following the name-parser-rust <artifactId>, so plugin
#    and other dependency versions in the poms are left alone.
perl -0777 -i -pe \
  's{(<artifactId>name-parser-rust</artifactId>\s*<version>)[^<]*(</version>)}{${1}'"$VERSION"'-SNAPSHOT${2}}g' \
  bindings/java/pom.xml bindings/java/jmh/pom.xml

echo
echo "Done. Version now set to:"
printf '  %-16s %s\n' "Cargo workspace" "$(grep -m1 '^version = ' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')"
printf '  %-16s %s\n' "Python"          "$(grep -m1 '^version = ' crates/nameparser-py/pyproject.toml | sed -E 's/.*"([^"]+)".*/\1/')"
printf '  %-16s %s\n' "R"               "$(grep -m1 '^Version: ' bindings/r/DESCRIPTION | awk '{print $2}')"
printf '  %-16s %s\n' "Java (pom)"      "$(grep -A1 '<artifactId>name-parser-rust</artifactId>' bindings/java/pom.xml | grep -m1 '<version>' | sed -E 's/.*<version>([^<]+)<.*/\1/')"
echo
echo "Next: review 'git diff', run the tests, then follow RELEASE.md to tag + deploy each channel."
