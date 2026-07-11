# Phase 4c — `validate` LLM-audit command (Rust CLI): status

**Done.** The Java CLI's `validate` command is ported to pure Rust — a new `validate` subcommand
in `crates/nameparser-cli` that streams a name corpus through the parser, samples a bounded,
seeded "suspicious tail" + baseline, sends batches to an LLM that judges whether each `ParsedName`
faithfully represents the raw input, caches verdicts, and writes a JSONL report of flagged rows
for human review. It **complements `compare`**: `compare` proves *Rust ≡ Java*; `validate` asks
*is the parse actually right* — the correctness axis a byte-diff can't see, and exactly where
messy real-world verbatim data exposes genuine parser weaknesses.

## Flow

`select` (single pass: barcode/OTU pre-filter → parse → classify → reservoir-sample into
`chosen`) → chunk by `--batch` → per chunk: cache-lookup each item, one LLM call for the uncached
remainder → reconcile verdicts **by the model's echoed index** → write one JSONL report row per
item → stderr summary (verdict counts + most-flagged-fields histogram).

## Options (mirrors the Java CLI)

`--provider` (`anthropic` | `openai` | `local`/`ollama`), `--model`, `--input`, `--output`
(`validate-report.jsonl`), `--budget` (2000), `--sample-normal` (200), `--batch` (25), `--seed`
(17), `--cache` (`validate-cache.jsonl`; `none` disables persistence), `--api-url`, `--dry-run`.

## Providers

- **`anthropic`** (default model `claude-opus-4-8`) — `ANTHROPIC_API_KEY` (or `ANTHROPIC_AUTH_TOKEN`);
  `/v1/messages` with a JSON-schema-constrained verdict output.
- **`openai` / `local` / `ollama`** (default `qwen2.5:14b-instruct`) — the same OpenAI-compatible
  `/v1/chat/completions` client; `OPENAI_API_KEY` optional; base `http://localhost:11434` (Ollama).
  **`--provider=local` runs entirely free against a local model — no key, no cloud cost.**

HTTP via `ureq` (synchronous, no async runtime); retry (4 attempts, backoff on 429/5xx); a tolerant
response parser that strips `<think>` blocks and salvages complete verdict objects from a truncated
batch; a SHA-256-keyed JSONL verdict cache (content-hashed over prompt-version + model + input +
parse-shape) so re-runs skip already-judged names.

## Reproducibility

Sampling is bit-exact reproducible: a hand-rolled `java.util.Random` (verified against `jshell`
for multiple seeds) drives Algorithm-R reservoirs, so the **same `--seed` selects the same names**
— and the same names Java's tool would. `--dry-run --cache=none` is byte-identical across runs.

## Cost-free `--dry-run`

`--dry-run` does the full selection + batching + report (consulting the cache, like Java, so a
pre-populated cache surfaces its verdicts) and dumps the exact first-batch request payload to
stderr — **no API calls, no cost** — so you can inspect what would be sent before spending anything.

## Running it on the verbatim corpus

```bash
CLI=./target/release/nameparser-cli
# free dry-run first (selection + payload preview, no API):
$CLI validate --input=testdata/clb-verbatim-names.txt --dry-run --cache=none
# free local judging (needs Ollama running):  ollama pull qwen2.5:14b-instruct
$CLI validate --provider=local --input=testdata/clb-verbatim-names.txt --budget=200
# or cloud:  ANTHROPIC_API_KEY=… $CLI validate --provider=anthropic --input=… --budget=200
```
(The current `parse`/`select` reader is single-column plain text — name before the first TAB — so
project a multi-column export to its name column first, as with `benchmark`.)

## Deliberate deferrals (documented, not gaps)

- **ColDP TSV/CSV input auto-detection** — plain-text only for now, matching the existing
  `parse`/`benchmark` readers; a shared follow-up would benefit all three subcommands.
- **The `canonical` prompt-payload field** — the Rust core has no `NameFormatter` yet (a separate
  deferred Phase-1 item); Java itself treats `canonical` as best-effort (silently omitted on
  failure), so the port ships without it. Every other payload field is present.
- **Live-API integration tests** — no test issues a real HTTP request (they use fixtures / fake
  judges / `--dry-run`), so CI is offline and free; a live smoke against local Ollama is manual.

## Also in this slice

The `validate` count outputs use `%,d`-style thousands grouping, matching Java (the final
`ok/suspect/wrong` line and the `judged X/Y (Z from cache)` progress line; the trailing
`(no verdict=N)` is left ungrouped, exactly as Java concatenates it). The `parse` command's
`input` field continues to echo the **trimmed, extracted** name — matching the Java CLI
(`ParseCli` sets `input = row.name()`, a `PlainTextReader.trim()` value), verified byte-for-byte
against the shaded jar on a whitespace-padded input.

## Next

Phase 4b — the R (extendr) binding — remains, to complete the binding set (`rextendr` to be
verified first). Then Phase 5 — the backend cutover.
