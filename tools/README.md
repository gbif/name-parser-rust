# `tools/` — oracle generators for the cross-validation goldens

Small Java helpers that produce the Java-side "oracle" files the Rust golden tests diff
against. They are **not** part of the build; they are run by hand (or CI) to regenerate a
golden when the corpus or the Java reference changes.

## `FormatOracle.java`

Generates `testdata/expected-format.tsv`, the Java `org.gbif.nameparser.util.NameFormatter`
oracle for `crates/nameparser/tests/format_golden.rs`. For each input name it parses with the
real Java `NameParserImpl` and emits one TSV row of the five public renderings
(`canonical`, `canonicalWithoutAuthorship`, `canonicalMinimal`, `canonicalComplete`,
`authorshipComplete`).

Regenerate (Java 25 + the name-parser-cli shaded jar on the classpath):

```sh
[ -s "$HOME/.sdkman/bin/sdkman-init.sh" ] && source "$HOME/.sdkman/bin/sdkman-init.sh"
JAR=$(ls /path/to/name-parser/name-parser-cli/target/name-parser-cli-*-shaded.jar | head -1)
javac -cp "$JAR" -d /tmp/oracle tools/FormatOracle.java
java -cp "$JAR:/tmp/oracle" FormatOracle < testdata/benchmark-data.txt > testdata/expected-format.tsv
```

The generated `.tsv` is git-ignored (`testdata/*.tsv`); `format_golden.rs` SKIPs cleanly when
it is absent (the always-on structural coverage lives in `src/format.rs`'s own unit tests,
whose expected values were produced by this same oracle).

## `path_divergence.py`

Measures how often ChecklistBank's own rows parse differently with the authorship passed
separately (`parse(name, authorship, rank, code)`, how CLB calls the parser) and with it joined
into the name string — the corpus counterpart of the test suite's
`crates/nameparser/tests/common/path_divergences.tsv`. Needs the local
`testdata/clb-verbatim-names.tsv` and the Python binding built from the working tree:

```sh
(cd crates/nameparser-py && .venv/bin/maturin develop --release)
crates/nameparser-py/.venv/bin/python -I tools/path_divergence.py --sample 100000 --dump before.jsonl
# … change the engine, rebuild the binding …
crates/nameparser-py/.venv/bin/python -I tools/path_divergence.py --sample 100000 --dump after.jsonl
crates/nameparser-py/.venv/bin/python -I tools/path_divergence.py --diff before.jsonl after.jsonl
```

The sample is seeded (`--seed`, default 42), so two runs over the same file compare the same rows.

## `code_inference_eval.py`

Measures code inference against the nomenclatural code ChecklistBank datasets declare: samples
`testdata/clb-verbatim-names.tsv` rows that carry an authorship and a code, parses them without the
code hint and reports correct / wrong / no inference, overall, per declared code and per authorship
shape (year with or without a comma, ex-author, hybrid, in-citation, …), with `--examples N` wrong
inferences per shape. Run it before and after changing `pipeline/code_inference.rs`:

```sh
crates/nameparser-py/.venv/bin/python -I tools/code_inference_eval.py --sample 200000 --examples 10
```
