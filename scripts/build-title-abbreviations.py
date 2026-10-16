#!/usr/bin/env python3
"""Builds crates/nameparser/resources/title-abbreviations.txt from IPNI and TL-2 (#147).

    scripts/build-title-abbreviations.py IPNI_DUMP.sql TL2_AUTHORS.csv > \
        crates/nameparser/resources/title-abbreviations.txt

IPNI_DUMP.sql is a PostgreSQL dump of IPNI's `names` table (ipni-2017-11-06.sql) and
TL2_AUTHORS.csv the TL-2 authors export (tl2-authors.csv, Smithsonian Libraries). Nothing from TL-2
is written out: its author abbreviations only keep words out of the list.

Two kinds of line:
- a word abbreviated with a full stop in three or more titles IPNI cites (`diagn`, `notizbl`),
  which no author abbreviation in IPNI or TL-2 uses with a full stop, and which is not in
  citation-title-words.txt already;
- the first two words of a title whose first word abbreviates an author too (`bull misc` for
  Bull. Misc. Inform. Kew, where Bull. is also Bulliard), cited in three names or more.
"""

import collections
import csv
import re
import sys
from pathlib import Path

# an abbreviated word: letters, then a full stop
DOTTED = re.compile(r"(?<![\w'-])([^\W\d_][\w'-]+)\.")
# a title's first word with its full stop, then its second word
OPENING = re.compile(r"\s*([^\W\d_][\w'-]+)\.\s*([^\W\d_][\w'-]*)")
# notes, titles of persons and places, not titles of works: "cf.", "Dr.", "Mt. Yun"
NOT_TITLES = {"cf", "not", "an", "dr", "mr", "mrs", "prof", "mt"}
MIN_TITLES = 3
MIN_NAMES = 3

RESOURCES = Path(__file__).resolve().parent.parent / "crates/nameparser/resources"


def ipni(dump):
    """Names citing each publication title, and the author abbreviations, from the dump."""
    titles = collections.Counter()
    authors = set()
    i_pub = i_auth = None
    with open(dump, encoding="utf-8") as f:
        for line in f:
            if i_pub is None:
                if line.startswith("COPY names ("):
                    cols = [c.strip() for c in line[12 : line.index(")")].split(",")]
                    i_pub = cols.index("publication_s_lower")
                    i_auth = cols.index("authors_t")
                continue
            if line.startswith("\\."):
                break
            row = line.rstrip("\n").split("\t")
            # the title in lower case
            if row[i_pub] != "\\N":
                titles[row[i_pub]] += 1
            if row[i_auth] != "\\N":
                authors.add(row[i_auth])
    return titles, authors


def author_words(ipni_authors, tl2_csv):
    words = set()
    for a in ipni_authors:
        words.update(w.lower() for w in DOTTED.findall(a))
    with open(tl2_csv, encoding="utf-8-sig") as f:
        for r in csv.DictReader(f):
            for k in ("tl2_abbreviation", "tl2_abbreviation_alt"):
                words.update(w.lower() for w in DOTTED.findall(r[k] or ""))
    return words


def main(dump, tl2_csv):
    titles, ipni_authors = ipni(dump)
    in_authors = author_words(ipni_authors, tl2_csv)
    known = {
        w.strip()
        for w in (RESOURCES / "citation-title-words.txt").read_text().splitlines()
        if w.strip() and not w.startswith("#")
    }
    in_titles = collections.Counter()
    for t in titles:
        in_titles.update({w.lower() for w in DOTTED.findall(t)})
    words = sorted(
        w
        for w, n in in_titles.items()
        if n >= MIN_TITLES and w not in in_authors and w not in known and w not in NOT_TITLES
    )
    openings = collections.Counter()
    for t, n in titles.items():
        m = OPENING.match(t)
        if not m:
            continue
        first, second = m.group(1).lower(), m.group(2).lower()
        if first in in_authors and first not in known and first not in NOT_TITLES and second:
            openings[f"{first} {second}"] += n
    pairs = sorted(p for p, n in openings.items() if n >= MIN_NAMES)
    print("# Abbreviated words of titles, and the opening words of titles whose first word abbreviates")
    print("# an author too. Generated from IPNI (2017-11-06) by scripts/build-title-abbreviations.py,")
    print("# which says how; do not edit by hand. Lower case, without the full stop. See README.md.")
    for line in words + pairs:
        print(line)


if __name__ == "__main__":
    main(*sys.argv[1:3])
