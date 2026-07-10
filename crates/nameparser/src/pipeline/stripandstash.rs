// SPDX-License-Identifier: Apache-2.0
//! Java `org.gbif.nameparser.pipeline.StripAndStash` — the pre-tokenisation stripper.
//! Removes annotations from the working string and stashes them onto
//! `ParseContext`/`ParsedName`. Per Java's own class doc: order matters — markers are
//! stripped from the most specific to the most general so that, for instance, a
//! "[sic, porphyria]" comment doesn't leak through the plainer "[sic]" path. [`run`] is
//! the explicit, ordered list of 55 strip steps; each step is a self-contained function
//! that takes the working string, mutates the [`ParseContext`] as needed, and returns
//! the (possibly shortened/rewritten) working string.
//!
//! **This file (Phase 1 Slice 2, Task 2): skeleton only.** [`run`] dispatches all 55
//! steps in Java's exact `StripAndStash.run(ParseContext)` order — that order is
//! load-bearing and is what this task locks in — but every step is currently a
//! no-op passthrough (`// TODO batch N` marks each one). `ctx.working` round-trips
//! byte-for-byte unchanged until each batch replaces its stubs with the faithful port
//! (see `docs/superpowers/plans/2026-07-10-phase1-stripandstash.md` for the batch
//! breakdown: batch 1 = steps 1-19, batch 2 = steps 20-30, batch 3 = steps 31-44, batch 4
//! = steps 45-52, batch 5 = steps 53-55 + the separate `stripAuthorshipMarkers` helper).

use crate::pipeline::ParseContext;

/// Java `StripAndStash.run(ParseContext ctx)`. Ordered dispatcher: threads the working
/// string through all 55 steps in Java's exact order (`StripAndStash.java:568-626`),
/// each step consuming the string by value and returning the (possibly
/// shortened/rewritten) result; steps also mutate `ctx` (`ctx.name`, `ctx.pending*`, …)
/// as a side effect for the ones that stash annotations rather than just discard them.
/// Every step is currently a `// TODO batch N` no-op stub (see the module doc), so this
/// call is presently a no-op overall: `ctx.working` comes out identical to how it went
/// in.
pub(crate) fn run(ctx: &mut ParseContext) {
    let mut s = ctx.working.clone();
    s = flag_uncertain_authorship(ctx, s);
    s = extract_generic_author(ctx, s);
    s = strip_quoted_monomial(ctx, s);
    s = apply_missing_genus_placeholder(ctx, s);
    s = strip_infra_rank_letters(ctx, s);
    s = normalise_letter_subdivision_marker(ctx, s);
    s = repair_question_mark_in_word(ctx, s);
    s = strip_strain_designation(ctx, s);
    s = stash_trailing_strain_code(ctx, s);
    s = strip_imprint_years(ctx, s);
    s = strip_null_between_epithets(ctx, s);
    s = normalise_hyphens(ctx, s);
    s = replace_homoglyphs(ctx, s);
    s = repair_win1252_artefacts(ctx, s);
    s = normalise_double_underscores(ctx, s);
    s = stash_trailing_otu_code(ctx, s);
    s = strip_serovar_serotype(ctx, s);
    s = strip_angle_bracket_authorship(ctx, s);
    s = strip_html(ctx, s);
    s = strip_candidatus(ctx, s);
    s = normalise_hort_ex_placeholder(ctx, s);
    s = strip_cultivar_group_grex(ctx, s);
    s = strip_quoted_cultivar(ctx, s);
    s = strip_extinct_dagger(ctx, s);
    s = strip_tinfr_marker(ctx, s);
    s = strip_doubtful_genus_brackets(ctx, s);
    s = strip_sic_and_corrig(ctx, s);
    s = stash_synonym_bracket(ctx, s);
    s = strip_bracketed_nom_note(ctx, s);
    s = strip_nom_note(ctx, s);
    s = strip_authorship_placeholders(ctx, s);
    s = strip_trailing_species_word(ctx, s);
    s = strip_pro_parte(ctx, s);
    s = strip_pro_sp_annotation(ctx, s);
    s = strip_approved_lists(ctx, s);
    s = strip_mihi(ctx, s);
    s = normalise_anon(ctx, s);
    s = strip_colon_concept_reference(ctx, s);
    s = strip_bracketed_tax_note(ctx, s);
    s = strip_paren_tax_note(ctx, s);
    s = strip_sensu_lato_remainder(ctx, s);
    s = strip_sensu_stricto_ss(ctx, s);
    s = strip_tax_note(ctx, s);
    s = strip_aggregate_suffix(ctx, s);
    s = strip_published_page(ctx, s);
    s = strip_in_press(ctx, s);
    s = strip_in_author_in_parens(ctx, s);
    s = strip_in_author_citation(ctx, s);
    s = strip_ipni_citation(ctx, s);
    s = strip_period_separated_reference(ctx, s);
    s = strip_comma_prefixed_reference(ctx, s);
    s = strip_manuscript_marker(ctx, s);
    s = strip_supra_rank_prefix(ctx, s);
    s = strip_leading_infrageneric_marker(ctx, s);
    s = stash_phrase_name(ctx, s);
    ctx.working = s;
}

// ---------------------------------------------------------------------------------
// Batch 1 (steps 1-19): leading normalizers + flaggers. Java StripAndStash.java's
// `flagUncertainAuthorship` through `stripHtml`.
// ---------------------------------------------------------------------------------

// TODO batch 1 — Java `flagUncertainAuthorship`: a trailing standalone "?" is dropped
// but marks doubtful; a "?" glued to an author word, or alternative authors joined by
// "or"/"/", mark the authorship uncertain (doubtful + UNCERTAIN_AUTHORSHIP warning).
fn flag_uncertain_authorship(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `extractGenericAuthor`: authorship placed BEFORE an infrageneric
// rank marker ("Cordia (Adans.) Kuntze sect. Salimori") is split off into
// `ctx.pending_generic_author`, leaving "Cordia sect. Salimori".
fn extract_generic_author(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `stripQuotedMonomial`: a leading quoted monomial ("'Prosthète'
// Hesse, 1861") is unquoted for parsing; the quote char is stashed in
// `ctx.quoted_monomial` (for Assemble to re-wrap later) and the name flagged doubtful.
fn strip_quoted_monomial(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `applyMissingGenusPlaceholder`: a lowercase-starting epithet
// followed by a capitalised author/year (or an explicit "Missing "/quoted-"?" marker)
// is rewritten to a "? <epithet>" placeholder form; sets type=PLACEHOLDER (+
// MISSING_GENUS warning for the inferred case).
fn apply_missing_genus_placeholder(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `stripInfraRankLetters`: Greek-letter (β, δ, …) or "***"
// fungal/informal rank markers sitting between two lowercase epithets are normalised to
// plain spaces so they aren't mistaken for authorship.
fn strip_infra_rank_letters(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `normaliseLetterSubdivisionMarker`: old-flora informal letter
// subdivisions ("a.", "b.", "a.b.") between a species and a trailing epithet are
// rewritten to the synthetic `RankMarkers.LETTER_SUBDIVISION` token (maps to Rank.OTHER
// downstream) unless the "letter" is itself a real rank marker (e.g. "f." = forma).
fn normalise_letter_subdivision_marker(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `repairQuestionMarkInWord`: "?" glued inside a word — a
// transcription artefact for a missing letter ("Istv?nffi") — is dropped (gluing the
// surrounding word parts back together) and flags doubtful + QUESTION_MARKS_REMOVED.
fn repair_question_mark_in_word(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `stripStrainDesignation`: a quoted strain designation introduced
// by "str"/"strain" ("… str .'Aph K2'") is kept as `ctx.name.phrase` (type=INFORMAL),
// leaving "Genus species str." for NameTokens to resolve to Rank.STRAIN.
fn strip_strain_designation(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `stashTrailingStrainCode`: a trailing uppercase/digit strain code
// on a binomial ("Candida albicans RNA_CTR0-3") becomes `ctx.name.phrase`
// (type=INFORMAL), reducing the working string to "Genus species".
fn stash_trailing_strain_code(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `stripImprintYears`: quoted-year-in-brackets, "(imprint YYYY)" /
// "(not YYYY)", and "& YYYY" trailing alternate-year annotations are stripped into
// `ctx.pending_imprint_year`.
fn strip_imprint_years(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `stripNullBetweenEpithets`: a bare "null" between two lowercase
// epithets (a data-quality artefact) is dropped, flagging doubtful + NULL_EPITHET.
fn strip_null_between_epithets(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `normaliseHyphens`: Unicode hyphen variants (‐‑‒–—) are folded to
// ASCII "-", flagging HOMOGLYHPS when anything actually changed.
fn normalise_hyphens(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `replaceHomoglyphs`: Latin look-alike homoglyphs from other
// scripts (via `UnicodeUtils`) are replaced with their canonical Latin counterpart,
// flagging HOMOGLYHPS when anything changed.
fn replace_homoglyphs(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `repairWin1252Artefacts(ParsedName, String)`: a small set of
// Win-1252-to-UTF-8 transcription artefacts (¡¢£‚„‰) between letters are mapped to their
// Latin look-alikes, flagging HOMOGLYHPS. NB Java's signature takes `ParsedName`
// directly (it's shared with the separately-supplied-authorship path,
// `stripAuthorshipMarkers`) rather than the full `ParseContext` every other step takes;
// this stub keeps the uniform `(ctx, s)` shape and reaches into `ctx.name` once ported.
fn repair_win1252_artefacts(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `normaliseDoubleUnderscores`: runs of 2+ underscores between
// letters ("Pseudocercospora__dendrobii") collapse to a single space.
fn normalise_double_underscores(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `stashTrailingOtuCode`: a trailing OTU-code identifier ("Oxalis
// barrelieri XXZ_21243") is stripped into `ctx.pending_unparsed`, leaving the name
// portion to parse normally.
fn stash_trailing_otu_code(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `stripSerovarSerotype`: bacterial serovar/serotype (+
// optional str./strain) annotations on a binomial are stripped silently.
fn strip_serovar_serotype(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `stripAngleBracketAuthorship`: an angle-bracketed authorship
// placeholder ("Doradidae <Unspecified Agent>") is stripped, flagging
// AUTHORSHIP_REMOVED + UNUSUAL_CHARACTERS and marking doubtful.
fn strip_angle_bracket_authorship(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 1 — Java `stripHtml`: HTML tags are stripped (keeping their text content)
// and a handful of HTML entities decoded, flagging XML_TAGS / HTML_ENTITIES
// respectively when either actually fired.
fn strip_html(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// ---------------------------------------------------------------------------------
// Batch 2 (steps 20-30): candidatus, cultivar Group/grex/quoted, extinct, t.infr.,
// doubtful-genus brackets, sic/corrig, synonym bracket, bracketed + bare nom-note.
// ---------------------------------------------------------------------------------

// TODO batch 2 — Java `stripCandidatus`: a leading "Candidatus "/"Ca. " prefix sets
// `candidatus = true` and `code = BACTERIAL`, and is stripped from the working string.
fn strip_candidatus(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `normaliseHortExPlaceholder`: "cv. ex" / "Hort. ex" /
// "hortus(a) ex" / "ht." horticultural placeholder variants are normalised to the
// canonical lower-case "hort.".
fn normalise_hort_ex_placeholder(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripCultivarGroupGrex`: a trailing "... CapWord(s) (Group|grex|gx)"
// sets `cultivarEpithet` + `code = CULTIVARS` + rank (CULTIVAR_GROUP or GREX).
fn strip_cultivar_group_grex(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripQuotedCultivar`: a quoted cultivar epithet, at end of
// string or mid-string before a trailing author span, sets `cultivarEpithet` +
// `code = CULTIVARS` + `rank = CULTIVAR` (splitting off a preceding species author into
// `ctx.pending_specific_author` for the mid-string form).
fn strip_quoted_cultivar(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripExtinctDagger`: "†"/"✝" anywhere in the string set
// `extinct = true` and are stripped.
fn strip_extinct_dagger(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripTinfrMarker`: the "t.infr." infraspecific abbreviation
// (Hieracium notation) is stripped so the trailing epithet parses as a normal
// infraspecific name.
fn strip_tinfr_marker(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripDoubtfulGenusBrackets`: a leading bracketed genus
// ("[Acontia] chia ...") has its brackets dropped, flagging doubtful + DOUBTFUL_GENUS.
fn strip_doubtful_genus_brackets(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripSicAndCorrig`: "[sic]"/"(sic)"/"[sic, comment]" set
// `originalSpelling = true` (the comment form also stashes the comment into
// `ctx.pending_unparsed`); "corrig."/"(corrig.)" sets `originalSpelling = false`.
fn strip_sic_and_corrig(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stashSynonymBracket`: a trailing "[= Grislea L. 1753]" synonymy
// reference is parked in `ctx.pending_unparsed`, flagging doubtful.
fn stash_synonym_bracket(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripBracketedNomNote`: a trailing bracketed/parenthesised
// nom-note ("[nom. et typ. cons.]", "(nom. nud.)") OVERWRITES `nomenclaturalNote`
// (not an append — this is the first step that can populate the field, so there is
// nothing to append to yet in Java's own call order either).
fn strip_bracketed_nom_note(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 2 — Java `stripNomNote`: a nom/comb/orth/sp.nov./pro-syn. keyword tail
// APPENDS to `nomenclaturalNote` (via `ctx.name.add_nomenclatural_note` once ported),
// plus manuscript-flag and rank-hint side effects.
fn strip_nom_note(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// ---------------------------------------------------------------------------------
// Batch 3 (steps 31-44): authorship placeholders, trailing-species, pro parte / pro
// sp. / approved-lists, mihi, anon, the taxonomic-note family, aggregate suffix.
// ---------------------------------------------------------------------------------

// TODO batch 3 — Java `stripAuthorshipPlaceholders`: "Not applicable"/"Not given"/
// "Not known"/… is stripped silently, flagging AUTHORSHIP_REMOVED.
fn strip_authorship_placeholders(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripTrailingSpeciesWord`: a trailing " species" on a bare
// Title-cased uninomial is dropped, producing a plain monomial.
fn strip_trailing_species_word(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripProParte`: ", pro parte" / ", p.p." is stripped silently,
// flagging doubtful.
fn strip_pro_parte(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripProSpAnnotation`: " (pro sp./spec./syn./hyb.)" is stripped
// silently.
fn strip_pro_sp_annotation(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripApprovedLists`: " (Approved Lists YYYY)" is stripped
// silently.
fn strip_approved_lists(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripMihi`: "mihi"/"Mihi" self-attribution (wherever it occurs)
// is stripped, flagging AUTHORSHIP_REMOVED when it actually fired.
fn strip_mihi(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `normaliseAnon`: "Anon."/"Anon"/"anon" is normalised to the
// canonical lower-case "anon." so it parses as a real (anonymous) authorship.
fn normalise_anon(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripColonConceptReference`: a trailing ": Author, YYYY"
// botanical taxonomic-concept citation APPENDS to `taxonomicNote`.
fn strip_colon_concept_reference(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripBracketedTaxNote`: a trailing "[auctt./sensu/sec/non/nec/
// misspelling/misapplied/misident ...]" bracket APPENDS to `taxonomicNote`.
fn strip_bracketed_tax_note(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripParenTaxNote`: a trailing "(nec/non/not …, YYYY)" homonym
// citation APPENDS to `taxonomicNote`.
fn strip_paren_tax_note(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripSensuLatoRemainder`: mid-string "s.l."/"s.str."/"s.lat."/
// "s.ampl." followed by trailing junk APPENDS the (lower-cased, whitespace-collapsed)
// marker to `taxonomicNote` and parks the junk in `ctx.pending_unparsed`.
fn strip_sensu_lato_remainder(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripSensuStrictoSS`: a trailing "s.s." (optionally followed by
// junk) APPENDS "s.s." to `taxonomicNote` and parks any junk in `ctx.pending_unparsed`.
fn strip_sensu_stricto_ss(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripTaxNote`: the general end-anchored taxonomic-note anchor
// (auct./sensu/sec./nec/emend./fide/according to/excl./ss/s.l./s.str./…) OVERWRITES
// `taxonomicNote` (the last of the taxonomic-note family to run, so anything already
// captured by the more specific steps above it never reaches this one).
fn strip_tax_note(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 3 — Java `stripAggregateSuffix`: " agg."/"aggregate"/"species group"/
// "species complex"/"group"/"complex"/"-group"/"-aggregate" sets `ctx.aggregate = true`.
fn strip_aggregate_suffix(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// ---------------------------------------------------------------------------------
// Batch 4 (steps 45-52): published-in family (page, in-press, in-author variants,
// IPNI, period-/comma-separated references) + manuscript marker.
// ---------------------------------------------------------------------------------

// TODO batch 4 — Java `stripPublishedPage`: a trailing ": 377" / ": 12-18" page
// reference is pulled into `publishedInPage`.
fn strip_published_page(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 4 — Java `stripInPress`: " in press" sets `manuscript = true` and APPENDS
// "in press" to `nomenclaturalNote`.
fn strip_in_press(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 4 — Java `stripInAuthorInParens`: an "in <publication>" citation INSIDE a
// parenthesised basionym ("(Geoffroy in Fourcroy, 1785)") rewrites the parens to just
// the basionym author (+ year, moved over when the basionym itself had none) and
// APPENDS the publication reference to `publishedIn`.
fn strip_in_author_in_parens(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 4 — Java `stripInAuthorCitation`: a trailing " in <Author>" / " apud
// <Author>" tail APPENDS to `publishedIn` and records a code-neutral
// `ctx.pending_year`/`pending_year_from_publication`.
fn strip_in_author_citation(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 4 — Java `stripIpniCitation`: an IPNI-style "Author., Title (Year)."
// citation OVERWRITES `publishedIn` (extracting any embedded nom-note first, which
// APPENDS to `nomenclaturalNote`) and records the pending publication year.
fn strip_ipni_citation(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 4 — Java `stripPeriodSeparatedReference`: a "Surname. <Reference Title>
// ... year ..." citation OVERWRITES `publishedIn`; a page-range ref flags
// NOMENCLATURAL_REFERENCE instead of propagating the year, a clean one propagates the
// pending publication year.
fn strip_period_separated_reference(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 4 — Java `stripCommaPrefixedReference`: an "Author(s), <Reference Title>
// …" citation OVERWRITES `publishedIn` and flags NOMENCLATURAL_REFERENCE (the year is
// never propagated for this form).
fn strip_comma_prefixed_reference(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 4 — Java `stripManuscriptMarker`: a trailing "ined."/"ms."/"msc."/
// "unpublished" sets `manuscript = true` and APPENDS the (lower-cased) tag to
// `nomenclaturalNote`.
fn strip_manuscript_marker(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// ---------------------------------------------------------------------------------
// Batch 5 (steps 53-55): suprarank prefix, leading infrageneric marker, phrase name.
// (Plus the separate `stripAuthorshipMarkers` auxiliary-authorship reimplementation,
// not part of this ordered dispatch — see the investigation §4.)
// ---------------------------------------------------------------------------------

// TODO batch 5 — Java `stripSupraRankPrefix`: a leading "<Family> <suprageneric-rank-
// marker>" (or a bare suprageneric marker with no family) strips the prefix/marker and
// pins the rank.
fn strip_supra_rank_prefix(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 5 — Java `stripLeadingInfragenericMarker`: a leading infrageneric rank
// marker with no genus prefix ("subgen. Trematostoma Sacc.") strips the marker and pins
// the rank (swapped to its zoological counterpart under a caller-supplied ZOOLOGICAL
// code).
fn strip_leading_infrageneric_marker(_ctx: &mut ParseContext, s: String) -> String {
    s
}

// TODO batch 5 — Java `stashPhraseName`: BOLD/specimen-style phrase-name forms
// ("Prostanthera sp. Somersbey (B.J.Conn 4024)") set `ctx.name.phrase` and rewrite the
// working string to "Genus[ species] marker. [Author]" so NameTokens sees an indet name.
fn stash_phrase_name(_ctx: &mut ParseContext, s: String) -> String {
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This task (Phase 1 Slice 2, Task 2) stubs every one of the 55 steps as a pure
    /// passthrough — the dispatcher order is locked in, but nothing observable changes
    /// yet. Locks that invariant down so a batch landing later can't silently leave a
    /// stub half-wired (e.g. dropping `s` instead of threading it through, or mutating
    /// `ctx.name` from inside a stub ahead of its own batch).
    #[test]
    fn run_is_a_complete_noop_until_the_batches_land() {
        let mut ctx = ParseContext::new("Abies alba Mill.".to_string(), None, None, None);
        let before_working = ctx.working.clone();
        let before_name = ctx.name.clone();
        run(&mut ctx);
        assert_eq!(ctx.working, before_working);
        assert_eq!(ctx.name, before_name);
    }
}
