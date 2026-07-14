// SPDX-License-Identifier: Apache-2.0

//! Java `org.gbif.nameparser.pipeline.NameTokens` (712 lines) — walks the token range
//! `[0, boundary)` (the "name" span `AuthorshipSplit::find_boundary`, Phase 1 Slice 3 Task
//! 2, has already separated from the trailing authorship) and classifies it into the
//! structural slots of a scientific name: uninomial / genus / subgenus / infrageneric /
//! specific / infraspecific, plus the hybrid-marker (`notho`), open-nomenclature
//! (`epithetQualifier`), and aggregate/indet side signals. Entirely regex-free: a
//! hand-written token-index state machine, mirroring `AuthorshipSplit`'s own style.
//!
//! Three load-bearing quirks, preserved verbatim (see the plan's Global Constraints):
//!
//! 1. **`boundary == 0`** short-circuits immediately: `ctx.name.state = PARTIAL`,
//!    `ctx.name.unparsed = ctx.original`, return — no other field is touched.
//! 2. **The `set_notho`/`add_notho` overwrite asymmetry.** A `HYBRID_MARK` token anywhere
//!    in the name adds to the notho set (`ParsedName::add_notho`, additive, deduped). But
//!    the post-loop `if inline_rank_notho { ctx.name.set_notho(Infraspecific) }` — fired by
//!    a notho-prefixed infraspecific rank marker like "nothovar."/"nvar." — REPLACES the
//!    whole set, silently erasing any earlier `add_notho(Generic)` a leading hybrid mark
//!    ("×Abies alba nothovar. rubra") already recorded. Reproduced exactly: [`set_notho`]
//!    (`model::name::ParsedName::set_notho`) overwrites, [`add_notho`] inserts.
//! 3. **[`skip_paren_author_block`] here is NameTokens's OWN copy**, textually different
//!    from `authorship_split::skip_paren_author_block` (Task 2) of the same name: that one
//!    gates its match on `has_epithet_after_marker` before returning the marker index; this
//!    one returns as soon as it finds ANY infraspecific-marker-shaped word, with no such
//!    follow-up check. The two are intentionally NOT unified — this matches the Java
//!    source, which keeps two separate private methods of the same name in two different
//!    classes (see `authorship_split`'s own module doc for the full rationale).
//!
//! **Adapting Java's `boundary` parameter.** Several Java helpers
//! (`hasInfraspecificEpithetAfter`, `skipParenAuthorBlock`, and the
//! `AuthorshipSplit.midNameAuthorEnd` bridge) take an explicit `int boundary` third
//! argument distinct from `ts.size()` (`ts` is always the FULL token list; `boundary` is
//! the name/author split Task 2 already computed). Rather than threading a redundant
//! `boundary`/`to` parameter through every helper, [`classify`] slices once, up front —
//! `let ts: &[Token] = &ctx.tokens[..boundary];` — and every helper below (including the
//! two `authorship_split` bridges, [`authorship_split::mid_name_author_end`] and
//! [`authorship_split::is_apostrophe_particle`]) is then called with THIS cropped slice,
//! so a plain `tokens.len()` inside each helper reproduces Java's `boundary` bound exactly
//! (slicing from the front never changes any in-range index, so every recorded token index,
//! e.g. `ctx.mid_author_from`/`ctx.mid_author_to`, stays numerically identical to the
//! corresponding index into the untruncated `ctx.tokens`).
//!
//! Ported helpers not among the Java source's own 8 private statics (`render_author_span`,
//! `skip_paren_author_block`, `recover_case`, `is_all_upper_letters`,
//! `has_infraspecific_epithet_after`, `is_strain_code`, `is_all_letter_case`, `strip_dot`):
//! `starts_upper`/`starts_lower`/`starts_digit_epithet` duplicate `authorship_split.rs`'s
//! own identically-named, identically-behaved private bridges (`Token` doesn't carry these
//! as methods yet — see that module's doc comment for why they're local free functions
//! rather than additions to `token.rs`). `Rank.isInfraspecific()`/`Rank.getMarker()` are
//! called directly as [`crate::model::Rank::is_infraspecific`]/
//! [`crate::model::Rank::marker`] — Phase 1 Slice 4 Task 1 made the full `Rank` model the
//! single source of truth, replacing this file's former ad-hoc `rank_is_infraspecific`/
//! `rank_marker` free functions.

use crate::model::{warnings, NamePart, NameType, Rank, State};
use crate::pipeline::authorship_split;
use crate::pipeline::rank_markers;
use crate::pipeline::ParseContext;
use crate::token::{self, Token, TokenKind};

/// Java `NameTokens.AGG_HYPHEN_SUFFIXES` (`NameTokens.java:21`).
const AGG_HYPHEN_SUFFIXES: [&str; 3] = ["-group", "-complex", "-aggregate"];

/// Java `NameTokens.classify(ParseContext, int)` (`NameTokens.java:25-536`). Walks
/// `ctx.tokens[0, boundary)`, classifying it into the structural name-part fields on
/// `ctx.name`, plus `ctx.mid_author_from`/`ctx.mid_author_to` (the species-level author
/// span for a later autonym check) and `ctx.aggregate`. See this module's own doc comment
/// for the 3 load-bearing quirks preserved verbatim.
pub(crate) fn classify(ctx: &mut ParseContext, boundary: usize) {
    if boundary == 0 {
        ctx.name.state = State::Partial;
        ctx.name.unparsed = Some(ctx.original.clone());
        return;
    }

    // See this module's doc comment: slicing once up front lets every helper below use
    // `tokens.len()` as Java's separately-threaded `boundary` parameter.
    let ts: &[Token] = &ctx.tokens[..boundary];

    // True when AuthorshipSplit left NO authorship tail — the name section spans the whole
    // input. Gates the trailing-tag phrase capture below: only flatten a supraspecific indet
    // tail into a phrase when find_boundary already decided the whole string is the name (it
    // returns `n` for a yearless "Genus sp. <tag>"); when it kept an author+year tail out of
    // the name section (`boundary < len`, e.g. "Bacterium sp. (serotype) aboney Dräger 1951"),
    // the in-name tokens are structural (subgenus, epithet) and must parse normally, not flatten.
    let name_section_covers_all = boundary == ctx.tokens.len();

    let mut genus: Option<String> = None;
    let mut subgenus: Option<String> = None;
    let mut infrageneric: Option<Rank> = None;
    let mut infragen_epithet: Option<String> = None;
    let mut lower_epithets: Vec<String> = Vec::with_capacity(4);
    let mut marker_idx_in_epithets: i32 = -1;
    let mut inline_rank: Option<Rank> = None;
    let mut inline_rank_notho = false;
    let mut indet = false;
    // A bare supraspecific indet ("Genus sp." with no distinguishing tail) — stays flagged
    // INDETERMINED even though its phrase now carries the verbatim marker (see the top-check
    // in the indet branch and the warning block near the end of this fn).
    let mut indet_bare = false;
    let mut cf_aff_qualifier: Option<String> = None;
    // Tracks the most recently skipped mid-name author span so that, when a second
    // infraspecific marker overrides the first, we can describe the dropped middle
    // classification ("Intermediate classification removed: subsp.X Author") in a warning.
    let mut pending_mid_name_author: Option<String> = None;

    let mut i = 0usize;
    while i < ts.len() {
        let t = &ts[i];

        if t.kind == TokenKind::HybridMark {
            if genus.is_some() && lower_epithets.is_empty() {
                ctx.name.add_notho(NamePart::Specific);
            } else if genus.is_none() {
                ctx.name.add_notho(NamePart::Generic);
            } else {
                ctx.name.add_notho(NamePart::Infraspecific);
            }
            i += 1;
            continue;
        }

        // "(Word)" -> subgenus, but only before any species epithet. After a species
        // epithet "(Klatt)" is a basionym author span (e.g. the autonym "Trimezia
        // spathata (Klatt) Baker subsp. spathata"), handled by the
        // skip_paren_author_block branch below.
        if t.kind == TokenKind::OpenParen
            && lower_epithets.is_empty()
            && subgenus.is_none()
            && i + 2 < ts.len()
            && ts[i + 1].kind == TokenKind::Word
            && (starts_upper(&ts[i + 1]) || starts_lower(&ts[i + 1]))
            && ts[i + 2].kind == TokenKind::CloseParen
        {
            let raw = ts[i + 1].text.clone();
            // A lower-case parenthesised subgenus ("(acanthoderes)") is malformed —
            // capitalise it to the conventional form and flag the name doubtful.
            let sub = if starts_lower(&ts[i + 1]) {
                ctx.name.doubtful = true;
                let mut chars = raw.chars();
                match chars.next() {
                    Some(c) => {
                        let mut s: String = c.to_uppercase().collect();
                        s.push_str(chars.as_str());
                        s
                    }
                    None => raw,
                }
            } else {
                raw
            };
            subgenus = Some(sub);
            i += 3;
            continue;
        }
        // Abbreviated subgenus: "(Tin.)" / "(G.)" — Title-cased word + DOT inside parens.
        // Only fires before any species epithet, otherwise "(Aubl.)" after a species is a
        // basionym author span and the next block (skip_paren_author_block) handles it.
        if t.kind == TokenKind::OpenParen
            && lower_epithets.is_empty()
            && subgenus.is_none()
            && i + 3 < ts.len()
            && ts[i + 1].kind == TokenKind::Word
            && starts_upper(&ts[i + 1])
            && ts[i + 2].kind == TokenKind::Dot
            && ts[i + 3].kind == TokenKind::CloseParen
        {
            subgenus = Some(format!("{}.", ts[i + 1].text));
            ctx.name.add_warning(warnings::ABBREVIATED_SUBGENUS);
            i += 4;
            continue;
        }
        // After the species epithet, "(BasionymAuth) CombAuth …" sits between species and
        // an infraspecific rank marker. Skip the parens + comb-author span so the rank
        // marker that follows can be classified normally.
        if t.kind == TokenKind::OpenParen && genus.is_some() && !lower_epithets.is_empty() {
            if let Some(after) = skip_paren_author_block(ts, i) {
                // Record the species-level author span [i, after) so the pipeline can
                // attach it to an autonym (ICN Art. 22.1/26.1). Only the first span, right
                // after the species epithet and before any infraspecific marker, is the
                // species author.
                if ctx.mid_author_from < 0
                    && lower_epithets.len() == 1
                    && marker_idx_in_epithets < 0
                {
                    ctx.mid_author_from = i as i32;
                    ctx.mid_author_to = after as i32;
                }
                i = after;
                continue;
            }
        }
        // Abbreviated genus: Title-cased word of 1-4 chars then DOT, only when no genus
        // yet. The single-letter form ("M. alpium") is unambiguous; 2-4 letter forms
        // ("Mo. alpium", "Phl. guttella", "Pseud. dendrobii") are recognised when the next
        // non-DOT token is a lowercase epithet (so an authorship-sequence like
        // "Mo.J.Wong, 1990" doesn't trip).
        if genus.is_none()
            && t.kind == TokenKind::Word
            && (1..=4).contains(&t.text.chars().count())
            && starts_upper(t)
            && i + 1 < ts.len()
            && ts[i + 1].kind == TokenKind::Dot
        {
            let next_is_lower_epithet =
                i + 2 < ts.len() && ts[i + 2].kind == TokenKind::Word && starts_lower(&ts[i + 2]);
            if t.text.chars().count() == 1 || next_is_lower_epithet {
                genus = Some(format!("{}.", t.text));
                ctx.name.type_ = NameType::Informal;
                ctx.name.add_warning(warnings::ABBREVIATED_GENUS);
                i += 2;
                continue;
            }
        }
        // Missing-genus placeholder: "?" as the genus stand-in. Only at the very start.
        if genus.is_none() && t.kind == TokenKind::Other && t.text == "?" {
            genus = Some("?".to_string());
            i += 1;
            continue;
        }
        // Open-nomenclature doubtful-identification marker: "?" between epithets, like
        // "Ferganoconcha? oblonga" or "Buteo borealis ? ventralis". Treat exactly like
        // cf./aff.: skip the token, attach the qualifier to the next epithet (specific when
        // no species yet, infraspecific when one exists), set type=INFORMAL, doubtful, and
        // emit QUESTION_MARKS_REMOVED.
        if genus.is_some()
            && cf_aff_qualifier.is_none()
            && t.kind == TokenKind::Other
            && t.text == "?"
            && lower_epithets.len() < 2
        {
            cf_aff_qualifier = Some("?".to_string());
            ctx.name.type_ = NameType::Informal;
            ctx.name.doubtful = true;
            ctx.name.add_warning(warnings::QUESTION_MARKS_REMOVED);
            i += 1;
            continue;
        }

        if t.kind == TokenKind::Word {
            // Mid-name author span (uppercase Author abbreviations followed by a rank
            // marker, or particle-starting authors like "d'Urv. subsp.") — silently
            // skipped so that downstream classification operates only on the structural
            // tokens.
            let can_start_author = starts_upper(t)
                || (starts_lower(t)
                    && (token::is_particle(&t.text)
                        || authorship_split::is_apostrophe_particle(&t.text)));
            if genus.is_some() && can_start_author {
                if let Some(after) = authorship_split::mid_name_author_end(ts, i) {
                    // Record the species-level author span so the pipeline can attach it
                    // to an autonym (ICN Art. 22.1/26.1): only the first span, sitting
                    // directly after the species epithet and before any infraspecific
                    // marker.
                    if ctx.mid_author_from < 0
                        && lower_epithets.len() == 1
                        && marker_idx_in_epithets < 0
                    {
                        ctx.mid_author_from = i as i32;
                        ctx.mid_author_to = after as i32;
                    }
                    pending_mid_name_author = Some(render_author_span(ts, i, after));
                    i = after;
                    continue;
                }
            }
            if genus.is_none() {
                // For a regular capital-letter genus we keep the input as-is; for
                // all-caps or all-lowercase we recover the canonical form.
                let mut g = recover_case(&t.text, true);
                i += 1;
                // pull abbreviated-genus dot through
                if g.chars().count() == 2 && g.ends_with('.') {
                    // Dead in practice: a WORD token's text can never itself contain a
                    // literal '.' (the tokenizer always splits it into its own DOT
                    // token), so a freshly case-recovered genus can never already end in
                    // a dot. Ported verbatim for branch-for-branch fidelity with
                    // `NameTokens.java:187-188` (its own "already a dot — nothing to do"
                    // comment), not because it's reachable.
                } else if g.chars().count() == 1 && i < ts.len() && ts[i].kind == TokenKind::Dot {
                    g.push('.');
                    ctx.name.type_ = NameType::Informal;
                    i += 1;
                }
                genus = Some(g);
                continue;
            }
            // upper-case epithets (e.g. all-caps "ELEVATA") — treat as lower-case epithets
            // after recovery. A diacritic is not a discriminator: "ELEVÄTA" is lower-cased
            // to an epithet exactly like its ASCII twin "ELEVATA". Only an abbreviation dot
            // ("ELEV." -> "ELEV" + ".") still routes the token to author-recovery, since
            // that shape is an abbreviated author surname.
            if starts_upper(t)
                && genus.is_some()
                && t.text.chars().count() > 1
                && is_all_upper_letters(&t.text)
            {
                let is_abbrev = i + 1 < ts.len() && ts[i + 1].kind == TokenKind::Dot;
                if !is_abbrev {
                    lower_epithets.push(t.text.to_lowercase());
                    i += 1;
                    continue;
                }
                // Otherwise fall through to author-recovery handling below.
            }
            if genus.is_some() && starts_digit_epithet(t) {
                // lower-case like the ordinary-epithet branch below ("11-Punctata" ->
                // "11-punctata")
                lower_epithets.push(t.text.to_lowercase());
                i += 1;
                continue;
            }
            if starts_lower(t) {
                let w = strip_dot(&t.text);
                // 0. Single Greek letter or short ASCII letter between two lowercase
                // epithets ("Agaricus collinitus β mucosus", "Cyphelium disseminatum c
                // subsessile") is an informal infra-rank marker, not an epithet. Skip
                // silently when there's already a species epithet AND another lower-case
                // epithet follows — otherwise the letter IS the epithet (e.g. "var. β").
                if w.chars().count() == 1 && !lower_epithets.is_empty() {
                    let cp = w.chars().next().unwrap() as u32;
                    let is_greek = (0x03B1..=0x03C9).contains(&cp);
                    let is_fungi_ascii =
                        cp == 0x237A || (u32::from(b'a')..=u32::from(b'g')).contains(&cp);
                    if is_greek || is_fungi_ascii {
                        let peek = i + 1;
                        if peek < ts.len()
                            && ts[peek].kind == TokenKind::Word
                            && starts_lower(&ts[peek])
                        {
                            i += 1;
                            continue;
                        }
                    }
                }
                // 1. cf./aff. qualifier
                if (w.eq_ignore_ascii_case("cf") || w.eq_ignore_ascii_case("aff"))
                    && lower_epithets.len() < 2
                {
                    cf_aff_qualifier = Some(format!("{w}."));
                    ctx.name.type_ = NameType::Informal;
                    i += 1;
                    if i < ts.len() && ts[i].kind == TokenKind::Dot {
                        i += 1;
                    }
                    continue;
                }
                // 2. indet markers — sp./spec./indet
                // A number right after the marker (past an optional dot) is a placeholder tag:
                // "Genus sp. 1" (monomial) or "Genus epithet species 12" (after a binomial). Preserve
                // it as an informal phrase rather than reading "species"/"sp" as a (blacklisted)
                // infraspecific epithet and dropping the number — so `species N` round-trips verbatim.
                let number_follows_marker = {
                    let mut j = i + 1;
                    if j < ts.len() && ts[j].kind == TokenKind::Dot {
                        j += 1;
                    }
                    j < ts.len() && ts[j].kind == TokenKind::Number
                };
                if (w.eq_ignore_ascii_case("sp")
                    || w.eq_ignore_ascii_case("spec")
                    || w.eq_ignore_ascii_case("species")
                    || w.eq_ignore_ascii_case("indet"))
                    && (lower_epithets.is_empty()
                        || marker_idx_in_epithets >= 0
                        || number_follows_marker)
                {
                    let marker_start = ts[i].start;
                    indet = true;
                    ctx.name.type_ = NameType::Informal;
                    i += 1;
                    if i < ts.len() && ts[i].kind == TokenKind::Dot {
                        i += 1;
                    }
                    // 5.0.0 Informal-phrase contract: a supraspecific indet (no species epithet yet)
                    // keeps the WHOLE verbatim tail FROM the marker as its phrase — marker included,
                    // original spelling preserved — so it round-trips exactly and the formatter's
                    // `phrase_leads_with_species_marker` guard emits it as-is instead of re-synthesising
                    // "sp." from the rank. With a trailing designation that is "sp. RMCC TR1811" /
                    // "species 1" / "spec. 3"; a bare "Genus sp." captures just the marker itself
                    // ("sp."), so every Informal name's phrase is uniformly the printable text after
                    // the anchor taxon (no None special-case, no synth-from-rank). `ts[ts.len() - 1]`
                    // is the last consumed token — the trailing tag when present, else the marker/dot.
                    // The marker-stripping sub-branches below now serve only the binomial indet case (a
                    // species epithet is already present, so it stays Parsed, not Informal).
                    // Only the species-level markers (sp/spec/species) are round-tripped by the
                    // formatter's `phrase_leads_with_species_marker` guard; a fully-generic "indet"
                    // is NOT, so keeping it in the phrase would double under the synthesised "sp."
                    // ("Aster indet." -> "Aster sp. indet."). Restrict the marker capture to
                    // sp/spec/species; "indet" falls through and renders via the synthesised rank.
                    let is_species_marker = w.eq_ignore_ascii_case("sp")
                        || w.eq_ignore_ascii_case("spec")
                        || w.eq_ignore_ascii_case("species");
                    // A cultivar epithet (extracted upstream from "Genus sp. cv. 'Name'") is the
                    // operative designation, not the "sp." — so it is NOT an informal indet and the
                    // marker must not be captured as a phrase (assemble step 17 clears INFORMAL for
                    // it); fall through and leave the phrase to the cultivar rendering. Likewise, an
                    // already-set phrase means an upstream step (the voucher `stash_phrase_name`,
                    // which rewrites working to a bare "Genus sp." while stashing the full "sp. <voucher>"
                    // on the phrase) owns it — don't clobber it with the bare marker here.
                    if lower_epithets.is_empty()
                        && ctx.name.cultivar_epithet.is_none()
                        && ctx.name.phrase.is_none()
                        && is_species_marker
                    {
                        // A distinguishing tail after the marker (a specimen tag, number, voucher)
                        // makes the name determinable; a bare "Genus sp." does not, so it stays
                        // flagged INDETERMINED below even though its phrase carries the marker.
                        indet_bare = i >= ts.len();
                        ctx.name.phrase =
                            Some(ctx.working[marker_start..ts[ts.len() - 1].end].to_string());
                        i = ts.len();
                        continue;
                    }
                    // A number immediately following the indet marker becomes the
                    // informal phrase. When the source spelled out the marker as the
                    // full word "species" we keep it verbatim in the phrase ("Allium
                    // species 1" -> phrase "species 1") rather than collapsing it to the
                    // synthetic "sp." marker; the formatter then renders the phrase
                    // as-is. Abbreviated "sp."/"spec." keep the number-only phrase.
                    if i < ts.len() && ts[i].kind == TokenKind::Number {
                        let number = ts[i].text.clone();
                        i += 1;
                        // Rule: anything after "(sp|spec|species) N" belongs to the phrase — once a
                        // phrase starts it runs to the end of the input. So when tokens follow the
                        // number, capture the whole VERBATIM tail from the marker ("Dichanthelium
                        // species 12 (=chrysopsidifolium)" -> phrase "species 12 (=chrysopsidifolium)")
                        // rather than leaving "(=chrysopsidifolium)" to be misread as a subgenus /
                        // epithet. `ctx.working` is the source string (token offsets index into it), so
                        // the slice keeps the original spacing.
                        if i < ts.len() {
                            ctx.name.phrase =
                                Some(ctx.working[marker_start..ts[ts.len() - 1].end].to_string());
                            i = ts.len();
                        } else if w.eq_ignore_ascii_case("species") {
                            ctx.name.phrase = Some(format!("species {number}"));
                        } else {
                            ctx.name.phrase = Some(number);
                        }
                    } else if i < ts.len()
                        && ts[i].kind == TokenKind::Word
                        && ts[i].text.chars().count() == 1
                        && starts_upper(&ts[i])
                        && i + 1 == ts.len()
                    {
                        // Single uppercase letter following sp. — informal phrase
                        // identifier ("Bryozoan sp. E"). Stored as phrase, leaves
                        // indet=true.
                        ctx.name.phrase = Some(ts[i].text.clone());
                        i += 1;
                    } else if i < ts.len()
                        && ts[i].kind == TokenKind::Word
                        && ts[i].text.chars().count() >= 2
                        && (i + 1 == ts.len()
                            || (i + 1 < ts.len()
                                && ts[i + 1].kind == TokenKind::Number
                                && i + 2 == ts.len()))
                    {
                        // "Genus sp. JYr4" / "Genus sp. JGP0404" — strain code
                        // immediately after the marker with nothing else following.
                        // Capture as informal phrase (species stays indet). Allow WORD
                        // or WORD+NUMBER (when the tokenizer splits a mixed
                        // letters-and-digits code like "JGP" + "0404").
                        let mut code = ts[i].text.clone();
                        i += 1;
                        if i < ts.len() && ts[i].kind == TokenKind::Number {
                            code.push_str(&ts[i].text);
                            i += 1;
                        }
                        if is_strain_code(&code) {
                            ctx.name.phrase = Some(code);
                        }
                    }
                    // 5.0.0 enhancement (deliberately BEYOND Java 4.2.0): a supraspecific indet
                    // marker followed by a trailing tag the narrow branches above did not capture —
                    // a multi-token specimen/culture/BOLD code like "Rhizobium sp. RMCC TR1811" or
                    // "Ichneumonidae sp. UAM Ento 145060" — captures the whole VERBATIM tail as the
                    // informal phrase instead of silently dropping it token-by-token at the
                    // "upper word … skip" fallthrough below. Fires only for the pure
                    // supraspecific-provisional band (no species epithet, so it stays out of the
                    // binomial-core names that remain Parsed) and only when nothing was captured
                    // above. Rescues the ~382k "tag not captured" rows the 67.5M verbatim-corpus
                    // study found (`docs/superpowers/findings/`). Token offsets index into
                    // `ctx.working` (`ctx.tokens = tokenize(&ctx.working)`, unmodified until here),
                    // so the slice is the exact source substring with its original spacing.
                    if name_section_covers_all
                        && ctx.name.phrase.is_none()
                        && lower_epithets.is_empty()
                        && i < ts.len()
                    {
                        let start = ts[i].start;
                        let end = ts[ts.len() - 1].end;
                        ctx.name.phrase = Some(ctx.working[start..end].to_string());
                        i = ts.len();
                    }
                    continue;
                }
                // 2b. "sp." between species and infraspecific epithet is almost always a
                // misspelling of "ssp." (subspecies). Only triggers when there's already
                // a species epithet and a lower epithet follows.
                if w.eq_ignore_ascii_case("sp")
                    && lower_epithets.len() == 1
                    && marker_idx_in_epithets < 0
                    && has_infraspecific_epithet_after(ts, i)
                {
                    inline_rank = Some(Rank::Subspecies);
                    marker_idx_in_epithets = lower_epithets.len() as i32;
                    i += 1;
                    if i < ts.len() && ts[i].kind == TokenKind::Dot {
                        i += 1;
                    }
                    continue;
                }
                // 2c. A bare trailing "sp."/"spec."/"species" after a species epithet,
                // with nothing following, is a redundant leftover marker — drop it and
                // keep the binomial at SPECIES rather than reading "sp" as an
                // infraspecific epithet (which yielded INFRASPECIFIC_NAME with epithet
                // "sp"). The sp.->ssp. case (2b) already handled a following epithet.
                if (w.eq_ignore_ascii_case("sp")
                    || w.eq_ignore_ascii_case("spec")
                    || w.eq_ignore_ascii_case("species"))
                    && !lower_epithets.is_empty()
                    && marker_idx_in_epithets < 0
                {
                    let mut j = i + 1;
                    if j < ts.len() && ts[j].kind == TokenKind::Dot {
                        j += 1;
                    }
                    if j >= ts.len() {
                        i = j;
                        continue;
                    }
                }
                // 3. infraspecific rank marker (with notho-prefix support)
                if let Some((rm_infra, notho_flag)) =
                    rank_markers::match_infraspecific_allow_notho(w)
                {
                    if has_infraspecific_epithet_after(ts, i) {
                        // Second marker overriding the first: the previous
                        // classification (oldRank.epithet + author) was an intermediate
                        // level the model can't hold, so warn about the drop.
                        if let Some(old_rank) = inline_rank.filter(|_| {
                            marker_idx_in_epithets >= 0
                                && (marker_idx_in_epithets as usize) < lower_epithets.len()
                        }) {
                            let old_marker = old_rank.marker();
                            let old_epithet = &lower_epithets[marker_idx_in_epithets as usize];
                            let mut sb = String::from(warnings::REMOVED_PREFIX);
                            if let Some(m) = old_marker {
                                sb.push_str(m);
                                sb.push(' ');
                            }
                            sb.push_str(old_epithet);
                            if let Some(pma) = &pending_mid_name_author {
                                if !pma.is_empty() {
                                    sb.push(' ');
                                    sb.push_str(pma);
                                }
                            }
                            ctx.name.add_warning(&sb);
                            ctx.name.add_warning(warnings::QUADRINOMIAL);
                        }
                        inline_rank = Some(rm_infra);
                        inline_rank_notho = notho_flag;
                        marker_idx_in_epithets = lower_epithets.len() as i32;
                        pending_mid_name_author = None;
                        i += 1;
                        if i < ts.len() && ts[i].kind == TokenKind::Dot {
                            i += 1;
                        }
                        // microbial "f. sp." -> forma specialis: skip the "sp" + dot too
                        if i + 1 < ts.len()
                            && ts[i].kind == TokenKind::Word
                            && ts[i].text.eq_ignore_ascii_case("sp")
                        {
                            inline_rank = Some(Rank::FormaSpecialis);
                            i += 1;
                            if i < ts.len() && ts[i].kind == TokenKind::Dot {
                                i += 1;
                            }
                        }
                        // Informal infra epithet: a single uppercase letter / digit
                        // immediately following the rank marker ("form A", "f. B") —
                        // consume it here so the normal lowercase-epithet path doesn't
                        // drop it as an "upper word".
                        if i < ts.len()
                            && ts[i].kind == TokenKind::Word
                            && ts[i].text.chars().count() == 1
                            && starts_upper(&ts[i])
                        {
                            lower_epithets.push(ts[i].text.clone());
                            indet = true; // INFORMAL informal infra epithet
                            i += 1;
                        }
                    } else if !lower_epithets.is_empty() {
                        // Trailing rank marker with no following epithet = indetermined
                        // infraspecific
                        indet = true;
                        inline_rank = Some(rm_infra);
                        inline_rank_notho = notho_flag;
                        marker_idx_in_epithets = lower_epithets.len() as i32;
                        i += 1;
                        if i < ts.len() && ts[i].kind == TokenKind::Dot {
                            i += 1;
                        }
                    } else {
                        // Rank marker before any lower epithet and no following epithet:
                        // treat it as the specific epithet (e.g. "Foa fo" — "fo" is the
                        // species name, not a forma rank indicator).
                        lower_epithets.push(w.to_string());
                        i += 1;
                        if i < ts.len() && ts[i].kind == TokenKind::Dot {
                            i += 1;
                        }
                    }
                    continue;
                }
                // 4. infrageneric marker (with optional "notho-" prefix)
                if let Some((rm_infragen, gnotho)) = rank_markers::match_infrageneric_allow_notho(w)
                {
                    if lower_epithets.is_empty() && subgenus.is_none() && infragen_epithet.is_none()
                    {
                        i += 1;
                        if i < ts.len() && ts[i].kind == TokenKind::Dot {
                            i += 1;
                        }
                        if i < ts.len() && ts[i].kind == TokenKind::Word && starts_upper(&ts[i]) {
                            infragen_epithet = Some(ts[i].text.clone());
                            infrageneric = Some(rm_infragen);
                            if gnotho {
                                ctx.name.add_notho(NamePart::Infrageneric);
                            }
                            i += 1;
                        }
                        continue;
                    }
                }
                // 5. hyphenated aggregate suffix: "X-group"
                {
                    let low = t.text.to_lowercase();
                    let mut stripped: Option<String> = None;
                    for suf in AGG_HYPHEN_SUFFIXES {
                        let char_count = t.text.chars().count();
                        let suf_chars = suf.chars().count();
                        if low.ends_with(suf) && char_count > suf_chars {
                            stripped = Some(t.text.chars().take(char_count - suf_chars).collect());
                            break;
                        }
                    }
                    if let Some(s) = stripped {
                        ctx.aggregate = true;
                        lower_epithets.push(s);
                        i += 1;
                        continue;
                    }
                }
                // 6. standalone aggregate marker after a species epithet
                if !lower_epithets.is_empty()
                    && (w.eq_ignore_ascii_case("agg")
                        || w.eq_ignore_ascii_case("aggregate")
                        || w.eq_ignore_ascii_case("group")
                        || w.eq_ignore_ascii_case("complex"))
                {
                    ctx.aggregate = true;
                    i += 1;
                    if i < ts.len() && ts[i].kind == TokenKind::Dot {
                        i += 1;
                    }
                    continue;
                }
                // 7. "species group" two-word marker
                if !lower_epithets.is_empty()
                    && w.eq_ignore_ascii_case("species")
                    && i + 1 < ts.len()
                    && ts[i + 1].kind == TokenKind::Word
                    && ts[i + 1].text.eq_ignore_ascii_case("group")
                {
                    ctx.aggregate = true;
                    i += 2;
                    continue;
                }
                // 8. ordinary epithet
                lower_epithets.push(t.text.clone());
                i += 1;
                continue;
            }
            // upper word inside name section: skip
            i += 1;
            continue;
        }
        i += 1;
    }

    let mut specific: Option<String> = None;
    let mut infraspecific: Option<String> = None;
    let mut rank: Option<Rank> = None;

    if marker_idx_in_epithets >= 0 {
        let midx = marker_idx_in_epithets as usize;
        if marker_idx_in_epithets >= 1 {
            specific = Some(lower_epithets[0].clone());
        }
        if midx < lower_epithets.len() {
            infraspecific = Some(lower_epithets[midx].clone());
        }
        rank = inline_rank;
        if lower_epithets.len() as i32 > marker_idx_in_epithets + 1 {
            let mut rem = String::new();
            for epithet in &lower_epithets[(midx + 1)..] {
                if !rem.is_empty() {
                    rem.push(' ');
                }
                rem.push_str(epithet);
            }
            ctx.name.state = State::Partial;
            ctx.name.unparsed = Some(rem);
        }
    } else if !lower_epithets.is_empty() {
        specific = Some(lower_epithets[0].clone());
        if lower_epithets.len() >= 2 {
            infraspecific = Some(lower_epithets[lower_epithets.len() - 1].clone());
            rank = Some(if lower_epithets.len() == 2 {
                Rank::InfraspecificName
            } else {
                Rank::InfrasubspecificName
            });
        } else {
            rank = Some(Rank::Species);
        }
    }

    let treat_as_genus = ctx.name.cultivar_epithet.is_some()
        || (indet && lower_epithets.is_empty() && genus.is_some());
    if !treat_as_genus
        && genus.is_some()
        && specific.is_none()
        && subgenus.is_none()
        && infragen_epithet.is_none()
    {
        ctx.name.uninomial = genus.clone();
    } else {
        ctx.name.genus = genus.clone();
        if let Some(sg) = &subgenus {
            ctx.name.infrageneric_epithet = Some(sg.clone());
        }
        if let Some(ie) = &infragen_epithet {
            ctx.name.infrageneric_epithet = Some(ie.clone());
        }
        if let Some(sp) = &specific {
            ctx.name.specific_epithet = Some(sp.clone());
        }
        if let Some(isp) = &infraspecific {
            ctx.name.infraspecific_epithet = Some(isp.clone());
        }
    }

    let requested = ctx.requested_rank;
    let cultivar_set = ctx.name.cultivar_epithet.is_some();
    // `.filter(|_| !cultivar_set)` preserves the Java `X != null && !cultivarSet` compound
    // condition exactly (as opposed to nesting `if !cultivar_set` INSIDE `if let Some(x) =
    // X`, which would wrongly skip the next `else if` whenever `X` is `Some` but
    // `cultivar_set` is true, rather than falling through to test it like Java's flat
    // `else if` chain does).
    if let Some(r) = rank.filter(|_| !cultivar_set) {
        ctx.name.rank = r;
    } else if let Some(ig) = infrageneric.filter(|_| !cultivar_set) {
        ctx.name.rank = ig;
    } else if subgenus.is_some()
        && !cultivar_set
        && specific.is_none()
        && infraspecific.is_none()
        && ctx.name.rank == Rank::Unranked
    {
        // Paren-based subgenus ("Calathus (Lindrothius)") without an explicit rank
        // marker — default to the generic infrageneric rank.
        ctx.name.rank = Rank::InfragenericName;
    }

    if inline_rank_notho {
        ctx.name.set_notho(NamePart::Infraspecific);
    }
    if let Some(q) = &cf_aff_qualifier {
        // Qualifier applies to the specific or infraspecific epithet, whichever is the
        // most-specific present.
        let part = if infraspecific.is_some() {
            NamePart::Infraspecific
        } else {
            NamePart::Specific
        };
        ctx.name.set_epithet_qualifier(part, q);
    }
    if indet {
        ctx.name.type_ = NameType::Informal;
        if infraspecific.is_none() {
            if lower_epithets.is_empty() && ctx.name.rank == Rank::Unranked {
                ctx.name.rank = Rank::Species;
            }
            if ctx.name.phrase.is_none() || indet_bare {
                ctx.name.add_warning(warnings::INDETERMINED);
            }
        }
    }
    // Caller-supplied infraspecific rank on a binomial with no infraspecific epithet ->
    // treat as indeterminate infraspecific (e.g. "Lepidoptera alba DC." + SUBSPECIES)
    if !indet
        && requested.is_some_and(|r| r != Rank::Unranked && r.is_infraspecific())
        && specific.is_some()
        && infraspecific.is_none()
    {
        ctx.name.type_ = NameType::Informal;
        ctx.name.rank = requested.unwrap();
        ctx.name.add_warning(warnings::INDETERMINED);
    }
    if requested == Some(Rank::Species) && infraspecific.is_some() {
        ctx.name.add_warning(warnings::SUBSPECIES_ASSIGNED);
    }
}

/// Java `Token.startsUpper()` (`Token.java:20-22`). Duplicates
/// `authorship_split::starts_upper` (private there) rather than importing it — see this
/// module's own doc comment for why (`Token` doesn't carry this as a method yet, and each
/// class's own copy mirrors the Java source, which also has no shared free-function form).
fn starts_upper(t: &Token) -> bool {
    t.text.chars().next().is_some_and(|c| c.is_uppercase())
}

/// Java `Token.startsLower()` (`Token.java:24-26`). See [`starts_upper`]'s doc comment.
fn starts_lower(t: &Token) -> bool {
    t.text.chars().next().is_some_and(|c| c.is_lowercase())
}

/// Java `Token.startsDigitEpithet()` (`Token.java:29-33`). See [`starts_upper`]'s doc
/// comment; same ASCII-only digit approximation as `authorship_split::starts_digit_epithet`
/// and `token.rs`'s own tokenizer-internal `is_digit`.
fn starts_digit_epithet(t: &Token) -> bool {
    t.kind == TokenKind::Word
        && t.text.chars().next().is_some_and(|c| c.is_ascii_digit())
        && t.text.chars().any(|c| c.is_alphabetic())
}

/// Renders the WORD/DOT tokens in `[from, to)` into the canonical inline author form ("B.
/// Boivin" -> "B.Boivin"). Java `NameTokens.renderAuthorSpan(List<Token>, int, int)`
/// (`NameTokens.java:543-555`). Used to record the dropped author span when an intermediate
/// classification is overridden by a later rank marker.
fn render_author_span(ts: &[Token], from: usize, to: usize) -> String {
    let mut sb = String::new();
    for t in &ts[from..to] {
        match t.kind {
            TokenKind::Word => {
                if !sb.is_empty() && !sb.ends_with('.') {
                    sb.push(' ');
                }
                sb.push_str(&t.text);
            }
            TokenKind::Dot if !sb.is_empty() && !sb.ends_with('.') => {
                sb.push('.');
            }
            _ => {}
        }
    }
    sb
}

/// Java `NameTokens.skipParenAuthorBlock(List<Token>, int, int)` (`NameTokens.java:561-594`)
/// — **NameTokens's OWN copy**, textually different from
/// `authorship_split::skip_paren_author_block` of the same name: this one returns as soon
/// as it finds a matching infraspecific-marker word, with NO `hasEpithetAfterMarker`-style
/// follow-up check — the two copies are intentionally NOT unified (see this module's own
/// doc comment, and `authorship_split`'s, for the full rationale). If a "(...) Author"
/// span sits between the species epithet and an infraspecific rank marker, returns the
/// index of the rank-marker token; otherwise `None`.
fn skip_paren_author_block(ts: &[Token], open_idx: usize) -> Option<usize> {
    let n = ts.len();
    let mut depth = 1i32;
    let mut j = open_idx + 1;
    while j < n && depth > 0 {
        let k = ts[j].kind;
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
    while j < n {
        let t = &ts[j];
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
            if rank_markers::match_infraspecific_allow_notho(w).is_some() {
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

/// Title-cases all-caps or all-lower-case Latin words used as a genus/uninomial. Java
/// `NameTokens.recoverCase(ParseContext, String, boolean)` (`NameTokens.java:597-654`). The
/// Java signature's `ParseContext ctx` parameter is never actually read in the method body
/// (verified against the Java source — every branch touches only `text`/`isGenus`), so
/// it's dropped here rather than threaded through as an unused parameter.
///
/// `Character.toUpperCase`/`toLowerCase` (single-codepoint, Java's SIMPLE case mapping)
/// vs. Rust's `char::to_uppercase`/`to_lowercase` (FULL Unicode case mapping, which for a
/// handful of characters — e.g. German 'ß' -> "SS" — yields more than one output char):
/// genus/epithet text is near-universally Latin/ASCII, so this divergence is not expected
/// to be exercised by the corpus; any case where it is would surface as a golden-corpus
/// mismatch (same acceptable-divergence rationale already documented elsewhere in this
/// port, e.g. `token.rs`'s `is_digit`).
fn recover_case(text: &str, is_genus: bool) -> String {
    if text.chars().count() < 2 {
        return text.to_string();
    }

    // For hyphenated genera, normalize subsequent segments to lowercase under specific
    // conditions:
    // - first segment is <= 3 chars (short prefix like "Eu-", "Le-", "Uva-"), OR
    // - name has 3+ hyphenated segments (e.g. "Prunus-Lauro-Cerasus")
    // This does NOT apply when subsequent segments already start lowercase.
    if is_genus && text.contains('-') {
        let parts: Vec<&str> = text.split('-').collect();
        if parts.len() >= 2 {
            let needs_norm = parts[1..]
                .iter()
                .any(|p| p.chars().next().is_some_and(|c| c.is_uppercase()));
            if needs_norm && (parts[0].chars().count() <= 3 || parts.len() >= 3) {
                let mut sb = String::from(parts[0]);
                for p in &parts[1..] {
                    sb.push('-');
                    let mut chars = p.chars();
                    match chars.next() {
                        Some(c) if c.is_uppercase() => {
                            let lc: String = c.to_lowercase().collect();
                            sb.push_str(&lc);
                            sb.push_str(chars.as_str());
                        }
                        _ => sb.push_str(p),
                    }
                }
                return sb;
            }
        }
    }

    let upper = is_all_letter_case(text, true);
    let lower = !upper && is_all_letter_case(text, false);
    if !upper && !lower {
        return text.to_string();
    }
    let mut b = String::with_capacity(text.len());
    let mut first = true;
    for c in text.chars() {
        if c.is_alphabetic() {
            if first {
                let uc: String = c.to_uppercase().collect();
                b.push_str(&uc);
                first = false;
            } else {
                let lc: String = c.to_lowercase().collect();
                b.push_str(&lc);
            }
        } else {
            b.push(c);
            first = true;
        }
    }
    b
}

/// Java `NameTokens.isAllUpperLetters(String)` (`NameTokens.java:656-658`) — thin delegate
/// to [`is_all_letter_case`].
fn is_all_upper_letters(s: &str) -> bool {
    is_all_letter_case(s, true)
}

/// True if the marker at `marker_idx` is followed by a recognisable infraspecific epithet —
/// either a lowercase word, or a single uppercase letter (informal collector tag like "f.
/// A" / "form A"). Java `NameTokens.hasInfraspecificEpithetAfter(List<Token>, int, int)`
/// (`NameTokens.java:665-675`).
fn has_infraspecific_epithet_after(ts: &[Token], marker_idx: usize) -> bool {
    let n = ts.len();
    let mut k = marker_idx + 1;
    if k < n && ts[k].kind == TokenKind::Dot {
        k += 1;
    }
    // tolerate an interleaving hybrid mark before the epithet ("var. ×alpina")
    if k < n && ts[k].kind == TokenKind::HybridMark {
        k += 1;
    }
    if k >= n {
        return false;
    }
    let nx = &ts[k];
    if nx.kind == TokenKind::Word && starts_lower(nx) {
        return true;
    }
    if nx.kind == TokenKind::Word && nx.text.chars().count() == 1 && starts_upper(nx) {
        return true;
    }
    false
}

/// True for strain-code-shaped tokens — mixed letters and digits, no spaces, length >= 3
/// (e.g. "JGP0404", "ANIC_3", "Sb1"). Used to glue the code onto an "sp." indet marker so
/// the species epithet rendering keeps the strain identifier. Java
/// `NameTokens.isStrainCode(String)` (`NameTokens.java:682-693`).
fn is_strain_code(s: &str) -> bool {
    if s.chars().count() < 3 {
        return false;
    }
    let mut has_letter = false;
    let mut has_digit = false;
    for c in s.chars() {
        if c.is_alphabetic() {
            has_letter = true;
        } else if c.is_ascii_digit() {
            // Java's Character.isDigit is Unicode Nd; token.rs's own is_digit already
            // documents this crate's ASCII-only approximation for Character.isDigit —
            // matched here for consistency.
            has_digit = true;
        }
    }
    has_letter && has_digit
}

/// Java `NameTokens.isAllLetterCase(String, boolean)` (`NameTokens.java:695-707`).
fn is_all_letter_case(s: &str, upper: bool) -> bool {
    let mut have_letter = false;
    for c in s.chars() {
        if c.is_alphabetic() {
            have_letter = true;
            if upper && !c.is_uppercase() {
                return false;
            }
            if !upper && !c.is_lowercase() {
                return false;
            }
        }
    }
    have_letter
}

/// Java `NameTokens.stripDot(String)` (`NameTokens.java:709-711`) — NameTokens's OWN copy
/// (identical body to `authorship_split::strip_dot`, matching Java's own two separate
/// private statics of the same name).
fn strip_dot(s: &str) -> &str {
    s.strip_suffix('.').unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::NomCode;
    use crate::token::tokenize;

    /// Runs the real pipeline stages `classify` depends on (tokenize + find_boundary) and
    /// then `classify` itself, returning the resulting `ParseContext` for inspection of
    /// both `ctx.name` and the `ctx` side-channels (`mid_author_from`/`aggregate`).
    fn run(input: &str, requested_rank: Option<Rank>) -> ParseContext {
        let mut ctx = ParseContext::new(input.to_string(), None, requested_rank, None);
        ctx.tokens = tokenize(input);
        let boundary = authorship_split::find_boundary(&ctx.tokens, &ctx);
        classify(&mut ctx, boundary);
        ctx
    }

    #[test]
    fn boundary_zero_short_circuits_to_partial_unparsed_and_touches_nothing_else() {
        // "a" alone (a single lowercase letter) makes find_boundary return 0 (Java:
        // AuthorshipSplit's own first-token handling only accepts a lower-case first
        // token when it's 2+ chars long). Testing this in isolation with `boundary`
        // passed explicitly (rather than relying on find_boundary to produce 0) is the
        // most direct way to exercise NameTokens's own early-return branch.
        let mut ctx = ParseContext::new("a".to_string(), None, None, None);
        ctx.tokens = tokenize("a");
        classify(&mut ctx, 0);
        assert_eq!(ctx.name.state, State::Partial);
        assert_eq!(ctx.name.unparsed, Some("a".to_string()));
        assert_eq!(ctx.name.genus, None);
        assert_eq!(ctx.name.uninomial, None);
        assert!(ctx.name.warnings.is_empty());
    }

    #[test]
    fn bare_uninomial_routes_to_uninomial_not_genus() {
        let ctx = run("Abies", None);
        assert_eq!(ctx.name.uninomial, Some("Abies".to_string()));
        assert_eq!(ctx.name.genus, None);
        assert_eq!(ctx.name.rank, Rank::Unranked);
    }

    #[test]
    fn binomial_routes_genus_and_specific_epithet_not_uninomial() {
        let ctx = run("Abies alba", None);
        assert_eq!(ctx.name.genus, Some("Abies".to_string()));
        assert_eq!(ctx.name.uninomial, None);
        assert_eq!(ctx.name.specific_epithet, Some("alba".to_string()));
        assert_eq!(ctx.name.rank, Rank::Species);
    }

    #[test]
    fn cultivar_epithet_present_forces_genus_routing_even_for_a_bare_uninomial() {
        let mut ctx = ParseContext::new("Acer".to_string(), None, None, None);
        ctx.name.cultivar_epithet = Some("Elsrijk".to_string());
        ctx.tokens = tokenize("Acer");
        let boundary = authorship_split::find_boundary(&ctx.tokens, &ctx);
        classify(&mut ctx, boundary);
        assert_eq!(ctx.name.genus, Some("Acer".to_string()));
        assert_eq!(
            ctx.name.uninomial, None,
            "treatAsGenus (cultivarEpithet != null) must route through setGenus, not setUninomial"
        );
    }

    #[test]
    fn paren_subgenus_with_trailing_species_sets_infrageneric_epithet_and_rank_from_species() {
        let ctx = run("Amnicola (Amnicola) dubrueilliana", None);
        assert_eq!(ctx.name.genus, Some("Amnicola".to_string()));
        assert_eq!(ctx.name.infrageneric_epithet, Some("Amnicola".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("dubrueilliana".to_string()));
        assert_eq!(ctx.name.rank, Rank::Species);
    }

    #[test]
    fn paren_subgenus_alone_with_no_species_defaults_rank_to_infrageneric_name() {
        let ctx = run("Arrhoges (Antarctohoges)", None);
        assert_eq!(ctx.name.genus, Some("Arrhoges".to_string()));
        assert_eq!(
            ctx.name.infrageneric_epithet,
            Some("Antarctohoges".to_string())
        );
        assert_eq!(ctx.name.specific_epithet, None);
        assert_eq!(
            ctx.name.rank,
            Rank::InfragenericName,
            "a bare paren-subgenus with no explicit rank marker defaults to INFRAGENERIC_NAME"
        );
        assert_eq!(
            ctx.name.uninomial, None,
            "subgenus != null routes to genus even though there's no species epithet"
        );
    }

    #[test]
    fn abbreviated_subgenus_sets_infrageneric_epithet_with_dot_and_warns() {
        let ctx = run("Phalaena (Tin.) guttella", None);
        assert_eq!(ctx.name.genus, Some("Phalaena".to_string()));
        assert_eq!(ctx.name.infrageneric_epithet, Some("Tin.".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("guttella".to_string()));
        assert!(ctx
            .name
            .warnings
            .contains(&warnings::ABBREVIATED_SUBGENUS.to_string()));
    }

    #[test]
    fn infrageneric_marker_then_paren_subgenus_the_marker_form_wins_the_overwrite() {
        // Regression for the "infragenericEpithet from subgenus then overwritten by
        // infragenEpithet" trap: NameTokens.java always applies `if (subgenus != null)
        // setInfragenericEpithet(subgenus)` BEFORE `if (infragenEpithet != null)
        // setInfragenericEpithet(infragenEpithet)`, so when both are non-null the
        // marker-based one (set second) wins. `find_boundary` would normally exclude a
        // paren immediately following a consumed infrageneric marker from the name
        // boundary (its own `after_subgenus` flag blocks the subgenus cascade there) —
        // so this test calls `classify` directly with a hand-picked boundary that
        // includes both, to exercise NameTokens's own internal transition in isolation
        // (a legitimate use of `classify`'s explicit `boundary` parameter).
        let input = "Genus sect. Foo (Bar)";
        let mut ctx = ParseContext::new(input.to_string(), None, None, None);
        ctx.tokens = tokenize(input);
        let boundary = ctx.tokens.len();
        classify(&mut ctx, boundary);
        assert_eq!(ctx.name.genus, Some("Genus".to_string()));
        assert_eq!(
            ctx.name.infrageneric_epithet,
            Some("Foo".to_string()),
            "the infrageneric-MARKER form must win over the paren-subgenus form"
        );
        assert_eq!(ctx.name.rank, Rank::SectionBotany);
    }

    #[test]
    fn subsp_rank_marker_sets_infraspecific_epithet_and_rank_with_no_warnings() {
        let ctx = run("Abies alba subsp. alba", None);
        assert_eq!(ctx.name.genus, Some("Abies".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("alba".to_string()));
        assert_eq!(ctx.name.infraspecific_epithet, Some("alba".to_string()));
        assert_eq!(ctx.name.rank, Rank::Subspecies);
        assert!(ctx.name.warnings.is_empty());
    }

    #[test]
    fn apostrophe_particle_mid_name_author_is_skipped_and_records_mid_author_span() {
        let ctx = run("Cirsium creticum d'Urv. subsp. creticum", None);
        assert_eq!(ctx.name.genus, Some("Cirsium".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("creticum".to_string()));
        assert_eq!(ctx.name.infraspecific_epithet, Some("creticum".to_string()));
        assert_eq!(ctx.name.rank, Rank::Subspecies);
        // tokens: [Cirsium(0) creticum(1) d'Urv(2) .(3) subsp(4) .(5) creticum(6)]
        assert_eq!(ctx.mid_author_from, 2);
        assert_eq!(ctx.mid_author_to, 4);
        assert!(ctx.name.warnings.is_empty());
    }

    #[test]
    fn hybrid_mark_before_genus_adds_generic_notho() {
        let ctx = run("x Abies alba", None);
        assert_eq!(ctx.name.genus, Some("Abies".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("alba".to_string()));
        assert_eq!(ctx.name.notho, Some(vec![NamePart::Generic]));
    }

    #[test]
    fn hybrid_mark_after_genus_before_any_epithet_adds_specific_notho() {
        let ctx = run("Abies \u{00D7} alba", None);
        assert_eq!(ctx.name.genus, Some("Abies".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("alba".to_string()));
        assert_eq!(ctx.name.notho, Some(vec![NamePart::Specific]));
    }

    #[test]
    fn set_notho_overwrite_asymmetry_erases_earlier_generic_notho() {
        // A leading HYBRID_MARK adds GENERIC via add_notho (additive); the trailing
        // notho-prefixed infraspecific marker ("nothovar.") then fires the post-loop
        // `if (inlineRankNotho) setNotho(INFRASPECIFIC)`, which REPLACES the whole set —
        // erasing the earlier GENERIC entry rather than adding to it. This is the
        // load-bearing overwrite asymmetry the brief calls out.
        let ctx = run("x Abies alba nothovar. rubra", None);
        assert_eq!(ctx.name.genus, Some("Abies".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("alba".to_string()));
        assert_eq!(ctx.name.infraspecific_epithet, Some("rubra".to_string()));
        assert_eq!(ctx.name.rank, Rank::Variety);
        assert_eq!(
            ctx.name.notho,
            Some(vec![NamePart::Infraspecific]),
            "setNotho(INFRASPECIFIC) must REPLACE the set, not add to the earlier GENERIC entry"
        );
    }

    #[test]
    fn cf_qualifier_sets_epithet_qualifier_on_specific_and_marks_informal() {
        let ctx = run("Abies cf. alba", None);
        assert_eq!(ctx.name.genus, Some("Abies".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("alba".to_string()));
        assert_eq!(ctx.name.type_, NameType::Informal);
        let mut expected = std::collections::BTreeMap::new();
        expected.insert(NamePart::Specific, "cf.".to_string());
        assert_eq!(ctx.name.epithet_qualifier, Some(expected));
    }

    #[test]
    fn open_nomenclature_question_mark_between_epithets_sets_qualifier_on_infraspecific() {
        let ctx = run("Buteo borealis ? ventralis", None);
        assert_eq!(ctx.name.genus, Some("Buteo".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("borealis".to_string()));
        assert_eq!(
            ctx.name.infraspecific_epithet,
            Some("ventralis".to_string())
        );
        assert_eq!(ctx.name.rank, Rank::InfraspecificName);
        assert!(ctx.name.doubtful);
        assert_eq!(ctx.name.type_, NameType::Informal);
        assert!(ctx
            .name
            .warnings
            .contains(&warnings::QUESTION_MARKS_REMOVED.to_string()));
        let mut expected = std::collections::BTreeMap::new();
        expected.insert(NamePart::Infraspecific, "?".to_string());
        assert_eq!(ctx.name.epithet_qualifier, Some(expected));
    }

    #[test]
    fn indet_species_marker_with_single_letter_phrase_suppresses_the_indetermined_warning() {
        let ctx = run("Bryozoan sp. E", None);
        assert_eq!(ctx.name.genus, Some("Bryozoan".to_string()));
        assert_eq!(
            ctx.name.uninomial, None,
            "indet routes through genus, not uninomial"
        );
        assert_eq!(ctx.name.specific_epithet, None);
        assert_eq!(ctx.name.phrase, Some("sp. E".to_string()));
        assert_eq!(ctx.name.type_, NameType::Informal);
        assert_eq!(ctx.name.rank, Rank::Species);
        assert!(
            ctx.name.warnings.is_empty(),
            "a phrase identifier suppresses the generic INDETERMINED warning"
        );
    }

    #[test]
    fn bare_indet_species_marker_captures_the_marker_and_keeps_the_indetermined_warning() {
        // 5.0.0 Informal-phrase contract: a bare "Genus sp." captures the verbatim marker as its
        // phrase ("sp.") for a uniform taxon+phrase round-trip, but — unlike a name with a
        // distinguishing tail — it is still genuinely indeterminate, so the INDETERMINED warning
        // stays.
        let ctx = run("Bryozoan sp.", None);
        assert_eq!(ctx.name.genus, Some("Bryozoan".to_string()));
        assert_eq!(ctx.name.phrase, Some("sp.".to_string()));
        assert_eq!(ctx.name.type_, NameType::Informal);
        assert_eq!(ctx.name.rank, Rank::Species);
        assert!(ctx
            .name
            .warnings
            .contains(&warnings::INDETERMINED.to_string()));
    }

    #[test]
    fn informal_single_uppercase_letter_after_rank_marker_is_captured_as_infraspecific() {
        let ctx = run("Foo bar var. A", None);
        assert_eq!(ctx.name.genus, Some("Foo".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("bar".to_string()));
        assert_eq!(ctx.name.infraspecific_epithet, Some("A".to_string()));
        assert_eq!(ctx.name.rank, Rank::Variety);
        assert_eq!(ctx.name.type_, NameType::Informal);
        assert!(
            ctx.name.warnings.is_empty(),
            "infraspecific is set, so the indet-with-no-infraspecific INDETERMINED path must not fire"
        );
    }

    #[test]
    fn quadrinomial_overflow_drops_the_first_marker_and_flags_partial_unparsed() {
        let ctx = run("Abies alba subsp. nigra var. rubra pallens", None);
        assert_eq!(ctx.name.genus, Some("Abies".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("alba".to_string()));
        assert_eq!(
            ctx.name.infraspecific_epithet,
            Some("rubra".to_string()),
            "the SECOND marker (var.) wins; the first (subsp. nigra) is dropped"
        );
        assert_eq!(ctx.name.rank, Rank::Variety);
        assert_eq!(ctx.name.state, State::Partial);
        assert_eq!(ctx.name.unparsed, Some("pallens".to_string()));
        assert!(ctx
            .name
            .warnings
            .contains(&"Removed: subsp. nigra".to_string()));
        assert!(ctx
            .name
            .warnings
            .contains(&warnings::QUADRINOMIAL.to_string()));
    }

    #[test]
    fn abbreviated_genus_sets_informal_type_and_warns() {
        let ctx = run("M. alpium", None);
        assert_eq!(ctx.name.genus, Some("M.".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("alpium".to_string()));
        assert_eq!(ctx.name.type_, NameType::Informal);
        assert!(ctx
            .name
            .warnings
            .contains(&warnings::ABBREVIATED_GENUS.to_string()));
    }

    #[test]
    fn digit_leading_epithet_is_lowercased_into_specific_epithet() {
        let ctx = run("Adalia 11-punctata", None);
        assert_eq!(ctx.name.genus, Some("Adalia".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("11-punctata".to_string()));
        assert_eq!(ctx.name.rank, Rank::Species);
    }

    #[test]
    fn standalone_aggregate_marker_sets_ctx_aggregate() {
        let ctx = run("Poecilia sphenops agg.", None);
        assert_eq!(ctx.name.genus, Some("Poecilia".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("sphenops".to_string()));
        assert!(ctx.aggregate);
    }

    #[test]
    fn hyphenated_aggregate_suffix_strips_the_suffix_and_sets_ctx_aggregate() {
        let ctx = run("Poecilia sphenops-group", None);
        assert_eq!(ctx.name.genus, Some("Poecilia".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("sphenops".to_string()));
        assert!(ctx.aggregate);
    }

    #[test]
    fn requested_species_rank_with_an_infraspecific_epithet_warns_subspecies_assigned() {
        let ctx = run("Abies alba subsp. alba", Some(Rank::Species));
        assert_eq!(ctx.name.infraspecific_epithet, Some("alba".to_string()));
        assert!(ctx
            .name
            .warnings
            .contains(&warnings::SUBSPECIES_ASSIGNED.to_string()));
    }

    #[test]
    fn requested_infraspecific_rank_on_a_bare_binomial_is_marked_informal_indetermined() {
        let ctx = run("Abies alba", Some(Rank::Subspecies));
        assert_eq!(ctx.name.genus, Some("Abies".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("alba".to_string()));
        assert_eq!(ctx.name.infraspecific_epithet, None);
        assert_eq!(
            ctx.name.rank,
            Rank::Subspecies,
            "the caller-requested rank overwrites the post-loop SPECIES default"
        );
        assert_eq!(ctx.name.type_, NameType::Informal);
        assert!(ctx
            .name
            .warnings
            .contains(&warnings::INDETERMINED.to_string()));
    }

    #[test]
    fn missing_genus_placeholder_question_mark_is_kept_as_genus() {
        let ctx = run("? gryphoides", None);
        assert_eq!(ctx.name.genus, Some("?".to_string()));
        assert_eq!(ctx.name.specific_epithet, Some("gryphoides".to_string()));
    }

    #[test]
    fn caller_supplied_code_is_unaffected_by_classify() {
        // Sanity check that classify doesn't clobber a field it never touches.
        let mut ctx = ParseContext::new(
            "Abies alba".to_string(),
            None,
            None,
            Some(NomCode::Botanical),
        );
        ctx.tokens = tokenize(&ctx.original.clone());
        let boundary = authorship_split::find_boundary(&ctx.tokens, &ctx);
        classify(&mut ctx, boundary);
        assert_eq!(ctx.name.code, Some(NomCode::Botanical));
    }
}
