#!/usr/bin/env python3
"""Streaming analyzer for the 67.5M verbatim corpus CLI output (JSONL on stdin).

Characterises the "informal / semistructured" band to inform the 5.0.0 ParseResult
type-model decision: how common is it, what sub-bands exist, what the anchor ranks and
phrase forms look like, and reservoir samples of edge cases per sub-band.
"""
import sys, json, collections, random

random.seed(42)
SAMPLE_CAP = 60

total = 0
outcome = collections.Counter()          # parsed:<TYPE> / error:<TYPE>
band = collections.Counter()             # semistructured sub-band
band_anchor_rank = collections.Counter() # rank of the anchor within the band
band_anchor_kind = collections.Counter() # genus-anchored vs uninomial(higher)-anchored
qualifier_vals = collections.Counter()   # epithetQualifier values seen
phrase_head = collections.Counter()      # leading token of the phrase
# cross-cutting prevalence across ALL parsed rows:
parsed_with_phrase = 0
parsed_with_qualifier = 0
parsed_indet = 0
samples = collections.defaultdict(list)  # sub-band -> reservoir sample of inputs
seen_per_band = collections.Counter()

def reservoir(bucket, item):
    seen_per_band[bucket] += 1
    s = samples[bucket]
    if len(s) < SAMPLE_CAP:
        s.append(item)
    else:
        j = random.randint(0, seen_per_band[bucket]-1)
        if j < SAMPLE_CAP:
            s[j] = item

def has(p, k):
    v = p.get(k)
    return v not in (None, "", [], {})

for line in sys.stdin:
    total += 1
    if total % 5_000_000 == 0:
        sys.stderr.write(f"  ...{total:,} rows\n"); sys.stderr.flush()
    try:
        o = json.loads(line)
    except Exception:
        outcome["BADJSON"] += 1
        continue
    inp = o.get("input", "")
    err = o.get("error")
    if err is not None:
        et = err.get("type", "?")
        outcome[f"error:{et}"] += 1
        # anchored-looking errors are candidates for the new class; sample the informal/placeholder/other
        if et in ("INFORMAL", "PLACEHOLDER", "OTHER", "FORMULA"):
            reservoir(f"ERR:{et}", inp)
        continue
    p = o.get("parsed")
    if p is None:
        outcome["NOPARSE"] += 1
        continue
    t = p.get("type", "?")
    outcome[f"parsed:{t}"] += 1

    hp = has(p, "phrase")
    hq = has(p, "epithetQualifier")
    he = has(p, "specificEpithet") or has(p, "infraspecificEpithet") or has(p, "cultivarEpithet")
    anchor_genus = has(p, "genus")
    anchor_uni = has(p, "uninomial")
    is_indet = (anchor_genus or anchor_uni) and not he and not hp
    if hp: parsed_with_phrase += 1
    if hq: parsed_with_qualifier += 1
    if is_indet: parsed_indet += 1

    # focus band: everything the parser flags INFORMAL, plus any phrase-bearing parsed row
    if t == "INFORMAL" or hp:
        if he and not hp:
            sub = "1:determined+annotated"   # complete epithet, informal only via qualifier/dangling marker
        elif hp:
            sub = "3:phrase"                 # anchor + non-code phrase
        elif anchor_genus or anchor_uni:
            sub = "2:indetermined"           # anchor + rank, no terminal epithet, no phrase
        else:
            sub = "0:informal-other"
        band[sub] += 1
        band_anchor_rank[(sub, p.get("rank"))] += 1
        band_anchor_kind[(sub, "uninomial(higher)" if (anchor_uni and not anchor_genus) else ("genus" if anchor_genus else "none"))] += 1
        if hq:
            for v in (p.get("epithetQualifier") or {}).values():
                qualifier_vals[v] += 1
        if hp:
            ph = str(p.get("phrase") or "")
            phrase_head[ph.split()[0] if ph.split() else ""] += 1
        reservoir(sub, inp + (f"   ‹phrase={p.get('phrase')!r} rank={p.get('rank')} qual={p.get('epithetQualifier')}›"))

# ---- report ----
out = []
out.append(f"TOTAL rows: {total:,}\n")
out.append("=== outcome distribution (top 25) ===")
for k, n in outcome.most_common(25):
    out.append(f"  {n:>12,}  {100*n/total:6.3f}%  {k}")
out.append("")
out.append("=== cross-cutting prevalence across ALL parsed rows ===")
out.append(f"  parsed with a phrase:        {parsed_with_phrase:>12,}")
out.append(f"  parsed with a qualifier:     {parsed_with_qualifier:>12,}")
out.append(f"  parsed indetermined:         {parsed_indet:>12,}")
out.append("")
out.append("=== SEMISTRUCTURED band sub-bands ===")
band_total = sum(band.values())
out.append(f"  band total: {band_total:,}  ({100*band_total/total:.3f}% of all rows)")
for k, n in band.most_common():
    out.append(f"  {n:>12,}  {100*n/band_total:6.2f}% of band   {k}")
out.append("")
out.append("=== anchor kind per sub-band (genus vs higher uninomial) ===")
for (sub, kind), n in sorted(band_anchor_kind.items()):
    out.append(f"  {sub:24} {kind:20} {n:>12,}")
out.append("")
out.append("=== anchor rank per sub-band (top 30) ===")
for (sub, rank), n in sorted(band_anchor_rank.items(), key=lambda x:-x[1])[:30]:
    out.append(f"  {sub:24} rank={str(rank):16} {n:>12,}")
out.append("")
out.append("=== qualifier values (top 20) ===")
for v, n in qualifier_vals.most_common(20):
    out.append(f"  {n:>10,}  {v!r}")
out.append("")
out.append("=== phrase leading token (top 30) ===")
for v, n in phrase_head.most_common(30):
    out.append(f"  {n:>10,}  {v!r}")
out.append("")
out.append("=== reservoir samples per bucket ===")
for bucket in sorted(samples):
    out.append(f"--- {bucket}  (n≈{seen_per_band[bucket]:,}) ---")
    for s in samples[bucket]:
        out.append(f"    {s}")
    out.append("")

print("\n".join(out))
