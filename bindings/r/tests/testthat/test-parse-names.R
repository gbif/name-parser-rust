test_that("parse_names returns a tibble with a row per name and NA on the error column for parsed names", {
  out <- parse_names(c("Abies alba Mill.", "Tobacco mosaic virus"))
  expect_s3_class(out, "tbl_df")
  expect_equal(nrow(out), 2L)
  expect_equal(out$scientificName, c("Abies alba Mill.", "Tobacco mosaic virus"))
  expect_equal(out$parsed, c(TRUE, FALSE))
  expect_true(is.na(out$error[1]))       # Vec<Option<String>> None -> NA
  expect_false(is.na(out$error[2]))      # unparsable -> message present
})
