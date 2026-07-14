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
