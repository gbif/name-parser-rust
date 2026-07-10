# Phase 1 Slice 2 — StripAndStash (annotation stripper)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Port the Java `StripAndStash` stage (55 ordered strip steps, 106 patterns) to Rust, validated by extending the golden harness to diff the **downstream-independent fields** it exclusively owns.

**Architecture:** `StripAndStash::run(ctx)` runs after `Preflight`, before the (not-yet-ported) tokenizer. It strips annotations from `ctx.working` and stashes them onto `ctx.name` + `ctx.pending*`. Because the 10 downstream-independent fields (`extinct`, `originalSpelling`, `nomenclaturalNote`, `taxonomicNote`, `publishedIn`, `publishedInPage`, `publishedInYear`, `manuscript`, `candidatus`, `cultivarEpithet`) are set ONLY here and never modified by later stages, the existing full-parse Java oracle (`expected-parse.jsonl`) already contains their final values — so Rust `parse()` (Preflight + StripAndStash) can be diffed against them at corpus scale now. Steps are ported in run()-order across sub-slices; the per-field mismatch count is the climbing gate (baseline → 0).

**Tech Stack:** Rust (edition 2021), `regex` + `fancy-regex`, `serde_json`. Java oracle = the existing shaded jar `parse` output.

**Scope of THIS sub-slice (2a):** the harness extension + baseline (Task 1), the `run` skeleton + model prep (Task 2), and the **first step batch** (Task 3, steps 1–19 in run()-order). Batches 2b–2e (steps 20–55, outlined at the end) are follow-on sub-slices, each reducing the mismatch count further. StripAndStash is NOT expected to reach 0 mismatches until all 55 steps are ported.

## Global Constraints

- **Faithful port; Java is the oracle** (`/Users/markus/code/gbif/name-parser/`). The 55-step ORDER in `StripAndStash.run` is load-bearing — port and dispatch them in exactly that order (see the reference map in `docs/superpowers/findings/` / the investigation; the ordered list is Task 3's spec).
- **Per-pattern regex-flag fidelity.** 106 patterns across 7 flag groups. Rule: `UNICODE_CHARACTER_CLASS` in Java → keep the crate's default Unicode `\d\s\w\b`; otherwise ASCII-scope them via `(?-u:…)`. `\p{Lu}`/`\p{Ll}`/`\p{L}` are always Unicode. Java `CASE_INSENSITIVE`→`(?i)`; inline `(?i:…)`/`(?-i:…)` scopes port verbatim. Use `crate::unicode::java_trim` for Java `.trim()`.
- **`publishedInYear` is derived, never set directly.** Java `ParsedAuthorship.setPublishedIn(String)` computes `publishedInYear` as a side effect (last regex match of `\b(1[5-9]\d{2}|20\d{2}|2100)\b`). The Rust model's `set_published_in` (add one) must replicate this; the port never sets `published_in_year` directly.
- **Append-vs-overwrite is per-step, not uniform.** `nomenclaturalNote`/`taxonomicNote`/`publishedIn` are APPENDED (`existing==null ? note : existing + " " + note`) by most steps but OVERWRITTEN by a few (`stripBracketedNomNote`; `stripTaxNote`; `stripIpniCitation`/`stripPeriodSeparatedReference`/`stripCommaPrefixedReference`). Replicate each step's exact behavior — a blanket "always append" diverges.
- **`warnings` dedup:** use `ParsedName::add_warning` (added in the foundation slice), never a raw push.
- **`pending*` fields are NOT validated this slice** — they don't reach `ParsedName` until `Pipeline`/`Assemble` (not ported). Populate them faithfully (later slices consume them) but expect them absent from the gate.
- **SPDX header on every new/edited source file; crate 0.0.0.**
- **Working dir** `/Users/markus/code/gbif/name-parser-rust/`. Preamble: `export PATH="$HOME/.cargo/bin:$PATH"; [ -s "$HOME/.sdkman/bin/sdkman-init.sh" ] && source "$HOME/.sdkman/bin/sdkman-init.sh"`.

## Downstream-independent field set (the gate) — verified set-only-in-StripAndStash

`extinct` (bool), `originalSpelling` (Option<bool>), `nomenclaturalNote` (Option<String>), `taxonomicNote` (Option<String>), `publishedIn` (Option<String>), `publishedInPage` (Option<String>), `publishedInYear` (Option<i32>, derived), `manuscript` (bool), `candidatus` (bool), `cultivarEpithet` (Option<String>).

---

## Task 1: Extend the golden harness to the downstream-independent fields + baseline

**Files:** Modify `crates/nameparser/tests/parse_golden.rs`.

- [ ] **Step 1:** Extend the existing `parse_golden` test: for every oracle row Java `parsed` (not error), additionally compare the 10 downstream-independent fields between the Java `parsed` object and Rust's `parse()` result. Use serde_json `Value` for the Java side; read the field via its JSON key (e.g. `parsed["nomenclaturalNote"]`, absent = None). Compare each field; tally per-field mismatches. Keep the existing error-classification assertions unchanged.
- [ ] **Step 2:** Run `cargo test -p nameparser --test parse_golden -- --nocapture`. Since StripAndStash isn't ported yet, Rust produces none of these fields → the output prints the **baseline**: per-field mismatch counts = the number of corpus names where Java sets each field. Record these numbers (they quantify the remaining work and are the trajectory metric). **Temporarily** assert only that the error-classification counts are still 0; gate the new field-mismatches with a `TODO`/`eprintln` (do NOT assert 0 yet — that comes as steps are ported). Add a clear comment that the field-mismatch assertion is deferred until batch 2e.
- [ ] **Step 3:** Commit `Phase 1 s2: golden harness diffs downstream-independent fields + baseline`.

---

## Task 2: StripAndStash skeleton + model prep

**Files:** Create `crates/nameparser/src/pipeline/stripandstash.rs`; modify `pipeline/mod.rs` (wire the call), `model/name.rs` (`set_published_in`).

- [ ] **Step 1:** In `model/name.rs`, add `ParsedName::set_published_in(&mut self, s: &str)` that sets `published_in = Some(s.to_string())` AND derives `published_in_year` via the last match of `(?-u:\b)(1[5-9]\d{2}|20\d{2}|2100)(?-u:\b)` (ASCII `\b`, matching Java's non-UNICODE default in `ParsedAuthorship`). Test: `set_published_in("Annals … 1988")` → `published_in_year == Some(1988)`; a ref with no year → None; a page-range-then-year (`75: 1658-1662 … 1988`) → 1988 (last match). Also add `ParsedName::add_nomenclatural_note`/`add_taxonomic_note` (append with `" "` sep, null-coalescing) and matching overwrite via the plain `Option` field for the overwrite-steps.
- [ ] **Step 2:** Create `pipeline/stripandstash.rs` with `pub(crate) fn run(ctx: &mut ParseContext)` — the ordered dispatcher: `let mut s = ctx.working.clone(); s = step1(ctx, s); … s = step55(ctx, s); ctx.working = s;`. Stub every one of the 55 step fns as `fn stepN(_ctx: &mut ParseContext, s: String) -> String { s }` (no-op passthrough) with a `// TODO batch X` note, so the dispatcher compiles and the order is locked in. (Batches fill the bodies.) The 55 names + order are in Task 3's spec.
- [ ] **Step 3:** Wire `stripandstash::run(&mut ctx)` into `pipeline::run`, immediately after `preflight::run(...)?` and before the `hasLetter` guard (matching `Pipeline.java:70-71`). `cargo test -p nameparser` stays green (all steps are no-ops, so no field changes yet — baseline unchanged).
- [ ] **Step 4:** Commit `Phase 1 s2: StripAndStash run() skeleton (55 no-op steps) + set_published_in`.

---

## Task 3: Port batch 1 — steps 1–19 (leading normalizers + flaggers)

Java ref: `StripAndStash.java`. Port steps 1–19 IN ORDER, replacing each no-op stub with the faithful port. This batch is mostly working-string normalization + `doubtful`/`warnings`/`pending*` (few downstream-independent fields — `candidatus` at step 20 is batch 2), so the gate metric moves little here; the value is establishing the working-string transformations the later note/ref steps depend on.

Steps (name → stashes; full detail in the investigation map):
1 `flagUncertainAuthorship` (doubtful, warnings) · 2 `extractGenericAuthor` (pendingGenericAuthor) · 3 `stripQuotedMonomial` (quotedMonomial, doubtful) · 4 `applyMissingGenusPlaceholder` (type=PLACEHOLDER, warning) · 5 `stripInfraRankLetters` · 6 `normaliseLetterSubdivisionMarker` · 7 `repairQuestionMarkInWord` (doubtful, warning) · 8 `stripStrainDesignation` (phrase, type=INFORMAL) · 9 `stashTrailingStrainCode` (phrase, type=INFORMAL) · 10 `stripImprintYears` (pendingImprintYear) · 11 `stripNullBetweenEpithets` (doubtful, warning) · 12 `normaliseHyphens` (structural; HOMOGLYHPS warning) · 13 `replaceHomoglyphs` (structural, UnicodeUtils) · 14 `repairWin1252Artefacts` (structural; shared w/ stripAuthorshipMarkers) · 15 `normaliseDoubleUnderscores` · 16 `stashTrailingOtuCode` (pendingUnparsed) · 17 `stripSerovarSerotype` · 18 `stripAngleBracketAuthorship` (warnings, doubtful) · 19 `stripHtml` (XML_TAGS/HTML_ENTITIES warnings).

- [ ] **Step 1: Behavioural tests first.** For each step that sets an observable field or has a documented example (in the investigation map / CLAUDE.md), write a focused unit test calling `stripandstash::run` (or the step) and asserting the working-string result and/or the stashed field. Verify RED.
- [ ] **Step 2: Port steps 1–19** in order, applying the per-pattern flag rule to each `Pattern` (check its flag group). Port `replaceHomoglyphs`/`repairWin1252Artefacts`/`normaliseHyphens` as structural (they use no `Pattern` — port `UnicodeUtils` bits as needed; note `replaceHomoglyphs` may need a `homoglyphs` map — port the subset used, or defer with a documented stub if large). Where a step needs a not-yet-ported helper (`AuthorParticles.isParticle`, `RankMarkers.*`), port the minimal helper or stub with a note.
- [ ] **Step 3: Run tests + the golden harness.** `cargo test -p nameparser`. Record the (A)-field mismatch trajectory (should be ~unchanged from baseline — batch 1 sets few (A) fields — confirming no regressions). Fix any Preflight/skeleton interaction bugs.
- [ ] **Step 4: Commit** `Phase 1 s2: port StripAndStash steps 1-19 (normalizers + flaggers)`.

---

## Follow-on sub-slices (outline — each its own plan+SDD run)

- **2b — steps 20–30** (candidatus, cultivar group/grex, quoted cultivar, extinct dagger, t.infr., doubtful-genus brackets, sic/corrig [reuse spike CORRIG], synonym bracket, bracketed + bare nom-note). First big (A)-field gains: `candidatus`, `cultivarEpithet`, `extinct`, `originalSpelling`, `nomenclaturalNote`.
- **2c — steps 31–44** (authorship placeholders, trailing-species, pro parte / pro sp. / approved-lists, mihi, anon, the 6 taxonomic-note steps, aggregate). Gains: `taxonomicNote`.
- **2d — steps 45–52** (published page, in-press, in-author-in-parens, in-author citation, IPNI, period-/comma-separated references, manuscript marker). Gains: `publishedIn`/`publishedInPage`/`publishedInYear`, `manuscript`.
- **2e — steps 53–55** (suprarank prefix, leading infrageneric marker, phrase name) + `stripAuthorshipMarkers` (the separate auxiliary-authorship reimplementation, per the investigation §4). **Gate flips on:** assert the 10 downstream-independent field mismatches == 0 (or documented allowlist).

## Self-Review
Covers the design's §6.2 pipeline (StripAndStash stage) and realizes findings-requirement #2 incrementally (corpus diff, downstream-independent-field level). Deferred + stated: `pending*`/working-string validation (until downstream stages), the 0-mismatch gate (until batch 2e). Placeholder scan: the step bodies are "port from the named Java method" (authoritative in-repo source), not invented logic. Type consistency: `stripandstash::run` consumes `ParseContext` (foundation) + `ParsedName::set_published_in`/`add_*` (Task 1); the harness (Task 1) consumes `nameparser::parse` + the model fields.
