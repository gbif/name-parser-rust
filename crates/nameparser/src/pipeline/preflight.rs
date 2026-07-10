// SPDX-License-Identifier: Apache-2.0
//! Java `org.gbif.nameparser.pipeline.Preflight` — the 33-pattern early gate that rejects
//! viruses, hybrid formulas, placeholders, OTU codes and other non-scientific-name input
//! before the rest of the pipeline tokenises it.

use crate::model::ParseError;
use crate::pipeline::ParseContext;

// STUB: real Preflight ported in Task 5. Until then this is a no-op that lets every
// input reach the (also still-skeletal) downstream stages unchanged, mirroring Java's
// `Preflight.run` signature (`static void run(String original, ParseContext ctx) throws
// UnparsableNameException`) so `pipeline::run` can wire the real call site once, ahead of
// the port landing.
pub fn run(_original: &str, _ctx: &mut ParseContext) -> Result<(), ParseError> {
    Ok(())
}
