# name-parser-rust

The GBIF name parser — a linear-time, ReDoS-free parser that turns a scientific name into a
structured `ParsedName`. This is the authoritative implementation: the Java
[`gbif/name-parser`](https://github.com/gbif/name-parser) repo now publishes only the
`org.gbif:name-parser-api` contract, and every binding here — Java, Python, R, and the native
CLI — runs this one engine.

> **Status: released and in production.** The core parser, the native CLI, and the Java, Python,
> and R bindings are complete, cross-validated, and published at 0.2.0 (R excepted — not yet on
> CRAN). The ChecklistBank backend has completed its cutover and now parses through this engine.

## Why

The Java parser implementation ended at `4.2.0`; this engine took its place, and the Java library
is now a thin FFM binding over it. Three motivations:

- **Polyglot reach** — usable outside the JVM (Java, a native CLI, Python, and R).
- **Throughput** — faster batch parsing of multi-million-name corpora.
- **Robustness** — a linear-time regex engine structurally eliminates the catastrophic-backtracking
  (ReDoS) tail the Java parser hand-fights with ~20 possessive quantifiers.

## Validated parity & performance

Measured against `org.gbif:name-parser` 4.2.0 — the last Java implementation release, kept as the
regression oracle every binding is validated against — on the same machine:

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
| Java (Panama/FFM) | `bindings/java` | Complete & parity-validated; a classes JAR plus one cdylib JAR per platform (`linux-x86_64`, `linux-aarch_64`, `osx-x86_64`, `osx-aarch_64`, `windows-x86_64`), picked by `${os.detected.classifier}` — **published to GBIF Nexus** (`org.gbif.nameparser:name-parser-rust:0.2.0`; snapshots auto-deploy on every push to `main`), see [`DISTRIBUTION.md`](DISTRIBUTION.md) |
| Python (PyO3) | `crates/nameparser-py` | Complete & parity-validated (11,302/11,302 vs the Java oracle); **published to [PyPI](https://pypi.org/project/gbif-name-parser/)** (`0.2.0`) — `pip install gbif-name-parser` |
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

`gbif/name-parser` is API-only as of `5.0.0`: it publishes a single artifact,
`org.gbif:name-parser-api`, holding the `ParsedName` model, the formatter, and the
`NameParser`/`ParseResult` contract. The Java parser implementation, `org.gbif:name-parser`,
stopped at `4.2.0` and is no longer developed.

This repo implements that contract. `org.gbif.nameparser.rust.NameParserRust` implements
`org.gbif.nameparser.api.NameParser`, so a consumer swaps `new NameParserImpl()` →
`new NameParserRust()` and changes nothing else.

`4.2.0` stays in the loop as a frozen regression oracle — the cross-validation corpora are diffed
against it on every release, and the one known residual is documented in
[`cross-validation.md`](cross-validation.md) — but it is a record of the behaviour being preserved,
not the source of truth.

## License

Apache-2.0, matching `org.gbif:name-parser`.
