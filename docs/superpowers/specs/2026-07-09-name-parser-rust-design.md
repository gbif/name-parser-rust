# Name Parser — Rust core, single source of truth

**Date:** 2026-07-09
**Status:** Approved design; implementation not started
**Related:** `github.com/gbif/name-parser` (current Java implementation, v4.2.0)

## 1. Context & motivation

The GBIF Name Parser is a mature Java library that parses scientific names into a
structured `ParsedName` model. The core (`pipeline` + `token` packages) is ~5,400 lines
of regex-heavy logic — **146 compiled patterns**, ~100 documented edge-case behaviours
(catalogued in the repo `CLAUDE.md`), validated by ~1,925 fluent test assertions and
3,290 lines of corpora. It parses at ~28 µs/name (warmed), is stateless and thread-safe.

Three motivations drive a move to a native Rust core (confirmed with the maintainer):

1. **Polyglot reach** — make the parser usable outside the JVM (Python, R, CLI) for the
   wider biodiversity community, the way GNA's `gnparser` (Go) is.
2. **Performance / throughput** — faster batch parsing of multi-million-name corpora.
3. **ReDoS / robustness** — the codebase carries **~20 possessive quantifiers** (14 in
   Preflight, 6 in StripAndStash) hand-fighting Java's backtracking regex engine;
   comments repeatedly flag catastrophic-backtracking hazards and "the parser has no
   execution timeout" (observed Max latency 3.3 ms). A linear-time engine ends this
   structurally.

## 2. Goals & non-goals

**Goals**
- A single authoritative parser implementation in Rust.
- Byte-for-byte behavioural parity with the current Java parser before any cutover.
- Bindings for Java (Panama/FFM), Python (PyO3), R (extendr), and a native CLI.
- Eliminate the backtracking/ReDoS tail.

**Non-goals**
- JS/WASM/browser binding (explicitly deferred — not a current driver).
- Changing parsing *behaviour* (bug fixes are tracked as explicit divergences, not
  silent changes).
- Single static-binary distribution as a primary objective (falls out of the CLI, but
  isn't a driver).

## 3. End-state

One parser. The Rust core is authoritative; Java, Python, R, and the CLI are bindings.
The Java `pipeline` package is **deleted**; `org.gbif:name-parser` becomes an FFM binding
plus the `ParsedName` model, preserving the `NameParser` interface so the ChecklistBank
backend swaps `new NameParserImpl()` → `new NameParserRust()` with no other change. This
happens only *after* the Rust core reproduces current Java output on the full corpus.

## 4. Scoping decisions (confirmed)

| # | Decision | Choice |
|---|---|---|
| Direction | End-state | **B — single source of truth** (Rust authoritative; Java becomes a binding) |
| Java binding | In-process path | **Panama/FFM** (`java.lang.foreign`); backend moves to Java 22+ |
| Reach | Non-JVM bindings | **Python (PyO3), native CLI, R (extendr)**. Not JS/WASM. |
| Language | Rust vs Go | **Rust** — no runtime to embed (clean C ABI), `fancy-regex` escape hatch, best PyO3/extendr story |
| 1 | Repo | **Separate repo** (`name-parser-rust`, Cargo workspace) — not vendored into the Java monorepo |
| 2 | Java binding location | Lives in the **existing Java `name-parser` repo**, consuming the published cdylib; keeps `org.gbif:name-parser` coordinates + Maven release flow |
| 3 | FFI marshalling | **Flat fixed-layout struct** in a shared `MemorySegment`; FlatBuffers only as fallback; JSON rejected for the hot path |
| 4 | Faithfulness | Port **both** the golden corpus *and* the ~1,925 unit assertions |
| 5 | Hard regexes | **`fancy-regex`** for the irreducible lookarounds/backrefs, isolated and audited |

## 5. Performance model (the spike must replace estimates with measurements)

The FFI *call* is cheap; the *result* is where the in-process number is won or lost.

Single name, Java→Rust via FFM (estimates, ~30-char name):

| Step | Cost | Note |
|---|---|---|
| FFM downcall | ~5–20 ns | Negligible |
| Encode input → native UTF-8 | ~50–150 ns | Near-zero with a reused thread-local segment |
| Rust parse | ~6–12 µs | vs ~25–28 µs warmed Java; est. ~3×, wide error bars |
| Marshal result (flat struct) | 0.3–1 µs | JSON would be 1–5 µs — rejected |
| Build Java `ParsedName` | ~2–8 µs | **The floor** — unavoidable while returning a full Java object |

**The Java-object floor** caps the in-process speedup: Rust makes the *parse* ~3× cheaper
but cannot remove the Java-side object construction (already inside today's 28 µs). Net:

| Use case | Today | Rust | Notes |
|---|---|---|---|
| In-process single name (FFM) | 28 µs | ~10–14 µs (**~2–2.5×**) | Boundary is <1 µs; capped by the object floor |
| Batch — CLI / Python / R | 34 µs/name × 1 core | ~6–10 µs/name × N cores | No object floor, no GC, trivially multicore; 6.3M names: ~3.5 min → tens of s |
| Tail (p99 / max) | Max 3.3 ms | collapses toward p95 | Linear-time regex eliminates pathological cases |

Honest summary: **in-process ≈ 2×** (and parsing likely isn't the backend bottleneck);
the compelling wins are **batch throughput** (which is also the reach goal — one
investment) and **killing the 3.3 ms tail** (the ReDoS goal made concrete). All three
motivations converge on the same native-core-with-flat-ABI design.

Only if the spike shows the object-build fraction is large is a **lazy segment-backed
`ParsedName` façade** (fields decoded on access) worth prototyping to get under the floor.

## 6. Architecture

### 6.1 Repo & crate layout

```
name-parser-rust/
├── crates/
│   ├── nameparser/          # THE core — pure Rust, zero FFI. All logic here.
│   │   ├── src/model.rs       # ParsedName, Authorship, Rank, NomCode, NameType, State, Warnings
│   │   ├── src/regexes.rs     # all 146 patterns, compiled once (LazyLock)
│   │   ├── src/pipeline/      # preflight → strip → tokenizer → authorship_split
│   │   │                      #   → name_tokens → authorship → code_inference → assemble
│   │   ├── src/format.rs      # NameFormatter port
│   │   └── tests/             # golden-corpus diff + ported unit assertions
│   ├── nameparser-ffi/      # C-ABI cdylib (flat struct) for Java FFM
│   ├── nameparser-cli/      # native CLI (clap): parse / compare / benchmark / validate
│   ├── nameparser-py/       # PyO3 → wheel (maturin)
│   └── nameparser-r/        # extendr → R package
└── testdata/               # frozen oracle: inputs + expected JSONL from the Java parser
```

The core crate has **no FFI concerns**, so Python/R/CLI bind it *natively* (no marshalling
floor). Only the Java path pays the C-ABI cost.

### 6.2 The core port

- **Model**: Rust enums map 1:1 to `Rank`/`NomCode`/`NameType`/`State`; `Option<String>`
  for nullable fields, `Vec<String>` for author lists + warnings. `#[derive(Serialize)]`
  for JSONL output only — not for the FFI boundary.
- **Pipeline**: the same eight ordered stages over a shared mutable `ParseContext` — a
  near-mechanical structural port, since the Java stages are already small and
  single-responsibility. The load-bearing ordering of `StripAndStash.run` steps and the
  `Assemble.finish` / `CodeInference.infer` invariants are preserved.
- **Two-phase authorship** (`parseAuthorship` via a synthetic `"Abies alba <auth>"`, and a
  separately-supplied authorship argument merged onto the name) is preserved.

### 6.3 The FFM boundary

A flat, versioned C ABI — no heap ownership crosses the boundary:

```
np_parse(input_ptr, input_len, code, rank, out_ptr, out_cap) -> status
```

Rust writes the result into a caller-owned `MemorySegment` as a fixed-layout struct:
scalar fields at known offsets (enums as `i32`, flags as `u8`, year as `i32`) + a string
table (offset/len pairs into a trailing byte region). Java reads via `VarHandle`s and
builds the `ParsedName`. One downcall + local reads → sub-µs boundary. A layout-version
field prevents silent Java/Rust desync across releases. Unparsable names return a status
code + `NameType`/`NomCode`, which the façade turns back into `UnparsableNameException`.

Java side (in the existing Java repo): `NameParserRust implements NameParser`, holding the
FFM downcall handle + a thread-local scratch segment, with the per-platform cdylib bundled
in the jar and extract-and-loaded (the netty / sqlite-jdbc pattern).

### 6.4 Regex porting strategy

- 146 patterns → `regex` crate, compiled once via `LazyLock`.
- The **~20 possessive quantifiers** (14 in Preflight, 6 in StripAndStash) are
  dropped (a linear engine doesn't need them).
- The **~25 lookarounds + 4 backreferences**: restructure simple boundary assertions into
  match-context checks in code; for the genuinely irreducible ones use **`fancy-regex`**,
  each isolated in `regexes.rs` and doc-commented with *why* it can't be linearized and a
  note that its input is length-bounded (no ReDoS reintroduced). The `MAX_LENGTH` (1000-char)
  cap stays.
- Inline flags `(?i)`, `(?i:…)`, `(?-i:…)` and Unicode classes `\p{Lu}`/`\p{Ll}` are
  supported by both crates. Java↔Rust Unicode edge differences (`\b`, specific case-folds)
  are caught by the golden corpus.

### 6.5 Bindings — CLI / Python / R

All bind the core crate **natively** (no C-ABI, no floor):
- **CLI** (clap): mirrors `parse` / `compare` / `benchmark` / `validate`; same JSONL/CSV/TSV
  output, so it drops into existing pipelines and doubles as the cross-impl validator.
- **Python** (PyO3 + maturin): `parse(name, code=None, rank=None)` → dataclass, plus batched
  `parse_all(iter)`; wheels via cibuildwheel (mac/linux/win).
- **R** (extendr): vectorized `parse_name()` over a character vector → data.frame.

## 7. Faithfulness methodology (the golden-corpus method)

The safety mechanism for retiring a battle-tested parser:

1. **Freeze the oracle**: run today's Java CLI over the full corpus — the ~8k benchmark set,
   the 3,290 lines of test corpora, and a large `col-names.tsv` slice (~1M rows) — into a
   canonical `expected.jsonl`.
2. **Port stage by stage**, each diffed against the oracle (byte-for-byte, canonicalized
   field order). Tokenizer first (most self-contained), then outward.
3. **Port the ~1,925 fluent assertions** — they localize *intent* a corpus diff may not.
   `NameAssertion` → a Rust macro; mostly mechanical.
4. **CI gate**: Rust CI fails on any corpus diff. When Java behaviour changes during the
   parallel window, regenerate the oracle → Rust must catch up.
5. **Divergence log**: where a Java bug would be *fixed* in the port, record it explicitly
   rather than letting it hide in the diff.

The existing `compare` command already diffs two JSONL files, so Rust-CLI-output vs
Java-CLI-output validation is free tooling that already exists.

## 8. Phased roadmap

| Phase | Deliverable | Gate |
|---|---|---|
| **0 — Spike** | Workspace; port tokenizer + strip + 1 stage; golden harness; JMH FFM micro-bench of the flat struct | Real numbers: raw speedup, object-build fraction, lookaround pain → **confirm B** |
| **1 — Faithful core** | Whole pipeline + formatter, pure Rust | 100% golden corpus + ported assertions |
| **2 — CLI + cross-validate** | Native CLI; `compare` in CI over 1M+ rows | Declared parity vs Java |
| **3 — Java FFM binding** | `nameparser-ffi` cdylib + `NameParserRust`; shadow-run in a non-prod import | Zero diffs on real data; ≥2× in JMH |
| **4 — Python + R** | PyO3 wheel + extendr package published | Parity-tested |
| **5 — Cutover** | Backend defaults to Rust; delete the Java `pipeline` | Single source of truth reached |

## 9. Risks & mitigations

| Risk | Mitigation |
|---|---|
| Behavioural divergence (the #1 risk) | Golden corpus + ported assertions + shadow runs; short parallel window |
| Java↔Rust Unicode/regex edge differences | Corpus catches them; document |
| Native artifact distribution (cdylib in jar, wheels, R binaries) | maturin / cibuildwheel / extract-and-load — solved patterns, real ops |
| Java 22 dependency for FFM | Interim: subprocess over the existing JSONL CLI until the backend moves |
| `fancy-regex` reintroducing backtracking | Isolate, bound input length, audit each use; `MAX_LENGTH` stays |
| Dual maintenance during parallel period | Drive hard to Phase 5; every Java change mirrored via the oracle |
| Loss of CLAUDE.md domain knowledge | Port the ~100 behaviours into Rust doc-comments + tests |

## 10. Success criteria

- Rust core passes 100% of the golden corpus + ported unit assertions.
- FFM path measured ≥2× faster single-name, sub-µs boundary, in JMH.
- CLI batch ≥3× faster single-core, scales with cores.
- p99/max tail collapses (no >100 µs pathological cases on the corpus).
- Python wheel + R package published and parity-tested.
- Backend runs `NameParserRust` in shadow mode with zero diffs on a full import before cutover.

## 11. To validate in the spike (Phase 0)

- Actual Rust-vs-Java raw parse speedup on a representative sample.
- The object-build fraction of today's 28 µs (the in-process ceiling).
- Real FFM boundary cost for the flat-struct layout (JMH).
- How painful the ~29 lookaround/backref patterns are in practice (restructure vs `fancy-regex`).
- Any Java↔Rust regex/Unicode semantic gaps surfaced by the first stages' corpus diff.
