#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Build the nameparser-ffi release cdylib for the current platform, bootstrapping rustup first if
# the Jenkins agent has no Rust — so the pipeline runs on a plain `agent any` (see Jenkinsfile).
# Idempotent: ~/.cargo lives in the agent's home (outside the wiped workspace), so rustup is
# installed at most once per agent. The Java FFM binding's pom (bindings/java/pom.xml) then copies
# the result from target/release/ into native/${os.detected.classifier}/ inside the JAR.
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/cargo" ]; then
  echo "[build-cdylib] no Rust found — installing rustup (stable, minimal profile)…"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal --no-modify-path
fi

# shellcheck disable=SC1091
. "$HOME/.cargo/env"
cargo --version
cargo build --release -p nameparser-ffi
ls -la target/release/libnameparser_ffi.* target/release/nameparser_ffi.* 2>/dev/null || true
