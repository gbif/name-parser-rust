# Java `validate` LLM-audit subsystem — recon for Rust port

Source root: `/Users/markus/code/gbif/name-parser/name-parser-cli/`
Java files read in full: `VALIDATE.md`, `ValidateCli.java`, `Reservoir.java`, `BarcodeOtuFilter.java`,
`ParseResult.java`, `Args.java`, `llm/{ValidationPrompt,Judge,Verdicts,Verdict,VerdictCache,AnthropicClient,OpenAiClient}.java`,
`io/{NameInput,NameInputReader,InputDetector,PlainTextReader,ColdpReader}.java`, `Main.java`,
plus `name-parser-api`'s `ParsedName.java`/`NameType.java`/`UnparsableNameException.java`, the module `pom.xml`s,
and all test files (`ValidateCliTest`, `BarcodeOtuFilterTest`, `ReservoirTest`, `AnthropicClientTest`, `OpenAiClientTest`).
Also cross-checked against the existing Rust workspace at `/Users/markus/code/gbif/name-parser-rust/` (`crates/nameparser-cli/src/main.rs`,
`crates/nameparser/src/model/{name,mod,enums}.rs`, all `Cargo.toml`/`Cargo.lock`) to ground the port notes in what already exists.

---

## 1. CLI surface

Dispatch: `Main.java` routes `argv[0]` to one of `parse|compare|benchmark|validate`; `validate` gets `argv[1..]` as `ValidateCli.main(rest)`.

`VALIDATE.md`'s option table and `ValidateCli`'s javadoc + `printUsage()` are **fully consistent** — no discrepancies found. Verbatim option set:

| Option | Default | Meaning |
|---|---|---|
| `--provider=P` | `anthropic` | `anthropic` (cloud Claude) or `openai`/`local`/`ollama` (OpenAI-compatible local server). `local` and `ollama` are both normalized to internal `provider="openai"` (`ValidateCli.java:79-81`) — there is no separate "local" client class, `OpenAiClient` **is** the local/Ollama/LM-Studio/llama.cpp client, just defaulted to a local URL. |
| `--model=ID` | `claude-opus-4-8` (cloud) / `qwen2.5:14b-instruct` (local) | model id, passed straight through, no validation |
| `--input=PATH` | `data/col-names.tsv` (`ValidateCli.DEFAULT_INPUT`) | corpus — plain text (one name/line) or ColDP TSV/CSV, auto-detected |
| `--output=PATH` | `validate-report.jsonl` | JSONL report |
| `--budget=N` | `2000` | max names sent to the LLM |
| `--sample-normal=N` | `200` | of those, ordinary names as baseline; **clamped** `Math.min(sampleNormal, budget)` |
| `--batch=N` | `25` | names per request; **clamped** `Math.max(1, batch)` |
| `--seed=N` | `17` | selection seed (parsed as `int` via `Args.integer`, stored as `long`) |
| `--cache=PATH` | `validate-cache.jsonl` | verdict cache; literal `none` (case-insensitive) disables persistence |
| `--api-url=URL` | `null` | endpoint override. anthropic: overrides `ANTHROPIC_BASE_URL`/`https://api.anthropic.com`, `/v1/messages` appended. openai/local: overrides `OPENAI_BASE_URL`/`http://localhost:11434` (Ollama), `/v1/chat/completions` appended. LM Studio (`:1234`)/llama.cpp (`:8080`) are mentioned in help text only — never auto-detected. |
| `--dry-run` | off | select + build batches, no API calls |
| `-h` / `--help` | — | print usage, exit (0 if explicitly requested, 2 if no args at all — that's `Main`'s behavior, not `ValidateCli`'s own) |

`Args` (`Args.java`) is a bespoke, minimal parser: `--key=value`, bare `--flag` (⇒ `"true"`), `-h`/`--help` aliases to a `help` flag; everything else positional. No space-separated `--key value` form, no type validation beyond `integer()`.

Auth env vars (cloud only): `ANTHROPIC_API_KEY` preferred, else `ANTHROPIC_AUTH_TOKEN` (bearer). Local: none required, `OPENAI_API_KEY` optional.

**Command flow, input file → JSONL report:**
1. Parse args, resolve provider/model/paths.
2. Fail fast (`exit(2)`) if `--input` doesn't exist, with a helpful message pointing at the default path.
3. **Phase 1 — select:** open `NameInputReader` (auto-detects plain text vs ColDP TSV/CSV from the first non-blank/non-`#` line, see §1/io notes below), stream every row, pre-parse-filter barcode/OTU codes (§3), parse the rest, classify each into "interesting" or "ordinary" (§2), reservoir-sample both into a combined, line-sorted `chosen` list bounded by `--budget`. Print a scan summary to stderr.
4. Open `VerdictCache` (or a disabled stand-in).
5. If not `--dry-run`, construct the `Judge` (`AnthropicClient` or `OpenAiClient`) — this is also where a missing credential throws and aborts (unless `--dry-run`).
6. **Phase 2 — judge + report:** iterate `chosen` in `--batch`-sized chunks; per chunk, split into cache-hits (looked up immediately) and `uncached`; if `uncached` non-empty and not dry-run, issue **one** `Judge.judge(...)` call for the whole uncached sub-batch, reconcile verdicts by echoed `index`, write them to the cache; write one JSONL row per chunk item (cached, freshly-judged, or verdict-less) to the (buffered, only-flushed-at-the-end) report writer; log a per-chunk progress line to stderr.
7. Close the cache (flush/close file handle) in a `finally`.
8. Print either the dry-run summary (+ first-batch payload dump) or the full `Summary` (verdict counts + most-flagged-fields histogram) to stderr.

---

## 2. Selection / sampling

**"Suspicious tail" predicate** — `ValidateCli.isInteresting(ParseResult r)`, verbatim:
```java
static boolean isInteresting(ParseResult r) {
  if (r.error != null) return true;
  ParsedName pn = r.parsed;
  if (pn == null) return true;
  if (pn.getWarnings() != null && !pn.getWarnings().isEmpty()) return true;
  if (pn.getState() != null && pn.getState() != ParsedName.State.COMPLETE) return true;
  return pn.getType() != null && pn.getType() != NameType.SCIENTIFIC;
}
```
So "interesting" = unparsable (caught exception) **OR** null parse (defensive) **OR** any warnings present **OR** `state != COMPLETE` (i.e. `PARTIAL` or `NONE`) **OR** `type != SCIENTIFIC` (i.e. `FORMULA`/`INFORMAL`/`PLACEHOLDER`/`OTHER`). Everything else is "ordinary" (only sampled for baseline).

**Reservoir sampling** (`Reservoir<T>`, Algorithm R / Vitter, using `java.util.Random`):
```java
public void offer(T item) {
  seen++;
  if (capacity == 0) return;
  if (items.size() < capacity) {
    items.add(item);
  } else {
    long j = (long) (rng.nextDouble() * seen); // uniform in [0, seen)
    if (j < capacity) items.set((int) j, item);
  }
}
```
Two independent reservoirs, deliberately different seeds:
- `interesting = new Reservoir<>(Math.max(0, budget - sampleNormal), seed)`
- `ordinary = new Reservoir<>(sampleNormal, seed + 1)`

After the single-pass scan: `chosen = interesting.items() + ordinary.items()`, then **sorted by `line` ascending** (`Comparator.comparingLong(r -> r.line)`) — the reservoirs' internal (unordered) retention order is discarded; final selection is in original-file order.

Progress: every `PROGRESS_EVERY = 500_000` scanned rows, `"  scanned %,d…"` to stderr. Final scan-summary line: `Scanned %,d names in %.1fs: %,d excluded (barcode/OTU), %,d interesting, %,d ordinary. Selected %,d for validation (budget %,d).`

Reproducibility: `ReservoirTest` confirms same-seed ⇒ identical sample; different seed (practically) differs. `ValidateCliTest.dryRunIsReproducible` confirms the *whole* select→report pipeline is byte-identical across two runs with the same `--seed` and `--cache=none`.

Bounded memory regardless of corpus size (~6.3M names per the doc comments) — this is the entire point of reservoir sampling here: single pass, no full-corpus buffering.

---

## 3. BarcodeOtuFilter

`BarcodeOtuFilter.java`, verbatim patterns:
```java
private static final Pattern UNITE_SH = Pattern.compile("(?i)^SH\\d{5,}(\\.\\d+)?FU\\b");
private static final Pattern BOLD_BIN = Pattern.compile("(?i)^BOLD:[A-Z]{2,5}\\d+\\b");

public static boolean isBarcodeOtu(String name) {
  if (name == null) return false;
  String s = name.strip();
  return UNITE_SH.matcher(s).find() || BOLD_BIN.matcher(s).find();
}
```
- Case-insensitive (`(?i)`), anchored at **start only** (`^`, no `$`) — a `.find()` match, so trailing content after the pattern doesn't prevent exclusion; input is `.strip()`ped first.
- UNITE SH: `SH` + ≥5 digits + optional `.` + digits + `FU` + word boundary. Matches (from `BarcodeOtuFilterTest`): `SH1957732.10FU`, `SH0864666.10FU`, `  SH1958183.10FU  ` (whitespace stripped), `sh1958183.10fu`.
- BOLD BIN: `BOLD:` + 2–5 uppercase letters + ≥1 digit + word boundary. Matches: `BOLD:AAA0001`, `BOLD:ACR2714`, `bold:aab5053`.
- Negatives confirmed by test: `Abies alba Mill.`, `Vulpes vulpes silaceus Miller, 1907`, `Shorea` (superficially similar prefix, doesn't match digit run), `Boldenaria`, `null`, `""`.

**Where applied:** pre-parse, on the raw input string, inside `ValidateCli.select()`'s scan loop — matched rows are excluded (`sel.excluded++`) **before** ever reaching `parser.parse(...)`.

**Important doc/code discrepancy found:** `BarcodeOtuFilter`'s own class javadoc claims *"the validator additionally skips any parse rejected with `NameType#OTU`, so an OTU code never reaches the model however the parser classifies it."* This is **not true of the actual code**, and `NameType.OTU` **does not exist** in the current `name-parser-api` (`NameType` has only `SCIENTIFIC, FORMULA, INFORMAL, PLACEHOLDER, OTHER` — `OTHER`'s own javadoc says former OTU names like `BOLD:AAB5053`/`SH0864666.10FU` land there now). `ValidateCli.select()`'s actual comment (lines 196-199) says the opposite: *"We deliberately do NOT exclude by NameType.OTHER: OTU codes now fall into OTHER, but so do many genuinely odd unparsable strings that are exactly the tail worth reviewing."* **Ground truth = the regex pre-filter is the only OTU exclusion; anything that slips past it and parses/fails as `OTHER` is intentionally treated as part of the suspicious tail, not filtered.** The Rust port should follow `ValidateCli`'s actual behavior (pre-parse regex only), not the stale javadoc claim.

---

## 4. The prompt (`ValidationPrompt.java`)

`VERSION = "v1"` — bump on any prompt/payload shape change; it's baked into the cache key so old verdicts don't leak across prompt versions.

**SYSTEM** (sent as the `system` field for Anthropic; prepended to a `system`-role message for OpenAI-compatible), full text:
```
You are a meticulous reviewer of scientific-name parsing results.

The GBIF name parser is a deterministic, rule-based parser. It takes a raw
scientific name string and produces a structured ParsedName. Your job is to
judge whether each ParsedName faithfully represents the raw input, according to
the parser's own documented conventions below. You are NOT re-parsing from
scratch and you are NOT imposing your own preferences — you are checking the
parser against its contract.

Be conservative. Only flag a result as 'suspect' or 'wrong' when you can point
to a concrete field and say what it should be and why. When in doubt, answer
'ok'. Formatting/whitespace differences and equally-valid alternatives are NOT
errors. Prefer high precision over high recall — a human reviews every non-ok
verdict, so false alarms waste their time.

Parser conventions you must respect:
- Zoological trinomials default to SUBSPECIES: ICZN uses no rank marker, so
  'Vulpes vulpes silaceus Miller, 1907' is rank SUBSPECIES, not a generic
  INFRASPECIFIC_NAME. Botanical infraspecific names DO require an explicit
  subsp./var./f. marker, so absent a marker they stay INFRASPECIFIC_NAME.
- Code inference signals (priority order): a sanctioning author (e.g. ': Fr.')
  => BOTANICAL; '(BasAuthor) RecombAuthor, year' with an explicit infraspecific
  marker => BOTANICAL (the year is the publication year); any other year on the
  author span => ZOOLOGICAL; a filius (f./fil.) suffix on a non-ex author with
  NO year => BOTANICAL; basionym + combination authors without years => BOTANICAL;
  basionym-only without years => ZOOLOGICAL.
- A year extracted from a stripped 'published in' reference is the publication
  year of the work, is code-NEUTRAL, and must NOT by itself imply ZOOLOGICAL.
- Abbreviation of authors or journals is only a weak hint, never a code signal.
- Taxonomic-concept references (sensu, sec., auct., non/nec, emend., fide, ...)
  belong in taxonomicNote, not in the name.
- Viruses, hybrid formulas, OTU/specimen codes, and placeholders are legitimately
  UNPARSABLE — for an unparsable input, judge whether the reported NameType and
  the fact that it was rejected are appropriate, not that it failed to parse.

For every item you are given, return exactly one verdict object. Echo the item's
'index'. Use verdict 'ok' | 'suspect' | 'wrong' and confidence 'low' | 'med' |
'high'. List only the fields you believe are wrong.
```
(Javadoc notes this encodes conventions from the main repo's `CLAUDE.md` "Authorship conventions" section, so the model is held to the parser's *own* documented contract, not the model's guesswork.)

**OUTPUT_INSTRUCTION** (appended only for the local/OpenAI-compatible path — Anthropic gets a JSON-schema constraint instead, see §6):
```
Respond with ONLY a JSON object, no prose and no markdown fences, of the form:
{"verdicts":[{"index":0,"verdict":"ok|suspect|wrong",
"confidence":"low|med|high","fields":[{"name":"...","parsed":"...",
"expected":"...","reason":"..."}],"note":"..."}]}
Return exactly one verdict per input item and echo its 'index'. Use an empty
'fields' array when the verdict is 'ok'.
```

**User message** (`ValidationPrompt.userMessage(batch)`): one string —
`"Judge each of the following " + batch.size() + " parser results.\n" + <compact JSON array>`. Each array element (`item(index, r)`):
- `index` (int, position in this batch)
- `input` (raw string)
- if parsed: `parsed` = full Gson tree of the `ParsedName` (default Gson reflection, **nulls omitted**, no `serializeNulls()`), plus `canonical` = `pn.canonicalNameComplete()` **only if** non-blank (wrapped in try/catch → silently omitted on any `RuntimeException`)
- if unparsable: `unparsable` = `{type: <NameType name>, message: <exception message>}` (`type` omitted if null)

**Expected response shape** (the verdict schema, both paths agree on the same JSON shape — see §6 for how each backend *enforces* it):
```json
{"verdicts":[
  {"index":0,"verdict":"ok|suspect|wrong","confidence":"low|med|high",
   "fields":[{"name":"...","parsed":"...","expected":"...","reason":"..."}],
   "note":"..."}
]}
```

---

## 5. Judge + Verdicts

`Judge` interface: `List<Verdict> judge(String userMessage, int batchSize) throws IOException, InterruptedException` — one call = one HTTP request = one batch. `batchSize` is used **only** to size `max_tokens` headroom, not for chunking (chunking is entirely `ValidateCli`'s responsibility, before `judge()` is ever called).

**Batch assembly** (in `ValidateCli.main`): `chosen` is split into `--batch`-sized `chunk`s; each `chunk` is split into cache-hits (resolved immediately, no request) and `uncached`; if `uncached` is non-empty and not dry-run, exactly **one** `judge(ValidationPrompt.userMessage(uncached), uncached.size())` call is made for the whole uncached remainder of that chunk (so a fully-cached chunk costs 0 API calls; a partially-cached chunk sends only the gap).

**Reconciliation:** the model's `Verdict.index` is 0-based **within the uncached sub-batch just sent**, not within the original chunk or the whole run. `byIndex = {v.index: v}`; for `i` in `[0, uncached.size())`, missing indices (model dropped/omitted one, e.g. truncated reply) yield `verdicts.put(r, null)` — a `null` verdict, counted as "missing" downstream and **never cached** (so a later run retries it).

**Error/retry handling** (both clients): `MAX_ATTEMPTS = 4`. HTTP 429 or ≥500 ⇒ retry with backoff; anything else non-200 ⇒ throw immediately (`IOException`, body truncated to 500 chars). Backoff: Anthropic honors a `retry-after` header (seconds→ms) if present & positive, else `pow(2, attempt) * 500ms`; OpenAI-compatible always uses the `pow(2, attempt) * 500ms` formula (no `retry-after` support). **Asymmetry to note:** `ValidateCli.main` has no try/catch around `client.judge(...)` — an `AnthropicClient` failure that exhausts retries (or any non-retryable HTTP status) propagates up and **aborts the entire run**. `OpenAiClient` is resilient only to *parseable-200-but-garbage-content* failures (`parseReply` catches `RuntimeException` internally, logs a warning, returns `List.of()` so those names stay unjudged/uncached rather than crashing) — persistent HTTP-level failures still throw out of `judge()` exactly like Anthropic's.

**`Verdicts.parse` tolerant extraction** (the resilience layer, mainly exercised by the local/OpenAI path but shared by both):
1. Strip `<think>…</think>` / `<thinking>…</thinking>` blocks first (regex `(?is)<think(?:ing)?>.*?</think(?:ing)?>`), even if they themselves contain braces.
2. Locate the `"verdicts"` key, then its `[`.
3. Walk element-by-element with a proper brace-depth + string-literal + backslash-escape-aware scanner (`matchObject`) — not a naive first-`{`/last-`}` span. Ignores stray characters/whitespace/commas between elements.
4. A trailing object left unbalanced (model hit `max_tokens` mid-object) is **dropped**, salvaging all complete verdicts emitted before it rather than losing the whole batch.
5. Throws `IllegalStateException` if input is blank, or no `"verdicts"` key/array is found at all.
6. A custom Gson `TypeAdapter` for `Verdict.FieldIssue` coerces **any** JSON shape (object, array, number, boolean, null) in `name`/`parsed`/`expected`/`reason` down to a display string — primitives via `getAsString()`, objects/arrays via compact `.toString()` — defends against local models (e.g. gemma) echoing a whole nested object as a field value.

**`Verdict` data model** (`Verdict.java`), plain Gson-reflected POJO, fields are untyped `String`s (verdict/confidence membership is prompt-contract only, enforced server-side for Anthropic via JSON schema `enum`, **not enforced at all** for local models):
```java
public final class Verdict {
  public int index;
  public String verdict;      // "ok" | "suspect" | "wrong"
  public String confidence;   // "low" | "med" | "high"
  public List<FieldIssue> fields; // empty/null when verdict == ok
  public String note;
  public boolean isOk() { return "ok".equalsIgnoreCase(verdict); }
  public static final class FieldIssue {
    public String name;     // e.g. "rank", "code", "combinationAuthorship.year"
    public String parsed;
    public String expected;
    public String reason;
  }
}
```
**Loose-typing note for the port:** `ValidateCli.Summary.record` buckets *any* verdict string that isn't case-insensitively `"wrong"` or `"suspect"` as `"ok"` — including unexpected/malformed strings from a misbehaving local model. Worth deciding deliberately in Rust (strict enum parse + error, vs. Java's permissive fallback-to-ok).

---

## 6. Providers

### AnthropicClient
- Endpoint: `trimTrailingSlash(baseUrl) + "/v1/messages"`.
- `fromEnv(baseUrl, model)`: `baseUrl` priority = explicit `--api-url` > `ANTHROPIC_BASE_URL` env > `https://api.anthropic.com`.
- Auth: `ANTHROPIC_API_KEY` (header `x-api-key`) preferred; else `ANTHROPIC_AUTH_TOKEN` (header `authorization: Bearer <token>` **plus** `anthropic-beta: oauth-2025-04-20`); else throws `IllegalStateException` with a message suggesting `ant auth print-credentials --access-token` or `--dry-run`.
- Request body (`requestBody`):
  - `model`
  - `max_tokens`: `min(32000, 2000 + batchSize * 400)` (400 = `maxTokensPerName`)
  - `thinking`: `{"type": "adaptive"}` (Anthropic adaptive extended-thinking)
  - `system`: `ValidationPrompt.SYSTEM` (plain string)
  - `messages`: `[{"role":"user","content": userMessage}]` (content is a plain string, not content blocks)
  - `output_config`: `{"format": {"type": "json_schema", "schema": <verdictSchema()>}}` — a fully-specified JSON Schema: root object `{verdicts: array<verdict>}`, `required=[verdicts]`, `additionalProperties:false`; each `verdict` object requires **all** of `index/verdict/confidence/fields/note` (even `note`/`fields`, which can be blank/empty), `additionalProperties:false`, `verdict`/`confidence` are string `enum`s; each `fields[]` issue requires all of `name/parsed/expected/reason` (all typed `string`), `additionalProperties:false`.
- HTTP: JDK `java.net.http.HttpClient`, connect timeout 30s, **request timeout 5 min**, headers `content-type: application/json`, `anthropic-version: 2023-06-01`.
- Response parsing (`parseVerdicts`): response has a `content` array of blocks; concatenate the `text` of every block with `type == "text"` (skips e.g. `thinking` blocks); feed the concatenation into `Verdicts.parse`.
- Default model: `claude-opus-4-8`.

### OpenAiClient (also serves `--provider=local`/`ollama`)
- Endpoint: `trimTrailingSlash(baseUrl) + "/v1/chat/completions"`.
- `fromEnv(baseUrl, model)`: `baseUrl` priority = explicit `--api-url` > `OPENAI_BASE_URL` env > `http://localhost:11434` (Ollama default).
- Auth: `OPENAI_API_KEY` optional — sent as `authorization: Bearer <key>` only if present; local servers generally ignore/don't need it.
- Request body: `model`, `temperature: 0`, `max_tokens` (same formula as Anthropic), `stream: false`, `response_format: {"type":"json_object"}` (weak "must be valid JSON" constraint only, not a schema), `messages: [{role:"system", content: SYSTEM + "\n\n" + OUTPUT_INSTRUCTION}, {role:"user", content: userMessage}]`.
- HTTP: same JDK client, connect timeout 30s, **request timeout 10 min** (longer — "local generation can be slow"), no special headers beyond `content-type`.
- Response parsing: `extractContent` pulls `choices[0].message.content` (throws if absent); `finishReason` pulls `choices[0].finish_reason`, logged as a warning pre-parse if `"length"` (truncated); `parseReply` wraps `Verdicts.parse` in try/catch, degrading to `List.of()` + SLF4J warning on **any** `RuntimeException` or an empty verdict list (see §5's resilience note).
- Default model: `qwen2.5:14b-instruct`. (VALIDATE.md's usage examples illustrate `qwen2.5:32b-instruct` as a *bigger* option, not the coded default.)

Both clients are built from raw JDK `HttpClient` + hand-built Gson JSON trees — **no Anthropic/OpenAI SDK dependency at all** in Java.

---

## 7. VerdictCache

Format: **JSONL**, one object per line: `{"key":"<sha256-hex>","verdict":{...Verdict fields...}}`.

`open(file)`: if it exists, `Files.readAllLines` the whole thing into a `HashMap<String,Verdict>` up front (comment: "verdict records are tiny"); else creates the parent dir. Then opens a `BufferedWriter` in `CREATE`+`APPEND` mode for subsequent writes — old entries are preserved, new ones appended (never rewritten/compacted).

`disabled()` (`--cache=none`): seeded from an empty immutable `Map.of()`, `appender=null`. **Subtlety:** `put()` still unconditionally does `byKey.put(key, verdict)` before checking `appender == null` — so a "disabled" cache is actually a real in-memory `HashMap` for the *duration of one run*; it just isn't loaded from or persisted to disk. A name appearing twice in one corpus (same content-hash) would still hit the in-memory cache on its second occurrence even with `--cache=none`.

**Key** — `VerdictCache.key(String... parts)` = SHA-256 hex digest over each part's UTF-8 bytes, with a single `\0` byte appended after **every** part (including the last). Null parts treated as `""`.

`ValidateCli.cacheKey(model, r)`:
```java
String shape = r.parsed != null ? GSON.toJson(r.parsed)
    : (r.error == null ? "" : GSON.toJson(r.error));
return VerdictCache.key(ValidationPrompt.VERSION, model, r.input, shape);
```
Four hashed parts, in order: **prompt `VERSION`** ("v1"), **model id**, **raw input string**, **"shape"** = full Gson JSON of the parsed output (or error, or `""`). **Line number and record `id` are NOT part of the key** — identical (name, parse-output) pairs anywhere in the corpus (or across separate runs/files) share one cache entry; this is deliberate (re-runs and budget bumps don't re-judge already-judged content).

Cache hits skip API calls entirely — checked *before* the uncached list is built, per §5. Writes happen immediately per judged item, with an **immediate `flush()` after every `put`** — this is why VALIDATE.md recommends `tail -f validate-cache.jsonl` for live progress (the `--output` report itself is only flushed/closed at the very end of the run). `close()` happens in a `finally`.

Cross-provider/model safety: because `model` is in the key, cloud vs local verdicts never collide; because `VERSION` is in the key, a prompt-shape change invalidates old entries automatically.

---

## 8. `--dry-run`

- No `Judge` is constructed at all (`client` stays `null`) — so a missing API key/credential never even triggers with `--dry-run` (this is explicitly documented as an escape hatch in `AnthropicClient.fromEnv`'s error message).
- Cache lookups **still run normally** — cached verdicts (from a prior real run against the same default `--cache=validate-report.jsonl`... actually `validate-cache.jsonl`) would still surface in a dry-run's report unless `--cache=none` is also passed (which is exactly why `ValidateCliTest` always pairs `--dry-run` with `--cache=none`).
- For the genuinely-`uncached` remainder of each chunk: `if (dryRun) { for (r : uncached) verdicts.put(r, null); }` — every such item gets an explicit `null` verdict (same downstream effect as a failed judge: report row omits `verdict`/`confidence`/`note`/`fields`, `Summary` would count it "missing" — though `Summary.print` isn't even called in dry-run mode, see below).
- The Phase-1 "Scanned N names…" summary line prints **unconditionally**, dry-run or not.
- Final output diverges from the normal path — instead of `Summary.print(...)`, dry-run prints:
  ```
  Dry run: built %,d batches for %,d names, no API calls made. Report → %s
  ```
  (batch count = `ceil(chosen.size() / batchSize)`), followed by `dumpFirstBatch`: if `chosen` is non-empty, prints
  ```

  --- first batch payload (dry run) ---
  <ValidationPrompt.userMessage() output for the first min(batchSize, chosen.size()) chosen items>
  ```
  i.e. the **exact** request payload that would be sent for the first batch, so a user can inspect cost/shape without spending anything.
- The JSONL `--output` report file **is still written** in dry-run mode (with `parsed`/`error`/`line`/`input` populated but no verdict fields) — `ValidateCliTest.dryRunSelectsAndExcludes` asserts on its contents directly.

---

## 9. Output report

One JSON object per line, `reportRow(r, v)`:

| Field | When present | Source |
|---|---|---|
| `line` | always | `ParseResult.line` (1-based source line) |
| `input` | always | raw name string |
| `parsed` | `r.parsed != null` | full Gson tree of `ParsedName` |
| `error` | `r.error != null` | `{type, code, message}` from `ParseResult.Err` (mutually exclusive with `parsed`) |
| `verdict` | a non-null `Verdict` was obtained | `"ok"\|"suspect"\|"wrong"` (whatever string the model/cache returned, not re-validated) |
| `confidence` | alongside `verdict` | `"low"\|"med"\|"high"` |
| `note` | `v.note` non-null and non-blank | free text |
| `fields` | `v.fields` non-null and non-empty | array of `{name,parsed,expected,reason}` |

Row order = `chosen` order = line-ascending (established at end of Phase 1, preserved through chunked iteration). Written incrementally per chunk but the file handle/buffer isn't closed until the whole run finishes — **not** safe to `tail -f` for live progress (VALIDATE.md explicitly calls this out; use the cache file instead, per §7).

Stderr summary (`Summary.print`, non-dry-run only):
```
Validated %,d names in %,d API call(s), %,d from cache.
  ok=%,d  suspect=%,d  wrong=%,d[  (no verdict=%,d)]
Most-flagged fields:                          <- only if any field was ever flagged
  <name, left-padded to 32>   <count>          <- top 15 by count, descending
Report → <absolute path>  (review 'verdict' != ok rows; jq '. | select(.verdict!=\"ok\")')
```
`apiCalls` counts actual `judge()` **invocations** (i.e. chunks with a non-empty uncached remainder), not names. `fromCache` counts individual cache-hit `ParseResult`s across the whole run. `byField` is a `TreeMap<String,Integer>` built by scanning every recorded verdict's `fields[].name` with `merge(name, 1, Integer::sum)`, then re-sorted descending by count at print time.

---

## 10. Rust-port notes

### Java deps this subsystem actually uses
- `com.google.gson:gson` (2.14.0) — manual `JsonObject`/`JsonArray` tree-building for requests, reflective POJO serialization for `ParsedName`/`Verdict` (nulls omitted by default, no `.serializeNulls()`), `JsonParser.parseString` for responses, one custom `JsonDeserializer<Verdict.FieldIssue>`.
- `java.net.http.HttpClient` (**JDK built-in**, no OkHttp/Apache HttpClient/SDK) — both providers hand-roll raw HTTP.
- `java.security.MessageDigest` (JDK built-in) — SHA-256 for cache keys.
- `java.util.Random` (JDK built-in) — reservoir sampling PRNG.
- `org.slf4j`/`logback` — used **only** inside `OpenAiClient` (`LOG.warn`); `ValidateCli`/`AnthropicClient` use `System.err`/`PrintStream` directly. Inconsistent in Java itself.
- `org.catalogueoflife:coldp` (`ColdpTerm`) — ColDP column-header detection for `--input` auto-detection, tangential to the LLM subsystem but part of the input path.

### What already exists on the Rust side (checked directly, not assumed)
- `crates/nameparser-cli/src/main.rs` (1497 lines) is a **single file** using `clap` derive: a `Command` enum (`Subcommand`) with `Parse(ParseArgs)`/`Benchmark(BenchmarkArgs)`/`Compare(CompareArgs)` variants, one `XArgs` struct + one `run_x` function per subcommand, dispatched from `fn main()`. A `Validate(ValidateArgs)` variant + `run_validate` slots into this pattern directly. Given the file is already large and `validate` is by far the biggest subcommand, consider splitting it into its own `src/validate.rs` module (`mod validate;`) rather than growing `main.rs` further.
- `nameparser-cli` is (per its own module doc) **the only workspace crate allowed to depend on `clap`**, keeping the core `nameparser` crate dependency-lean — extend that same discipline to any new HTTP/hashing/RNG deps `validate` needs: they belong in `nameparser-cli`'s `Cargo.toml` only.
- `ParsedName` (`crates/nameparser/src/model/name.rs`) **already derives `Serialize`** with `#[serde(skip_serializing_if = "Option::is_none")]` on every optional field, explicitly engineered to match Gson's field order and null-omission "byte-for-byte" (per its own doc comment, proven by existing golden tests) — the `"parsed"` payload field and the report's `parsed` field need no new serialization work, just reuse the struct.
- `NameType` in Rust already has exactly the same 5 variants as Java (`Scientific, Formula, Informal, Placeholder, Other`) — no `Otu` variant on either side, consistent with §3's finding.
- `ParseError` (`crates/nameparser/src/model/mod.rs`) already mirrors `UnparsableNameException`/`ParseResult.Err`'s shape (`type_`, `code: Option<NomCode>`, `name`, `message`).
- Confirmed via `Cargo.lock`/every `Cargo.toml` in the workspace: **none** of `rand`, `sha2`, `reqwest`, `ureq`, `rustls`, `tokio` exist anywhere yet — `validate` is a genuinely new dependency surface, not an extension of something already pulled in transitively.

### Concrete crate choices / decisions to make explicit in the plan
- **JSON:** reuse `serde_json` (already everywhere). Request bodies via `serde_json::json!{...}` mirrors Java's manual `JsonObject` tree-building closely. Response parsing can mix a typed `#[derive(Deserialize)] struct Verdict` for the strict Anthropic structured-output path with manual `serde_json::Value` walking for the tolerant local-model path (mirroring `Verdicts.java`'s brace-matching salvage logic — that logic has no off-the-shelf crate equivalent and should be hand-ported, including its unit tests almost verbatim from `AnthropicClientTest`/`OpenAiClientTest`: think-tag stripping with embedded braces, markdown-fence/preamble tolerance, brace-matching that respects string literals/escapes, truncated-batch salvage, non-string field-value coercion).
- **Hashing:** `sha2` crate (`Sha256`) — direct, no design decision, matches `MessageDigest.getInstance("SHA-256")` 1:1. Remember the `\0`-separator-after-every-part detail if byte-identical cache keys with the Java tool ever matter (they don't need to for the Rust tool's own cache to work correctly — only for reading/reusing a Java-produced cache file, which is presumably out of scope).
- **HTTP client — real decision point:** the whole existing Rust CLI is synchronous (no `tokio`/async anywhere in the workspace). `reqwest`'s blocking client still pulls in a tokio runtime as a transitive dependency even with only the `blocking` feature enabled. `ureq` is a genuinely synchronous HTTP client with no async-runtime dependency at all, and this tool's request pattern (sequential batch-by-batch, no concurrency in Java either — see below) doesn't need anything reqwest offers over it. **Recommend `ureq`** unless a concrete future need (streaming responses, concurrent batches) justifies reqwest's heavier dependency tree.
- **Reservoir PRNG — the trickiest decision:** the Algorithm R logic itself is ~10 lines and trivially hand-ported with no crate. The PRNG underneath is the issue: `java.util.Random` is a specific, fully-documented 48-bit LCG (`seed = (seed*0x5DEECE66DL + 0xB) & ((1<<48)-1)`, `nextDouble()` from two `next(bits)` calls). **No Rust `rand` crate RNG (`StdRng`, `SmallRng`, etc.) reproduces this bit sequence.** If "same `--seed` picks the same names as the Java tool, for the same corpus" is a goal, the port needs a small hand-rolled `JavaRandom` type replicating the LCG + `nextDouble()` exactly (~20 lines, no crate). If cross-language reproducibility is *not* actually required (only within-Rust-tool reproducibility across reruns, which is what the caching/regression-test workflow actually depends on), a idiomatic Rust PRNG (e.g. a hand-rolled tiny xorshift, or `rand`+`rand_chacha` if a crate is preferred) is simpler and still fully satisfies `ReservoirTest`-equivalent tests. **This should be a stated, deliberate choice in the plan, not an accident** — recommend deciding explicitly and documenting it, defaulting to "within-tool reproducibility only" (simpler, no crate) unless there's a real cross-tool-corpus-sharing use case.
- **Retry/backoff:** hand-roll (as Java does) — `std::thread::sleep` + the same `pow(2, attempt) * 500ms` formula (+ `retry-after` header honoring for the Anthropic path). No crate needed; adding one (e.g. `backoff`) would be inconsistent with this workspace's minimal-deps ethos for something this small.
- **Logging:** the Rust CLI has no logging framework at all today (`eprintln!` throughout `main.rs`) — continue that; no `log`/`tracing`/`env_logger` needed, and it actually makes the Rust version *more* consistent than Java (where only `OpenAiClient` uses SLF4J and everything else uses `System.err` directly).
- **CLI parsing:** trivial — add `Validate(ValidateArgs)` to the existing `Command` enum, `#[arg(long)]` fields map 1:1 onto `Args`'s `--key=value` options. Decide whether to hand-write `validate --help` text matching Java's `printUsage()` for fidelity, or accept clap's auto-generated help (Java's manual approach was presumably chosen only because the underlying `Args` class has no help-generation of its own).
- **Concurrency:** Java's batch loop is single-threaded/sequential end-to-end (`VerdictCache`'s own javadoc admits "Not thread-safe; guard put externally when judging concurrently" — implying concurrent judging isn't actually wired up). Port as sequential; no `rayon`/`tokio` needed.

### Genuine scope gaps to flag (not just dependency choices)
- **ColDP TSV/CSV input is explicitly out of scope on the Rust side today.** The existing `parse` subcommand's own module doc states it "only ever reads plain text... a real ColDP header is not sniffed" — this is the *same* `NameInputReader`/`InputDetector`/`ColdpReader` machinery the Java `validate` also uses for `--input`. Recommend the plan make the same choice `parse` already made: plain-text-only input for `validate` v1 (a bare name-per-line file, first-tab-truncated, matching `PlainTextReader`'s behavior — which already "just works" against a bare `col-names.tsv`), with ColDP TSV/CSV detection as an explicit, separately-scoped follow-up (it would benefit `parse` too, not just `validate`, so it may deserve its own task rather than being bundled into the `validate` port).
- **No canonical-name formatter exists yet in the Rust `nameparser` crate.** `ValidationPrompt.item()` adds an optional `"canonical"` convenience field via `pn.canonicalNameComplete()` (Java's `NameFormatter.canonicalComplete()`). Grepped `crates/nameparser/src/` for `canonical`/`NameFormatter` — nothing resembling this exists; the Rust port's own planning memory (`~/.claude/projects/.../gbif-name-parser-rust-port.md`) separately lists `NameFormatter` as a "remaining Phase 1 stage," not yet done. Since Java's own code treats `canonical` as best-effort (wrapped in try/catch, silently omitted on failure), recommend the Rust `validate` v1 **ship without the `canonical` field** — the model still sees the complete structured `parsed` object either way — rather than blocking the port on porting `NameFormatter` first. Flag this as a deliberate, documented deferral.
- **Anthropic's `output_config`/`thinking:adaptive` structured-output feature** is fully hand-specified in `AnthropicClient.verdictSchema()` (plain nested `JsonObject`s) — no schema-generation crate (`schemars` etc.) is needed or used in Java; a direct `serde_json::json!` transcription is the right-sized Rust equivalent.
- **Testing/oracle strategy** should follow the precedent already set by the Rust `compare`/`parse` subcommands (diffed against Java-CLI-generated golden files): port `BarcodeOtuFilterTest`'s cases verbatim as regex unit tests; port `AnthropicClientTest`/`OpenAiClientTest`'s tolerant-parsing fixtures verbatim (they're pure string-in/struct-out cases, trivially portable); reproduce `ValidateCliTest`'s `--dry-run` + `--cache=none` end-to-end reproducibility test. Whether to *also* pursue cross-language selection-parity (same seed ⇒ same names as Java) depends on the PRNG decision above.
