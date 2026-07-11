// SPDX-License-Identifier: Apache-2.0

//! `nameparser-cli` — native command-line tools wrapping the [`nameparser`] crate, mirroring
//! the Java `org.gbif.nameparser.cli` module (`/Users/markus/code/gbif/name-parser/name-parser-cli/`).
//! This is the ONLY crate in the workspace that depends on `clap` — the core `nameparser`
//! crate stays dependency-lean, per the Phase 2 plan's Global Constraints.
//!
//! ## `parse` (Phase 2 Task 1 — implemented)
//!
//! Streams a plain-text, one-name-per-line input through [`nameparser::parse`] and writes one
//! compact JSON object per line (JSONL), matching the Java CLI's `ParseCli`/`JsonlWriter`
//! (Gson, no pretty-printing, nulls omitted) byte-for-byte — proven by
//! `tests/parse_cli.rs`, which diffs this binary's output against the same
//! `testdata/expected-parse.jsonl` Java-CLI-generated oracle the core crate's own golden
//! harness (`crates/nameparser/tests/parse_golden.rs`) uses.
//!
//! Deferred (not implemented by this task — see the crate's own doc comments at the deferral
//! points below for the exact scope):
//!   - ColDP TSV/CSV auto-detection (`InputDetector`/`ColdpReader` in Java) — this CLI only
//!     ever reads plain text (name-before-first-TAB), which already makes a bare
//!     `col-names.tsv` usable, matching the Java reader's own plain-text fallback; a real
//!     ColDP header (`ID`/`rank`/`code`/`authorship` columns) is not sniffed, so those columns
//!     are not fed to the parser as pre-split hints.
//!   - `--format=json|csv|tsv` (only `jsonl`, the default, is implemented; the others exit
//!     with a clear "not implemented yet" message rather than silently mis-writing).
//!
//! ## `benchmark` (Phase 2 Task 2 — implemented)
//!
//! Mirrors the Java CLI's `BenchmarkCli`: reads a name-per-line input file (default
//! `testdata/benchmark-data.txt`), optionally runs an untimed `--warmup` pre-pass over the
//! first 100 names (Rust has no JIT to warm up, but the pass — and the flag — are kept so a
//! `--warmup` run stays a fair, apples-to-apples comparison against the Java benchmark, which
//! does pay a real JIT-warmup cost), then times a full pass parsing every row and reports
//! count / total / average / min / p50 / p95 / max plus a by-[`nameparser::model::NameType`]
//! breakdown to stdout — nothing else goes there; progress/warnings/errors go to stderr, just
//! like Java. `benchmarks.md` at the repo root records the actual Rust-vs-Java full-parser
//! throughput comparison this command was built to make (the comparison the Phase 0 spike,
//! which only measured components, deferred).
//!
//! ## `compare` (Phase 2 Task 3 — implemented)
//!
//! Mirrors the Java CLI's `CompareCli`: streams two JSONL files (as produced by `parse`) in
//! lockstep and reports rows compared / identical / differing, status transitions
//! (`PARSED→ERROR` etc.), and the top differing field paths, plus a per-row dump of every
//! differing leaf value (capped by `--max-diffs`, default 100 — the aggregate counts are never
//! capped). `--ignore-whitespace` strips whitespace from string leaves before comparing.
//! Whichever of `a`/`b` runs out of lines first is reported as `Extra rows in A/B`; a
//! `line`-field mismatch between the two rows at the same position is counted but does not stop
//! the comparison. Like Java's `CompareCli`, everything (the per-row diff dump AND the summary)
//! goes to stdout unless `--output=PATH` redirects the diff dump to a file, in which case the
//! summary (plus a "Per-row diffs written to ..." trailer) still goes to stdout.
//!
//! One deliberate divergence from Java's exact algorithm, required rather than optional: Java's
//! own `CompareCli` diffs `parsed.warnings` (and would diff `notho`/`epithetQualifier`, though
//! neither has actually been observed to disagree) as a plain positional JSON array — but
//! `warnings` is backed by a Java `HashSet<String>` on the oracle side and by an
//! insertion-order `Vec<String>` on this crate's side (see `model::name::ParsedName::warnings`'s
//! own doc comment), so the same *content* can legitimately render in a different array order
//! on the two sides. This `compare` sorts the value under the `warnings`/`notho`/
//! `epithetQualifier` keys (by each element's rendered JSON text) before comparing, at any
//! nesting depth — the exact same 3-key definition, and the same rationale, as the core crate's
//! own golden harness (`crates/nameparser/tests/parse_golden.rs`'s `UNORDERED_FIELD_KEYS`/
//! `json_eq_unordered`) — so a set-identical-but-differently-ordered row is correctly reported
//! as IDENTICAL rather than a false positive. Every other field, at every nesting depth, is
//! compared positionally, which is correct since every other array in this schema
//! (`authors`/`exAuthors` in particular) is genuinely ordered.
//!
//! Deliberately narrower than Java's `Args`-based option surface: only `--output`,
//! `--ignore-whitespace`, and `--max-diffs` are implemented (matching this task's brief); `a`/
//! `b` are plain required positional arguments rather than also accepting Java's `--a=`/`--b=`
//! flag alternates or a third positional path as an alternate spelling of `--output` — those add
//! no behaviour beyond what `--output` plus two positional args already covers.

mod validate;

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use clap::{Args, Parser, Subcommand, ValueEnum};
use nameparser::model::{NameType, ParseError, ParsedName};
use serde_json::Value;

/// Print a progress line to stderr every this-many parsed rows (unless `--quiet`). Matches
/// the Java CLI's `ParseCli.PROGRESS_EVERY`.
const PROGRESS_EVERY: u64 = 100_000;

/// Literal input/output path meaning "use stdin/stdout" — matches the Java CLI's `STDIO`.
const STDIO: &str = "-";

/// Default input for `benchmark` — matches the Java CLI's `BenchmarkCli.DEFAULT_INPUT`
/// (`data/benchmark-data.txt` there; this repo keeps its data under `testdata/` instead).
const DEFAULT_BENCHMARK_INPUT: &str = "testdata/benchmark-data.txt";

/// Number of names parsed during the optional `--warmup` pre-pass — matches the Java CLI's
/// `BenchmarkCli.WARMUP_NAMES`. Rust has no JIT to warm up, but the same fixed pre-pass (and
/// the `--warmup` flag itself) is kept for CLI parity and so a `--warmup` run stays a fair,
/// apples-to-apples comparison against the Java benchmark: both pay the same warmup cost
/// before the timed pass.
const WARMUP_NAMES: usize = 100;

/// Any single parse slower than this is logged to stderr with the offending name — matches the
/// Java CLI's `BenchmarkCli.SLOW_PARSE_THRESHOLD_NANOS`. The parser has no internal timeout, so
/// a catastrophic-backtracking regression would otherwise be invisible (it just inflates the
/// max/p95 stats); flagging the individual row makes the culprit name observable. 50 ms is
/// ~1000x a normal parse.
const SLOW_PARSE_THRESHOLD_NANOS: u64 = 50_000_000;

/// Number of [`NameType`] variants — sizes [`NAME_TYPES`] and `BenchmarkReport::by_type`.
const NAME_TYPE_COUNT: usize = 5;

/// Fixed ordinal order matching the Java CLI's `NameType` declaration order (and this crate's
/// own `NameType`, whose declaration order in `model/enums.rs` already matches it) — keys the
/// `benchmark` by-type breakdown array and seeds its stable tie-break order (see
/// `BenchmarkReport::print`'s sort, which mirrors Java's `EnumMap`-iteration-then-stable-sort).
const NAME_TYPES: [NameType; NAME_TYPE_COUNT] = [
    NameType::Scientific,
    NameType::Formula,
    NameType::Informal,
    NameType::Placeholder,
    NameType::Other,
];

/// Index of `t` into [`NAME_TYPES`] / `BenchmarkReport::by_type`. Written as an exhaustive
/// `match` (rather than a `NAME_TYPES.iter().position(...)` lookup) so the compiler itself
/// forces an update here if `NameType` ever grows a new variant upstream, instead of silently
/// panicking — or worse, silently mis-bucketing — at run time.
fn name_type_ordinal(t: NameType) -> usize {
    match t {
        NameType::Scientific => 0,
        NameType::Formula => 1,
        NameType::Informal => 2,
        NameType::Placeholder => 3,
        NameType::Other => 4,
    }
}

#[derive(Parser)]
#[command(
    name = "nameparser-cli",
    version,
    about = "GBIF scientific name parser — native command-line tools (Rust port of name-parser-cli)."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Stream a list of names through the parser and write one JSON row per name.
    Parse(ParseArgs),
    /// Measure parser throughput on a name-per-line input file.
    Benchmark(BenchmarkArgs),
    /// Diff two JSONL files produced by `parse`, in lockstep.
    Compare(CompareArgs),
    /// Sample a corpus's "suspicious tail", have an LLM judge each parse, report.
    Validate(validate::ValidateArgs),
}

/// Options for `nameparser-cli parse`, mirroring the Java CLI's `ParseCli` option set
/// (`--input`, `--output`, `--format`, `--quiet`) — see that class's doc comment for the
/// full behavioural contract this reproduces.
#[derive(Args)]
struct ParseArgs {
    /// Source file ('-' or omitted = stdin).
    #[arg(long)]
    input: Option<String>,

    /// Target file ('-' or omitted = stdout).
    #[arg(long)]
    output: Option<String>,

    /// Output format: jsonl (default), json, csv, tsv. Only jsonl is implemented so far.
    #[arg(long, value_enum, default_value = "jsonl")]
    format: OutputFormat,

    /// Suppress per-batch progress lines (the final summary still prints).
    #[arg(long)]
    quiet: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Jsonl,
    Json,
    Csv,
    Tsv,
}

/// Options for `nameparser-cli benchmark`, mirroring the Java CLI's `BenchmarkCli` option set
/// (`--input`, `--warmup`) — see that class's doc comment for the full behavioural contract
/// this reproduces.
#[derive(Args)]
struct BenchmarkArgs {
    /// Source file (default: testdata/benchmark-data.txt).
    #[arg(long)]
    input: Option<String>,

    /// Parse the first 100 names untimed first to warm up caches/branch-prediction before the
    /// timed pass. Rust has no JIT (unlike Java's HotSpot), but the flag is kept for CLI parity
    /// and so a `--warmup` run stays a fair comparison against the Java benchmark.
    #[arg(long)]
    warmup: bool,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Parse(args) => run_parse(args),
        Command::Benchmark(args) => run_benchmark(args),
        Command::Compare(args) => run_compare(args),
        Command::Validate(args) => validate::run_validate(args),
    };
    if let Err(e) = result {
        eprintln!("nameparser-cli: {e}");
        std::process::exit(1);
    }
}

/// Runs the `parse` subcommand: stream `args.input` line-by-line, parse each extracted name,
/// and write one JSONL row per line to `args.output`. Progress goes to stderr (unless
/// `--quiet`); the final `Parsed N names (X ok, Y unparsable)` summary always prints to
/// stderr, so stdout stays a clean data stream when piping — matches the Java CLI's
/// `ParseCli.run`.
fn run_parse(args: ParseArgs) -> io::Result<()> {
    if args.format != OutputFormat::Jsonl {
        eprintln!(
            "nameparser-cli: --format={:?} is not implemented yet (only jsonl is supported) \
             — see the crate's module doc comment for the full list of deferrals",
            args.format
        );
        std::process::exit(2);
    }

    let reader = open_input(args.input.as_deref())?;
    let mut writer = open_output(args.output.as_deref())?;

    let mut line_no: u64 = 0;
    let mut total: u64 = 0;
    let mut ok: u64 = 0;
    let mut unparsable: u64 = 0;

    for line in reader.lines() {
        let raw = line?;
        line_no += 1;
        let Some(name) = extract_name(&raw) else {
            continue;
        };

        let result = nameparser::parse(name, None, None, None);
        let is_ok = result.is_ok();
        writeln!(writer, "{}", render_row(line_no, name, &result))?;

        total += 1;
        if is_ok {
            ok += 1;
        } else {
            unparsable += 1;
        }
        if !args.quiet && total.is_multiple_of(PROGRESS_EVERY) {
            eprintln!("  {total} parsed (line {line_no})");
        }
    }
    writer.flush()?;

    // Unconditional, like Java's `summary.printf(...)` outside the `if (!quiet)` guard —
    // only the periodic progress lines above are gated by `--quiet`.
    eprintln!("Parsed {total} names ({ok} ok, {unparsable} unparsable)");
    Ok(())
}

/// Applies the plain-text input rules, mirroring the Java CLI's `PlainTextReader`: a raw line
/// is skipped outright if it is empty or starts with `#`. Otherwise the name is the substring
/// before the first TAB, trimmed — and skipped too if that trim leaves it empty, or leaves
/// exactly the literal `scientificName` (the lone header Java's plain-text reader also
/// tolerates, since a bare `col-names.tsv`-style single-column TSV extract is a valid plain-
/// text input). Returns `None` when the line contributes no name; the caller keeps reading
/// (the line still counts towards the 1-based line numbering either way).
pub(crate) fn extract_name(raw: &str) -> Option<&str> {
    if raw.is_empty() || raw.starts_with('#') {
        return None;
    }
    let name = raw.split('\t').next().unwrap_or(raw).trim();
    if name.is_empty() || name == "scientificName" {
        return None;
    }
    Some(name)
}

/// Renders one JSONL row exactly as the Java CLI's `JsonlWriter` does — Gson with
/// `disableHtmlEscaping()` and no `serializeNulls()`, i.e. compact JSON with absent fields
/// omitted entirely (not `null`): `{"line":N,"input":"...","parsed":{...}}` on success, or
/// `{"line":N,"input":"...","error":{"type":"...","code":"...","message":"..."}}` on failure
/// (`code` omitted when absent, matching `ParseResult.Err`'s nullable `NomCode` field).
///
/// Built by hand rather than via `serde_json::Map`/`json!`, because this crate's `serde_json`
/// dependency has no `preserve_order` feature enabled: a dynamically-built `Value::Object` is
/// `BTreeMap`-backed there and would serialize its keys alphabetically, silently reordering
/// `line`/`input`/`parsed` away from the Java oracle's declared field order. Each leaf value
/// below is still produced by `serde_json::to_string` (proper JSON string escaping, and the
/// exact enum-name / nested-struct rendering the core crate's own golden harness already
/// proves matches Java field-for-field) — only the small, fixed top-level envelope is
/// hand-assembled, and struct-direct serialization (`ParsedName`'s own `#[derive(Serialize)]`)
/// always writes its fields in declaration order regardless of that same feature flag, so
/// nesting it verbatim here is exactly as order-safe as the envelope itself.
pub(crate) fn render_row(
    line_no: u64,
    name: &str,
    result: &Result<ParsedName, ParseError>,
) -> String {
    let mut out = String::with_capacity(128);
    out.push_str("{\"line\":");
    out.push_str(&line_no.to_string());
    out.push_str(",\"input\":");
    out.push_str(&serde_json::to_string(name).expect("a &str always serializes to a JSON string"));
    match result {
        Ok(pn) => {
            out.push_str(",\"parsed\":");
            out.push_str(&serde_json::to_string(pn).expect("ParsedName always serializes to JSON"));
        }
        Err(e) => {
            out.push_str(",\"error\":{\"type\":");
            out.push_str(
                &serde_json::to_string(&e.type_).expect("NameType always serializes to JSON"),
            );
            if let Some(code) = &e.code {
                out.push_str(",\"code\":");
                out.push_str(
                    &serde_json::to_string(code).expect("NomCode always serializes to JSON"),
                );
            }
            out.push_str(",\"message\":");
            out.push_str(
                &serde_json::to_string(&e.message)
                    .expect("a String always serializes to a JSON string"),
            );
            out.push('}');
        }
    }
    out.push('}');
    out
}

/// Opens the `parse` input stream: `None` or the literal `-` means stdin; anything else is a
/// file path. Matches the Java CLI's `--input=PATH` (`'-' = stdin`) contract.
fn open_input(path: Option<&str>) -> io::Result<Box<dyn BufRead>> {
    match path {
        None => Ok(Box::new(BufReader::new(io::stdin().lock()))),
        Some(STDIO) => Ok(Box::new(BufReader::new(io::stdin().lock()))),
        Some(p) => Ok(Box::new(BufReader::new(File::open(p)?))),
    }
}

/// Opens the `parse` output stream: `None` or the literal `-` means stdout; anything else is
/// a file path. Matches the Java CLI's `--output=PATH` (`'-' = stdout`) contract.
fn open_output(path: Option<&str>) -> io::Result<Box<dyn Write>> {
    match path {
        None => Ok(Box::new(BufWriter::new(io::stdout().lock()))),
        Some(STDIO) => Ok(Box::new(BufWriter::new(io::stdout().lock()))),
        Some(p) => Ok(Box::new(BufWriter::new(File::create(p)?))),
    }
}

// ---------------------------------------------------------------------------------------
// benchmark
// ---------------------------------------------------------------------------------------

/// Runs the `benchmark` subcommand: optionally warm up, then time a full pass parsing every
/// row of `args.input`, and print the count/total/avg/min/p50/p95/max report plus a by-name-
/// type breakdown to stdout — matches the Java CLI's `BenchmarkCli.main`. Nothing else goes to
/// stdout; the warmup banner, any `SLOW` rows, and a missing-input error all go to stderr, the
/// last of which also matches Java's exit code 2.
fn run_benchmark(args: BenchmarkArgs) -> io::Result<()> {
    let input = args.input.as_deref().unwrap_or(DEFAULT_BENCHMARK_INPUT);
    let path = Path::new(input);
    if !path.exists() {
        eprintln!("Input not found: {}", absolute_path(path).display());
        std::process::exit(2);
    }

    if args.warmup {
        eprintln!("Warming up — parsing the first {WARMUP_NAMES} names without timing…");
        warmup(path)?;
    }

    let report = run_timed(path)?;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    report.print(&mut out)?;
    out.flush()
}

/// Lexically resolves `path` against the current directory when it's relative, without
/// requiring the path to exist — matches Java's `Path.toAbsolutePath()`, used to render the
/// "Input not found" message the same way `BenchmarkCli.main` does. Falls back to `path`
/// unchanged if the current directory can't be read, matching `toAbsolutePath()`'s own
/// contract (it never fails just because the target is missing).
pub(crate) fn absolute_path(path: &Path) -> std::path::PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

/// Untimed parse pass over (at most) the first [`WARMUP_NAMES`] names in `path`, discarding
/// every result — matches the Java CLI's `BenchmarkCli.warmup`. Purely touches the parser's
/// code paths before the timed pass; a trailer of blank/comment lines never counts against the
/// 100, matching Java's `n < WARMUP_NAMES` guard around each raw-line read.
fn warmup(path: &Path) -> io::Result<()> {
    let reader = BufReader::new(File::open(path)?);
    let mut lines = reader.lines();
    let mut n = 0usize;
    while n < WARMUP_NAMES {
        let Some(line) = lines.next() else {
            break;
        };
        let raw = line?;
        if let Some(name) = extract_benchmark_name(&raw) {
            let _ = nameparser::parse(name, None, None, None);
            n += 1;
        }
    }
    Ok(())
}

/// Timed parse pass — every non-blank, non-comment line in `path` is parsed and its elapsed
/// time recorded in nanoseconds, alongside a running by-[`NameType`] tally — matches the Java
/// CLI's `BenchmarkCli.run`.
fn run_timed(path: &Path) -> io::Result<BenchmarkReport> {
    let reader = BufReader::new(File::open(path)?);
    let mut timings: Vec<u64> = Vec::with_capacity(1024);
    let mut failures: u64 = 0;
    let mut by_type = [0u64; NAME_TYPE_COUNT];

    for line in reader.lines() {
        let raw = line?;
        let Some(name) = extract_benchmark_name(&raw) else {
            continue;
        };

        let t0 = Instant::now();
        let result = nameparser::parse(name, None, None, None);
        let elapsed_nanos = t0.elapsed().as_nanos() as u64;

        if elapsed_nanos > SLOW_PARSE_THRESHOLD_NANOS {
            eprintln!("SLOW {} ms: {name}", elapsed_nanos / 1_000_000);
        }

        let type_ = match &result {
            Ok(pn) => pn.type_,
            Err(e) => e.type_,
        };
        if result.is_err() {
            failures += 1;
        }
        by_type[name_type_ordinal(type_)] += 1;
        timings.push(elapsed_nanos);
    }

    Ok(BenchmarkReport {
        timings,
        failures,
        by_type,
    })
}

/// Applies the benchmark's plain-line rule: a raw line is skipped if it is empty or starts with
/// `#`; otherwise the name is the substring before the first TAB, trimmed (col-1) — the same rule
/// `parse`'s [`extract_name`] above uses. This DELIBERATELY diverges from Java's `BenchmarkCli`
/// (which reads the whole line): splitting on TAB is a strict superset of that behaviour — a
/// single-column corpus has no TAB, so nothing changes — and it means pointing `--input` at a
/// multi-column TSV benchmarks the name column, instead of timing the raw
/// `name<TAB>author<TAB>…` row as one "name" (which silently inflates the figures and
/// mis-classifies types).
fn extract_benchmark_name(raw: &str) -> Option<&str> {
    if raw.is_empty() || raw.starts_with('#') {
        return None;
    }
    let name = raw.split('\t').next().unwrap_or(raw).trim();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Collected timings from a `benchmark` timed pass plus the by-name-type breakdown — mirrors
/// the Java CLI's `BenchmarkCli.Result`.
struct BenchmarkReport {
    /// One entry per parsed row, in nanoseconds.
    timings: Vec<u64>,
    failures: u64,
    /// Indexed by [`name_type_ordinal`]; a count of 0 means that type was never seen and is
    /// omitted from the printed breakdown — matching Java's `EnumMap`, which only ever gains an
    /// entry for a type actually encountered.
    by_type: [u64; NAME_TYPE_COUNT],
}

impl BenchmarkReport {
    fn count(&self) -> usize {
        self.timings.len()
    }

    /// Prints count/total/avg/min/p50/p95/max plus the by-type breakdown — matches the Java
    /// CLI's `BenchmarkCli.Result.report(PrintStream)` field-for-field and format-for-format,
    /// including its exact label padding.
    fn print<W: Write>(&self, out: &mut W) -> io::Result<()> {
        if self.timings.is_empty() {
            return writeln!(out, "No timings collected.");
        }
        let mut sorted = self.timings.clone();
        sorted.sort_unstable();
        let min = sorted[0];
        let max = sorted[sorted.len() - 1];
        let sum: u64 = sorted.iter().sum();
        let avg = sum as f64 / sorted.len() as f64;
        let p50 = percentile(&sorted, 50);
        let p95 = percentile(&sorted, 95);

        writeln!(
            out,
            "Parsed names: {} ({} unparsable)",
            self.count(),
            self.failures
        )?;
        writeln!(out, "Total:   {}", fmt_nanos(sum as f64))?;
        writeln!(out, "Average: {}", fmt_nanos(avg))?;
        writeln!(out, "Min:     {}", fmt_nanos(min as f64))?;
        writeln!(out, "p50:     {}", fmt_nanos(p50 as f64))?;
        writeln!(out, "p95:     {}", fmt_nanos(p95 as f64))?;
        writeln!(out, "Max:     {}", fmt_nanos(max as f64))?;
        writeln!(out)?;
        writeln!(out, "Breakdown by name type:")?;

        let mut entries: Vec<(NameType, u64)> = NAME_TYPES
            .iter()
            .copied()
            .zip(self.by_type.iter().copied())
            .filter(|(_, c)| *c > 0)
            .collect();
        // Stable sort, descending by count — ties keep `entries`' incoming order, which is
        // NAME_TYPES' ordinal order, matching Java's `EnumMap`-iteration-then-stable-sort.
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        for (t, c) in entries {
            writeln!(out, "  {:<20} {c}", name_type_label(t))?;
        }
        Ok(())
    }
}

/// Same nearest-rank percentile formula as the Java CLI's `BenchmarkCli.Result.percentile`.
fn percentile(sorted: &[u64], p: u32) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = (f64::from(p) / 100.0 * sorted.len() as f64).ceil() as i64 - 1;
    let idx = idx.clamp(0, sorted.len() as i64 - 1) as usize;
    sorted[idx]
}

/// Same nanosecond-magnitude formatting as the Java CLI's `BenchmarkCli.Result.fmt`: ms above 1
/// million ns, µs above 1 thousand ns, otherwise whole ns.
fn fmt_nanos(nanos: f64) -> String {
    if nanos >= 1_000_000.0 {
        format!("{:.2} ms", nanos / 1_000_000.0)
    } else if nanos >= 1_000.0 {
        format!("{:.2} µs", nanos / 1_000.0)
    } else {
        format!("{:.0} ns", nanos)
    }
}

/// Java `Enum.name()` for a [`NameType`] (e.g. `"SCIENTIFIC"`), reusing the same
/// serialize-then-trim-quotes idiom as the core crate's own (crate-private) `model::java_name`
/// helper, so these breakdown labels are guaranteed to match the `"type"` field `parse` already
/// writes to JSON, without re-exporting that helper across the crate boundary.
fn name_type_label(t: NameType) -> String {
    serde_json::to_string(&t)
        .expect("NameType always serializes to a JSON string")
        .trim_matches('"')
        .to_string()
}

// ---------------------------------------------------------------------------------------
// compare
// ---------------------------------------------------------------------------------------

/// Field names — the exact JSON wire spelling `ParsedName` serialises to (see
/// `nameparser::model::name`) — whose value is backed by a Java `Set`/`Map`-shaped collection
/// (`warnings`: `HashSet<String>`; `notho`: `EnumSet<NamePart>`; `epithetQualifier`:
/// `EnumMap<NamePart, String>`) on the Java oracle side, and are therefore compared
/// order-insensitively below: the exact same 3-key definition (and rationale) as the core
/// crate's own golden harness's `UNORDERED_FIELD_KEYS`
/// (`crates/nameparser/tests/parse_golden.rs`) — matching "the golden harness's parity
/// definition" for these fields, not just `warnings` alone, so a real `notho`/`epithetQualifier`
/// order artifact surfaced by a corpus that harness doesn't cover would not be misreported as a
/// bug either. In practice only `warnings` has actually been observed to disagree in order
/// (`notho`'s own dedicated unit tests in `model::name` prove the Rust side always emits
/// ordinal order, matching `EnumSet`'s deterministic ordinal iteration on the Java side;
/// `epithetQualifier` is a JSON *object*, which the recursive comparison below already treats
/// order-insensitively by walking the union of both sides' keys regardless of declaration
/// order) — but all three are listed here, not just the one known to matter today.
const UNORDERED_FIELD_KEYS: [&str; 3] = ["warnings", "notho", "epithetQualifier"];

/// Options for `nameparser-cli compare`, mirroring the Java CLI's `CompareCli` option set
/// (`--output`, `--ignore-whitespace`, `--max-diffs`) — see this module's doc comment for the
/// full behavioural contract this reproduces, and for the deliberate narrower option surface
/// (no `--a=`/`--b=`/positional-third-arg alternates).
#[derive(Args)]
struct CompareArgs {
    /// First JSONL file, as produced by `parse`.
    a: String,

    /// Second JSONL file, as produced by `parse` — same source and line order as `a`.
    b: String,

    /// Write per-row diffs here instead of stdout.
    #[arg(long)]
    output: Option<String>,

    /// Strip whitespace from string leaves before comparing (formatting-only spacing
    /// differences won't count as a diff).
    #[arg(long)]
    ignore_whitespace: bool,

    /// Cap the per-row diff dump at this many differing rows. The aggregate counts (rows
    /// differing, status transitions, top fields) are never capped.
    #[arg(long, default_value_t = 100)]
    max_diffs: usize,
}

/// The classification `compare` buckets each row into, mirroring the Java CLI's
/// `CompareCli.Status` — used only for the `statusTransitions` breakdown (e.g. `PARSED→ERROR`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RowStatus {
    Parsed,
    Error,
    Empty,
}

impl RowStatus {
    /// Classifies one JSONL row: a present, non-null `error` key wins over a present, non-null
    /// `parsed` key (a row should never have both, but this matches Java's `if`/`else if` order
    /// exactly in case it ever does); otherwise the row is `Empty`.
    fn of(row: &Value) -> Self {
        let present = |k: &str| row.get(k).is_some_and(|v| !v.is_null());
        if present("error") {
            RowStatus::Error
        } else if present("parsed") {
            RowStatus::Parsed
        } else {
            RowStatus::Empty
        }
    }

    fn label(self) -> &'static str {
        match self {
            RowStatus::Parsed => "PARSED",
            RowStatus::Error => "ERROR",
            RowStatus::Empty => "EMPTY",
        }
    }
}

/// One differing leaf value, keyed by its dotted/bracketed path (e.g.
/// `parsed.combinationAuthorship.authors[0]`) — mirrors the Java CLI's `CompareCli.Diff`.
#[derive(Debug)]
struct FieldDiff {
    path: String,
    left: String,
    right: String,
}

/// A simple insertion-order-preserving counter, replicating Java's `LinkedHashMap<String,
/// Long>` + `Map.Entry.comparingByValue().reversed()` stream: entries keep first-seen order
/// among themselves, and [`Self::sorted_desc`] sorts by count descending with a *stable* sort,
/// so ties keep that first-seen order — matching Java's behaviour exactly. A linear-scan
/// `Vec<(String, u64)>` rather than a `HashMap` is deliberate: the cardinality here is bounded
/// by the small, fixed JSON schema (a few dozen field paths at most, 9 possible status-pairs),
/// never by row count, so this stays cheap in practice regardless of corpus size, and needs no
/// extra dependency (an ordered map isn't in `std`).
#[derive(Default)]
struct OrderedCounter {
    entries: Vec<(String, u64)>,
}

impl OrderedCounter {
    fn increment(&mut self, key: &str) {
        match self.entries.iter_mut().find(|(k, _)| k == key) {
            Some(e) => e.1 += 1,
            None => self.entries.push((key.to_string(), 1)),
        }
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Entries sorted descending by count; ties keep insertion order (Rust's `sort_by` is
    /// stable), matching Java's `Map.Entry.comparingByValue().reversed()` over a
    /// `LinkedHashMap`'s stream.
    fn sorted_desc(&self) -> Vec<(&str, u64)> {
        let mut v: Vec<(&str, u64)> = self.entries.iter().map(|(k, c)| (k.as_str(), *c)).collect();
        v.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
        v
    }
}

/// Running counters + metadata for one `compare` run — mirrors the Java CLI's
/// `CompareCli.Report`.
struct CompareReport {
    file_a: String,
    file_b: String,
    ignore_whitespace: bool,
    rows_compared: u64,
    rows_differed: u64,
    line_number_mismatches: u64,
    extra_rows_a: u64,
    extra_rows_b: u64,
    status_transitions: OrderedCounter,
    field_diff_counts: OrderedCounter,
}

impl CompareReport {
    fn new(file_a: String, file_b: String, ignore_whitespace: bool) -> Self {
        CompareReport {
            file_a,
            file_b,
            ignore_whitespace,
            rows_compared: 0,
            rows_differed: 0,
            line_number_mismatches: 0,
            extra_rows_a: 0,
            extra_rows_b: 0,
            status_transitions: OrderedCounter::default(),
            field_diff_counts: OrderedCounter::default(),
        }
    }

    /// Prints the aggregate summary — matches the Java CLI's `CompareCli.Report.printSummary`
    /// section-for-section (modulo Java's `%,d` thousands-separator formatting, which this
    /// skips: the CLI crate's dependency budget is `clap` only, so no formatting crate was
    /// added for a cosmetic difference that affects no parity-relevant behaviour).
    fn print_summary<W: Write>(&self, out: &mut W) -> io::Result<()> {
        writeln!(out, "=== JSONL comparison summary ===")?;
        writeln!(out, "A: {}", self.file_a)?;
        writeln!(out, "B: {}", self.file_b)?;
        writeln!(out, "ignore-whitespace: {}", self.ignore_whitespace)?;
        writeln!(out, "Rows compared:      {}", self.rows_compared)?;
        writeln!(
            out,
            "Rows identical:     {}",
            self.rows_compared - self.rows_differed
        )?;
        let pct = if self.rows_compared == 0 {
            0.0
        } else {
            100.0 * self.rows_differed as f64 / self.rows_compared as f64
        };
        writeln!(
            out,
            "Rows differing:     {} ({pct:.2}%)",
            self.rows_differed
        )?;
        if self.line_number_mismatches > 0 {
            writeln!(
                out,
                "Line-number mismatches: {}",
                self.line_number_mismatches
            )?;
        }
        if self.extra_rows_a > 0 {
            writeln!(out, "Extra rows in A: {}", self.extra_rows_a)?;
        }
        if self.extra_rows_b > 0 {
            writeln!(out, "Extra rows in B: {}", self.extra_rows_b)?;
        }
        if !self.status_transitions.is_empty() {
            writeln!(out)?;
            writeln!(out, "Status transitions (parsed/error/empty):")?;
            for (k, c) in self.status_transitions.sorted_desc() {
                writeln!(out, "  {k:<20} {c}")?;
            }
        }
        if !self.field_diff_counts.is_empty() {
            writeln!(out)?;
            writeln!(out, "Top differing fields:")?;
            for (k, c) in self.field_diff_counts.sorted_desc().into_iter().take(40) {
                writeln!(out, "  {k:<44} {c}")?;
            }
        }
        Ok(())
    }
}

/// Runs the `compare` subcommand end to end: open both files, stream them in lockstep, write
/// per-row diffs to stdout or `--output`, and always print the summary to stdout afterwards —
/// matches the Java CLI's `CompareCli.main`.
fn run_compare(args: CompareArgs) -> io::Result<()> {
    let file_a = File::open(&args.a)
        .map_err(|e| io::Error::new(e.kind(), format!("cannot open {}: {e}", args.a)))?;
    let file_b = File::open(&args.b)
        .map_err(|e| io::Error::new(e.kind(), format!("cannot open {}: {e}", args.b)))?;

    let stdout = io::stdout();
    match args.output.as_deref() {
        None => {
            let mut sink = stdout.lock();
            let report = compare_streams(
                BufReader::new(file_a),
                BufReader::new(file_b),
                &args,
                &mut sink,
            )?;
            report.print_summary(&mut sink)
        }
        Some(path) => {
            let mut file_sink = BufWriter::new(File::create(path)?);
            let report = compare_streams(
                BufReader::new(file_a),
                BufReader::new(file_b),
                &args,
                &mut file_sink,
            )?;
            file_sink.flush()?;
            let mut out = stdout.lock();
            report.print_summary(&mut out)?;
            writeln!(
                out,
                "Per-row diffs written to {}",
                absolute_path(Path::new(path)).display()
            )
        }
    }
}

/// Streams `a`/`b` in lockstep, comparing one JSONL row at a time and writing per-row diffs to
/// `diff_sink` — matches the Java CLI's `CompareCli.compare`.
fn compare_streams<A: BufRead, B: BufRead, W: Write>(
    a: A,
    b: B,
    args: &CompareArgs,
    diff_sink: &mut W,
) -> io::Result<CompareReport> {
    let mut report = CompareReport::new(args.a.clone(), args.b.clone(), args.ignore_whitespace);
    let mut lines_a = a.lines();
    let mut lines_b = b.lines();

    loop {
        let la = lines_a.next().transpose()?;
        let lb = lines_b.next().transpose()?;
        match (la, lb) {
            (None, None) => break,
            (Some(_), None) => {
                let mut extra = 1u64;
                while lines_a.next().transpose()?.is_some() {
                    extra += 1;
                }
                report.extra_rows_a = extra;
                break;
            }
            (None, Some(_)) => {
                let mut extra = 1u64;
                while lines_b.next().transpose()?.is_some() {
                    extra += 1;
                }
                report.extra_rows_b = extra;
                break;
            }
            (Some(la), Some(lb)) => {
                report.rows_compared += 1;
                let oa: Value = serde_json::from_str(&la)
                    .map_err(|e| invalid_json_error(&args.a, report.rows_compared, &la, e))?;
                let ob: Value = serde_json::from_str(&lb)
                    .map_err(|e| invalid_json_error(&args.b, report.rows_compared, &lb, e))?;
                compare_row(&mut report, &oa, &ob, args, diff_sink)?;
            }
        }
    }
    Ok(report)
}

/// Wraps a per-row JSON parse failure with enough context (file, 1-based row number, an
/// abbreviated view of the offending line) to locate the bad input — where Java's
/// `JsonParser.parseString(...).getAsJsonObject()` would instead throw an uncaught exception
/// straight out of `main`.
fn invalid_json_error(file: &str, row: u64, raw: &str, err: serde_json::Error) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "invalid JSON on line {row} of {file}: {err} (raw: {})",
            abbreviate(raw)
        ),
    )
}

/// Compares one row pair: tallies a `line`-field mismatch and a status transition if either
/// applies, then walks the full value tree for differing leaves — matches the loop body of the
/// Java CLI's `CompareCli.compare`.
fn compare_row<W: Write>(
    report: &mut CompareReport,
    oa: &Value,
    ob: &Value,
    args: &CompareArgs,
    diff_sink: &mut W,
) -> io::Result<()> {
    let line_a = oa.get("line").and_then(Value::as_i64).unwrap_or(-1);
    let line_b = ob.get("line").and_then(Value::as_i64).unwrap_or(-1);
    if line_a != line_b {
        report.line_number_mismatches += 1;
    }

    let sa = RowStatus::of(oa);
    let sb = RowStatus::of(ob);
    if sa != sb {
        report
            .status_transitions
            .increment(&format!("{}→{}", sa.label(), sb.label()));
    }

    let mut diffs = Vec::new();
    diff_element("", oa, ob, args.ignore_whitespace, &mut diffs);
    if diffs.is_empty() {
        return Ok(());
    }

    report.rows_differed += 1;
    for d in &diffs {
        report.field_diff_counts.increment(&d.path);
    }
    if report.rows_differed <= args.max_diffs as u64 {
        let input = oa
            .get("input")
            .and_then(Value::as_str)
            .or_else(|| ob.get("input").and_then(Value::as_str))
            .unwrap_or("?");
        writeln!(
            diff_sink,
            "Line {line_a} \"{input}\" (status {} vs {}):",
            sa.label(),
            sb.label()
        )?;
        for d in &diffs {
            writeln!(
                diff_sink,
                "  {:<44}  {}  →  {}",
                d.path,
                abbreviate(&d.left),
                abbreviate(&d.right)
            )?;
        }
    } else if report.rows_differed == args.max_diffs as u64 + 1 {
        writeln!(
            diff_sink,
            "… further per-row diffs suppressed (--max-diffs={})",
            args.max_diffs
        )?;
    }
    Ok(())
}

/// Recursively walks `a`/`b`, appending a [`FieldDiff`] for every leaf where they disagree —
/// matches the Java CLI's `CompareCli.diffElement`, plus the `warnings`/`notho`/
/// `epithetQualifier` order-insensitive handling this module's doc comment describes.
fn diff_element(path: &str, a: &Value, b: &Value, ignore_ws: bool, out: &mut Vec<FieldDiff>) {
    if json_eq(a, b, ignore_ws) {
        return;
    }
    match (a, b) {
        (Value::Object(oa), Value::Object(ob)) => {
            let mut keys: BTreeSet<&String> = BTreeSet::new();
            keys.extend(oa.keys());
            keys.extend(ob.keys());
            for k in keys {
                let va = oa.get(k).cloned().unwrap_or(Value::Null);
                let vb = ob.get(k).cloned().unwrap_or(Value::Null);
                let (va, vb) = canonicalize_for_key(k, va, vb);
                let child_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                diff_element(&child_path, &va, &vb, ignore_ws, out);
            }
        }
        (Value::Array(aa), Value::Array(ab)) => {
            let n = aa.len().max(ab.len());
            for i in 0..n {
                let va = aa.get(i).cloned().unwrap_or(Value::Null);
                let vb = ab.get(i).cloned().unwrap_or(Value::Null);
                diff_element(&format!("{path}[{i}]"), &va, &vb, ignore_ws, out);
            }
        }
        _ => out.push(FieldDiff {
            path: path.to_string(),
            left: render(a),
            right: render(b),
        }),
    }
}

/// If `key` is one of [`UNORDERED_FIELD_KEYS`] and the corresponding value is a JSON array,
/// returns both values with that array sorted by each element's rendered JSON text (a JSON
/// *object* value under one of these keys — `epithetQualifier` — already compares
/// order-insensitively via the object branch above, so it passes through unchanged here).
/// Otherwise returns the pair unchanged.
fn canonicalize_for_key(key: &str, a: Value, b: Value) -> (Value, Value) {
    if UNORDERED_FIELD_KEYS.contains(&key) {
        (sort_if_array(a), sort_if_array(b))
    } else {
        (a, b)
    }
}

fn sort_if_array(v: Value) -> Value {
    match v {
        Value::Array(mut items) => {
            items.sort_by_key(|x| x.to_string());
            Value::Array(items)
        }
        other => other,
    }
}

/// Structural equality mirroring the Java CLI's `CompareCli.jsonEquals`: null only equals null;
/// strings compare (optionally whitespace-stripped); arrays compare positionally (the
/// order-insensitive handling for `warnings`/`notho` happens one level up, in
/// [`canonicalize_for_key`], before this function ever sees those arrays); objects compare by
/// the union of keys regardless of declaration order (so `epithetQualifier` needs no special
/// casing here); numbers/bools use plain `Value` equality.
fn json_eq(a: &Value, b: &Value, ignore_ws: bool) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Null, _) | (_, Value::Null) => false,
        (Value::String(sa), Value::String(sb)) => {
            if ignore_ws {
                strip_ws(sa) == strip_ws(sb)
            } else {
                sa == sb
            }
        }
        (Value::Array(aa), Value::Array(ab)) => {
            aa.len() == ab.len()
                && aa
                    .iter()
                    .zip(ab.iter())
                    .all(|(x, y)| json_eq(x, y, ignore_ws))
        }
        (Value::Object(oa), Value::Object(ob)) => {
            oa.len() == ob.len()
                && oa
                    .iter()
                    .all(|(k, v)| ob.get(k).is_some_and(|w| json_eq(v, w, ignore_ws)))
        }
        (x, y) => x == y,
    }
}

fn strip_ws(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Renders one JSON leaf for a per-row diff line — matches the Java CLI's
/// `CompareCli.render`: strings keep their quotes, numbers/bools print bare, arrays/objects
/// print their full compact JSON text (only reached here for a top-level type mismatch, e.g.
/// `parsed` present on one side and absent on the other).
fn render(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::String(s) => format!("\"{s}\""),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(_) | Value::Object(_) => v.to_string(),
    }
}

/// Truncates a rendered diff value to 80 characters, matching the Java CLI's
/// `CompareCli.abbreviate` (counting Java `char`s there; Rust `char`s here — both are a
/// display-truncation nicety, not a parity-relevant behaviour).
fn abbreviate(s: &str) -> String {
    const MAX: usize = 80;
    if s.chars().count() <= MAX {
        return s.to_string();
    }
    let truncated: String = s.chars().take(MAX - 1).collect();
    format!("{truncated}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use nameparser::model::{NameType, NomCode};

    #[test]
    fn extract_name_skips_blank_and_comment_lines() {
        assert_eq!(extract_name(""), None);
        assert_eq!(extract_name("# a comment"), None);
    }

    #[test]
    fn extract_name_takes_the_substring_before_the_first_tab_and_trims_it() {
        assert_eq!(
            extract_name("Abies alba\tsome other column"),
            Some("Abies alba")
        );
        assert_eq!(extract_name("  Abies alba  \tx"), Some("Abies alba"));
    }

    #[test]
    fn extract_name_skips_a_lone_scientific_name_header() {
        assert_eq!(extract_name("scientificName"), None);
        assert_eq!(extract_name("scientificName\tauthorship"), None);
    }

    #[test]
    fn extract_name_skips_a_line_that_is_only_whitespace_before_the_tab() {
        assert_eq!(extract_name("   \tx"), None);
    }

    #[test]
    fn extract_name_keeps_a_real_name_even_if_it_only_starts_with_hash_after_trim() {
        // Only a raw line *starting* with '#' is a comment — a literal '#' that isn't the
        // first character of the raw line must not be treated as one.
        assert_eq!(extract_name("Abies # alba"), Some("Abies # alba"));
    }

    #[test]
    fn render_row_matches_the_documented_success_shape() {
        let pn = nameparser::parse("Abies alba Mill.", None, None, None).unwrap();
        let row = render_row(2, "Abies alba Mill.", &Ok(pn));
        assert_eq!(
            row,
            r#"{"line":2,"input":"Abies alba Mill.","parsed":{"rank":"SPECIES","genus":"Abies","specificEpithet":"alba","candidatus":false,"type":"SCIENTIFIC","extinct":false,"doubtful":false,"manuscript":false,"state":"COMPLETE","warnings":[],"combinationAuthorship":{"authors":["Mill."],"exAuthors":[]},"basionymAuthorship":{"authors":[],"exAuthors":[]}}}"#
        );
    }

    #[test]
    fn render_row_matches_the_documented_error_shape_and_omits_code_when_absent() {
        let err = ParseError::new(NameType::Other, None, "???");
        let row = render_row(9, "???", &Err(err));
        assert_eq!(
            row,
            r#"{"line":9,"input":"???","error":{"type":"OTHER","message":"Unparsable OTHER name: ???"}}"#
        );
    }

    #[test]
    fn render_row_includes_code_when_present() {
        let err = ParseError::new(NameType::Other, Some(NomCode::Virus), "Tobacco mosaic virus");
        let row = render_row(1, "Tobacco mosaic virus", &Err(err));
        assert_eq!(
            row,
            r#"{"line":1,"input":"Tobacco mosaic virus","error":{"type":"OTHER","code":"VIRUS","message":"Unparsable OTHER name: Tobacco mosaic virus"}}"#
        );
    }

    #[test]
    fn render_row_escapes_the_input_name_as_a_json_string() {
        let err = ParseError::new(NameType::Other, None, "a \"quoted\" name");
        let row = render_row(1, "a \"quoted\" name", &Err(err));
        assert!(row.starts_with(r#"{"line":1,"input":"a \"quoted\" name","error":"#));
    }

    // ---- benchmark ----

    #[test]
    fn extract_benchmark_name_skips_blank_and_comment_lines() {
        assert_eq!(extract_benchmark_name(""), None);
        assert_eq!(extract_benchmark_name("# a comment"), None);
        assert_eq!(extract_benchmark_name("   "), None);
    }

    #[test]
    fn extract_benchmark_name_splits_on_the_first_tab() {
        // The benchmark reader splits on the first TAB (col-1), like `extract_name` (used by
        // `parse`) — deliberately unlike Java's `BenchmarkCli` — so a multi-column TSV benchmarks
        // the name column instead of the raw `name<TAB>…` row.
        assert_eq!(
            extract_benchmark_name("  Abies alba\tsome other column  "),
            Some("Abies alba")
        );
    }

    #[test]
    fn extract_benchmark_name_keeps_a_real_name_even_if_it_only_starts_with_hash_after_trim() {
        assert_eq!(extract_benchmark_name("Abies # alba"), Some("Abies # alba"));
    }

    #[test]
    fn name_type_ordinal_is_a_bijection_onto_0_dot_dot_len() {
        // Every NAME_TYPES entry must round-trip through its own ordinal, and the ordinals must
        // be exactly 0..NAME_TYPE_COUNT with no gaps or repeats — otherwise `by_type` counts
        // would silently collide or leave a slot dead.
        let mut seen = [false; NAME_TYPE_COUNT];
        for (i, &t) in NAME_TYPES.iter().enumerate() {
            let ord = name_type_ordinal(t);
            assert_eq!(ord, i, "NAME_TYPES[{i}] ordinal mismatch");
            assert!(!seen[ord], "ordinal {ord} produced twice");
            seen[ord] = true;
        }
        assert!(seen.iter().all(|&s| s), "not every ordinal slot was hit");
    }

    #[test]
    fn name_type_label_matches_the_json_type_field_spelling() {
        assert_eq!(name_type_label(NameType::Scientific), "SCIENTIFIC");
        assert_eq!(name_type_label(NameType::Formula), "FORMULA");
        assert_eq!(name_type_label(NameType::Informal), "INFORMAL");
        assert_eq!(name_type_label(NameType::Placeholder), "PLACEHOLDER");
        assert_eq!(name_type_label(NameType::Other), "OTHER");
    }

    #[test]
    fn percentile_matches_the_java_nearest_rank_formula_on_a_10_element_series() {
        let sorted: Vec<u64> = (1..=10).collect(); // 1..=10
        // ceil(50/100 * 10) - 1 = 4 -> sorted[4] == 5
        assert_eq!(percentile(&sorted, 50), 5);
        // ceil(95/100 * 10) - 1 = ceil(9.5) - 1 = 10 - 1 = 9 -> sorted[9] == 10
        assert_eq!(percentile(&sorted, 95), 10);
        assert_eq!(percentile(&sorted, 100), 10);
        assert_eq!(percentile(&[], 50), 0);
    }

    #[test]
    fn fmt_nanos_switches_units_at_the_documented_thresholds() {
        assert_eq!(fmt_nanos(999.0), "999 ns");
        assert_eq!(fmt_nanos(1_000.0), "1.00 µs");
        assert_eq!(fmt_nanos(1_500.0), "1.50 µs");
        assert_eq!(fmt_nanos(999_999.0), "1000.00 µs");
        assert_eq!(fmt_nanos(1_000_000.0), "1.00 ms");
        assert_eq!(fmt_nanos(2_500_000.0), "2.50 ms");
    }

    #[test]
    fn benchmark_report_print_matches_the_documented_report_shape() {
        // Three fake rows (1000ns, 2000ns, 3000ns) — small enough to hand-check every stat.
        let report = BenchmarkReport {
            timings: vec![2_000, 1_000, 3_000],
            failures: 1,
            by_type: {
                let mut t = [0u64; NAME_TYPE_COUNT];
                t[name_type_ordinal(NameType::Scientific)] = 2;
                t[name_type_ordinal(NameType::Other)] = 1;
                t
            },
        };
        let mut buf: Vec<u8> = Vec::new();
        report.print(&mut buf).expect("writing to a Vec<u8> cannot fail");
        let out = String::from_utf8(buf).expect("report is ASCII/UTF-8");
        assert_eq!(
            out,
            "Parsed names: 3 (1 unparsable)\n\
             Total:   6.00 µs\n\
             Average: 2.00 µs\n\
             Min:     1.00 µs\n\
             p50:     2.00 µs\n\
             p95:     3.00 µs\n\
             Max:     3.00 µs\n\
             \n\
             Breakdown by name type:\n\
             \x20\x20SCIENTIFIC           2\n\
             \x20\x20OTHER                1\n"
        );
    }

    #[test]
    fn benchmark_report_print_reports_no_timings_collected_when_empty() {
        let report = BenchmarkReport {
            timings: vec![],
            failures: 0,
            by_type: [0u64; NAME_TYPE_COUNT],
        };
        let mut buf: Vec<u8> = Vec::new();
        report.print(&mut buf).expect("writing to a Vec<u8> cannot fail");
        assert_eq!(String::from_utf8(buf).unwrap(), "No timings collected.\n");
    }

    // ---- compare ----

    fn v(json: &str) -> Value {
        serde_json::from_str(json).expect("test fixture must be valid JSON")
    }

    #[test]
    fn row_status_of_classifies_parsed_error_and_empty() {
        assert_eq!(
            RowStatus::of(&v(r#"{"parsed":{"rank":"SPECIES"}}"#)),
            RowStatus::Parsed
        );
        assert_eq!(
            RowStatus::of(&v(r#"{"error":{"type":"OTHER"}}"#)),
            RowStatus::Error
        );
        assert_eq!(RowStatus::of(&v(r#"{"line":1}"#)), RowStatus::Empty);
        assert_eq!(RowStatus::of(&v(r#"{"error":null}"#)), RowStatus::Empty);
        // A (pathological) row with both present: error wins, matching Java's if/else-if order.
        assert_eq!(
            RowStatus::of(&v(r#"{"parsed":{},"error":{}}"#)),
            RowStatus::Error
        );
    }

    #[test]
    fn diff_element_finds_a_nested_field_difference() {
        let a = v(r#"{"parsed":{"genus":"Abies","rank":"SPECIES"}}"#);
        let b = v(r#"{"parsed":{"genus":"Abia","rank":"SPECIES"}}"#);
        let mut diffs = Vec::new();
        diff_element("", &a, &b, false, &mut diffs);
        assert_eq!(diffs.len(), 1, "{diffs:?}");
        assert_eq!(diffs[0].path, "parsed.genus");
        assert_eq!(diffs[0].left, "\"Abies\"");
        assert_eq!(diffs[0].right, "\"Abia\"");
    }

    #[test]
    fn diff_element_reports_no_diffs_for_identical_rows() {
        let a = v(r#"{"line":1,"input":"x","parsed":{"rank":"SPECIES","genus":"Abies"}}"#);
        let b = v(r#"{"line":1,"input":"x","parsed":{"rank":"SPECIES","genus":"Abies"}}"#);
        let mut diffs = Vec::new();
        diff_element("", &a, &b, false, &mut diffs);
        assert!(diffs.is_empty(), "{diffs:?}");
    }

    #[test]
    fn diff_element_treats_warnings_as_an_order_insensitive_set() {
        let a = v(r#"{"parsed":{"warnings":["NAME_UNPARSABLE","HOMOGLYPH"]}}"#);
        let b = v(r#"{"parsed":{"warnings":["HOMOGLYPH","NAME_UNPARSABLE"]}}"#);
        let mut diffs = Vec::new();
        diff_element("", &a, &b, false, &mut diffs);
        assert!(
            diffs.is_empty(),
            "warnings differing only in order must not be reported: {diffs:?}"
        );
    }

    #[test]
    fn diff_element_still_flags_a_genuine_warnings_content_difference() {
        let a = v(r#"{"parsed":{"warnings":["HOMOGLYPH"]}}"#);
        let b = v(r#"{"parsed":{"warnings":["HOMOGLYPH","NAME_UNPARSABLE"]}}"#);
        let mut diffs = Vec::new();
        diff_element("", &a, &b, false, &mut diffs);
        assert!(
            !diffs.is_empty(),
            "a genuine content difference in warnings must still be flagged"
        );
        assert!(diffs.iter().any(|d| d.path.starts_with("parsed.warnings")));
    }

    #[test]
    fn diff_element_notho_is_also_order_insensitive() {
        // notho: EnumSet<NamePart> on the Java side, Vec<NamePart> here — same "unordered
        // collection" shape as warnings, even though (per this crate's own add_notho tests)
        // it's never actually been observed out of order in practice.
        let a = v(r#"{"parsed":{"notho":["SPECIFIC","INFRASPECIFIC"]}}"#);
        let b = v(r#"{"parsed":{"notho":["INFRASPECIFIC","SPECIFIC"]}}"#);
        let mut diffs = Vec::new();
        diff_element("", &a, &b, false, &mut diffs);
        assert!(
            diffs.is_empty(),
            "notho differing only in order must not be reported: {diffs:?}"
        );
    }

    #[test]
    fn diff_element_epithet_qualifier_object_is_key_order_insensitive() {
        let a = v(r#"{"parsed":{"epithetQualifier":{"SPECIFIC":"cf.","INFRASPECIFIC":"aff."}}}"#);
        let b = v(r#"{"parsed":{"epithetQualifier":{"INFRASPECIFIC":"aff.","SPECIFIC":"cf."}}}"#);
        let mut diffs = Vec::new();
        diff_element("", &a, &b, false, &mut diffs);
        assert!(diffs.is_empty(), "{diffs:?}");
    }

    #[test]
    fn diff_element_ignore_whitespace_strips_string_leaves() {
        let a = v(r#"{"parsed":{"genus":"Abies  alba"}}"#);
        let b = v(r#"{"parsed":{"genus":"Abiesalba"}}"#);

        let mut diffs = Vec::new();
        diff_element("", &a, &b, false, &mut diffs);
        assert_eq!(diffs.len(), 1, "must differ when whitespace is significant");

        let mut diffs_ws = Vec::new();
        diff_element("", &a, &b, true, &mut diffs_ws);
        assert!(
            diffs_ws.is_empty(),
            "--ignore-whitespace must strip whitespace before comparing"
        );
    }

    #[test]
    fn diff_element_array_path_uses_bracket_index_notation() {
        let a = v(r#"{"parsed":{"combinationAuthorship":{"authors":["Mill."]}}}"#);
        let b = v(r#"{"parsed":{"combinationAuthorship":{"authors":["Miller"]}}}"#);
        let mut diffs = Vec::new();
        diff_element("", &a, &b, false, &mut diffs);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "parsed.combinationAuthorship.authors[0]");
    }

    #[test]
    fn ordered_counter_sorted_desc_is_stable_on_ties() {
        let mut c = OrderedCounter::default();
        c.increment("b");
        c.increment("a");
        c.increment("a");
        c.increment("c");
        c.increment("c");
        // "a" and "c" tie at 2; "a" was first-seen before "c", so it must sort first.
        assert_eq!(c.sorted_desc(), vec![("a", 2), ("c", 2), ("b", 1)]);
    }

    #[test]
    fn abbreviate_truncates_long_strings_and_keeps_short_ones() {
        assert_eq!(abbreviate("short"), "short");
        let long = "a".repeat(100);
        let truncated = abbreviate(&long);
        assert_eq!(truncated.chars().count(), 80);
        assert!(truncated.ends_with('…'));
    }

    #[test]
    fn compare_report_print_summary_matches_the_documented_shape() {
        let mut report = CompareReport::new("a.jsonl".to_string(), "b.jsonl".to_string(), false);
        report.rows_compared = 4;
        report.rows_differed = 1;
        report.status_transitions.increment("PARSED_TO_ERROR");
        report.field_diff_counts.increment("genus");

        let mut buf: Vec<u8> = Vec::new();
        report.print_summary(&mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();

        let expected = format!(
            "=== JSONL comparison summary ===\n\
             A: a.jsonl\n\
             B: b.jsonl\n\
             ignore-whitespace: false\n\
             Rows compared:      4\n\
             Rows identical:     3\n\
             Rows differing:     1 (25.00%)\n\
             \n\
             Status transitions (parsed/error/empty):\n\
             \x20\x20{:<20} 1\n\
             \n\
             Top differing fields:\n\
             \x20\x20{:<44} 1\n",
            "PARSED_TO_ERROR", "genus"
        );
        assert_eq!(out, expected);
    }

    #[test]
    fn compare_report_print_summary_omits_optional_sections_when_empty() {
        let report = CompareReport::new("a.jsonl".to_string(), "b.jsonl".to_string(), true);
        let mut buf: Vec<u8> = Vec::new();
        report.print_summary(&mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert_eq!(
            out,
            "=== JSONL comparison summary ===\n\
             A: a.jsonl\n\
             B: b.jsonl\n\
             ignore-whitespace: true\n\
             Rows compared:      0\n\
             Rows identical:     0\n\
             Rows differing:     0 (0.00%)\n"
        );
    }

    #[test]
    fn compare_streams_reports_extra_rows_and_a_status_transition() {
        use std::io::Cursor;

        let a = Cursor::new(
            "{\"line\":1,\"input\":\"x\",\"parsed\":{\"rank\":\"SPECIES\"}}\n".as_bytes(),
        );
        let b = Cursor::new(
            "{\"line\":1,\"input\":\"x\",\"error\":{\"type\":\"OTHER\",\"message\":\"m\"}}\n\
             {\"line\":2,\"input\":\"y\",\"parsed\":{\"rank\":\"SPECIES\"}}\n"
                .as_bytes(),
        );
        let args = CompareArgs {
            a: "a.jsonl".to_string(),
            b: "b.jsonl".to_string(),
            output: None,
            ignore_whitespace: false,
            max_diffs: 100,
        };
        let mut sink: Vec<u8> = Vec::new();
        let report = compare_streams(a, b, &args, &mut sink).expect("compare_streams must succeed");

        assert_eq!(report.rows_compared, 1);
        assert_eq!(report.rows_differed, 1);
        assert_eq!(report.extra_rows_b, 1);
        assert_eq!(
            report.status_transitions.sorted_desc(),
            vec![("PARSED→ERROR", 1)]
        );
    }
}
