# Phase 1 Foundation — model + full-parse golden harness + Preflight

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lay Phase 1's foundation: the Rust `ParsedName` model (Gson-wire-faithful), the pipeline skeleton, the first stage (`Preflight`), and a **full-parse golden-diff harness** against the real Java CLI output — gated on **error-classification parity** (Rust rejects exactly the names Java rejects, with the same `NameType`/`NomCode`).

**Architecture:** Extend the existing `nameparser` crate (which already has a faithful `tokenizer` + some regex patterns). Add a `model` module (structs + enums serialized to match Java's Gson output byte-for-byte), a `pipeline` module (`Pipeline::run` orchestration skeleton + `ParseContext` + `Preflight`), a `viral` helper, and a public `parse()` entry point. The golden harness generates `expected.jsonl` from the Java shaded jar's `parse` command over the benchmark corpus, runs Rust `parse()` over the same inputs, and diffs. This slice validates only the parsable/unparsable partition + error `type`/`code`; parsed-field parity climbs in later per-stage slices.

**Tech Stack:** Rust (edition 2021), `regex` + `fancy-regex` (already deps), `serde` + `serde_json` (new deps, for wire-faithful serialization). Oracle from the Java CLI shaded jar (Java 21, already built at `name-parser-cli/target/name-parser-cli-4.2.0-SNAPSHOT-shaded.jar`).

## Global Constraints

- **Faithful port; Java is the oracle.** Every ported unit must reproduce Java behaviour. Behaviour changes are recorded as explicit divergences, never applied silently. Reference source: `/Users/markus/code/gbif/name-parser/` (paths below are relative to it).
- **Per-pattern regex-flag fidelity (the Phase-0 headline finding).** The Rust `regex`/`fancy-regex` crates make `\s \d \w \b` and `(?i)` **Unicode-aware by default**. For EACH ported Java pattern, replicate its `Pattern` flags: if the Java pattern is compiled with `Pattern.UNICODE_CHARACTER_CLASS`, keep the crate's default (Unicode) shorthand classes; if it is NOT, wrap the shorthand classes in `(?-u:…)` to force ASCII, matching Java's ASCII default. `\p{Lu}`/`\p{Ll}` are Unicode in both and stay as-is. `Pattern.CASE_INSENSITIVE` → `(?i)`.
- **Wire format = Gson, exact.** The golden harness compares against Gson output produced by `new GsonBuilder().disableHtmlEscaping().create()`. Rules, verbatim from the observed output:
  - Compact JSON (no spaces after `:`/`,`).
  - **Null / unset fields are OMITTED**, not emitted as `null`.
  - **Key order = Java field declaration order, most-derived class first**: `ParsedName`'s own 16 fields → `ParsedAuthorship`'s 11 → `CombinedAuthorship`'s 3. Rust must reproduce this order via ordered `serde` struct fields (a struct, never a map).
  - Enums serialize as their `.name()` (`"rank":"SUBSPECIES"`).
  - Eagerly-initialized empty collections serialize as `[]` (`authors`, `exAuthors`, `warnings`); no-default collections are omitted when unset (`notho`, `epithetQualifier`).
  - Primitive `boolean` fields always serialize incl. `false` (`candidatus`, `extinct`, `doubtful`, `manuscript`). Boxed `Boolean` `originalSpelling` is omitted when null.
  - `year`/`imprintYear` are **Strings**; `publishedInYear` is a bare **number** (Integer).
  - `warnings` is a Java `HashSet` — its array order is NOT insertion order. **Diff `warnings` as a set** (sort both sides), never positionally.
- **This slice's gate is error-classification only.** For each corpus row, Java emits either a `parsed` object or an `error{type,code,message}`. Rust must match the **error vs parsed partition** and, for error rows, the `type` and `code` (message text is NOT gated this slice). Parsed-row field content is out of scope here.
- **Rank is minimally stubbed this slice.** Port the full 117-constant `Rank` in the later rank-handling slice; here define only `Rank::Unranked` plus any constants Preflight/ViralSuffix/tests reference, enough for the model to compile and serialize. Note the stub in the code.
- **License:** Apache-2.0; `// SPDX-License-Identifier: Apache-2.0` header on every Rust source file. Crate stays `0.0.0`, edition 2021.
- **Working dir** for all commands: repo root `/Users/markus/code/gbif/name-parser-rust/`. Toolchain preamble for every bash invocation: `export PATH="$HOME/.cargo/bin:$PATH"; [ -s "$HOME/.sdkman/bin/sdkman-init.sh" ] && source "$HOME/.sdkman/bin/sdkman-init.sh"`.

## Reference: exact JSONL shape (authoritative serde target)

Successful parse (row 1 of the CLI over `Vulpes vulpes silaceus Miller, 1907`):
```json
{"line":1,"input":"Vulpes vulpes silaceus Miller, 1907","parsed":{"rank":"SUBSPECIES","code":"ZOOLOGICAL","genus":"Vulpes","specificEpithet":"vulpes","infraspecificEpithet":"silaceus","candidatus":false,"type":"SCIENTIFIC","extinct":false,"doubtful":false,"manuscript":false,"state":"COMPLETE","warnings":[],"combinationAuthorship":{"authors":["Miller"],"exAuthors":[],"year":"1907"},"basionymAuthorship":{"authors":[],"exAuthors":[]}}}
```
Error rows (the three shapes this slice gates on):
```json
{"line":5,"input":"Tobacco mosaic virus","error":{"type":"OTHER","code":"VIRUS","message":"Unparsable OTHER name: Tobacco mosaic virus"}}
{"line":6,"input":"Homo sapiens x Homo neanderthalensis","error":{"type":"FORMULA","message":"Unparsable FORMULA name: Homo sapiens x Homo neanderthalensis"}}
{"line":7,"input":"BOLD:ACW2100","error":{"type":"OTHER","message":"Unparsable OTHER name: BOLD:ACW2100"}}
```
`ParsedName` own fields, in serialize order: `rank, code, uninomial, genus, genericAuthorship, infragenericEpithet, specificEpithet, specificAuthorship, infraspecificEpithet, cultivarEpithet, phrase, candidatus, notho, originalSpelling, epithetQualifier, type`. Then `ParsedAuthorship`: `extinct, taxonomicNote, nomenclaturalNote, publishedIn, publishedInYear, publishedInPage, unparsed, doubtful, manuscript, state, warnings`. Then `CombinedAuthorship`: `combinationAuthorship, basionymAuthorship, sanctioningAuthor`. `Authorship`: `authors, exAuthors, year, imprintYear`.

Enums (verbatim): `NameType` = SCIENTIFIC, FORMULA, INFORMAL, PLACEHOLDER, OTHER. `NomCode` = BACTERIAL, BOTANICAL, CULTIVARS, PHYTO, VIRUS, ZOOLOGICAL, PHYLO. `NamePart` = GENERIC, INFRAGENERIC, SPECIFIC, INFRASPECIFIC. `State` = COMPLETE, PARTIAL, NONE.

---

## File Structure

| File | Responsibility |
|---|---|
| `crates/nameparser/src/model/enums.rs` | `NameType`, `NomCode`, `NamePart`, `State`, `Rank` (stub), `Warnings` string consts — with `.name()`-matching serde |
| `crates/nameparser/src/model/name.rs` | `Authorship`, `CombinedAuthorship`, `ParsedAuthorship`, `ParsedName` structs; ordered serde matching Gson |
| `crates/nameparser/src/model/mod.rs` | re-exports; `ParseError` (the `UnparsableNameException` equivalent) |
| `crates/nameparser/src/pipeline/context.rs` | `ParseContext` (shared mutable state) |
| `crates/nameparser/src/pipeline/preflight.rs` | `Preflight::run` — the 33-pattern gate |
| `crates/nameparser/src/pipeline/mod.rs` | `Pipeline::run` skeleton (guards → normalize → split-glued → Preflight; downstream stages TODO) |
| `crates/nameparser/src/viral.rs` | `viral::is_viral` (ViralSuffix port; Preflight dependency) |
| `crates/nameparser/src/unicode.rs` | `normalize_quotes` (UnicodeUtils subset used by Pipeline) |
| `crates/nameparser/src/lib.rs` | public `parse(...)`; module wiring |
| `crates/nameparser/tests/parse_golden.rs` | error-classification golden diff vs Java `expected.jsonl` over the corpus |
| `testdata/expected-parse.jsonl` | generated Java oracle (git-ignored) |

---

## Task 1: Model enums + Warnings + minimal Rank

**Files:** Create `crates/nameparser/src/model/enums.rs`, `crates/nameparser/src/model/mod.rs`; modify `crates/nameparser/src/lib.rs`, `crates/nameparser/Cargo.toml`.

**Interfaces produced:** `model::{NameType, NomCode, NamePart, State, Rank}` (enums with `#[derive(Serialize)]` rendering `SCREAMING_SNAKE` `.name()` values), `model::warnings` (module of `pub const … : &str`), `model::ParseError { type_: NameType, code: Option<NomCode>, name: String, message: String }`.

- [ ] **Step 1: Add serde deps.** In `crates/nameparser/Cargo.toml` add under `[dependencies]`: `serde = { version = "1", features = ["derive"] }` and `serde_json = "1"`. Run `export PATH="$HOME/.cargo/bin:$PATH"; cargo build -p nameparser` — expect clean.

- [ ] **Step 2: Write the enums (failing test first).** Create `crates/nameparser/src/model/enums.rs` with the four wire enums, the `Rank` stub, and the `Warnings` consts. Each wire enum derives `Serialize` with `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` so variants render as Java's `.name()`. Include a test asserting `serde_json::to_string(&NameType::Scientific).unwrap() == "\"SCIENTIFIC\""` and one for each enum's first/last variant, and `NomCode::Zoological → "ZOOLOGICAL"`. For `Rank`, define at minimum `Unranked` (serialize `"UNRANKED"`) with a `// STUB: full 117-constant port deferred to the rank slice` note; verify `Rank::Unranked → "UNRANKED"`.

```rust
// SPDX-License-Identifier: Apache-2.0
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NameType { Scientific, Formula, Informal, Placeholder, Other }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NomCode { Bacterial, Botanical, Cultivars, Phyto, Virus, Zoological, Phylo }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NamePart { Generic, Infrageneric, Specific, Infraspecific }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum State { Complete, Partial, None }

// STUB: full 117-constant port deferred to the rank-handling slice. Only variants
// referenced by Preflight/ViralSuffix/the skeleton + Unranked default exist for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Rank { Unranked }

pub mod warnings {
    // 26 String constants from Warnings.java — VALUES (not Java constant names).
    // NB HOMOGLYHPS is the Java constant name's typo; its value is "homoglyphs replaced".
    pub const LONG_NAME: &str = "name too long";
    // … (port the remaining 25 from name-parser-api/.../api/Warnings.java, verbatim values)
}
```
Port the remaining warning values verbatim from `name-parser-api/src/main/java/org/gbif/nameparser/api/Warnings.java` (read the string literal each constant is assigned). Only `LONG_NAME` is used by the skeleton this slice; the rest are needed later but cheap to define now.

- [ ] **Step 3: Run to verify pass.** `cargo test -p nameparser --lib model::enums` → PASS.

- [ ] **Step 4: ParseError.** Create `crates/nameparser/src/model/mod.rs`: `pub mod enums; pub mod name;` (name added in Task 2 — add an empty stub file so it compiles, or add `name` in Task 2 and gate this). Define `ParseError`:
```rust
// SPDX-License-Identifier: Apache-2.0
pub mod enums;
pub mod name;
pub use enums::*;
use enums::{NameType, NomCode};

/// Rust equivalent of Java UnparsableNameException (type/code/name/message).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError { pub type_: NameType, pub code: Option<NomCode>, pub name: String, pub message: String }
impl ParseError {
    pub fn new(type_: NameType, code: Option<NomCode>, name: impl Into<String>) -> Self {
        let name = name.into();
        let message = format!("Unparsable {type_:?} name: {name}"); // NB match Java exactly — see below
        Self { type_, code, name, message }
    }
}
```
**Faithfulness note:** Java's default message is `"Unparsable " + type + " name: " + name` where `type` is the enum `.toString()` = `.name()` (UPPERCASE, e.g. `OTHER`). Rust `{type_:?}` gives `Other`. Fix the message to use the uppercase `.name()` form (reuse the serde name or a `fn name(&self)->&str`), so it reads `Unparsable OTHER name: …`. Add a test: `ParseError::new(NameType::Other, None, "BOLD:ACW2100").message == "Unparsable OTHER name: BOLD:ACW2100"`.

- [ ] **Step 5: Wire + build.** In `lib.rs` add `pub mod model;`. `cargo test -p nameparser --lib` green, pristine. Commit: `Phase 1: model enums, Warnings, ParseError`.

---

## Task 2: The name model structs (Gson-wire-faithful)

**Files:** Create `crates/nameparser/src/model/name.rs`.

**Interfaces produced:** `model::{Authorship, CombinedAuthorship, ParsedAuthorship, ParsedName}` with `Serialize` producing byte-identical JSON to Java's Gson (field order + omission per Global Constraints). `ParsedName::default()` seeds `rank=Unranked, type=Scientific, state=Complete` (matching `ParseContext`'s seeding).

- [ ] **Step 1: Write the wire-shape tests first.** Add tests that build a `ParsedName` matching the reference rows and assert `serde_json::to_string` equals the exact `parsed` object substring. Use the two simplest reference cases: `Abies alba Mill.` (row 2) and `Vulpes vulpes silaceus Miller, 1907` (row 1). Copy the expected `parsed` JSON verbatim from the Reference section. This test is the wire-format contract.

- [ ] **Step 2: Run to verify fail.** `cargo test -p nameparser --lib model::name` → FAIL (types undefined).

- [ ] **Step 3: Implement the structs.** Field order MUST match the Reference. Use `#[serde(skip_serializing_if = "Option::is_none")]` on every nullable field; keep primitive bools always-serialized; give `authors`/`exAuthors`/`warnings` `#[serde(...)]` so empty vecs still emit `[]` (default serde behaviour — they serialize as `[]`, good). Represent `warnings` as `Vec<String>` (we insert in a deterministic order and the harness sorts before diffing). Represent `year`/`imprintYear` as `Option<String>`; `publishedInYear` as `Option<i32>`; `notho` as `Option<Vec<NamePart>>` (omit when None, else `["SPECIFIC"]`); `epithetQualifier` as `Option<BTreeMap<NamePart,String>>` serialized with `NamePart` keys as their `.name()`. `originalSpelling` as `Option<bool>`.

Sketch (fill every field per the Reference order; this shows the pattern):
```rust
// SPDX-License-Identifier: Apache-2.0
use serde::Serialize;
use crate::model::enums::{NameType, NomCode, NamePart, State, Rank};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Authorship {
    pub authors: Vec<String>,
    #[serde(rename = "exAuthors")] pub ex_authors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub year: Option<String>,
    #[serde(rename = "imprintYear", skip_serializing_if = "Option::is_none")] pub imprint_year: Option<String>,
}
impl Authorship { pub fn exists(&self) -> bool { !self.authors.is_empty() || !self.ex_authors.is_empty() || self.year.is_some() } }

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CombinedAuthorship {
    #[serde(rename = "combinationAuthorship")] pub combination: Authorship,
    #[serde(rename = "basionymAuthorship")] pub basionym: Authorship,
    #[serde(rename = "sanctioningAuthor", skip_serializing_if = "Option::is_none")] pub sanctioning_author: Option<String>,
}
// ParsedAuthorship + ParsedName: flatten the inheritance into ONE ParsedName struct whose
// serde field order is: [ParsedName-own 16] then [ParsedAuthorship 11] then [CombinedAuthorship 3].
// (Java's Gson walks most-derived first; a single flat struct in that field order reproduces it.)
```
**Key subtlety:** Java's class hierarchy `ParsedName : ParsedAuthorship : CombinedAuthorship` serializes most-derived-first. Model it as a **single flat `ParsedName` struct** with fields in exactly this order: the 16 ParsedName fields, then the 11 ParsedAuthorship fields, then `combinationAuthorship`, `basionymAuthorship`, `sanctioningAuthor`. `genericAuthorship`/`specificAuthorship` are `Option<CombinedAuthorship>`.

- [ ] **Step 4: Run to verify pass.** `cargo test -p nameparser --lib model::name` → PASS: the serialized JSON equals the reference `parsed` objects byte-for-byte. If any field's presence/order/omission differs, fix the struct until byte-identical. Commit: `Phase 1: ParsedName model with Gson-faithful serde`.

---

## Task 3: Unicode normalize + ParseContext + Pipeline skeleton

**Files:** Create `crates/nameparser/src/unicode.rs`, `crates/nameparser/src/pipeline/context.rs`, `crates/nameparser/src/pipeline/mod.rs`; modify `lib.rs`.

**Interfaces produced:** `unicode::normalize_quotes(&str) -> String`; `pipeline::ParseContext`; `pipeline::run(name, authorship, rank, code) -> Result<ParsedName, ParseError>` (skeleton: the 3 inline guards + normalize + split-glued + Preflight call [Preflight added Task 5; stub it as a no-op `fn run` until then], returning a seeded `ParsedName` for names that pass — downstream stages are `// TODO: later slices`); public `parse(...)` in `lib.rs` delegating to `pipeline::run`.

- [ ] **Step 1: `normalize_quotes`.** Port the quote/apostrophe folding subset of `UnicodeUtils.normalizeQuotes` (read `name-parser-api/src/main/java/org/gbif/nameparser/util/UnicodeUtils.java`; port only `normalizeQuotes`, not the homoglyph table). Test: a few smart-quote inputs fold to ASCII `'`/`"`.

- [ ] **Step 2: `ParseContext`.** Port the fields from `ParseContext.java` (Global Constraints Reference lists them): immutable `original, authorship_input, requested_rank, requested_code`; mutable `working, tokens, name, pending_unparsed, aggregate, viral_shape, pending_year, pending_year_from_publication, quoted_monomial, mid_author_from/to (i32, default -1), pending_imprint_year, pending_specific_author, pending_generic_author`. Constructor seeds `name.rank`/`name.code`/`name.type = Scientific`/`name.state = Complete`.

- [ ] **Step 3: Pipeline skeleton + the 3 inline guards (test first).** Port `Pipeline.run`'s guard clauses exactly (`Pipeline.java:43-57`): null/empty→`ParseError(OTHER)`, `len > 1000`→`ParseError(OTHER)`, `authorship len > 1000`→`ParseError(OTHER)`; then `normalize_quotes` both; build `ParseContext`; `LONG_NAME` warning over 250; `split_glued_phrase_name` (port `Pipeline.splitGluedPhraseName` + its `GLUED_PHRASE` pattern — `UNICODE_CHARACTER_CLASS`, so keep Unicode classes); call `Preflight::run` (no-op stub until Task 5); then `// TODO downstream stages` — return `ctx.name`. Tests: empty string and a 1001-char string both `Err(OTHER)`; a normal binomial returns `Ok` with a seeded name (fields mostly empty this slice).

- [ ] **Step 4: Build + commit.** `cargo test -p nameparser` green. Commit: `Phase 1: unicode normalize, ParseContext, Pipeline skeleton`.

---

## Task 4: ViralSuffix (`is_viral`)

**Files:** Create `crates/nameparser/src/viral.rs`. Java ref: `name-parser/src/main/java/org/gbif/nameparser/pipeline/ViralSuffix.java`.

**Interfaces produced:** `viral::is_viral(word: &str) -> bool` (used by Preflight's virus gate for the clean-ICTV-binomial bucket).

- [ ] **Step 1:** Read `ViralSuffix.java`; write tests from its own behaviour (e.g. `Lausannevirus`, `-viridae` family suffixes → true; a plain genus → false) BEFORE porting.
- [ ] **Step 2:** Run → fail. **Step 3:** Port `is_viral` (apply the per-pattern flag rule). **Step 4:** Run → pass. **Step 5:** Commit `Phase 1: port ViralSuffix::is_viral`.

---

## Task 5: Preflight

**Files:** Create `crates/nameparser/src/pipeline/preflight.rs`; wire the real call in `pipeline/mod.rs`. Java ref: `name-parser/src/main/java/org/gbif/nameparser/pipeline/Preflight.java` (full file — 441 lines, 33 patterns, `run`, `applyVirusGate`, `looksLikeHybridFormula`, helpers).

**Interfaces produced:** `preflight::run(original: &str, ctx: &mut ParseContext) -> Result<(), ParseError>` — returns `Ok(())` to let parsing proceed, `Err(ParseError)` to reject (mirrors Java's throw-or-return-silently).

- [ ] **Step 1: Write behavioural tests first** from the Reference error rows + `name-parser/src/test/resources/{viruses,otu,placeholder,hybrids,other}.txt` corpora. At minimum, one asserting each category and its `type`/`code`: `Tobacco mosaic virus`→Err(OTHER, Some(VIRUS)); `Homo sapiens x Homo neanderthalensis`→Err(FORMULA, None); `BOLD:ACW2100`→Err(OTHER, None); `Iteaphila-group`→Err(INFORMAL); `?? junk`→Err(OTHER); a placeholder→Err(PLACEHOLDER); and a clean binomial (`Abies alba`)→Ok(()). Also the two virus rescues: `Prion vittatus`→Ok (soft genus, no hard virus) and `Forcipomyia flavirustica Remm, 1968` style zoological-binomial override→Ok.

- [ ] **Step 2: Run → fail.**

- [ ] **Step 3: Port Preflight.** Transcribe the 33 `Pattern` constants into `LazyLock<Regex>` (or `fancy_regex` only where a pattern needs lookaround — check each; most are plain), **applying the per-pattern flag rule** (Global Constraints): patterns with `Pattern.UNICODE_CHARACTER_CLASS` keep default Unicode classes; those without ASCII-scope `\s\d\b\w` via `(?-u:…)`; `CASE_INSENSITIVE`→`(?i)`. Port `run`'s control flow **in the exact order** of the Java method (empty → single-letter/abbrev → HTML-entity → delete-markers → non-homonym → placeholder-keywords → `applyVirusGate` → monomial-aggregate → lineage → multi-question → question-placeholder → clade → OTU/code group [note the exception `name` uses the trimmed/uppercased/last-word form in specific branches — replicate exactly] → hybrid-formula). Port `applyVirusGate` (9 rules, uses `viral::is_viral`, `ctx.viral_shape`, `ctx.requested_code`, `ctx.authorship_input`) and `looksLikeHybridFormula` (structural scan) and the char helpers (`countLetters`/`hasDigit`/`countLatinWords`/etc.). Where Java throws `UnparsableNameException(type, [code,] name)`, return `Err(ParseError::new(type, code, name))`.

- [ ] **Step 4: Run → pass** the Task-5 tests. Then run the whole suite; pristine. Commit: `Phase 1: port Preflight gate`.

---

## Task 6: The full-parse golden harness (error-classification gate)

**Files:** Create `crates/nameparser/tests/parse_golden.rs`; add `testdata/expected-parse.jsonl` to `.gitignore`.

**Interfaces consumed:** `nameparser::parse`, `model::{NameType, NomCode}`.

- [ ] **Step 1: Generate the Java oracle.** Using the existing shaded jar, run the CLI `parse` over the benchmark corpus into `expected-parse.jsonl`:
```bash
export PATH="$HOME/.cargo/bin:$PATH"; [ -s "$HOME/.sdkman/bin/sdkman-init.sh" ] && source "$HOME/.sdkman/bin/sdkman-init.sh"
JAR=$(ls /Users/markus/code/gbif/name-parser/name-parser-cli/target/name-parser-cli-*-shaded.jar | head -1)
java -jar "$JAR" parse --input=- --output=- --format=jsonl \
  < testdata/benchmark-data.txt > testdata/expected-parse.jsonl 2>/dev/null
wc -l testdata/expected-parse.jsonl   # ~8017
```
Add `testdata/expected-parse.jsonl` to `.gitignore`.

- [ ] **Step 2: Write the gate test.** For each line of `expected-parse.jsonl`, parse the JSON (serde_json `Value`), read `input`, and whether it has `error` (with `error.type`, `error.code`) or `parsed`. Call Rust `nameparser::parse(input, None, None, None)`. Assert the **partition** matches: Java `error` ⇔ Rust `Err`; Java `parsed` ⇔ Rust `Ok`. For error rows, assert Rust's `ParseError.type_`/`code` equal Java's `error.type`/`error.code`. Tally and print `N rows, P partition-mismatches, T type/code-mismatches`; assert both mismatch counts are 0 (or a documented allowlist if a tiny irreducible residual remains — same policy as the Phase-0 tokenizer golden test).
```rust
// SPDX-License-Identifier: Apache-2.0
use nameparser::model::{NameType, NomCode};
// helper: map the Java enum string ("OTHER"/"VIRUS"/…) to compare against Rust Debug/name.
```

- [ ] **Step 3: Run + triage.** `cargo test -p nameparser --test parse_golden -- --nocapture`. Expect a small mismatch count on the first run; each mismatch is a Preflight fidelity bug — fix in `preflight.rs` (or `viral.rs`), re-run until 0 (or documented residual). Record the final numbers for Task 7.

- [ ] **Step 4: Commit.** `Phase 1: full-parse golden harness (error-classification gate)`.

---

## Task 7: Status doc

**Files:** Create `docs/superpowers/findings/2026-07-10-phase1-foundation-status.md`.

- [ ] **Step 1:** Record: corpus size; error-classification parity (partition + type/code mismatch counts, any allowlist); the wire-format model status (which reference rows serialize byte-identically); what's stubbed (full `Rank`, downstream stages, parsed-field parity); and the next slice (StripAndStash, with `NOM_NOTE` as the stress pattern). Commit: `Phase 1: foundation status + next-slice pointer`.

---

## Self-Review

**Spec coverage** (against the design spec §6.2 core-port and §7 golden method, and the Phase-0 findings' 6 requirements): the model + wire format (Task 2) realizes §6.2; the golden harness (Task 6) realizes findings-requirement #2 (corpus-level diff vs real Java) at the error-classification level; the per-pattern flag rule (Global Constraints + Task 5) realizes findings-requirement #1; Preflight (Task 5) is the first stage of the §6.2 pipeline. Deferred and stated: full `Rank`, downstream stages, parsed-field parity (later slices), the ~1,925 unit-assertion port (final slice).

**Placeholder scan:** the only `…` fillers are the `warnings` constant list (Task 1 Step 2, explicitly "port verbatim from Warnings.java") and the model struct body (Task 2 Step 3, "fill every field per the Reference order") — both point at an authoritative source in-repo. No TBD logic.

**Type consistency:** `ParseError` (Task 1) is produced by `pipeline::run` (Task 3) and `preflight::run` (Task 5) and consumed by the harness (Task 6). `ParseContext` (Task 3) is consumed by `preflight::run` (Task 5). `parse()` (Task 3) is consumed by the harness (Task 6). Enum names (Task 1) match the Java `.name()` strings the harness compares (Task 6).
