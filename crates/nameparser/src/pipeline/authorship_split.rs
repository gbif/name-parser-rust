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
//! capability Java's `Token` type exposes as instance methods
//! (`Token.startsUpper()`/`startsLower()`/`startsDigitEpithet()`) but this crate's `Token`
//! type doesn't yet carry as methods itself — see [`starts_upper`], [`starts_lower`] and
//! [`starts_digit_epithet`]'s own doc comments for why they're local free functions here
//! rather than additions to `token.rs` (this task's own brief scopes its file footprint to
//! this file, plus the one-line module declaration in `pipeline/mod.rs`). `Rank`'s own
//! ordinal predicates (`Rank.isInfragenericStrictly()`, ported as
//! [`crate::model::Rank::is_infrageneric_strictly`]) are called directly — Phase 1 Slice 4
//! Task 1 made the full `Rank` model the single source of truth, replacing this file's
//! former ad-hoc `rank_is_infrageneric_strictly` free function.

use crate::model::{NomCode, Rank};
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
                // "ex" before a capitalised author starts an authorship whose ex-author is lost
                // ("Abies alba ex DC.", gbif/name-parser#49), never an epithet; "ex gr." stays a
                // qualifier.
                if t.text == "ex"
                    && tokens
                        .get(i + 1)
                        .is_some_and(|next| next.kind == TokenKind::Word && starts_upper(next))
                {
                    return i;
                }
                // "ex gr." (ex grege) before an epithet is a qualifier like cf.
                if t.text == "ex" && crate::pipeline::name_tokens::is_ex_grege(tokens, i) {
                    i += 2;
                    if i < n && tokens[i].kind == TokenKind::Dot {
                        i += 1;
                    }
                    continue;
                }
                // cf./aff. qualifiers and indet markers — keep walking
                if w.eq_ignore_ascii_case("cf")
                    || w.eq_ignore_ascii_case("aff")
                    || w.eq_ignore_ascii_case("nr")
                    || w.eq_ignore_ascii_case("sp")
                    || w.eq_ignore_ascii_case("spp")
                    || w.eq_ignore_ascii_case("spec")
                    || w.eq_ignore_ascii_case("species")
                    || w.eq_ignore_ascii_case("indet")
                {
                    let is_sp = w.eq_ignore_ascii_case("sp")
                        || w.eq_ignore_ascii_case("spp")
                        || w.eq_ignore_ascii_case("spec");
                    let is_cf_or_aff = w.eq_ignore_ascii_case("cf")
                        || w.eq_ignore_ascii_case("aff")
                        || w.eq_ignore_ascii_case("nr");
                    // `spec` is also a genuine published epithet ("Hemicloeina spec Platnick,
                    // 2002", "Zygonyx spec Dijkstra & Kipping"; COL carries nine). Two signals
                    // mark those: the word carries NO abbreviation dot, and what follows is
                    // shaped like an authorship — a capitalised word or an opening basionym
                    // bracket, NO year required (a botanical "Genus spec Mill." qualifies just as
                    // well as a zoological one). The single next-token test covers both signals:
                    // an abbreviation dot would BE that token (the tokenizer always emits `.`
                    // separately), and it is neither a capitalised word nor an open paren. Leave
                    // the tail to the normal boundary logic below — skipping both the specimen-tag
                    // branch and the swallow-the-tail return — so the authorship parses and
                    // NameTokens's matching `is_published_spec_epithet` guard keeps `spec` as the
                    // epithet. Only `spec`: a bare `sp` is overwhelmingly a dot-less `sp.`.
                    let is_published_spec_epithet = w.eq_ignore_ascii_case("spec")
                        && !have_epithet
                        && i + 1 < n
                        && (tokens[i + 1].kind == TokenKind::OpenParen
                            || (tokens[i + 1].kind == TokenKind::Word
                                && starts_upper(&tokens[i + 1])));
                    i += 1;
                    if i < n && tokens[i].kind == TokenKind::Dot {
                        i += 1;
                    }
                    // 5.0.0 enhancement (deliberately BEYOND Java 4.2.0): after cf./aff. the genus
                    // is often written out again — "Sorex cf. S. shinto", "Microtus cf. Microtus
                    // arvalis" — which is just the longhand of "Sorex cf. shinto". That repetition
                    // used to trip the "capitalised word starts the authorship" boundary below, so
                    // the binomial collapsed to a uninomial with the invented author "S.shinto" and
                    // the epithet was lost. Skip it so the real epithet is reached. Only a genuine
                    // repetition of THIS name's genus qualifies, and only with an epithet behind it
                    // — a different taxon after the qualifier ("Onthophagus cf. Aphodius",
                    // "Veneridae cf. Phacosoma sp") is left exactly as it was.
                    if is_cf_or_aff && !have_epithet {
                        if let Some(after_repeat) = skip_repeated_genus(tokens, i, genus_text) {
                            i = after_repeat;
                            continue;
                        }
                        // Another taxon after the qualifier ("Acroceridae aff. Terphis sp.
                        // SLW-2002", "Anobiidae cf. Theca") is no authorship: with no epithet
                        // the qualifier opens the informal phrase, which runs to the end, as
                        // after "sp." below.
                        if tokens
                            .get(i)
                            .is_some_and(|t| t.kind == TokenKind::Word && starts_upper(t))
                        {
                            return n;
                        }
                    }
                    // A number immediately after an indet marker is the informal
                    // phrase, not authorship.
                    if i < n && tokens[i].kind == TokenKind::Number {
                        i += 1;
                    } else if is_sp
                        && !is_published_spec_epithet
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
                    // 5.0.0 enhancement (deliberately BEYOND Java 4.2.0): a TRUE indet marker
                    // (sp./spec./species/indet — NOT cf./aff., which precede a real species
                    // epithet) with NO species epithet yet is a supraspecific-provisional name;
                    // ANY remaining tokens are its specimen/culture/collection tag — the informal
                    // phrase — never authorship. Consume the whole tail into the name section so
                    // NameTokens.classify captures it as the phrase, rather than the next
                    // capitalised token tripping the "upper-case word → authorship" boundary below
                    // (misreading "Rhizobium sp. RMCC TR1811" as the author "Rmcc Tr1811"). The
                    // single number/strain/letter cases above already advanced i; this catches
                    // multi-token tails. Rescues the ~382k "tag not captured" rows the corpus study
                    // found.
                    //
                    // The tail runs to the end even when it looks like an AUTHORSHIP. Until 5.0.0 a
                    // year-bearing tail was exempted here, on the theory that it "IS an authorship
                    // citation, not a specimen tag" ("Aster sp. Linnaeus, 1753"). The 67.5M verbatim
                    // corpus does not bear that out: of 681 informal names that took the exemption,
                    // 43% produced a demonstrably spurious authorship — impossible years (1002,
                    // 2483, 2951) or "authors" carrying digits, underscores or slashes — and that
                    // undercounts, since collection acronyms like ZRC and MNHN parse as clean
                    // surnames. The exemption also TRUNCATED the tag it declined to capture:
                    // "Rhodococcus sp. 14-2483-1-2" kept only "sp. 14" and invented the year 2483.
                    // An informal name is not fully parsable by definition, so round-tripping beats
                    // structure: capture the tail verbatim and invent nothing. On the genuinely
                    // authored minority the citation is the ANCHOR's authorship anyway — an
                    // undetermined species has none — so a caller who wants it resolves `taxon`.
                    //
                    // The two carve-outs above still stand: cf./aff. precede a real epithet, and the
                    // dot-less-`spec` rescue keeps a published `spec` epithet parsing normally.
                    if !is_cf_or_aff && !have_epithet && !is_published_spec_epithet && i < n {
                        return n;
                    }
                    continue;
                }
                // Aggregate suffix words within the name section
                if w.eq_ignore_ascii_case("agg")
                    || w.eq_ignore_ascii_case("aggregate")
                    || w.eq_ignore_ascii_case("group")
                    || w.eq_ignore_ascii_case("complex")
                    || w.eq_ignore_ascii_case("superspecies")
                {
                    i += 1;
                    if i < n && tokens[i].kind == TokenKind::Dot {
                        i += 1;
                    }
                    continue;
                }
                // Infraspecific rank marker (incl. "notho" prefix variants)
                if rank_markers::match_infraspecific_allow_notho(w).is_some()
                    && (have_epithet || !rank_markers::needs_species_epithet(w))
                {
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
                    // A single letter immediately after a rank marker is an informal infra
                    // epithet ("form A", "f. B", "var. a", "f. (a)"), not the start of
                    // authorship — a lowercase `a` would otherwise open an author particle.
                    if have_epithet && is_capitalised_autonym(tokens, i) {
                        i += 1;
                    } else if let Some(len) = single_letter_designation(tokens, i) {
                        i += len;
                        have_epithet = true;
                    } else if have_epithet && i < n && tokens[i].kind == TokenKind::Number {
                        // gbif/name-parser-rust#16: a NUMBERED indeterminate infraspecific —
                        // "Abies alba var. 3", "Abies alba subsp. 7". The number is the
                        // designation, not authorship; keep it in the name section so
                        // NameTokens can make it the phrase. Exactly the rule the species-level
                        // indet markers above already apply ("A number immediately after an
                        // indet marker is the informal phrase, not authorship") — it was simply
                        // never extended to the infraspecific markers, so the designation
                        // reached the authorship parser and was dropped (a 3-4 digit one became
                        // a bogus year), leaving "Abies alba var." to collide with every other
                        // numbered variety of the species.
                        //
                        // `have_epithet` confines this to a marker that really is trailing a
                        // BINOMIAL. Without a species epithet ("Aquificales str. OlB-6",
                        // "planctomycete str. 394") NameTokens takes its "rank marker before any
                        // lower epithet" branch and makes the marker itself the epithet, so
                        // nothing downstream would consume the designation and it would be
                        // dropped outright — worse than the bogus author it gets today. Those
                        // monomials are left exactly as they were; the 3-token spelling of the
                        // same thing is already handled earlier by
                        // `StripAndStash::stash_trailing_rank_marker_code`.
                        i += 1;
                        // The tokenizer splits a digit+letter designation ("3a" -> NUMBER + WORD);
                        // a lowercase tail at the very end belongs to the designation, not to an
                        // author (an author would be capitalised).
                        if i + 1 == n
                            && tokens[i].kind == TokenKind::Word
                            && starts_lower(&tokens[i])
                        {
                            i += 1;
                        }
                    } else if have_epithet
                        && i + 1 == n
                        && tokens[i].kind == TokenKind::Word
                        && is_strain_code(&tokens[i].text)
                    {
                        // …and the letters-and-digits spelling of the same thing, "var. B12".
                        // `is_strain_code` requires a digit, so a real trailing author
                        // ("Abies alba var. Mill") can never match.
                        i += 1;
                    }
                    continue;
                }
                // Infrageneric rank marker, e.g. "subg." / "nothosect." — consume
                // marker, dot, and the following capitalised epithet.
                if after_genus
                    && !have_epithet
                    && !after_subgenus
                    && rank_markers::match_infrageneric_allow_notho(w).is_some()
                    && !is_epithet_before_dated_author(tokens, i)
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
                // A lone lowercase letter straight after the genus is a provisional species
                // designation (`Collettea a Blazewicz-Paszkowycz & Larsen, 2005`): no
                // species-group name has a single letter (ICZN Art. 11.9.1), and reading `a` as an
                // author particle swallowed it into the authorship. The abbreviated particles
                // `v` (von), `d` (de) and `y` stay particles: `Micropleura v Linstow, 1906`.
                if after_genus
                    && !have_epithet
                    && w.chars().count() == 1
                    && starts_lower(t)
                    && !matches!(w, "v" | "d" | "y")
                    && (i + 1 == n
                        || (tokens[i + 1].kind == TokenKind::Word && starts_upper(&tokens[i + 1])))
                {
                    name_words += 1;
                    have_epithet = true;
                    i += 1;
                    continue;
                }
                let weak_particle = is_weak_particle_before_surname(tokens, i);
                if token::is_particle(&t.text)
                    || looks_like_apostrophe_particle(&t.text)
                    || weak_particle
                {
                    // Particle authors may be followed by a structural rank marker —
                    // try to skip past the author span as a mid-name author so the
                    // marker still gets consumed by the name section.
                    if let Some(after_author) = consume_mid_name_author(tokens, i) {
                        i = after_author;
                        continue;
                    }
                    // A caller-supplied species-or-below rank outranks the particle reading:
                    // the source has asserted this IS a species, so the word sitting in the
                    // epithet slot is the epithet and the rest is the authorship
                    // ("Zodarion van Bosmans, 2009" -> van + Bosmans, 2009).
                    if particle_is_epithet_by_rank(tokens, i, ctx, after_genus, have_epithet) {
                        name_words += 1;
                        have_epithet = true;
                        after_subgenus = false;
                        i += 1;
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
            if after_genus && !have_epithet && is_capitalised_old_epithet(tokens, i, genus_text) {
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
            // ... or between the species epithet and an unmarked infraspecific one ("Loranthus
            // incanus Schumach. & Thonn. sessilis Sprague").
            if have_epithet && name_words == 2 {
                if let Some(epithet) = unmarked_infraspecific_after_author(tokens, i) {
                    i = epithet;
                    continue;
                }
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
                        let epithet_at = |k: usize| {
                            tokens.get(k).is_some_and(|nx| {
                                nx.kind == TokenKind::Word
                                    && starts_lower(nx)
                                    && !token::is_particle(&nx.text)
                            })
                        };
                        // a nothospecies' hybrid sign may stand before it: "Sorbus (Aria) × hybrida"
                        let trailing_is_epithet = epithet_at(after_paren)
                            || (next.is_some_and(|nx| nx.kind == TokenKind::HybridMark)
                                && epithet_at(after_paren + 1))
                            || is_capitalised_old_epithet(tokens, after_paren, genus_text);
                        let nominotypical =
                            genus_text.is_some_and(|g| eq_ignore_case(g, &tokens[j].text));
                        let rank_requests_infragen = ctx
                            .requested_rank
                            .is_some_and(|r| r.is_infrageneric_strictly());
                        // Rust-only: a code or rank hint settles a single bracketed word before an
                        // author, which the shape leaves open ("Humiriastrum (Urban) Cuatrecasas,
                        // 1961" is a genus with its basionym author, or a zoological subgenus with
                        // its author). The zoological code makes it the subgenus — genera have no
                        // basionym authors there — the botanical code, or else a genus-or-higher
                        // rank, the basionym author.
                        let rank_requests_genus = ctx
                            .requested_rank
                            .is_some_and(|r| r == Rank::Genus || r.is_suprageneric());
                        let code = ctx.requested_code;
                        let subgenus = if trailing_is_epithet {
                            true
                        } else if abbreviated {
                            false
                        } else if !has_trailing
                            || nominotypical
                            || rank_requests_infragen
                            || code == Some(NomCode::Zoological)
                        {
                            true
                        } else if code == Some(NomCode::Botanical) || rank_requests_genus {
                            false
                        } else {
                            has_year_token(tokens, after_paren, n)
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
                if name_words == 2 {
                    if let Some(epithet) = unmarked_infraspecific_after_author(tokens, i) {
                        i = epithet;
                        continue;
                    }
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

/// Twin of `NameTokens::is_strain_code` (`name_tokens.rs`), kept as this module's own copy for
/// the same reason its `starts_upper`/`skip_paren_author_block` twins are — see the module doc
/// comment. A designation carrying BOTH a letter and a digit ("B12", "JGP0404"); the digit is what
/// separates it from a trailing author surname.
fn is_strain_code(s: &str) -> bool {
    s.chars().count() >= 3
        && s.chars().any(|c| c.is_alphabetic())
        && s.chars().any(|c| c.is_ascii_digit())
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

/// After a `cf.`/`aff.` qualifier, does `tokens[i]` repeat the name's own genus — spelled out
/// (`"Microtus cf. Microtus arvalis"`) or abbreviated (`"Sorex cf. S. shinto"`) — with a real
/// species epithet behind it? Returns the index just past the repetition (and its abbreviation
/// dot), else `None`.
///
/// The abbreviation test is a case-insensitive PREFIX of the genus, which is what the convention
/// means: `S.` for *Sorex*, `D.` for *Diurodrilus*. Requiring a following lower-case word is what
/// keeps this off `"Onthophagus cf. Aphodius"` (a different genus, nothing behind it) and
/// `"Veneridae cf. Phacosoma sp"` (a family anchor naming a different genus) — neither is a
/// repetition, and neither gains an epithet from being skipped.
pub(crate) fn skip_repeated_genus(
    tokens: &[Token],
    i: usize,
    genus_text: Option<&str>,
) -> Option<usize> {
    let genus = genus_text?;
    let t = tokens.get(i)?;
    if t.kind != TokenKind::Word || !starts_upper(t) {
        return None;
    }
    // The word must be the genus itself or an abbreviated prefix of it.
    if !genus.to_lowercase().starts_with(&t.text.to_lowercase()) {
        return None;
    }
    let mut j = i + 1;
    if j < tokens.len() && tokens[j].kind == TokenKind::Dot {
        j += 1;
    }
    // A real species epithet must follow, else there is nothing to rescue.
    let next = tokens.get(j)?;
    if next.kind == TokenKind::Word && starts_lower(next) && !token::is_particle(&next.text) {
        Some(j)
    } else {
        None
    }
}

/// 5.0.0 enhancement (deliberately BEYOND Java 4.2.0): should the author particle at
/// `tokens[i]` be read as the SPECIES EPITHET instead of the start of the authorship?
///
/// `Genus <particle> Author, year` is genuinely ambiguous — `Allidothrips zur Strassen, 1968`
/// is the genus *Allidothrips* by *zur Strassen*, while `Zodarion van Bosmans, 2009` is the
/// species *Zodarion van* by *Bosmans*. Nothing in the string separates them, so with no rank
/// hint the particle keeps winning (the majority reading: of the 2960 distinct
/// `Genus <particle> …` names in the CoL corpus, `de`/`van`/`von`/`zur` alone account for 2686,
/// nearly all real particled authors). A caller-supplied rank of species-or-below is the one
/// signal that does settle it, and it is authoritative — so here the epithet reading wins.
///
/// This strictly improves that path: before, a `SPECIES` hint on such a name found no epithet,
/// fell into the indetermined branch and dropped the authorship too, losing BOTH `delli` and
/// `Forster, 1968` from `Cantuaria delli Forster, 1968` (the ChecklistBank "unparsable name" +
/// "indetermined" report on dataset 56185, the World Spider Catalog). Guards:
///
///   * only in the epithet slot — after a genus, before any epithet has been seen;
///   * only for real table particles, never an apostrophe particle (`d'Urv.` is an
///     abbreviated surname, never an epithet);
///   * only when what follows looks like the START of an authorship — a capitalised surname, an
///     opening basionym paren, a year, or nothing at all. Another LOWER-case word means we are
///     mid author chain ("van den Boom", "von der Linde", "v. d. Boom"), and an epithet is a
///     single word, so the chain stays one multi-word author. Note this cannot be a
///     particle-table test: "den" is deliberately absent from the table.
fn particle_is_epithet_by_rank(
    tokens: &[Token],
    i: usize,
    ctx: &ParseContext,
    after_genus: bool,
    have_epithet: bool,
) -> bool {
    if !after_genus || have_epithet {
        return false;
    }
    if !ctx.requested_rank.is_some_and(|r| r.is_species_or_below()) {
        return false;
    }
    if !token::is_particle(&tokens[i].text) && !is_weak_particle(&tokens[i].text) {
        return false;
    }
    // Chain guard: look past any abbreviation dots to the next word.
    let mut j = i + 1;
    while j < tokens.len() && tokens[j].kind == TokenKind::Dot {
        j += 1;
    }
    match tokens.get(j) {
        Some(next) if next.kind == TokenKind::Word => !starts_lower(next),
        _ => true,
    }
}

/// "den", "dem" and "ver": particles the table lacks, since they are epithets too ("Agnetina den",
/// "Gnathopleustes den (Barnard 1969)").
fn is_weak_particle(word: &str) -> bool {
    matches!(word, "den" | "dem" | "ver")
}

/// A weak particle at `i` starts an author only right before a capitalised surname ("Metrocoris
/// ciliatus den Boer, 1965") that is not written surname-first with its initials behind a comma —
/// that one follows an epithet ("Agnetina den Cao, T.K.T. & Bae, 2006").
fn is_weak_particle_before_surname(tokens: &[Token], i: usize) -> bool {
    let initials_after_comma = tokens
        .get(i + 2)
        .is_some_and(|c| c.kind == TokenKind::Comma)
        && tokens.get(i + 3).is_some_and(|t| {
            t.kind == TokenKind::Word && t.text.chars().count() == 1 && starts_upper(t)
        })
        && tokens.get(i + 4).is_some_and(|d| d.kind == TokenKind::Dot);
    is_weak_particle(&tokens[i].text)
        && tokens
            .get(i + 1)
            .is_some_and(|nx| nx.kind == TokenKind::Word && starts_upper(nx))
        && !initials_after_comma
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
        if continues_author_span(tokens, j, from) {
            j += 1;
            continue;
        }
        let t = &tokens[j];
        if t.kind == TokenKind::Word {
            let w = strip_dot(&t.text);
            let is_infra_marker = rank_markers::match_infraspecific(w).is_some()
                || rank_markers::match_infraspecific_allow_notho(w).is_some();
            let is_infra_gen_marker = rank_markers::match_infrageneric_allow_notho(w).is_some();
            if (is_infra_marker || is_infra_gen_marker)
                && j > from
                && has_epithet_after_marker(tokens, j, is_infra_gen_marker)
            {
                return Some(j);
            }
        }
        return None;
    }
    None
}

/// Lower-case words that open a note or a citation after an author, never an epithet.
const NOTE_WORDS: &[&str] = &[
    "non",
    "nec",
    "not",
    "auct",
    "auctt",
    "auctorum",
    "sensu",
    "sec",
    "secundum",
    "emend",
    "emd",
    "excl",
    "incl",
    "pro",
    "partim",
    "pars",
    "nom",
    "nomen",
    "comb",
    "stat",
    "nov",
    "fide",
    "teste",
    "vide",
    "ined",
    "sic",
    "corr",
    "orth",
    "err",
    "lapsus",
    "descr",
    "ampl",
    "mut",
    "nud",
    "illeg",
    "inval",
    "cons",
    "rej",
    "superfl",
    "syn",
    "olim",
    "nunc",
    "cit",
    "loc",
    "tab",
    "fig",
    "det",
    "leg",
    "coll",
    "the",
    "for",
    "from",
    "with",
    "was",
    "see",
    "also",
    "emended",
    "nach",
    "after",
    "according",
    "per",
    "vel",
    "aut",
    "seu",
    "sive",
    "variant",
    "race",
    "population",
    "ecotype",
    "hybrid",
    "male",
    "female",
    "generic",
    "nee",
    "neo",
];

/// After the species epithet, a species author starting at `from` — a surname, or a basionym
/// bracket with or without a combination author — followed by an unmarked infraspecific epithet
/// and that epithet's own author: "Loranthus incanus Schumach. & Thonn. sessilis Sprague",
/// "Polypodium pectinatum (L. f.) typica Rosent". Returns the epithet's index. The epithet is a
/// whole lower-case word of three or more letters set off by spaces that no author carries and
/// that opens no note; the species author before it ends on a dot, a bracket or a year, the author
/// after it is a capitalised word or a basionym bracket.
pub(crate) fn unmarked_infraspecific_after_author(tokens: &[Token], from: usize) -> Option<usize> {
    let n = tokens.len();
    // not after a rank marker: there the epithet is still to come ("var. (?) pubescens Benth.")
    let mut before = from;
    while before > 0 && tokens[before - 1].kind == TokenKind::Dot {
        before -= 1;
    }
    if before > 0
        && rank_markers::match_infraspecific_allow_notho(strip_dot(&tokens[before - 1].text))
            .is_some()
    {
        return None;
    }
    let mut j = from;
    match tokens.get(from)?.kind {
        TokenKind::OpenParen => {
            // a basionym bracket holds an author ("(L. f.)"), not "(?)", "(beta)" or "( , 1818)"
            let mut depth = 0i32;
            let mut author = false;
            loop {
                let t = tokens.get(j)?;
                match t.kind {
                    TokenKind::OpenParen => depth += 1,
                    TokenKind::CloseParen => depth -= 1,
                    TokenKind::Word => author |= starts_upper(t),
                    _ => {}
                }
                j += 1;
                if depth == 0 {
                    break;
                }
            }
            // nor the genus repeated as a subgenus ("Janthina janthina (Janthina) janthina")
            let repeated_genus = j == from + 3
                && tokens[from + 1].kind == TokenKind::Word
                && tokens[from + 1].text == tokens[0].text;
            if !author || repeated_genus {
                return None;
            }
        }
        TokenKind::Word if starts_upper(&tokens[from]) => {}
        _ => return None,
    }
    let span_from = j;
    while j < n && continues_author_span(tokens, j, span_from) {
        // a capitalised rank marker is no author ("Eburodacrys mexicana Var. interrupta"); a
        // single capital is an initial ("F.Muell.")
        if tokens[j].kind == TokenKind::Word
            && tokens[j].text.chars().count() > 1
            && rank_markers::match_infraspecific_allow_notho(&tokens[j].text).is_some()
        {
            return None;
        }
        j += 1;
    }
    // the species author ends on an abbreviation dot, its bracket or a year: after a plain
    // capitalised word the lower-case one may be a second name's epithet ("Mesalia zinkeni (Dunker
    // 1851) & Promathildia turritella (Dunker 1851)")
    if j <= from
        || !matches!(
            tokens[j - 1].kind,
            TokenKind::Dot | TokenKind::CloseParen | TokenKind::Number
        )
    {
        return None;
    }
    // after two initials the lower-case word is a surname ("G.B. sowerby II"); a lone capital is
    // an abbreviated author ("Rumex pulcher L. woodsii (De Not.) Arcang.")
    let initial = |k: usize| {
        tokens[k].kind == TokenKind::Word
            && tokens[k].text.chars().count() == 1
            && tokens[k + 1].kind == TokenKind::Dot
    };
    if j >= 4 && initial(j - 2) && initial(j - 4) {
        return None;
    }
    let t = tokens.get(j)?;
    let w = t.text.as_str();
    let spaced =
        tokens[j - 1].end < t.start && tokens.get(j + 1).is_some_and(|nx| t.end < nx.start);
    let epithet = t.kind == TokenKind::Word
        && spaced
        && w.chars().count() >= 3
        && w.chars().all(|c| c.is_alphabetic() && c.is_lowercase())
        && !crate::pipeline::authorship_parser::is_lower_author_word(w)
        && !NOTE_WORDS.contains(&w)
        && rank_markers::match_infraspecific_allow_notho(w).is_none()
        && rank_markers::match_infrageneric_allow_notho(w).is_none()
        && !matches!(
            w,
            "sp" | "spp"
                | "spec"
                | "species"
                | "cf"
                | "aff"
                | "indet"
                | "agg"
                | "group"
                | "complex"
        );
    let author_follows = match tokens.get(j + 1) {
        Some(nx) if nx.kind == TokenKind::Word => starts_upper(nx),
        Some(nx) if nx.kind == TokenKind::OpenParen => tokens
            .get(j + 2)
            .is_some_and(|a| a.kind == TokenKind::Word && starts_upper(a)),
        _ => false,
    };
    (epithet && author_follows).then_some(j)
}

/// The marker word "ser" or "subser" at `i` — real epithets too — is undotted and followed by an
/// author with a year, or by nothing at all: a species epithet, not the botanical rank ("Serina
/// ser Gredler, 1898", "Serina subser Gredler, 1898" — snails; "Serina ser" with its authorship
/// given apart).
pub(crate) fn is_epithet_before_dated_author(tokens: &[Token], i: usize) -> bool {
    if !matches!(tokens[i].text.as_str(), "ser" | "subser") {
        return false;
    }
    if i + 1 == tokens.len() {
        return true;
    }
    let author = tokens
        .get(i + 1)
        .is_some_and(|t| t.kind == TokenKind::Word && starts_upper(t));
    let mut k = i + 2;
    if tokens.get(k).is_some_and(|t| t.kind == TokenKind::Comma) {
        k += 1;
    }
    author
        && tokens
            .get(k)
            .is_some_and(|t| has_year_token(std::slice::from_ref(t), 0, 1))
}

/// The word at `k`, in the species-epithet slot, is an epithet capitalised in the old style:
/// undotted, no repetition of the genus, and followed by an infraspecific rank marker with its
/// epithet ("Aphaenogaster (Ichnomyrmex) Schwammerdami var. spinipes", "Delias Abnormis var.
/// euryxantha Honrath, 1892"). It used to be read as the species author, the species epithet lost.
pub(crate) fn is_capitalised_old_epithet(tokens: &[Token], k: usize, genus: Option<&str>) -> bool {
    let Some(t) = tokens.get(k) else {
        return false;
    };
    let marker_at = k + 1;
    t.kind == TokenKind::Word
        && starts_upper(t)
        && t.text.chars().count() >= 3
        && t.text.chars().skip(1).all(|c| c.is_lowercase())
        && !genus.is_some_and(|g| eq_ignore_case(g, &t.text))
        && tokens.get(marker_at).is_some_and(|m| {
            m.kind == TokenKind::Word
                && m.text != "f"
                && rank_markers::match_infraspecific_allow_notho(strip_dot(&m.text)).is_some()
        })
        && has_epithet_after_marker(tokens, marker_at, false)
}

/// The word at `k` repeats an earlier lower-case epithet with a capital initial: the capitalised
/// final epithet of an autonym ("Phyllanthus tenuicaulis Muell.-Arg. var. Tenuicaulis").
pub(crate) fn is_capitalised_autonym(tokens: &[Token], k: usize) -> bool {
    tokens.get(k).is_some_and(|t| {
        t.kind == TokenKind::Word
            && starts_upper(t)
            && t.text.chars().count() > 1
            && tokens[..k].iter().any(|e| {
                e.kind == TokenKind::Word && starts_lower(e) && eq_ignore_case(&e.text, &t.text)
            })
    })
}

/// Does `tokens[j]` continue an author span that began at `from`? A surname, a particle, the dot of
/// an initial, a separator, an "et al.", an apostrophe (`M'Coy`), and — never as its first token —
/// an "ex", "et" or "and" before the next author ("Nees ex Thwaites", "Pallas ex de Candolle",
/// "Hatus. et Ohwi"), the hyphen of an abbreviated double name ("Baum.-Bod."), a filius right
/// before a rank marker or an "ex" ("Hook.f. var.", "Hook.f. ex A.W.Benn.") and a year
/// ("Günther, 1867 ssp. tanganica"). Shared by the walks that skip a species author standing
/// before a rank marker.
pub(crate) fn continues_author_span(tokens: &[Token], j: usize, from: usize) -> bool {
    let t = &tokens[j];
    // an "ex" may follow the basionym bracket straight away ("(Kütz.) ex Ralfs var. laevis")
    let inner = j > from || (j > 0 && tokens[j - 1].kind == TokenKind::CloseParen);
    match t.kind {
        TokenKind::Word => {
            starts_upper(t)
                || token::is_particle(&t.text)
                || looks_like_apostrophe_particle(&t.text)
                || t.text.eq_ignore_ascii_case("al")
                // the Dutch "'t" ("'t Hart")
                || (t.text == "t"
                    && j > 0
                    && tokens[j - 1].text == "'"
                    && next_word_starts_upper(tokens, j + 1))
                || (inner
                    && matches!(t.text.as_str(), "ex" | "et" | "and")
                    && next_word_starts_author(tokens, j + 1))
                || (inner && t.text == "f" && filius_before_marker_or_ex(tokens, j))
        }
        TokenKind::Dot | TokenKind::Ampersand | TokenKind::Comma => true,
        TokenKind::Other => {
            t.text == "'"
                || (inner
                    && t.text == "-"
                    && tokens[j - 1].kind == TokenKind::Dot
                    && next_word_starts_upper(tokens, j + 1))
        }
        TokenKind::Number => {
            inner
                && t.text.chars().count() == 4
                && matches!(t.text.chars().next(), Some('1') | Some('2'))
        }
        _ => false,
    }
}

/// The word token at `j` (after any dots) starts an author: a capitalised surname, a particle or
/// the "al." of "et al.".
fn next_word_starts_author(tokens: &[Token], mut j: usize) -> bool {
    while tokens.get(j).is_some_and(|t| t.kind == TokenKind::Dot) {
        j += 1;
    }
    tokens.get(j).is_some_and(|t| {
        t.kind == TokenKind::Word
            && (starts_upper(t) || token::is_particle(&t.text) || t.text == "al")
    })
}

/// The word token at `j` (after any dots) starts with an upper-case letter.
fn next_word_starts_upper(tokens: &[Token], mut j: usize) -> bool {
    while tokens.get(j).is_some_and(|t| t.kind == TokenKind::Dot) {
        j += 1;
    }
    tokens
        .get(j)
        .is_some_and(|t| t.kind == TokenKind::Word && starts_upper(t))
}

/// The `f` at `f_idx` is followed (past its dot) by an infraspecific rank marker, an `ex` or another
/// author ("Hook.f. & Wilson var. pusillum", "Hook.f. et Thomson var.") — so it is the filius of
/// the author before it.
fn filius_before_marker_or_ex(tokens: &[Token], f_idx: usize) -> bool {
    let mut k = f_idx + 1;
    if tokens.get(k).is_some_and(|t| t.kind == TokenKind::Dot) {
        k += 1;
    }
    tokens.get(k).is_some_and(|t| match t.kind {
        TokenKind::Ampersand | TokenKind::Comma => true,
        TokenKind::Word => {
            matches!(t.text.as_str(), "ex" | "et" | "and")
                || (t.text != "f"
                    && rank_markers::match_infraspecific_allow_notho(strip_dot(&t.text)).is_some())
        }
        _ => false,
    })
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
    // a hybrid sign before a nothotaxon's epithet ("subsp. ×medium")
    if k < n && tokens[k].kind == TokenKind::HybridMark && k + 1 < n {
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
    // A letter designation ("var. d Lecomte", "var. B Körn.") and a capitalised autonym epithet
    // ("Nasa pteridophylla Weigend & Dostert ssp. Pteridophylla") stand in for the epithet too.
    if single_letter_designation(tokens, k).is_some()
        && (k + 1 == n
            || (tokens[k + 1].kind != TokenKind::Dot && next_word_starts_upper(tokens, k + 1)))
    {
        return true;
    }
    if is_capitalised_autonym(tokens, k) {
        return true;
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
    let from = j;
    while j < n {
        if continues_author_span(tokens, j, from) {
            j += 1;
            continue;
        }
        let t = &tokens[j];
        if t.kind == TokenKind::Word {
            let w = strip_dot(&t.text);
            let is_infra_marker = rank_markers::match_infraspecific_allow_notho(w).is_some();
            if is_infra_marker && has_epithet_after_marker(tokens, j, false) {
                return Some(j);
            }
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

/// A single-letter designation starting at `i` — a bare letter (`A`, `a`) or a parenthesised one
/// (`(a)`) — as its token count, or `None`. Used after an infraspecific rank marker.
pub(crate) fn single_letter_designation(tokens: &[Token], i: usize) -> Option<usize> {
    let is_letter = |t: &Token| {
        // ASCII only: a Greek letter (`var. β`) is the classic name of a variety and stays one
        t.kind == TokenKind::Word && t.text.len() == 1 && t.text.as_bytes()[0].is_ascii_alphabetic()
    };
    if tokens.get(i).is_some_and(is_letter) {
        // Not before a lowercase word: `ab. n. undularia` abbreviates (n. = nova), and
        // `var. b minor` is an informal rank letter ahead of the real epithet.
        let lower_word_at = |j: usize| {
            tokens
                .get(j)
                .is_some_and(|t| t.kind == TokenKind::Word && starts_lower(t))
        };
        let dot_next = tokens.get(i + 1).is_some_and(|t| t.kind == TokenKind::Dot);
        let epithet_follows = lower_word_at(i + 1) || (dot_next && lower_word_at(i + 2));
        return (!epithet_follows).then_some(1);
    }
    if tokens
        .get(i)
        .is_some_and(|t| t.kind == TokenKind::OpenParen)
        && tokens.get(i + 1).is_some_and(is_letter)
        && tokens
            .get(i + 2)
            .is_some_and(|t| t.kind == TokenKind::CloseParen)
    {
        return Some(3);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
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
