#' Parse scientific names into a tibble (one row per name).
#'
#' @param scientificname character vector of scientific names.
#' @param authorship,rank,code optional scalar (length-1) hints applied to every name;
#'   `rank`/`code` use the parser's SCREAMING_SNAKE_CASE names (e.g. "SPECIES", "ZOOLOGICAL").
#' @return a tibble, one row per input name; unparsable names have `parsed = FALSE`.
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
