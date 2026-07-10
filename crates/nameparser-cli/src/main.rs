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
//!   - `benchmark` / `compare` subcommands (Phase 2 Tasks 2/3).

use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};

use clap::{Args, Parser, Subcommand, ValueEnum};
use nameparser::model::{ParseError, ParsedName};

/// Print a progress line to stderr every this-many parsed rows (unless `--quiet`). Matches
/// the Java CLI's `ParseCli.PROGRESS_EVERY`.
const PROGRESS_EVERY: u64 = 100_000;

/// Literal input/output path meaning "use stdin/stdout" — matches the Java CLI's `STDIO`.
const STDIO: &str = "-";

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

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Parse(args) => run_parse(args),
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
}
