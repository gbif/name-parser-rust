# SPDX-License-Identifier: Apache-2.0
"""Shared corpus-reading helpers for the Phase 4a Python-binding tests (Task 3).

`extract_name`/`read_corpus` port `extract_name` in `crates/nameparser-cli/src/main.rs`
byte-for-byte: a plain-text line is a name if it is non-empty, does not start with '#',
and — after taking the substring before the first TAB and trimming — is not empty and is
not the literal header `scientificName`. Verified independently (before this file existed)
to reproduce the known-good per-corpus counts recorded in `cross-validation.md` / the
Phase 3 Java `ParityTest` report: 8017 + 14 + 4 + 13 + 20 + 8 + 3226 = 11,302.

Used by both `test_parity.py` (the full-corpus oracle diff) and
`test_getter_consistency.py` (a representative sample drawn from the same corpora), so
every test in this directory reads the seven `testdata/*.txt` files identically to the
Rust and Java CLIs.
"""
from __future__ import annotations

from pathlib import Path

# This file lives at crates/nameparser-py/python/tests/_corpus.py, so the repo root
# (containing testdata/) is 4 parents up. Asserted below rather than trusted blindly, so a
# future move of this file fails loudly instead of silently reading an empty corpus dir.
REPO_ROOT = Path(__file__).resolve().parents[4]
TESTDATA_DIR = REPO_ROOT / "testdata"

if not TESTDATA_DIR.is_dir():
    raise RuntimeError(
        f"testdata/ not found at {TESTDATA_DIR!s} — the parents[4] relative-path "
        f"resolution in {__file__} no longer matches this file's location in the repo tree"
    )

# Same 7 corpora, same order, as Phase 2/3's own cross-validation (cross-validation.md) and
# scripts/cross-validate.sh's default corpus list.
CORPORA: list[str] = [
    "benchmark-data",
    "names-with-authors",
    "hybrids",
    "other",
    "otu",
    "placeholder",
    "viruses",
]


def extract_name(raw: str) -> str | None:
    """Port of `extract_name` in `crates/nameparser-cli/src/main.rs`: a raw line is
    skipped (returns None) if it is empty or starts with '#'. Otherwise the name is the
    substring before the first TAB, trimmed — also skipped if that trim leaves it empty,
    or leaves exactly the literal header `scientificName`."""
    if raw == "" or raw.startswith("#"):
        return None
    name = raw.split("\t", 1)[0].strip()
    if name == "" or name == "scientificName":
        return None
    return name


def read_corpus(corpus: str) -> list[str]:
    """Reads `testdata/<corpus>.txt` and returns the extracted names, in file order — the
    same set and order `nameparser-cli parse` / the Java CLI's plain-text reader produce.

    Splits on '\\n' only (NOT `str.splitlines()`, which also breaks on several Unicode
    line-separator code points Rust's `BufRead::lines()` does not treat as line
    boundaries), stripping one trailing '\\r' per line so CRLF input is tolerated the same
    way Rust's `lines()` is. A trailing empty element after a final '\\n' (an artifact of
    `str.split`, absent from Rust's `lines()`) is harmless here: `extract_name` maps it to
    `None`, the same as `lines()` simply never emitting it.
    """
    text = (TESTDATA_DIR / f"{corpus}.txt").read_text(encoding="utf-8")
    names: list[str] = []
    for raw in text.split("\n"):
        if raw.endswith("\r"):
            raw = raw[:-1]
        name = extract_name(raw)
        if name is not None:
            names.append(name)
    return names
