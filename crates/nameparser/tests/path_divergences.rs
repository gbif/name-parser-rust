// SPDX-License-Identifier: Apache-2.0
//! Gate on `common/path_divergences.tsv`, the known divergences between the authorship paths (see
//! `common/paths.rs`): every row must still diverge exactly as listed, and no row may repeat.
//! Independent of the tests whose calls the rows came from, so a fixed divergence is caught even
//! if its test is gone.

mod common;
use common::*;

#[test]
fn every_listed_divergence_still_diverges_as_listed() {
    let mut problems = Vec::new();
    for k in KNOWN.iter() {
        let call = k.call();
        let now = path_divergences(&call, k.shape());
        match now.iter().find(|d| d.variant == k.variant) {
            Some(d) if d.fields == k.fields => {}
            Some(d) => problems.push(format!(
                "{KNOWN_FILE}:{}: `{}` {} lists `{}`, now `{}`",
                k.line, k.input, k.variant, k.fields, d.fields
            )),
            None => problems.push(format!(
                "{KNOWN_FILE}:{}: `{}` {} now parses alike — remove the line",
                k.line, k.input, k.variant
            )),
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn no_divergence_is_listed_twice() {
    let mut seen = std::collections::HashMap::new();
    for k in KNOWN.iter() {
        let key = (
            k.variant.as_str(),
            k.input.as_str(),
            k.authorship.as_deref(),
            k.rank,
            k.code,
        );
        if let Some(first) = seen.insert(key, k.line) {
            panic!("{KNOWN_FILE}:{} repeats line {first}", k.line);
        }
    }
}
