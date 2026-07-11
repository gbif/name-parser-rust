#' Parse scientific names
#'
#' @param scientificname character vector of scientific names.
#' @param authorship,rank,code optional scalar hints (currently unused in the spike).
#' @return a tibble, one row per input name.
#' @export
parse_names <- function(scientificname, authorship = NULL, rank = NULL, code = NULL) {
  stopifnot(is.character(scientificname))
  cols <- parse_names_impl(scientificname)
  tibble::as_tibble(cols)
}
