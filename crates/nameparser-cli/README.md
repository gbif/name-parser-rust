# nameparser-cli

Native command-line tools for the GBIF scientific-name parser — a Rust port of the Java
`name-parser-cli`. Parse and **standardize** names (JSON out), benchmark throughput, diff two parse
runs, and (research) validate a corpus with an LLM judge. Part of
[`name-parser-rust`](../../README.md).

## Install

Prebuilt binaries are attached to each `cli-v*`
[GitHub release](https://github.com/gbif/name-parser-rust/releases); or build from source:

```sh
cargo build --release -p nameparser-cli   # -> target/release/nameparser-cli
```

Run `nameparser-cli --help`, or `nameparser-cli <command> --help` for a subcommand's full options.

## `parse` — names → JSON

Reads a name-per-line input (a plain list, or the first tab-column of a TSV; blank and `#` lines are
skipped) and writes one JSON object per line.

```sh
printf 'Abies alba Mill.\nRhizobium sp. RMCC TR1811\nBOLD:AAA0001\n' | nameparser-cli parse
```
```jsonl
{"line":1,"input":"Abies alba Mill.","parsed":{"rank":"SPECIES","genus":"Abies","specificEpithet":"alba","type":"SCIENTIFIC", ...}}
{"line":2,"input":"Rhizobium sp. RMCC TR1811","parsed":{"genus":"Rhizobium","phrase":"sp. RMCC TR1811","type":"INFORMAL", ...}}
{"line":3,"input":"BOLD:AAA0001","error":{"type":"IDENTIFIER","message":"Unparsable IDENTIFIER name: BOLD:AAA0001"}}
```

Every row is `{"line", "input", <outcome>}`:

| Outcome key | When | Notable fields |
|---|---|---|
| `parsed` | a name the parser can structure | the full `ParsedName`; `type` = `SCIENTIFIC` / `INFORMAL` / … |
| `error`  | not a scientific name | `type` = `FORMULA` / `PLACEHOLDER` / `IDENTIFIER` / `OTHER` (+ optional `code`) |

`--input <FILE>` / `--output <FILE>` default to stdin/stdout (`-` also means stdin/stdout);
`--quiet` silences the per-batch progress printed to stderr.

### `--canonical` — standardize names

Adds a top-level `canonical` field (right after `input`) with the cleaned, standardized name — the
common "normalise my list of names" job. It's emitted for every row: a parsed name is reformatted,
an unparsable one gets its canonicalised input (e.g. an uppercased identifier).

```sh
printf 'Abies    alba   Mill.\nBetula pendula ROTH\nxAgropogon littoralis\nsh1957732.10fu\n' \
  | nameparser-cli parse --canonical
```
| input | `canonical` |
|---|---|
| `Abies    alba   Mill.` | `Abies alba Mill.` |
| `Betula pendula ROTH` | `Betula pendula Roth` |
| `xAgropogon littoralis` | `× Agropogon littoralis` |
| `sh1957732.10fu` | `SH1957732.10FU` |

Add `--no-authorship` to drop the author from the field (`Abies alba` instead of `Abies alba Mill.`).

### `--three-way` — the 5.0.0 informal result

Splits semistructured names out into their own `informal` object (a flat `taxon` + `rank` +
`phrase`, plus a `canonical` rendering) instead of folding them into a `parsed` row with
`type=INFORMAL`. The default, flag-less `parse` output is the language bindings' parity oracle and
is deliberately left untouched by both `--three-way` and `--canonical`.

## `benchmark` — throughput

```sh
nameparser-cli benchmark --input testdata/benchmark-data.txt
```
Reports names/sec over a name-per-line file. `--warmup` runs an untimed warm-up pass first (kept for
parity with the Java benchmark; Rust has no JIT to warm).

## `compare` — diff two parse runs

```sh
nameparser-cli parse --input=a.txt --output=a.jsonl
nameparser-cli parse --input=b.txt --output=b.jsonl   # same source + line order as a.txt
nameparser-cli compare a.jsonl b.jsonl
```
Lockstep, row-by-row diff of two `parse` JSONL files — reports the rows that differ, the status
transitions (parsed ↔ informal ↔ unparsable), and the top differing fields. `--ignore-whitespace`
ignores formatting-only spacing; `--max-diffs <N>` caps the per-row dump (the aggregate counts are
never capped). Used to cross-validate the Rust parser against the Java oracle at corpus scale.

## `validate` — LLM-judged spot-check (research)

Samples a corpus's "suspicious tail" (errors / warnings / non-`SCIENTIFIC` results), asks an LLM to
judge each parse, and writes a JSONL report — a way to surface likely parser bugs at scale. Needs a
provider: `--provider anthropic` (uses `ANTHROPIC_API_KEY`) or `--provider ollama` / `local` for an
OpenAI-compatible local server. `--dry-run` selects and batches names without making any LLM call.

```sh
nameparser-cli validate --input corpus.tsv --budget 2000 --dry-run
```
