#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""How well does code inference agree with the code a dataset declares?

Samples ChecklistBank rows that carry both an authorship and a nomenclatural code
(`testdata/clb-verbatim-names.tsv`), parses each WITHOUT the code hint — as ChecklistBank does for
datasets that declare none — and compares the inferred code with the declared one:

  correct   inferred == declared
  wrong     inferred != declared (a false inference — the costly kind)
  none      nothing inferred

Rows are bucketed by authorship shape (year with/without comma, ex-author, hybrid sign, …) so a
change to one inference rule can be judged on the rows it touches. `--examples N` prints wrong
inferences per bucket.

    crates/nameparser-py/.venv/bin/python -I tools/code_inference_eval.py --sample 200000
"""
import argparse
import collections
import os
import random
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from path_divergence import CODE_OF, rank_hint  # noqa: E402

DECLARED = {"ZOOLOGICAL", "BOTANICAL", "BACTERIAL", "VIRUS"}


def buckets(name, authorship):
    """The authorship-shape buckets a row falls into."""
    full = f"{name} {authorship}"
    out = []
    if re.search(r",\s*\(?\d{4}\)?\s*$|,\s*\d{4}\b", authorship):
        out.append("year after comma")
    elif re.search(r"\b\d{4}\b", authorship):
        out.append("year without comma")
    else:
        out.append("no year")
    if re.search(r"\bex\.?\s", authorship):
        out.append("ex-author")
    if re.search(r"[×]|\bnotho|(?:^|\s)[xX]\s", full):
        out.append("hybrid")
    if re.search(r"\b(?:emend|corrig)\.?\b|Approved Lists", authorship):
        out.append("emend/corrig/Approved Lists")
    if re.search(r"\b(?:in|apud)\s+\S", authorship):
        out.append("in-citation")
    if re.search(r"\(.*\)", authorship):
        out.append("basionym")
    if re.search(r"\b[A-Z][a-z]{0,5}\.(?:\s|$)", authorship):
        out.append("abbreviated author")
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("tsv", nargs="?", default="testdata/clb-verbatim-names.tsv")
    ap.add_argument("--sample", type=int, default=200_000)
    ap.add_argument("--seed", type=int, default=7)
    ap.add_argument("--examples", type=int, default=0)
    args = ap.parse_args()

    import nameparser

    rng = random.Random(args.seed)
    sample, seen = [], 0
    with open(args.tsv, encoding="utf-8", errors="replace") as fh:
        next(fh)
        for line in fh:
            c = line.rstrip("\n").split("\t")
            if len(c) < 5 or not c[1].strip() or not c[2].strip():
                continue
            declared = CODE_OF.get(c[4].strip().lower())
            if declared not in DECLARED:
                continue
            seen += 1
            row = (c[1], c[2], rank_hint(c[3]), declared)
            if len(sample) < args.sample:
                sample.append(row)
            elif (j := rng.randrange(seen)) < args.sample:
                sample[j] = row

    total = collections.Counter()
    by_code = collections.defaultdict(collections.Counter)
    per = collections.defaultdict(collections.Counter)
    wrong = collections.defaultdict(list)
    for name, authorship, rank, declared in sample:
        try:
            r = nameparser.parse(name, authorship=authorship, rank=rank)
            inferred = r.code
        except nameparser.UnparsableNameError as e:
            inferred = e.code
        outcome = "none" if inferred is None else ("correct" if inferred == declared else "wrong")
        total[outcome] += 1
        by_code[declared][outcome] += 1
        for b in buckets(name, authorship):
            per[b][outcome] += 1
            if outcome == "wrong" and len(wrong[b]) < args.examples:
                wrong[b].append(f"{name!r} + {authorship!r}: {inferred}, declared {declared}")

    def summary(label, c):
        n = sum(c.values())
        pct = lambda k: 100.0 * c[k] / max(n, 1)
        return (f"{label:30} {n:8}  correct {pct('correct'):5.1f}%  wrong {pct('wrong'):5.2f}%"
                f"  none {pct('none'):5.1f}%")

    print(f"{seen} rows with an authorship and a declared code, sampled {len(sample)}")
    print(summary("all", total))
    for code, c in sorted(by_code.items(), key=lambda kv: -sum(kv[1].values())):
        print(summary(f"declared {code}", c))
    for b, c in sorted(per.items(), key=lambda kv: -sum(kv[1].values())):
        print(summary(b, c))
    for b, ex in wrong.items():
        print(f"\n== wrong: {b}")
        for e in ex:
            print("  " + e)


if __name__ == "__main__":
    main()
