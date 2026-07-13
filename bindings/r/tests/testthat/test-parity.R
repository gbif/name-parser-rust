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
    # Normalize ONLY the warnings set (Java HashSet order vs our Vec order) before comparing.
    # Object key order is NOT normalized here — jsonlite::toJSON preserves the list's order —
    # so the compare relies on ParsedName's field-declaration order being pinned to match the
    # Java oracle's key order by design (a core-crate wire-format guarantee). Net effect: a
    # genuine field/value difference fails; only warnings-set ordering is ignored.
    if (!is.null(o$warnings)) o$warnings <- sort(unlist(o$warnings))
    jsonlite::toJSON(o, auto_unbox = TRUE, null = "null")
  }
  # Inputs the 5.0.0 parser deliberately parses differently from the frozen 4.2.0 oracle — the
  # informal "tag capture" enhancement (Phase 5): a yearless "Genus sp. <tag>" now captures the tag
  # as the phrase (parse_name_json uses the raw parse() path, which carries the enhancement) instead
  # of the 4.2.0 oracle's misread author. Kept 1:1 with parse_golden::INFORMAL_5_0_0_DIVERGENCES.
  informal_divergences <- c(
    "Lacanobia sp. nr. subjuncta Bold:Aab, 0925",
    "Burkholderia sp. (Gigaspora margarita endosymbiont)",
    "Elaeocarpus sp. Rocky Creek"
  )
  mismatches <- 0L
  for (i in seq_along(names_vec)) {
    if (names_vec[i] %in% informal_divergences) next
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
