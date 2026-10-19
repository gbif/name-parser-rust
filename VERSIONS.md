# Versions

The major changes of the Rust name parser. The engine and all its bindings share one version: the
crate `gbif-name-parser`, the CLI, the Python and R packages and the Java `name-parser-rust`. The
model they implement is the Java `org.gbif:name-parser-api`, versioned on its own (see its
[VERSIONS.md](https://github.com/gbif/name-parser/blob/dev/VERSIONS.md)).

## Unreleased (`main`, api 6.0.0)

- No `Informal` parse result any more: a name in open nomenclature is a parsed name of type
  `INFORMAL`, with `ParsedName::anchor()` for the taxon it hangs off (#134).
- `Authorship.inAuthors` from in citations (#172); title abbreviations from IPNI tell a title from an
  author (#147).
- Made-up ranks, zoological sections within a genus, vectors and plasmids, all-capital authors
  (#167, #171, #124, #123).
- The nomenclatural code only from evidence of one code, never from the style of the authorship
  (#60). Only Fries and Persoon are sanctioning authors. An unranked infrageneric name or a
  zoological subgenus is written in brackets; a subgenus otherwise with `subg.`, or `subgen.` under
  the bacterial code.

## 0.4.0 (2026-10-14, api 5.4.0)

`Authorship.bracketedYear` and `Informal.candidatus`; serovars with their own rank and Salmonella's
`ser.`; misspelt rank markers; a whitelist of real names with a blacklisted epithet.

## 0.3.0 (2026-10-08, api 5.1.0 to 5.3.0)

`Authorship.anonymous` and the sanctioning author on `Authorship`; taxonomic notes (`non`, `sensu`,
`auct.`) and authorship placeholders no longer read as authors; notho rank markers; CJK authors
written surname first; the dual-path test harness, so a name parses alike with its authorship
embedded or given separately.

## 0.2.x (2026-08 to 2026-10, api 5.0.1)

- 0.2.0 (2026-08-25): the first release on all four channels (Maven, crates.io, PyPI, CLI). The
  informal phrase swallows a trailing authorship, and a rank hint beats an author particle.
  ChecklistBank parses through this engine since.
- 0.2.1 (2026-09-04): no silent truncation of numbers, numbered infraspecifics or codes after a rank
  marker (#16); the R package builds for CRAN.
- 0.2.2 (2026-10-02): WoRMS' temporary names, quoted names, trace fossils and organism labels.

## 0.1.0 (2026-07-14, api 5.0.0)

The first release: a port of the Java 4.2 pipeline, cross-validated against Java on 11,302 test
names and 6.4 million ChecklistBank names. A native CLI and bindings for Java (FFM, struct-only
wire, no JSON), Python and R. `NameType.IDENTIFIER` for identifiers such as BOLD BINs.

## Before Rust: the Java parser

- 5.0.0 (2026-07): the Java implementation removed, the api alone left in Java.
- 4.x (2026): a new Java step pipeline with code inference; 4.2.0 is what Rust ported.
- 3.x (2017 to 2026): the model in its own `name-parser-api`, authorships parsed down to individual
  authors.
- 2.x (2014 to 2017): GBIF's regex parser, filling the `ParsedName` of the GBIF api.
