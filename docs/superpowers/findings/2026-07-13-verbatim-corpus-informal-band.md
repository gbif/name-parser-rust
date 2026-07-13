# The informal / semistructured name band — a 67.5M verbatim-corpus study

*2026-07-13. Motivated the 5.0.0 `ParseResult.Informal` result variant (see the design plan and
`docs/superpowers/plans/`). Raw report: `2026-07-13-verbatim-corpus-band-report.txt`; the streaming
classifier that produced it: `tools/band_analyze.py`.*

## Why

The 5.0.0 API is exceptionless (`NameParser.parse` returns a `ParseResult`) and the Rust port is now the sole
implementation. Before designing how the parser should represent **"informal / semistructured" names** — names
anchored to a real taxon but carrying a non-code designation (`Rhizobium sp. RMCC TR1811`, `Ichneumonidae sp.`,
`Bartonella group`, `Salix alba subsp. B`) — we measured how common they really are and what they look like, on
the real workload rather than curated fixtures.

## The corpus

`testdata/clb-verbatim-names.tsv` — **67,471,137** raw ChecklistBank verbatim names (the *input* distribution,
not post-parser). Columns `rowType, scientificName, authorship, rank, code`. (This file was also cleaned in the
same session: a header row added and postgres `\N` NULL markers replaced with empty strings.) Each
`scientificName` was run through the Rust CLI `parse` and classified by a streaming analyzer.

## Outcome distribution (67,192,265 parsed; ~279k empty names skipped)

| Outcome | Count | % |
|---|---|---|
| parsed **SCIENTIFIC** | 59,779,793 | 88.97% |
| parsed **INFORMAL** (the band) | **3,699,795** | **5.51%** |
| error OTHER (junk) | 3,289,532 | 4.90% |
| parsed PLACEHOLDER | 226,073 | 0.34% |
| error FORMULA (hybrids) | 108,393 | 0.16% |
| error PLACEHOLDER | 84,083 | 0.13% |
| error **INFORMAL** | **4,596** | **0.007%** |

**Two headlines:**
1. The informal band is **5.5% of all real names — 1 in 18**. It is a first-class part of the workload, not an
   edge case, and warrants a first-class representation.
2. INFORMAL-as-*unparsable* (the thing that first surfaced this, under the old two-variant model where
   `ParseResult.Unparsable` rejects `isParsable()` types) is a **rounding error (4,596)**. The design driver is
   the 3.7M *parsed*-INFORMAL names, not the handful of errors.

## The band is not one thing — it splits, and the split has a clean home

| Sub-band | Count | Real example | Home in 5.0.0 |
|---|---|---|---|
| cf./aff./? on a complete binomial | ~161k | `Salicornia cf. patula`, `Perizoma? albifascia` | **Parsed** (determined; qualifier is an annotation in `epithetQualifier`) |
| provisional species w/ a captured phrase tag | 3,105,962 | `Serratia sp. RE1-2a`, `Plasmodium sp. SYBOR9` | **Informal** |
| provisional species, tag *not* captured | 381,914 | `Rhizobium sp. RMCC TR1811`, `Ichneumonidae sp. UAM Ento 145060` | **Informal** (after the tag-capture fix) |
| infraspecific / strain indet | ~52k | `Pygopleurus purpureus ab.`, `…ubique strain HIMB058`, `Salix alba subsp. B` | **Parsed** (has a binomial core) |

Qualifier counts across the band: `cf.` 93,534 · `aff.` 35,391 · `?` 30,032.

## Key insights (these drove the design)

1. **Overwhelmingly molecular / DNA-barcoding provisional species** — `Genus sp. <specimen/culture/BOLD code>`,
   ~99.8% **genus**-anchored, almost always **SPECIES** rank. Phrase leading tokens are dominated by bare
   **numbers** (`sp. 1`, `sp. 2`, …). Higher-taxon anchors (`Ichneumonidae sp.`, family-level) are real but
   ~0.2% (~6k).
2. **`cf.`/`aff.`/`?` are cleanly `Parsed`.** They are complete binomials that were only "informal" because of an
   open-nomenclature qualifier; the qualifier is an annotation, not a reclassification.
3. **"indetermined" vs "phrase" is a parser-capture artifact, not a real distinction.** `Serratia sp. RE1-2a`
   captured `RE1-2a` as a phrase; `Rhizobium sp. RMCC TR1811` *dropped* the multi-token tag and merely looks
   "indetermined" — the same kind of name. So the two must be treated as one concept, and the capture fixed so
   the trailing tag is always grabbed (this rescues the ~382k).
4. **The anchor is scattered across `ParsedName` fields** (genus vs uninomial vs genus+species), and an
   unvalidated `Rhizobium`/`fungal` mislabelled as "genus" is a false promise — so an informal result should be
   a *dedicated* type with one explicit taxon anchor, not a reused `ParsedName`.
5. **The binomial split.** The clean cut is not "informal or not" but **binomial-ness**: a name with a species
   epithet (a binomial core) stays `Parsed` — `ParsedName` natively holds its `specificAuthorship`, which a flat
   anchor could never represent (`Salix alba L. subsp. B`); a supraspecific anchor + a provisional designation
   with *no* species epithet becomes `Informal`, whose anchor is then always a single supraspecific taxon and a
   flat `taxon` + `taxonRank` is unambiguous.

## Design conclusion

A three-way `ParseResult` — `Parsed | Informal | Unparsable`:

- `Informal(taxon, taxonRank, rank, phrase, code)` — a dedicated flat type for supraspecific-provisional names.
- Discriminator: `type == INFORMAL && specificEpithet == null` → `Informal`; else parsable → `Parsed`.
- Net reclassification: **~213k binomial-informals stay `Parsed`** (unchanged), **~3.49M supraspecific-provisional
  → `Informal`**, **~4.6k `error:INFORMAL` → `OTHER` or rescued** (anchored ones like `Bartonella group` become
  `Informal`; anchorless ones like `Unnamed clade` stay `Unparsable(OTHER)`).

## Reproduce

```sh
# the classifier streams the CLI output; ~20 min over the full 67.5M corpus
tail -n +2 testdata/clb-verbatim-names.tsv | cut -f2 | grep -v '^$' \
  | ./target/release/nameparser-cli parse --input=- --output=- --quiet \
  | python3 tools/band_analyze.py > band_report.txt
```
