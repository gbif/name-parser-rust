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
  # own golden fixtures, e.g. crates/nameparser-cli/tests/parse_cli.rs and
  # crates/nameparser-ffi/tests/ffi_json.rs, both of which pin
  # {"type":"OTHER","code":"VIRUS","message":"Unparsable OTHER name: ..."}
  # for this exact input. (crates/nameparser/tests/parse_golden.rs only checks
  # the unrelated NomCode::Virus -> "VIRUS" name mapping, not this fixture.)
  expect_false(out$parsed[3])
  expect_equal(out$type[3], "OTHER")
  expect_true(is.na(out$rank[3]))        # no ParsedName on error -> NA
  expect_true(is.na(out$candidatus[3]))  # bool flags are NA on error rows
})

test_that("authorship columns and warnings are flattened", {
  out <- parse_names("Vulpes vulpes silaceus Miller, 1907")
  expect_equal(out$combinationAuthors, "Miller")
  expect_equal(out$combinationYear, "1907")
  expect_true(is.na(out$basionymAuthors))
  expect_true("warnings" %in% names(out))
})

test_that("parse_name_json returns the core's full nested JSON", {
  js <- parse_name_json("Abies alba Mill.")
  obj <- jsonlite::fromJSON(js, simplifyVector = FALSE)
  expect_equal(obj$genus, "Abies")
  expect_equal(obj$specificEpithet, "alba")
  expect_equal(obj$combinationAuthorship$authors[[1]], "Mill.")
})

test_that("ex-author, basionym-year, and sanctioning-author columns are populated", {
  # fixtures confirmed against core-crate test cases before writing these assertions:
  # combination-level ex-author split (authorship_parser.rs:1344 ex_author_split shape)
  # and basionym-level ex-author split (same split, inside basionym parens); basionym
  # year (authorship_parser.rs:1193 basionym_and_combination_authors_with_year shape);
  # sanctioning author (authorship_parser.rs:1249 top_level_colon_splits_off_sanctioning_author).
  out <- parse_names(c(
    "Abies alba Wedd. ex Sch. Bip.",
    "Abies alba (Wedd. ex Sch. Bip.) Rehder",
    "Abies alba (Wang & Liu, 1996) Rehder",
    "Agaricus arvensis L. : Fr."
  ))
  # row 1: combination-level "ex" split
  expect_equal(out$combinationAuthors[1], "Sch.Bip.")
  expect_equal(out$combinationExAuthors[1], "Wedd.")
  # row 2: basionym-level "ex" split (same split, inside the basionym parens)
  expect_equal(out$basionymAuthors[2], "Sch.Bip.")
  expect_equal(out$basionymExAuthors[2], "Wedd.")
  # row 3: basionym year
  expect_equal(out$basionymAuthors[3], "Wang, Liu")
  expect_equal(out$basionymYear[3], "1996")
  # row 4: fungal sanctioning colon
  expect_equal(out$combinationAuthors[4], "L.")
  expect_equal(out$sanctioningAuthor[4], "Fr.")
})

test_that("warnings column carries a real (non-empty) warning, not just NA", {
  # "Buteo borealis ? ventralis" is the core crate's own fixture for this warning
  # (name_tokens.rs:1174 open_nomenclature_question_mark_between_epithets_sets_qualifier_on_infraspecific).
  out <- parse_names("Buteo borealis ? ventralis")
  expect_true(out$doubtful)
  expect_equal(out$warnings, "question marks removed")
})

test_that("parse_name_json's error envelope matches the FFI/CLI shape exactly", {
  # Byte-for-byte the same shape nameparser-ffi's `unparsable_json` and nameparser-cli's
  # `render_row` produce: {"type",["code" only when Some],"message"} -- no "name" key,
  # "code" OMITTED (not null) when absent, and this exact key order (not alphabetical).
  js_virus <- parse_name_json("Tobacco mosaic virus")
  expect_equal(
    js_virus,
    '{"error":{"type":"OTHER","code":"VIRUS","message":"Unparsable OTHER name: Tobacco mosaic virus"}}'
  )
  js_empty <- parse_name_json("")
  expect_equal(
    js_empty,
    '{"error":{"type":"OTHER","message":"Unparsable OTHER name: "}}'
  )
  obj <- jsonlite::fromJSON(js_virus, simplifyVector = FALSE)
  expect_false("name" %in% names(obj$error))
})
