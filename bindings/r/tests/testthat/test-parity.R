# Parity gate: parse_name_json() must reproduce the core's `parsed` object exactly, so it
# is byte-comparable (after normalizing the warnings set + key order) to the Java CLI oracle
# testdata/expected-parse.jsonl -- the same oracle crates/nameparser-cli and nameparser-py
# validate against. Skips cleanly if the (git-ignored, regenerated) oracle isn't present.
testthat::test_that("parse_name_json matches the Java CLI oracle over the benchmark corpus", {
  root <- normalizePath(file.path(testthat::test_path(), "..", "..", "..", ".."))
  corpus <- file.path(root, "testdata", "benchmark-data.txt")
  oracle <- file.path(root, "testdata", "expected-parse.jsonl")
  testthat::skip_if_not(file.exists(oracle) && file.exists(corpus),
                        "oracle/corpus not present (see parse_golden.rs to regenerate)")

  # PlainTextReader rule: name is the text before the first TAB, trimmed; skip blank/# lines.
  raw <- readLines(corpus, warn = FALSE)
  names_vec <- trimws(sub("\t.*$", "", raw))
  keep <- nzchar(names_vec) & !startsWith(raw, "#")
  names_vec <- names_vec[keep]

  expected <- readLines(oracle, warn = FALSE)
  testthat::expect_equal(length(names_vec), length(expected))

  norm <- function(o) {
    # sort the warnings set (Java HashSet order vs our Vec order) then re-serialize with
    # sorted keys, so the compare ignores warnings-order + object-key-order only.
    if (!is.null(o$warnings)) o$warnings <- sort(unlist(o$warnings))
    jsonlite::toJSON(o, auto_unbox = TRUE, null = "null")
  }
  mismatches <- 0L
  for (i in seq_along(names_vec)) {
    got <- jsonlite::fromJSON(parse_name_json(names_vec[i]), simplifyVector = FALSE)
    exp_line <- jsonlite::fromJSON(expected[i], simplifyVector = FALSE)
    # oracle rows are {"line":N,"input":..,"parsed":{..}} or {..,"error":{..}}
    if (!is.null(exp_line$parsed)) {
      if (norm(got) != norm(exp_line$parsed)) mismatches <- mismatches + 1L
    } else {
      if (is.null(got$error)) mismatches <- mismatches + 1L   # oracle errored; we parsed
    }
  }
  testthat::expect_equal(mismatches, 0L)
})
