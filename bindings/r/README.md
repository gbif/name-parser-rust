# nameparser (R)

An R binding — via [extendr](https://extendr.rs)/[rextendr](https://extendr.rs/rextendr/) —
to the Rust port of the [GBIF name parser](https://github.com/gbif/name-parser). Parses
scientific names into their structured atoms (genus, epithets, authorship, rank, ...) with
byte-for-byte behavioural parity to the Java `org.gbif:name-parser`, at native speed and with
no JVM dependency.

## Install

Requires a Rust toolchain (`cargo`/`rustc`) on `PATH` at build time — this package compiles a
small Rust crate (`src/rust/`) and links it into the R package's shared library. There is no
CRAN release yet (see "Deferrals" below), so install from a local checkout or GitHub:

```r
# from a local checkout of this repo:
install.packages("rextendr")   # if not already installed; needed to build from source
devtools::install("bindings/r")
# or, without devtools:
# R CMD INSTALL bindings/r

# from GitHub:
# remotes::install_github("gbif/name-parser-rust", subdir = "bindings/r")
```

## Usage

### `parse_names()` — vectorized tibble

The primary entry point. Takes a character vector of scientific names and returns a
[tibble](https://tibble.tidyverse.org/), one row per input name, **never throwing** — an
unparsable name gets a row with `parsed = FALSE` and `NA` atoms rather than an R condition.

```r
library(nameparser)

out <- parse_names(c(
  "Abies alba Mill.",
  "Vulpes vulpes silaceus Miller, 1907",
  "Tobacco mosaic virus"
))
out[, c("scientificName", "parsed", "rank", "genus", "specificEpithet",
        "infraspecificEpithet", "combinationAuthors", "combinationYear", "error")]
#> # A tibble: 3 x 9
#>   scientificName         parsed rank  genus specificEpithet infraspecificEpithet
#>   <chr>                  <lgl>  <chr> <chr> <chr>           <chr>
#> 1 Abies alba Mill.       TRUE   SPEC… Abies alba            <NA>
#> 2 Vulpes vulpes silaceu… TRUE   SUBS… Vulp… vulpes          silaceus
#> 3 Tobacco mosaic virus   FALSE  <NA>  <NA>  <NA>            <NA>
#> # i 3 more variables: combinationAuthors <chr>, combinationYear <chr>, error <chr>
```

`parse_names()` accepts optional scalar hints applied to every name in the vector:
`authorship`, `rank`, `code` (the parser's `SCREAMING_SNAKE_CASE` enum names, e.g.
`"SPECIES"`, `"ZOOLOGICAL"`).

All 43 columns, in order: `scientificName`, `result`, `parsed`, `error`, `type`, `taxon`,
`taxonRank`, `rank`, `code`, `uninomial`, `genus`, `infragenericEpithet`, `specificEpithet`,
`infraspecificEpithet`, `cultivarEpithet`, `phrase`, `candidatus`, `notho`, `originalSpelling`,
`epithetQualifier`, `extinct`, `taxonomicNote`, `nomenclaturalNote`, `publishedIn`,
`publishedInYear`, `publishedInPage`, `unparsed`, `doubtful`, `manuscript`, `state`,
`combinationAuthors`, `combinationExAuthors`, `combinationYear`, `basionymAuthors`,
`basionymExAuthors`, `basionymYear`, `sanctioningAuthor`, `warnings`, `canonical`,
`canonicalWithoutAuthorship`, `canonicalMinimal`, `canonicalComplete`, `authorshipComplete`.

`result` is the 5.0.0 three-way outcome (`"parsed"` / `"informal"` / `"unparsable"`); `taxon`
and `taxonRank` carry an informal name's supraspecific anchor; the last five are `NameFormatter`
renderings (`canonical` is populated for informal rows too, the other four are `NA` there).

### `parse_name_json()` — lossless escape hatch

For the rare case a flattened tibble column can't represent (see "Known limitations" below),
`parse_name_json()` returns the parser's complete, nested JSON for a single name — the exact
same wire shape the Java CLI and this project's Python binding emit — which you can parse
with [`jsonlite`](https://cran.r-project.org/package=jsonlite):

```r
js <- parse_name_json("Abies alba Mill.")
jsonlite::fromJSON(js, simplifyVector = FALSE)$combinationAuthorship
#> $authors
#> $authors[[1]]
#> [1] "Mill."
#>
#> $exAuthors
#> list()
```

On an unparsable name, it returns `{"error":{"type":...,"code":...,"message":...}}` (`code`
omitted, not `null`, when the parser has none) — the same shape
`crates/nameparser-ffi`'s `unparsable_json` and `crates/nameparser-cli`'s `render_row`
produce, so it is directly diffable against those surfaces' output.

## Design: fitting the `rgbif` ecosystem

The API is shaped to feel at home next to [`rgbif`](https://docs.ropensci.org/rgbif/)'s own
`name_parse()`: a vectorized function taking a `scientificname` argument and returning one
row per name, rather than a per-name object a caller must loop over. That is where the
resemblance is *intentional* stops, though — the **columns themselves are the new, faithful
`ParsedName` model** (the same schema the Java FFM, Python, and CLI bindings all expose),
not `rgbif::name_parse()`'s legacy GBIF Name Parser v1 web-service schema. Concretely:
camelCase column names matching the JSON/Java/Python wire format (`specificEpithet`, not
`specificepithet` or `species`); `parsed = FALSE` + `NA` atoms for unparsable rows rather than
a separate `parsedPartially` flag; enum columns (`type`, `rank`, `code`, `state`) as plain
`SCREAMING_SNAKE_CASE` strings, not integers or factors.

## Known limitations / deferrals

- **`errorCode` is not a tibble column.** On an unparsable row, `parse_names()` surfaces
  `type` (the `NameType`) and `error` (the message) but **not** `ParseError.code` (the
  `NomCode`, e.g. `"VIRUS"` for `"Tobacco mosaic virus"`) — adding a 36th column just for
  this one `Err`-only field was judged not worth re-touching the five parallel column lists
  `parse_names_impl` maintains. `parse_name_json()` **does** carry it losslessly (see above)
  — use it when you need `code` on an error row.
- **`genericAuthorship`/`specificAuthorship` are JSON-only.** These niche botanical
  `CombinedAuthorship` bundles (infrageneric-rank names with two independent author strings)
  are not flattened into `parse_names()` columns; get them via `parse_name_json()`.
- **Not on CRAN.** CRAN packages that build native code from a non-vendored, network-fetched
  dependency tree (this crate's core-parser path dependency plus its own crates.io
  dependencies) need the whole dependency graph vendored into the source tarball per CRAN's
  offline-build policy. That vendoring step is deferred; today's only install path is a
  local checkout or `remotes::install_github()`.

## Testing

```bash
Rscript -e 'devtools::load_all("bindings/r", quiet=TRUE); testthat::test_dir("bindings/r/tests/testthat")'
```

Includes a corpus **parity gate** (`tests/testthat/test-parity.R`) that diffs
`parse_name_json()` against the frozen golden snapshot `testdata/golden/expected-parse.jsonl`
over the ~8,000-name benchmark corpus — the same snapshot the native CLI and Python binding
validate against. It skips cleanly (does not fail) if that git-ignored file isn't present
locally; see `crates/nameparser/tests/parse_golden.rs`'s module doc to regenerate it.

## License

Apache-2.0, matching the rest of this repository and `org.gbif:name-parser`.
