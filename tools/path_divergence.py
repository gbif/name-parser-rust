#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""How often do the two authorship paths disagree on ChecklistBank's own rows?

ChecklistBank parses a record as `parse(scientificName, authorship, rank, code)`. This samples rows
that carry an authorship from `testdata/clb-verbatim-names.tsv` (columns rowType, scientificName,
authorship, rank, code) and compares that call with the same name and authorship joined into one
string, `parse("scientificName authorship", None, rank, code)` — or with the name alone when it
already carries an authorship of its own (sources repeat it, not always identically). It reports
how many rows differ, per field, with examples: the real-world counterpart of
`crates/nameparser/tests/common/path_divergences.tsv`.

`--dump FILE` writes the separate-path result of every sampled row as JSONL; `--diff OLD NEW`
compares two such dumps (same seed and sample size), to count the rows an engine change moves.

Runs on the Python binding (`maturin develop` in crates/nameparser-py), e.g.
    crates/nameparser-py/.venv/bin/python -I tools/path_divergence.py --sample 200000
"""
import argparse
import collections
import json
import random
import sys

CODES = {
    "ZOOLOGICAL": ["iczn", "zoological"],
    "BOTANICAL": ["icn", "icbn", "icnafp", "botanical", "ibc"],
    "BACTERIAL": ["icnp", "icnb", "bacterial", "ibnc"],
    "VIRUS": ["icvcn", "ictv", "icvn", "virus"],
    "CULTIVARS": ["icncp", "cultivars"],
}
CODE_OF = {alias: wire for wire, aliases in CODES.items() for alias in aliases}
RANK_ABBREVIATIONS = {
    "f.": "FORM", "forma": "FORM", "fo.": "FORM", "var.": "VARIETY", "variedad": "VARIETY",
    "subsp.": "SUBSPECIES", "ssp.": "SUBSPECIES", "ser.": "SERIES", "sect.": "SECTION",
    "subg.": "SUBGENUS", "subfamilia": "SUBFAMILY", "familia": "FAMILY",
}


def rank_hint(rank):
    r = rank.strip().lower()
    if not r:
        return None
    return RANK_ABBREVIATIONS.get(r, r.upper().replace(" ", "_").replace("-", "_"))


def code_hint(code):
    return CODE_OF.get(code.strip().lower())


def sample_rows(path, size, seed):
    """Reservoir sample of `size` rows with a scientificName and an authorship."""
    rng = random.Random(seed)
    sample, seen = [], 0
    with open(path, encoding="utf-8", errors="replace") as fh:
        next(fh)  # header
        for line in fh:
            cols = line.rstrip("\n").split("\t")
            if len(cols) < 5 or not cols[1].strip() or not cols[2].strip():
                continue
            seen += 1
            row = (cols[1], cols[2], rank_hint(cols[3]), code_hint(cols[4]))
            if len(sample) < size:
                sample.append(row)
            else:
                j = rng.randrange(seen)
                if j < size:
                    sample[j] = row
    return sample, seen


def parse(nameparser, name, authorship, rank, code):
    try:
        r = nameparser.parse(name, authorship=authorship, rank=rank, code=code)
        return {"informal" if isinstance(r, nameparser.Informal) else "parsed": r.to_dict()}
    except nameparser.UnparsableNameError as e:
        return {"unparsable": {"type": e.name_type, "code": e.code}}


def carries_authorship(result):
    d = result.get("parsed")
    if not d:
        return False
    def has(a):
        return bool(a.get("authors") or a.get("exAuthors") or a.get("year") or a.get("anonymous"))
    return has(d["combinationAuthorship"]) or has(d["basionymAuthorship"])


def flat(result):
    """`{field: value}` of a parse result, with its kind as the `kind` field."""
    (kind, d), = result.items()
    out = {"kind": kind}
    for k, v in d.items():
        out[k] = json.dumps(sorted(v) if k in ("warnings", "notho") else v, sort_keys=True)
    return out


def differing(a, b):
    fa, fb = flat(a), flat(b)
    return sorted(k for k in set(fa) | set(fb) if fa.get(k) != fb.get(k))


def report(pairs, examples, label_a, label_b):
    per_field, shown = collections.Counter(), collections.defaultdict(list)
    diverging = 0
    for key, a, b in pairs:
        fields = differing(a, b)
        if not fields:
            continue
        diverging += 1
        per_field.update(fields)
        for f in fields:
            if len(shown[f]) < examples:
                fa, fb = flat(a), flat(b)
                shown[f].append(f"{key}\n        {label_a}: {fa.get(f, '-')}\n        {label_b}: {fb.get(f, '-')}")
    n = len(pairs)
    print(f"compared {n}, differing {diverging} ({100.0 * diverging / max(n, 1):.2f}%)")
    for f, c in per_field.most_common():
        print(f"  {f:28} {c:8}  ({100.0 * c / max(n, 1):.2f}%)")
    for f, _ in per_field.most_common():
        print(f"\n== {f}")
        for e in shown[f]:
            print(f"  {e}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("tsv", nargs="?", default="testdata/clb-verbatim-names.tsv")
    ap.add_argument("--sample", type=int, default=200_000, help="rows with an authorship to compare")
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--examples", type=int, default=5, help="examples shown per differing field")
    ap.add_argument("--dump", help="write the separate-path result of every sampled row as JSONL")
    ap.add_argument("--diff", nargs=2, metavar=("OLD", "NEW"), help="compare two dumps instead")
    args = ap.parse_args()

    if args.diff:
        with open(args.diff[0]) as fa, open(args.diff[1]) as fb:
            pairs = []
            for la, lb in zip(fa, fb):
                ra, rb = json.loads(la), json.loads(lb)
                key = f"`{ra['name']}` + `{ra['authorship']}` ({ra['rank']}, {ra['code']})"
                pairs.append((key, ra["result"], rb["result"]))
        report(pairs, args.examples, "old", "new")
        return

    import nameparser

    rows, seen = sample_rows(args.tsv, args.sample, args.seed)
    print(f"{seen} rows with an authorship, sampled {len(rows)} (seed {args.seed})", file=sys.stderr)
    dump = open(args.dump, "w") if args.dump else None
    pairs = []
    for name, authorship, rank, code in rows:
        separate = parse(nameparser, name, authorship, rank, code)
        alone = parse(nameparser, name, None, rank, code)
        # a name that already carries its authorship (the source repeats it, not always
        # identically) is its own embedded form
        embedded = alone if carries_authorship(alone) else parse(
            nameparser, f"{name} {authorship}", None, rank, code)
        pairs.append((f"`{name}` + `{authorship}` ({rank}, {code})", separate, embedded))
        if dump:
            dump.write(json.dumps({"name": name, "authorship": authorship, "rank": rank,
                                   "code": code, "result": separate}, sort_keys=True) + "\n")
    report(pairs, args.examples, "separate", "embedded")


if __name__ == "__main__":
    main()
