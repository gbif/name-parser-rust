// SPDX-License-Identifier: Apache-2.0

//! `validate` — LLM-audited correctness sampling for the parser, mirroring the Java CLI's
//! `org.gbif.nameparser.cli.ValidateCli` / `BarcodeOtuFilter`
//! (`/Users/markus/code/gbif/name-parser/name-parser-cli/src/main/java/org/gbif/nameparser/cli/`).
//! See `docs/superpowers/findings/2026-07-11-validate-java-recon.md` for the full verified map
//! of the Java subsystem this ports, and `docs/superpowers/plans/2026-07-11-phase4c-validate.md`
//! for the task breakdown and Global Constraints binding every task in this port.
//!
//! ## Status: cache + LLM clients wired, judge loop still pending (Phase 4c Task 4)
//!
//! Task 1 provided the pieces that need no LLM/HTTP/sampling machinery: the [`ValidateArgs`]
//! CLI surface, [`is_barcode_otu`] (`BarcodeOtuFilter`), and [`is_interesting`] (the
//! "suspicious tail" predicate, `ValidateCli.isInteresting`).
//!
//! Task 2 added the reproducible-sampling core: [`JavaRandom`] (a bit-exact hand-port of
//! `java.util.Random`'s 48-bit LCG), [`Reservoir`] (Algorithm R over [`JavaRandom`]), and
//! [`select`] (`ValidateCli.select` — the single-pass corpus scan that pre-filters barcode/OTU
//! codes, parses the rest, and reservoir-samples a bounded, line-ordered `interesting +
//! ordinary` selection).
//!
//! Task 3 added the LLM-message layer, with no HTTP client behind it yet: [`ValidationPrompt`]
//! (`llm.ValidationPrompt` — the verbatim system/output-instruction prompt text plus
//! [`ValidationPrompt::user_message`], the per-batch request payload builder), [`Verdict`]/
//! [`FieldIssue`] (`llm.Verdict` — the model's reply shape, with `FieldIssue`'s four fields
//! tolerantly coerced to display strings), and [`parse_verdicts`] (`llm.Verdicts.parse` — the
//! tolerant `{"verdicts":[...]}` extractor: `<think>` traces, markdown fences/preamble, and a
//! `max_tokens`-truncated trailing object are all handled the same way Java's does).
//! `run_validate`'s `--dry-run` path also dumps the exact first-batch request payload to
//! stderr (`ValidationPrompt::user_message` over the first `min(batch, chosen.len())` items),
//! matching Java's `dumpFirstBatch`.
//!
//! Task 4 (this task) closes a Task 3 review finding and adds everything a judge loop needs,
//! but does NOT wire it into [`run_validate`] yet (that's Task 5): [`parse_verdicts`] now
//! skips-and-continues on a single malformed verdict object (`eprintln!`s a warning and drops
//! just that one) instead of erroring the whole reply — see its doc comment point 5. The
//! [`cache`] submodule adds [`cache::VerdictCache`]/[`cache::cache_key`] (`llm.VerdictCache` —
//! a SHA-256-keyed, JSONL-backed verdict cache). The [`client`] submodule adds the
//! [`client::Judge`] trait, [`client::AnthropicClient`] and [`client::OpenAiClient`] (the
//! latter also serves `--provider=local`/`ollama`), [`client::build_judge`] (provider
//! normalization + default-model resolution), and the shared hand-rolled retry/backoff
//! (`client::retry_decision`) both clients use. A non-dry-run [`run_validate`] invocation still
//! isn't implemented: it runs the same selection scan (so its stderr summary is real) but then
//! reports that judging isn't wired up yet — the per-chunk cache-lookup + judge + reconcile +
//! report loop lands in Task 5.
//!
//! `nameparser-cli` is a binary-only crate (no library target), so `pub` here doesn't exempt an
//! item from the `dead_code` lint the way it would in a library. [`run_validate`] now reaches
//! most of this module's items through [`select`] (which itself calls [`is_barcode_otu`],
//! [`nameparser::parse`], [`is_interesting`], and both [`Reservoir`]/[`JavaRandom`]) and through
//! the dry-run payload dump (which calls [`ValidationPrompt::user_message`]), but a few items
//! still have no caller outside this module's (and its submodules') own tests: `ValidateArgs`'s
//! `provider`/`model`/`cache`/`api_url` fields, [`ValidationPrompt::VERSION`], and everything in
//! [`cache`]/[`client`] — all consumed by the Task 5 judge loop, not yet by [`run_validate`]
//! itself. [`Reservoir::seen`] is kept for parity with Java `Reservoir.seen()`, exercised only
//! by this module's own reservoir tests. The blanket allow stays until Task 5 wires these in.

#![allow(dead_code)]

pub mod cache;
pub mod client;

use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Instant;

use clap::Args;
use nameparser::model::{NameType, ParseError, ParsedName, State};
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::value::RawValue;

/// Options for `nameparser-cli validate`, mirroring the Java CLI's `ValidateCli` option set —
/// see `VALIDATE.md`'s option table / `ValidateCli`'s `printUsage()`, cross-checked in the
/// recon doc §1, which this reproduces option-for-option and default-for-default.
#[derive(Args)]
pub struct ValidateArgs {
    /// LLM provider: `anthropic` (cloud Claude) or `openai`/`local`/`ollama` (OpenAI-compatible
    /// local server). `local`/`ollama` are normalized to the openai-compatible client at
    /// resolution time (a later task) — there is no separate "local" client type.
    #[arg(long, default_value = "anthropic")]
    pub provider: String,

    /// Model id, passed straight through with no validation. The default is resolved per
    /// `--provider` (`claude-opus-4-8` for anthropic, `qwen2.5:14b-instruct` for
    /// openai/local/ollama) once the provider is known, in a later task — not a clap default,
    /// since it depends on another field.
    #[arg(long)]
    pub model: Option<String>,

    /// Corpus to sample from: plain text, one name per line (name = substring before the first
    /// TAB, trimmed; blank/`#` lines skipped) — matches the `parse`/`benchmark` readers' plain-
    /// text rules. Java's own default additionally auto-detects ColDP TSV/CSV; that detection
    /// is explicitly out of scope for this port (same deferral `parse` already made), so a real
    /// ColDP file is read column-0-as-name rather than column-sniffed. The literal default path
    /// below (matching `ValidateCli.DEFAULT_INPUT`) is not shipped in this repository — pass
    /// `--input` explicitly to point at a real corpus.
    #[arg(long, default_value = "data/col-names.tsv")]
    pub input: PathBuf,

    /// JSONL report path.
    #[arg(long, default_value = "validate-report.jsonl")]
    pub output: PathBuf,

    /// Max names sent to the LLM.
    #[arg(long, default_value_t = 2000)]
    pub budget: usize,

    /// Of the budget, how many ordinary (non-"interesting") names to sample as a baseline.
    /// Clamped to `min(sample_normal, budget)` inside [`select`], where it's consumed as the
    /// `ordinary` reservoir's capacity.
    #[arg(long, default_value_t = 200)]
    pub sample_normal: usize,

    /// Names per LLM request. Clamped to `max(1, batch)` in [`run_validate`], where it's
    /// consumed to report the `--dry-run` batch count (a later task will also use it to chunk
    /// `chosen` for the real judge loop).
    #[arg(long, default_value_t = 25)]
    pub batch: usize,

    /// Reservoir-sampling seed.
    #[arg(long, default_value_t = 17)]
    pub seed: i64,

    /// Verdict cache path (JSONL). The literal value `none` (case-insensitive) disables
    /// persistence — checked where the cache is opened (a later task), not here.
    #[arg(long, default_value = "validate-cache.jsonl")]
    pub cache: String,

    /// Endpoint override. anthropic: overrides `ANTHROPIC_BASE_URL`/the public API default.
    /// openai/local: overrides `OPENAI_BASE_URL`/the local Ollama default.
    #[arg(long)]
    pub api_url: Option<String>,

    /// Select and build batches only; make no LLM calls.
    #[arg(long)]
    pub dry_run: bool,
}

/// Runs the `validate` subcommand — matches the shape of `ValidateCli.main`'s two phases,
/// though Phase 2 (judge + report) is only implemented for `--dry-run` so far (Task 2); the
/// real judge/report loop lands in Task 5.
///
/// Phase 1 always runs: [`select`] streams `args.input` (exiting the process with code 2,
/// per Java, if it doesn't exist) and reservoir-samples the `chosen` selection, printing its
/// own scan-summary line to stderr.
///
/// Phase 2: for `--dry-run`, writes one verdict-less JSONL report row per `chosen` item to
/// `args.output` (matching `ValidateCli.reportRow(r, null)` — no `verdict`/`confidence`/
/// `note`/`fields`, since no cache or judge exists yet), prints the same
/// `"Dry run: built N batches..."` summary line Java prints, batching `chosen` into
/// `args.batch`-sized (clamped to at least 1, matching Java's `Math.max(1, batch)`) chunks
/// purely to report how many batches a real run would send, then (`ValidateCli.dumpFirstBatch`)
/// dumps the exact first-batch request payload — [`ValidationPrompt::user_message`] over the
/// first `min(batch, chosen.len())` chosen items — to stderr, so a user can inspect cost/shape
/// without spending anything.
///
/// For a non-dry-run, there is no `Judge`/LLM client to call yet (Task 4), so rather than
/// silently doing nothing or panicking on an absent client, this prints a clear "not
/// implemented yet, use --dry-run" message and returns — the Phase 1 scan (and its stderr
/// summary) still ran for real above.
pub fn run_validate(args: ValidateArgs) -> io::Result<()> {
    let (chosen, _counts) = select(&args);

    if !args.dry_run {
        eprintln!(
            "nameparser-cli validate: judging (a non-dry-run) isn't implemented yet — the LLM \
             client, verdict cache, and judge/report loop land in later Phase 4c tasks. \
             Re-run with --dry-run to select and preview batches without any API calls."
        );
        return Ok(());
    }

    write_report(&args.output, &chosen)?;

    let batch = args.batch.max(1);
    let num_batches = chosen.len().div_ceil(batch);
    eprintln!(
        "Dry run: built {num_batches} batches for {} names, no API calls made. Report → {}",
        chosen.len(),
        crate::absolute_path(&args.output).display()
    );
    dump_first_batch(&chosen, batch);
    Ok(())
}

/// Java `ValidateCli.dumpFirstBatch`: if `chosen` is non-empty, prints the exact request
/// payload the first real batch would send — a blank line, a header line, then
/// [`ValidationPrompt::user_message`] over the first `min(batch, chosen.len())` items.
fn dump_first_batch(chosen: &[Item], batch: usize) {
    if chosen.is_empty() {
        return;
    }
    let first = &chosen[..batch.min(chosen.len())];
    eprintln!();
    eprintln!("--- first batch payload (dry run) ---");
    eprintln!("{}", ValidationPrompt::user_message(first));
}

/// Writes one JSONL report row per `chosen` item, reusing [`crate::render_row`] — the exact
/// same `{"line":...,"input":...,"parsed":{...}}` / `..."error":{...}}` envelope `parse`
/// already writes, since Java's `reportRow(r, v)` with `v == null` (no cache/judge exists yet
/// in this task) is exactly `parse`'s row shape with no additional fields.
fn write_report(path: &Path, chosen: &[Item]) -> io::Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    for item in chosen {
        writeln!(
            writer,
            "{}",
            crate::render_row(item.line as u64, &item.input, &item.outcome)
        )?;
    }
    writer.flush()
}

// ---------------------------------------------------------------------------------------
// BarcodeOtuFilter
// ---------------------------------------------------------------------------------------

/// Java `BarcodeOtuFilter.UNITE_SH`, verbatim: `SH` + ≥5 digits + optional `.`+digits + `FU`,
/// case-insensitive, anchored at the start only (`^`, no `$`) — a `.find()`-style match, so
/// trailing content after the pattern doesn't prevent a match. `(?-u:…)` scopes `\d`/`\b` to
/// ASCII for the whole pattern (see [`is_barcode_otu`]'s doc comment for why); the literal
/// `SH`/`FU` text is already plain ASCII either way.
static UNITE_SH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?-u:^SH\d{5,}(\.\d+)?FU\b)").unwrap());

/// Java `BarcodeOtuFilter.BOLD_BIN`, verbatim: `BOLD:` + 2-5 uppercase letters + ≥1 digit,
/// case-insensitive, anchored at the start only. `(?-u:…)` ASCII-scopes `\d`/`\b`, same as
/// [`UNITE_SH`]; `[A-Z]`/`BOLD` are already plain ASCII.
static BOLD_BIN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?-u:^BOLD:[A-Z]{2,5}\d+\b)").unwrap());

/// Java `BarcodeOtuFilter.isBarcodeOtu(String)`: `true` if `name`, trimmed, matches either
/// [`UNITE_SH`] or [`BOLD_BIN`] at the start. Applied pre-parse, on the raw input string, so a
/// UNITE/BOLD barcode/OTU code is excluded from the corpus before it ever reaches the parser
/// (recon doc §3: this regex pre-filter is the ONLY OTU exclusion point — a code that slips
/// past it and later parses/fails as `NameType::Other` is intentionally NOT re-excluded
/// downstream; there is no `NameType::Otu` variant to filter on, on either the Java or Rust
/// side).
///
/// Rust's `regex` crate is Unicode-aware by default (`\d`/`\b` match more than plain ASCII),
/// unlike Java's `Pattern` (ASCII-only unless `UNICODE_CHARACTER_CLASS` is set) — [`UNITE_SH`]/
/// [`BOLD_BIN`] are ASCII-scoped via `(?-u:…)` for exact parity with Java, even though every
/// `BarcodeOtuFilterTest` case is plain ASCII and the tests below confirm the two engines
/// already agreed on all of them without it.
pub fn is_barcode_otu(name: &str) -> bool {
    let s = name.trim();
    UNITE_SH.is_match(s) || BOLD_BIN.is_match(s)
}

// ---------------------------------------------------------------------------------------
// is_interesting — the "suspicious tail" predicate
// ---------------------------------------------------------------------------------------

/// The result of parsing one corpus row — an alias for [`nameparser::parse`]'s own return type,
/// named here to match the Java recon's `ParseResult`/`isInteresting` naming without
/// introducing a new struct. [`Item`] pairs this with the `line`/`input` Java's `ParseResult`
/// also carries.
pub type ParseOutcome = Result<ParsedName, ParseError>;

/// Java `ValidateCli.isInteresting(ParseResult)`, verbatim predicate (recon doc §2): `true` if
/// the parse failed (`Err`); otherwise `true` if the [`ParsedName`] carries any warnings, or
/// its `state` isn't [`State::Complete`], or its `type_` isn't [`NameType::Scientific`].
/// Everything else ("boring": clean, complete, scientific, no warnings) is `false` — only
/// sampled as ordinary baseline, not because it's suspicious.
///
/// Java's predicate also has an explicit `pn == null` defensive branch (`ParseResult.parsed`
/// can apparently be null there even without an accompanying `error`) — that state isn't
/// representable by this port's `Result<ParsedName, ParseError>` (every `Ok` carries a real
/// `ParsedName`), so there is nothing to port for that branch.
pub fn is_interesting(outcome: &ParseOutcome) -> bool {
    match outcome {
        Err(_) => true,
        Ok(pn) => {
            !pn.warnings.is_empty()
                || pn.state != State::Complete
                || pn.type_ != NameType::Scientific
        }
    }
}

// ---------------------------------------------------------------------------------------
// JavaRandom — a bit-exact hand-port of java.util.Random's 48-bit LCG
// ---------------------------------------------------------------------------------------

/// Java `java.util.Random`'s 48-bit linear congruential generator, hand-ported bit-for-bit
/// (the parent plan's Global Constraints) rather than using an idiomatic Rust RNG (`rand`'s
/// `StdRng`/`SmallRng` etc. use different algorithms entirely and would never reproduce
/// Java's sequence) — this is what makes [`Reservoir`] reproducible seed-for-seed, and,
/// bonus, makes `--seed=N` select the identical items the Java CLI would for the same corpus.
/// Only `next`/`next_double` are ported: the two operations [`Reservoir::offer`] actually
/// needs (nothing else in this port calls `nextInt`/`nextLong`/`nextBoolean`/etc.).
///
/// Verified bit-exact against real `java.util.Random` output in the tests below (captured via
/// `jshell` against JDK 25 — see the tests' doc comments for the exact reference values and
/// how to reproduce them).
pub struct JavaRandom {
    state: i64,
}

impl JavaRandom {
    /// `java.util.Random`'s `multiplier` constant.
    const MULTIPLIER: i64 = 0x5DEECE66D;
    /// `java.util.Random`'s `addend` constant.
    const ADDEND: i64 = 0xB;
    /// `java.util.Random`'s `mask` constant: the low 48 bits set, bit 63 (the sign bit) clear.
    const MASK: i64 = (1i64 << 48) - 1;

    /// Java `new Random(seed)`'s constructor scramble: `(seed ^ multiplier) & mask`.
    pub fn new(seed: i64) -> Self {
        JavaRandom {
            state: (seed ^ Self::MULTIPLIER) & Self::MASK,
        }
    }

    /// Java `Random.next(int bits)`: advances the LCG state and returns its top `bits` bits
    /// (`0 < bits <= 32`) as a non-negative value. `wrapping_mul`/`wrapping_add` reproduce
    /// Java `long` arithmetic's silent two's-complement overflow exactly; ANDing with
    /// [`Self::MASK`] (bit 63 always clear) leaves `state` non-negative as an `i64`, so the
    /// plain arithmetic `>>` below behaves exactly like Java's unsigned `>>>` here (there are
    /// no set bits above the shifted-in range for sign-extension to smear).
    pub fn next(&mut self, bits: u32) -> i32 {
        self.state = (self
            .state
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::ADDEND))
            & Self::MASK;
        (self.state >> (48 - bits)) as i32
    }

    /// Java `Random.nextDouble()`: a 53-bit mantissa assembled from two `next()` calls (26 +
    /// 27 bits), scaled into `[0.0, 1.0)` by `1 / 2^53`.
    pub fn next_double(&mut self) -> f64 {
        let hi = i64::from(self.next(26));
        let lo = i64::from(self.next(27));
        (((hi << 27) + lo) as f64) * (1.0 / (1i64 << 53) as f64)
    }
}

// ---------------------------------------------------------------------------------------
// Reservoir — Algorithm R (Vitter) reservoir sampling
// ---------------------------------------------------------------------------------------

/// Java `Reservoir<T>` (Algorithm R): keeps a uniform random sample of at most `capacity`
/// items from a stream of unknown length, in one pass and bounded memory — [`select`] uses
/// two of these (one per `interesting`/`ordinary` bucket) so picking which parses to send to
/// the LLM never requires holding the whole (potentially ~6.3M-name) corpus in memory. Driven
/// by [`JavaRandom`] rather than an idiomatic Rust RNG so a fixed seed is reproducible AND
/// matches the Java CLI's own selection for the same seed/corpus — see [`JavaRandom`]'s doc
/// comment.
pub struct Reservoir<T> {
    capacity: usize,
    rng: JavaRandom,
    items: Vec<T>,
    seen: u64,
}

impl<T> Reservoir<T> {
    /// Matches Java `new Reservoir<>(capacity, seed)`. Unlike Java (`Math.max(0, capacity)`),
    /// `capacity` here is already an unsigned `usize`, so no clamp is needed — a "negative
    /// capacity" simply isn't representable.
    pub fn new(capacity: usize, seed: i64) -> Self {
        Reservoir {
            capacity,
            rng: JavaRandom::new(seed),
            items: Vec::with_capacity(capacity.min(1024)),
            seen: 0,
        }
    }

    /// Offers one item; it may or may not be retained. Matches Java `Reservoir.offer`
    /// verbatim: `seen` always increments, even at `capacity == 0` (checked and returned
    /// immediately after); the reservoir fills up to `capacity` in arrival order, then, for
    /// every subsequent item, replaces a uniformly-chosen existing slot with shrinking
    /// probability (`capacity / seen`) as more items are seen — the defining property of
    /// Algorithm R: every item seen so far has an equal `capacity / seen` chance of being in
    /// the final sample.
    pub fn offer(&mut self, item: T) {
        self.seen += 1;
        if self.capacity == 0 {
            return;
        }
        if self.items.len() < self.capacity {
            self.items.push(item);
        } else {
            // Uniform in [0, seen); truncating (not rounding) matches Java's `(long)` cast,
            // and is always non-negative since `next_double()` is always in [0.0, 1.0).
            let j = (self.rng.next_double() * self.seen as f64) as i64;
            if (j as usize) < self.capacity {
                self.items[j as usize] = item;
            }
        }
    }

    /// The retained sample — order is not meaningful (matches Java `Reservoir.items()`).
    /// Consumes `self` since nothing in this port needs to keep offering after reading it out
    /// (unlike Java's borrowing `items()`, which returns a live view of the backing list).
    pub fn into_items(self) -> Vec<T> {
        self.items
    }

    /// Total number of items offered so far — matches Java `Reservoir.seen()`.
    pub fn seen(&self) -> u64 {
        self.seen
    }
}

// ---------------------------------------------------------------------------------------
// select — the single-pass corpus scan
// ---------------------------------------------------------------------------------------

/// How often (in [`ScanCounts::total`] — i.e. names actually parsed, not raw file lines)
/// [`select`] prints a progress line to stderr. Matches the Java CLI's
/// `ValidateCli.PROGRESS_EVERY`.
const PROGRESS_EVERY: u64 = 500_000;

/// Matches Java `ValidateCli.DEFAULT_INPUT`'s string form, used only in the "input not found"
/// hint message below — the actual `--input` default is wired through [`ValidateArgs`]'s clap
/// `default_value` (Task 1), not this constant.
const DEFAULT_INPUT_HINT: &str = "data/col-names.tsv";

/// One corpus row that survived the barcode/OTU pre-filter and was parsed — mirrors the Java
/// CLI's `ParseResult`, minus the ColDP-only `id` field (input is plain-text-only per this
/// port's Global Constraints, so there's never an external record id to carry).
#[derive(Debug, Clone)]
pub struct Item {
    /// 1-based source line number (matches Java `ParseResult.line`).
    pub line: usize,
    /// The raw (pre-first-TAB, trimmed) name string that was actually parsed.
    pub input: String,
    pub outcome: ParseOutcome,
}

/// Running counters from one [`select`] pass — mirrors the Java CLI's (package-private)
/// `ValidateCli.Selection`, minus `chosen`, which `select` returns as its own `Vec<Item>`.
#[derive(Debug, Default, Clone, Copy)]
pub struct ScanCounts {
    /// Names actually parsed, i.e. NOT excluded as barcode/OTU — always
    /// `interesting_seen + ordinary_seen`. Matches Java `Selection.total`.
    pub total: u64,
    /// Rows excluded pre-parse by [`is_barcode_otu`] (never reached the parser at all).
    pub excluded: u64,
    pub interesting_seen: u64,
    pub ordinary_seen: u64,
    /// Wall-clock seconds the scan took — matches Java `Selection.scanSeconds`
    /// (`System.nanoTime()`-measured there; [`std::time::Instant`] here).
    pub scan_seconds: f64,
}

/// Java `ValidateCli.select(...)`: a single pass over `args.input`, reservoir-sampling a
/// bounded, seeded, line-ordered selection of "interesting" (suspicious tail) and "ordinary"
/// (baseline) parses, bounded by `args.budget` regardless of corpus size. Exits the process
/// with code 2 (matching Java's `System.exit(2)`), printing the same two-line, actionable
/// message first, if `args.input` doesn't exist — the recon doc's §1 fail-fast contract.
///
/// Line extraction reuses [`crate::extract_name`] (the same plain-text rule `parse` already
/// uses: blank/`#`-prefixed lines skipped outright; otherwise the substring before the first
/// TAB, trimmed, with a lone `scientificName` header also skipped) — confirmed against the
/// Java CLI's actual `PlainTextReader.next()` to be the identical rule `validate` needs too,
/// not just a convenient reuse.
pub fn select(args: &ValidateArgs) -> (Vec<Item>, ScanCounts) {
    if !args.input.exists() {
        eprintln!(
            "Input not found: {}",
            crate::absolute_path(&args.input).display()
        );
        eprintln!(
            "col-names.tsv is a large, gitignored, user-supplied file — drop your copy at \
             {DEFAULT_INPUT_HINT} or pass --input=PATH."
        );
        std::process::exit(2);
    }

    // Java: `int interestingCap = Math.max(0, budget - sampleNormal);` — `saturating_sub` on
    // unsigned `usize` operands gives the same "floor at 0" behaviour directly. The ordinary
    // cap mirrors Java's args-parsing-time clamp (`Math.min(sampleNormal, budget)`) applied
    // here instead, so `select` stays self-contained regardless of whether a caller already
    // clamped `args.sample_normal`.
    let interesting_cap = args.budget.saturating_sub(args.sample_normal);
    let ordinary_cap = args.sample_normal.min(args.budget);
    let mut interesting = Reservoir::new(interesting_cap, args.seed);
    let mut ordinary = Reservoir::new(ordinary_cap, args.seed + 1);
    let mut counts = ScanCounts::default();

    let start = Instant::now();
    let file = File::open(&args.input).expect("existence just verified above");
    let reader = BufReader::new(file);

    for (idx, raw_line) in reader.lines().enumerate() {
        let raw = raw_line.expect("failed to read a line from --input");
        let line_no = idx + 1; // 1-based, matching Java's `PlainTextReader` line numbering.
        let Some(name) = crate::extract_name(&raw) else {
            continue;
        };

        // Barcode/OTU exclusion is a pre-parse regex on the raw input (UNITE SH, BOLD BIN).
        // We deliberately do NOT exclude by NameType::Other: OTU codes now fall into Other,
        // but so do many genuinely odd unparsable strings that are exactly the tail worth
        // reviewing (recon doc §3).
        if is_barcode_otu(name) {
            counts.excluded += 1;
            continue;
        }

        let outcome = nameparser::parse(name, None, None, None);
        counts.total += 1;
        let interesting_flag = is_interesting(&outcome);
        let item = Item {
            line: line_no,
            input: name.to_string(),
            outcome,
        };
        if interesting_flag {
            counts.interesting_seen += 1;
            interesting.offer(item);
        } else {
            counts.ordinary_seen += 1;
            ordinary.offer(item);
        }

        if counts.total.is_multiple_of(PROGRESS_EVERY) {
            eprintln!("  scanned {}…", counts.total);
        }
    }
    counts.scan_seconds = start.elapsed().as_secs_f64();

    // Java: `chosen = interesting.items(); chosen.addAll(ordinary.items());
    // chosen.sort(Comparator.comparingLong(r -> r.line));` — the reservoirs' own (unordered)
    // retention order is discarded; final selection is in original-file order.
    let mut chosen = interesting.into_items();
    chosen.extend(ordinary.into_items());
    chosen.sort_by_key(|it| it.line);

    eprintln!(
        "Scanned {} names in {:.1}s: {} excluded (barcode/OTU), {} interesting, {} ordinary. \
         Selected {} for validation (budget {}).",
        counts.total,
        counts.scan_seconds,
        counts.excluded,
        counts.interesting_seen,
        counts.ordinary_seen,
        chosen.len(),
        args.budget
    );

    (chosen, counts)
}

// ---------------------------------------------------------------------------------------
// ValidationPrompt — the LLM judging prompt (Java `llm.ValidationPrompt`)
// ---------------------------------------------------------------------------------------

/// Namespace for the LLM judging prompt, mirroring Java's `ValidationPrompt` (a `final class`
/// with only static members and a private constructor) — kept as a zero-sized type with
/// associated consts/fns, rather than free items, so call sites read `ValidationPrompt::SYSTEM`
/// / `ValidationPrompt::user_message(...)`, matching the Java call sites 1:1.
///
/// The system prompt encodes the parser's own documented conventions (this repo's `CLAUDE.md`
/// "Authorship conventions") so the judging model holds the parser to *its own contract* rather
/// than to the model's guesswork about how names "should" be structured — see the recon doc §4
/// for the full verified transcription this was checked against.
pub struct ValidationPrompt;

impl ValidationPrompt {
    /// Bumped on any change to the system prompt or payload shape; feeds the verdict-cache key
    /// (a later task) so cached verdicts from an older prompt are never reused silently.
    pub const VERSION: &'static str = "v1";

    /// Verbatim transcription of Java `ValidationPrompt.SYSTEM` (`String.join("\n", ...)` over
    /// one array element per line below, joined with `\n`, no trailing newline) — checked
    /// character-for-character against the Java source, including its three em dashes (U+2014,
    /// not hyphens) and straight `'`/ASCII-only quoting throughout.
    pub const SYSTEM: &'static str = concat!(
        "You are a meticulous reviewer of scientific-name parsing results.\n",
        "\n",
        "The GBIF name parser is a deterministic, rule-based parser. It takes a raw\n",
        "scientific name string and produces a structured ParsedName. Your job is to\n",
        "judge whether each ParsedName faithfully represents the raw input, according to\n",
        "the parser's own documented conventions below. You are NOT re-parsing from\n",
        "scratch and you are NOT imposing your own preferences — you are checking the\n",
        "parser against its contract.\n",
        "\n",
        "Be conservative. Only flag a result as 'suspect' or 'wrong' when you can point\n",
        "to a concrete field and say what it should be and why. When in doubt, answer\n",
        "'ok'. Formatting/whitespace differences and equally-valid alternatives are NOT\n",
        "errors. Prefer high precision over high recall — a human reviews every non-ok\n",
        "verdict, so false alarms waste their time.\n",
        "\n",
        "Parser conventions you must respect:\n",
        "- Zoological trinomials default to SUBSPECIES: ICZN uses no rank marker, so\n",
        "  'Vulpes vulpes silaceus Miller, 1907' is rank SUBSPECIES, not a generic\n",
        "  INFRASPECIFIC_NAME. Botanical infraspecific names DO require an explicit\n",
        "  subsp./var./f. marker, so absent a marker they stay INFRASPECIFIC_NAME.\n",
        "- Code inference signals (priority order): a sanctioning author (e.g. ': Fr.')\n",
        "  => BOTANICAL; '(BasAuthor) RecombAuthor, year' with an explicit infraspecific\n",
        "  marker => BOTANICAL (the year is the publication year); any other year on the\n",
        "  author span => ZOOLOGICAL; a filius (f./fil.) suffix on a non-ex author with\n",
        "  NO year => BOTANICAL; basionym + combination authors without years => BOTANICAL;\n",
        "  basionym-only without years => ZOOLOGICAL.\n",
        "- A year extracted from a stripped 'published in' reference is the publication\n",
        "  year of the work, is code-NEUTRAL, and must NOT by itself imply ZOOLOGICAL.\n",
        "- Abbreviation of authors or journals is only a weak hint, never a code signal.\n",
        "- Taxonomic-concept references (sensu, sec., auct., non/nec, emend., fide, ...)\n",
        "  belong in taxonomicNote, not in the name.\n",
        "- Viruses, hybrid formulas, OTU/specimen codes, and placeholders are legitimately\n",
        "  UNPARSABLE — for an unparsable input, judge whether the reported NameType and\n",
        "  the fact that it was rejected are appropriate, not that it failed to parse.\n",
        "\n",
        "For every item you are given, return exactly one verdict object. Echo the item's\n",
        "'index'. Use verdict 'ok' | 'suspect' | 'wrong' and confidence 'low' | 'med' |\n",
        "'high'. List only the fields you believe are wrong."
    );

    /// Verbatim transcription of Java `ValidationPrompt.OUTPUT_INSTRUCTION`. Spells out the
    /// exact reply shape as JSON; appended only for the local/OpenAI-compatible path (a later
    /// task) — Anthropic gets a JSON-schema constraint on the request instead, so never needs
    /// this text.
    pub const OUTPUT_INSTRUCTION: &'static str = concat!(
        "Respond with ONLY a JSON object, no prose and no markdown fences, of the form:\n",
        "{\"verdicts\":[{\"index\":0,\"verdict\":\"ok|suspect|wrong\",\n",
        "\"confidence\":\"low|med|high\",\"fields\":[{\"name\":\"...\",\"parsed\":\"...\",\n",
        "\"expected\":\"...\",\"reason\":\"...\"}],\"note\":\"...\"}]}\n",
        "Return exactly one verdict per input item and echo its 'index'. Use an empty\n",
        "'fields' array when the verdict is 'ok'."
    );

    /// Java `ValidationPrompt.userMessage(List<ParseResult>)`: a short header line naming the
    /// batch size, then a compact JSON array of [`item_json`] objects — one per `items`
    /// element, `index` = its position in *this* batch (0-based, local to whatever sub-batch is
    /// actually sent — see the recon doc §5 "Reconciliation" note, relevant once a later task
    /// chunks/re-sends only the uncached remainder of a chunk).
    pub fn user_message(items: &[Item]) -> String {
        let mut arr = String::from("[");
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                arr.push(',');
            }
            arr.push_str(&item_json(i, item));
        }
        arr.push(']');
        format!(
            "Judge each of the following {} parser results.\n{arr}",
            items.len()
        )
    }
}

/// Java `ValidationPrompt.item(int, ParseResult)`: one batch-array element —
/// `{"index":...,"input":...,"parsed":{...}}` on a successful parse, or
/// `{"index":...,"input":...,"unparsable":{"type":...,"message":...}}` on failure.
///
/// Deliberately **omits** Java's optional `canonical` field (`pn.canonicalNameComplete()`,
/// best-effort/try-catch-guarded there): no canonical-name formatter exists yet in the core
/// `nameparser` crate (recon doc §10 "genuine scope gaps"; the parent plan's Global Constraints
/// call this out explicitly as a deferred field, not an oversight) — the model still sees the
/// complete structured `parsed` object either way.
///
/// Built by hand (string concatenation) rather than via `serde_json::Value`/`Map`/`json!`, for
/// the exact reason [`crate::render_row`] is: this crate's `serde_json` has no `preserve_order`
/// feature, so a dynamically-built `Value::Object` would serialize its keys alphabetically
/// (`index`/`input`/`parsed`/`unparsable` happen to alphabetize correctly today, but relying on
/// that coincidence is fragile). `ParsedName`'s own `#[derive(Serialize)]` (nested in verbatim
/// via `serde_json::to_string`) always writes fields in declaration order regardless, so nesting
/// it here is exactly as order-safe as the hand-built envelope around it.
fn item_json(index: usize, item: &Item) -> String {
    let mut o = String::from("{\"index\":");
    o.push_str(&index.to_string());
    o.push_str(",\"input\":");
    o.push_str(
        &serde_json::to_string(&item.input).expect("a String always serializes to a JSON string"),
    );
    match &item.outcome {
        Ok(pn) => {
            o.push_str(",\"parsed\":");
            o.push_str(&serde_json::to_string(pn).expect("ParsedName always serializes to JSON"));
        }
        Err(e) => {
            o.push_str(",\"unparsable\":{\"type\":");
            o.push_str(
                &serde_json::to_string(&e.type_).expect("NameType always serializes to JSON"),
            );
            o.push_str(",\"message\":");
            o.push_str(
                &serde_json::to_string(&e.message)
                    .expect("a String always serializes to a JSON string"),
            );
            o.push('}');
        }
    }
    o.push('}');
    o
}

// ---------------------------------------------------------------------------------------
// Verdict / FieldIssue — the model's reply shape (Java `llm.Verdict`)
// ---------------------------------------------------------------------------------------

/// One LLM verdict on a single parser result — Java `Verdict`, populated here by `serde` from
/// the model's (possibly salvaged, see [`parse_verdicts`]) JSON reply rather than Gson
/// reflection, so field names below are the wire/schema property names, not renamed.
///
/// `verdict`/`confidence` are plain `String`s, not enums: Anthropic's structured-output request
/// (a later task) constrains them server-side via a JSON-schema `enum`, but local models get no
/// such enforcement, so a strict Rust enum could fail to parse an otherwise-usable reply from a
/// misbehaving local model — matching the recon doc §5 "loose-typing" note, this port keeps
/// `Verdict`'s own fields untyped strings and defers any stricter validation to the call site
/// (e.g. the summary/report step, a later task).
///
/// `index`/`verdict`/`confidence` are required: unlike Gson (which silently leaves a missing
/// primitive/String field at its default, `0`/`null`), a verdict object missing any of these
/// three fails to deserialize — a deliberate, disclosed tightening for this port (the recon doc
/// flags this exact tradeoff as "worth deciding deliberately"); `fields`/`note` default when
/// absent, matching Java's `fields`/`note` being genuinely optional (empty/blank when
/// `verdict == "ok"`).
///
/// Also `Serialize` (Task 4): [`cache::VerdictCache`] writes a judged `Verdict` back out as the
/// JSONL cache's `"verdict"` object — the same field set, round-tripped through this one type
/// rather than a separate write-side shape. `note: None` is omitted on write
/// (`skip_serializing_if`), matching Gson's own default null-omission behavior (no
/// `serializeNulls()` in Java either).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    /// 0-based position within the batch this verdict belongs to (echoed back by the model).
    pub index: usize,
    /// `"ok"` | `"suspect"` | `"wrong"`.
    pub verdict: String,
    /// `"low"` | `"med"` | `"high"`.
    pub confidence: String,
    /// Per-field problems the model identified; empty when `verdict == "ok"`.
    #[serde(default)]
    pub fields: Vec<FieldIssue>,
    /// Free-text explanation, one or two sentences.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Verdict {
    /// Java `Verdict.isOk()`: case-insensitive `"ok"` comparison (so a model that replies
    /// `"OK"`/`"Ok"` still counts).
    pub fn is_ok(&self) -> bool {
        self.verdict.eq_ignore_ascii_case("ok")
    }
}

/// A single field the model believes the parser got wrong — Java `Verdict.FieldIssue`. Every
/// field is coerced to a display string via [`coerce_to_string`] regardless of the JSON shape
/// the model actually sent (Java's `Verdicts.fieldIssueDeserializer`): local models (e.g.
/// gemma) sometimes echo a whole nested object, or a bare number/boolean, as a field's
/// `parsed`/`expected` value instead of a flat string — coercing defends against that instead
/// of aborting the whole judging run over one loose field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldIssue {
    /// ParsedName field name, e.g. `rank`, `code`, `combinationAuthorship.year`.
    #[serde(default, deserialize_with = "coerce_to_string")]
    pub name: String,
    /// What the parser produced for that field.
    #[serde(default, deserialize_with = "coerce_to_string")]
    pub parsed: String,
    /// What the model believes it should be.
    #[serde(default, deserialize_with = "coerce_to_string")]
    pub expected: String,
    /// Why.
    #[serde(default, deserialize_with = "coerce_to_string")]
    pub reason: String,
}

/// Java `Verdicts.asString(JsonElement)`: coerce any JSON value to a display string — a JSON
/// string is unescaped to its plain content; `null` (or an absent key, via `#[serde(default)]`
/// on the field itself) becomes `""`; anything else (number, boolean, object, array) is kept as
/// its exact original compact JSON source text.
///
/// Deliberately captures via [`RawValue`] rather than deserializing into a `serde_json::Value`
/// and re-serializing it: this crate's `serde_json` has no `preserve_order` feature, so a
/// `Value::Object` round-trip would alphabetically re-sort a nested object's keys, diverging
/// from Java's `JsonObject.toString()` (which preserves the model's original member order) —
/// see the `raw_value` feature comment in `Cargo.toml`. `RawValue` sidesteps the whole problem
/// by never materializing a reordered structure: it captures the exact source bytes.
fn coerce_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Box::<RawValue>::deserialize(deserializer)?;
    let text = raw.get().trim();
    if text == "null" {
        return Ok(String::new());
    }
    if let Some(stripped) = text.strip_prefix('"') {
        debug_assert!(stripped.ends_with('"'));
        return serde_json::from_str::<String>(text).map_err(serde::de::Error::custom);
    }
    Ok(text.to_string())
}

// ---------------------------------------------------------------------------------------
// parse_verdicts — tolerant extraction (Java `llm.Verdicts.parse`)
// ---------------------------------------------------------------------------------------

/// Java `Verdicts.THINK`: strips `<think>…</think>` / `<thinking>…</thinking>` reasoning traces
/// (case-insensitive, `.` matches newlines too) before any JSON scanning — a reasoning model's
/// trace may itself contain braces, which would confuse a naive brace-matching scan if not
/// removed first. Java replaces each match with a single space (not empty), reproduced here for
/// parity even though it makes no observable difference to the downstream scan (stray
/// whitespace between array elements is already skipped).
static THINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<think(?:ing)?>.*?</think(?:ing)?>").unwrap());

/// Java `Verdicts.parse(String)`: the resilience layer that makes judging tolerant of everything
/// a local/reasoning model routinely does to an otherwise-valid `{"verdicts":[...]}` reply —
/// `<think>` traces, markdown fences, prose preamble, a `max_tokens` cutoff mid-object, and (a
/// deliberate, disclosed EXTENSION beyond Java, see point 5) one malformed verdict object amid
/// otherwise-good ones.
///
/// 1. Errors (matching Java's `IllegalStateException`, ported here as
///    [`io::ErrorKind::InvalidData`]) if `reply` is blank.
/// 2. Strips `<think(ing)>…</think(ing)>` via [`THINK`].
/// 3. Locates the `"verdicts"` key, then its `[`; errors if not found.
/// 4. Walks the array element-by-element with a string-literal-and-escape-aware brace-depth
///    scanner ([`match_object`]) rather than a naive first-`{`/last-`}` span, collecting each
///    complete top-level `{…}` object's raw text and skipping stray whitespace/commas between
///    elements. A trailing object left unbalanced (the model hit `max_tokens` mid-object) is
///    silently dropped — the complete verdicts already collected are salvaged rather than
///    losing the whole batch.
/// 5. Deserializes each collected object's raw text into a [`Verdict`] via `serde_json`,
///    **independently, skipping-and-continuing on a single failure**: this port's `Verdict`
///    deserialization is stricter than Java's Gson (see `Verdict`'s doc comment — a missing
///    required field fails here where Gson would silently default it), so unlike Java, one
///    malformed object is a real possibility this port must tolerate on its own. A verdict
///    object that fails to deserialize is `eprintln!`-warned and dropped, keeping the
///    successfully-parsed rest — mirroring step 4's "salvage what's usable" philosophy, and
///    safe for the same reason: the (Task 5) reconcile step treats a missing index as "retry
///    next run," so losing one bad object is fine, but erroring the *entire* reply over it is
///    not (for `AnthropicClient`, which has no enclosing try/catch, that would abort the whole
///    run). Only a blank reply or a missing `"verdicts"` array (steps 1/3) remain hard errors.
pub fn parse_verdicts(reply: &str) -> io::Result<Vec<Verdict>> {
    if reply.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Empty model output",
        ));
    }
    let cleaned = THINK.replace_all(reply, " ");
    let objects = extract_verdict_objects(&cleaned)?;
    let mut out = Vec::with_capacity(objects.len());
    for obj in objects {
        match serde_json::from_str::<Verdict>(obj) {
            Ok(verdict) => out.push(verdict),
            Err(e) => eprintln!(
                "Skipping malformed verdict object: {e} (raw: {})",
                brief(obj)
            ),
        }
    }
    Ok(out)
}

/// Java `Verdicts.extractVerdictObjects(String)`. Returns each complete top-level verdict
/// object's raw text, in order.
fn extract_verdict_objects(text: &str) -> io::Result<Vec<&str>> {
    let key = text.find("\"verdicts\"");
    let arr = key.and_then(|k| text[k..].find('[').map(|off| k + off));
    let Some(arr) = arr else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Model output has no 'verdicts' array: {}", brief(text)),
        ));
    };

    // Byte-indexed on purpose, not `char_indices()`: every byte this loop inspects/matches on
    // (`"`, `\`, `{`, `}`, `]`) is single-byte ASCII, and UTF-8's self-synchronizing design
    // guarantees no multi-byte character's continuation bytes can equal an ASCII byte value —
    // so every slice boundary produced below still lands on a valid `char` boundary, even
    // though the input (e.g. a `note` field) may contain arbitrary Unicode.
    let bytes = text.as_bytes();
    let mut objects = Vec::new();
    let mut i = arr + 1;
    let n = bytes.len();
    while i < n {
        match bytes[i] {
            b']' => break, // array closed cleanly
            b'{' => match match_object(bytes, i) {
                Some(end) => {
                    objects.push(&text[i..=end]);
                    i = end + 1;
                }
                None => break, // trailing object truncated at max_tokens — salvage what came before
            },
            _ => i += 1, // whitespace, commas, stray characters between elements
        }
    }
    Ok(objects)
}

/// Java `Verdicts.matchObject(String, int)`: byte index of the `}` closing the object that
/// opens at `open`, honouring string literals and backslash escapes so a `{`/`}`/`"` inside a
/// JSON string value is never mistaken for structural JSON. `None` if the object never closes
/// (truncated input — the max_tokens salvage case).
fn match_object(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (i, &c) in bytes.iter().enumerate().skip(open) {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_string = false;
            }
            continue;
        }
        match c {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Java `Verdicts.brief(String)`: a trimmed, `…`-truncated (at 500 chars) preview of `s`, for
/// error messages that otherwise might dump an entire (possibly huge) model reply. `pub(crate)`
/// (not private) so the `client` submodule's HTTP-error messages (Task 4) can reuse it too,
/// exactly like Java's single `Verdicts.brief` is shared by `ValidateCli`/`AnthropicClient`/
/// `OpenAiClient` alike.
pub(crate) fn brief(s: &str) -> String {
    let t = s.trim();
    if t.chars().count() > 500 {
        let head: String = t.chars().take(500).collect();
        format!("{head}…")
    } else {
        t.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- BarcodeOtuFilter — BarcodeOtuFilterTest cases, verbatim ----

    #[test]
    fn barcode_otu_matches_unite_sh_codes() {
        assert!(is_barcode_otu("SH1957732.10FU"));
        assert!(is_barcode_otu("sh1958183.10fu"));
    }

    #[test]
    fn barcode_otu_matches_unite_sh_code_with_surrounding_whitespace() {
        // Proves the `.trim()` in `is_barcode_otu` is load-bearing: the regex is anchored at
        // `^` with no leading `\s*`, so without trimming this would NOT match — matching
        // Java's `BarcodeOtuFilterTest` whitespace-padded case verbatim.
        assert!(is_barcode_otu("  SH1958183.10FU  "));
    }

    #[test]
    fn barcode_otu_matches_bold_bin_codes() {
        assert!(is_barcode_otu("BOLD:AAA0001"));
        assert!(is_barcode_otu("bold:aab5053"));
    }

    #[test]
    fn barcode_otu_rejects_ordinary_scientific_names() {
        assert!(!is_barcode_otu("Abies alba Mill."));
        assert!(!is_barcode_otu("Shorea"));
        assert!(!is_barcode_otu("Boldenaria"));
    }

    #[test]
    fn barcode_otu_rejects_empty_and_whitespace_without_panicking() {
        assert!(!is_barcode_otu(""));
        assert!(!is_barcode_otu("   "));
    }

    // ---- is_interesting — one test per branch ----

    #[test]
    fn is_interesting_true_for_an_unparsable_name() {
        let outcome = nameparser::parse("", None, None, None);
        assert!(outcome.is_err());
        assert!(is_interesting(&outcome));
    }

    #[test]
    fn is_interesting_true_for_a_name_with_a_warning() {
        let outcome = nameparser::parse("Abies null Hood", None, None, None);
        let pn = outcome.as_ref().expect("should parse");
        assert!(!pn.warnings.is_empty());
        assert!(is_interesting(&outcome));
    }

    #[test]
    fn is_interesting_true_for_a_partial_state_name() {
        let outcome = nameparser::parse("Foo bar (auct.) Rolfe", None, None, None);
        let pn = outcome.as_ref().expect("should parse");
        assert_eq!(pn.state, State::Partial);
        assert!(is_interesting(&outcome));
    }

    #[test]
    fn is_interesting_true_for_a_non_scientific_type_name() {
        // `NameType::Formula`/`Other` are only ever produced via `Err(..)` in this pipeline
        // (viruses, hybrid formulas, OTU codes — all unparsable), which the Err branch above
        // already covers; `Informal`/`Placeholder` are the reachable non-Scientific types on
        // the `Ok(..)` path, so one of those is what actually exercises the `type_ !=
        // NameType::Scientific` arm of the predicate on a successful parse.
        let outcome = nameparser::parse("GenusANIC_3", None, None, None);
        let pn = outcome.as_ref().expect("should parse");
        assert_eq!(pn.type_, NameType::Informal);
        assert!(is_interesting(&outcome));
    }

    #[test]
    fn is_interesting_false_for_a_clean_scientific_complete_binomial() {
        let outcome = nameparser::parse("Abies alba Mill.", None, None, None);
        let pn = outcome.as_ref().expect("should parse");
        assert!(pn.warnings.is_empty());
        assert_eq!(pn.state, State::Complete);
        assert_eq!(pn.type_, NameType::Scientific);
        assert!(!is_interesting(&outcome));
    }

    // ---- JavaRandom — bit-exactness against real java.util.Random ----
    //
    // Reference values captured by running the following through `jshell` on JDK 25
    // (`java.util.Random`'s LCG algorithm is fixed by its own class contract and has not
    // changed since Java 1.0, so any JDK version reproduces the same sequence):
    //
    //   java.util.Random r = new java.util.Random(17);
    //   for (int i = 0; i < 5; i++) System.out.println(r.nextDouble());
    //
    // (and likewise for seeds 0 and -42). `new Random(0).nextDouble() ==
    // 0.730967787376657` is also a widely-published reference value for this exact algorithm,
    // independently corroborating the captured sequence below.

    #[test]
    fn java_random_matches_real_java_util_random_for_seed_17() {
        let mut rng = JavaRandom::new(17);
        let expected = [
            0.7323115139597316,
            0.6973704783607497,
            0.08295611145017068,
            0.8162364511057306,
            0.0443859375038691,
        ];
        for e in expected {
            assert_eq!(rng.next_double(), e);
        }
    }

    #[test]
    fn java_random_matches_real_java_util_random_for_seed_0() {
        let mut rng = JavaRandom::new(0);
        let expected = [0.730967787376657, 0.24053641567148587, 0.6374174253501083];
        for e in expected {
            assert_eq!(rng.next_double(), e);
        }
    }

    #[test]
    fn java_random_matches_real_java_util_random_for_a_negative_seed() {
        // Exercises the `seed ^ MULTIPLIER` constructor path with a negative `i64` seed
        // (Java `long` seeds can be negative; `--seed` is a plain `i64` in `ValidateArgs`).
        let mut rng = JavaRandom::new(-42);
        let expected = [0.2726154686397476, 0.06094973837072859, 0.2798902062508173];
        for e in expected {
            assert_eq!(rng.next_double(), e);
        }
    }

    #[test]
    fn java_random_same_seed_reproduces_the_same_sequence() {
        let mut a = JavaRandom::new(2026);
        let mut b = JavaRandom::new(2026);
        for _ in 0..10 {
            assert_eq!(a.next_double(), b.next_double());
        }
    }

    // ---- Reservoir — Algorithm R ----

    #[test]
    fn reservoir_same_seed_yields_an_identical_sample() {
        let mut a = Reservoir::new(10, 42);
        let mut b = Reservoir::new(10, 42);
        for i in 0..1000 {
            a.offer(i);
            b.offer(i);
        }
        assert_eq!(a.into_items(), b.into_items());
    }

    #[test]
    fn reservoir_different_seed_diverges_in_practice() {
        let mut a = Reservoir::new(10, 1);
        let mut b = Reservoir::new(10, 2);
        for i in 0..1000 {
            a.offer(i);
            b.offer(i);
        }
        assert_ne!(
            a.into_items(),
            b.into_items(),
            "practically impossible for two different seeds to coincide over 1000 offers"
        );
    }

    #[test]
    fn reservoir_capacity_zero_retains_nothing_but_still_counts_seen() {
        let mut r: Reservoir<i32> = Reservoir::new(0, 7);
        for i in 0..50 {
            r.offer(i);
        }
        assert_eq!(r.seen(), 50);
        assert!(r.into_items().is_empty());
    }

    #[test]
    fn reservoir_capacity_at_least_n_retains_every_item_in_arrival_order() {
        let mut r = Reservoir::new(100, 7);
        for i in 0..10 {
            r.offer(i);
        }
        assert_eq!(r.into_items(), (0..10).collect::<Vec<_>>());
    }

    // ---- select / --dry-run — end-to-end reproducibility (mirrors ValidateCliTest) ----

    /// A small, hand-classified corpus: 4 "interesting" names (2 unparsable viruses, 1
    /// `Informal`-typed, 1 `Partial`-state — reusing exactly the fixtures the `is_interesting`
    /// tests above already prove), 6 "ordinary" clean-binomial names, 2 barcode/OTU codes that
    /// must never reach the parser at all, plus a blank line and a `#`-comment line to prove
    /// those are skipped too. Every classification below was independently confirmed by
    /// running this corpus through the real `parse` subcommand before being hard-coded here.
    fn small_mixed_corpus() -> String {
        [
            "Tobacco mosaic virus",
            "Uranotaenia sapphirina NPV",
            "GenusANIC_3",
            "Foo bar (auct.) Rolfe",
            "",
            "Abies alba Mill.",
            "Quercus robur L.",
            "Picea abies (L.) H.Karst.",
            "Pinus sylvestris L.",
            "Betula pendula Roth",
            "Fagus sylvatica L.",
            "BOLD:AAA0001",
            "SH1957732.10FU",
            "# a comment line, skipped",
        ]
        .join("\n")
    }

    /// Builds a temp dir under `std::env::temp_dir()` unique to this test process/call —
    /// avoids a dependency on a temp-file crate for a handful of small, self-cleaning test
    /// fixtures.
    fn temp_dir_for(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nameparser-cli-validate-test-{label}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn dry_run_select_and_report_is_reproducible_and_classifies_correctly() {
        let dir = temp_dir_for("dry-run-repro");
        let input_path = dir.join("corpus.txt");
        std::fs::write(&input_path, small_mixed_corpus()).expect("write corpus");

        let make_args = |output: PathBuf| ValidateArgs {
            provider: "anthropic".to_string(),
            model: None,
            input: input_path.clone(),
            output,
            budget: 6,
            sample_normal: 2,
            batch: 25,
            seed: 17,
            cache: "none".to_string(),
            api_url: None,
            dry_run: true,
        };

        let out1 = dir.join("report1.jsonl");
        let out2 = dir.join("report2.jsonl");
        run_validate(make_args(out1.clone())).expect("first dry run should succeed");
        run_validate(make_args(out2.clone())).expect("second dry run should succeed");

        let bytes1 = std::fs::read(&out1).expect("read report1");
        let bytes2 = std::fs::read(&out2).expect("read report2");
        assert_eq!(
            bytes1, bytes2,
            "the same --seed over the same corpus must produce a byte-identical report"
        );

        let report = String::from_utf8(bytes1).expect("report must be UTF-8");
        let rows: Vec<serde_json::Value> = report
            .lines()
            .map(|l| serde_json::from_str(l).expect("each report row must be valid JSON"))
            .collect();

        // budget=6, sample_normal=2 => interesting_cap=4, ordinary_cap=2. Exactly 4
        // interesting candidates are offered (== cap, so Algorithm R never evicts any of
        // them — deterministic, not just seed-reproducible) and 6 ordinary candidates are
        // offered against a cap of 2 (forces real reservoir eviction, so the byte-identical
        // assertion above is actually exercising JavaRandom-driven reproducibility, not
        // trivially true because nothing was ever evicted).
        assert_eq!(rows.len(), 6, "budget must be filled exactly: {report}");

        let inputs: std::collections::HashSet<&str> =
            rows.iter().map(|r| r["input"].as_str().unwrap()).collect();

        for expected_interesting in [
            "Tobacco mosaic virus",
            "Uranotaenia sapphirina NPV",
            "GenusANIC_3",
            "Foo bar (auct.) Rolfe",
        ] {
            assert!(
                inputs.contains(expected_interesting),
                "interesting name {expected_interesting:?} must be selected \
                 (offered == cap, so it can never be evicted); got: {inputs:?}"
            );
        }

        for excluded in ["BOLD:AAA0001", "SH1957732.10FU"] {
            assert!(
                !inputs.contains(excluded),
                "{excluded:?} is a barcode/OTU code and must be excluded before parsing"
            );
        }

        let known_ordinary = [
            "Abies alba Mill.",
            "Quercus robur L.",
            "Picea abies (L.) H.Karst.",
            "Pinus sylvestris L.",
            "Betula pendula Roth",
            "Fagus sylvatica L.",
        ];
        let ordinary_selected = known_ordinary
            .iter()
            .filter(|n| inputs.contains(*n))
            .count();
        assert_eq!(
            ordinary_selected, 2,
            "ordinary_cap=2 with 6 ordinary candidates offered must retain exactly 2: {inputs:?}"
        );

        // Every selected row must carry either `parsed` or `error` but never a verdict field
        // yet (Task 2 has no cache/judge at all) — matches Java's `reportRow(r, null)`.
        for row in &rows {
            assert!(
                row.get("parsed").is_some() || row.get("error").is_some(),
                "every row must carry parsed or error: {row}"
            );
            assert!(row.get("verdict").is_none(), "no verdict field yet: {row}");
            assert!(
                row.get("confidence").is_none(),
                "no confidence field yet: {row}"
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- ValidationPrompt ----

    #[test]
    fn version_is_v1() {
        assert_eq!(ValidationPrompt::VERSION, "v1");
    }

    #[test]
    fn system_prompt_contains_the_documented_conventions() {
        // Spot-checks rather than a giant literal re-assertion (the constant itself, defined
        // via `concat!` fragment-for-fragment against the Java source, IS the verbatim
        // transcription) — these pin the load-bearing conventions plus the exact em dashes
        // (U+2014, not a hyphen) so a future accidental re-encoding is caught.
        let s = ValidationPrompt::SYSTEM;
        assert!(s.starts_with("You are a meticulous reviewer of scientific-name parsing results."));
        assert!(s.ends_with("'high'. List only the fields you believe are wrong."));
        // Three em dashes (U+2014), verbatim from the Java source — not ASCII hyphens.
        assert_eq!(s.matches('\u{2014}').count(), 3);
        assert!(s.contains("Vulpes vulpes silaceus Miller, 1907"));
        assert!(s.contains("Zoological trinomials default to SUBSPECIES"));
        assert!(s.contains("basionym-only without years => ZOOLOGICAL."));
        assert!(s.contains(
            "Taxonomic-concept references (sensu, sec., auct., non/nec, emend., fide, ...)"
        ));
        assert!(s.contains("Viruses, hybrid formulas, OTU/specimen codes, and placeholders"));
    }

    #[test]
    fn output_instruction_is_the_documented_verbatim_text() {
        let s = ValidationPrompt::OUTPUT_INSTRUCTION;
        assert!(s.starts_with(
            "Respond with ONLY a JSON object, no prose and no markdown fences, of the form:"
        ));
        assert!(s.contains("{\"verdicts\":[{\"index\":0,\"verdict\":\"ok|suspect|wrong\","));
        assert!(s.ends_with("'fields' array when the verdict is 'ok'."));
    }

    /// Small helper: an `Ok` [`Item`] from a real parse, for building test batches without
    /// duplicating `nameparser::parse` call sites everywhere.
    fn ok_item(line: usize, name: &str) -> Item {
        Item {
            line,
            input: name.to_string(),
            outcome: nameparser::parse(name, None, None, None),
        }
    }

    #[test]
    fn user_message_shape_for_a_mixed_batch_of_parsed_and_unparsable() {
        let items = vec![
            ok_item(1, "Abies alba Mill."),
            Item {
                line: 2,
                input: "".to_string(),
                outcome: nameparser::parse("", None, None, None),
            },
        ];
        assert!(items[0].outcome.is_ok());
        let expected_err = items[1].outcome.as_ref().unwrap_err().clone();

        let msg = ValidationPrompt::user_message(&items);
        let (header, json_part) = msg.split_once('\n').expect("header line then JSON array");
        assert_eq!(header, "Judge each of the following 2 parser results.");

        let arr: serde_json::Value = serde_json::from_str(json_part).expect("valid JSON array");
        let arr = arr.as_array().expect("top level is an array");
        assert_eq!(arr.len(), 2);

        let first = &arr[0];
        assert_eq!(first["index"], 0);
        assert_eq!(first["input"], "Abies alba Mill.");
        assert!(
            first.get("parsed").is_some(),
            "parsed item must carry `parsed`: {first}"
        );
        assert!(first.get("unparsable").is_none());
        assert!(
            first.get("canonical").is_none(),
            "`canonical` must be omitted entirely (deferred, no NameFormatter yet): {first}"
        );
        assert_eq!(first["parsed"]["genus"], "Abies");
        assert_eq!(first["parsed"]["specificEpithet"], "alba");

        let second = &arr[1];
        assert_eq!(second["index"], 1);
        assert_eq!(second["input"], "");
        assert!(second.get("parsed").is_none());
        assert!(second.get("canonical").is_none());
        let unparsable = second
            .get("unparsable")
            .expect("error item must carry `unparsable`");
        assert_eq!(
            unparsable["type"],
            serde_json::to_value(expected_err.type_).unwrap()
        );
        assert_eq!(unparsable["message"], expected_err.message);
    }

    #[test]
    fn user_message_header_counts_the_batch_not_a_global_total() {
        let items = vec![
            ok_item(1, "Abies alba Mill."),
            ok_item(2, "Quercus robur L."),
        ];
        let msg = ValidationPrompt::user_message(&items);
        assert!(msg.starts_with("Judge each of the following 2 parser results.\n"));
    }

    #[test]
    fn user_message_of_an_empty_batch_is_an_empty_array() {
        let msg = ValidationPrompt::user_message(&[]);
        assert_eq!(msg, "Judge each of the following 0 parser results.\n[]");
    }

    // ---- Verdict / parse_verdicts — ported from AnthropicClientTest / OpenAiClientTest ----
    //
    // These port the `Verdicts.parse`-equivalent core of the Java LLM-client test suites
    // (`.../cli/llm/{AnthropicClientTest,OpenAiClientTest}.java`). Cases specific to the outer
    // HTTP response envelope — Anthropic's `content` block array (`parsesStructuredVerdicts`
    // wraps its assertions in one), `AnthropicClient.verdictSchema()`, and
    // `OpenAiClient.{extractContent,finishReason,parseReply}` — belong to the HTTP clients
    // (Task 4), not to this task's `parse_verdicts`; only the fixtures that exercise
    // `Verdicts.parse`/[`parse_verdicts`] itself are ported here, several used verbatim as the
    // model's already-extracted reply text.

    #[test]
    fn verdict_round_trips_from_a_clean_json_reply() {
        // The inner JSON `AnthropicClientTest.parsesStructuredVerdicts` feeds through
        // `Verdicts.parse` once its outer `content` block array is unwrapped (that unwrapping
        // is `AnthropicClient.parseVerdicts`, Task 4) — used directly here.
        let reply = concat!(
            "{\"verdicts\":[",
            "{\"index\":0,\"verdict\":\"ok\",\"confidence\":\"high\",\"fields\":[],\"note\":\"\"},",
            "{\"index\":1,\"verdict\":\"wrong\",\"confidence\":\"med\",",
            "\"fields\":[{\"name\":\"rank\",\"parsed\":\"INFRASPECIFIC_NAME\",",
            "\"expected\":\"SUBSPECIES\",\"reason\":\"zoological trinomial\"}],\"note\":\"x\"}",
            "]}",
        );

        let verdicts = parse_verdicts(reply).expect("clean reply must parse");
        assert_eq!(verdicts.len(), 2);

        let ok = &verdicts[0];
        assert_eq!(ok.index, 0);
        assert!(ok.is_ok());

        let wrong = &verdicts[1];
        assert_eq!(wrong.index, 1);
        assert_eq!(wrong.verdict, "wrong");
        assert_eq!(wrong.fields.len(), 1);
        assert_eq!(wrong.fields[0].name, "rank");
        assert_eq!(wrong.fields[0].expected, "SUBSPECIES");
    }

    #[test]
    fn parse_verdicts_tolerates_markdown_fence_and_prose_preamble() {
        // Ported from `OpenAiClientTest.toleratesFencesAndPreamble`.
        let content = concat!(
            "Sure, here are my verdicts:\n```json\n",
            "{\"verdicts\":[{\"index\":0,\"verdict\":\"ok\",\"confidence\":\"high\",",
            "\"fields\":[],\"note\":\"\"},",
            "{\"index\":1,\"verdict\":\"suspect\",\"confidence\":\"low\",",
            "\"fields\":[{\"name\":\"code\",\"parsed\":\"ZOOLOGICAL\",\"expected\":\"BOTANICAL\",",
            "\"reason\":\"sanctioning author\"}],\"note\":\"maybe\"}]}",
            "\n```\n",
        );
        let verdicts = parse_verdicts(content).expect("fenced/preamble reply must parse");
        assert_eq!(verdicts.len(), 2);
        assert!(verdicts[0].is_ok());
        assert_eq!(verdicts[1].verdict, "suspect");
        assert_eq!(verdicts[1].fields[0].name, "code");
    }

    #[test]
    fn parse_verdicts_strips_reasoning_trace_with_embedded_braces() {
        // Ported from `OpenAiClientTest.stripsReasoningTraceWithBraces`: a naive first-`{`/
        // last-`}` span would break on the braces inside the `<think>` trace itself.
        let content = concat!(
            "<think>Let me check item 0: rank looks like {SUBSPECIES}? ",
            "Actually the parsed {genus} is fine.</think>\n",
            "{\"verdicts\":[{\"index\":0,\"verdict\":\"wrong\",\"confidence\":\"high\",",
            "\"fields\":[{\"name\":\"rank\",\"parsed\":\"INFRASPECIFIC_NAME\",",
            "\"expected\":\"SUBSPECIES\",\"reason\":\"zoological trinomial\"}],\"note\":\"\"}]}",
        );
        let verdicts = parse_verdicts(content).expect("think-tag reply must parse");
        assert_eq!(verdicts.len(), 1);
        assert_eq!(verdicts[0].verdict, "wrong");
        assert_eq!(verdicts[0].fields[0].expected, "SUBSPECIES");
    }

    #[test]
    fn parse_verdicts_ignores_braces_inside_string_values() {
        // Ported from `OpenAiClientTest.ignoresBracesInsideStringValues`.
        let content = concat!(
            "{\"verdicts\":[{\"index\":0,\"verdict\":\"suspect\",\"confidence\":\"low\",",
            "\"fields\":[],\"note\":\"odd char } in the name\"}]}",
        );
        let verdicts = parse_verdicts(content).expect("must parse despite `}` inside a string");
        assert_eq!(verdicts.len(), 1);
        assert_eq!(verdicts[0].note.as_deref(), Some("odd char } in the name"));
    }

    #[test]
    fn parse_verdicts_rejects_non_json_text() {
        // Ported from `OpenAiClientTest.rejectsNonJson` (there, `assertThrows
        // IllegalStateException`; here, an `io::ErrorKind::InvalidData` error).
        let err = parse_verdicts("I could not produce JSON, sorry.").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn parse_verdicts_rejects_blank_input() {
        // Java `Verdicts.parse`: `modelText == null || modelText.isBlank()` => throws. `&str`
        // can't be null on the Rust side, so only the all-whitespace/empty half applies.
        assert_eq!(
            parse_verdicts("").unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            parse_verdicts("   \n\t  ").unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn parse_verdicts_salvages_complete_verdicts_from_truncated_output() {
        // Ported from `OpenAiClientTest.salvagesCompleteVerdictsFromTruncatedOutput`: gemma hit
        // max_tokens mid-array — two complete verdicts, then a third cut off inside a string
        // (note the trailing `…`, an intentionally-unterminated JSON string literal).
        let content = concat!(
            "{\"verdicts\":[{\"index\":0,\"verdict\":\"ok\",\"confidence\":\"high\",\"fields\":[]},",
            "{\"index\":1,\"verdict\":\"wrong\",\"confidence\":\"high\",",
            "\"fields\":[{\"name\":\"rank\",\"parsed\":\"SPECIES\",\"expected\":\"DIVISION\",",
            "\"reason\":\"div. is a rank indicator\"}]},",
            "{\"index\":2,\"verdict\":\"wrong\",\"confidence\":\"high\",\"fields\":[{\"name\":",
            "\"specificEpithet\",\"parsed\":\"div\",\"expected\":\"\",\"reason\":\"div. is a rank indicator, not a specific …",
        );
        let verdicts = parse_verdicts(content).expect("must salvage the complete objects");
        assert_eq!(
            verdicts.len(),
            2,
            "the truncated 3rd object must be dropped, not error"
        );
        assert_eq!(verdicts[0].index, 0);
        assert_eq!(verdicts[1].index, 1);
        assert_eq!(verdicts[1].fields[0].expected, "DIVISION");
    }

    #[test]
    fn parse_verdicts_coerces_object_and_non_string_field_values() {
        // Ported from `OpenAiClientTest.coercesObjectAndNonStringFieldValues`: a nested object
        // (preserving its own original key order), a boolean, and a number must all coerce to
        // their compact-JSON/primitive display-string form rather than failing to deserialize.
        let content = concat!(
            "{\"verdicts\":[{\"index\":0,\"verdict\":\"wrong\",\"confidence\":\"high\",",
            "\"fields\":[{\"name\":\"code\",\"parsed\":\"ZOOLOGICAL\",\"expected\":\"BOTANICAL\",",
            "\"reason\":\"ok\"},",
            "{\"name\":\"combinationAuthorship\",\"parsed\":{\"authors\":[\"Miller\"],\"year\":1907},",
            "\"expected\":\"Miller, 1907\",\"reason\":\"nested\"},",
            "{\"name\":\"rank\",\"parsed\":true,\"expected\":42,\"reason\":\"scalar\"}],",
            "\"note\":\"\"}]}",
        );
        let verdicts = parse_verdicts(content).expect("must coerce non-string field values");
        assert_eq!(verdicts.len(), 1);
        let v = &verdicts[0];
        assert_eq!(v.fields[0].parsed, "ZOOLOGICAL");
        assert_eq!(
            v.fields[1].parsed,
            "{\"authors\":[\"Miller\"],\"year\":1907}"
        );
        assert_eq!(v.fields[2].parsed, "true");
        assert_eq!(v.fields[2].expected, "42");
    }

    #[test]
    fn parse_verdicts_missing_field_issue_subfield_defaults_to_empty_string() {
        // Not itself a ported Java case (Gson would silently leave a missing String field
        // `null`, which this port's plain (non-`Option`) `String` fields represent as `""`
        // instead — see `FieldIssue`'s doc comment) — pins that deliberate, disclosed behavior.
        let content = concat!(
            "{\"verdicts\":[{\"index\":0,\"verdict\":\"suspect\",\"confidence\":\"low\",",
            "\"fields\":[{\"name\":\"rank\"}],\"note\":\"\"}]}",
        );
        let verdicts = parse_verdicts(content).expect("must parse despite missing subfields");
        let f = &verdicts[0].fields[0];
        assert_eq!(f.name, "rank");
        assert_eq!(f.parsed, "");
        assert_eq!(f.expected, "");
        assert_eq!(f.reason, "");
    }

    #[test]
    fn parse_verdicts_a_lone_malformed_object_is_skipped_not_an_error() {
        // Task 4 review-fix (task-3-review.md [Important]): a verdict object that fails to
        // deserialize (here, missing the required `verdict` field — a deliberate, disclosed
        // tightening vs. Java's Gson leniency, see `Verdict`'s doc comment) must not error the
        // *whole* reply. `extract_verdict_objects` still found a well-formed `"verdicts"` array
        // with one structurally-complete (brace-balanced) object in it — the failure is only in
        // decoding that one object's fields, which is now a skip-with-warning, not a hard error.
        // This matters because the (Task 5) reconcile step treats a missing index as "retry
        // next run" — an empty `Ok(vec![])` here is safe; erroring the whole batch is not (it
        // would abort the entire run for `AnthropicClient`, which has no enclosing try/catch).
        let content = "{\"verdicts\":[{\"index\":0,\"confidence\":\"high\",\"fields\":[]}]}";
        let verdicts =
            parse_verdicts(content).expect("a single malformed object must not error the reply");
        assert!(
            verdicts.is_empty(),
            "the sole malformed object must be skipped, not defaulted: {verdicts:?}"
        );
    }

    #[test]
    fn parse_verdicts_skips_only_the_malformed_object_among_several() {
        // The brief's explicit acceptance case: 3 verdict objects, the middle one missing a
        // required field ('verdict') -> the 2 good ones are still returned, in order, with
        // their original `index` values intact (0 and 2, not renumbered to 0 and 1).
        let content = concat!(
            "{\"verdicts\":[",
            "{\"index\":0,\"verdict\":\"ok\",\"confidence\":\"high\",\"fields\":[],\"note\":\"\"},",
            "{\"index\":1,\"confidence\":\"high\",\"fields\":[]},",
            "{\"index\":2,\"verdict\":\"suspect\",\"confidence\":\"low\",\"fields\":[],\"note\":\"\"}",
            "]}",
        );
        let verdicts = parse_verdicts(content)
            .expect("one bad element among several must not error the whole reply");
        assert_eq!(
            verdicts.len(),
            2,
            "the malformed middle object must be skipped, keeping the other two: {verdicts:?}"
        );
        assert_eq!(verdicts[0].index, 0);
        assert!(verdicts[0].is_ok());
        assert_eq!(verdicts[1].index, 2);
        assert_eq!(verdicts[1].verdict, "suspect");
    }

    #[test]
    fn parse_verdicts_still_errors_on_blank_input_or_a_missing_verdicts_key() {
        // The two conditions that remain hard errors even after the skip-and-continue fix
        // (brief: "Still error only if the reply is blank / has no 'verdicts' key").
        assert_eq!(
            parse_verdicts("").unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            parse_verdicts("{\"notVerdicts\":[]}").unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn is_ok_is_case_insensitive() {
        let v = Verdict {
            index: 0,
            verdict: "OK".to_string(),
            confidence: "high".to_string(),
            fields: Vec::new(),
            note: None,
        };
        assert!(v.is_ok());
        let mut wrong = v.clone();
        wrong.verdict = "wrong".to_string();
        assert!(!wrong.is_ok());
    }
}
