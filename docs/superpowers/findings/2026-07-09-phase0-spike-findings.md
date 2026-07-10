# Phase 0 Spike — Findings

**Date:** 2026-07-09
**Question:** Can the GBIF name parser port to Rust faithfully and faster (approach B)?

**Spike plan:** `docs/superpowers/plans/2026-07-09-phase0-spike.md`
**Design spec:** `docs/superpowers/specs/2026-07-09-name-parser-rust-design.md` (§11, "To validate in the spike")

## 1. Port faithfulness (tokenizer golden diff)

- Corpus size: **8017 names** — the real benchmark corpus
  (`name-parser-cli/data/benchmark-data.txt`, 8018 lines minus 1 header/comment line),
  copied verbatim into `testdata/benchmark-data.txt`.
- Mismatches after triage: **0**. The Rust tokenizer matched the Java oracle's token
  stream on all 8017 names on the very first run, before any change to `token.rs`. No
  helper fixes were needed and no allowlist mechanism was added or required. This result
  was **independently re-verified by the reviewer**, who regenerated the Java oracle
  from scratch in a separate pass and got a byte-identical `expected-tokens.tsv`.
- Irreducible Unicode/semantics gaps found: **none, on this corpus** — the golden diff
  came back clean, and five specific trap categories the Java source itself flags
  (non-breaking space U+00A0, ideographic space U+3000, decomposed combining marks
  U+0301/U+030C, the replacement character U+FFFD, and the non-breaking hyphen U+2011)
  were individually spot-checked against the oracle and confirmed to agree in both
  languages. Two approximations remain **latent but unexercised** by this corpus, and
  are recorded here as known risk rather than "fixed":
  - `is_letter` (Rust `char::is_alphabetic`, the Unicode `Alphabetic` property) is
    **broader** than Java's `Character.isLetter` (`L*` categories only): it additionally
    accepts Letter Numbers (e.g. U+2160 "Ⅰ" Roman numeral one), U+00AA "ª" feminine
    ordinal indicator, and `Other_Alphabetic` combining marks used in Hebrew/Greek
    vowel-pointing (e.g. U+05B0, U+0345). Phase 1 handling: leave as-is; revisit only if
    a future corpus includes non-Latin-script author citations, at which point adopt an
    exact Unicode-category crate rather than hand-approximating (see §5).
  - `is_digit` (Rust `char::is_ascii_digit`) is **narrower** than Java's
    `Character.isDigit` (Unicode `Nd` category): it misses non-ASCII decimal digits such
    as Arabic-indic (U+0669), Devanagari (U+0966), and fullwidth (U+FF10). Phase 1
    handling: same as above — a corpus-driven decision, not a spike-time guess.
  - Neither gap is exercised anywhere in the 8017-name corpus (Latin-script binomials
    and author strings throughout), so nothing was allowlisted; both are flagged purely
    as known-latent risk for a broader future corpus.
- Verdict: **the tokenizer is a faithful, near-mechanical port** of the Java original.

**Cross-cutting finding surfaced elsewhere (not by this golden diff, but important enough
to flag here since it's a Unicode/semantics gap of the same family):** the regex-porting
task (§2/§5 below) found that the Rust `regex` crate's shorthand classes (`\s`, `\d`,
`\b`) and `(?i)` are Unicode-aware **by default**, while the equivalent Java patterns
are ASCII-only (none of them set `UNICODE_CHARACTER_CLASS`/`UNICODE_CASE`). This means
verbatim regex ports can silently *broaden* matching relative to Java, in the opposite
direction from the tokenizer's `is_digit` narrowing above. Concretely demonstrated: the
ported `SIC` pattern strips "(sic)" across a non-breaking space (U+00A0), and
`PUBLISHED_PAGE`'s `\d+` captures full-width digits — Java's originals do neither. This
was **not** caught by unit tests, only by deliberate manual probing, and it affects every
regex pattern in the pipeline, not just these two. See §5, requirement 1.

## 2. Lookaround pain (CORRIG probe)

- **Restructured-onto-`regex`:** it worked, but was fiddlier than the brief's own
  starting code initially reflected. The bracketed CORRIG alternative
  (`\s*[(\[]\s*corrig\.?\s*[)\]]`) needed no lookaround at all and ported trivially. The
  bare alternative — Java's `(?<=\s)corrig\.?(?=\s|$)` — required capturing the boundary
  characters and re-inserting them (`${1}${2}`) instead of asserting them with zero
  width. That splice mechanic itself was simple once the capture groups were right, but
  getting them right surfaced two real gotchas:
  1. **Call-site harness dependence (the deeper lesson).** Testing the bare pattern in
     isolation is a trap. On its own, Java's `(?<=\s)corrig\.?(?=\s|$)` cannot match at
     start-of-input (a lookbehind needs a real preceding character), which *suggests* Java
     would leave a leading "corrig. Rest" untouched. But that is **not** how Java uses it:
     both real call sites (`StripAndStash.java:432` and `:1117`) apply CORRIG with a
     leading-space pad — `CORRIG.matcher(" " + s)`, commented *"prepend a space so a leading
     'corrig.' also matches"* — plus a whitespace-collapse and trim. So the real pipeline
     **does strip** a leading/standalone marker (`"corrig. Peters, 1878"` → `"Peters, 1878"`;
     `"corrig."` → `""`). The spike's first pass drew the opposite conclusion by validating
     the isolated pattern (via fancy-regex as oracle); the final whole-branch review caught
     it against the actual Java source. The probe functions now replicate the pad + collapse
     + trim harness, so both variants strip leading markers. The lesson: **lookaround
     faithfulness depends on the call-site harness, not just the pattern** — which is exactly
     why §5.2 (a corpus-level golden diff against the real Java pipeline output, not unit
     tests on isolated patterns) is a hard Phase-1 requirement. With the pad in place the
     `(\s)`-vs-`(^|\s)` boundary question is moot — the pad guarantees a preceding whitespace
     character, so `(\s)corrig…` is correct.
  2. **Shared-boundary non-overlap.** `replace_all`'s matches are non-overlapping, so
     they cannot reuse a consumed boundary character. Two **adjacent** bare "corrig."
     tokens sharing one whitespace boundary therefore diverge from Java's zero-width
     semantics — e.g. "a b corrig. corrig. c": Java strips both occurrences, the
     restructured pattern strips only the first. This is **intrinsic** to the
     capture-and-splice technique (unlike gotcha 1, it is not fixable per-pattern with
     more careful boundary derivation) and is documented directly in the source
     (`crates/nameparser/src/regexes.rs`).
- **`fancy-regex` verbatim:** worked as a **zero-friction drop-in**. The Java pattern
  was used completely unmodified; the `find_iter` / `Result`-wrapped-`Match` API matched
  the brief's code exactly and compiled and passed on the first try. The only ergonomic
  cost is a manual splice loop in place of `regex`'s `replace_all` sugar, because
  `fancy-regex` matches are `Result`-wrapped (to account for its backtracking limit)
  rather than infallible.
- **Recommended default strategy for the ~29 lookaround/backreference patterns:**
  restructure onto the plain `regex` crate only where the lookaround is mechanical,
  fixed-width, and **provably non-adjacent**; use `fancy-regex` for anything with
  possible adjacency, variable width, or backreferences. Reasoning: restructuring keeps
  the dependency graph smaller and stays linear-time, but each restructured boundary
  carries a proof obligation — re-derive what the assertion actually tests,
  independently per side, rather than pattern-matching on the shape of the original
  regex — and the capture-and-splice technique has the adjacency limitation above
  permanently baked in, with no per-pattern fix available. `fancy-regex` is a faithful,
  low-friction escape hatch for those cases, and because the pipeline bounds input
  length (`MAX_LENGTH`), its backtracking is not a practical ReDoS risk here.
- **Any pattern the `regex` crate rejected outright:** none. All 5 lookaround-free
  anchored patterns (`SIC`, `AGGREGATE`, `IN_PRESS`, `PUBLISHED_PAGE`, and `TAX_NOTE`)
  compiled in the `regex` crate on the first attempt — including `TAX_NOTE`'s inner
  `(?-i:…)` scoped case-sensitivity toggle, syntax that might plausibly have needed a
  `fancy-regex` fallback but turned out to be directly supported. No `fancy-regex`
  fallback was needed for any of these five.

## 3. Speed (criterion vs Java)

- tokenize: **0.3215 µs/name** (criterion mid estimate; `tokenize_corpus` benchmark,
  2.5775 ms per iteration over the full 8017-name corpus; range 0.3194–0.3239 µs/name
  across the low–high estimate).
- regex batch (4 patterns — `SIC`, `AGGREGATE`, `TAX_NOTE`, `PUBLISHED_PAGE`, each one
  `replace_all` pass): **0.1447 µs/name** (criterion mid estimate; `regex_batch_corpus`
  benchmark, 1.1597 ms per iteration; range 0.1437–0.1457 µs/name).
- Java full parse reference: **28 µs/name** (whole pipeline: tokenize + all regex
  normalization + grammar/state-machine classification + object construction).
- Extrapolated expectation for the full Rust pipeline: these two numbers are
  per-**component** measurements, not a full-parse ratio, and must **not** be read as
  "Rust is 87x/194x faster than Java" — that would compare a sliver of the future Rust
  pipeline against Java's total. What they legitimately show: tokenizing and running a
  representative 4-pattern regex batch over a name together cost about **0.47 µs/name**
  (0.3215 + 0.1447), under 2% of Java's 28 µs/name full-parse budget — i.e. both
  measured components are comfortably sub-microsecond, leaving large headroom for the
  remaining, unported stages (the rest of the ~29+ regex/strip patterns, grammar/
  state-machine classification, and object construction) to fit within, or beat, the
  28 µs/name budget. Whether they actually will is not yet known from this spike alone:
  a true full-pipeline Rust-vs-Java comparison requires the complete port and is
  explicitly deferred to Phase 1.

## 4. Recommendation

- [x] **Proceed to Phase 1 (faithful whole-pipeline port).**
- [ ] Proceed, but revise the design because: not applicable — no design revision is
  called for; see rationale below.
- [ ] Stop / reconsider because: not applicable — no stop condition was found; see
  rationale below.

**Rationale:** the port is provably faithful (0/8017 tokenizer-golden-diff mismatches,
independently re-verified by the reviewer from a from-scratch oracle regeneration); the
clean anchored regexes port with zero rejections, including a syntax construct
(`(?-i:…)`) that might plausibly have needed a fallback; the lookaround problem is
solved with a validated two-pronged strategy (restructure where safe, `fancy-regex`
elsewhere); and component-level speed is excellent, with both measured components
comfortably sub-microsecond and large headroom remaining under the 28 µs/name Java
full-parse budget. Nothing found in this spike constitutes a reason to change the
approach-B design or to stop.

## 5. Carry-forward risks for Phase 1

1. **Decide ASCII-scoping vs. Unicode-broadening for every shorthand class.** This is
   the spike's headline cross-language finding (§1/§2): the Rust `regex` crate's `\s`,
   `\d`, `\b`, and `(?i)` are Unicode-aware by default, while the Java patterns (none of
   which set `UNICODE_CHARACTER_CLASS`/`UNICODE_CASE`) are ASCII-only, so verbatim ports
   silently broaden matching (demonstrated: `SIC` strips "(sic)" across a non-breaking
   space; `PUBLISHED_PAGE` captures full-width digits). Phase 1 must explicitly choose,
   per pattern or globally, between ASCII-scoping the classes (e.g. `(?-u:\s)`,
   `(?-u:\d)`) to match Java exactly, or consciously accepting the broader Unicode
   matching as a robustness improvement — but the choice must be made deliberately, not
   left as an accidental side effect of a verbatim port.
2. **Add a corpus-level golden diff for the regex/strip stages**, reusing the Task 3
   oracle method (Java-side dump vs. Rust output, diffed over the full benchmark
   corpus). Unit tests did not catch the Unicode-broadening gap above; only corpus-scale
   comparison against the Java oracle is likely to catch this class of gap reliably
   across all ~29+ patterns.
3. **Use `fancy-regex` for lookaround patterns with adjacency, variable width, or
   backreferences**; restructure onto the plain `regex` crate only for mechanical,
   fixed-width, provably-non-adjacent cases (§2). Treat every restructuring as a proof
   obligation per boundary, not a syntactic transformation.
4. **CI must generate (or commit) the tokenizer oracle** (`testdata/expected-tokens.tsv`)
   before the golden test can be relied on as a permanent regression gate — it currently
   skips vacuously (passes without comparing anything) if the oracle file is absent,
   which would silently defang the gate in any environment lacking the Java toolchain.
5. **Port and benchmark `NOM_NOTE` (`StripAndStash.java:45-57`) early** in Phase 1 — it
   is the heaviest possessive-quantifier-plus-lookahead pattern in the source, making it
   simultaneously the worst case for the restructure-vs-`fancy-regex` decision (item 3)
   and for speed.
6. **Decide exact `is_letter`/`is_digit` semantics** (Java's `Character.isLetter`/
   `isDigit`, i.e. Unicode `L*`/`Nd` categories) rather than continuing with the current
   approximations, adopting a Unicode-category crate (e.g. `unicode-general-category`)
   if a future, broader corpus exercises non-Latin-script input (§1).
