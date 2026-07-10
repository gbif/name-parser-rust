# Cross-validation: Rust vs Java parity

Phase 2 Task 3's parity deliverable: the Rust `nameparser-cli parse` output diffed against the
Java `name-parser-cli`'s, field-for-field, using the new `nameparser-cli compare` subcommand
(`crates/nameparser-cli/src/main.rs`), over every corpus available in this checkout.

## Method

For each corpus `testdata/<name>.txt`:

```sh
JAR=$(ls /Users/markus/code/gbif/name-parser/name-parser-cli/target/name-parser-cli-*-shaded.jar | head -1)
java -jar "$JAR" parse --input=testdata/<name>.txt --output=- --format=jsonl > testdata/<name>.java.jsonl 2>/dev/null
./target/release/nameparser-cli parse --input=testdata/<name>.txt --output=- --quiet > testdata/<name>.rust.jsonl
./target/release/nameparser-cli compare testdata/<name>.java.jsonl testdata/<name>.rust.jsonl
```

`scripts/cross-validate.sh` runs exactly this over the default corpus set and prints one summary
block per corpus; the generated `<name>.java.jsonl`/`<name>.rust.jsonl` files are git-ignored
(regenerated every run — see `.gitignore`).

**Corpora:**

- `testdata/benchmark-data.txt` — the 8017-name benchmark corpus this repo already cross-checks
  in-harness (`crates/nameparser/tests/parse_golden.rs`, `crates/nameparser-cli/tests/parse_cli.rs`).
- The 6 test-resource corpora from the Java `name-parser` module
  (`name-parser/name-parser/src/test/resources/*.txt`, copied verbatim into `testdata/`):
  `names-with-authors.txt`, `hybrids.txt`, `other.txt`, `otu.txt`, `placeholder.txt`,
  `viruses.txt`. These are the exact fixtures `NameParserImplTest` iterates (`iterResource(...)`
  calls at lines 2701/2714/2730/2746, and `resourceReader("viruses.txt")` at line 3466) — so this
  cross-validation doubles as a Phase-1 "ported assertions" check for those test classes'
  underlying data, over and above the field-level JSON diffing `compare` does.
- Corpora copies are small (679 B – 82.6 KB, ~84 KB total) and committed to `testdata/` so
  `scripts/cross-validate.sh` is reproducible without a sibling `name-parser` checkout.
- `doubtful.txt`, named in the Phase 2 plan doc's corpus list, does not exist in the Java
  repo's `src/test/resources/` (confirmed by directory listing) — presumably a stale reference;
  not included here.
- **Not covered:** the ~6.3M-row `col-names.tsv` full Catalogue of Life dump — not present in
  this checkout (per the Phase 2 plan's "Data note"); deferred until it's dropped into `testdata/`.

## Per-corpus parity table

| Corpus | Rows compared | Identical | Differing | Cause |
|---|---:|---:|---:|---|
| `benchmark-data.txt` | 8,017 | 8,017 | 0 | — (5 rows carry a `warnings`-order-only raw-byte difference; see below) |
| `names-with-authors.txt` | 14 | 14 | 0 | — |
| `hybrids.txt` | 4 | 4 | 0 | — |
| `other.txt` | 13 | 13 | 0 | — |
| `otu.txt` | 20 | 20 | 0 | — |
| `placeholder.txt` | 8 | 8 | 0 | — |
| `viruses.txt` | 3,226 | 3,226 | 0 | — |
| **Total** | **11,302** | **11,302** | **0** | **100.00% parity** |

Zero `Extra rows in A/B` and zero `Line-number mismatches` on every corpus (both sides read and
skip blank/comment lines identically). **No core-crate bugs were found or fixed by this task** —
the Rust port already had full field-level parity with the Java oracle on every corpus available
in this checkout, including `names-with-authors.txt` (the author-parsing edge cases the task
brief specifically flagged as likely to surface remaining issues) and `viruses.txt` (3226 rows,
by far the largest of the 6 test-resource corpora).

## The one documented residual: `warnings` array order

Raw `diff` of the two JSONL files (before `compare`'s order-insensitive handling) shows exactly
**5 of 8017** rows in `benchmark-data.txt` differing — 0 in every other corpus. All 5 are the
same root cause, and it is not a bug:

- Java's `ParsedName.warnings` is a `HashSet<String>`; this field's iteration order is a
  deterministic function of the strings' hash codes, not of insertion order.
- This crate's `warnings: Vec<String>` (`crates/nameparser/src/model/name.rs`) preserves
  insertion order instead.
- Both orders are individually deterministic (stable across reruns on each side) but do not
  always agree with each other when a name carries 2+ warnings — e.g. line 422,
  `"Hieracium laevigatum Willd. subsp. levigans var. levigans f. platyphyllum Zahn lusus
  scopiforme Schack & Zahn"`: Java emits
  `["Removed: subsp. levigans","Removed: var. levigans","name was quadrinomial"]`, Rust emits
  `["Removed: subsp. levigans","name was quadrinomial","Removed: var. levigans"]` — same 3
  strings, same set, different order. (The other 4 rows: lines 4365/5770 — the same input,
  `"Cerastium arvense ssp. velutinum var. velutinum (Raf.) Britton f."`, appearing twice in the
  corpus — plus lines 5039 and 5234.)

This is a pre-existing, already-documented Phase 1 modeling characteristic (see
`parse_golden.rs`'s `UNORDERED_FIELD_KEYS` and `parse_cli.rs`'s module doc), not something this
task introduced or needed to fix — sorting `warnings` on the Rust side would just trade Java's
hash-bucket order for an arbitrary alphabetical one, with no better a claim to being "the"
canonical order. `compare` treats `warnings` (and, by the same reasoning, `notho`/
`epithetQualifier`) as an order-insensitive set (see `main.rs`'s `UNORDERED_FIELD_KEYS`
constant), matching the golden harness's own parity definition — so all 5 rows correctly report
as **identical**, and the table above shows 0 differing.

**This residual-detection logic was verified, not assumed:** an adversarial check corrupted one
field in a copy of the Rust `names-with-authors.txt` output and re-ran `compare`, which correctly
flagged exactly that one row/field as differing (see the Task 3 report for the transcript) —
confirming `compare`'s 0-differences results above are genuine parity, not a false negative from
a no-op comparison.

## Conclusion

Full parity is declared over every corpus currently available in this checkout: **11,302/11,302
rows identical (100.00%)**, modulo the one documented, non-bug `warnings`-order artifact that
`compare` correctly treats as set-equal. `crates/nameparser/tests/parse_golden.rs` (the golden
harness) remains green throughout — no core-crate changes were needed. The next parity milestone
is the 6.3M-row `col-names.tsv` cross-validation, deferred pending that file's availability.
