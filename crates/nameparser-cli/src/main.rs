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
//!   - `compare` subcommand (Phase 2 Task 3).
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

use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use clap::{Args, Parser, Subcommand, ValueEnum};
use nameparser::model::{NameType, ParseError, ParsedName};

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
fn extract_name(raw: &str) -> Option<&str> {
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
fn render_row(line_no: u64, name: &str, result: &Result<ParsedName, ParseError>) -> String {
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
fn absolute_path(path: &Path) -> std::path::PathBuf {
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

/// Applies the benchmark's plain-line rule, mirroring the Java CLI's `BenchmarkCli.run`/
/// `warmup`: a raw line is skipped if it is empty or starts with `#`; otherwise the WHOLE line,
/// trimmed, is the name. Unlike `parse`'s [`extract_name`] above, this does NOT split on TAB —
/// `BenchmarkCli` never does either, so pointing `--input` at a TSV would time the raw,
/// untrimmed row as a single "name", exactly as Java would.
fn extract_benchmark_name(raw: &str) -> Option<&str> {
    if raw.is_empty() || raw.starts_with('#') {
        return None;
    }
    let name = raw.trim();
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
    fn extract_benchmark_name_trims_but_does_not_split_on_tab() {
        // Unlike `extract_name` (used by `parse`), the benchmark reader never splits on TAB —
        // the whole trimmed line is the "name", matching Java's `BenchmarkCli.run`/`warmup`.
        assert_eq!(
            extract_benchmark_name("  Abies alba\tsome other column  "),
            Some("Abies alba\tsome other column")
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
}
