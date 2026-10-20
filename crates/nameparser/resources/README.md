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

## Name whitelist (`whitelist-epithets.tsv`)

Real **names** whose epithet is on the blacklist: *Caryota no* Becc., *Hypanus say* (Lesueur,
1817), *Agra not* Erwin, 2002. The words stay blacklisted, since they turn up as junk far more often
than as epithets; a listed name is not flagged. A listed epithet that is also an author particle
(`du`, `das`, `los`) is read as the epithet rather than as part of the author: `Sympycnus du
Curran, 1929` was the genus *Sympycnus* by `du Curran`. Rust-only: the Java name-parser has no such
list. See `whitelisted` in `src/pipeline/blacklisted_epithets.rs` and `particle_is_epithet` in
`src/pipeline/authorship_split.rs`.

Each line gives the epithet, its genus and its original author (the basionym author when there is
one, else the author), plus the name as found. A name matches when its epithet does and either its
genus or its original author does. The genus keeps `Abies du` flagged while `Sympycnus du` is not; the
author carries a line across recombinations (`Raja say Lesueur` also covers *Hypanus say*
(Lesueur, 1817)). The author is compared by surname without case or accents.

Seeded in October 2026 (#116) from the ChecklistBank verbatim names with an authorship and a
blacklisted epithet that match a name in the COL release exactly (33 names), plus *Ficus du*
K.Schum. & Lauterb. from WFO. To add a name, check it in a checklist (COL, WFO, a taxonomic
database) and add a line for it.

The eight species of the COL release with the epithet `spec` were added too (#155): an undotted
`spec` with an authorship after it is the epithet anyway, but a listed genus keeps it the epithet
when the name comes without one ("Trichosternus spec"), where any other `Genus spec` is an
indeterminate species.

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

## Citation title words (`citation-title-words.txt`)

Words of the title of a work, abbreviated or not (`dict`, `sci`, `nat`, `fl`, `prodr`, `herb`), that
are no author's surname. An `in` citation whose text holds one of them names a reference rather than
an author team ("Mill. in Gard. Dict.", "Lesson in Ann. Sci. Nat."), so it gets no in-authors; see
`src/pipeline/in_citation.rs` for how a citation is read and the other signs of a reference (digits,
a colon, brackets, a possessive, a bare initial).

Built in October 2026 (#172) from the 29,783 distinct `in` citations of the ChecklistBank verbatim
names: their most frequent abbreviated words, checked by hand against the citations holding them,
plus common journal and book title words. Words that abbreviate authors as well stay out (`bull`
Bulliard, `linn` Linnaeus, `ser` Seringe, `rich` Richard, `cl` Cleve, `lec` LeConte, `deg` Degener),
as the file's header says. A word is matched against each part of a parsed author split at dots,
hyphens and spaces, lower-cased.

The list also tells where a cited work's title starts after the authors' comma (`Nees, Syn. Hepat.`,
#147), together with the title abbreviations below.

## Title abbreviations (`title-abbreviations.txt`)

More abbreviated words of titles (`diagn`, `notizbl`, `tijdschr`), and the first two words of titles
whose first word abbreviates an author too (`bull misc` for Bull. Misc. Inform. Kew, where `Bull.` is
also Bulliard). After the last comma of an authorship, a run of abbreviated words that opens like this
or with a word of `citation-title-words.txt` is the reference, not one more author; see
`strip_abbreviated_title` in `src/pipeline/stripandstash.rs`.

Generated in October 2026 (#147) by `scripts/build-title-abbreviations.py`, which documents the
rules, from the publication titles of an IPNI dump of 2017-11-06 (1.7M names, 31k titles): a word
abbreviated in three titles or more that no author abbreviation in IPNI or TL-2 (the TL-2 authors
export, Smithsonian Libraries) abbreviates with a full stop. Nothing from TL-2 is in the file; it only
keeps words out. Titles of persons and places (`dr`, `prof`, `mt`) are left out by hand in the script.
Regenerate rather than edit.

## Basionym authors (`basionym-authors.txt`)

Basionym authors written in full (`Bentham`, `Nitschke`, `Linnaeus`). In brackets after a genus,
one of them is the basionym author of that genus, even with a year after the combination author:
`Eutypella (Nitschke) Sacc., 1875`, `Clathrotropis (Bentham) Harms in Dalla Torre & Harms, 1901`.
Any other single word there, with a year, is a zoological subgenus: `Aedes (Stegomyia) Theobald,
1901`. See `find_boundary` in `src/pipeline/authorship_split.rs`.

Generated in October 2026 by `scripts/build-basionym-authors.py`, which documents the rules, from
the ChecklistBank verbatim names (`testdata/clb-verbatim-names.tsv`). An author is listed when it
opens a separately given authorship as its bracketed basionym in twenty rows or more, and names a
bracketed subgenus in fewer than five. Authors that are subgenera as often (`Mitra`, `Costa`, `Hua`)
stay out. Regenerate rather than edit.
