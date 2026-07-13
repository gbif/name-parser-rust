# SPDX-License-Identifier: Apache-2.0
"""Corpus parity test (Phase 4a, Task 3) — the gate proving the Python binding produces
byte-for-byte the core's `parse()` output over this repo's whole test corpus.

For every one of the ~11,302 names across `testdata/{benchmark-data,names-with-authors,
hybrids,other,otu,placeholder,viruses}.txt`, this diffs `nameparser.parse(name).to_dict()`
against an independent oracle row produced by running the SAME corpus file through the
Java `name-parser-cli` shaded jar (falling back to this repo's own Rust `nameparser-cli`
release binary if the jar isn't available) — the identical oracle Phase 2/3's own
CLI-vs-CLI parity tests used. Since the Python binding calls the exact same
`nameparser::parse` the Rust/Java CLIs already cross-validated field-for-field, this is
expected to be inherently 0 diffs; a diff here points at the *binding*'s plumbing (a
serde/pythonize edge, an off-by-one in reading the corpus, …), not the core parser.

Run (from the repo root, inside the project venv, after `maturin develop --release`):

    . .venv/bin/activate
    maturin develop --release -m crates/nameparser-py/Cargo.toml
    pytest crates/nameparser-py/python/tests/test_parity.py -v -s

(`-s` shows the per-corpus + total tally that otherwise prints to stdout pytest would
normally capture and hide on a passing run.)

## Comparator

A Python port of `crates/nameparser-cli/src/main.rs`'s `diff_element` +
`canonicalize_for_key` (the same recursive JSON-tree comparator Phase 2's `compare`
subcommand and Phase 3's Java `ParityTest` both use): objects compare by the union of both
sides' keys (a missing key and an explicit JSON `null` compare equal, matching
`Value::Null`-defaulting semantics on both the Rust and Java sides); arrays compare
positionally; `warnings`/`notho`/`epithetQualifier` — [`UNORDERED_FIELD_KEYS`], copied
verbatim from the Rust CLI's constant of the same name — are, if array-shaped, sorted by
each element's rendered JSON text before comparing, at every nesting depth, because
`warnings` is backed by a Java `HashSet<String>` (insertion order is not guaranteed) on the
oracle side and an insertion-order `Vec<String>` on the Rust/Python side. This intentionally
skips porting the Rust source's `json_eq` fast-path: that function is a pure "equal
subtrees don't need to recurse for diff-reporting" performance optimization and changes no
observable result (see the reasoning recorded in this repo's history — a diff is reported
for a key/index if and only if a leaf under it differs, with or without the fast path).

## Unparsable names: message equality (not a reconstructed message), plus structured type/code

For a name neither side can parse, the Python binding raises `UnparsableNameError` whose
`str()` is exactly the core's `ParseError.message` (`crates/nameparser-py/src/lib.rs`'s
`parse()` forwards `e.message` verbatim, unmodified, into `UnparsableNameError::new_err`).
This test compares that string against the oracle row's `error.message` **directly** —
deliberately NOT by reconstructing an expected `f"Unparsable {type} name: {name}"` string
from the oracle's `error.type` and the corpus-extracted name and asserting equality against
*that*. Reconstruction was the first design tried here, and empirically it is wrong for a
small but real slice of the corpus: the core's `ParseError::new(type_, code, name)` is
*not* always called with the verbatim corpus input as `name` — five rows in
`benchmark-data.txt` (all BOLD/SH barcode-style OTU codes, e.g. `"Coleoptera sp.
BOLD:AAV0432"`) produce a message that echoes only the extracted barcode substring (and, in
one case, a case-folded variant), e.g. `"Unparsable OTHER name: BOLD:AAV0432"` — not
`"Unparsable OTHER name: Coleoptera sp. BOLD:AAV0432"`. A reconstructed-message assertion
would misreport those five as binding bugs. Comparing the two sides' *actual* message
strings directly sidesteps this entirely (no assumption about what the trailing text should
be).

**Phase 4a Task 4 closes the Task 3 review's `[Important]` finding**: `UnparsableNameError`
now carries structured `.name_type`/`.code` attributes (see `crates/nameparser-py/src/lib.rs`'s
`unparsable_name_error`), so this test independently asserts `exc.name_type == error["type"]`
and `exc.code == error.get("code")` against every one of the ~6,609 error rows in the corpus,
not just a hand-picked example — a real, narrow gap message-equality alone could not close
(message equality soundly implies `NameType` agreement, since `java_name(&type_)` is injective
per variant and is the message's leading token — see `crates/nameparser/src/model/mod.rs`'s
`ParseError::new` — but `code` is a separate struct field never interpolated into the message at
all, so it was structurally unobservable from the message string alone). `error.get("code")`
(not `error["code"]`) because the oracle envelope *omits* the `"code"` key entirely rather than
emitting `null` when the core `ParseError`'s `code` is `None` — confirmed directly for the
Rust-CLI-fallback oracle by `crates/nameparser-cli/src/main.rs`'s own
`render_row_matches_the_documented_error_shape_and_omits_code_when_absent` test, and for the
Java oracle by the pre-existing Phase 2/3 CLI-level parity gate, which already walks `error.code`
as an ordinary JSON leaf (0 diffs across the same corpus) — so both producers are already known
to agree on when the key is present vs. omitted. `exc.name` (the core `ParseError.name`, which —
per the BOLD/SH example above — is not always the verbatim corpus input) has no independent
oracle field to compare against beyond the message body itself, so it is intentionally left to
the existing message-equality check rather than given a redundant, message-derived assertion of
its own.
"""
from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path
from typing import Callable

import pytest

import nameparser
from _corpus import CORPORA, REPO_ROOT, read_corpus

JAVA_SHADED_JAR = Path(
    "/Users/markus/code/gbif/name-parser/name-parser-cli/target/"
    "name-parser-cli-4.2.0-SNAPSHOT-shaded.jar"
)
RUST_CLI = REPO_ROOT / "target" / "release" / "nameparser-cli"

# Verbatim copy of crates/nameparser-cli/src/main.rs's UNORDERED_FIELD_KEYS (Phase 2's
# `compare` subcommand) — the same 3 JSON field names Phase 2/3's own parity tooling
# already treats as set/map-shaped (backed by a Java HashSet/EnumSet/EnumMap) rather than
# a positionally-ordered array.
UNORDERED_FIELD_KEYS = {"warnings", "notho", "epithetQualifier"}

# Global cap on how many differing-row examples are collected for the failure message —
# matches the Phase 3 Java ParityTest's MAX_EXAMPLES.
MAX_EXAMPLES = 20


def _find_java_jar() -> Path | None:
    """The fixed shaded-jar path this repo's Phase 2/3 tooling already standardizes on,
    with a version-glob fallback (mirrors `scripts/cross-validate.sh`'s own
    `name-parser-cli-*-shaded.jar` glob) in case the pinned SNAPSHOT version moves on."""
    if JAVA_SHADED_JAR.is_file():
        return JAVA_SHADED_JAR
    target_dir = JAVA_SHADED_JAR.parent
    if not target_dir.is_dir():
        return None
    candidates = sorted(target_dir.glob("name-parser-cli-*-shaded.jar"))
    return candidates[0] if candidates else None


def _java_oracle_rows(corpus: str, jar: Path) -> list[dict]:
    proc = subprocess.run(
        [
            "java",
            "-jar",
            str(jar),
            "parse",
            f"--input=testdata/{corpus}.txt",
            "--output=-",
            "--format=jsonl",
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        encoding="utf-8",
        timeout=120,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"java -jar {jar.name} parse --input=testdata/{corpus}.txt failed "
            f"(exit {proc.returncode}): {proc.stderr[:2000]}"
        )
    return [json.loads(line) for line in proc.stdout.splitlines() if line.strip()]


def _rust_cli_oracle_rows(corpus: str) -> list[dict]:
    proc = subprocess.run(
        [str(RUST_CLI), "parse", f"--input=testdata/{corpus}.txt", "--output=-", "--quiet"],
        cwd=REPO_ROOT,
        capture_output=True,
        encoding="utf-8",
        timeout=120,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"nameparser-cli parse --input=testdata/{corpus}.txt failed "
            f"(exit {proc.returncode}): {proc.stderr[:2000]}"
        )
    return [json.loads(line) for line in proc.stdout.splitlines() if line.strip()]


def _select_oracle() -> tuple[str, Callable[[str], list[dict]]]:
    """Prefers the Java shaded jar — an independent, cross-language oracle, so this test
    also re-confirms Python<->Java parity, not just Python<->Rust-serialization plumbing —
    falling back to this repo's own release `nameparser-cli` (still a valid oracle: it
    reaches the core through clap + `BufRead` + `ParsedName`'s own `Serialize` impl
    straight to `serde_json`, a completely different path from PyO3/`pythonize`) if the jar
    or a `java` binary isn't available. Skips the whole test only if neither is available.
    """
    jar = _find_java_jar()
    if jar is not None and shutil.which("java") is not None:
        return f"java -jar {jar.name}", (lambda corpus: _java_oracle_rows(corpus, jar))
    if RUST_CLI.is_file():
        return "nameparser-cli (release, Rust-CLI fallback oracle)", _rust_cli_oracle_rows
    pytest.skip(
        f"Neither the Java shaded jar ({JAVA_SHADED_JAR}) nor a release nameparser-cli "
        f"binary ({RUST_CLI}) is available — cannot build an oracle for the corpus parity "
        f"test. Build one of: `cd .../name-parser/name-parser-cli && mvn -q package`, or "
        f"`cargo build --release -p nameparser-cli`."
    )


def _canonicalize(key: str, value):
    """Port of `canonicalize_for_key`/`sort_if_array`: if `key` is one of
    UNORDERED_FIELD_KEYS and `value` is array-shaped, sort it by each element's rendered
    JSON text (matching Rust's `x.to_string()` sort key on a `serde_json::Value`) before
    comparing. A JSON *object* under `epithetQualifier` passes through unchanged — the
    dict-vs-dict branch below is already order-insensitive by construction."""
    if key in UNORDERED_FIELD_KEYS and isinstance(value, list):
        return sorted(value, key=lambda item: json.dumps(item, sort_keys=True))
    return value


def _diff(path: str, a, b, out: list[tuple[str, object, object]]) -> None:
    """Port of `diff_element`: recursively appends `(path, a, b)` for every leaf where `a`
    and `b` disagree. A missing dict key and an explicit JSON `null` are indistinguishable
    here (both read back as Python `None`, via `dict.get`/`json.loads` respectively) —
    exactly the `Value::Null`-defaulting semantics `diff_element` itself relies on."""
    if isinstance(a, dict) and isinstance(b, dict):
        for key in sorted(set(a) | set(b)):
            va = _canonicalize(key, a.get(key))
            vb = _canonicalize(key, b.get(key))
            _diff(f"{path}.{key}" if path else key, va, vb, out)
        return
    if isinstance(a, list) and isinstance(b, list):
        for i in range(max(len(a), len(b))):
            va = a[i] if i < len(a) else None
            vb = b[i] if i < len(b) else None
            _diff(f"{path}[{i}]", va, vb, out)
        return
    if a != b:
        out.append((path, a, b))


def _render(v: object) -> str:
    return json.dumps(v, ensure_ascii=False)


# --- 5.0.0 three-way mapping of the oracle ------------------------------------------------------
# The oracle CLI still emits the raw core `parse()` output (a ParsedName, or an error), while the
# Python binding now returns the 5.0.0 three-way `parse()` — an `Informal` for a
# supraspecific-provisional name, and a type-clamped `Unparsable` for an informal-but-unrepresentable
# error. To compare like-for-like, map each oracle row through the SAME split/clamp the binding
# applies (deliberate mirrors of Rust `is_informal`/`to_informal`/`ParseError::clamped_to_unparsable`
# — see crates/nameparser/src/lib.rs + model/mod.rs). The engine output underneath is identical, so
# post-mapping this stays a 0-diff gate.

def _oracle_is_informal(p: dict) -> bool:
    """Mirror of Rust `is_informal`: an oracle ParsedName the 5.0.0 binding surfaces as `Informal`."""
    return bool(
        p.get("type") == "INFORMAL"
        and not p.get("specificEpithet")
        and (p.get("genus") or p.get("uninomial") or p.get("infragenericEpithet"))
    )


def _oracle_to_informal(p: dict) -> dict:
    """Mirror of Rust `to_informal`: the flat Informal in the same wire shape `Informal.to_dict()`
    emits (taxon, taxonRank, rank, and — omitted when absent — phrase, code)."""
    genus = p.get("genus")
    if genus:
        taxon, taxon_rank = genus, "GENUS"
    elif p.get("uninomial"):
        taxon, taxon_rank = p["uninomial"], p.get("rank")
    else:
        taxon, taxon_rank = p.get("infragenericEpithet"), p.get("rank")
    out = {"taxon": taxon, "taxonRank": taxon_rank, "rank": p.get("rank")}
    if p.get("phrase") is not None:
        out["phrase"] = p["phrase"]
    if p.get("code") is not None:
        out["code"] = p["code"]
    return out


def _clamp_oracle_error(err: dict) -> tuple[str | None, str | None, str | None]:
    """Mirror of `ParseError::clamped_to_unparsable`: a parsable oracle error type
    (INFORMAL/SCIENTIFIC) becomes OTHER, with the message's type token rebuilt to match; code is
    preserved. Returns (message, type, code)."""
    msg = err.get("message")
    t = err.get("type")
    code = err.get("code")
    if t in ("INFORMAL", "SCIENTIFIC"):
        if isinstance(msg, str):
            msg = msg.replace(f"Unparsable {t} name:", "Unparsable OTHER name:", 1)
        t = "OTHER"
    return msg, t, code


def test_python_binding_matches_oracle_across_all_corpora():
    """The parity gate. For every corpus name: both sides must agree on parsability; when
    both parse, `nameparser.parse(name).to_dict()` must match the oracle's `parsed` object
    field-for-field (warnings/notho/epithetQualifier order-insensitively, everything else
    exactly); when both fail, the raised `UnparsableNameError` must match the oracle's
    `error` object on `message` (exactly — see the module docstring for why this is message
    equality, not a reconstructed-message assertion), `name_type` vs. `error["type"]`, and
    `code` vs. `error.get("code")` (the oracle omits rather than nulls a missing code — see
    the module docstring). Prints a per-corpus + total tally unconditionally (visible with
    `pytest -s`); asserts 0 diffs over all corpora."""
    oracle_label, oracle_rows_fn = _select_oracle()

    tallies: list[tuple[str, int, int]] = []
    examples: list[str] = []
    total_compared = 0
    total_diffs = 0

    for corpus in CORPORA:
        names = read_corpus(corpus)
        oracle_rows = oracle_rows_fn(corpus)

        assert len(names) == len(oracle_rows), (
            f"{corpus}: extracted {len(names)} names from testdata/{corpus}.txt but the "
            f"{oracle_label} oracle produced {len(oracle_rows)} rows for the same file — "
            f"corpus-reading is misaligned between this test and the oracle CLI (e.g. an "
            f"off-by-one or a skip-rule mismatch) before any per-name comparison can start"
        )

        compared = 0
        diffs = 0
        for i, name in enumerate(names):
            row = oracle_rows[i]
            assert row.get("input") == name, (
                f"{corpus} line {i + 1}: oracle input {row.get('input')!r} != extracted "
                f"name {name!r} — row alignment was lost partway through the corpus"
            )
            compared += 1

            try:
                parsed = nameparser.parse(name)
            except nameparser.UnparsableNameError as exc:
                # Pull every needed value out of `exc` now: Python implicitly `del`s the
                # `except ... as exc` target at the end of this block, so a bare reference to
                # `exc` itself would not survive down to the comparison code below.
                python_outcome = ("error", str(exc), exc.name_type, exc.code)
            else:
                # 5.0.0 three-way: parse() returns a ParsedName OR an Informal (unparsable raised
                # above). Map the oracle's raw ParsedName through the same split so both sides are
                # compared on the same surface.
                kind = "informal" if isinstance(parsed, nameparser.Informal) else "ok"
                python_outcome = (kind, parsed.to_dict())

            if "error" in row:
                oracle_outcome = ("error", row["error"])
            elif _oracle_is_informal(row.get("parsed") or {}):
                oracle_outcome = ("informal", _oracle_to_informal(row["parsed"]))
            else:
                oracle_outcome = ("ok", row.get("parsed"))

            mismatch = None
            if python_outcome[0] != oracle_outcome[0]:
                mismatch = (
                    f"parsability disagreement: python={python_outcome[0]!r} "
                    f"oracle={oracle_outcome[0]!r}"
                )
            elif python_outcome[0] in ("ok", "informal"):
                leaf_diffs: list[tuple[str, object, object]] = []
                _diff("", python_outcome[1], oracle_outcome[1], leaf_diffs)
                if leaf_diffs:
                    shown = "; ".join(
                        f"{p or '<root>'}: {_render(a)} != {_render(b)}"
                        for p, a, b in leaf_diffs[:5]
                    )
                    mismatch = f"{len(leaf_diffs)} field diff(s): {shown}"
            else:
                _, python_message, python_name_type, python_code = python_outcome
                # `.get(..., {})`, not `oracle_outcome[1][...]` — a bare `dict` is untyped
                # from Pyright's view here, and while `error` is never structurally `null` on
                # either producer (confirmed in the module docstring), this is the cheap,
                # crash-proof form regardless. The oracle error is mapped through the 5.0.0
                # Unparsable clamp (INFORMAL/SCIENTIFIC → OTHER, message rebuilt) that the binding's
                # parse() applies but the raw parse() oracle does not.
                oracle_error = oracle_outcome[1] or {}
                oracle_message, oracle_type, oracle_code = _clamp_oracle_error(oracle_error)
                error_mismatches: list[str] = []
                if python_message != oracle_message:
                    error_mismatches.append(
                        f"message: python={python_message!r} oracle={oracle_message!r}"
                    )
                if python_name_type != oracle_type:
                    error_mismatches.append(
                        f"name_type: python={python_name_type!r} oracle={oracle_type!r}"
                    )
                if python_code != oracle_code:
                    error_mismatches.append(
                        f"code: python={python_code!r} oracle={oracle_code!r}"
                    )
                if error_mismatches:
                    mismatch = "; ".join(error_mismatches)

            if mismatch:
                diffs += 1
                if len(examples) < MAX_EXAMPLES:
                    examples.append(f"[{corpus} line {i + 1}] {name!r}: {mismatch}")

        tallies.append((corpus, compared, diffs))
        total_compared += compared
        total_diffs += diffs

    report_lines = [f"Python binding parity vs {oracle_label}, per-corpus tally:"]
    for corpus, compared, diffs in tallies:
        report_lines.append(f"  {corpus + '.txt':<24} {compared:>5} compared, {diffs:>5} diffs")
    report_lines.append(
        f"  {'TOTAL':<24} {total_compared:>5} compared, {total_diffs:>5} diffs"
    )
    report = "\n".join(report_lines)
    print("\n" + report)

    assert total_compared >= 11000, (
        f"only {total_compared} names were compared across {len(CORPORA)} corpora — "
        f"expected ~11,302; a corpus file may be missing, truncated, or empty"
    )

    if total_diffs:
        detail = "\n".join(examples)
        pytest.fail(
            f"{report}\n\nFirst {len(examples)} of {total_diffs} example(s):\n{detail}"
        )
