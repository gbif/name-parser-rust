# `nameparser` data resources — provenance

How the data files in this directory were built and are maintained. They are embedded into the
crate at compile time via `include_str!`, so they ship inside the compiled library — but this
note records where they came from and how to re-validate them.

Ported from the Java name-parser's `name-parser/dev/README.md`.

## Epithet blacklist (`blacklist-epithets.txt`)

A stop-list of words that look like specific epithets but are common false positives; a
blacklisted epithet flags the name **doubtful** with a `blacklisted epithet` warning. See
`src/pipeline/blacklisted_epithets.rs` for the loader and membership check.

The file was copied byte-for-byte from the Java name-parser's
`nameparser/blacklist-epithets.txt` classpath resource (`diff`-verified identical), so both
implementations flag exactly the same words. It has 275 lowercase, ASCII, one-per-line entries.

To re-validate the list, query the GBIF ChecklistBank API for each epithet and check how many
real names match (the Java project kept a `blacklist-test.py` for this) — an entry that matches
many valid names is a candidate for the whitelist below.

### Blacklisted epithets that still yield some GBIF matches (kept anyway)

- `die` — `Anticharis die Isiana Pilg.` is a bad name based on `Anticharis dielsiana Pilg.`
- `mon` — `Euchroeus mon` is a bad name based on *Euchroeus mongolicus*.

### Whitelist — words once blacklisted but removed because they are real epithets

- `alle` — *Alle alle* (Linnaeus, 1758)
- `an` — *Ischnothyreus an* Tong & Li, 2016
- `be` — *Linta be* 2004
- `den` — *Agnetina den* 2006
- `far` — *Esox far* Forsskål, 1775
- `get` — *Kibenikhoria get*, G. G. Simpson 1935
- `incertae` — *Sigmesalia incertae* (Deshayes, 1832)
- `may` — *Anelosimus may* Agnarsson, 2005
- `now` — *Apopyllus now* Platnick & Shadab, 1984
- `nur` — *Diospyros nur* Ritter, N. & De la Barra, N. 2016
- `once` — *Heterospilus once* Marsh, 2013
- `our` — *Mugil our* Forsskål, 1775
- `pas` — *Cantabroplectus pas* Struyve, 2018
- `plus` — *Rubus plus* L.H.Bailey
- `qui` — *Willowsia qui* Zhang, Chen & Deharveng, 2011
- `that` — *Xerolinus that* (Steiner, 2006)
- `this` — *Xerolinus this* (Steiner, 2006)
- `une` — *Trechiama une* Ueno, 2001

## Homoglyph table (`homoglyphs.txt`)

Confusable-character normalisation table, likewise copied from the Java name-parser resource of
the same name and `diff`-verified. See `src/unicode.rs` for its use.

## Double surnames (`double-surnames.tsv`)

Authors whose two surnames are joined by "y" — Spanish (and Catalan, Mexican, Cuban, …) double
surnames, one person each: `Bolívar y Pieltain`, `Dusmet y Alonso`, `Caballero y Caballero`. The
authorship parser joins an `X y Y` into one author only when the pair is listed; any other `y`
separates two people, because the string alone cannot tell `Dusmet y Alonso` (one person) from
`Spix y Agassiz` (two). See `src/pipeline/double_surnames.rs` for the loader and the matching
(word before the `y`, first non-particle word after it, case and accents ignored).

Built in October 2026 (164 people) from three sources:

- **ChecklistBank and COL**: every capitalised `X y Y` in the authorships of the ChecklistBank
  verbatim names (3,588 rows, 376 spellings) and COL (697 rows), reviewed by hand into persons
  and two-person teams. The spellings found for a person (`Bolivar y Pidtain`, `Dus. y Alon.`,
  `Caballero y CF.`) are its variants. This is where the zoologists come from (Bolívar y Pieltain,
  Bofill y Poch, Junco y Reyes, Lizer y Trelles, …), which make up most of the rows.
- **Wikidata**: taxon authors (with an IPNI author ID P586, a ZooBank author ID P2006, a zoological
  author citation P835, a botanist author abbreviation P428 or a Harvard Index ID P6264) whose
  Spanish label holds a "y" between two surnames, labels and aliases as variants; institutions
  dropped. The Wikidata item, IPNI and ZooBank ids are recorded.
- **The CLB person registry** (`CatalogueOfLife/backend`, branch `feat/person-author-comparison`,
  `core/src/test/resources/authorship/persons/{persons,names}.tsv`): persons whose family name or
  name forms hold a "y". It is built from IPNI and Wikidata, so it adds botanists; it has no
  ZooBank yet and lacks the zoologists above.

Every person identified that way in ChecklistBank and COL is listed; a handful of rarely cited
pairs that could be either (`Ferrer y Hernández`, `Marrero y Galindez`, 1–4 rows each) are not. Not
listed, deliberately: two people written with `y` (`Spix y Agassiz`, `Quoy y Gaimard`, `Rivero y
Serna`, `Núñez y Barro`, `Expósito Hermosa y Martínez Borrego`). To add a person, add a line with
the surnames as cited and any misspelt or abbreviated forms seen in the data as variants; spellings
that differ only in case or accents need no variant.
