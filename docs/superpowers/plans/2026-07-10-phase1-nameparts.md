# Phase 1 Slice 3 — AuthorshipSplit + NameTokens (name-part classification)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use `- [ ]`.

**Goal:** Port `AuthorshipSplit.findBoundary` + `NameTokens.classify` to Rust — the stages that find the name/author boundary and classify the pre-boundary tokens into name parts — validated by extending the golden harness to the name-part fields.

**Architecture:** `Pipeline::run` currently runs Preflight → StripAndStash → tokenizer[done] → then a `// TODO`. This slice wires in `AuthorshipSplit::find_boundary(&ctx.tokens, &ctx) -> usize` then `NameTokens::classify(&mut ctx, boundary)`. Both are **regex-free** hand-written token-index state machines (AuthorshipSplit 471 lines / NameTokens 712 lines in Java). NameTokens sets the name-part `ParsedName` fields; AuthorshipSplit is a pure function returning the boundary (no side effects). The observable name-part fields let the corpus harness validate the combination.

**Tech Stack:** Rust (edition 2021). **No new regex** — pure `Vec<Token>` walking. `RankMarkers` = two `HashMap<&str, Rank>` lookups (port). `AuthorParticles::is_particle` already ported (batch-1 `PARTICLES`).

## Global Constraints

- **Faithful port; Java is the oracle** (`/Users/markus/code/gbif/name-parser/`). Both stages are pure token-index logic — port the control flow branch-for-branch. No behaviour changes; divergences documented.
- **Known Java quirks to preserve (do NOT "clean up"):**
  1. `NameTokens` `setNotho(INFRASPECIFIC)` (post-loop, `inlineRankNotho`) is a single-value setter that **overwrites** the whole notho EnumSet — so an earlier `addNotho(GENERIC)` from a `HYBRID_MARK` gets erased. Reproduce this (a `set_notho` that replaces vs `add_notho` that inserts).
  2. `AuthorshipSplit.skipParenAuthorBlock` and `NameTokens.skipParenAuthorBlock` are **two non-identical copies** — AuthorshipSplit's gates on `hasEpithetAfterMarker(...false)` before returning; NameTokens's returns at the first infraspecific-marker word with no epithet-follow check. Port BOTH copies faithfully (do not unify).
  3. The `OPEN_PAREN` subgenus-vs-basionym-author **7-rule cascade** (AuthorshipSplit, documented inline) — port the rule order exactly.
- **`boundary == 0`** in NameTokens → set `state=PARTIAL`, `unparsed=ctx.original`, return immediately.
- **`notho`/`epithetQualifier` serialization** already defined in the model (Option<Vec<NamePart>> / Option<BTreeMap>). Use `add_notho`/`set_notho`/`set_epithet_qualifier` (add these to `ParsedName` if absent).
- `Rank` is still a partial stub (~28/117). NameTokens/AuthorshipSplit reference rank markers + a few rank constants (INFRASPECIFIC_NAME, INFRASUBSPECIFIC_NAME, SPECIES, INFRAGENERIC_NAME, SUBGENUS, …) — add the variants they need (note the rest still deferred).
- SPDX header; crate 0.0.0; no new deps.
- **Working dir** `/Users/markus/code/gbif/name-parser-rust/`. Preamble: `export PATH="$HOME/.cargo/bin:$PATH"; [ -s "$HOME/.sdkman/bin/sdkman-init.sh" ] && source "$HOME/.sdkman/bin/sdkman-init.sh"`.

## Validation strategy (field ownership — verified)

Extend the golden harness to 7 name-part fields.
- **ASSERT 0 (NameTokens-final):** `infraspecificEpithet`, `epithetQualifier`, `notho`. (Set only in NameTokens; never modified downstream.)
- **MEASURE, don't assert 0 yet (Assemble-rewrite residuals — assert 0 in the Assemble slice):** `genus`, `uninomial`, `infragenericEpithet`, `specificEpithet`. Residual causes: Assemble's uninomial↔genus moves (caller-rank / infrageneric-strict / underscore-split), quoted-monomial re-wrap, phrase-promotion; StripAndStash's `PHRASE_GENUS_SUBGENUS` infragenericEpithet. Document the residual and its cause.
- `notho`/`epithetQualifier` diff as sets/maps (order-insensitive) like `warnings`.

---

## Task 1: RankMarkers maps + harness extension to name-part fields + baseline

**Files:** Create `crates/nameparser/src/pipeline/rank_markers.rs`; modify `crates/nameparser/tests/parse_golden.rs`, `model/name.rs` (name-part setters if absent), `model/enums.rs` (Rank variants).

- [ ] **Step 1: `RankMarkers`.** Port the two Java maps (`RankMarkers.INFRASPECIFIC`, `RankMarkers.INFRAGENERIC`: marker-string → `Rank`) and `match_infraspecific_allow_notho`/`match_infrageneric_allow_notho` (strip a leading `"notho"` — or bare `"n"` for infraspecific — then look up). Add the `Rank` variants referenced. Unit-test a few markers (`subsp.`→SUBSPECIES, `var.`→VARIETY, `sect.`→SECTION_BOTANY, `subg.`→SUBGENUS, `nothovar.`→VARIETY+notho).
- [ ] **Step 2: Harness extension.** In `parse_golden.rs`, for each both-parsed row, diff the 7 name-part fields (Java `parsed[key]` vs Rust). `notho`/`epithetQualifier` compared order-insensitively. Tally per-field. Assert 0 ONLY for `infraspecificEpithet`/`epithetQualifier`/`notho` (deferred until NameTokens lands — for THIS task, since NameTokens isn't ported yet, keep them as `eprintln` baseline, NOT asserted; add the assert in Task 3). Measure `genus`/`uninomial`/`infragenericEpithet`/`specificEpithet` (eprintln). Keep the StripAndStash 10-field asserts + error-classification asserts intact.
- [ ] **Step 3: Baseline.** Run the harness; record per-field baseline counts (with AuthorshipSplit/NameTokens not yet ported, Rust sets no name-part fields → baseline = count of names Java sets each). Commit `Phase 1 s3: RankMarkers + harness name-part fields + baseline`.

---

## Task 2: AuthorshipSplit::find_boundary

**Files:** Create `crates/nameparser/src/pipeline/authorship_split.rs`. Java ref: `AuthorshipSplit.java` (471 lines, regex-free).

**Interfaces produced:** `authorship_split::find_boundary(tokens: &[Token], ctx: &ParseContext) -> usize` (pure — reads only `ctx.requested_rank`); plus the two bridges NameTokens needs: `mid_name_author_end(...)`, `is_apostrophe_particle(...)`.

- [ ] **Step 1: Unit tests first** (from Java behaviour — AuthorshipSplit returns an int, so it's unit-tested here; the corpus validates it via NameTokens in Task 3). Cases: `[Abies, alba]`→2 (no author); `[Vulpes, vulpes, silaceus, Miller, COMMA, 1907]`→3; `Genus (Subgenus) species`→subgenus kept in name; `Genus (Author) …`→basionym boundary; the 7-rule paren cascade cases; a trinomial with rank marker.
- [ ] **Step 2: Run → fail. Step 3: Port `find_boundary`** + the 8 private helpers (`strip_dot`, `consume_mid_name_author`, `has_epithet_after_marker`, `has_year_token`, `skip_paren_author_block` [AuthorshipSplit's copy], `is_family_shape`, `looks_like_apostrophe_particle`, `is_all_upper`) branch-for-branch. Uses `rank_markers::*` + `AuthorParticles::is_particle`. **Step 4: Run → pass.** Full suite green.
- [ ] **Step 5: Commit** `Phase 1 s3: port AuthorshipSplit::find_boundary`.

---

## Task 3: NameTokens::classify + wire + corpus-validate

**Files:** Create `crates/nameparser/src/pipeline/name_tokens.rs`; modify `pipeline/mod.rs` (wire the calls), `parse_golden.rs` (flip the 3 NameTokens-final asserts on). Java ref: `NameTokens.java` (712 lines, regex-free).

**Interfaces produced:** `name_tokens::classify(ctx: &mut ParseContext, boundary: usize)`.

- [ ] **Step 1: Unit tests first** — the setter table from the map: uninomial vs genus routing; subgenus/infrageneric; `subsp.`/`var.` → infraspecificEpithet + rank; hybrid `notho`; `cf.`/`aff.` → epithetQualifier + type=INFORMAL; `?` open-nomenclature; indet sub-cascade → phrase; the `setNotho`-overwrite asymmetry; quadrinomial overflow → PARTIAL+unparsed+warnings.
- [ ] **Step 2: Run → fail. Step 3: Port `classify`** + the 8 helpers (`render_author_span`, `skip_paren_author_block` [NameTokens's DIFFERENT copy], `recover_case`, `is_all_upper_letters`, `has_infraspecific_epithet_after`, `is_strain_code`, `is_all_letter_case`, `strip_dot`) branch-for-branch. Set `ctx.mid_author_from/to`, `ctx.aggregate`, and every `ctx.name` field per the table. Use `add_notho`/`set_notho` (the overwrite asymmetry), `set_epithet_qualifier`, `add_warning`.
- [ ] **Step 4: Wire** in `pipeline/mod.rs`: after `ctx.tokens = tokenize(...)`, do `let boundary = authorship_split::find_boundary(&ctx.tokens, &ctx); name_tokens::classify(&mut ctx, boundary);` (matching `Pipeline.java:76-77`). (The autonym mid-author + authorship-parse paths remain `// TODO` — AuthorshipParser slice.)
- [ ] **Step 5: Flip the gate + corpus-validate.** In `parse_golden.rs`, turn the `infraspecificEpithet`/`epithetQualifier`/`notho` tallies into `assert_eq!(_, 0)`. Run the harness. **Expected: those 3 → 0; `genus`/`uninomial`/`infragenericEpithet`/`specificEpithet` drop to a small residual (Assemble edge cases).** Error-classification + the StripAndStash 10 fields must NOT regress. Triage: a NameTokens-final field not at 0 is a boundary (AuthorshipSplit) or classify bug — investigate the printed mismatches, fix. Document the genus/uninomial residual (Assemble). Record before→after.
- [ ] **Step 6: Commit** `Phase 1 s3: port NameTokens::classify + wire; assert name-part-final fields = 0`.

---

## Task 4: Status doc

- [ ] Record: name-part gate result (the 3 final fields at 0; genus/uninomial/etc residual + cause); what's deferred (Assemble rewrites of genus/uninomial, the autonym/aux-authorship paths → AuthorshipParser slice); next slice (AuthorshipParser → authorship fields). Commit.

## Self-Review
Realizes the design's §6.2 pipeline (AuthorshipSplit + NameTokens stages) and extends the golden method to name-part fields. Deferred + stated: genus/uninomial/infragenericEpithet/specificEpithet full parity (Assemble slice), authorship fields (AuthorshipParser slice). Both stages regex-free → no per-pattern-flag / possessive concerns. Type consistency: `find_boundary` (Task 2) feeds `classify` (Task 3); both consume `rank_markers` (Task 1); the harness (Task 1) consumes `nameparser::parse` + the model name-part fields.
