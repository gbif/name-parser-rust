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
