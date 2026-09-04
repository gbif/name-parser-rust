# Note: Any variables prefixed with `.` are used for text
# replacement in the Makevars.in and Makevars.win.in

# check the packages MSRV first
source("tools/msrv.R")

# check DEBUG and NOT_CRAN environment variables
env_debug <- Sys.getenv("DEBUG")
env_not_cran <- Sys.getenv("NOT_CRAN")

# check if the vendored zip file exists
vendor_exists <- file.exists("src/rust/vendor.tar.xz")

is_not_cran <- env_not_cran != ""
is_debug <- env_debug != ""

if (is_debug) {
  # if we have DEBUG then we set not cran to true
  # CRAN is always release build
  is_not_cran <- TRUE
  message("Creating DEBUG build.")
}

if (!is_not_cran) {
  message("Building for CRAN.")
}

# we set cran flags only if NOT_CRAN is empty and if
# the vendored crates are present.
.cran_flags <- ifelse(
  !is_not_cran && vendor_exists,
  "-j 2 --offline",
  ""
)

# when DEBUG env var is present we use `--debug` build
.profile <- ifelse(is_debug, "", "--release")
.clean_targets <- ifelse(is_debug, "", "$(TARGET_DIR)")

# We specify this target when building for webR
webr_target <- "wasm32-unknown-emscripten"

# here we check if the platform we are building for is webr
is_wasm <- identical(R.version$platform, webr_target)

# print to terminal to inform we are building for webr
if (is_wasm) {
  message("Building for WebR")
}

# we check if we are making a debug build or not
# if so, the LIBDIR environment variable becomes:
# LIBDIR = $(TARGET_DIR)/{wasm32-unknown-emscripten}/debug
# this will be used to fill out the LIBDIR env var for Makevars.in
target_libpath <- if (is_wasm) "wasm32-unknown-emscripten" else NULL
cfg <- if (is_debug) "debug" else "release"

# used to replace @LIBDIR@
.libdir <- paste(c(target_libpath, cfg), collapse = "/")

# use this to replace @TARGET@
# we specify the target _only_ on webR
# there may be use cases later where this can be adapted or expanded
.target <- ifelse(is_wasm, paste0("--target=", webr_target), "")

# add panic exports only for WASM builds
.panic_exports <- ifelse(
  is_wasm,
  "CARGO_PROFILE_DEV_PANIC=\"abort\" CARGO_PROFILE_RELEASE_PANIC=\"abort\" ",
  ""
)

# read in the Makevars.in file checking
is_windows <- .Platform[["OS.type"]] == "windows"

# Wrapper regeneration -- `cargo run --bin document`, which rewrites R/extendr-wrappers.R -- is a
# DEVELOPMENT step: rextendr::document() / devtools::document() drive it through this Makevars
# recipe. It must NOT run when a distributed tarball is being installed, for two reasons:
#
#   1. The wrappers already ship in R/, so regenerating them is pure work -- and it writes inside
#      the package directory being installed.
#   2. It builds and then *executes* a host binary. On CRAN's Windows builder that fails outright:
#        error: failed to run custom build command for `proc-macro2`
#        could not execute process ...\target\debug\build\proc-macro2-*\build-script-build
#        %1 ist keine zulaessige Win32-Anwendung. (os error 193)
#      -- the build scripts under the cross-compiled target are not host-executable there. It took
#      `R CMD INSTALL` down with it, so the whole check came back as 1 ERROR.
#
# A vendored tarball is the reliable signal for "distributed package, not a checkout":
# scripts/build-r-tarball.sh always produces src/rust/vendor.tar.xz, and a git checkout never has
# one. (NOT_CRAN is NOT usable here -- pkgbuild::compile_dll, which rextendr::document() goes
# through, does not set it, so keying on it would silently stop regenerating wrappers in dev.)
.document <- if (vendor_exists) {
  "echo '=== Distributed build: keeping the shipped R/extendr-wrappers.R ==='"
} else if (is_windows) {
  "cargo run @CRAN_FLAGS@ --bin document --target $(TARGET) --manifest-path=./rust/Cargo.toml --target-dir $(TARGET_DIR)"
} else {
  "cargo run @CRAN_FLAGS@ --bin document --manifest-path=./rust/Cargo.toml --target-dir $(TARGET_DIR) @TARGET@"
}

# if windows we replace in the Makevars.win.in
mv_fp <- ifelse(
  is_windows,
  "src/Makevars.win.in",
  "src/Makevars.in"
)

# set the output file
mv_ofp <- ifelse(
  is_windows,
  "src/Makevars.win",
  "src/Makevars"
)

# delete the existing Makevars{.win/.wasm}
if (file.exists(mv_ofp)) {
  message("Cleaning previous `", mv_ofp, "`.")
  invisible(file.remove(mv_ofp))
}

# read as a single string
mv_txt <- readLines(mv_fp)

# replace placeholder values.
# @DOCUMENT@ goes first: its expansion may itself contain @CRAN_FLAGS@ / @TARGET@, which the
# substitutions below then resolve.
new_txt <- gsub("@DOCUMENT@", .document, mv_txt) |>
  gsub("@CRAN_FLAGS@", .cran_flags, x = _) |>
  gsub("@PROFILE@", .profile, x = _) |>
  gsub("@CLEAN_TARGET@", .clean_targets, x = _) |>
  gsub("@LIBDIR@", .libdir, x = _) |>
  gsub("@TARGET@", .target, x = _) |>
  gsub("@PANIC_EXPORTS@", .panic_exports, x = _)

message("Writing `", mv_ofp, "`.")
con <- file(mv_ofp, open = "wb")
writeLines(new_txt, con, sep = "\n")
close(con)

message("`tools/config.R` has finished.")
