#!/usr/bin/env python3
"""Builds crates/nameparser/resources/basionym-authors.txt from the ChecklistBank verbatim names.

    scripts/build-basionym-authors.py testdata/clb-verbatim-names.tsv > \
        crates/nameparser/resources/basionym-authors.txt

The input has the columns rowType, scientificName, authorship, rank and code. A word is listed
when it is the basionym author of an authorship given apart from its name in twenty rows or more,
written in full in the brackets on its own ("(Bentham) Harms", "(Kunth) Baker, 1881"), and it names
a bracketed subgenus in fewer than five ("Mitra" and "Costa" are subgenera as often as authors).
Such a word in brackets after a genus is the basionym author of that genus, not a subgenus, even
with a year after the author: "Eutypella (Nitschke) Sacc., 1875".
"""

import collections
import re
import sys

# a word written in full: a capital, then lower-case letters
WORD = r"[^\W\d_a-z][^\W\d_A-Z]+"
BASIONYM = re.compile(rf"\(({WORD})\) \S")
SUBGENUS = re.compile(rf"[^\W\d_a-z][^\W\d_A-Z]+ \(({WORD})\)(?: [a-z]|$)")
MIN_AUTHOR_ROWS = 20
MAX_SUBGENUS_ROWS = 4


def main(path):
    authors = collections.Counter()
    subgenera = collections.Counter()
    with open(path, encoding="utf-8") as f:
        next(f)
        for line in f:
            cols = line.rstrip("\n").split("\t")
            if len(cols) < 3:
                continue
            m = BASIONYM.match(cols[2])
            if m:
                authors[m.group(1)] += 1
            m = SUBGENUS.match(cols[1])
            if m:
                subgenera[m.group(1)] += 1
    words = sorted(
        w
        for w, n in authors.items()
        if n >= MIN_AUTHOR_ROWS and subgenera[w] <= MAX_SUBGENUS_ROWS
    )
    print("# Basionym authors written in full, which in brackets after a genus are no subgenus:")
    print('# "Eutypella (Nitschke) Sacc., 1875". Generated from the ChecklistBank verbatim names by')
    print("# scripts/build-basionym-authors.py, which says how; do not edit by hand. See README.md.")
    for w in words:
        print(w)


if __name__ == "__main__":
    main(sys.argv[1])
