// SPDX-License-Identifier: Apache-2.0

//! `validate` — LLM-audited correctness sampling for the parser, mirroring the Java CLI's
//! `org.gbif.nameparser.cli.ValidateCli` / `BarcodeOtuFilter`
//! (`/Users/markus/code/gbif/name-parser/name-parser-cli/src/main/java/org/gbif/nameparser/cli/`).
//! See `docs/superpowers/findings/2026-07-11-validate-java-recon.md` for the full verified map
//! of the Java subsystem this ports, and `docs/superpowers/plans/2026-07-11-phase4c-validate.md`
//! for the task breakdown and Global Constraints binding every task in this port.
//!
//! ## Status: scaffold only (Phase 4c Task 1)
//!
//! This module currently provides only the pieces that need no LLM/HTTP/sampling machinery:
//! the [`ValidateArgs`] CLI surface, [`is_barcode_otu`] (`BarcodeOtuFilter`), and
//! [`is_interesting`] (the "suspicious tail" predicate, `ValidateCli.isInteresting`).
//! [`run_validate`] is a stub that always returns `Ok(())` without reading `args.input` or
//! doing anything else. Reservoir sampling, the `JavaRandom` LCG, the corpus scan, the
//! judge/report loop, the LLM clients, and the verdict cache all land in later Phase 4c tasks.
//!
//! `nameparser-cli` is a binary-only crate (no library target), so `pub` here doesn't exempt an
//! item from the `dead_code` lint the way it would in a library — every item below IS part of
//! this module's public building-block API (and is exercised by this module's own tests), it
//! just has no caller from `main` yet, since [`run_validate`] doesn't invoke the corpus scan
//! that will consume [`is_barcode_otu`]/[`is_interesting`]/[`ParseOutcome`] until Task 2.

#![allow(dead_code)]

use std::io;
use std::path::PathBuf;
use std::sync::LazyLock;

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
    /// Clamped to `min(sample_normal, budget)` where it's actually consumed (selection, a later
    /// task) — this scaffold only carries the raw value through.
    #[arg(long, default_value_t = 200)]
    pub sample_normal: usize,

    /// Names per LLM request. Clamped to `max(1, batch)` where it's actually consumed (a later
    /// task) — this scaffold only carries the raw value through.
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

/// Runs the `validate` subcommand. **Stub (Phase 4c Task 1):** always succeeds without reading
/// `args.input` or doing anything else — the select/judge/report pipeline lands in later tasks.
pub fn run_validate(_args: ValidateArgs) -> io::Result<()> {
    Ok(())
}

// ---------------------------------------------------------------------------------------
// BarcodeOtuFilter
// ---------------------------------------------------------------------------------------

/// Java `BarcodeOtuFilter.UNITE_SH`, verbatim: `SH` + ≥5 digits + optional `.`+digits + `FU`,
/// case-insensitive, anchored at the start only (`^`, no `$`) — a `.find()`-style match, so
/// trailing content after the pattern doesn't prevent a match.
static UNITE_SH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^SH\d{5,}(\.\d+)?FU\b").unwrap());

/// Java `BarcodeOtuFilter.BOLD_BIN`, verbatim: `BOLD:` + 2-5 uppercase letters + ≥1 digit,
/// case-insensitive, anchored at the start only.
static BOLD_BIN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^BOLD:[A-Z]{2,5}\d+\b").unwrap());

/// Java `BarcodeOtuFilter.isBarcodeOtu(String)`: `true` if `name`, trimmed, matches either
/// [`UNITE_SH`] or [`BOLD_BIN`] at the start. Applied pre-parse, on the raw input string, so a
/// UNITE/BOLD barcode/OTU code is excluded from the corpus before it ever reaches the parser
/// (recon doc §3: this regex pre-filter is the ONLY OTU exclusion point — a code that slips
/// past it and later parses/fails as `NameType::Other` is intentionally NOT re-excluded
/// downstream; there is no `NameType::Otu` variant to filter on, on either the Java or Rust
/// side).
///
/// Rust's `regex` crate is Unicode-aware by default (`\d`/`\b` match more than plain ASCII),
/// unlike Java's `Pattern` (ASCII-only unless `UNICODE_CHARACTER_CLASS` is set) — but every
/// `BarcodeOtuFilterTest` case is plain ASCII, and the tests below confirm the two engines
/// agree on all of them, so no `(?-u:…)` ASCII-scoping was needed in practice.
pub fn is_barcode_otu(name: &str) -> bool {
    let s = name.trim();
    UNITE_SH.is_match(s) || BOLD_BIN.is_match(s)
}

// ---------------------------------------------------------------------------------------
// is_interesting — the "suspicious tail" predicate
// ---------------------------------------------------------------------------------------

/// The result of parsing one corpus row — an alias for [`nameparser::parse`]'s own return type,
/// named here to match the Java recon's `ParseResult`/`isInteresting` naming without
/// introducing a new struct (there is no additional data to carry yet: `line`/`input` join
/// this in later tasks, once `select` exists).
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
}
