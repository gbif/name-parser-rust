# Benchmarks

Results from a MacBook Pro, Apple M4 Pro, 48 GB — the same machine as the Java repo's own
`benchmarks.md`. Rust built with `cargo build --release` (rustc/cargo 1.97.0); Java run against
the shaded `name-parser-cli` jar with Liberica OpenJDK 25.0.3. Same 8017-name corpus both sides:
`testdata/benchmark-data.txt` here is byte-identical (modulo the header comment line) to
`name-parser/name-parser-cli/data/benchmark-data.txt`.

## 0.0.0 (2026-07-10) — full-pipeline Rust vs Java

The Phase 0 spike measured only *components* (tokenize 0.3215 µs/name + a 4-pattern regex batch
0.1447 µs/name, ~0.47 µs/name combined — under 2% of Java's 28 µs/name full-parse reference) and
explicitly deferred a real full-pipeline comparison until the port was complete. This is that
comparison, run back-to-back on the same machine against the same corpus.

> `nameparser-cli benchmark --warmup --input=testdata/benchmark-data.txt`

```
Warming up — parsing the first 100 names without timing…
Parsed names: 8017 (3345 unparsable)
Total:   110.08 ms
Average: 13.73 µs
Min:     208 ns
p50:     14.46 µs
p95:     32.29 µs
Max:     959.46 µs

Breakdown by name type:
  SCIENTIFIC           4532
  OTHER                3258
  INFORMAL             143
  FORMULA              42
  PLACEHOLDER          42
```

> `java -jar name-parser-cli-4.2.0-SNAPSHOT-shaded.jar benchmark --warmup --input=data/benchmark-data.txt`

```
Warming up the JIT — parsing the first 100 names without timing…
Parsed names: 8017 (3345 unparsable)
Total:   230.61 ms
Average: 28.77 µs
Min:     750 ns
p50:     24.79 µs
p95:     78.50 µs
Max:     3.47 ms

Breakdown by name type:
  SCIENTIFIC           4532
  OTHER                3258
  INFORMAL             143
  FORMULA              42
  PLACEHOLDER          42
```

**Parity check:** count, unparsable count, and the full by-`NameType` breakdown are identical
between the two runs — and match the Java repo's own recorded 4.2.0-SNAPSHOT entry exactly. The
speed difference below is not a shortcut/partial parse; both sides did the same classification
work on the same 8017 names.

**Ratio (Java ÷ Rust — how much faster the Rust port is):**

| Stat    | Rust      | Java      | Ratio (Java/Rust) |
|---------|-----------|-----------|--------------------|
| Total   | 110.08 ms | 230.61 ms | 2.10x              |
| Average | 13.73 µs  | 28.77 µs  | 2.10x              |
| p50     | 14.46 µs  | 24.79 µs  | 1.71x              |
| p95     | 32.29 µs  | 78.50 µs  | 2.43x              |

Repeated 3x per side back-to-back to check stability (every run: identical 8017/3345 count and
identical breakdown; figures below are avg / p50 / p95, in µs):

| Run | Rust                  | Java                  |
|-----|-----------------------|------------------------|
| 1   | 13.73 / 14.46 / 32.29 | 28.77 / 24.79 / 78.50 |
| 2   | 13.95 / 15.17 / 32.33 | 29.56 / 25.00 / 82.17 |
| 3   | 12.95 / 14.12 / 30.00 | 29.49 / 24.54 / 82.17 |

The ratio holds consistently — roughly 1.7x at p50 up to ~2.6x at p95 — across repeats. **The
full Rust pipeline parses at roughly double Java's throughput** on this corpus and machine. This
resolves the question the Phase 0 spike deferred: the component-level headroom the spike found
(tokenize + a regex batch together used under 2% of Java's 28 µs/name budget) carried through
the rest of the pipeline — the ported grammar/state-machine classification and object
construction stages the spike couldn't measure did not erase that lead, and the two full-parser
averages (Rust 13.73 µs, Java 28.77 µs) land close to the spike's Java reference figure (28
µs/name), confirming that reference was itself accurate.

### Notes

- `--warmup` on the Rust side is a no-op in substance (Rust has no JIT to warm up) but is kept,
  and pays the same fixed 100-name pre-pass cost, so the two invocations stay directly
  comparable command-for-command.
- Both binaries were run directly from the shell (not through `cargo run`/`java -jar` wrapped in
  any additional harness), immediately after a fresh `--release` build (Rust) / an already-built
  shaded jar (Java), with no other significant load on the machine.
- The 1M+/6.3M-row `col-names.tsv` comparison from the Java repo's own `benchmarks.md` (its `V4`
  entries) is not reproduced here — that corpus isn't present in this repo yet (see the Phase 2
  plan's "Data note"); this entry only covers the 8017-row `benchmark-data.txt` corpus both
  repos ship.
