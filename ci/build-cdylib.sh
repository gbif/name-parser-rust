#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Build the nameparser-ffi cdylib and stage it for the Java module's per-platform classifier JARs.
#  - Always builds the HOST platform (plain cargo) into target/release/, which the Java tests load
#    via -Dnameparser.ffi.lib, and stages it as the host's classifier.
#  - When zig + cargo-zigbuild are available, ALSO cross-builds the other supported platforms
#    (incl. Apple Silicon) and stages each. Missing tools -> host-only (still a working classifier
#    JAR for the build platform, which on CI is the linux-x86_64 deploy target).
# Bootstraps rustup (and, best-effort, zig + cargo-zigbuild) so the pipeline runs on a plain agent.
# Each native is staged to bindings/java/native-staging/<classifier>/native/<classifier>/<lib>,
# which the pom packages as name-parser-rust-<version>-<classifier>.jar (see bindings/java/pom.xml).
set -euo pipefail

STAGE="bindings/java/native-staging"
# rust-target-triple : os-maven-plugin classifier : cdylib filename
TARGETS=(
  "x86_64-unknown-linux-gnu:linux-x86_64:libnameparser_ffi.so"
  "aarch64-unknown-linux-gnu:linux-aarch_64:libnameparser_ffi.so"
  "x86_64-apple-darwin:osx-x86_64:libnameparser_ffi.dylib"
  "aarch64-apple-darwin:osx-aarch_64:libnameparser_ffi.dylib"
  "x86_64-pc-windows-gnu:windows-x86_64:nameparser_ffi.dll"
)

# ---- rustup ----
if ! command -v cargo >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/cargo" ]; then
  echo "[build-cdylib] installing rustup (stable, minimal profile)…"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal --no-modify-path
fi
# shellcheck disable=SC1091
. "$HOME/.cargo/env"
cargo --version

# ---- host build: for the Java tests (-Dnameparser.ffi.lib -> target/release) and always staged ----
cargo build --release -p nameparser-ffi
HOST_TRIPLE="$(rustc -vV | awk '/^host:/{print $2}')"

# ---- zig + cargo-zigbuild for cross-compilation (best-effort; host-only if unavailable) ----
# Zig ships as a single self-contained tarball (no deps) — works on a plain Linux CI agent that
# has neither brew nor a system zig. Override the pinned version with ZIG_VERSION if needed.
ensure_zig() {
  command -v zig >/dev/null 2>&1 && return 0
  local os arch ver="${ZIG_VERSION:-0.13.0}"
  case "$(uname -s)" in Linux) os=linux ;; Darwin) os=macos ;; *) return 1 ;; esac
  case "$(uname -m)" in x86_64|amd64) arch=x86_64 ;; arm64|aarch64) arch=aarch64 ;; *) return 1 ;; esac
  if [ "$os" = macos ] && command -v brew >/dev/null 2>&1 && brew install zig >/dev/null 2>&1; then return 0; fi
  local dir="$HOME/.local/zig-$ver"
  if [ ! -x "$dir/zig" ]; then
    echo "[build-cdylib] downloading zig $ver ($os-$arch)…"
    mkdir -p "$dir"
    curl -fsSL "https://ziglang.org/download/$ver/zig-$os-$arch-$ver.tar.xz" | tar -xJ -C "$dir" --strip-components=1 || return 1
  fi
  export PATH="$dir:$PATH"
  command -v zig >/dev/null 2>&1
}
HAVE_ZIG=0
if ensure_zig && { cargo zigbuild --help >/dev/null 2>&1 || cargo install cargo-zigbuild >/dev/null 2>&1; }; then HAVE_ZIG=1; fi
[ "$HAVE_ZIG" = 1 ] && echo "[build-cdylib] zig cross-compilation available" \
                    || echo "[build-cdylib] no zig/cargo-zigbuild — building host platform only"

# ---- stage each platform ----
rm -rf "$STAGE"
for entry in "${TARGETS[@]}"; do
  triple="${entry%%:*}"; rest="${entry#*:}"; cls="${rest%%:*}"; lib="${rest##*:}"
  if [ "$triple" = "$HOST_TRIPLE" ]; then
    src="target/release/$lib"
  elif [ "$HAVE_ZIG" = 1 ]; then
    rustup target add "$triple" >/dev/null 2>&1 || true
    echo "[build-cdylib] cross-building $triple ($cls)…"
    cargo zigbuild --release --target "$triple" -p nameparser-ffi || { echo "[build-cdylib] WARN: $triple failed, skipping"; continue; }
    src="target/$triple/release/$lib"
  else
    continue
  fi
  if [ -f "$src" ]; then
    dest="$STAGE/$cls/native/$cls"
    mkdir -p "$dest"
    cp "$src" "$dest/$lib"
    echo "[build-cdylib] staged $cls"
  fi
done
echo "[build-cdylib] staged platforms:"; ls -1 "$STAGE" 2>/dev/null | sed 's/^/  /' || echo "  (none)"
