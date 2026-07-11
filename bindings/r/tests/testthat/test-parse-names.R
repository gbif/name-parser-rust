test_that("parse_names returns a tibble with a row per name and NA on the error column for parsed names", {
  out <- parse_names(c("Abies alba Mill.", "Tobacco mosaic virus"))
  expect_s3_class(out, "tbl_df")
  expect_equal(nrow(out), 2L)
  expect_equal(out$scientificName, c("Abies alba Mill.", "Tobacco mosaic virus"))
  expect_equal(out$parsed, c(TRUE, FALSE))
  expect_true(is.na(out$error[1]))       # Vec<Option<String>> None -> NA
  expect_false(is.na(out$error[2]))      # unparsable -> message present
})

test_that("scalar columns match the known Java/Python oracle values", {
  out <- parse_names(c("Abies alba Mill.",
                       "Vulpes vulpes silaceus Miller, 1907",
                       "Tobacco mosaic virus"))
  # Abies alba Mill.
  expect_equal(out$type[1], "SCIENTIFIC")
  expect_equal(out$rank[1], "SPECIES")
  expect_equal(out$genus[1], "Abies")
  expect_equal(out$specificEpithet[1], "alba")
  expect_true(is.na(out$code[1]))
  expect_false(out$candidatus[1])
  # Vulpes vulpes silaceus Miller, 1907 -> zoological subspecies
  expect_equal(out$rank[2], "SUBSPECIES")
  expect_equal(out$code[2], "ZOOLOGICAL")
  expect_equal(out$infraspecificEpithet[2], "silaceus")
  # Tobacco mosaic virus -> unparsable, NameType OTHER (ParseError.code is
  # VIRUS, but NameType has no VIRUS variant -- confirmed against the core's
  # own golden fixtures, e.g. crates/nameparser/tests/parse_golden.rs and
  # crates/nameparser-cli/tests/parse_cli.rs, both of which pin
  # {"type":"OTHER","code":"VIRUS","message":"Unparsable OTHER name: ..."}
  # for this exact input).
  expect_false(out$parsed[3])
  expect_equal(out$type[3], "OTHER")
  expect_true(is.na(out$rank[3]))        # no ParsedName on error -> NA
  expect_true(is.na(out$candidatus[3]))  # bool flags are NA on error rows
})
