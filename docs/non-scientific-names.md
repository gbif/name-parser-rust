# Non-scientific "names" — handling & backlog

Molecular biology, environmental sequencing, virology and microbiology have produced a whole
ecosystem of name-like strings that are **not** governed by the ICN, ICZN or ICNP. This document
catalogues the common shapes, records how the parser treats each one today, and tracks what is still
open.

Related, already shipped:
- **`NameType.IDENTIFIER`** — [design spec](nametype-identifier-design.md).
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

The tail runs to the **end of the input, authorship included** — `Cantuaria sp. Forster, 1968` →
phrase `sp. Forster, 1968`, with no `combinationAuthorship` and no inferred `code`. An informal
name is not fully parsable by definition, so nothing in the tail is interpreted and `taxon` + `" "`
+ `phrase` round-trips the input exactly. Until 5.0.0 a year-bearing tail was exempted and routed
to the authorship instead; over this corpus that produced a spurious authorship 43% of the time
(impossible years like `1002`/`2483`/`2951`, "authors" carrying digits or slashes, and collection
acronyms such as `ZRC`/`MNHN` that parse as clean surnames) and truncated the tag it declined to
capture — `Rhodococcus sp. 14-2483-1-2` kept only `sp. 14` and invented the year 2483. On the
genuinely authored minority the citation belongs to the anchor taxon, not to the undetermined
species, so a caller who wants it resolves `taxon`. The `cf.`/`aff.` and dot-less-`spec` carve-outs
are unaffected: `Hemicloeina spec Platnick, 2002` still parses as a determined binomial.

Two StripAndStash steps run *before* tokenising and used to amputate part of that tail, so Assemble
puts it back when — and only when — the finished parse is an indet informal. `stashTrailingOtuCode`
took an underscored code into `unparsed` (`Streptomyces sp. NBC_00448` → phrase `sp.`, PARTIAL),
and `stripPublishedPage` read a catalogue number's `:<digits>` as a page (`Trachipterus sp.
HUMZ:220860` → phrase `sp. HUMZ`, `publishedInPage=220860`). Neither field exists on a flat
`Informal`, so both were lost outright at the three-way boundary. Both decisions key on the parse
OUTCOME rather than the input's shape, because the shapes are genuinely ambiguous — the real page
citation `Raphitydeus Thor 1933:54` is written exactly as tightly as `Prevotella sp. CAG:1031`, and
`Braconidae gen. n. sp. JS10_00530` ends in `sp.` yet never parses as an indet at all. A DETERMINED
name keeps both strips: it stays `Parsed`, where `unparsed` and `publishedInPage` remain reachable
on the `ParsedName`, so nothing is lost there either.

### 11. "cf." / "aff." / "near" — ✅
Open-nomenclature uncertainty, captured in `epithetQualifier` with the name staying `Parsed` (type
`INFORMAL`). `cf.`/`aff.` are stored with their abbreviation dot; `near` (a full English word,
synonymous with aff.) is stored verbatim. All three are recognised lowercase; an uppercased marker
is still read as an author (a shared pre-existing limitation).

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

## Bare strain codes trailing a binomial — ✅

A strain code with no collection acronym in front of it, sitting on an otherwise determined
`Genus species`, is captured as the `phrase` (type `INFORMAL`) rather than fed to the authorship
parser. Java only recognised codes that START WITH A LETTER (`Bacteroides caccae CAG21`,
`Candida albicans RNA_CTR0-3`); a DIGIT-LEADING code was silently disposed of instead
(gbif/name-parser-rust#16):

| input | Java / ≤ 0.2.0 | now |
|---|---|---|
| `Escherichia coli 18-41` | `Escherichia coli`, code dropped | phrase `18-41` |
| `Prunus domestica 5a` | `Prunus domestica` + fabricated author `a` | phrase `5a` |
| `Acinetobacter baumannii 6112` | `Acinetobacter baumannii` + fabricated year `6112` | phrase `6112` |
| `Prunus domestica 6/12` | `Prunus domestica`, code dropped | phrase `6/12` |

All three old outcomes reported `type=SCIENTIFIC`, `state=COMPLETE` and an empty `unparsed`, so the
truncated result was byte-identical to a DIFFERENT real taxon and collapsed onto it in a names
index — which is how a ChecklistBank import ended up placing a taxon under its own grandchild
(CatalogueOfLife/checklistbank#1725). Keeping the code in the `phrase` keeps it in the canonical
rendering, so the collision cannot form.

A single full stop closing the string (`Prunus domestica 6.`) is sentence punctuation: it is
tolerated and left out of the phrase. Three shapes are deliberately excluded, because something
else already reads them correctly:

- **a bare `1xxx`/`2xxx`** — `Prunus domestica 1888` is a publication year, unchanged;
- **a numeral-prefixed Latin epithet** — see the next section;
- **an indeterminate marker in the epithet slot** — `Allium species 1` keeps its marker in the
  phrase (`species 1`), the same treatment `Genus sp. <n>` already gets.

---

## Indeterminate `Genus <rank marker> <code>` — ✅

A rank marker in the epithet slot is not an epithet, so the whole `<marker> <code>` tail is the
designation: it becomes the verbatim `phrase` (type `INFORMAL`), the rank comes from the marker,
and the taxon reduces to the bare genus, so `taxon + " " + phrase` round-trips.

| input | Java / ≤ 0.2.0 | now |
|---|---|---|
| `Allium sect 1` | `Allium 1` — marker lost | phrase `sect 1` |
| `Allium sect. 1` | `Allium` — whole tail lost | phrase `sect. 1` |
| `Allium subg. 3` | `Allium` — whole tail lost | phrase `subg. 3` |
| `Trachelomonas strain T101` | phrase `T101`, but `specificEpithet="strain"` | `STRAIN`, phrase `strain T101` |

Only an **infraspecific** marker's rank is applied. An infrageneric one's is not: at an
infrageneric rank the assembler reads the single remaining word as the infrageneric *epithet*, so
`Allium sect 1` came back as `infragenericEpithet="Allium"` with no genus at all and rendered
`sect. Allium sect 1`. Allium is the genus there — the section is the unnamed thing, and no field
can hold an unnamed one. The marker survives verbatim in the phrase either way.

`sp` / `spec` / `species` / `indet` are excluded — those are handled earlier and better, keeping
the marker in the phrase at `SPECIES` rank.

The four-token form, where the species IS named, is handled too — `Abies alba var. 3`,
`subsp. 7`, `f. 2`, `var. 3a`, `var. B12`, `ssp. 1-CPM-2005`. There the marker stays OUT of the
phrase (`rank` already carries it and the formatter re-emits it), so the phrase is just the
designation and the input still round-trips. Previously the designation reached the authorship
parser and was dropped there — a 3-4 digit one becoming a bogus year — so every numbered variety
of a species rendered as the same bare `Abies alba var.`. A real authorship still parses
alongside it: `Abies alba subsp. 7 Mill.` yields both phrase `7` and author `Mill.`.

Two exclusions, both found by corpus diffs: a numeral-prefixed epithet after the marker
(`Benthogone rosea var. 4-lineata R. Perrier, 1896` — a real epithet, not a designation), and the
form with NO species epithet (`Aquificales str. OlB-6`), where nothing downstream would consume
the designation and it would be dropped outright.

---

## Unhyphenated numeral epithets — ✅

`Coccinella 11-punctata` (= *undecimpunctata*) has always tokenised as one word. The equally common
unhyphenated spelling did not, so the digit was dropped and the epithet promoted to an author:

| input | Java / ≤ 0.2.0 | now |
|---|---|---|
| `Coccinella 6maculata Fabricius, 1781` | uninomial `Coccinella`, author `maculata Fabricius` | species `6maculata`, author `Fabricius` |
| `Camponotus sericeus 4maculatus` | `Camponotus sericeus`, author `maculatus` | infraspecific `4maculatus` |
| `Chalcis 2spinosa (Fabricius, 1804)` | uninomial `Chalcis`, combination author `spinosa Fabricius` | species `2spinosa`, **basionym** author `Fabricius, 1804` |

`token::is_numeral_epithet` is the single authority for the shape, because three places must agree
on it or they undo each other: the tokenizer (which makes these words), the strain-code stash
(which must not take one as a code) and the numbered-infraspecific capture (which must not take one
as a designation).

The tokenizer rule is deliberately much narrower than the hyphenated one, because without the
hyphen the shape collides with things that must stay split: at most two digits (so a year with a
word glued on, `1976var`, keeps its `NUMBER`), no leading zero (the OCR of a capital O — `0ersted`
for Ørsted, `0lsson` for Olsson — where gluing would swallow the author), at least three lowercase
letters (so the year disambiguator `1935h` and the codes `5a` / `16S` / `2016Iso3` are untouched),
and the word must end there (`12abc4` stays split).

It cannot be told apart lexically from a strain code of the same shape, so a handful of those
(`Aeromonas hydrophila 11novo`, `Pandoraea pnomenusa 3kgm` — 6 names in 67.5M) come back as
epithets rather than phrases. Both readings keep the whole input and invent no authorship, so
neither can produce #16's collision.

---

## Measured impact

Over the 67.5M-row verbatim corpus, 1,220,644 distinct names sit in the shapes these changes can
touch, and **15,559 change**: 15,452 gain a phrase they previously lost, 34 gain or correct an
epithet, and 72 keep a marker or a digit that used to fall out of a (junk) author string. **No name
loses a phrase.** Over the 6.4M-name COL corpus, 38 change — 32 numeral epithets recovered from a
fabricated author, 6 numbered varieties that gain their designation.

One known regression, one name in 1.22M: `Aeromonas salmonicida subsp. pectinolytica 34mel` used
to keep half its strain code as a fabricated author `mel` and now drops `34mel` entirely — the
glued word becomes a fifth epithet on an already-complete quadrinomial, which the model cannot
hold. Both readings are wrong; neither is worth a fifth mechanism.

---

## Open backlog (summary)

- ⬜ **Keyword-driven phrase capture** on a *complete binomial* (categories 1, 2) — `clone`/`isolate`/
  `voucher`/`phytoplasma`/… after a determined name.
- ⬜ **Pathovar/biovar/serovar as infraspecific ranks** (category 7); capitalised serovars
  (`Salmonella Typhi`).
- ⬜ **Molecular clades / lineages** mis-parsing as scientific names (`Lineage B.1.1.7`, category 13).
- ⬜ **`OtherType` sub-classification** of the residual `OTHER` bucket.
- ⬜ **Prefix-less assembly accessions** (`JAFGQ01`, category 9).
