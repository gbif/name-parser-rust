// SPDX-License-Identifier: Apache-2.0

//! Java `org.gbif.nameparser.pipeline.AuthorshipParser` (817 lines) — parses the
//! authorship section of a tokenised name into a basionym [`Authorship`] + a combination
//! [`Authorship`], plus a sanctioning author and a couple of parse-quality flags. Entirely
//! regex-free: pure `TokenKind` dispatch, `&str`/`char` tests, and two small fixed-string
//! membership sets ([`AUTHOR_SUFFIXES`], [`GENERATIONAL_SUFFIXES`]).
//!
//! [`parse`] runs in three phases over `tokens[from..]` (`from` is the authorship-section
//! boundary found by `AuthorshipSplit`, Phase 1 Slice 3):
//!   - **Phase A** — a leading `(...)` is a basionym candidate. [`find_close`] finds the
//!     matching close paren (a plain depth counter). Inside the parens, a top-level colon
//!     ([`find_last_colon`], paren/bracket-depth-aware) separates a basionym sanctioning
//!     author, which is dropped (canonical names attribute sanctioning to the species
//!     level, never the basionym). [`has_upper_word`] then gates whether this is really a
//!     basionym at all: a `(...)` containing no upper-case WORD (e.g. a malformed
//!     `"(ilic)"`) is NOT a basionym — it is parked as `unparsed_from`/`unparsed_text` and
//!     skipped over.
//!   - **Phase B** — the remaining span is the combination. A top-level trailing colon
//!     splits off a sanctioning author (`"Boletus versicolor L. : Fr."` →
//!     `sanctioning_author = Some("Fr.")`, combination span truncated before the colon).
//!     Whatever remains (with or without a sanctioning author extracted) is parsed as the
//!     combination authors; the whole trailing span is consumed (`i` set to `tokens.len()`
//!     unconditionally) — nothing after the combination span is ever unparsed.
//!   - **Phase C** — a final `if i < n` catch-all that would record any remainder as
//!     unparsed. **This is unreachable given Phase B's unconditional `i = n`** (confirmed
//!     by exhaustive case analysis of every path through `parse`, and the Java source's own
//!     comment at that point already says as much: "nothing is unparsed afterwards").
//!     Ported verbatim anyway (branch-for-branch fidelity; it costs nothing and Rust proves
//!     it dead via a coverage-style regression test below rather than by deleting it).
//!
//! [`parse_authors`] is the 14-case token walk that actually builds author strings within
//! one span (basionym or combination), handling years (incl. bracketed/second-plain
//! imprint years and year ranges), `ex`-author splitting, separators, particles, the
//! surname/initials inversion (with its generational-suffix and "forename M.Surname"
//! carve-outs), suffix gluing, apostrophes, mid-initial hyphens and slashes. Its trailing
//! post-processing ([`invert_all`]/[`invert_author`]/[`format_initials`]) re-orders
//! "Surname, Initials"-shaped strings into the canonical "Initials.Surname" form.
//!
//! ## Quirks preserved verbatim (do not "fix" — see the plan's Global Constraints 1–6)
//! 1. `COMMA`'s three lookahead branches (comma+year / comma+capitalised-or-particle-word /
//!    anything else) all currently execute the identical `flush(); i++; continue;` — the
//!    classification is vestigial. Preserved as three branches, not collapsed.
//! 2. [`AUTHOR_SUFFIXES`] membership is checked **case-sensitively** in the post-surname
//!    chaining sub-loop (an uppercase "F" there is an initial, not the filius suffix) but
//!    **case-insensitively** in the standalone-after-separator branch.
//! 3. [`GENERATIONAL_SUFFIXES`] excludes bare `I`/`V`/`X` — those are genuine author
//!    initials, not generation-suffix Roman numerals.
//! 4. A bracketed `[YYYY]` is always the *imprint* year (first-wins), never the main year,
//!    even when it is the only year given. A second *plain* year is the imprint year. A
//!    year range (`NUMBER "-"|"/" NUMBER`) keeps only the first year and sets
//!    [`AuthState::year_range`].
//! 5. The surname-first "**M** Balsamo"-style inversion only fires when a real surname
//!    (not just particles) is already in the buffer — [`ends_with_particle_only`] guards
//!    this, so `"H.da C." + "Monteiro"` stays a single chained author, not an inversion.
//!    **The Java inline comment on this branch is stale relative to the actual code**: it
//!    claims an isolated undotted trailing initial ("Zhang F" at end-of-segment or before a
//!    separator) keeps the CJK surname-first form, but `middle_initial_surname_follows`
//!    requires having consumed at least one `DOT` after the initials token (`j > i + 1`) —
//!    an undotted token never satisfies that, so the flip fires unconditionally whenever a
//!    real surname precedes it, comment notwithstanding. Verified against
//!    `NameParserImplTest` (`"... Zhang F & Pan Z-X in Zhang, F, ..., 2016"` →
//!    `combAuthors(..., "F.Zhang", "Z-X.Pan")` — "F" is immediately followed by "&", a
//!    separator, and still flips) and against the Java repo's `CLAUDE.md` authorship-
//!    conventions note, which documents the flipping behaviour directly. Ported to match
//!    the verified *behaviour*.
//! 6. `format_initials` keeps a dot-per-letter and the input case of a hyphenated
//!    lower-case follow-up for dotted input (`"Y.-j."` stays `"Y.-j."`), but upper-cases a
//!    dot-less CJK-style hyphenated pair and adds a single trailing dot (`"Z-X"` →
//!    `"Z-X."`).
//!
//! `invert_author` and `normalise_author_case` are package-private (widened visibility) in
//! Java; neither has any caller outside `AuthorshipParser` in this port, so both stay
//! private here.

// This entire module is ported ahead of its sole consumer: `Pipeline::run` doesn't call
// `authorship_parser::parse` yet — that wiring is Phase 1 Slice 4 Task 4. Until then,
// everything here is only exercised by this module's own `#[cfg(test)]` tests, so a normal
// (non-test) build sees the whole module as dead code (same situation `ParseContext` hit in
// `pipeline::context`, handled there the same way). Allowed at the module level rather than
// per-item since the reason is uniform; drop this attribute once Task 4 wires `parse` into
// `Pipeline::run`.
#![allow(dead_code)]

use crate::model::Authorship;
use crate::token::{is_particle, Token, TokenKind};

/// Java `AuthorshipParser.AUTHOR_SUFFIXES` — tokens that are filius/junior/etc.
/// abbreviations gluing onto the previous author. Checked case-sensitively in the
/// post-surname chaining sub-loop of [`parse_authors`] and case-insensitively in the
/// standalone-after-separator branch (Quirk 2 above) — the same 14-entry set both times.
const AUTHOR_SUFFIXES: &[&str] = &[
    "f", "fil", "filius", "j", "jr", "junior", "jun", "sr", "senior", "sen", "ms", "ined", "Bis",
    "bis",
];

/// Java `AuthorshipParser.GENERATIONAL_SUFFIXES` — Roman-numeral generational suffixes on a
/// surname ("Loeblich III" = Loeblich the third). Kept verbatim (upper-case) behind the
/// surname, never read as the initials "I.I.I.". Single letters (I/V/X) are deliberately
/// excluded — those are author initials, not suffixes.
const GENERATIONAL_SUFFIXES: &[&str] = &["II", "III", "IV", "VI", "VII", "VIII", "IX"];

/// Java `AuthorshipParser.AuthState` (package-private nested class). All fields were
/// package-private in Java; kept `pub` here (the enclosing struct is already capped at
/// `pub(crate)`, so this changes nothing about actual visibility, matching the interface
/// contract this type was specified against).
#[derive(Debug)]
pub(crate) struct AuthState {
    pub combination: Authorship,
    pub basionym: Authorship,
    pub basionym_present: bool,
    pub year_range: bool,
    /// True when an "f."/"fil."/"filius" suffix appeared on any author — botanical signal.
    pub has_filius: bool,
    pub sanctioning_author: Option<String>,
    /// Java `int unparsedFrom = -1;` — kept as a sentinel `i32`, matching the established
    /// convention for this exact shape of field elsewhere in the port (see
    /// `ParseContext::mid_author_from`/`mid_author_to`).
    pub unparsed_from: i32,
    pub unparsed_text: Option<String>,
}

impl Default for AuthState {
    fn default() -> Self {
        AuthState {
            combination: Authorship::default(),
            basionym: Authorship::default(),
            basionym_present: false,
            year_range: false,
            has_filius: false,
            sanctioning_author: None,
            unparsed_from: -1,
            unparsed_text: None,
        }
    }
}

/// Java `AuthorshipParser.parse(List<Token> tokens, int from)` (`AuthorshipParser.java:48-109`).
/// See the module doc comment for the phase A/B/C walkthrough.
pub(crate) fn parse(tokens: &[Token], from: usize) -> AuthState {
    let mut s = AuthState::default();
    let mut i = from;
    let n = tokens.len();

    // Phase A: leading "(...)" basionym candidate.
    if i < n && tokens[i].kind == TokenKind::OpenParen {
        if let Some(close) = find_close(tokens, i) {
            // Inside the basionym brackets, a colon also separates the original author
            // from the sanctioning author ("(Fr. : Fr.)"). Drop the sanctioning span;
            // canonical names attribute the sanctioning to the species level.
            let bas_from = i + 1;
            let mut bas_end = close;
            if let Some(bas_colon) = find_last_colon(tokens, bas_from, bas_end) {
                if bas_colon > bas_from {
                    bas_end = bas_colon;
                }
            }
            // Reject a basionym made up of only lowercase tokens (no real surname).
            // "(ilic)" is malformed — capture it as unparsed instead.
            if has_upper_word(tokens, bas_from, bas_end) {
                let yr = parse_authors(tokens, bas_from, bas_end, &mut s.basionym);
                s.year_range |= yr;
                s.has_filius |= contains_filius_suffix(tokens, bas_from, bas_end);
                s.basionym_present = true;
            } else {
                // Park the whole "(...)" span as unparsed and skip past it.
                let open_tok = &tokens[i];
                let close_tok = &tokens[close];
                s.unparsed_from = i as i32;
                s.unparsed_text = Some(slice_text(tokens, open_tok.start, close_tok.end));
            }
            i = close + 1;
        }
    }

    // Phase B: whatever remains is the combination (+ optional trailing sanctioning author).
    if i < n {
        let comb_from = i;
        let mut comb_end = n;
        // pull out a colon + sanctioning author at the end of the combination span
        if let Some(colon) = find_last_colon(tokens, comb_from, comb_end) {
            if colon > comb_from {
                let mut sb = String::new();
                append_author_words(tokens, colon + 1, comb_end, &mut sb);
                if !sb.is_empty() {
                    s.sanctioning_author = Some(sb);
                    comb_end = colon;
                }
            }
        }
        let yr = parse_authors(tokens, comb_from, comb_end, &mut s.combination);
        s.year_range |= yr;
        s.has_filius |= contains_filius_suffix(tokens, comb_from, comb_end);
        // Whether or not a sanctioning author was extracted, the entire trailing span
        // belongs to combination + sanctioning; nothing is unparsed afterwards.
        i = n;
    }

    // Phase C: catch-all for any remainder. Unreachable in practice — Phase B above
    // unconditionally sets `i = n` whenever it runs at all, and it runs whenever `i < n`
    // going in; so by this point `i == n` on every path. Ported verbatim regardless (see
    // the module doc comment) rather than dropped as "dead code".
    if i < n {
        let first = &tokens[i];
        let last = &tokens[n - 1];
        s.unparsed_from = i as i32;
        s.unparsed_text = Some(slice_text(tokens, first.start, last.end));
    }
    s
}

/// Java `AuthorshipParser.findLastColon(List<Token>, int, int)`. Depth-aware over both
/// paren and bracket nesting; `None` where Java returns `-1`.
fn find_last_colon(tokens: &[Token], from: usize, to: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut last = None;
    for (j, tok) in tokens.iter().enumerate().take(to).skip(from) {
        match tok.kind {
            TokenKind::OpenParen | TokenKind::OpenBracket => depth += 1,
            TokenKind::CloseParen | TokenKind::CloseBracket => depth -= 1,
            TokenKind::Colon if depth == 0 => last = Some(j),
            _ => {}
        }
    }
    last
}

/// Java `AuthorshipParser.findClose(List<Token>, int)`. Plain paren-depth counter (brackets
/// are not tracked here, unlike [`find_last_colon`]); `None` where Java returns `-1`.
fn find_close(tokens: &[Token], open_idx: usize) -> Option<usize> {
    let mut depth = 1i32;
    for (j, tok) in tokens.iter().enumerate().skip(open_idx + 1) {
        match tok.kind {
            TokenKind::OpenParen => depth += 1,
            TokenKind::CloseParen => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
            _ => {}
        }
    }
    None
}

/// Java `AuthorshipParser.parseAuthors(List<Token>, int, int, Authorship, AuthState)`.
/// The `AuthState state` parameter is accepted but never read anywhere in the Java method
/// body (verified against the source) — dropped here rather than carried as an unused Rust
/// parameter; this is a zero-behaviour-change omission, not a port gap.
///
/// Parses the author list within `tokens[from..to)`, populating `into`. Handles `ex`
/// splitting (ex authors come before main authors). A second 4-digit year encountered after
/// the first (or a bracketed year) becomes `into`'s imprint year, sitting next to its
/// publication year on the [`Authorship`]. Returns `true` if a year range was detected
/// (e.g. "1845-1847", "1987-92").
fn parse_authors(tokens: &[Token], from: usize, to: usize, into: &mut Authorship) -> bool {
    let mut authors: Vec<String> = Vec::new();
    let mut ex_authors: Option<Vec<String>> = None;
    let mut cur = String::new();
    let mut year_range = false;
    let mut i = from;

    while i < to {
        let t = &tokens[i];

        // year
        if t.kind == TokenKind::Number && (3..=4).contains(&t.text.chars().count()) {
            flush(&mut cur, &mut authors);
            let mut year = t.text.clone();
            // Detect a bracketed year: "[YYYY]" / "[YYYY?]". A year inside square brackets
            // is by convention the imprint year (the year actually printed on the work),
            // not the nominal publication year — even when it is the only year given.
            let in_brackets_initial = i > from && tokens[i - 1].kind == TokenKind::OpenBracket;
            i += 1;
            // Uncertain year: a trailing "?" is part of the year ("198?" → year="198?").
            if i < to && tokens[i].kind == TokenKind::Other && tokens[i].text == "?" {
                year.push('?');
                i += 1;
            }
            // Confirm we're still inside the brackets (close-bracket follows the year).
            let in_brackets =
                in_brackets_initial && i < to && tokens[i].kind == TokenKind::CloseBracket;
            if in_brackets {
                // A bracketed year is the imprint year of THIS authorship (basionym or
                // combination), sitting next to its publication year; never its main year.
                if into.imprint_year.is_none() {
                    into.imprint_year = Some(year);
                }
                i += 1; // skip CLOSE_BRACKET
                continue;
            }
            // First year wins — imprint dates ("Linnaeus, 1898, 1897") keep the first
            // (the actual publication year); the second becomes this authorship's imprint year.
            if into.year.is_none() {
                into.year = Some(year);
            } else if into.imprint_year.is_none() {
                into.imprint_year = Some(year);
            }
            // Detect year range: NUMBER + OTHER("-" or "/") + NUMBER → keep first year only
            if i + 1 < to {
                let sep = &tokens[i];
                if sep.kind == TokenKind::Other && (sep.text == "-" || sep.text == "/") {
                    let nx = &tokens[i + 1];
                    if nx.kind == TokenKind::Number && (1..=4).contains(&nx.text.chars().count()) {
                        year_range = true;
                        i += 2;
                    }
                }
            }
            // Drop a single trailing lowercase-letter year disambiguator ("1935h" / "1935 h",
            // or "193k7" where the k is an OCR/typo artifact followed by digits).
            if i < to {
                let nx = &tokens[i];
                if nx.kind == TokenKind::Word
                    && starts_lower(&nx.text)
                    && is_year_disambiguator(&nx.text)
                {
                    i += 1;
                }
            }
            continue;
        }

        // ex separator
        if t.kind == TokenKind::Word && t.text == "ex" {
            flush(&mut cur, &mut authors);
            // everything collected so far becomes ex authors
            ex_authors = Some(authors.clone());
            authors.clear();
            i += 1;
            continue;
        }

        // separators
        if t.kind == TokenKind::Ampersand
            || (t.kind == TokenKind::Word
                && (t.text.eq_ignore_ascii_case("and") || t.text.eq_ignore_ascii_case("et")))
            || (t.kind == TokenKind::Word && t.text == "y")
        {
            flush(&mut cur, &mut authors);
            i += 1;
            continue;
        }

        if t.kind == TokenKind::Semicolon {
            // Semicolons separate authors in citation lists ("Choi,J.H.; Im,W.T.; …")
            flush(&mut cur, &mut authors);
            i += 1;
            continue;
        }

        if t.kind == TokenKind::Comma {
            // peek next token to decide: comma+year, comma+author, comma+& etc.
            //
            // Quirk 1 (preserved verbatim, do not "fix"): all three outcomes below run the
            // exact same `flush(); i += 1; continue;` regardless of which lookahead branch
            // matched — the classification is vestigial in the current Java source.
            let mut j = i + 1;
            while j < to
                && (tokens[j].kind == TokenKind::Ampersand
                    || (tokens[j].kind == TokenKind::Word
                        && (tokens[j].text.eq_ignore_ascii_case("and")
                            || tokens[j].text.eq_ignore_ascii_case("et"))))
            {
                j += 1;
            }
            if j < to {
                let next = &tokens[j];
                if next.kind == TokenKind::Number {
                    flush(&mut cur, &mut authors);
                    i += 1;
                    continue;
                }
                if next.kind == TokenKind::Word
                    && (starts_upper(&next.text) || is_particle(&next.text))
                {
                    flush(&mut cur, &mut authors);
                    i += 1;
                    continue;
                }
            }
            flush(&mut cur, &mut authors);
            i += 1;
            continue;
        }

        // particle
        if t.kind == TokenKind::Word && starts_lower(&t.text) && is_particle(&t.text) {
            append_space(&mut cur);
            cur.push_str(&t.text);
            i += 1;
            // Pull through abbreviation dots that follow ("v." / "v.d." style particles).
            while i < to && tokens[i].kind == TokenKind::Dot {
                cur.push('.');
                i += 1;
            }
            continue;
        }

        // surname token
        if t.kind == TokenKind::Word && starts_upper(&t.text) {
            let text = t.text.clone();
            // A roman-numeral generational suffix directly after a complete surname
            // ("Loeblich III" = Loeblich the third) stays behind the surname as a suffix,
            // rendered upper-case ("Iii" → "III") — it is NOT the initials "I.I.I." that
            // the all-caps-trailing-initials inversion below would otherwise produce.
            if !cur.is_empty()
                && contains_lower(&cur)
                && !cur.ends_with('.')
                && !ends_with_particle_only(&cur)
                && GENERATIONAL_SUFFIXES.contains(&text.to_uppercase().as_str())
            {
                append_space(&mut cur);
                cur.push_str(&text.to_uppercase());
                i += 1;
                continue;
            }
            // No-comma "<Surname> <Initials>" inversion pattern: if cur already holds a
            // Latin surname AND the incoming token is a short all-caps word, treat it as
            // the initials trailing the surname and flush as a single inverted author. A
            // trailing dot in cur means we are mid-abbreviation (e.g. "v.d." awaiting
            // "L"), not after a complete surname — skip the inversion in that case.
            if !cur.is_empty()
                && text.chars().count() <= 3
                && is_all_upper(&text)
                && contains_lower(&cur)
                && !cur.ends_with('.')
                && !ends_with_particle_only(&cur)
            {
                let mut initials = text.clone();
                let mut j = i + 1;
                while j < to && tokens[j].kind == TokenKind::Dot {
                    initials.push('.');
                    j += 1;
                }
                // Pick up an optional "-X" continuation so "Pan Z-X" is treated as one
                // author pair (initials "Z-X") rather than two. (In practice the
                // tokenizer already glues an undotted "Z-X" into one WORD token, so this
                // peek only ever fires when a DOT separates the hyphen from what came
                // before, e.g. "Pan Z.-X" / "Pan Z .-X".)
                let mut k = j;
                if k < to {
                    let peek = &tokens[k];
                    if peek.kind == TokenKind::Other
                        && peek.text == "-"
                        && k + 1 < to
                        && tokens[k + 1].kind == TokenKind::Word
                        && tokens[k + 1].text.chars().count() == 1
                    {
                        initials.push('-');
                        initials.push_str(&tokens[k + 1].text);
                        k += 2;
                        while k < to && tokens[k].kind == TokenKind::Dot {
                            initials.push('.');
                            k += 1;
                        }
                    }
                }
                // "Forename M. Surname": a DOTTED middle initial followed by a
                // capitalised surname-shaped word means the all-caps token is a middle
                // initial of a single spelled-out-forename author ("Roy L.Taylor"), not
                // the inversion "L.Roy" + "Taylor". Skip the flip in that case and let
                // the normal surname handling below build the full author. Only dotted
                // initials qualify (`j > i + 1` requires at least one DOT consumed) — an
                // undotted run-on ("Balsamo M Fregni") still flips to end the current
                // author, and always does so regardless of what follows (see the module
                // doc comment's Quirk 5 note on the stale Java inline comment here).
                let middle_initial_surname_follows = j > i + 1
                    && k < to
                    && tokens[k].kind == TokenKind::Word
                    && starts_upper(&tokens[k].text)
                    && contains_lower(&tokens[k].text);
                if !middle_initial_surname_follows {
                    let surname = cur.trim().to_string();
                    cur.clear();
                    authors.push(format!("{}{}", format_initials(&initials), surname));
                    i = k;
                    continue;
                }
            }
            // If full ALL-CAPS author name (e.g. FISCHER) length > 1, normalise to title case
            let text = normalise_author_case(&t.text);
            append_space(&mut cur);
            cur.push_str(&text);
            i += 1;
            // A single capital letter without a trailing dot is an abbreviated initial —
            // supply the dot so subsequent tokens chain ("A S. Xu" → "A.S.Xu").
            if text.chars().count() == 1
                && text.chars().next().is_some_and(|c| c.is_uppercase())
                && (i >= to || tokens[i].kind != TokenKind::Dot)
            {
                cur.push('.');
            }
            // chain "<DOT> [WORD]" sequences for "Müll.Arg." style and "L. f" style.
            while i < to {
                let nx = &tokens[i];
                if nx.kind == TokenKind::Dot {
                    cur.push('.');
                    i += 1;
                    continue;
                }
                if nx.kind == TokenKind::Word {
                    let nxt = nx.text.clone();
                    // Filius / junior / etc. — case-sensitive: lowercase only. An
                    // uppercase "F" following a surname is an initial, not the filius
                    // suffix, so we don't collapse it here.
                    if AUTHOR_SUFFIXES.contains(&nxt.as_str()) {
                        // abbreviated surname ends with '.': "Burm.f." — no separator needed
                        // full surname ends with a letter: "Hooker f." — use a space
                        if !cur.is_empty() && !cur.ends_with('.') {
                            cur.push(' ');
                        }
                        cur.push_str(&nxt);
                        i += 1;
                        // optional dot after suffix
                        if i < to && tokens[i].kind == TokenKind::Dot {
                            cur.push('.');
                            i += 1;
                        }
                        continue;
                    }
                    // continued upper-case piece (common in compound surnames
                    // "Saint-Lager", "Müll.Arg.")
                    if starts_upper(&nx.text) && !cur.is_empty() && cur.ends_with('.') {
                        cur.push_str(&normalise_author_case(&nx.text));
                        i += 1;
                        continue;
                    }
                    break;
                }
                break;
            }
            continue;
        }

        // lone "f." or "f" sitting after a separator: treat as filius glued to last author
        if t.kind == TokenKind::Word
            && t.text.chars().count() <= 3
            && AUTHOR_SUFFIXES.contains(&t.text.to_lowercase().as_str())
        {
            // attach to the last completed author if any, else to current buffer
            if cur.is_empty() && !authors.is_empty() {
                let mut last = authors.pop().expect("just checked non-empty");
                if !last.ends_with('.') {
                    last.push('.');
                }
                last.push_str(&t.text);
                authors.push(last);
            } else {
                if !cur.is_empty() && !cur.ends_with('.') {
                    cur.push('.');
                }
                cur.push_str(&t.text);
            }
            i += 1;
            if i < to && tokens[i].kind == TokenKind::Dot {
                if !cur.is_empty() && !cur.ends_with('.') {
                    cur.push('.');
                } else if !authors.is_empty() {
                    let last_idx = authors.len() - 1;
                    if !authors[last_idx].ends_with('.') {
                        authors[last_idx].push('.');
                    }
                }
                i += 1;
            }
            continue;
        }

        if t.kind == TokenKind::Word {
            append_space(&mut cur);
            cur.push_str(&t.text);
            i += 1;
            // chain trailing dot for abbreviations like "al."
            if i < to && tokens[i].kind == TokenKind::Dot {
                cur.push('.');
                i += 1;
            }
            continue;
        }

        // Apostrophe between authors / inside an author span — preserve it so that
        // names with internal apostrophes ("L.'t Mannetje", "M'Coy", "d'Urv.", "'t Hart")
        // render verbatim. Glue to the preceding character when there's no whitespace
        // gap in the input ("d'Urv"); otherwise insert a space ("Henk 't").
        if t.kind == TokenKind::Other && t.text == "'" {
            let has_gap = !cur.is_empty() && i > 0 && tokens[i - 1].end < t.start;
            if has_gap {
                append_space(&mut cur);
            }
            cur.push('\'');
            i += 1;
            if i < to && tokens[i].kind == TokenKind::Word {
                cur.push_str(&tokens[i].text);
                i += 1;
            }
            continue;
        }
        // "?" inside an author span (typically a transcription artefact for a missing
        // letter, "Istv?nffi") — silently glue the next word onto cur without a space.
        if t.kind == TokenKind::Other && t.text == "?" && !cur.is_empty() {
            i += 1;
            if i < to && tokens[i].kind == TokenKind::Word {
                cur.push_str(&tokens[i].text);
                i += 1;
            }
            continue;
        }
        // Hyphen between abbreviated initial parts ("C.-K.", "J.-j.") — keep the hyphen
        // and preserve the input case of the single-letter follow-up so compound initials
        // round-trip cleanly ("Y.-j." stays "Y.-j.", "C.-K." stays "C.-K.").
        if t.kind == TokenKind::Other
            && t.text == "-"
            && !cur.is_empty()
            && cur.ends_with('.')
            && i + 1 < to
            && tokens[i + 1].kind == TokenKind::Word
        {
            let nx_text = tokens[i + 1].text.clone();
            cur.push('-');
            if nx_text.chars().count() == 1 {
                cur.push_str(&nx_text);
                i += 2;
                if i < to && tokens[i].kind == TokenKind::Dot {
                    cur.push('.');
                    i += 1;
                }
                continue;
            }
            i += 1;
            continue;
        }
        // "/" between two author words ("Smith/Jones") marks alternative authorship. Keep
        // the slash glued (no surrounding spaces) so the ambiguity stays visible in the
        // output; the name is already flagged UNCERTAIN_AUTHORSHIP upstream.
        if t.kind == TokenKind::Other
            && t.text == "/"
            && !cur.is_empty()
            && i + 1 < to
            && tokens[i + 1].kind == TokenKind::Word
        {
            cur.push('/');
            cur.push_str(&tokens[i + 1].text);
            i += 2;
            continue;
        }
        // Unknown punctuation in an author run — skip silently
        i += 1;
    }
    flush(&mut cur, &mut authors);

    if !authors.is_empty() {
        into.authors = invert_all(&authors);
    }
    if let Some(ex) = ex_authors {
        if !ex.is_empty() {
            into.ex_authors = invert_all(&ex);
        }
    }
    year_range
}

/// Java `AuthorshipParser.invertAll(List<String>)`. Walks the author list applying two
/// transforms: (1) merge pairs where author N is a surname and author N+1 is initials
/// only, producing the combined "Initials.Surname" form ("LeConte" + "J.L." →
/// "J.L.LeConte"); (2) invert single authors of the form "Surname X.Y." or "Surname,
/// X.Y." into the same canonical form.
fn invert_all(authors: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(authors.len());
    let mut i = 0;
    while i < authors.len() {
        let cur = &authors[i];
        if i + 1 < authors.len() {
            let next = &authors[i + 1];
            if looks_like_surname(cur) && looks_like_initials(next) {
                out.push(format!("{}{}", format_initials(next), cur));
                i += 2;
                continue;
            }
        }
        out.push(invert_author(cur));
        i += 1;
    }
    out
}

/// Java `AuthorshipParser.invertAuthor(String)`. Re-orders names like "Surname, J.L." or
/// "Surname Initials" into the canonical "J.L.Surname" form (no space between dotted
/// initials and the surname). Package-private in Java (widened visibility); no caller
/// outside this module in this port, so kept private here (see the module doc comment).
fn invert_author(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return String::new();
    }

    // Pattern A: "Surname, X.Y." or "Surname, XY" — exactly one comma, not at position 0.
    if let Some(comma) = s.find(',') {
        if comma > 0 && !s[comma + 1..].contains(',') {
            let surname = s[..comma].trim();
            let initials = s[comma + 1..].trim();
            if looks_like_surname(surname) && looks_like_initials(initials) {
                return format!("{}{}", format_initials(initials), surname);
            }
        }
    }
    // Pattern B: "Surname X.Y." (trailing dotted initials). Without a dot the trailing
    // all-caps part is a CJK-style surname-first author ("Zhang F", "Pan Z-X") and must
    // be preserved verbatim. Particles like "Van", "de" must be kept on the surname side.
    if let Some(last_space) = s.rfind(' ') {
        if last_space > 0 {
            let first = s[..last_space].trim();
            let last = s[last_space + 1..].trim();
            if last.contains('.')
                && looks_like_surname(first)
                && looks_like_initials(last)
                && !contains_particle(first)
            {
                return format!("{}{}", format_initials(last), first);
            }
        }
    }
    s.to_string()
}

/// Java `AuthorshipParser.looksLikeSurname(String)`.
fn looks_like_surname(s: &str) -> bool {
    if s.chars().count() < 2 {
        return false;
    }
    match s.chars().next() {
        Some(c) if c.is_uppercase() => {}
        _ => return false,
    }
    s.chars().any(|c| c.is_lowercase())
}

/// Java `AuthorshipParser.looksLikeInitials(String)`. Initials are ≤4 single letters, the
/// first upper-case. A lower-case letter is only allowed immediately after a hyphen —
/// that is a hyphenated given-name initial ("Y.-j.", "C.-K."). This still rejects
/// abbreviated surnames ("Fr.", "Liu", "Sacc.") whose lower-case follow-up letters are NOT
/// preceded by a hyphen.
fn looks_like_initials(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut letters = 0u32;
    let mut first_letter_seen = false;
    let mut prev_hyphen = false;
    for c in s.chars() {
        if c == '.' || c == ' ' {
            continue;
        }
        if c == '-' {
            prev_hyphen = true;
            continue;
        }
        if !c.is_alphabetic() {
            return false;
        }
        letters += 1;
        if letters > 4 {
            return false;
        }
        let upper = c.is_uppercase();
        if !first_letter_seen {
            if !upper {
                return false;
            }
            first_letter_seen = true;
        } else if !upper && !prev_hyphen {
            return false;
        }
        prev_hyphen = false;
    }
    letters > 0
}

/// Java `AuthorshipParser.formatInitials(String)`. Dotted hyphenated initials ("C.-K.",
/// "Y.-j.") get a dot after each letter and keep the input case of a single-letter
/// follow-up so compound initials round-trip cleanly. Dot-less CJK-style hyphenated
/// initials ("Z-X") get a single trailing dot, upper-cased. Plain (non-hyphenated) input
/// gets one dot per letter, upper-cased.
fn format_initials(s: &str) -> String {
    let had_dots = s.contains('.');
    let cleaned: String = s.chars().filter(|&c| c != '.' && c != ' ').collect();
    if cleaned.contains('-') {
        let mut b = String::new();
        for c in cleaned.chars() {
            if c == '-' {
                b.push('-');
            } else if had_dots {
                b.push(c);
                b.push('.');
            } else {
                b.extend(c.to_uppercase());
            }
        }
        if !had_dots {
            b.push('.');
        }
        return b;
    }
    let mut b = String::new();
    for c in cleaned.chars() {
        b.extend(c.to_uppercase());
        b.push('.');
    }
    b
}

/// Java `AuthorshipParser.isYearDisambiguator(String)`. True when the token text looks
/// like a year-disambiguator suffix that should be dropped after a year token: a single
/// lowercase letter optionally followed by all-digit characters — e.g. "h" (from
/// "1935h"), "k7" (OCR-garbled year-suffix artifact in "193k7").
fn is_year_disambiguator(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_digit())
}

/// Java `AuthorshipParser.isAllUpper(String)` — this class's own copy (distinct from the
/// tokenizer's and from `AuthorshipSplit`'s own same-named private helpers).
fn is_all_upper(s: &str) -> bool {
    let mut any = false;
    for c in s.chars() {
        if c.is_alphabetic() {
            any = true;
            if !c.is_uppercase() {
                return false;
            }
        }
    }
    any
}

/// True when the token text starts with an upper-case character — Java `Token.startsUpper()`
/// applied here to a plain `&str` (this file has no `Token` in scope at every call site,
/// e.g. inside `invert_author`/`looks_like_surname`, so it's a free `&str` function rather
/// than a `Token` method, matching `authorship_split.rs`'s own established pattern of
/// small per-file free-function bridges).
fn starts_upper(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_uppercase())
}

/// Java `Token.startsLower()` — see [`starts_upper`]'s doc comment for why this is a free
/// `&str` function here.
fn starts_lower(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_lowercase())
}

/// Java `AuthorshipParser.endsWithParticleOnly(StringBuilder)`. True when the accumulated
/// buffer ends with a particle word (or is just particles). In that case the next
/// all-caps token is the next author's initial, not a trailing surname-inversion signal —
/// there's no real surname accumulated yet. Particle attached to a preceding dot
/// ("H.da") is still considered a particle tail — split on whitespace and dots to find
/// the last word-ish token.
fn ends_with_particle_only(cur: &str) -> bool {
    let s = cur.trim();
    if s.is_empty() {
        return true;
    }
    let last = s
        .split(|c: char| c.is_whitespace() || c == '.')
        .rfind(|p| !p.is_empty())
        .unwrap_or("");
    is_particle(last)
}

/// Java `AuthorshipParser.containsLower(CharSequence)`.
fn contains_lower(s: &str) -> bool {
    s.chars().any(|c| c.is_lowercase())
}

/// Java `AuthorshipParser.containsParticle(String)`.
fn contains_particle(s: &str) -> bool {
    s.split_whitespace().any(is_particle)
}

/// Java `AuthorshipParser.flush(StringBuilder, List<String>)`.
fn flush(cur: &mut String, authors: &mut Vec<String>) {
    if !cur.is_empty() {
        authors.push(cur.trim().to_string());
        cur.clear();
    }
}

/// Java `AuthorshipParser.appendSpace(StringBuilder)`.
fn append_space(cur: &mut String) {
    match cur.chars().last() {
        Some(' ') | Some('.') | Some('-') | None => {}
        Some(_) => cur.push(' '),
    }
}

/// Java `AuthorshipParser.appendAuthorWords(List<Token>, int, int, StringBuilder)`. Used
/// only to render the sanctioning-author span (a plain word+dot run, no separators/years).
fn append_author_words(tokens: &[Token], from: usize, to: usize, sb: &mut String) {
    let mut i = from;
    while i < to {
        if tokens[i].kind == TokenKind::Word {
            if !sb.is_empty() && !sb.ends_with('.') {
                sb.push(' ');
            }
            sb.push_str(&normalise_author_case(&tokens[i].text));
            i += 1;
            while i < to && tokens[i].kind == TokenKind::Dot {
                sb.push('.');
                i += 1;
            }
        } else {
            i += 1;
        }
    }
}

/// Java `AuthorshipParser.normaliseAuthorCase(String)`. Normalises an ALL-CAPS author word
/// to title case ("FISCHER" → "Fischer"). Short all-caps tokens (< 4 chars) are kept
/// as-is — they are likely initials ("MA", "DC"). Package-private in Java (widened
/// visibility); no caller outside this module in this port, so kept private here.
fn normalise_author_case(s: &str) -> String {
    if s.chars().count() < 4 {
        return s.to_string();
    }
    let all_upper = !s.chars().any(|c| c.is_alphabetic() && !c.is_uppercase());
    if !all_upper {
        return s.to_string();
    }
    let mut b = String::with_capacity(s.len());
    let mut first = true;
    for c in s.chars() {
        if c.is_alphabetic() {
            if first {
                b.push(c);
                first = false;
            } else {
                b.extend(c.to_lowercase());
            }
        } else {
            b.push(c);
            first = true;
        }
    }
    b
}

/// Java `AuthorshipParser.sliceText(List<Token>, int, int)`. Iterates the FULL token list
/// (not just a sub-range by index) filtering by absolute character-offset containment —
/// matches the Java source's own (slightly unusual but harmless) structure.
fn slice_text(tokens: &[Token], start: usize, end: usize) -> String {
    let mut sb = String::new();
    for t in tokens {
        if t.start >= start && t.end <= end {
            sb.push_str(&t.text);
        }
    }
    sb
}

/// Java `AuthorshipParser.hasUpperWord(List<Token>, int, int)`. True if any token in
/// `[from, to)` is a WORD that starts with an upper-case letter.
fn has_upper_word(tokens: &[Token], from: usize, to: usize) -> bool {
    tokens[from..to]
        .iter()
        .any(|t| t.kind == TokenKind::Word && starts_upper(&t.text))
}

/// Java `AuthorshipParser.containsFiliusSuffix(List<Token>, int, int)`. Scans the token
/// range for an "f"/"fil"/"filius" word — botanical filius marker. Pre-"ex" tokens are
/// skipped: a filius on an ex-author isn't a code signal because the author itself was
/// never the validating author. The "f"/"fil" token must be preceded by whitespace in the
/// input to count as a filius marker; an adjacent "L.f" (no space) is just another
/// initial.
fn contains_filius_suffix(tokens: &[Token], from: usize, to: usize) -> bool {
    let mut start = from;
    for (j, tok) in tokens.iter().enumerate().take(to).skip(from) {
        if tok.kind == TokenKind::Word && tok.text == "ex" {
            start = j + 1;
        }
    }
    for j in start..to {
        let t = &tokens[j];
        if t.kind != TokenKind::Word {
            continue;
        }
        let w = t.text.as_str();
        if !(w == "f" || w == "fil" || w == "filius") {
            continue;
        }
        if j > 0 && tokens[j - 1].end < t.start {
            return true;
        }
        if j == 0 {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::tokenize;

    fn parse_str(input: &str) -> AuthState {
        let tokens = tokenize(input);
        parse(&tokens, 0)
    }

    fn authors(state: &AuthState) -> &[String] {
        &state.combination.authors
    }

    // ---- basic surname/initials inversion -------------------------------------------

    #[test]
    fn walker_f_flips_to_f_walker() {
        assert_eq!(authors(&parse_str("Walker F")), &["F.Walker".to_string()]);
    }

    #[test]
    fn run_on_all_caps_list_flips_every_author() {
        // The star example from the brief: three "Surname Initial(s)" pairs run
        // together with no separators at all — every all-caps token is undotted, so
        // every one flips (Quirk 5).
        assert_eq!(
            authors(&parse_str("Balsamo M Fregni E Tongiorgi MA")),
            &[
                "M.Balsamo".to_string(),
                "E.Fregni".to_string(),
                "M.A.Tongiorgi".to_string()
            ]
        );
    }

    #[test]
    fn balsamo_m_todaro_ma_variant() {
        assert_eq!(
            authors(&parse_str("Balsamo M Todaro MA")),
            &["M.Balsamo".to_string(), "M.A.Todaro".to_string()]
        );
    }

    #[test]
    fn comma_form_walker_f_also_flips() {
        assert_eq!(authors(&parse_str("Walker, F.")), &["F.Walker".to_string()]);
        assert_eq!(authors(&parse_str("Walker, F")), &["F.Walker".to_string()]);
    }

    #[test]
    fn particle_chain_keeps_h_da_c_monteiro_together() {
        // "H.da C." followed by "Monteiro": the buffer ends with a particle-only tail
        // ("da"/"C." — endsWithParticleOnly guards on the PARTICLE word, not the
        // trailing initial), so no inversion fires; it stays one chained author.
        assert_eq!(
            authors(&parse_str("H. da C. Monteiro")),
            &["H.da C.Monteiro".to_string()]
        );
    }

    #[test]
    fn isolated_zhang_f_before_separator_still_flips() {
        // Verified against NameParserImplTest's
        // "... Zhang F & Pan Z-X in Zhang, F, Pan, Z-X, ..., 2016" -> combAuthors(...,
        // "F.Zhang", "Z-X.Pan") — "F" is immediately followed by "&" (a separator, not
        // another surname) and still flips. This directly contradicts the Java inline
        // comment on this branch (see the module doc comment's Quirk 5 note); ported to
        // match the verified behaviour.
        assert_eq!(
            authors(&parse_str("Zhang F & Pan Z-X")),
            &["F.Zhang".to_string(), "Z-X.Pan".to_string()]
        );
    }

    #[test]
    fn middle_initial_surname_follows_keeps_one_author() {
        // "Roy L. Taylor": a DOTTED middle initial followed by a capitalised
        // surname-shaped word means this is ONE author with a spelled-out forename,
        // not an inversion.
        assert_eq!(
            authors(&parse_str("Calder & Roy L. Taylor")),
            &["Calder".to_string(), "Roy L.Taylor".to_string()]
        );
    }

    // ---- hyphenated initials ----------------------------------------------------------

    #[test]
    fn dotted_hyphenated_initials_preserve_case_and_dots() {
        assert_eq!(
            authors(&parse_str("Y.-j. Wang")),
            &["Y.-j.Wang".to_string()]
        );
        assert_eq!(
            authors(&parse_str("C.-K. Yang")),
            &["C.-K.Yang".to_string()]
        );
    }

    #[test]
    fn comma_inverted_hyphenated_initials() {
        assert_eq!(
            authors(&parse_str("Wang, Y.-j.")),
            &["Y.-j.Wang".to_string()]
        );
        assert_eq!(
            authors(&parse_str("Yang, C.-K.")),
            &["C.-K.Yang".to_string()]
        );
        assert_eq!(
            authors(&parse_str("Wang, Y.-j. & Liu, Z.-q.")),
            &["Y.-j.Wang".to_string(), "Z.-q.Liu".to_string()]
        );
    }

    // ---- generational suffix ------------------------------------------------------------

    #[test]
    fn generational_suffix_stays_upper_and_behind_surname() {
        assert_eq!(
            authors(&parse_str("Loeblich III")),
            &["Loeblich III".to_string()]
        );
    }

    #[test]
    fn generational_suffix_is_matched_case_insensitively_but_rendered_upper() {
        assert_eq!(
            authors(&parse_str("Loeblich Iii")),
            &["Loeblich III".to_string()]
        );
    }

    #[test]
    fn generational_suffix_with_year_and_basionym() {
        let s = parse_str("(Paulsen) Loeblich III, 1969");
        assert_eq!(s.basionym.authors, vec!["Paulsen".to_string()]);
        assert_eq!(s.combination.authors, vec!["Loeblich III".to_string()]);
        assert_eq!(s.combination.year, Some("1969".to_string()));
        assert!(s.basionym_present);
    }

    #[test]
    fn bare_initials_i_v_x_are_not_generational_suffixes() {
        // Single-letter I/V/X are genuine author initials, not generation-suffix Roman
        // numerals — GENERATIONAL_SUFFIXES deliberately excludes them, so these still go
        // through the ordinary (undotted, run-on) flip instead.
        assert_eq!(authors(&parse_str("Smith V")), &["V.Smith".to_string()]);
    }

    // ---- apostrophes ------------------------------------------------------------------

    #[test]
    fn dot_then_apostrophe_particle_glues_with_no_gap() {
        assert_eq!(
            authors(&parse_str("L.'t Mannetje")),
            &["L.'t Mannetje".to_string()]
        );
    }

    #[test]
    fn glued_apostrophe_inside_a_word_is_a_single_token_already() {
        assert_eq!(authors(&parse_str("M'Coy")), &["M'Coy".to_string()]);
        assert_eq!(
            authors(&parse_str("Abdallah & Sa'ad")),
            &["Abdallah".to_string(), "Sa'ad".to_string()]
        );
    }

    #[test]
    fn leading_apostrophe_particle_with_no_preceding_gap() {
        assert_eq!(authors(&parse_str("'t Hart")), &["'t Hart".to_string()]);
    }

    #[test]
    fn apostrophe_particle_preceded_by_a_space_gets_a_space_back() {
        assert_eq!(
            authors(&parse_str("Henk 't Hart")),
            &["Henk 't Hart".to_string()]
        );
    }

    #[test]
    fn question_mark_transcription_artefact_glues_silently() {
        assert_eq!(
            authors(&parse_str("Istv?nffi, 1898")),
            &["Istvnffi".to_string()]
        );
        assert_eq!(
            parse_str("Istv?nffi, 1898").combination.year,
            Some("1898".to_string())
        );
    }

    // ---- basionym / phase A ------------------------------------------------------------

    #[test]
    fn basionym_and_combination_authors_with_year() {
        let s = parse_str("(L.) L., 1753");
        assert_eq!(s.basionym.authors, vec!["L.".to_string()]);
        assert_eq!(s.combination.authors, vec!["L.".to_string()]);
        assert_eq!(s.combination.year, Some("1753".to_string()));
        assert!(s.basionym_present);
    }

    #[test]
    fn basionym_only_no_trailing_combination() {
        let s = parse_str("(Wang & Liu, 1996)");
        assert_eq!(
            s.basionym.authors,
            vec!["Wang".to_string(), "Liu".to_string()]
        );
        assert_eq!(s.basionym.year, Some("1996".to_string()));
        assert!(s.basionym_present);
        assert!(s.combination.authors.is_empty());
        assert_eq!(s.unparsed_from, -1);
    }

    #[test]
    fn basionym_bracketed_imprint_year() {
        let s = parse_str("(Peters, 1876 [1877])");
        assert_eq!(s.basionym.authors, vec!["Peters".to_string()]);
        assert_eq!(s.basionym.year, Some("1876".to_string()));
        assert_eq!(s.basionym.imprint_year, Some("1877".to_string()));
    }

    #[test]
    fn lowercase_only_parens_are_not_a_basionym_and_are_parked_unparsed() {
        // hasUpperWord guard: "(ilic)" has no upper-case WORD inside, so it is NOT read
        // as a basionym — it is captured as unparsed and skipped, and parsing continues
        // with the trailing "L." as the combination.
        let s = parse_str("(ilic) L.");
        assert!(!s.basionym_present);
        assert!(s.basionym.authors.is_empty());
        assert_eq!(s.unparsed_from, 0);
        assert_eq!(s.unparsed_text, Some("(ilic)".to_string()));
        assert_eq!(s.combination.authors, vec!["L.".to_string()]);
    }

    #[test]
    fn basionym_colon_sanctioning_is_dropped_inside_parens() {
        // "(Fr. : Fr.)": the colon inside the basionym parens separates the original
        // author from a sanctioning author, which is dropped entirely (canonical names
        // attribute sanctioning to the species level, never the basionym).
        let s = parse_str("(Fr. : Fr.) Fr.");
        assert_eq!(s.basionym.authors, vec!["Fr.".to_string()]);
        assert_eq!(s.combination.authors, vec!["Fr.".to_string()]);
        assert_eq!(s.sanctioning_author, None);
    }

    // ---- sanctioning author (phase B, top-level colon) --------------------------------

    #[test]
    fn top_level_colon_splits_off_sanctioning_author() {
        let s = parse_str("L. : Fr.");
        assert_eq!(s.combination.authors, vec!["L.".to_string()]);
        assert_eq!(s.sanctioning_author, Some("Fr.".to_string()));
    }

    #[test]
    fn pers_colon_fr_sanctioning() {
        let s = parse_str("Pers. : Fr.");
        assert_eq!(s.combination.authors, vec!["Pers.".to_string()]);
        assert_eq!(s.sanctioning_author, Some("Fr.".to_string()));
    }

    // ---- years --------------------------------------------------------------------------

    #[test]
    fn plain_year_only() {
        let s = parse_str("1771");
        assert_eq!(s.combination.year, Some("1771".to_string()));
        assert!(s.combination.authors.is_empty());
    }

    #[test]
    fn second_plain_year_becomes_imprint_year() {
        // No brackets at all: "Ehrenberg, 1870, 1869" — second plain year is the
        // imprint year (first-wins for the main year).
        let s = parse_str("Ehrenberg, 1870, 1869");
        assert_eq!(s.combination.authors, vec!["Ehrenberg".to_string()]);
        assert_eq!(s.combination.year, Some("1870".to_string()));
        assert_eq!(s.combination.imprint_year, Some("1869".to_string()));
    }

    #[test]
    fn unquoted_bracketed_year_is_imprint_year() {
        let s = parse_str("Storr, 1970 [1969]");
        assert_eq!(s.combination.authors, vec!["Storr".to_string()]);
        assert_eq!(s.combination.year, Some("1970".to_string()));
        assert_eq!(s.combination.imprint_year, Some("1969".to_string()));
    }

    #[test]
    fn bracketed_year_wins_as_imprint_even_when_the_only_year_given() {
        let s = parse_str("Fruhstorfer, [1912]");
        assert_eq!(s.combination.authors, vec!["Fruhstorfer".to_string()]);
        assert_eq!(s.combination.year, None);
        assert_eq!(s.combination.imprint_year, Some("1912".to_string()));
    }

    #[test]
    fn year_range_with_hyphen_keeps_first_year_and_sets_flag() {
        let s = parse_str("Smith, 1845-1847");
        assert_eq!(s.combination.year, Some("1845".to_string()));
        assert!(s.year_range);
    }

    #[test]
    fn year_range_with_short_second_year_and_slash() {
        let s = parse_str("Smith, 1987/92");
        assert_eq!(s.combination.year, Some("1987".to_string()));
        assert!(s.year_range);
    }

    #[test]
    fn non_range_year_does_not_set_year_range_flag() {
        let s = parse_str("Smith, 1987");
        assert_eq!(s.combination.year, Some("1987".to_string()));
        assert!(!s.year_range);
    }

    #[test]
    fn uncertain_trailing_question_mark_folds_into_year() {
        let s = parse_str("Smith, 198?");
        assert_eq!(s.combination.year, Some("198?".to_string()));
    }

    #[test]
    fn trailing_year_disambiguator_letter_is_dropped() {
        let s = parse_str("Smith, 1935h");
        assert_eq!(s.combination.year, Some("1935".to_string()));
        assert_eq!(s.combination.authors, vec!["Smith".to_string()]);
    }

    #[test]
    fn short_number_that_is_not_year_shaped_is_silently_dropped() {
        // A 1-2 digit number doesn't match the year branch (3-4 digits required) and
        // isn't any other token kind either, so it falls through to the catch-all and
        // is silently dropped rather than glued into the author text.
        let s = parse_str("Smith 12");
        assert_eq!(s.combination.authors, vec!["Smith".to_string()]);
        assert_eq!(s.combination.year, None);
    }

    // ---- ex-authors ---------------------------------------------------------------------

    #[test]
    fn ex_author_split() {
        let s = parse_str("Wedd. ex Sch. Bip.");
        assert_eq!(s.combination.ex_authors, vec!["Wedd.".to_string()]);
        assert_eq!(s.combination.authors, vec!["Sch.Bip.".to_string()]);
    }

    #[test]
    fn hort_ex_author_already_normalised_by_stripandstash() {
        // NB: the "Hort."/"hortus(a)" -> lower-case "hort." normalisation is
        // StripAndStash's job (a separate, already-ported stage) — by the time
        // AuthorshipParser sees tokens the word is already "hort.". This test exercises
        // AuthorshipParser's own ex-author handling directly on that post-normalisation
        // form, not the raw "Hort. ex Vilmorin" input.
        let s = parse_str("hort. ex Vilmorin");
        assert_eq!(s.combination.ex_authors, vec!["hort.".to_string()]);
        assert_eq!(s.combination.authors, vec!["Vilmorin".to_string()]);
    }

    // ---- suffixes (AUTHOR_SUFFIXES dual case-discipline) --------------------------------

    #[test]
    fn dot_glued_filius_chains_case_sensitively_no_separator() {
        assert_eq!(authors(&parse_str("L.f")), &["L.f".to_string()]);
    }

    #[test]
    fn spaced_filius_chains_with_a_separating_space() {
        assert_eq!(authors(&parse_str("Hooker f.")), &["Hooker f.".to_string()]);
    }

    #[test]
    fn bis_suffix_chains_onto_the_surname() {
        assert_eq!(
            authors(&parse_str("Yong Wang bis")),
            &["Yong Wang bis".to_string()]
        );
        assert_eq!(
            authors(&parse_str("A.Murray bis")),
            &["A.Murray bis".to_string()]
        );
    }

    #[test]
    fn lone_suffix_after_comma_attaches_to_the_previous_completed_author() {
        // Constructed to exercise the case-insensitive standalone-after-separator
        // branch specifically (Quirk 2): after the COMMA flush, `cur` is empty and
        // "jr" arrives as a fresh top-level token (not chained from "Smith" within the
        // same pass), so it hits the `cur.is_empty() && !authors.is_empty()` arm which
        // pops the last author, appends a trailing dot if missing, then glues "jr" —
        // this is a structurally distinct code path from the chain sub-loop exercised
        // by `bis_suffix_chains_onto_the_surname` above (verified by tracing the Java
        // source; no directly analogous assertion exists in `NameParserImplTest` or the
        // corpus for this exact input).
        let s = parse_str("Smith, jr");
        assert_eq!(s.combination.authors, vec!["Smith.jr".to_string()]);
    }

    #[test]
    fn author_suffixes_case_insensitive_lone_branch_accepts_mixed_case_too() {
        // "jR" (lower-case first letter, so it does NOT hit the surname/upper-case
        // branch first — that branch would otherwise intercept anything starting
        // upper-case, e.g. a fully-capitalised "JR", before the lone-suffix branch ever
        // gets a look) still matches AUTHOR_SUFFIXES via the case-insensitive
        // `.to_lowercase()` comparison, and is glued on using its ORIGINAL casing.
        assert_eq!(authors(&parse_str("Smith, jR")), &["Smith.jR".to_string()]);
    }

    // ---- filius flag (has_filius) -------------------------------------------------------

    #[test]
    fn spaced_f_sets_has_filius_flag() {
        assert!(parse_str("Hooker f.").has_filius);
    }

    #[test]
    fn glued_f_does_not_set_has_filius_flag() {
        assert!(!parse_str("Burm.f.").has_filius);
    }

    #[test]
    fn filius_before_ex_does_not_count() {
        // "Baker f. ex Rose": the filius marker is on the EX author, which was never
        // the validating author — containsFiliusSuffix skips everything before the
        // LAST "ex" token.
        assert!(!parse_str("Baker f. ex Rose").has_filius);
    }

    // ---- semicolon-separated citation list ----------------------------------------------

    #[test]
    fn semicolon_separated_citation_list() {
        assert_eq!(
            authors(&parse_str("Choi,J.H.; Im,W.T.; Yoo,J.S.")),
            &[
                "J.H.Choi".to_string(),
                "W.T.Im".to_string(),
                "J.S.Yoo".to_string()
            ]
        );
    }

    // ---- slash between authors -----------------------------------------------------------

    #[test]
    fn slash_between_two_authors_stays_glued() {
        assert_eq!(
            authors(&parse_str("Smith/Jones")),
            &["Smith/Jones".to_string()]
        );
    }

    // ---- malformed / catch-all ------------------------------------------------------------

    #[test]
    fn stray_unmatched_open_paren_is_silently_skipped() {
        let s = parse_str("Wedd. ex Sch. Bip. (");
        assert_eq!(s.combination.ex_authors, vec!["Wedd.".to_string()]);
        assert_eq!(s.combination.authors, vec!["Sch.Bip.".to_string()]);
    }

    #[test]
    fn diacritic_surname_chains_through_dot() {
        assert_eq!(authors(&parse_str("Müll.Arg.")), &["Müll.Arg.".to_string()]);
    }

    #[test]
    fn all_caps_long_surname_is_title_cased() {
        let s = parse_str("FISCHER 1885");
        assert_eq!(s.combination.authors, vec!["Fischer".to_string()]);
        assert_eq!(s.combination.year, Some("1885".to_string()));
    }

    #[test]
    fn short_all_caps_word_is_kept_as_initials_not_title_cased() {
        // normaliseAuthorCase only title-cases ALL-CAPS words of length >= 4; "MA" (2
        // chars) stays untouched, matching its role as initials rather than a shouted
        // surname.
        assert_eq!(
            authors(&parse_str("Balsamo M Todaro MA")),
            &["M.Balsamo".to_string(), "M.A.Todaro".to_string()]
        );
    }

    // ---- particles -----------------------------------------------------------------------

    #[test]
    fn van_der_particle_chain_stays_lower_case() {
        assert_eq!(
            authors(&parse_str("van der Wulp")),
            &["van der Wulp".to_string()]
        );
    }

    #[test]
    fn capitalised_van_particle_chain() {
        assert_eq!(
            authors(&parse_str("Van de Putte")),
            &["Van de Putte".to_string()]
        );
    }

    // ---- multi-author separators ----------------------------------------------------------

    #[test]
    fn ampersand_and_et_and_y_separators() {
        assert_eq!(
            authors(&parse_str("Xing, Yan & Yin")),
            &["Xing".to_string(), "Yan".to_string(), "Yin".to_string()]
        );
        assert_eq!(
            authors(&parse_str("Martinez y Saez")),
            &["Martinez".to_string(), "Saez".to_string()]
        );
    }

    #[test]
    fn literal_or_is_not_a_recognised_separator() {
        // "or" is not "and"/"et"/"y"/"&" — it glues into a single literal author string.
        assert_eq!(
            authors(&parse_str("Jarocki or Schinz")),
            &["Jarocki or Schinz".to_string()]
        );
    }

    // ---- edge cases / dead-code confirmation ----------------------------------------------

    #[test]
    fn empty_span_returns_a_fully_default_state_without_panicking() {
        let tokens = tokenize("Aus bus");
        // `from == tokens.len()`: both phase-A and phase-B guards are false, so phase C's
        // guard is also false on entry (see the module doc comment) — nothing panics and
        // nothing is recorded as unparsed.
        let s = parse(&tokens, tokens.len());
        assert!(s.combination.authors.is_empty());
        assert_eq!(s.unparsed_from, -1);
        assert_eq!(s.unparsed_text, None);
    }

    #[test]
    fn phase_c_never_fires_a_regression_pin() {
        // Phase B unconditionally sets `i = n` whenever it runs, and it runs whenever
        // `i < n` on entry — so by construction `i == n` on every path through `parse`,
        // meaning Phase C's own `if i < n` can never be true. This is a regression pin
        // for that observation across a handful of shapes that might plausibly have
        // been thought to reach it (a trailing unmatched paren, a basionym-only input, a
        // combination with a sanctioning author).
        // NB: "(ilic) L." is deliberately NOT in this list — it sets `unparsed_from`
        // too, but via Phase A's own malformed-basionym guard (`hasUpperWord` failing),
        // a completely different branch from Phase C; see
        // `lowercase_only_parens_are_not_a_basionym_and_are_parked_unparsed` above.
        for input in ["Wedd. ex Sch. Bip. (", "(Wang & Liu, 1996)", "L. : Fr."] {
            let s = parse_str(input);
            assert_eq!(
                s.unparsed_from, -1,
                "input {input:?} unexpectedly hit phase C"
            );
        }
    }
}
