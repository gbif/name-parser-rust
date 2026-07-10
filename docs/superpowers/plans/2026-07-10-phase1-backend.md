# Phase 1 Slice 4 — Back-end: AuthorshipParser + CodeInference + Assemble (full parsed-field parity)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use `- [ ]`.

**Goal:** Port the coupled back-end — `AuthorshipParser`, `CodeInference`, `Assemble.finish`, `BlacklistedEpithets` — and wire the full `Pipeline` authorship/code/assembly plumbing, reaching **full `ParsedName` parity** with Java over the corpus (authorship fields + `code` + closing the name-part residuals).

**Architecture:** After `NameTokens.classify` (done), `Pipeline` currently stops (`// TODO`). This slice adds: `AuthorshipParser::parse(tokens, from) -> AuthState` (100% structural, zero regex); `CodeInference` (2 trivial patterns, sets `code`); `Assemble::finish` (final invariants, closes name-part residuals + code); `BlacklistedEpithets` (`include_str!` a 274-line stop-word file). Then the `Pipeline.run` back-end timeline (§2 of the investigation): applyAuthorship → autonym mid-author → aux-authorship (via the now-portable `stripAuthorshipMarkers`) → sanctioning propagation → codeState selection → pendingYear (before Assemble) → **Assemble.finish** → pendingYear (after) → imprintYear → pendingSpecific/GenericAuthor. Result: the full ParsedName, validated by extending the golden harness to every field.

**Tech Stack:** Rust 2021. AuthorshipParser needs **no regex**. CodeInference: 2 plain `regex` patterns (ICN/ICZN status, no flags). Assemble: 1 pattern (`\d{4}`). No `fancy_regex` anywhere in this slice.

## Global Constraints

- **Faithful port; Java is the oracle** (`/Users/markus/code/gbif/name-parser/`). All four files are structural/near-structural — port branch-for-branch. Regex ports follow the per-pattern flag rule (all here are ASCII/no-flag → ASCII-scope `\d\b` via `(?-u:…)`).
- **AuthorshipParser quirks to preserve (from the investigation):**
  1. The `COMMA` handler's 3 lookahead branches currently all run identical `flush; i++; continue` — replicate the flush-always behaviour (do NOT "fix" the vestigial classification).
  2. Sanctioning-author **last-write-wins**: aux applied first, embedded second (embedded wins).
  3. `AUTHOR_SUFFIXES` is checked **case-sensitively** in the post-surname chaining branch but **case-insensitively** in the standalone-after-separator branch (same 14-entry set).
  4. Bracketed year `[YYYY]` → always `imprintYear` (first-wins), never `year`. Second plain year → imprintYear. Year-range → keep first + `yearRange=true`.
  5. Surname-first inversion + `formatInitials` (hyphenated `Y.-j.` preserves case + dots; CJK `Z-X`→`Z-X.`; generational `Loeblich III` not read as initials `I/V/X` excluded); particle/apostrophe gluing.
  6. The basionym `hasUpperWord` guard (a `(...)` with no upper-case WORD is NOT a basionym → captured as unparsed).
- **CodeInference:** two pins (rank-code; nom-note status via `ICN_STATUS`/`ICZN_STATUS` word-boundary patterns) then a vote tally; set only when exactly one distinct code voted. Only invoked from `Assemble.finish` when `code == null`. `applyRankCodeMismatch` runs unconditionally.
- **Assemble.finish:** the 19 ordered steps (investigation §4) — INDETERMINED placeholder NULLS the authorship; uninomial↔genus/infragenericEpithet moves; underscore-split; phrase-promotion; quoted-monomial re-wrap; `viralShape`→VIRUS; `INFRASPECIFIC_NAME`+ZOOLOGICAL→SUBSPECIES; suffix-based rank; blacklisted-epithets; unlikely-years. Order is load-bearing.
- **pendingYear timing:** applied to `combinationAuthorship.year` BEFORE `Assemble.finish` iff `!pending_year_from_publication`, AFTER iff `pending_year_from_publication` (exactly one fires). `pendingImprintYear` always applied AFTER. (These pending fields already exist on `ParseContext` from the StripAndStash slice.)
- **`stripAuthorshipMarkers`** (deferred from StripAndStash) is ported here (the aux-authorship path needs it) — a separate inline reimplementation reusing the StripAndStash pattern constants (investigation §4 of the StripAndStash map). If large, port the subset the aux path exercises + note.
- **`Rank` is still a partial stub (~49/117).** Assemble/CodeInference reference `Rank.getCode()` (68 code-carrying ranks), `isInfragenericStrictly`, `higherThan(SPECIES)`, SPECIES_AGGREGATE, SUBSPECIES, FAMILY/SUBFAMILY suffix ranks, etc. **This slice likely forces porting the FULL 117-constant `Rank`** (with each constant's `code`/`marker` + the ordinal predicates + `RankUtils.SUFFICES_RANK_MAP`). Do the full Rank as Task 1 (it unblocks the ordinal predicates Assemble needs).
- SPDX; crate 0.0.0; no new deps (all plain `regex`/`std`).
- **Working dir** `/Users/markus/code/gbif/name-parser-rust/`. Preamble: `export PATH="$HOME/.cargo/bin:$PATH"; [ -s "$HOME/.sdkman/bin/sdkman-init.sh" ] && source "$HOME/.sdkman/bin/sdkman-init.sh"`.

## Validation strategy

Extend the golden harness to the remaining fields → **full ParsedName parity**:
- **Authorship:** `combinationAuthorship.{authors,exAuthors,year,imprintYear}`, `basionymAuthorship.{…}`, `sanctioningAuthor` (nested objects — compare structurally; `authors`/`exAuthors` are ordered lists = positional).
- **`code`** (closed by CodeInference+Assemble+the StripAndStash pins already in place).
- **Flip the name-part residuals** `genus`/`uninomial`/`infragenericEpithet`/`specificEpithet` to `assert_eq!(_, 0)` (Assemble closes them). Also `rank`/`type`/`doubtful`/`phrase`/`state`/`unparsed` become validatable — add them, assert 0 (or a documented allowlist for any irreducible residual, e.g. the `replace_homoglyphs` stub's handful, or `RankUtils.SUFFICES_RANK_MAP` if deferred).
- Keep the StripAndStash-10 + name-part-3 + error-class asserts. The end state: **the entire `parsed` JSON object matches Java** (modulo documented stubs).

---

## Task 1: Full `Rank` (117 constants) + ordinal predicates

**Files:** `crates/nameparser/src/model/enums.rs` (expand `Rank`), maybe `model/rank.rs`.

- [ ] Port ALL 117 `Rank` constants from `Rank.java` in declaration order (ordinal order matters for the predicates), each with its `code: Option<NomCode>` and `marker: Option<&str>` (+ plural where present). Port the predicates Assemble/CodeInference/AuthorshipSplit/NameTokens use: `is_infrageneric_strictly`, `is_infraspecific`, `is_suprageneric`, `higher_than`/`lower_than` (ordinal compare), `is_species_or_below`, `get_major_rank`/MAJOR_RANKS, `is_restricted_to_code`/`get_code`, `is_uncomparable`, `is_legacy`, `is_family_group`, `is_genus_group`. Replace the ad-hoc `rank_is_infrageneric_strictly`/`rank_is_infraspecific` in authorship_split.rs/name_tokens.rs with the model's versions (make `Rank::code()` etc. the single source). Also port `RankUtils.SUFFICES_RANK_MAP` (suffix→rank) if Assemble step 14 needs it (else stub + note). Unit-test the predicates + a sample of code-carrying ranks. Commit `Phase 1 s4: full 117-constant Rank + ordinal predicates`.

## Task 2: AuthorshipParser::parse + AuthState

**Files:** Create `crates/nameparser/src/pipeline/authorship_parser.rs`. Java ref: `AuthorshipParser.java` (817 lines, **regex-free**).

- [ ] Port `AuthState` (8 fields) + `parse(tokens, from) -> AuthState` (phases A/B/C) + `parse_authors` (the 14-case token walk) + all 18 helpers (`invert_all`/`invert_author`/`format_initials`/`normalise_author_case`/`looks_like_surname`/`looks_like_initials`/`middle_initial_surname_follows`/`find_close`/`find_last_colon`/`append_author_words`/`is_year_disambiguator`/`AUTHOR_SUFFIXES`/`GENERATIONAL_SUFFIXES`/…). Preserve the quirks (Global Constraints 1–6). TDD FIRST from `CLAUDE.md`'s authorship examples + `names-with-authors.txt` test corpus + `NameParserImplTest` author cases (e.g. `Walker F`→`F.Walker`; `Balsamo M Fregni E`→`M.Balsamo, E.Fregni`; `H.da C.` + `Monteiro`; `Y.-j. Wang`→`Y.-j.Wang`; `Loeblich III`; `L.'t Mannetje`; `(L.) L., 1753`; `Storr, 1970 [1969]`; `L. : Fr.`). This is the most edge-case-dense stage — write many tests. Commit `Phase 1 s4: port AuthorshipParser::parse`.

## Task 3: CodeInference + Assemble + BlacklistedEpithets

**Files:** Create `crates/nameparser/src/pipeline/{code_inference,assemble,blacklisted_epithets}.rs`; copy `blacklist-epithets.txt` into the crate (`include_str!`).

- [ ] **BlacklistedEpithets:** `include_str!("../../resources/blacklist-epithets.txt")` (copy the file from `name-parser/src/main/resources/nameparser/blacklist-epithets.txt`) → `LazyLock<HashSet<&str>>`; `contains(epithet)` lowercases + looks up. Test resource parity (274 entries).
- [ ] **CodeInference:** `infer(ctx, authState)` (2 pins + vote tally), `code_from_nom_note(note)` (ICN_STATUS/ICZN_STATUS plain `regex`, ASCII-scoped `\b`), `apply_rank_code_mismatch(ctx)`. Unit-test the pins + votes (a `(Basionym) Recomb`→BOTANICAL; `(Author, 1901)`→ZOOLOGICAL; `Candidatus`→BACTERIAL; `nom. cons.`→BOTANICAL; contradicting votes→null).
- [ ] **Assemble::finish(ctx, authState):** the 19 ordered steps + helpers (`flag_unlikely_years`/`is_unlikely_year`/`flag_blacklisted_epithets`/`rank_from_suffix`/`rank_from_global_suffix`). Calls `code_inference::infer`/`apply_rank_code_mismatch`. Unit-test the key rewrites (INDETERMINED nulls authorship; underscore-split; INFRASPECIFIC_NAME+ZOOLOGICAL→SUBSPECIES; quoted-monomial re-wrap; blacklisted/unlikely-year flags). Commit `Phase 1 s4: port CodeInference + Assemble + BlacklistedEpithets`.

## Task 4: Wire the full Pipeline back-end + stripAuthorshipMarkers + full-parity gate

**Files:** `pipeline/mod.rs` (the back-end timeline); `pipeline/stripandstash.rs` (port `strip_authorship_markers`); `tests/parse_golden.rs` (extend to all fields + flip gates).

- [ ] Port `strip_authorship_markers(authorship, name)` (StripAndStash's aux reimplementation — investigation §4 of the StripAndStash map; reuses pattern constants, no `ctx`).
- [ ] Wire `Pipeline.run` steps 9–19 (investigation §2 table) after `classify`: embedded `parse`+`applyAuthorship`+`unparsedFrom`→PARTIAL; autonym mid-author (guarded); aux-authorship (`strip_authorship_markers`→tokenize→parse→applyAuthorship, sanctioning here); embedded sanctioning (last-write); codeState selection; pendingYear before Assemble (`!from_publication`); **`assemble::finish(&mut ctx, code_state)`**; pendingYear after (`from_publication`); pendingImprintYear; pendingSpecific/GenericAuthor → `specificAuthorship`/`genericAuthorship`.
- [ ] **Extend the harness** to the authorship fields + `code` + `rank`/`type`/`doubtful`/`phrase`/`state`/`unparsed`; **flip** the name-part residuals (`genus`/`uninomial`/`infragenericEpithet`/`specificEpithet`) to `assert_eq!(0)`. Run the corpus. **Expected: full parsed-field parity** — every field 0 mismatches (or a small documented allowlist: the `replace_homoglyphs` stub, any deferred `SUFFICES_RANK_MAP`). Triage every residual (author-normalization edge, code-vote edge, Assemble-step edge) and fix. Record the final per-field mismatch table. This is the milestone: **the Rust parser reproduces Java's full `ParsedName` over 8017 names.** Commit `Phase 1 s4: wire back-end + stripAuthorshipMarkers; full parsed-field parity gate`.

## Task 5: Status doc — Phase 1 core parity
- [ ] Record: full-parity result (per-field mismatch table, any allowlist + cause); what remains (the ~1,925 Java assertions as a regression-port, `replace_homoglyphs` table, any `SUFFICES_RANK_MAP` deferral); Phase 1 core is done → Phase 2 (native CLI + cross-validation). Commit.

## Self-Review
This is the coupled back-end (design §6.2). It realizes full parsed-field parity — the culmination of the golden method. Deferred + stated: the ~1,925 assertion port (regression hardening), `replace_homoglyphs` table, possibly `SUFFICES_RANK_MAP`. No fancy_regex needed anywhere. Type consistency: `AuthState` (Task 2) consumed by the Pipeline wiring + CodeInference (Task 4/3); `Rank` predicates (Task 1) consumed by Assemble/CodeInference (Task 3); the harness (Task 4) consumes the full model.
