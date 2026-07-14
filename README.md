# name-parser-rust

A Rust port of the [GBIF name parser](https://github.com/gbif/name-parser) — a linear-time,
ReDoS-free reimplementation that parses scientific names into a structured `ParsedName`, with
byte-for-byte behavioural parity to the Java `org.gbif:name-parser`.

> **Status: released.** The core parser, the native CLI, and the Java, Python, and R bindings
> are complete, cross-validated, and published at 0.1.0. The backend cutover is not yet done
> — see the [roadmap](#roadmap).

## Why

The Rust core is the single authoritative implementation; the Java library
becomes a thin binding over it. Three motivations:

- **Polyglot reach** — usable outside the JVM (Java, a native CLI, Python, and R).
- **Throughput** — faster batch parsing of multi-million-name corpora.
- **Robustness** — a linear-time regex engine structurally eliminates the catastrophic-backtracking
  (ReDoS) tail the Java parser hand-fights with ~20 possessive quantifiers.

## Validated parity & performance

Measured against the Java parser (`org.gbif:name-parser` 4.2.0), same machine:

| Check | Result |
|---|---|
| Full `ParsedName` field parity (in-harness, 8,017 names) | **30 / 30 fields, 0 mismatches** |
| Rust CLI vs Java CLI cross-validation | **11,302 / 11,302** and **7,991,756 / 7,991,756** (8 M CoL names) — 0 diffs |
| Batch throughput (CLI, single core) | **~2.1× faster** (13.7 vs 28.8 µs/name); p95 tail ~2.4× |
| In-process Java via FFM/Panama (JMH, single name) | 1.38× (flat-struct wire format) — capped by the Java-object-build floor |

Full cross-era (3.x → 4.x → Rust) and cross-binding breakdown — with methodology, percentiles,
error bars, and the ReDoS-tail comparison — is in [`BENCHMARKS.md`](BENCHMARKS.md); field-level
correctness parity is in [`cross-validation.md`](cross-validation.md).

## Layout

```
crates/
  nameparser/       # the core parser — pure Rust, zero FFI. All parsing logic lives here.
  nameparser-cli/   # native CLI (clap): parse / benchmark / compare / validate — see its README
  nameparser-ffi/   # C-ABI cdylib (JSON + flat-struct wire formats) for the Java binding
  nameparser-py/    # native Python binding (PyO3), depends on the core crate directly
bindings/
  java/             # NameParserRust implements org.gbif.nameparser.api.NameParser, via Panama/FFM
  r/                # R package `nameparser` (extendr): parse_names() tibble + parse_name_json()
```

## Bindings

| Binding | Path | Status |
|---|---|---|
| Java (Panama/FFM) | `bindings/java` | Complete & parity-validated; self-contained JAR (bundles the `nameparser-ffi` cdylib) — **published to GBIF Nexus** (`org.gbif.nameparser:name-parser-rust:0.1.0`; snapshots auto-deploy on every push to `main`), see [`DISTRIBUTION.md`](DISTRIBUTION.md) |
| Python (PyO3) | `crates/nameparser-py` | Complete & parity-validated (11,302/11,302 vs the Java oracle); **published to PyPI** — `pip install gbif-name-parser` |
| R (extendr) | `bindings/r` | Complete & parity-validated (8,017/8,017 vs the Java oracle); install from a local checkout or GitHub, not yet on CRAN — see [`bindings/r/README.md`](bindings/r/README.md) |

## Native CLI

`nameparser-cli` runs the parser from the command line — parse names to JSON, **standardize** them
(`--canonical`), benchmark throughput, diff two parse runs, or run an LLM-judged validation sweep.
A quick taste:

```sh
echo 'Betula pendula ROTH' | nameparser-cli parse --canonical
# {"line":1,"input":"Betula pendula ROTH","canonical":"Betula pendula Roth","parsed":{…}}
```

Full command + flag reference: [`crates/nameparser-cli/README.md`](crates/nameparser-cli/README.md).

## Build & test

```bash
cargo build --release            # workspace: core + CLI + ffi cdylib
cargo test --workspace           # core tests incl. corpus golden-diff parity gates

# Java binding (needs JDK 22+, where java.lang.foreign is stable):
cargo build -p nameparser-ffi --release
mvn -f bindings/java/pom.xml test # smoke + the ~8,017-name golden-snapshot parity test
```

## Relationship to `gbif/name-parser`

This repo targets behavioural parity with the Java parser's `4.2.x` line and preserves the
`org.gbif.nameparser.api.NameParser` interface, so that a future cutover can swap
`new NameParserImpl()` → `new NameParserRust()` with no other change. Until then the Java
library remains authoritative; this port is validated against it, not the reverse.

## Roadmap

- [x] Rust core — the full parsing pipeline, byte-for-byte field parity with the Java parser
- [x] Native CLI + large-corpus cross-validation
- [x] Java (FFM/Panama), Python (PyO3), and R (extendr) bindings — published at 0.1.0
- [ ] Backend cutover — swap `NameParserImpl` → `NameParserRust`, retire the Java `pipeline` package

## License

Apache-2.0, matching `org.gbif:name-parser`.
