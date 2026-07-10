// SPDX-License-Identifier: Apache-2.0

//! Java `org.gbif.nameparser.pipeline.AuthorshipSplit` (471 lines) — locates the token
//! index where the authorship section begins in a tokenised input. Entirely regex-free: a
//! hand-written token-index state machine, and a pure function of `tokens` +
//! `ctx.requested_rank` (no side effects — unlike `NameTokens`, Phase 1 Slice 3 Task 3,
//! which mutates `ParseContext` to set the name-part fields once the boundary is known).
//!
//! Ported branch-for-branch from the Java source, including the `OPEN_PAREN`
//! subgenus-vs-parenthesised-basionym-author 7-rule cascade documented inline on
//! [`find_boundary`] (Java `AuthorshipSplit.java` ~lines 232-252) — the rule order there is
//! load-bearing and preserved exactly.
//!
//! [`skip_paren_author_block`] here is **AuthorshipSplit's own copy**: it gates its match on
//! [`has_epithet_after_marker`] (with `infrageneric = false`) before returning the marker
//! index. `NameTokens` (Task 3) has its own textually-different copy of a same-named helper
//! that returns at the first infraspecific-marker word with no such epithet-follow check —
//! the two are intentionally NOT unified (matches the Java source, which also keeps two
//! separate private methods of the same name in two different classes).
//!
//! This file also carries a handful of small free functions that exist only to bridge a
//! capability Java's `Token`/`Rank` types expose as instance methods
//! (`Token.startsUpper()`/`startsLower()`/`startsDigitEpithet()`,
//! `Rank.isInfragenericStrictly()`) but this crate's `Token`/`Rank` types don't yet carry as
//! methods themselves — see [`starts_upper`], [`starts_lower`], [`starts_digit_epithet`] and
//! [`rank_is_infrageneric_strictly`]'s own doc comments for why they're local free functions
//! here rather than additions to `token.rs`/`model/enums.rs` (this task's own brief scopes
//! its file footprint to this file, plus the one-line module declaration in
//! `pipeline/mod.rs`).

use crate::model::Rank;
use crate::pipeline::rank_markers;
use crate::pipeline::ParseContext;
use crate::token::{self, Token, TokenKind};

/// Java `AuthorshipSplit.findBoundary(List<Token> tokens, ParseContext ctx)`
/// (`AuthorshipSplit.java:17-297`). Walks the token list from the start, tracking just
/// enough state (word count, whether a genus/subgenus/epithet has been seen, whether the
/// genus was all-caps or family-shaped) to recognise where the name section ends and the
/// authorship section begins. Returns `tokens.len()` when the whole input is name (no
/// authorship present at all).
///
/// The `OPEN_PAREN` branch below implements a 7-rule cascade to decide whether a
/// parenthesised word after a genus is a **subgenus** (name continues) or a **parenthesised
/// basionym author** (the paren opens the authorship). In order:
///  1. a species epithet (lower-case, non-particle) follows → subgenus, the name continues
///     below it ("Amnicola (Amnicola) dubrueilliana", "Phalaena (Tin.) guttella Fab.") — a
///     basionym author cannot sit before a species epithet, so even an abbreviated "(Tin.)"
///     is the subgenus here;
///  2. otherwise an abbreviated / initialled word ("(Griseb.)", "(Grev.)") is a basionym
///     author — subgenera are always a single UNabbreviated capitalised word, never initials
///     ("Thliphthisa (Griseb.) P.Caputo & Del Guacchio", "Genus (Grev.) Kütz. 1849");
///  3. nothing follows ("Arrhoges (Antarctohoges)") → subgenus;
///  4. the word repeats the genus — a nominotypical subgenus ("Morea (Morea) …");
///  5. the caller asked for an infrageneric rank → subgenus;
///  6. the trailing authorship carries a year OUTSIDE the parens — the zoological "Genus
///     (Subgenus) Author, year" form ("Dicromita (Pterodicromita) Fowler, 1925") → subgenus;
///  7. otherwise a trailing author with no year makes "(Word)" the basionym author of a
///     botanical genus recombination ("Kyphocarpa (Fenzl) Lopr.").
///
/// (A "(Author, year)" with the year INSIDE the parens is a multi-token paren that never
/// reaches this single-word branch and is treated as a basionym below, via
/// [`skip_paren_author_block`].)
pub fn find_boundary(tokens: &[Token], ctx: &ParseContext) -> usize {
    let n = tokens.len();
    if n == 0 {
        return 0;
    }

    let mut i = 0usize;
    let mut name_words = 0u32;
    let mut after_genus = false;
    let mut after_subgenus = false;
    let mut have_epithet = false;
    let mut genus_all_caps = false;
    let mut genus_family_shape = false;
    let mut genus_text: Option<&str> = None;

    while i < n {
        let t = &tokens[i];

        if t.kind == TokenKind::HybridMark {
            i += 1;
            continue;
        }

        // Missing-genus placeholder: "?" as the genus stand-in.
        if name_words == 0 && t.kind == TokenKind::Other && t.text == "?" {
            name_words += 1;
            after_genus = true;
            i += 1;
            continue;
        }
        // Open-nomenclature doubtful-identification "?" between epithets — like cf./aff.
        // Skip the marker so the next epithet is included in the name section.
        if after_genus && t.kind == TokenKind::Other && t.text == "?" {
            i += 1;
            continue;
        }

        if t.kind == TokenKind::Word {
            if name_words == 0 {
                if starts_upper(t) {
                    name_words += 1;
                    after_genus = true;
                    let len = t.text.chars().count();
                    genus_all_caps = len > 1 && is_all_upper(&t.text);
                    genus_family_shape = is_family_shape(&t.text);
                    genus_text = Some(t.text.as_str());
                    i += 1;
                    // Abbreviated genus: 1-letter ("M.") always; 2-4 letters ("Mo.",
                    // "Phl.") only when the next non-dot token is a lowercase epithet —
                    // so we don't fold a real binomial like "Mo Bing 1980" into "Mo." +
                    // "Bing".
                    if (1..=4).contains(&len) && i < n && tokens[i].kind == TokenKind::Dot {
                        let short_enough_for_abbrev = len == 1
                            || (i + 1 < n
                                && tokens[i + 1].kind == TokenKind::Word
                                && starts_lower(&tokens[i + 1]));
                        if short_enough_for_abbrev {
                            i += 1;
                        }
                    }
                    continue;
                }
                // Lower-case first token — accept as a recovered genus and continue.
                if t.text.chars().count() >= 2 {
                    name_words += 1;
                    after_genus = true;
                    i += 1;
                    continue;
                }
                return i;
            }
            if starts_lower(t) {
                let w = strip_dot(&t.text);
                // "anon" / "anon." — anonymous-author placeholder. Treated as the start
                // of authorship even though it's lowercase.
                if w.eq_ignore_ascii_case("anon") {
                    return i;
                }
                // cf./aff. qualifiers and indet markers — keep walking
                if w.eq_ignore_ascii_case("cf")
                    || w.eq_ignore_ascii_case("aff")
                    || w.eq_ignore_ascii_case("sp")
                    || w.eq_ignore_ascii_case("spec")
                    || w.eq_ignore_ascii_case("species")
                    || w.eq_ignore_ascii_case("indet")
                {
                    let is_sp = w.eq_ignore_ascii_case("sp") || w.eq_ignore_ascii_case("spec");
                    i += 1;
                    if i < n && tokens[i].kind == TokenKind::Dot {
                        i += 1;
                    }
                    // A number immediately after an indet marker is the informal
                    // phrase, not authorship.
                    if i < n && tokens[i].kind == TokenKind::Number {
                        i += 1;
                    } else if is_sp
                        && i < n
                        && tokens[i].kind == TokenKind::Word
                        && (tokens[i].text.chars().count() >= 2
                            || (tokens[i].text.chars().count() == 1 && starts_upper(&tokens[i])))
                        && (i + 1 == n
                            || (i + 1 < n && tokens[i + 1].kind == TokenKind::Number && i + 2 == n))
                    {
                        // Strain-code-shaped trailing token(s) ("Lepidoptera sp.
                        // JGP0404") OR a single uppercase letter ("Bryozoan sp. E") form
                        // the species epithet payload / phrase, not authorship —
                        // include them in the name span.
                        i += 1;
                        if i < n && tokens[i].kind == TokenKind::Number {
                            i += 1;
                        }
                    }
                    continue;
                }
                // Aggregate suffix words within the name section
                if w.eq_ignore_ascii_case("agg")
                    || w.eq_ignore_ascii_case("aggregate")
                    || w.eq_ignore_ascii_case("group")
                    || w.eq_ignore_ascii_case("complex")
                {
                    i += 1;
                    if i < n && tokens[i].kind == TokenKind::Dot {
                        i += 1;
                    }
                    continue;
                }
                // Infraspecific rank marker (incl. "notho" prefix variants)
                if rank_markers::match_infraspecific_allow_notho(w).is_some() {
                    i += 1;
                    if i < n && tokens[i].kind == TokenKind::Dot {
                        i += 1;
                    }
                    // microbial f. sp.
                    if i + 1 < n
                        && tokens[i].kind == TokenKind::Word
                        && tokens[i].text.eq_ignore_ascii_case("sp")
                    {
                        i += 1;
                        if i < n && tokens[i].kind == TokenKind::Dot {
                            i += 1;
                        }
                    }
                    // Single uppercase letter immediately after a rank marker is an
                    // informal infra epithet ("form A", "f. B"), not the start of
                    // authorship.
                    if i < n
                        && tokens[i].kind == TokenKind::Word
                        && tokens[i].text.chars().count() == 1
                        && starts_upper(&tokens[i])
                    {
                        i += 1;
                        have_epithet = true;
                    }
                    continue;
                }
                // Infrageneric rank marker, e.g. "subg." / "nothosect." — consume
                // marker, dot, and the following capitalised epithet.
                if after_genus
                    && !have_epithet
                    && !after_subgenus
                    && rank_markers::match_infrageneric_allow_notho(w).is_some()
                {
                    i += 1;
                    if i < n && tokens[i].kind == TokenKind::Dot {
                        i += 1;
                    }
                    if i < n && tokens[i].kind == TokenKind::Word && starts_upper(&tokens[i]) {
                        i += 1;
                        after_subgenus = true;
                    }
                    continue;
                }
                if token::is_particle(&t.text) || looks_like_apostrophe_particle(&t.text) {
                    // Particle authors may be followed by a structural rank marker —
                    // try to skip past the author span as a mid-name author so the
                    // marker still gets consumed by the name section.
                    if let Some(after_author) = consume_mid_name_author(tokens, i) {
                        i = after_author;
                        continue;
                    }
                    return i;
                }
                // "hort." — horticultural marker, used as an ex-author placeholder
                // ("Acacia hort. ex Dallim."). Treat as authorship boundary.
                if w.eq_ignore_ascii_case("hort") {
                    return i;
                }
                name_words += 1;
                have_epithet = true;
                after_subgenus = false;
                i += 1;
                continue;
            }
            if after_genus && starts_digit_epithet(t) {
                name_words += 1;
                have_epithet = true;
                after_subgenus = false;
                i += 1;
                continue;
            }
            // Mid-name author span: an Author abbreviation between the genus (or
            // species epithet) and a following rank marker. e.g. "Centaurea L. subg.
            // Jacea" or "Festuca ovina L. subvar. gracilis Hackel". The author tokens
            // are silently consumed so the boundary stays at the structural marker.
            if let Some(after_author) = consume_mid_name_author(tokens, i) {
                i = after_author;
                continue;
            }
            // All-caps multi-letter word in epithet position only counts as an
            // upper-cased epithet when the genus itself was all-caps (so the whole
            // input is shouted) and it isn't followed by an abbreviation dot (ELEV. →
            // author). A diacritic no longer disqualifies it: "CHIONE ELEVÄTA" is read
            // like "CHIONE ELEVATA".
            if genus_all_caps && after_genus && t.text.chars().count() > 1 && is_all_upper(&t.text)
            {
                let is_abbrev = i + 1 < n && tokens[i + 1].kind == TokenKind::Dot;
                if !is_abbrev {
                    name_words += 1;
                    have_epithet = true;
                    after_subgenus = false;
                    i += 1;
                    continue;
                }
            }
            // Upper-case word in non-first position → authorship.
            return i;
        }

        if t.kind == TokenKind::OpenParen {
            if after_genus && !have_epithet && !after_subgenus && !genus_family_shape {
                let j = i + 1;
                // The subgenus word is normally Title-cased; a lower-case word
                // ("(acanthoderes)") is a malformed subgenus that NameTokens
                // capitalises and flags doubtful.
                if j < n
                    && tokens[j].kind == TokenKind::Word
                    && (starts_upper(&tokens[j]) || starts_lower(&tokens[j]))
                {
                    // A single parenthesised word — plain "(Word)" or abbreviated
                    // "(Word.)".
                    let k = j + 1;
                    let mut after_paren: Option<usize> = None;
                    let mut abbreviated = false;
                    if k < n && tokens[k].kind == TokenKind::CloseParen {
                        after_paren = Some(k + 1);
                    } else if starts_upper(&tokens[j])
                        && k + 1 < n
                        && tokens[k].kind == TokenKind::Dot
                        && tokens[k + 1].kind == TokenKind::CloseParen
                    {
                        after_paren = Some(k + 2);
                        abbreviated = true;
                    }
                    if let Some(after_paren) = after_paren {
                        let has_trailing = after_paren < n;
                        let next = if has_trailing {
                            Some(&tokens[after_paren])
                        } else {
                            None
                        };
                        let trailing_is_epithet = next.is_some_and(|nx| {
                            nx.kind == TokenKind::Word
                                && starts_lower(nx)
                                && !token::is_particle(&nx.text)
                        });
                        let nominotypical =
                            genus_text.is_some_and(|g| eq_ignore_case(g, &tokens[j].text));
                        let rank_requests_infragen = ctx
                            .requested_rank
                            .is_some_and(rank_is_infrageneric_strictly);
                        let subgenus = if trailing_is_epithet {
                            true
                        } else if abbreviated {
                            false
                        } else {
                            !has_trailing
                                || nominotypical
                                || rank_requests_infragen
                                || has_year_token(tokens, after_paren, n)
                        };
                        if subgenus {
                            i = after_paren;
                            after_subgenus = true;
                            continue;
                        }
                        return i;
                    }
                }
            }
            // After the species epithet, an "(BasAuth) CombAuth var. infraspecific"
            // pattern means the parenthesised basionym + combination author span sits
            // between the species and the infraspecific portion. Skip it so the rank
            // marker + epithet can be consumed as part of the name span.
            if have_epithet && !after_subgenus {
                if let Some(after_span) = skip_paren_author_block(tokens, i) {
                    i = after_span;
                    continue;
                }
            }
            return i;
        }

        // any other token (number, dot, comma, dagger, etc.) → authorship boundary
        return i;
    }
    n
}

/// Java `Token.startsUpper()` (`Token.java:20-22`). Not a Rust `Token` method (this task's
/// brief caps its file footprint at this file + the one-line module declaration in
/// `pipeline/mod.rs`, so `token.rs` is intentionally left untouched) — ported here as a
/// free function taking `&Token` instead. `Character.isUpperCase(codePointAt(0))` ~ Rust
/// `char::is_uppercase()` on the first `char` (Unicode scalar value, i.e. already a decoded
/// code point — matching Java's `codePointAt` semantics even for astral characters, more
/// faithfully than a naive UTF-16-code-unit read would).
fn starts_upper(t: &Token) -> bool {
    t.text.chars().next().is_some_and(|c| c.is_uppercase())
}

/// Java `Token.startsLower()` (`Token.java:24-26`). See [`starts_upper`]'s doc comment for
/// why this is a free function rather than a `Token` method.
fn starts_lower(t: &Token) -> bool {
    t.text.chars().next().is_some_and(|c| c.is_lowercase())
}

/// Java `Token.startsDigitEpithet()` (`Token.java:29-33`): true for an alphanumeric epithet
/// word that begins with a digit, e.g. "11-punctata". `Character.isDigit` is approximated
/// as ASCII-only, matching `token.rs`'s own established approximation for the tokenizer's
/// digit recognition (see that module's `is_digit` doc comment) — consistent, since a WORD
/// token can only ever begin with a digit at all when the tokenizer's own (ASCII-only)
/// digit-glued-word rule produced it in the first place.
fn starts_digit_epithet(t: &Token) -> bool {
    t.kind == TokenKind::Word
        && t.text.chars().next().is_some_and(|c| c.is_ascii_digit())
        && t.text.chars().any(|c| c.is_alphabetic())
}

/// Java `Rank.isInfragenericStrictly()` (`Rank.java:500-502`):
/// `isInfrageneric() && ordinal() < SPECIES_AGGREGATE.ordinal()`, where `isInfrageneric()`
/// is `ordinal() > GENUS.ordinal() && notOtherOrUnranked()`. In Java's full 117-constant
/// enum, the ranks strictly between `GENUS` and `SPECIES_AGGREGATE` are `SUBGENUS`,
/// `INFRAGENUS`, `DIVISION_BOTANY`, `SUPERSECTION_BOTANY`, `SECTION_BOTANY`,
/// `SUBSECTION_BOTANY`, `SUPERSERIES_BOTANY`, `SERIES_BOTANY`, `SUBSERIES_BOTANY`,
/// `INFRAGENERIC_NAME` — the last of those was added to this crate's `Rank` stub by Phase 1
/// Slice 3 Task 3 (`NameTokens` needs it as a bare `Rank.INFRAGENERIC_NAME` literal) and is
/// matched `true` below alongside the rest; `Infragenus` remains the sole member of that
/// Java range still absent from the stub (`model/enums.rs`) — every OTHER member already
/// exists as a Rust variant and is matched `true` below.
///
/// This crate's `Rank` doesn't carry Java's ordinal order in its own declaration order (the
/// stub's variants are grouped by the task that added them, not by Java ordinal), so this
/// can't be a `<` comparison the way `Rank.java` writes it; an explicit match reproduces the
/// same rank set instead. Exhaustive (no wildcard arm), matching the precedent set by this
/// crate's `Rank::code()` (`model/enums.rs`): a future slice adding a `Rank` variant gets a
/// compile error here forcing an explicit decision, rather than silently defaulting to
/// `false`. Scoped as a private free function rather than a `Rank` method for the same
/// file-footprint reason as [`starts_upper`].
fn rank_is_infrageneric_strictly(rank: Rank) -> bool {
    match rank {
        Rank::Subgenus
        | Rank::DivisionBotany
        | Rank::SupersectionBotany
        | Rank::SectionBotany
        | Rank::SubsectionBotany
        | Rank::SuperseriesBotany
        | Rank::SeriesBotany
        | Rank::SubseriesBotany
        | Rank::InfragenericName => true,
        Rank::Unranked
        | Rank::Family
        | Rank::Genus
        | Rank::Species
        | Rank::Grex
        | Rank::Subspecies
        | Rank::CultivarGroup
        | Rank::Variety
        | Rank::Form
        | Rank::Cultivar
        | Rank::Subfamily
        | Rank::Tribe
        | Rank::Subtribe
        | Rank::Supertribe
        | Rank::Infratribe
        | Rank::SectionZoology
        | Rank::SubsectionZoology
        | Rank::SupersectionZoology
        | Rank::SeriesZoology
        | Rank::SubseriesZoology
        | Rank::SuperseriesZoology
        | Rank::Other
        | Rank::Subvariety
        | Rank::Subform
        | Rank::Pathovar
        | Rank::Biovar
        | Rank::Chemoform
        | Rank::Serovar
        | Rank::Morph
        | Rank::Morphovar
        | Rank::Phagovar
        | Rank::Natio
        | Rank::Mutatio
        | Rank::Convariety
        | Rank::Proles
        | Rank::Aberration
        | Rank::Strain
        | Rank::InfraspecificName
        | Rank::FormaSpecialis
        | Rank::InfrasubspecificName => false,
    }
}

/// Java `String.equalsIgnoreCase(String)` used on two runtime-derived strings (the genus
/// text vs. a repeated subgenus word — neither is a fixed ASCII literal, unlike the many
/// `eq_ignore_ascii_case("literal")` checks elsewhere in this file), so a full Unicode case
/// fold is used, matching the `.to_lowercase()` idiom already established for this purpose
/// elsewhere in the crate (`token::is_particle`, `rank_markers::match_infraspecific`).
fn eq_ignore_case(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// Java `AuthorshipSplit.stripDot(String)` (`AuthorshipSplit.java:299-301`).
fn strip_dot(s: &str) -> &str {
    s.strip_suffix('.').unwrap_or(s)
}

/// Java `AuthorshipSplit.midNameAuthorEnd(List<Token>, int, int)` (`AuthorshipSplit.java:309-311`).
/// Public bridge so `NameTokens` (Task 3) can apply the same mid-name-author skipping.
/// Adapted to Rust slices/indices: the redundant Java `n` parameter (always
/// `tokens.size()`) is dropped in favour of `tokens.len()`, and the `-1`-sentinel `int`
/// return becomes `Option<usize>` (`None` where Java returns `-1`) — a value-preserving
/// adaptation, since every actual (non-sentinel) return value of the underlying
/// `consumeMidNameAuthor`/[`consume_mid_name_author`] is, by construction, strictly greater
/// than `from` (see that function's own doc comment), exactly the condition every Java call
/// site tests for (`afterAuthor > i`) before using the value.
pub fn mid_name_author_end(tokens: &[Token], from: usize) -> Option<usize> {
    consume_mid_name_author(tokens, from)
}

/// Java `AuthorshipSplit.consumeMidNameAuthor(List<Token>, int, int)`
/// (`AuthorshipSplit.java:313-358`). If `tokens[from]` starts an author span that is
/// followed by a rank marker (e.g. "L. subg.", "L. subvar.", "Asch. subsp."), returns the
/// index just past the author span (the index of the rank marker); otherwise `None`.
///
/// Every success path below requires `j > from` before returning `Some(j)` (mirrored from
/// Java's own `j > from` guard) — so `Some(j)` here always means `j > from`, matching every
/// Java call site's `afterAuthor > i` test (see [`mid_name_author_end`]'s doc comment).
fn consume_mid_name_author(tokens: &[Token], from: usize) -> Option<usize> {
    let n = tokens.len();
    if from >= n {
        return None;
    }
    let first = &tokens[from];
    if first.kind != TokenKind::Word {
        return None;
    }
    // Author span starts with an uppercase word OR a particle ("d'", "de", "van", …).
    if !starts_upper(first)
        && !token::is_particle(&first.text)
        && !looks_like_apostrophe_particle(&first.text)
    {
        return None;
    }
    let mut j = from;
    while j < n {
        let t = &tokens[j];
        if t.kind == TokenKind::Word {
            if starts_upper(t) {
                j += 1;
                continue;
            }
            if token::is_particle(&t.text) {
                j += 1;
                continue;
            }
            // Apostrophe-particle word ("d'Urv", "L'Hér") — keep walking.
            if looks_like_apostrophe_particle(&t.text) {
                j += 1;
                continue;
            }
            // "al" / "al." inside an author span ("Boiss. & al. var. paryadrica") — the
            // "et al." abbreviation. Keep walking.
            if t.text.eq_ignore_ascii_case("al") {
                j += 1;
                continue;
            }
            let w = strip_dot(&t.text);
            let is_infra_marker = rank_markers::match_infraspecific(w).is_some()
                || rank_markers::match_infraspecific_allow_notho(w).is_some();
            let is_infra_gen_marker = rank_markers::match_infrageneric(w).is_some();
            if (is_infra_marker || is_infra_gen_marker)
                && j > from
                && has_epithet_after_marker(tokens, j, is_infra_gen_marker)
            {
                return Some(j);
            }
            return None;
        }
        if t.kind == TokenKind::Dot || t.kind == TokenKind::Ampersand || t.kind == TokenKind::Comma
        {
            j += 1;
            continue;
        }
        // Apostrophe inside an author (M'Coy, d'Urv., L'Hér.) — keep walking.
        if t.kind == TokenKind::Other && t.text == "'" {
            j += 1;
            continue;
        }
        return None;
    }
    None
}

/// Java `AuthorshipSplit.hasEpithetAfterMarker(List<Token>, int, int, boolean)`
/// (`AuthorshipSplit.java:365-384`). After a rank marker we expect an epithet — lowercase
/// for infraspecific markers, uppercase for infrageneric ones. Without it, the apparent
/// "marker" was just a lowercase token (e.g. "f.") that happened to spell a known marker.
fn has_epithet_after_marker(tokens: &[Token], marker_idx: usize, infrageneric: bool) -> bool {
    let n = tokens.len();
    let mut k = marker_idx + 1;
    if k < n && tokens[k].kind == TokenKind::Dot {
        k += 1;
    }
    if k >= n {
        // "f" is ambiguous: could be forma-rank or the "filius" author suffix. Treat a
        // trailing "f." as filius (not a rank marker) to avoid misclassification.
        let mw = tokens[marker_idx].text.to_lowercase();
        return mw != "f";
    }
    let t = &tokens[k];
    if t.kind != TokenKind::Word {
        return false;
    }
    if infrageneric {
        return starts_upper(t);
    }
    if !starts_lower(t) {
        return false;
    }
    // Reject lowercase tokens that aren't real epithets (ex/and/et/y separators,
    // particles).
    if t.text.eq_ignore_ascii_case("ex")
        || t.text.eq_ignore_ascii_case("and")
        || t.text.eq_ignore_ascii_case("et")
        || t.text == "y"
    {
        return false;
    }
    if token::is_particle(&t.text) {
        return false;
    }
    true
}

/// Java `AuthorshipSplit.hasYearToken(List<Token>, int, int)` (`AuthorshipSplit.java:391-400`).
/// True when `tokens[from, to)` contain a 4-digit year-shaped number (1xxx / 2xxx). Written
/// as an explicit index loop (not a `tokens[from..to]` slice) so an out-of-order `from > to`
/// — never hit by the current call site's own invariant, but not statically impossible —
/// behaves like Java's `for (i = from; i < to; i++)` (simply doesn't iterate) rather than
/// panicking the way slicing with a backwards range would.
fn has_year_token(tokens: &[Token], from: usize, to: usize) -> bool {
    let mut i = from;
    while i < to {
        let t = &tokens[i];
        if t.kind == TokenKind::Number
            && t.text.chars().count() == 4
            && matches!(t.text.chars().next(), Some('1') | Some('2'))
        {
            return true;
        }
        i += 1;
    }
    false
}

/// Java `AuthorshipSplit.skipParenAuthorBlock(List<Token>, int, int)`
/// (`AuthorshipSplit.java:402-438`) — **this class's own copy**; see this module's own doc
/// comment for why `NameTokens`'s same-named-but-different helper (Task 3) is not unified
/// with this one. If a "(...) Author. ranklabel." span sits between the species and an
/// infraspecific epithet, returns the index of the rank-marker word; otherwise `None`.
fn skip_paren_author_block(tokens: &[Token], open_idx: usize) -> Option<usize> {
    let n = tokens.len();
    // Match the closing paren.
    let mut depth = 1i32;
    let mut j = open_idx + 1;
    while j < n && depth > 0 {
        let k = tokens[j].kind;
        if k == TokenKind::OpenParen {
            depth += 1;
        } else if k == TokenKind::CloseParen {
            depth -= 1;
        }
        if depth == 0 {
            break;
        }
        j += 1;
    }
    if j >= n || depth != 0 {
        return None;
    }
    j += 1; // skip past the close paren
            // Walk over an author span (uppercase words, dots, particles) until a rank marker.
    while j < n {
        let t = &tokens[j];
        if t.kind == TokenKind::Word {
            if starts_upper(t) {
                j += 1;
                continue;
            }
            if token::is_particle(&t.text) {
                j += 1;
                continue;
            }
            let w = strip_dot(&t.text);
            let is_infra_marker = rank_markers::match_infraspecific_allow_notho(w).is_some();
            if is_infra_marker && has_epithet_after_marker(tokens, j, false) {
                return Some(j);
            }
            return None;
        }
        if t.kind == TokenKind::Dot || t.kind == TokenKind::Ampersand || t.kind == TokenKind::Comma
        {
            j += 1;
            continue;
        }
        return None;
    }
    None
}

/// Java `AuthorshipSplit.isFamilyShape(String)` (`AuthorshipSplit.java:442-445`).
/// Globally-unambiguous family-shape suffix: a leading word ending in "-aceae" or
/// "-oideae" is always a botanical family-group name (per `RankUtils.GLOBAL_SUFFICES`).
fn is_family_shape(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower.ends_with("aceae") || lower.ends_with("oideae")
}

/// Java `AuthorshipSplit.isApostropheParticle(String)` (`AuthorshipSplit.java:448-450`).
/// Public bridge so `NameTokens` shares the same apostrophe-particle test ("d'Urv", "L'Hér").
pub fn is_apostrophe_particle(s: &str) -> bool {
    looks_like_apostrophe_particle(s)
}

/// Java `AuthorshipSplit.looksLikeApostropheParticle(String)`
/// (`AuthorshipSplit.java:452-457`). True when `s` contains an apostrophe that (a) has at
/// least one character before it and (b) is immediately followed by an upper-case
/// character. Byte-offset based rather than a collected `Vec<char>`: `'` is a single-byte
/// ASCII character, so `apo` (a byte offset) is a valid `char` boundary and — since "at
/// least one character precedes the apostrophe" is a zero-vs-nonzero question — the byte
/// offset agrees with Java's UTF-16 code-unit index on that question regardless of what
/// (possibly multi-byte/astral) characters precede it.
fn looks_like_apostrophe_particle(s: &str) -> bool {
    let Some(apo) = s.find('\'') else {
        return false;
    };
    if apo < 1 {
        return false;
    }
    match s[apo + 1..].chars().next() {
        Some(c) => c.is_uppercase(),
        None => false,
    }
}

/// Java `AuthorshipSplit.isAllUpper(String)` (`AuthorshipSplit.java:459-470`) — this class's
/// own copy, distinct from `token.rs`'s private tokenizer-internal helper of the same name:
/// unlike that one, this requires at least one letter to be present (a string with no
/// letters at all, e.g. a bare number, is NOT "all upper" here), matching Java's `any`
/// accumulator exactly.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Rank;
    use crate::token::tokenize;

    fn ctx(requested_rank: Option<Rank>) -> ParseContext {
        ParseContext::new("test".to_string(), None, requested_rank, None)
    }

    fn boundary(input: &str, requested_rank: Option<Rank>) -> usize {
        let tokens = tokenize(input);
        find_boundary(&tokens, &ctx(requested_rank))
    }

    #[test]
    fn simple_binomial_with_no_author_returns_full_length() {
        assert_eq!(boundary("Abies alba", None), 2);
    }

    #[test]
    fn trinomial_with_zoological_author_and_year_returns_boundary_before_author() {
        assert_eq!(boundary("Vulpes vulpes silaceus Miller, 1907", None), 3);
    }

    #[test]
    fn subgenus_with_trailing_epithet_is_kept_in_name_rule1() {
        assert_eq!(boundary("Amnicola (Amnicola) dubrueilliana", None), 5);
    }

    #[test]
    fn subgenus_with_trailing_epithet_and_trailing_author_rule1() {
        assert_eq!(boundary("Amnicola (Amnicola) dubrueilliana Smith", None), 5);
    }

    #[test]
    fn trailing_epithet_overrides_abbreviated_paren_rule1_over_rule2() {
        assert_eq!(boundary("Phalaena (Tin.) guttella Fab.", None), 6);
    }

    #[test]
    fn abbreviated_parenthesised_word_is_basionym_author_rule2() {
        assert_eq!(boundary("Thliphthisa (Griseb.) Caputo", None), 1);
    }

    #[test]
    fn nothing_follows_parens_is_subgenus_rule3() {
        assert_eq!(boundary("Arrhoges (Antarctohoges)", None), 4);
    }

    #[test]
    fn nominotypical_repeat_is_subgenus_rule4() {
        assert_eq!(boundary("Morea (Morea) Link", None), 4);
    }

    #[test]
    fn requested_infrageneric_rank_makes_paren_a_subgenus_rule5() {
        assert_eq!(boundary("Genus (Section) Author", Some(Rank::Subgenus)), 4);
    }

    #[test]
    fn without_requested_infrageneric_rank_same_input_is_basionym_author() {
        assert_eq!(boundary("Genus (Section) Author", None), 1);
    }

    #[test]
    fn year_outside_parens_is_subgenus_zoological_form_rule6() {
        assert_eq!(boundary("Dicromita (Pterodicromita) Fowler, 1925", None), 4);
    }

    #[test]
    fn no_year_no_nominotypical_makes_paren_a_basionym_author_rule7() {
        assert_eq!(boundary("Kyphocarpa (Fenzl) Lopr.", None), 1);
    }

    #[test]
    fn trinomial_with_infraspecific_rank_marker_is_kept_in_name() {
        assert_eq!(boundary("Abies alba subsp. alba", None), 5);
    }

    #[test]
    fn trinomial_with_infraspecific_rank_marker_and_trailing_author() {
        assert_eq!(boundary("Abies alba subsp. alba Mill.", None), 5);
    }

    #[test]
    fn single_letter_abbreviated_genus_is_kept_in_name() {
        assert_eq!(boundary("B. alba", None), 3);
    }

    #[test]
    fn two_to_four_letter_abbreviated_genus_followed_by_lowercase_epithet() {
        assert_eq!(boundary("Mo. bella", None), 3);
    }

    #[test]
    fn two_to_four_letter_genus_not_abbreviated_when_next_word_is_uppercase() {
        // Guards against folding a real binomial like "Mo Bing" into "Mo." + "Bing":
        // the dot is only consumed when the next word is a lowercase epithet, so here
        // the boundary lands right at the dot.
        assert_eq!(boundary("Mo. Bing", None), 1);
    }

    #[test]
    fn mid_name_author_before_infrageneric_marker_is_consumed() {
        assert_eq!(boundary("Centaurea L. subg. Jacea", None), 6);
    }

    #[test]
    fn missing_genus_placeholder_question_mark_is_kept_in_name() {
        assert_eq!(boundary("? gryphoides", None), 2);
    }

    #[test]
    fn empty_token_list_returns_zero() {
        assert_eq!(find_boundary(&[], &ctx(None)), 0);
    }

    #[test]
    fn is_apostrophe_particle_recognises_apostrophe_followed_by_uppercase() {
        assert!(is_apostrophe_particle("d'Urv"));
        assert!(!is_apostrophe_particle("d'urville"));
        assert!(!is_apostrophe_particle("'Urv"));
        assert!(!is_apostrophe_particle("Durv"));
        assert!(!is_apostrophe_particle("Urv'"));
    }

    #[test]
    fn mid_name_author_end_matches_the_centaurea_case() {
        let tokens = tokenize("Centaurea L. subg. Jacea");
        // tokens: [Centaurea(0), L(1), .(2), subg(3), .(4), Jacea(5)]
        assert_eq!(mid_name_author_end(&tokens, 1), Some(3));
    }

    #[test]
    fn particle_triggered_mid_name_author() {
        // "d'Urv." is an apostrophe-particle author followed by an infraspecific rank marker
        // ("subsp."), so the mid-name-author path (line 246-253) consumes both the particle
        // author and the rank marker as part of the name span. The boundary is at the end.
        assert_eq!(boundary("Cirsium creticum d'Urv. subsp. creticum", None), 7);
    }

    #[test]
    fn sp_with_strain_code_stays_in_name() {
        // "sp." + a strain-code-shaped token ("JGP0404") are kept in the name span via the
        // logic at lines 169-185, which recognises the strain code pattern and skips past it
        // as part of the informal phrase, not authorship. The boundary is at the end.
        assert_eq!(boundary("Lepidoptera sp. JGP0404", None), 4);
    }

    #[test]
    fn genus_all_caps_shouted_binomial() {
        // A shouted binomial (all-caps genus + all-caps epithet with no following dot) is kept
        // in the name via the logic at lines 287-296, which recognises that all-caps epithets
        // form part of the name when the genus itself is all-caps. The boundary is at the end.
        assert_eq!(boundary("CHIONE ELEVATA", None), 2);
    }
}
