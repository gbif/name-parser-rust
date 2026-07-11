# name-parser-rust

A Rust port of the [GBIF name parser](https://github.com/gbif/name-parser) — a linear-time,
ReDoS-free reimplementation that parses scientific names into a structured `ParsedName`, with
byte-for-byte behavioural parity to the Java `org.gbif:name-parser`.

> **Status: work in progress.** The core parser, the native CLI, and the Java, Python, and R
> bindings are all complete and validated (Phases 1–4). The backend cutover is not yet done
> — see the [roadmap](#roadmap).

## Why

The Rust core is the single authoritative implementation
([approach B](docs/superpowers/specs/2026-07-09-name-parser-rust-design.md)); the Java library
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
error bars, and the ReDoS-tail comparison — is in [`benchmarks.md`](benchmarks.md); field-level
correctness parity is in [`cross-validation.md`](cross-validation.md).

## Layout

```
crates/
  nameparser/       # the core parser — pure Rust, zero FFI. All parsing logic lives here.
  nameparser-cli/   # native CLI (clap): parse / benchmark / compare
  nameparser-ffi/   # C-ABI cdylib (JSON + flat-struct wire formats) for the Java binding
  nameparser-py/    # native Python binding (PyO3), depends on the core crate directly
bindings/
  java/             # NameParserRust implements org.gbif.nameparser.api.NameParser, via Panama/FFM
  r/                # R package `nameparser` (extendr): parse_names() tibble + parse_name_json()
docs/superpowers/   # design spec, implementation plans, and per-phase findings
```

## Bindings

| Binding | Path | Status |
|---|---|---|
| Java (Panama/FFM) | `bindings/java` | Complete & parity-validated; dev-only until the native `nameparser-ffi` cdylib is packaged for a real Maven dependency (see [`DISTRIBUTION.md`](DISTRIBUTION.md)) |
| Python (PyO3) | `crates/nameparser-py` | Complete & parity-validated (11,302/11,302 vs the Java oracle); wheel built locally, not yet published to PyPI |
| R (extendr) | `bindings/r` | Complete & parity-validated (8,017/8,017 vs the Java oracle); install from a local checkout or GitHub, not yet on CRAN — see [`bindings/r/README.md`](bindings/r/README.md) |

## Build & test

```bash
cargo build --release            # workspace: core + CLI + ffi cdylib
cargo test --workspace           # core tests incl. corpus golden-diff parity gates

# Java binding (needs JDK 22+, where java.lang.foreign is stable):
cargo build -p nameparser-ffi --release
mvn -f bindings/java/pom.xml test # smoke + the 11,302-name parity test vs NameParserImpl
```

## Relationship to `gbif/name-parser`

This repo targets behavioural parity with the Java parser's `4.2.x` line and preserves the
`org.gbif.nameparser.api.NameParser` interface, so that a future cutover can swap
`new NameParserImpl()` → `new NameParserRust()` with no other change. Until then the Java
library remains authoritative; this port is validated against it, not the reverse.

## Roadmap

- [x] **Phase 1** — faithful core: the whole pipeline, full field parity
- [x] **Phase 2** — native CLI + large-corpus cross-validation
- [x] **Phase 3** — in-process Java FFM/Panama binding
- [x] **Phase 4** — Python (PyO3) + R (extendr) bindings
- [ ] **Phase 5** — backend cutover; retire the Java `pipeline` package

## License

Apache-2.0, matching `org.gbif:name-parser`.
