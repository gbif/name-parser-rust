// SPDX-License-Identifier: Apache-2.0

//! `validate` — LLM-audited correctness sampling for the parser, mirroring the Java CLI's
//! `org.gbif.nameparser.cli.ValidateCli` / `BarcodeOtuFilter`
//! (`/Users/markus/code/gbif/name-parser/name-parser-cli/src/main/java/org/gbif/nameparser/cli/`).
//! See `docs/superpowers/findings/2026-07-11-validate-java-recon.md` for the full verified map
//! of the Java subsystem this ports, and `docs/superpowers/plans/2026-07-11-phase4c-validate.md`
//! for the task breakdown and Global Constraints binding every task in this port.
//!
//! ## Status: sampling core wired (Phase 4c Task 2)
//!
//! Task 1 provided the pieces that need no LLM/HTTP/sampling machinery: the [`ValidateArgs`]
//! CLI surface, [`is_barcode_otu`] (`BarcodeOtuFilter`), and [`is_interesting`] (the
//! "suspicious tail" predicate, `ValidateCli.isInteresting`).
//!
//! Task 2 (this task) adds the reproducible-sampling core: [`JavaRandom`] (a bit-exact
//! hand-port of `java.util.Random`'s 48-bit LCG), [`Reservoir`] (Algorithm R over
//! [`JavaRandom`]), and [`select`] (`ValidateCli.select` — the single-pass corpus scan that
//! pre-filters barcode/OTU codes, parses the rest, and reservoir-samples a bounded,
//! line-ordered `interesting + ordinary` selection). [`run_validate`] now runs `select` and,
//! for `--dry-run`, writes the verdict-less JSONL report and prints the batch-count summary —
//! matching `ValidateCli.main`'s Phase 1 plus the dry-run half of Phase 2. A non-dry-run
//! invocation still isn't implemented: it runs the same selection scan (so its stderr summary
//! is real) but then reports that judging isn't wired up yet, rather than either silently
//! doing nothing or attempting an LLM call with no client to make it. The judge/report loop
//! for a REAL run — the LLM clients, the verdict cache, per-chunk judging, the first-batch
//! prompt-payload dump — lands in Tasks 3-5.
//!
//! `nameparser-cli` is a binary-only crate (no library target), so `pub` here doesn't exempt an
//! item from the `dead_code` lint the way it would in a library. [`run_validate`] now reaches
//! most of this module's items through [`select`] (which itself calls [`is_barcode_otu`],
//! [`nameparser::parse`], [`is_interesting`], and both [`Reservoir`]/[`JavaRandom`]), but a few
//! items still have no caller outside this module's own tests: `ValidateArgs`'s `provider`/
//! `model`/`cache`/`api_url` fields (read once the LLM client exists, Tasks 3-4) and
//! [`Reservoir::seen`] (kept for parity with Java `Reservoir.seen()`, exercised by this
//! module's own reservoir tests). The blanket allow stays until those land.

#![allow(dead_code)]

use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Instant;

use clap::Args;
use nameparser::model::{NameType, ParseError, ParsedName, State};
use regex::Regex;

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
/// `note`/`fields`, since no cache or judge exists yet) and prints the same
/// `"Dry run: built N batches..."` summary line Java prints, batching `chosen` into
/// `args.batch`-sized (clamped to at least 1, matching Java's `Math.max(1, batch)`) chunks
/// purely to report how many batches a real run would send — no batch's contents are used for
/// anything else yet (the first-batch prompt-payload dump is Task 3, once `ValidationPrompt`
/// exists to build it).
///
/// For a non-dry-run, there is no `Judge`/LLM client to call yet (Tasks 3-4), so rather than
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
    // TODO(Task 3): once `ValidationPrompt` exists, also dump the exact first-batch request
    // payload here (`ValidationPrompt::user_message` over the first `min(batch, chosen.len())`
    // chosen items), matching Java's `dumpFirstBatch`.
    eprintln!(
        "Dry run: built {num_batches} batches for {} names, no API calls made. Report → {}",
        chosen.len(),
        crate::absolute_path(&args.output).display()
    );
    Ok(())
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
}
