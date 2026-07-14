# `NameType.IDENTIFIER` + trailing culture-accession phrase capture — design

**Status:** approved (2026-07-14) · **Repos:** `name-parser-rust` (primary), `name-parser` (Java api)

## Goal

Two related changes, driven by a full-corpus review of the molecular / OTHER band:

- **A. Add `NameType.IDENTIFIER`** — a first-class type for anchorless, scheme-prefixed *machine identifiers* (UNITE **SH**, BOLD **BIN**, **OTU/ASV/ESV/zOTU/MAG/SAG/UBA**, and standalone **culture-collection accessions** like `DSM 10`, `ATCC 11775`). These are currently dumped in `OTHER`. IDENTIFIER is non-parsable, so it rides the existing `ParseResult.Unparsable` variant with a more specific `type()` — no `ParseResult` shape change.
- **B. Capture a trailing culture-collection accession as the phrase** on an otherwise-complete name (`Aquimarina muelleri DSM 19832` → phrase `DSM 19832`), instead of misreading `DSM` as an author.

GBIF is heavily and increasingly engaging with molecular data, so this band will keep growing; catering for it first-class (rather than as opaque `OTHER` / dropped tails) is worth the API change.

## Why (corpus evidence)

3M-row sample of the 67M-row ChecklistBank verbatim corpus (`testdata/clb-verbatim-names.tsv`):

| Signal | Count / 3M | % | Today |
|---|---|---|---|
| MOTU identifiers (BOLD 58k + UNITE SH 58k dominate) | 115,979 | **3.87%** | `Unparsable / OTHER` |
| culture-collection accession mentions | 2,016 | 0.067% | see below |
| … of which **standalone** | 2 | **0.1%** of accessions | `Unparsable / OTHER` |
| … of which **anchored** (trails a name) | 2,014 | **99.9%** of accessions | name parses; **accession dropped/misread** |

Two conclusions the data forces:

1. **IDENTIFIER phase-1 is essentially "relabel BOLD + SH".** They are ~all of the 3.87%, the parser already detects and canonicalises them, and they are anchorless (zero interaction with any phrase logic). Rescuing 3.9% of the entire corpus from `OTHER` is one branch change.
2. **Culture accessions are a *phrase* problem, not an IDENTIFIER problem.** 99.9% ride a name. The standalone case (→ IDENTIFIER) is a rounding error we cover for completeness; the real win is Part B. Today `Aquimarina muelleri DSM 19832` parses SCIENTIFIC but with `combinationAuthorship.authors = ["DSM"]` — the accession is swallowed as authorship.

## Non-goals (explicit scope boundaries)

- **Descriptive junk stays `OTHER`.** `uncultured bacterium`, `genotype II`, `Clade A`, `Lineage B.1.1.7`, environmental clones — these are prose descriptors, not machine IDs. IDENTIFIER means *structured, scheme-prefixed ID*; a descriptor is not that.
- **No `OtherType` sub-classification** (the `IDENTIFIER/ACCESSION/NUMERIC/ABBREVIATION/TEXT/UNKNOWN` idea). Deferred until there's a need to slice the residual `OTHER` bucket.
- **Pathovar/biovar/serovar-as-ranks** (`Xanthomonas campestris pv. campestris`) is a separate parse thread (infraspecific ranks), not part of this.
- **General host/annotation keyword capture** (`… phytoplasma`, `… endosymbiont`, the deferred `Persea americana phytoplasma`) is a natural *extension* of Part B's mechanism but is **out of scope here** — Part B is limited to the curated culture-collection accession shape. Noted as follow-on.

## Design

### Shared infrastructure: the culture-collection acronym list

A curated, maintained list of major culture-collection acronyms (the doc's ~30, covering ~90% of strain references), embedded at compile time like the existing epithet blacklist.

- **File:** `crates/nameparser/resources/culture-collections.txt`, one ALL-CAPS acronym per line, loaded once into a `HashSet<&str>` (mirror of `blacklisted_epithets.rs` / `resources/blacklist-epithets.txt`).
- **Seed contents:** `ATCC DSM DSMZ JCM NBRC CCUG LMG CBS NRRL CECT CIP NCTC NCIMB IAM VKM VKPM KCTC KACC CGMCC CICC MCCC BCRC MTCC MCC ICMP PCC SAG UTEX CCAP` (+ comment header noting the WDCM/CCINFO source and that additions are welcome).
- **Consumed by both** Part A (standalone accession → IDENTIFIER) and Part B (anchored accession → phrase).
- **Accession shape:** `<ACRONYM>` + optional separator (` `, `-`, `_`, `:`, `=`) + an accession body that starts with an optional short letter-prefix (`BAA-`, `PTA-`, `P-`) then a digit, then alphanumerics / `.` / `-` and an optional type-strain suffix (`T`, `A`): e.g. `DSM 30083`, `CBS 123.89`, `LMG 6923T`, `ATCC BAA-123`, `ATCC-11775`, `ATCC11775`.

**Ambiguity (decided — conservative start):** a few acronyms collide with other tokens — `SAG` is both the Göttingen algal collection *and* the MOTU scheme "Single Amplified Genome"; `PCC`, `MCC` are terse. The ALL-CAPS + immediately-accession-shaped + (for standalone) whole-string anchoring constraints keep false positives low. We seed the list conservatively and grow it as needed; the list is the trust boundary — see Risks.

### Part A — `NameType.IDENTIFIER`

#### A1. The enum (append-only, non-parsable)

- **Rust** `crates/nameparser/src/model/enums.rs`: add `Identifier` as the **last** `NameType` variant (ordinal 5, after `Other`). `#[serde(rename…)]` yields `"IDENTIFIER"`. `is_parsable()` is unchanged (Identifier is **not** `Scientific | Informal`), so it can only appear inside `ParseResult::Unparsable`.
- **Java** `name-parser/…/api/NameType.java`: add `IDENTIFIER` as the **last** constant (after `OTHER`); `isParsable()` returns false for it. Republish `name-parser-api` 5.0.0-SNAPSHOT.

Append-only is required so no existing ordinal shifts on the FFI wire.

#### A2. Detection & classification (`crates/nameparser/src/pipeline/preflight.rs`)

The identifier regexes already exist (`preflight.rs:114-121`): `OTU_BOLD`, `OTU_SH`, `OTU_GTDB_UBA`. The change is (a) route them to `Identifier` instead of `Other`, (b) add the missing MOTU schemes and the standalone-accession recogniser.

- **Relabel existing** (currently `preflight.rs:371-382`, all `NameType::Other`):
  - Split the `:371` branch — `OTU_BOLD || OTU_GTDB_UBA` → `Identifier`; keep `GEN_NOV || @` → `Other` (those aren't identifiers).
  - `OTU_SH` (`:380`) → `Identifier` (keep the `.to_uppercase()` canonicalisation — that is exactly the "keep BOLD/SH simpler" win).
- **Add MOTU schemes** — a new `OTU_MOTU` regex, whole-string, digit-required, applied in the same code-block:
  `(?i)^(?:z?OTU|ASV|ESV|MAG|SAG)(?-u:[ _-]?\d+)$` → `Identifier`. (Whole-string + trailing-digit avoids the *Uba fallai* / genus false positive class — a bare `^UBA\b` would wrongly catch the beetle genus *Uba*.)
- **Add standalone culture accession** — a new `CULTURE_ACCESSION_STANDALONE` regex built from the shared acronym set, whole-string only:
  `(?i)^(?:<ACRONYM>|…)(?-u:[ :=_-]*)(?-u:(?:[A-Z]{1,4}-)?\d[\w.:-]*)$` → `Identifier`. (Standalone only; the anchored case is Part B.)
- **Unchanged → still `Other`:** `PURE_ALPHANUM` generic mash (`:383+`), `CLADE_KEYWORD` (`:366`), lineage-code stems (`:340`), `MULTI_QUESTION_PREFIX`, `GEN_NOV`, `@`. These are junk/descriptive, not identifiers.

Result: these paths return `Err(ParseError::new(NameType::Identifier, None, canonical))`, surfaced as `ParseResult::Unparsable { type: IDENTIFIER, … }`.

#### A3. FFI wire (Java binding only)

The struct layout is **unchanged** — `name_type` is still one i32 ordinal at `OFF_NAME_TYPE = 16` (`layout.rs:221`). Only the valid ordinal *range* grows.

- **Rust** `crates/nameparser-ffi/src/layout.rs`: add `NameType::Identifier => 5` to the exhaustive `name_type_ordinal` match (`:362`). Bump `np_abi_version()` **3 → 4** (the enum value-range is part of the wire contract; a new cdylib emitting ordinal 5 into an old decoder would `AIOOBE`, so the ABI guard must force lockstep — this session already hit the stale-cdylib failure mode).
- **Java** `StructCodec.java`: bump the ordinal guard `requireEnumShape("NameType", NameType.values().length, 5)` → `6` (`:134`). Decode is `values()[ordinal]`, so appending `IDENTIFIER` to the Java enum makes `values()[5]` resolve automatically; verify `toParseResult` surfaces it on the `Unparsable.type()`.
- **Java** `Ffi.java`: `EXPECTED_ABI_VERSION = 3` → `4` (`:51`).

#### A4. Python / R (read the core model directly — no wire)

`"IDENTIFIER"` flows through the serde string surface for free. Only test additions + (R) confirming the `type` column carries it.

### Part B — trailing culture-accession → phrase

**Gap:** `Aquimarina muelleri DSM 19832` → SCIENTIFIC but `authors=["DSM"]`, accession lost.

**Mechanism:** extend the existing trailing-strain-code stasher. `stash_trailing_strain_code` runs at `stripandstash.rs:63`, driven by `STRAIN_DESIGNATION` (`:590`) and `is_strain_code` (`name_tokens.rs:1030`) — today it catches explicit `str.`/`strain`-marked designations. Add a branch that recognises a **trailing `<curated-acronym> <accession-body>`** run (using the shared list + accession shape) and stashes it on `ctx.name.phrase` **before** authorship parsing claims `DSM` as an author.

- **Anchor unchanged, name kept:** genus + species stay; `combinationAuthorship` no longer gets `DSM`.
- **Type/rank:** `type = INFORMAL`, `phrase = "DSM 19832"` — parity with the existing strain-designation convention (`Aphanizomenon flos-aquae str. 'Aph K2'` is already a Parsed binomial with `type=INFORMAL`, `phrase`). **Decided (2026-07-14): INFORMAL-with-phrase**, even for a bare, marker-less accession.
- **Result variant:** stays **Parsed** (a species epithet is present), so this rides the 5.0.0 `Parsed` path with the phrase carried on the `ParsedName`.

## Change-point summary

| Area | File | What |
|---|---|---|
| enum (core) | `crates/nameparser/src/model/enums.rs:9` | append `Identifier` |
| enum (api) | `name-parser/…/api/NameType.java` | append `IDENTIFIER`, `isParsable()`=false |
| shared list | `crates/nameparser/resources/culture-collections.txt` (new) + a loader `culture_collections.rs` (mirror `blacklisted_epithets.rs`) | curated ALL-CAPS acronyms |
| detect A | `crates/nameparser/src/pipeline/preflight.rs:371-382` (+ new `OTU_MOTU`, `CULTURE_ACCESSION_STANDALONE` regexes near `:114`) | route identifiers → `Identifier` |
| phrase B | `crates/nameparser/src/pipeline/stripandstash.rs:63,578-590` + `name_tokens.rs:1030` | recognise trailing acronym-accession → phrase |
| FFI | `crates/nameparser-ffi/src/layout.rs:362` + `np_abi_version` | ordinal 5, ABI 3→4 |
| Java wire | `StructCodec.java:134`, `Ffi.java:51` | guard 5→6, ABI 3→4 |
| goldens | `testdata/golden/expected-parse.jsonl`, `expected-format.tsv`, `testdata/otu.{rust,java}.jsonl` | re-base (BOLD/SH → IDENTIFIER; accession rows → phrase) |
| tests | `tests/informal.rs`/new `tests/identifier.rs`, `common/mod.rs` DSL, `NameParserRustSmokeTest.java`, `test_api.py`, `test-parse-names.R` | IDENTIFIER + accession-phrase cases |

## Testing / verification

- **DSL:** add `assert_identifier(input)` (or reuse `assert_unparsable` with a `NameType::Identifier` arg) for `BOLD:AAA0001`, `SH1957732.10FU`, `OTU-17`, `ASV_103`, `DSM 10`, `ATCC 11775`. Boundary: `Uba fallai …` must stay Parsed/SCIENTIFIC (the genus, not the UBA scheme); `uncultured bacterium`, `Clade A` must stay `OTHER`.
- **Part B:** `Aquimarina muelleri DSM 19832` → Parsed, phrase `DSM 19832`, `authors` empty; `Bacillus subtilis DSM 10`, `Escherichia coli ATCC 11775` likewise.
- **Corpus re-scan:** re-run `tools/band_analyze.py` over a sample; confirm ~3.9% moves `OTHER → IDENTIFIER` and nothing scientific regresses.
- **Golden re-base** (Rust snapshots) + all three bindings green + `parse_golden`/`format_golden` gates.

## Risks / open questions

1. **Acronym collisions** (`SAG`, `PCC`, `MCC`) — the curated list is the trust boundary; ALL-CAPS + accession-shape + whole-string (standalone) contain it. **Decided: start conservative** (seed list only, grow as needed); the list is maintainable.
2. **INFORMAL vs SCIENTIFIC for a bare trailing accession** (Part B) — **decided: INFORMAL-with-phrase** (existing strain convention).
3. **ABI bump** ripples to any out-of-tree consumer of the cdylib — none today (JAR bundles it), but note in `DISTRIBUTION.md`.
4. **api release** — adding a `NameType` constant is source-compatible for switches with a default, but exhaustive `switch` consumers in the backend must add an `IDENTIFIER` arm; audit CoL backend on cutover.
5. **`GEN_NOV`/`@`** deliberately stay `OTHER` — don't let the split at `preflight.rs:371` accidentally sweep them into IDENTIFIER.

## Suggested sequencing

1. Enum (core + api) + FFI/ABI + guard — the plumbing, golden-neutral except the relabel.
2. Part A detection (relabel BOLD/SH/UBA + MOTU + standalone accession) + golden re-base + DSL/binding tests.
3. Part B (trailing accession → phrase) + golden re-base + tests.
4. Corpus re-scan + CoL backend `switch` audit.
