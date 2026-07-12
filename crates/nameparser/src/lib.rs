// SPDX-License-Identifier: Apache-2.0

//! A faithful Rust port of the GBIF scientific name parser. [`parse`] turns a scientific name into
//! its structured atoms — genus, epithets, authorship, rank, nomenclatural code, notes, warnings —
//! as a [`model::ParsedName`]. Byte-for-byte cross-validated against the Java `name-parser`
//! (`NameParserImpl`) over 11,302 + 6.4M names (0 diffs). The same engine also ships as a native
//! CLI and as Java/Python/R bindings — see the repository README.

pub mod model;
pub mod pipeline;
pub mod regexes;
pub mod token;
pub mod unicode;
pub mod viral;

use model::{NomCode, ParseError, ParsedName, Rank};

/// Java `NameParserGBIF`/CLI-facing entry point. Parses a scientific name — optionally
/// alongside a separately supplied authorship string, a requested [`Rank`] and a
/// requested [`NomCode`] — into a [`ParsedName`], or `Err(`[`ParseError`]`)` when the
/// input cannot be parsed into a meaningful name. Delegates to [`pipeline::run`].
pub fn parse(
    name: &str,
    authorship: Option<&str>,
    rank: Option<Rank>,
    code: Option<NomCode>,
) -> Result<ParsedName, ParseError> {
    pipeline::run(name, authorship, rank, code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::NameType;

    #[test]
    fn parse_delegates_to_pipeline_run() {
        let pn = parse("Abies alba", None, None, None).expect("should parse");
        assert_eq!(pn.type_, NameType::Scientific);
    }

    #[test]
    fn parse_rejects_empty_input() {
        let err = parse("", None, None, None).unwrap_err();
        assert_eq!(err.type_, NameType::Other);
    }
}
