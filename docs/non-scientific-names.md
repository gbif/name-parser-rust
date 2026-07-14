# Non-scientific "names" — handling & backlog

Molecular biology, environmental sequencing, virology and microbiology have produced a whole
ecosystem of name-like strings that are **not** governed by the ICN, ICZN or ICNP. This document
catalogues the common shapes, records how the parser treats each one today, and tracks what is still
open.

Related, already shipped:
- **`NameType.IDENTIFIER`** — [design spec](superpowers/specs/2026-07-14-nametype-identifier-design.md).
- The curated culture-collection acronym list — `crates/nameparser/resources/culture-collections.txt`.
- The 5.0.0 `Informal` result + its verbatim-phrase contract (the tail after `sp.`/`spec.`/… is kept
  as the `phrase`).

**Status legend:** ✅ shipped · 🔶 partial · ⬜ open (backlog).

---

## Categories

### 1. Sequence-derived placeholders — 🔶
A genus + a specimen/culture provenance keyword + a code.

```
Bacillus sp. clone 23A
Bacillus sp. isolate XJ-17
Bacillus sp. strain DSM 12345
Bacillus sp. voucher Smith-42
```

`Genus sp. <tail>` parses as **`Informal`** with the whole tail as the `phrase` ✅. On a *complete
binomial* only a trailing culture-collection accession is captured so far (category 10 note); the
general keyword-driven capture (`clone`/`isolate`/`voucher` after a determined binomial) is ⬜.
Provenance keywords: `clone`, `isolate`, `strain`, `culture`, `voucher`, `specimen`, `sample`.

### 2. Host-associated names — 🔶
Common in plant pathology; the named organism is often a phytoplasma/fungus/virus *of* the host.

```
Alstroemeria sp. phytoplasma
Alstroemeria phytoplasma
Persea americana phytoplasma
Potato phytoplasma
```

`Genus sp. phytoplasma` → `Informal`, phrase `sp. phytoplasma` ✅. A bare `Genus phytoplasma` or a
complete binomial + annotation (`Persea americana phytoplasma`) still parses as a plain scientific
tri/binomial ⬜ — capturing the annotation needs the keyword thread (pinned as a boundary test).

### 3. Environmental samples — ✅
Descriptive, not identifiers → stay `OTHER`/`PLACEHOLDER` (deliberately *not* `IDENTIFIER`).

```
uncultured bacterium        uncultured marine bacterium
uncultured archaeon         environmental sample
uncultured fungus           activated sludge bacterium
uncultured eukaryote        deep-sea sediment archaeon
```

### 4. Candidate taxa — ✅
Provisional prokaryote names. Handled: the `Candidatus`/`Ca.` prefix sets the `candidatus` flag and
the name parses as `SCIENTIFIC`.

```
Candidatus Liberibacter asiaticus
Ca. Nitrosopelagicus brevis
Ca. Accumulibacter phosphatis
```

### 5. Molecular operational units — ✅
Not scientific names but **identifiers** → `NameType.IDENTIFIER` (was `OTHER` before 5.0.0). This is
the core of the shipped IDENTIFIER work; `IDENTIFIER` is deliberately more generic than the old
OTU-specific handling, and keeps BOLD/SH simpler than the catch-all `OTHER`.

```
OTU-17   OTU 34   ESV12   ASV_103   zOTU44   BIN BOLD:AAA1234   SH154321.09FU
```

| Scheme | Meaning |
|---|---|
| OTU | Operational Taxonomic Unit |
| ASV | Amplicon Sequence Variant |
| ESV | Exact Sequence Variant |
| BIN | Barcode Index Number (BOLD) |
| SH  | UNITE Species Hypothesis |

### 6. Genotype / haplotype designations — ✅
Typing labels, not names → stay `OTHER`. (Could be sub-classified later; see *OtherType*.)

```
genotype II   genotype IIIb   haplotype H4   ribotype 027   sequence type ST131   MLST ST-42
```

### 7. Pathotypes & pathogenic variants — ⬜
Infraspecific-like designations with their own rank (`pv.`/`bv.`/`serovar`). A dedicated **rank**
thread, closer to how subspecies is handled than to IDENTIFIER/OTHER — not yet done.

```
Xanthomonas campestris pv. campestris
Pseudomonas syringae pv. tomato
Escherichia coli O157:H7
Salmonella Typhi / Salmonella Enteritidis        (capitalised serovar convention)
```

Markers: pathovar (`pv.`), biovar (`bv.`), serovar / serotype, serogroup, phagetype.

### 8. Virus isolate conventions — ✅ (mostly)
Have their own ICTV conventions → `OTHER` with `code = VIRUS`.

```
Influenza A virus (A/Hong Kong/1/1968(H3N2))
SARS-CoV-2 isolate Wuhan-Hu-1
Tomato mosaic virus isolate Tm-17
```

### 9. MAGs & SAGs — 🔶
Metagenome-/single-amplified genomes. Scheme-prefixed ones are `IDENTIFIER` ✅ (`MAG`/`SAG`/`UBA`
patterns); a bare GenBank-assembly accession with no recognised prefix (`JAFGQ01`) is still `OTHER` ⬜.

```
MAG-24   SAG_102   UBA12345   JAFGQ01
```

### 10. Indeterminate genera/species — ✅
Common in GenBank; the tail is almost always an isolate designation.

```
Bacterium sp.   Bacterium sp. A   Bacterium sp. 17   Bacterium sp. B12   Bacterium sp. clone 15
Fungus sp.      Alga sp.          Diatom sp.
```

`Genus sp. <tail>` → `Informal`, tail kept as the `phrase`; a bare `Genus sp.` keeps the verbatim
marker (`sp.`) as its phrase and stays flagged indeterminate.

### 11. "cf." / "aff." / "near" — 🔶
Open-nomenclature uncertainty. `cf.`/`aff.` are captured in `epithetQualifier` and the name stays
`Parsed` ✅. `near` is ⬜ (not captured yet).

```
Quercus cf. robur   Agaricus aff. bisporus   Poa near pratensis
```

### 12. Species complexes — ✅
Not formal names, but treated as such with the `species_aggregate` rank.

```
Anopheles gambiae complex   Fusarium oxysporum species complex   Bemisia tabaci complex
```

### 13. Molecular clades — 🔶
Informal group labels → should be `OTHER`. `Clade A` is `OTHER` ✅, but `Lineage B.1.1.7` currently
mis-parses as a scientific binomial ⬜.

```
SAR11 clade   Roseobacter clade   Clade A   Clade II   Lineage B.1.1.7   subclade IIa
```

### 14. Environmental clone names — ✅
GenBank is full of these → `OTHER`.

```
uncultured bacterium clone S1-23   marine bacterium clone HF120   soil fungus clone A7
```

### 15. Database-generated placeholders — ✅
Historical naming from before genomes were available → `OTHER`.

```
Bacterium enrichment culture clone 45   Candidate division OP11 bacterium   Acidobacteria bacterium Ellin345
```

---

## `IDENTIFIER` type vs an `OTHER` subtype

**Resolved:** we added a top-level `NameType.IDENTIFIER` (category 5/9) rather than burying it in an
`OTHER` subtype — it is a large, filterable, well-defined slice of the data.

The subtype idea is **kept for the *residual* `OTHER` bucket** (⬜) — sub-classifying the genuinely
loose strings once there is a need to slice them:

```java
enum OtherType { IDENTIFIER, ACCESSION, NUMERIC, ABBREVIATION, TEXT, UNKNOWN }
```

---

## Keyword lists (for the open keyword-capture thread — ⬜)

Words that signal "everything after here is no longer nomenclature." The provenance group already
drives `Informal` phrases; the others are backlog.

- **Specimen provenance:** strain · isolate · clone · culture · culture collection · sample ·
  voucher · specimen · material · accession
- **Molecular:** haplotype · genotype · ribotype · sequence type · MLST · OTU · ASV · ESV · zOTU ·
  MAG · SAG · contig · scaffold · amplicon · barcode
- **Pathology** (mostly want to become *ranks*, category 7): serotype · serovar · serogroup ·
  biotype · biovar · chemotype · ecotype · pathotype · pathovar · forma specialis · f. sp. · race ·
  physiological race
- **Host / environment:** phytoplasma · endophyte · symbiont · epiphyte · parasite ·
  environmental sample · uncultured · metagenome · microbiome
- **Sequencing metadata:** DNA · RNA · 16S · 18S · ITS · ITS1 · ITS2 · COI · COX1 · matK · rbcL

---

## Culture-collection accessions — ✅

Most strain designations are `<collection acronym> <accession>`. There is no official exhaustive
registry of acronyms; the closest is the WDCM's *Culture Collections Information Worldwide* (CCINFO)
database, but a handful of major collections supply ~90% of references. We maintain a curated,
conservative seed list in `crates/nameparser/resources/culture-collections.txt` and build the
detection regexes from it.

- **Standalone** (`DSM 10`) → `NameType.IDENTIFIER`.
- **Trailing a determined binomial** (`Aquimarina muelleri DSM 19832`) → captured as the `phrase`
  (type `INFORMAL`), instead of `DSM` being misread as an author.

| Acronym | Collection |
|---|---|
| ATCC | American Type Culture Collection |
| DSM / DSMZ | German Collection of Microorganisms |
| JCM | Japan Collection of Microorganisms |
| NBRC | NITE Biological Resource Center |
| CCUG | Culture Collection, University of Gothenburg |
| LMG | Belgian Coordinated Collections of Microorganisms |
| CBS | Westerdijk Fungal Biodiversity Institute |
| NRRL | ARS Culture Collection |
| CECT | Spanish Type Culture Collection |
| CIP | Collection de l'Institut Pasteur |
| NCTC | National Collection of Type Cultures |
| NCIMB | National Collection of Industrial, Food and Marine Bacteria |
| IAM | Institute of Applied Microbiology |
| VKM | All-Russian / Russian Academy Collection |
| VKPM | Russian Industrial Microorganisms |
| KCTC | Korean Collection for Type Cultures |
| KACC | Korean Agricultural Culture Collection |
| CGMCC | China General Microbiological Culture Collection Center |
| CICC | China Center of Industrial Culture Collection |
| MCCC | Marine Culture Collection of China |
| BCRC | Bioresource Collection and Research Center |
| MTCC | Microbial Type Culture Collection |
| MCC | Microbial Culture Collection (India) |
| ICMP | International Collection of Microorganisms from Plants |
| PCC | Pasteur Culture Collection (cyanobacteria) |
| SAG | Göttingen Algal Collection |
| UTEX | University of Texas Algae Collection |
| CCAP | Culture Collection of Algae and Protozoa |

Accession shapes the recogniser must tolerate:

```
DSM 10        DSM 30083     ATCC 11775    CBS 123.89    LMG 6923T     JCM 1002    NBRC 14126
ATCC BAA-123  ATCC PTA-1234 DSM 12345T    CBS 12345A                             (letter prefixes / type-strain suffix)
ATCC-11775    CBS-12345                                                          (hyphen separator)
ATCC11775                                                                        (no separator)
```

---

## Open backlog (summary)

- ⬜ **Keyword-driven phrase capture** on a *complete binomial* (categories 1, 2) — `clone`/`isolate`/
  `voucher`/`phytoplasma`/… after a determined name.
- ⬜ **Pathovar/biovar/serovar as infraspecific ranks** (category 7); capitalised serovars
  (`Salmonella Typhi`).
- ⬜ **`near`** as an open-nomenclature qualifier (category 11).
- ⬜ **Molecular clades / lineages** mis-parsing as scientific names (`Lineage B.1.1.7`, category 13).
- ⬜ **`OtherType` sub-classification** of the residual `OTHER` bucket.
- ⬜ **Prefix-less assembly accessions** (`JAFGQ01`, category 9).
