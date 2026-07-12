# gbif-name-parser

A faithful Rust port of the [GBIF](https://www.gbif.org) scientific name parser — the engine
behind `org.gbif:name-parser`. It parses a scientific name into its structured atoms: genus,
epithets, authorship, rank, nomenclatural code, and notes/warnings.

The package is `gbif-name-parser`; the library is imported as `nameparser`:

```toml
[dependencies]
gbif-name-parser = "0.1"
```

```rust
use nameparser::parse;

let pn = parse("Vulpes vulpes silaceus Miller, 1907", None, None, None).unwrap();
assert_eq!(pn.genus.as_deref(), Some("Vulpes"));
assert_eq!(pn.specific_epithet.as_deref(), Some("vulpes"));
assert_eq!(pn.infraspecific_epithet.as_deref(), Some("silaceus"));
assert_eq!(pn.combination_authorship.authors, vec!["Miller"]);
```

The output is byte-for-byte cross-validated against the Java oracle (`NameParserImpl`) over
11,302 + 6.4M names (0 diffs).

## The same engine, other languages

This crate is the core; the same parser ships as a native CLI, a Java (FFM/Panama) binding, a
Python package (`pip install gbif-name-parser`, `import nameparser`), and an R package — all
versioned together, so one version means the same engine everywhere. See the
[repository](https://github.com/gbif/name-parser-rust).

## License

Apache-2.0.
