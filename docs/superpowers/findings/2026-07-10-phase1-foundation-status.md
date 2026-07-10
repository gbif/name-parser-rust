# Phase 1 Foundation — Status

**Date:** 2026-07-10
**Scope:** Phase 1 foundation plan, Tasks 1–6 (model, wire format, pipeline skeleton, `Preflight`,
error-classification golden harness). Task 7 is this document.
**Plan:** `docs/superpowers/plans/2026-07-10-phase1-foundation.md`
**Design spec:** `docs/superpowers/specs/2026-07-09-name-parser-rust-design.md` (§6.2 core port,
§7 golden-corpus method)
**Prior:** `docs/superpowers/findings/2026-07-09-phase0-spike-findings.md` (Phase 0 spike —
recommendation was "proceed")
**HEAD:** `1f54ab4` (baseline for this slice was `38f8696`, the end of the Phase 0 spike)

## 1. Gate result: error-classification parity achieved

The foundation gate the plan set — **Rust rejects exactly the names Java rejects, with the same
`NameType`/`NomCode`** — is met over the full benchmark corpus:

```
8017 rows, 0 partition mismatches, 0 type/code mismatches
```

- Corpus: `testdata/benchmark-data.txt` (8018 lines, 1 header/comment line), the real GBIF
  benchmark set. Oracle regenerated from the Java shaded CLI jar
  (`name-parser-cli-4.2.0-SNAPSHOT-shaded.jar`, Java 21) into `testdata/expected-parse.jsonl`
  (git-ignored, regenerable — see the header comment of `tests/parse_golden.rs`).
- Composition: 4672 rows Java parses, 3345 rows Java rejects as errors. Error `type` breakdown:
  OTHER 3258, FORMULA 42, PLACEHOLDER 37, INFORMAL 8; of the OTHER rows, 3210 carry
  `code:"VIRUS"`.
- **Partition** (error vs. parsed) and, for error rows, **`type`/`code`** both match Java on all
  8017 rows. No allowlist was added or needed anywhere in this slice.
- One real mismatch surfaced during Task 6 and was root-caused and fixed, not allowlisted — see
  the Task 6 row in §2.

This proves error-classification parity. It does **not** prove full-parse parity: parsed-row
field *content* (genus, epithets, authorship, …) is not yet compared against Java at all — see
§4.

## 2. What was built (Tasks 1–6)

| Task | Commit(s) | Delivered |
|---|---|---|
| 1 | `03ccbe4` | `model::enums` — `NameType`, `NomCode`, `NamePart`, `State` (Gson-`.name()`-faithful serde); all 26 `Warnings` string constants transcribed verbatim from `Warnings.java`; `ParseError` (default message matches `UnparsableNameException`'s: `"Unparsable OTHER name: .."`). `Rank` stub (`Unranked` only). |
| 2 | `5b01446`, `b4996d4` | `model::name` — `Authorship`, `CombinedAuthorship`, `ParsedName` (Java's `ParsedName`/`ParsedAuthorship`/`CombinedAuthorship` hierarchy flattened into one 30-field struct: 16 own + 11 `ParsedAuthorship` + 3 `CombinedAuthorship`, in Gson field order). `Rank` stub grew to `{Unranked, Species, Subspecies}` (the two gate reference rows need real ranks). |
| 3 | `7bc6f0a`, `8e54d1f` | `unicode::normalize_quotes` (quote/apostrophe→ASCII folding) and `java_trim` (Java `String.trim()` — only strips ≤U+0020, unlike Rust's Unicode-whitespace-aware default); `pipeline::ParseContext`; the `pipeline::run` skeleton (3 of Java's 4 inline guards + `split_glued_phrase_name` + a `Preflight` stub). |
| 4 | `30bb8f3` | `viral::is_viral` — port of `ViralSuffix` (`GENUS` / `HIGHER` suffix regexes, 4 + 15 suffixes). |
| 5 | `742fd93`, `bc111b6` | `pipeline::preflight::run` — the full `Preflight` gate: 33 patterns, the 9-rule virus gate (`apply_virus_gate`), `looks_like_hybrid_formula`, and 8 supporting helpers. |
| 6 | `1f54ab4` | `tests/parse_golden.rs`, the corpus golden harness — plus a real fix: `pipeline::run` was missing Java's 4th inline guard (`hasLetter`, which in Java sits after `StripAndStash` and before `Tokenizer`). Added a pre-`StripAndStash` stand-in, sound in the direction that matters (stripping only ever removes letters, never adds them). This took the gate from 1 mismatch to 0 — fixed at the root cause, not allowlisted. |

By the end of Task 6, `pipeline::run` carries **all 4** of Java's inline guards (empty/overlong
name, overlong authorship — both from Task 3 — plus the `hasLetter` guard added in Task 6),
followed by `split_glued_phrase_name` and the full `Preflight` call, before falling through to a
`// TODO` for the still-unported downstream stages (§4).

Current suite at `HEAD`: **104 lib unit tests + 1 Phase-0 tokenizer golden test + 2 parse-golden
tests = 107, all green.**

## 3. Wire-format status

`ParsedName` / `Authorship` / `CombinedAuthorship` serialize through `serde_json` and were
validated **byte-identical** to the real Java Gson output (`new
GsonBuilder().disableHtmlEscaping().create()`; confirmed no custom `TypeAdapter` is registered
anywhere in the Java source) for both reference rows named in the plan:

- `Vulpes vulpes silaceus Miller, 1907` (row 1, printed in the plan's Reference section) —
  byte-exact.
- `Abies alba Mill.` (row 2, not printed in the plan text — generated directly from the Java jar
  and cross-checked against it) — byte-exact.

This validates the *shape*: field declaration order, omitted-vs-`null` handling, `[]` for
eagerly-initialized collections, enum `.name()` casing, and primitive-`boolean`-vs-boxed-`Boolean`
serialization. It does **not** yet validate populated real-world field values beyond these two
rows — that is exactly what parsed-field parity (§4, §6) will add.

## 4. What's stubbed / deferred

- **`Rank`**: only 3 of the real 117 Java constants are ported (`Unranked`, `Species`,
  `Subspecies` — count verified directly against `Rank.java`). The remaining 114 land in a
  dedicated rank-handling slice.
- **Downstream pipeline stages**: `Preflight` is the only stage actually running. Not yet
  ported: `StripAndStash`, `AuthorshipSplit`, `NameTokens`, `AuthorshipParser`, `CodeInference`,
  `Assemble`. (`Tokenizer` itself was already ported and golden-tested in the Phase 0 spike —
  0/8017 mismatches — and exists as `token.rs`, but isn't wired into `pipeline::run` yet either.)
  `pipeline::run`'s own `// TODO` comment names this same chain.
- **Parsed-field parity**: the Task 6 harness gates only the parsable/unparsable partition and
  error `type`/`code`. It does not compare genus, epithets, authorship, or any other field of a
  successfully parsed name against Java — that climbs stage-by-stage as the pipeline is built
  out, per the design spec's golden-corpus method (§7).
- **The ~1,925 Java fluent-assertion port** (`NameAssertion` → a Rust macro) is explicitly the
  final slice and hasn't started.

## 5. Parity-hardening items surfaced (tracked for later, immaterial for this corpus)

None of these caused a corpus mismatch — the golden harness would have caught it if they had.
Recorded here so they aren't rediscovered from scratch once a broader corpus (or the rank/
downstream-stage slices) exercises them:

1. **Regex Unicode-vs-ASCII default, only partially handled so far.** The Rust `regex` crate's
   `\s`/`\d`/`\b` shorthand classes and `(?i)` are Unicode-aware by default; the Java patterns
   ported so far mostly don't set `UNICODE_CHARACTER_CLASS`/`UNICODE_CASE`, i.e. are ASCII-only.
   The shorthand-class half of this is handled per-pattern (ASCII-scoped via `(?-u:…)` wherever
   Java doesn't set the Unicode flag; left Unicode-default where it does) — applied across all 33
   `Preflight` patterns plus `viral.rs`. The `(?i)` case-*fold* half is **not** yet ASCII-scoped
   anywhere: Rust's case-insensitive matching is Unicode simple-case-folding (e.g. long-s "ſ"→"s"),
   Java's bare `CASE_INSENSITIVE` is ASCII-only fold. Inert everywhere it currently occurs (every
   case-insensitive alternative ported so far is a pure-ASCII literal), but not a general fix —
   would need `(?i-u:…)` scoping (fiddlier wherever `\p{…}` also appears in the same pattern) once
   a case-insensitive pattern's alternatives include non-ASCII text.
2. **`is_letter`/`is_digit` remain approximations, not exact ports.** `is_alphabetic()` (standing
   in for Java's `Character.isLetter`) is a Unicode-property *superset* of Java's `L*`-only
   category (also matches Letter Numbers, `Other_Alphabetic` marks). The ASCII-only digit check is
   a *subset* of Java's Unicode `Nd` check (misses non-ASCII decimal digits). Both originate from
   the Phase 0 spike and remain latent/unexercised — the whole 8017-row corpus is Latin-script.
3. **The golden test skips vacuously if the oracle file is absent.** `testdata/expected-parse.jsonl`
   is git-ignored and must be regenerated from the Java jar; CI must do this explicitly, or the
   gate silently stops checking anything rather than failing. Same pre-existing pattern as the
   Phase 0 tokenizer golden test.
4. **`PR2_LIKE` is dead code in the Java source itself**, not a porting artifact: every string its
   regex can match already falls inside `PURE_ALPHANUM`'s broader alphabet, and both call sites
   require the same `hasDigit` condition, so `PR2_LIKE`'s branch is unreachable in the original
   Java too. Ported verbatim anyway — faithful port, not a "fix" of the Java side — and flagged
   here for whoever eventually maintains the Java source.

## 6. Next slice

**`StripAndStash`**, using `NOM_NOTE` (`StripAndStash.java:45-57`) as the stress pattern: it
combines a possessive quantifier, several `(?i:…)` inline-scoped case-folding groups nested inside
an outer `Pattern.UNICODE_CHARACTER_CLASS`-flagged pattern, and a multi-branch trailing lookahead —
making it simultaneously the worst case for the restructure-vs-`fancy-regex` decision and for
speed (already flagged as a Phase-0 carry-forward risk).

Alongside it: extend the golden harness from error-classification to **parsed-field parity** —
diffing genus/epithet/authorship/etc. content on the 4672 rows Java successfully parses, not just
the partition and `type`/`code` on all 8017.
