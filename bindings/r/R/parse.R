#' Parse scientific names into a tibble (one row per name).
#'
#' @param scientificname character vector of scientific names.
#' @param authorship,rank,code optional scalar (length-1) hints applied to every name;
#'   `rank`/`code` use the parser's SCREAMING_SNAKE_CASE names (e.g. "SPECIES", "ZOOLOGICAL").
#' @return a tibble, one row per input name. The `result` column carries the 5.0.0 three-way
#'   outcome — `"parsed"`, `"informal"` or `"unparsable"` — and `parsed` is the convenience boolean
#'   (`TRUE` only for `"parsed"`). An **informal** name (a supraspecific taxon carrying a provisional
#'   designation, e.g. `"Rhizobium sp. RMCC TR1811"`) is flat: its `taxon` + `taxonRank` + `rank` +
#'   `phrase` + `code` are populated and every ParsedName-specific column is `NA` (the anchor lives
#'   in `taxon`, never a mislabelled `genus`). A name with a species epithet (incl. cf./aff.) stays
#'   `"parsed"`. Besides the parsed atoms, five `NameFormatter` rendering columns are included:
#'   `canonical` (full name with authorship), `canonicalWithoutAuthorship`,
#'   `canonicalMinimal` (bare parts, folded to ascii), `canonicalComplete` and
#'   `authorshipComplete` — all `NA` on informal + unparsable rows.
#' @export
parse_names <- function(scientificname, authorship = NULL, rank = NULL, code = NULL) {
  stopifnot(is.character(scientificname))
  cols <- parse_names_impl(
    scientificname,
    authorship = if (is.null(authorship)) NULL else as.character(authorship)[1],
    rank       = if (is.null(rank))       NULL else as.character(rank)[1],
    code       = if (is.null(code))       NULL else as.character(code)[1]
  )
  tibble::as_tibble(cols)
}

#' Parse a single scientific name to the parser's full nested JSON.
#'
#' The lossless representation — every field, including the nested
#' `combinationAuthorship`/`basionymAuthorship` and the niche
#' `genericAuthorship`/`specificAuthorship` bundles that `parse_names()` does not flatten.
#'
#' @param name a single scientific name.
#' @param authorship,rank,code optional scalar hints (see [parse_names()]).
#' @return a length-1 JSON string (parse it with `jsonlite::fromJSON`).
#' @export
parse_name_json <- function(name, authorship = NULL, rank = NULL, code = NULL) {
  stopifnot(is.character(name), length(name) == 1L)
  parse_name_json_impl(
    name,
    authorship = if (is.null(authorship)) NULL else as.character(authorship)[1],
    rank       = if (is.null(rank))       NULL else as.character(rank)[1],
    code       = if (is.null(code))       NULL else as.character(code)[1]
  )
}
