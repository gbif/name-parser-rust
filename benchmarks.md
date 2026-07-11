# Benchmarks & parity

A precision comparison of every way the GBIF name parser can be run — the legacy 3.x-generation
Java parser, the 4.x Java rewrite, the native Rust CLI, and the Rust core reached in-process from
Java over Panama/FFM — plus the correctness parity that underwrites all of it.

All figures are from **one MacBook Pro (Apple M4 Pro, 48 GB)**. Rust: `cargo build --release`
(rustc/cargo 1.97.0). Java: the shaded `name-parser-cli` jar (and, for the JMH runs,
`NameParserImpl` on the classpath) on **Liberica OpenJDK 25.0.3**. The historical Java rows come
from the Java repo's own `benchmarks.md` (JDK 17, same machine).

## TL;DR

Same 8,017-name corpus (`benchmark-data.txt`) for every row; batch, out-of-process, warmed:

| Parser | avg µs/name | worst single name | vs 4.2.0 |
|---|---:|---:|---:|
| Java 3.x generation ("DEV") | 100.75 | **14.42 ms** | 0.29× |
| Java 4.x rewrite (V4, 2026-05-16) | 37.19 | 3.13 ms | 0.76× |
| Java 4.2.0-SNAPSHOT (current) | 28.77 | 3.47 ms | 1.00× (baseline) |
| **Rust core (this repo), native CLI** | **13.73** | **0.96 ms** | **2.10×** |

- **~2.1× faster** than today's Java parser in batch, **~7× faster** than the 3.x generation still
  deployed in the CoL backend.
- **The tail collapses.** Linear-time regex means no catastrophic backtracking: the worst
  single-name parse is ~1 ms (0.96 ms on the 8k corpus, 1.17 ms across 6.4M names), versus the
  3.x parser's **1,010 ms** (a full second) on one name in the 6.3M col-names dump. This is the
  ReDoS-robustness goal, made concrete.
- **Zero behavioural difference.** Byte-for-byte parity with Java at every scale tested — 30/30
  fields in-harness, 11,302 names in-process via FFM (both wire formats), and **6,416,452** real
  Catalogue-of-Life names via the CLI — all 0 diffs (§5).

## Two measurement contexts (read this before comparing numbers)

Absolute µs/name are **only comparable within a context**, never across:

1. **Batch, out-of-process (CLI)** — §1, §2. Each parser is a separate process reading a file and
   timing only the `parse()` call (no output serialization inside the timed region). This is the
   throughput a pipeline actually sees. No cross-language object construction.
2. **In-process (JMH)** — §3. Java calls the Rust core over FFM and rebuilds a Java `ParsedName`.
   JMH warms the JIT hard and measures a fixed 2,000-name subset. This includes the FFM boundary
   and the Java-object-build floor, so its absolute numbers run *lower* than §1 (heavier warmup,
   no I/O, smaller subset) — do **not** read the §3 "9.29 µs" as faster than the §1 "13.73 µs";
   they measure different things. Within each context, the parser-to-parser **ratios** are valid.

---

## 1. Batch throughput — native CLI, 8,017-name corpus

`benchmark-data.txt` here is byte-identical (modulo a header comment) to the Java repo's
`data/benchmark-data.txt`. Both parsers classify the same 8,017 names into the same
by-`NameType` breakdown — this is full parsing, not a shortcut.

**Java parser across generations** (from the Java repo's `benchmarks.md`; `benchmark --warmup`):

| Version (date) | avg | p50 | p95 | Max | note |
|---|---:|---:|---:|---:|---|
| DEV — 3.x generation | 100.75 µs | 86.29 µs | 182.88 µs | **14.42 ms** | pre-rewrite; `NO_NAME`/`OTU`/`HYBRID_FORMULA` types — the lineage deployed as 3.16.0 |
| V4 (earlier) | 23.91 µs | 19.08 µs | 69.25 µs | 1.72 ms | rewrite in progress, partial feature set |
| V4 (2026-05-16) | 37.19 µs | 38.50 µs | 91.33 µs | 3.13 ms | full feature set |
| 4.2.0-SNAPSHOT (2026-07-05) | 28.25 µs | 24.96 µs | 74.50 µs | 3.30 ms | current `NameParserImpl` |

**Rust vs Java 4.2.0, back-to-back on this machine:**

```
# Rust:  nameparser-cli benchmark --warmup --input=testdata/benchmark-data.txt
Parsed names: 8017 (3345 unparsable)
Average: 13.73 µs   Min: 208 ns   p50: 14.46 µs   p95: 32.29 µs   Max: 959.46 µs

# Java:  java -jar name-parser-cli-4.2.0-SNAPSHOT-shaded.jar benchmark --warmup --input=data/benchmark-data.txt
Parsed names: 8017 (3345 unparsable)
Average: 28.77 µs   Min: 750 ns   p50: 24.79 µs   p95: 78.50 µs   Max: 3.47 ms
```

Identical count / unparsable count / by-`NameType` breakdown both sides — same work, on the same
names.

| Stat | Rust | Java 4.2.0 | Ratio (Java/Rust) |
|---|---:|---:|---:|
| Average | 13.73 µs | 28.77 µs | **2.10×** |
| p50 | 14.46 µs | 24.79 µs | 1.71× |
| p95 | 32.29 µs | 78.50 µs | 2.43× |
| Max | 0.96 ms | 3.47 ms | 3.62× |

Stable across 3 back-to-back repeats per side (avg / p50 / p95, µs; every run identical
8017/3345 count + breakdown):

| Run | Rust | Java |
|-----|-----------------------|------------------------|
| 1 | 13.73 / 14.46 / 32.29 | 28.77 / 24.79 / 78.50 |
| 2 | 13.95 / 15.17 / 32.33 | 29.56 / 25.00 / 82.17 |
| 3 | 12.95 / 14.12 / 30.00 | 29.49 / 24.54 / 82.17 |

## 2. Large corpus (millions of names) & the ReDoS tail

The two large runs are from **different CoL releases**, but both are now real-names-only
(BOLD/SH-OTU excluded) at ~6.3–6.4M — so the averages are directly indicative, and the **tail**
(worst single name) is the robust cross-corpus claim (it turns on whether backtracking-prone
inputs exist at all, and both corpora are similar-composition real names).

**Java — `col-names.txt`, 6,259,108 names** (Java repo `benchmarks.md`):

| Version | avg | p50 | p95 | Max |
|---|---:|---:|---:|---:|
| DEV — 3.x generation | 125.88 µs | 97.83 µs | 150.13 µs | **1,010.29 ms** |
| V4 (earlier) | 19.79 µs | 18.54 µs | 29.46 µs | 2.62 ms |
| V4 (2026-05-16) | 34.00 µs | 32.50 µs | 47.21 µs | 5.81 ms |

**Rust — `colxr26.6-names.tsv`, 6,416,452 names** (real names only — BOLD specimen IDs and UNITE
SH fungal OTUs excluded, so its composition mirrors the Java `col-names` set; CoL release 26.6).
Benchmarked on the **name column** (`cut -f1 … | benchmark`) — the `benchmark` command reads a
single-column corpus (whole trimmed line = name), so a raw multi-column TSV must be projected to
its name column first, or it times the parser on `name<TAB>author<TAB>…` junk:

```
Parsed names: 6416452 (9297 unparsable)     # unparsable count matches `parse`/cross-val exactly
Average: 12.21 µs   Min: 333 ns   p50: 11.62 µs   p95: 17.58 µs   Max: 1.17 ms
Breakdown: SCIENTIFIC 6406907 · OTHER 4744 · FORMULA 4522 · INFORMAL 277 · PLACEHOLDER 2
```

Both large corpora are now real-names-only at ~6.3–6.4M, so the averages are directly
**indicative** — Rust **12.21 µs** vs the Java 4.x rewrite's **19.8–34 µs** — though they remain
different CoL releases (26.6 vs an earlier one), not a controlled head-to-head, so no precise
ratio is claimed.

**The tail is the story.** A linear-time engine *cannot* backtrack catastrophically regardless of
input; the numbers bear it out at scale:

| | worst single name |
|---|---:|
| Java 3.x generation (6.3M col-names) | **1,010 ms** |
| Java 4.x rewrite (6.3M col-names) | 2.6–5.8 ms |
| **Rust core (6.4M colxr26.6-names)** | **1.17 ms** |

The 3.x parser — the one the CoL backend still runs — spent a **full second** on a single
pathological name; the Java rewrite fought this down to a few ms with ~20 hand-placed possessive
quantifiers; the Rust port removes the failure mode structurally (≈860× below the 3.x worst case,
≈2–5× below the 4.x worst case, on comparably large real corpora).

## 3. In-process — Java calling Rust via FFM/Panama (JMH)

This is the binding the CoL backend would use: `NameParserRust implements NameParser` downcalls
the Rust cdylib and rebuilds a Java `ParsedName`. Two wire formats were built and A/B'd — a JSON
string vs a flat fixed-layout binary struct. JMH `AverageTime`, µs per 2,000-name pass, 2 forks ×
(5 warmup + 5 measurement) iterations; per-name = score ÷ 2000.

| Arm | µs / 2000-name op | ± 99.9% CI | µs / name | vs javaImpl |
|---|---:|---:|---:|---:|
| `javaImpl` (Java `NameParserImpl`, in-JVM) | 25,590 | ±281 | 12.80 | 1.00× |
| `rustJson` (FFM → Rust → JSON → ParsedName) | 20,850 | ±217 | 10.43 | **1.23×** |
| `rustStruct` (FFM → Rust → flat struct → ParsedName) | 18,576 | ±727 | 9.29 | **1.38×** |

- **The flat struct wins** (~12% over JSON, non-overlapping CIs), vindicating the design's bet
  that a binary layout beats JSON marshalling. `rustStruct`'s wider error bar (±727) means the
  ranking is solid but the exact 12% has some spread.
- **Neither clears 2× in-process** — and that is expected. The in-process speedup is capped by the
  **Java-object-build floor**: Rust makes the *parse* cheaper, but every arm still constructs a
  full Java `ParsedName`, which no wire format can remove. In-process single-name was always the
  least-compelling of the three motivations; the decisive wins are batch throughput (§1) and the
  killed tail (§2), which need no FFM at all.
- Wire-format choice for the eventual cutover (struct's 1.38× vs JSON's far simpler,
  proven-serialization 1.23×) is a speed-vs-maintainability call deferred to Phase 5.

## 4. The bindings landscape

Every path to the parser, with the context that makes its number meaningful:

| Path | Context | µs/name | Worst case | Notes |
|---|---|---:|---:|---|
| Java 3.x `NameParserGBIF` (deployed 3.16.0) | in-JVM batch | ~100 (8k) | ~1,010 ms | thread-pool + timeout; catastrophic tail |
| Java 4.2.0 `NameParserImpl` | in-JVM batch | ~28 (8k) | ~3.3 ms | synchronous rewrite |
| Rust core, **native CLI** | out-of-process batch | **13.7** (8k) | **<1 ms** | no JVM; multi-core & static-binary capable |
| Rust core via **Java FFM (JSON)** | in-process, per name | 10.4* | — | reuses proven serialization; simplest |
| Rust core via **Java FFM (struct)** | in-process, per name | 9.3* | — | fastest in-process; hand-rolled layout |

\* §3 (JMH) context — not comparable to the batch rows above; see "Two measurement contexts".
Python (PyO3) and R (extendr) bindings (Phase 4) will bind the core *natively* like the CLI — no
object floor — so they inherit the §1/§2 batch profile, not the §3 in-process one.

## 5. Correctness parity

Speed is only meaningful because behaviour is identical. The Rust core is diffed against the Java
parser at three scales, every field, **0 differences** everywhere:

| Scale | Method | Result |
|---|---|---|
| 8,017 names, all 30 `ParsedName` fields | in-harness golden diff (`tests/parse_golden.rs`) | **30/30 fields, 0 mismatches** |
| 11,302 names (8k + 6 Java test corpora) | Rust CLI vs Java CLI (`compare`) | **11,302 / 11,302** |
| 11,302 names, in-process, **both wire formats** | `NameParserRust` vs `NameParserImpl` (`ParityTest`) | **11,302 / 11,302** each |
| 6,416,452 names (colxr26.6-names, real names only) | Rust CLI vs Java CLI 4.2.0 | **6,416,452 / 6,416,452** |

The one documented non-difference: Java serializes `ParsedName.warnings` (a `HashSet`) in
hash-bucket order, Rust in insertion order — a raw-byte difference on 5 of 8,017 rows that both
the golden harness and `compare` correctly treat as set-equal (neither order is more canonical).
Detection was verified adversarially (corrupting one field makes `compare` flag exactly that
row). Full detail and the per-corpus table: [`cross-validation.md`](cross-validation.md).

## Caveats

- One machine (M4 Pro, 48 GB); figures are representative, not a controlled multi-host study.
- Historical Java rows (§1 DEV/V4, §2) are from the Java repo's `benchmarks.md` (JDK 17); the
  Rust vs 4.2.0 head-to-head (§1) and the JMH runs (§3) are JDK 25, this repo, this session.
- §2's large corpora are different CoL releases (Java `col-names` 6.3M vs Rust `colxr26.6-names`
  6.4M) — both real-names-only now, so the averages are indicative; the tail is the like-for-like
  robustness claim. The `benchmark` command reads a single-column corpus, so the Rust row is
  measured on the projected name column (`cut -f1`), matching what `parse`/the cross-val see.
- `--warmup` on the Rust CLI is substantively a no-op (no JIT) but pays the same fixed 100-name
  pre-pass so the two commands stay directly comparable.
- The 6.3M `col-names.tsv` used by the Java large-corpus rows is not in this repo; the Rust
  large-corpus row uses `colxr26.6-names.tsv` (git-ignored; see `.gitignore`).
