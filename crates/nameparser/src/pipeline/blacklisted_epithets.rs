// SPDX-License-Identifier: Apache-2.0

//! Java `org.gbif.nameparser.pipeline.BlacklistedEpithets` (34 lines) — loads a fixed
//! stop-list once and exposes a case-insensitive membership check. A blacklisted epithet
//! flags the name doubtful with a [`crate::model::warnings::BLACKLISTED_EPITHET`] warning
//! (applied by [`crate::pipeline::assemble`]).
//!
//! The word list (`resources/blacklist-epithets.txt`, copied byte-for-byte from Java's
//! `nameparser/blacklist-epithets.txt` classpath resource — `diff`-verified identical) is
//! embedded at compile time via `include_str!` rather than read from disk at runtime. Java's
//! loader (`LineReader`, default `skipBlank=true, skipComments=true`) trims each line and
//! lower-cases it before inserting into the set; this file has zero blank lines and zero
//! `#`-comment lines, so every line is a real entry — but it has **275**, not 274: `wc -l`
//! reports 274 because the file's own last line (`"zwar"`) has no trailing newline, so `wc -l`
//! (which counts newline BYTES) undercounts the true line count by one (confirmed with
//! `awk 'END{print NR}'` and Python's `str.splitlines()`, both reporting 275, and
//! `sort -u | wc -l` confirming all 275 are case-insensitively distinct). Java's
//! `BufferedReader.readLine()` — underlying `LineReader` — also returns a final line with no
//! terminator (standard documented behaviour: a line is terminated by a line terminator "or by
//! reaching the end of file"), so Java's own loaded `EPITHETS` set is 275 entries too; this
//! is genuine parity, not a Rust-side off-by-one. Every line has no leading/trailing
//! whitespace, is pure ASCII, and is already all lower-case (verified against the source), so
//! `.lines()` + `.trim()` alone reproduces Java's trim step exactly — no separate
//! lower-casing allocation is needed at load time since the content is already lower-case.
//! [`contains`] still lower-cases the QUERY on every call, matching Java's own
//! `EPITHETS.contains(epithet.toLowerCase())` (the stored side is already lower-case; the
//! caller's `epithet` argument generally is not).

// Ported ahead of its caller, `pipeline::assemble` (Task 4 wires `assemble::finish` into
// `Pipeline::run`) — same situation as `pipeline::authorship_parser` (Task 2): a normal
// (non-test) build sees this module as dead code until then. Drop this attribute once Task 4
// lands.
#![allow(dead_code)]

use std::collections::HashSet;
use std::sync::LazyLock;

/// The blacklist file, embedded at compile time (copied verbatim from
/// `name-parser/src/main/resources/nameparser/blacklist-epithets.txt`).
const BLACKLIST_TXT: &str = include_str!("../../resources/blacklist-epithets.txt");

/// Java `BlacklistedEpithets.EPITHETS` (`private static final Set<String> EPITHETS =
/// load();`, `BlacklistedEpithets.java:16`).
static EPITHETS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    BLACKLIST_TXT
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
});

/// Java `BlacklistedEpithets.contains(String)` (`BlacklistedEpithets.java:20-22`). Java's own
/// null-guard (`epithet != null && …`) is the caller's job here — every real call site (in
/// `Assemble.flagBlacklistedEpithets`) already null-checks the epithet before calling this —
/// matching this port's established "non-nullable signature, guard at the call site"
/// convention (see e.g. `ParsedName::set_notho`'s doc comment).
pub(crate) fn contains(epithet: &str) -> bool {
    EPITHETS.contains(epithet.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Resource parity: the copied file has exactly 275 lines (NOT 274 — see the module doc
    /// comment on the `wc -l` off-by-one from the missing trailing newline), all data (no
    /// blanks/comments to skip, verified separately against the source file), and no
    /// case-insensitive duplicates (also verified against the source via `sort -u`), so the
    /// loaded set's size must be exactly 275 — a direct regression guard against a bad copy
    /// or a load-loop bug that silently drops entries.
    #[test]
    fn resource_parity_275_entries() {
        assert_eq!(EPITHETS.len(), 275);
    }

    #[test]
    fn sample_entries_present_case_insensitively() {
        for word in ["aber", "about", "accepted", "auct", "zwar"] {
            assert!(contains(word), "{word:?} should be blacklisted");
            assert!(
                contains(&word.to_uppercase()),
                "lookup must be case-insensitive: {word:?}"
            );
        }
    }

    #[test]
    fn a_real_epithet_is_not_blacklisted() {
        assert!(!contains("vulgaris"));
        assert!(!contains("alba"));
        assert!(!contains("barrelieri"));
    }

    #[test]
    fn empty_string_is_not_blacklisted() {
        assert!(!contains(""));
    }
}
