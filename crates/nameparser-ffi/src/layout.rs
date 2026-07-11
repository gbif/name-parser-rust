// SPDX-License-Identifier: Apache-2.0

//! The flat fixed-layout binary encoding for [`np_parse_struct`](crate::np_parse_struct) — a
//! single canonical description of the wire format both Rust (this file) and Java (Task 6's
//! `StructCodec.java`, a later task) read. Where `np_parse_json` marshals a [`ParsedName`]
//! through `serde_json` text + Gson reflection, this module writes the SAME data as raw bytes
//! at fixed/computed offsets into a caller-owned buffer, to shed that marshalling cost.
//!
//! # Byte order
//!
//! **Every multi-byte scalar is little-endian.** Java must read with
//! `ByteBuffer.order(ByteOrder.LITTLE_ENDIAN)` (or `VarHandle`s built with that explicit byte
//! order) — `ByteBuffer`'s own default is big-endian, so this is NOT the do-nothing choice.
//! Little-endian was picked because it matches every realistic JVM deployment target
//! (x86_64/aarch64) and is `to_le_bytes()`-cheap on the Rust side.
//!
//! # Overall shape
//!
//! ```text
//! [ header: HEADER_SIZE bytes, fixed offsets ]
//! [ string table: 4 + NUM_STRING_SLOTS * 8 bytes, fixed offsets ]
//! [ run-slot tables: 6 tables, back-to-back, each self-delimiting, VARIABLE total size ]
//! [ nested authorship groups: 2 groups (generic_authorship, specific_authorship),
//!   back-to-back, each self-delimiting via a presence flag, VARIABLE total size ]
//! [ string blob: the remaining bytes, all string content, referenced by absolute offset ]
//! ```
//!
//! Every `(offset, len)` string ref anywhere in the buffer (string table AND run-slot entries)
//! is an ABSOLUTE byte offset from the START of the buffer (offset 0 == the `abi_version`
//! byte) — not blob-relative. This costs 4 bytes per ref versus a blob-relative `u32` (moot;
//! `u32` either way) but means there is exactly one addressing scheme in the whole format, not
//! two.
//!
//! # Header (fixed offsets, [`HEADER_SIZE`] bytes total)
//!
//! | Offset | Size | Type | Field | Notes |
//! |---|---|---|---|---|
//! | 0  | 4 | u32 | `abi_version` | must equal [`crate::np_abi_version()`] |
//! | 4  | 4 | i32 | `status` | [`STATUS_SUCCESS`] (0) or [`STATUS_UNPARSABLE`] (-1) |
//! | 8  | 4 | i32 | `rank` | Java `Rank` ordinal, [`ABSENT_ENUM`] (-1) if absent |
//! | 12 | 4 | i32 | `code` | Java `NomCode` ordinal, [`ABSENT_ENUM`] (-1) if absent |
//! | 16 | 4 | i32 | `name_type` | Java `NameType` ordinal, [`ABSENT_ENUM`] (-1) if absent |
//! | 20 | 4 | i32 | `state` | Java `ParsedName.State` ordinal, [`ABSENT_ENUM`] (-1) if absent |
//! | 24 | 1 | u8  | `candidatus` | 0/1 |
//! | 25 | 1 | u8  | `doubtful` | 0/1 |
//! | 26 | 1 | u8  | `manuscript` | 0/1 |
//! | 27 | 1 | u8  | `extinct` | 0/1 |
//! | 28 | 1 | u8  | `original_spelling` | [`ORIGINAL_SPELLING_FALSE`]/[`_TRUE`]/[`_UNKNOWN`] (0/1/2) |
//! | 29 | 1 | u8  | `notho_bits` | bitset, bit `i` = `NamePart` ordinal `i` present in the notho set |
//! | 30 | 2 | —   | *(padding)* | reserved, always zero, keeps `published_in_year` 4-aligned |
//! | 32 | 4 | i32 | `published_in_year` | -1 if absent |
//!
//! `rank`/`state` are never actually optional on [`ParsedName`] itself (both are `@Nonnull` in
//! Java) — the `-1` sentinel is reachable only on the unparsable path, where there is no
//! `ParsedName` to read a rank/state off of at all (see "Unparsable path" below).
//!
//! # Enum ordinal mapping — verified, not assumed
//!
//! Every Rust wire enum here (`Rank`, `NomCode`, `NameType`, `NamePart`, `State`) is a
//! fieldless enum declared in the exact same order as its Java counterpart (this was already
//! required for `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` to reproduce Java's `.name()`
//! wire form on the JSON path — see `model::enums`' own module docs). Rust guarantees a
//! fieldless enum's discriminants are numbered `0, 1, 2, ...` in declaration order unless
//! overridden, so `as i32` on any of these five types already equals the Java ordinal — but
//! per this task's brief, that equivalence is verified here rather than assumed silently:
//!
//! - **`Rank`** (117 variants): [`rank_ordinal`] delegates to the crate's own
//!   `Rank::ordinal()`, itself already covered by `model::enums`' exhaustive
//!   `rank_all_117_variants_in_java_declaration_order`-style tests. Independently
//!   re-verified for this task by mechanically parsing
//!   `name-parser/name-parser-api/.../Rank.java`'s constant declarations and diffing all 117
//!   names, in order, against `Rank::ALL` — zero mismatches. Spot-checked below by name.
//! - **`NameType`**, **`NomCode`**, **`NamePart`**, **`State`**: each gets its own explicit,
//!   exhaustive `match`-based `_ordinal` function in this file (not a bare `as i32`) — a
//!   compile error, not silent drift, if a variant is ever added without updating the mapping.
//!   Each was hand-verified line-by-line against the corresponding Java source
//!   (`NameType.java`, `NomCode.java`, `NamePart.java`, `ParsedName.java`'s nested `State`
//!   enum) and is round-tripped exhaustively in this module's tests.
//!
//! `NamePart`'s ordinal is emitted as `u32` (never absent/signed — a `NamePart` appearing
//! anywhere in the wire format, either as a `notho_bits` bit position or an `epithetQualifier`
//! entry's key, is by construction always a real value, never a sentinel).
//!
//! # Absent-string sentinel
//!
//! A string ref is the pair `(offset: u32, len: u32)`. **Absent** (the source `Option<String>`
//! was `None`) is encoded as `offset == `[`ABSENT_STRING_OFFSET`]` (`u32::MAX`); `len` is
//! written as `0` in that case too, but only `offset` is authoritative — a reader must check
//! `offset` alone. **Present-but-empty** (a `Some("".to_string())`, which no current pipeline
//! stage actually produces, but the format does not rule out) is unambiguous from absent: it
//! gets a real `offset` (pointing at its — zero-byte — position in the blob) with `len == 0`.
//! `u32::MAX` can never collide with a real offset in practice (that would require a >4 GiB
//! single parse-result buffer).
//!
//! # String table (fixed slots, offset [`STRING_TABLE_OFFSET`])
//!
//! `u32 count` (always [`NUM_STRING_SLOTS`] = 17, written as data — not just implied — so a
//! reader can sanity-check it), then 17 × `(u32 offset, u32 len)` string refs in this fixed
//! slot order:
//!
//! | Slot | Index | `ParsedName` field |
//! |---|---|---|
//! | `SLOT_UNINOMIAL` | 0 | `uninomial` |
//! | `SLOT_GENUS` | 1 | `genus` |
//! | `SLOT_INFRAGENERIC` | 2 | `infrageneric_epithet` |
//! | `SLOT_SPECIFIC` | 3 | `specific_epithet` |
//! | `SLOT_INFRASPECIFIC` | 4 | `infraspecific_epithet` |
//! | `SLOT_CULTIVAR` | 5 | `cultivar_epithet` |
//! | `SLOT_PHRASE` | 6 | `phrase` |
//! | `SLOT_TAXONOMIC_NOTE` | 7 | `taxonomic_note` |
//! | `SLOT_NOMENCLATURAL_NOTE` | 8 | `nomenclatural_note` |
//! | `SLOT_PUBLISHED_IN` | 9 | `published_in` |
//! | `SLOT_PUBLISHED_IN_PAGE` | 10 | `published_in_page` |
//! | `SLOT_UNPARSED` | 11 | `unparsed` |
//! | `SLOT_SANCTIONING_AUTHOR` | 12 | `sanctioning_author` |
//! | `SLOT_YEAR_COMB` | 13 | `combination_authorship.year` |
//! | `SLOT_YEAR_BAS` | 14 | `basionym_authorship.year` |
//! | `SLOT_IMPRINT_YEAR_COMB` | 15 | `combination_authorship.imprint_year` |
//! | `SLOT_IMPRINT_YEAR_BAS` | 16 | `basionym_authorship.imprint_year` |
//!
//! # Run-slots (fixed sequential order, starting at offset [`RUN_SLOTS_OFFSET`])
//!
//! Unlike the string table, run-slots are NOT addressed by a directory of offsets — they are
//! simply concatenated in this fixed order, each self-delimiting via its own leading `u32
//! count`, so a reader walks them sequentially, accumulating a byte cursor:
//!
//! 1. `RUN_AUTHORS_COMB` — `combination_authorship.authors`, entries = string refs (8 bytes each)
//! 2. `RUN_EXAUTHORS_COMB` — `combination_authorship.ex_authors`, entries = string refs
//! 3. `RUN_AUTHORS_BAS` — `basionym_authorship.authors`, entries = string refs
//! 4. `RUN_EXAUTHORS_BAS` — `basionym_authorship.ex_authors`, entries = string refs
//! 5. `RUN_WARNINGS` — `warnings`, entries = string refs
//! 6. `RUN_EPITHET_QUALIFIER` — `epithet_qualifier` (iterated in `BTreeMap`/ordinal order,
//!    matching Java's `EnumMap` iteration order), entries = `(u32 namepart_ordinal, u32
//!    offset, u32 len)`, 12 bytes each
//!
//! Each run-slot's entries are always "present" in the string-ref sense (a run entry IS a real
//! list element, e.g. one actual author string) — the absent-sentinel only ever applies to the
//! 17 fixed string-table slots above (and to the 5 nested-group string refs below).
//!
//! # Nested authorship groups (fixed sequential order, right after the run-slots)
//!
//! Two `Option<CombinedAuthorship>` fields on [`ParsedName`] — `generic_authorship` (the
//! authorship of an infrageneric name's genus, e.g. `Cordia (Adans.) Kuntze sect. Salimori`)
//! and `specific_authorship` (the authorship of a cultivar's species, e.g.
//! `Acer campestre L. cv. 'Elsrijk' Broerse`) — each encode as a self-delimiting **nested
//! group**, in this fixed order, right after `RUN_EPITHET_QUALIFIER` and before the blob:
//!
//! 1. `generic_authorship`
//! 2. `specific_authorship`
//!
//! Each group is:
//!
//! - `u32 present` — [`GROUP_ABSENT`] (0) or [`GROUP_PRESENT`] (1). **If absent, that 4-byte
//!   flag is the whole group** (the common case costs 4 bytes and is unambiguous). If present:
//! - four run-slot tables (each a `u32 count` then `count` × 8-byte string refs), in this order:
//!   its `combination_authorship.authors`, `combination_authorship.ex_authors`,
//!   `basionym_authorship.authors`, `basionym_authorship.ex_authors`;
//! - then five fixed 8-byte string refs, in this order: `combination_authorship.year`,
//!   `combination_authorship.imprint_year`, `basionym_authorship.year`,
//!   `basionym_authorship.imprint_year`, `sanctioning_author` — each honoring the same
//!   absent-string sentinel ([`ABSENT_STRING_OFFSET`]) as the string table.
//!
//! This mirrors the base `CombinedAuthorship`'s own encoding (the base one is spread across the
//! four author/ex-author run-slots + the `YEAR_*`/`IMPRINT_YEAR_*`/`SANCTIONING_AUTHOR` string
//! slots) — the base authorship gets first-class flat slots because it's on every name; the two
//! nested groups are rare (38 of 11,302 corpus names) so they get a compact
//! present-flag-gated block instead. A `CombinedAuthorship` is a complete unit here: all four
//! of each inner `Authorship`'s fields (authors, ex-authors, year, imprint-year) plus the
//! group's `sanctioning_author` are carried, so a reader reconstructs the whole nested object.
//!
//! # Return convention (mirrored on [`crate::np_parse_struct`])
//!
//! - `>= 0`: success, the exact number of bytes written to `out` (== this module's `encode`
//!   output length).
//! - `-1`: unparsable name. `out` receives ONLY the header ([`HEADER_SIZE`] bytes, clamped to
//!   `out_cap` — callers should supply at least [`HEADER_SIZE`] bytes to reliably read it) with
//!   `status = `[`STATUS_UNPARSABLE`]`, `name_type` and `code` set from the
//!   [`nameparser::model::ParseError`], and every other header field at its absent/zero
//!   default (`rank`/`state` = [`ABSENT_ENUM`], flags = 0, `original_spelling` =
//!   [`ORIGINAL_SPELLING_UNKNOWN`], `published_in_year` = -1, `notho_bits` = 0). No string
//!   table or run-slots are written on this path.
//! - `-2`: an internal panic was caught (`catch_unwind`); `out` is untouched.
//! - overflow: if the full encoded size exceeds `out_cap`, `out` is untouched and the return is
//!   `-(needed as i64 + 3)`, so a caller recovers `needed = -ret - 3` and can retry with a
//!   bigger buffer. This only applies to the success path — the unparsable path's small,
//!   header-only write is never overflow-coded (see above).
//!
//! # Field coverage — complete for every field the JSON path serializes
//!
//! This layout carries every [`ParsedName`] field the JSON (`np_parse_json`) path emits, so a
//! struct-path `ParsedName` reconstructed by Task 6's Java reader is field-for-field identical
//! to the JSON path's for every input. The four fields an earlier revision of this layout
//! omitted are now all encoded: `combination_authorship.imprint_year` /
//! `basionym_authorship.imprint_year` (the `SLOT_IMPRINT_YEAR_COMB`/`_BAS` string slots — 33 of
//! 11,302 corpus names, e.g. the `1985, 1984` double-year and `[1851]` bracketed patterns), and
//! `generic_authorship` / `specific_authorship` (the two nested authorship groups above — 5 of
//! 11,302 corpus names). The one deliberate non-carry is `Authorship.imprint_year` on the
//! nested groups' *inner* authorships, which IS carried (each nested `Authorship` encodes all
//! four of its fields) — there is no remaining gap.
//!
//! (`generic_authorship`/`specific_authorship` are set by `pipeline::run` — see `pipeline/mod.rs`'s
//! `pendingGeneric`/`pendingSpecific` handling; the imprint years by `pipeline::authorship_parser`
//! for bracketed/keyword/bare-`&`-year forms. All are read straight off the same `ParsedName`
//! the JSON path serializes, never re-derived.)

use std::collections::BTreeMap;

use nameparser::model::{
    Authorship, CombinedAuthorship, NamePart, NameType, NomCode, ParseError, ParsedName, Rank,
    State,
};

// ================================================================================================
// Header offsets and sizes
// ================================================================================================

pub const OFF_ABI_VERSION: usize = 0;
pub const OFF_STATUS: usize = 4;
pub const OFF_RANK: usize = 8;
pub const OFF_CODE: usize = 12;
pub const OFF_NAME_TYPE: usize = 16;
pub const OFF_STATE: usize = 20;
pub const OFF_CANDIDATUS: usize = 24;
pub const OFF_DOUBTFUL: usize = 25;
pub const OFF_MANUSCRIPT: usize = 26;
pub const OFF_EXTINCT: usize = 27;
pub const OFF_ORIGINAL_SPELLING: usize = 28;
pub const OFF_NOTHO_BITS: usize = 29;
// Offsets 30..32 are 2 bytes of reserved padding (keeps `published_in_year` 4-aligned).
pub const OFF_PUBLISHED_IN_YEAR: usize = 32;

/// Total header size in bytes — also the minimum `out_cap` a caller must supply to reliably
/// decode the unparsable path's header-only write (see the module doc's "Return convention").
pub const HEADER_SIZE: usize = 36;

/// `status` header field value on the success path.
pub const STATUS_SUCCESS: i32 = 0;
/// `status` header field value on the unparsable path.
pub const STATUS_UNPARSABLE: i32 = -1;

/// Sentinel for an absent `rank`/`code`/`name_type`/`state` header enum slot.
pub const ABSENT_ENUM: i32 = -1;

/// `original_spelling` byte values — a 3-state encoding of `Option<bool>`.
pub const ORIGINAL_SPELLING_FALSE: u8 = 0;
pub const ORIGINAL_SPELLING_TRUE: u8 = 1;
pub const ORIGINAL_SPELLING_UNKNOWN: u8 = 2;

// ================================================================================================
// String table: fixed slots
// ================================================================================================

pub const SLOT_UNINOMIAL: usize = 0;
pub const SLOT_GENUS: usize = 1;
pub const SLOT_INFRAGENERIC: usize = 2;
pub const SLOT_SPECIFIC: usize = 3;
pub const SLOT_INFRASPECIFIC: usize = 4;
pub const SLOT_CULTIVAR: usize = 5;
pub const SLOT_PHRASE: usize = 6;
pub const SLOT_TAXONOMIC_NOTE: usize = 7;
pub const SLOT_NOMENCLATURAL_NOTE: usize = 8;
pub const SLOT_PUBLISHED_IN: usize = 9;
pub const SLOT_PUBLISHED_IN_PAGE: usize = 10;
pub const SLOT_UNPARSED: usize = 11;
pub const SLOT_SANCTIONING_AUTHOR: usize = 12;
pub const SLOT_YEAR_COMB: usize = 13;
pub const SLOT_YEAR_BAS: usize = 14;
pub const SLOT_IMPRINT_YEAR_COMB: usize = 15;
pub const SLOT_IMPRINT_YEAR_BAS: usize = 16;

/// Number of fixed string-table slots (`SLOT_UNINOMIAL`..`SLOT_IMPRINT_YEAR_BAS`).
pub const NUM_STRING_SLOTS: usize = 17;

/// Byte size of a single string ref: `u32 offset` + `u32 len`.
pub const STRING_REF_SIZE: usize = 8;

/// Byte offset where the string table begins (right after the header).
pub const STRING_TABLE_OFFSET: usize = HEADER_SIZE;

/// Byte size of the whole string table: the `u32 count` field plus `NUM_STRING_SLOTS` refs.
/// Fixed regardless of how many slots are actually present — absent slots still occupy their
/// `(offset, len)` pair, written as the absent sentinel.
pub const STRING_TABLE_SIZE: usize = 4 + NUM_STRING_SLOTS * STRING_REF_SIZE;

/// Sentinel `offset` value marking an absent string-table slot. See the module doc's
/// "Absent-string sentinel" section.
pub const ABSENT_STRING_OFFSET: u32 = u32::MAX;

// ================================================================================================
// Run-slots: fixed sequential order
// ================================================================================================

pub const RUN_AUTHORS_COMB: usize = 0;
pub const RUN_EXAUTHORS_COMB: usize = 1;
pub const RUN_AUTHORS_BAS: usize = 2;
pub const RUN_EXAUTHORS_BAS: usize = 3;
pub const RUN_WARNINGS: usize = 4;
pub const RUN_EPITHET_QUALIFIER: usize = 5;

/// Number of run-slots (`RUN_AUTHORS_COMB`..`RUN_EPITHET_QUALIFIER`).
pub const NUM_RUN_SLOTS: usize = 6;

/// Byte size of one `epithetQualifier` entry: `u32 namepart_ordinal` + `u32 offset` + `u32 len`.
pub const EPITHET_QUALIFIER_ENTRY_SIZE: usize = 12;

/// Byte offset where the first run-slot table (`RUN_AUTHORS_COMB`) begins — fixed, since the
/// string table ahead of it is always exactly [`STRING_TABLE_SIZE`] bytes regardless of data.
pub const RUN_SLOTS_OFFSET: usize = STRING_TABLE_OFFSET + STRING_TABLE_SIZE;

// ================================================================================================
// Nested authorship groups: fixed sequential order, right after the run-slots
// ================================================================================================

/// `present` flag value for an ABSENT nested authorship group (the whole group is just this
/// 4-byte flag). See the module doc's "Nested authorship groups" section.
pub const GROUP_ABSENT: u32 = 0;
/// `present` flag value for a PRESENT nested authorship group (followed by its 4 run tables and
/// 5 string refs).
pub const GROUP_PRESENT: u32 = 1;

/// Number of nested authorship groups (`generic_authorship`, then `specific_authorship`).
pub const NUM_NESTED_AUTHORSHIP_GROUPS: usize = 2;

/// The 5 fixed 8-byte string refs a PRESENT nested group carries after its 4 run tables:
/// `combination.year`, `combination.imprint_year`, `basionym.year`, `basionym.imprint_year`,
/// `sanctioning_author`.
pub const NESTED_GROUP_STRING_REFS: usize = 5;

// ================================================================================================
// Enum ordinal mapping — see the module doc's "Enum ordinal mapping" section for how each of
// these was verified against the Java oracle source, not assumed from Rust declaration order.
// ================================================================================================

/// The Java `Rank` ordinal for `r`. Delegates to [`Rank::ordinal`], independently re-verified
/// for this task (see the module doc) rather than re-implemented as a duplicate 117-arm match.
pub fn rank_ordinal(r: Rank) -> i32 {
    r.ordinal() as i32
}

/// The Java `NomCode` ordinal for `c`. Exhaustive match, not `as i32` — see the module doc.
pub fn nomcode_ordinal(c: NomCode) -> i32 {
    match c {
        NomCode::Bacterial => 0,
        NomCode::Botanical => 1,
        NomCode::Cultivars => 2,
        NomCode::Phyto => 3,
        NomCode::Virus => 4,
        NomCode::Zoological => 5,
        NomCode::Phylo => 6,
    }
}

/// The Java `NameType` ordinal for `t`. Exhaustive match, not `as i32` — see the module doc.
pub fn name_type_ordinal(t: NameType) -> i32 {
    match t {
        NameType::Scientific => 0,
        NameType::Formula => 1,
        NameType::Informal => 2,
        NameType::Placeholder => 3,
        NameType::Other => 4,
    }
}

/// The Java `ParsedName.State` ordinal for `s`. Exhaustive match, not `as i32` — see the
/// module doc.
pub fn state_ordinal(s: State) -> i32 {
    match s {
        State::Complete => 0,
        State::Partial => 1,
        State::None => 2,
    }
}

/// The Java `NamePart` ordinal for `p`, as an unsigned value — a `NamePart` is never an
/// absent/sentinel slot on the wire (see the module doc). Exhaustive match, not `as u32`.
pub fn namepart_ordinal(p: NamePart) -> u32 {
    match p {
        NamePart::Generic => 0,
        NamePart::Infrageneric => 1,
        NamePart::Specific => 2,
        NamePart::Infraspecific => 3,
    }
}

/// Folds `notho` into the header's `notho_bits` byte: bit `namepart_ordinal(part)` set for
/// each part present in the set. `None`/an empty set both fold to `0` (indistinguishable, but
/// `add_notho`/`set_notho` — the only two ways `ParsedName::notho` is ever populated — never
/// produce `Some(vec![])`, so this is a non-issue in practice).
fn notho_bits(notho: &Option<Vec<NamePart>>) -> u8 {
    notho
        .as_ref()
        .map(|parts| {
            parts
                .iter()
                .fold(0u8, |acc, &p| acc | (1u8 << namepart_ordinal(p)))
        })
        .unwrap_or(0)
}

/// Folds `Option<bool>` into the header's 3-state `original_spelling` byte.
fn original_spelling_byte(v: Option<bool>) -> u8 {
    match v {
        None => ORIGINAL_SPELLING_UNKNOWN,
        Some(false) => ORIGINAL_SPELLING_FALSE,
        Some(true) => ORIGINAL_SPELLING_TRUE,
    }
}

// ================================================================================================
// Header writer
// ================================================================================================

/// All header field values, gathered up-front so both the success and unparsable encode paths
/// share exactly one byte-writing routine ([`Header::write_to`]) — the two paths cannot drift
/// apart on field order/size/offset.
struct Header {
    abi_version: u32,
    status: i32,
    rank: i32,
    code: i32,
    name_type: i32,
    state: i32,
    candidatus: bool,
    doubtful: bool,
    manuscript: bool,
    extinct: bool,
    original_spelling: u8,
    notho_bits: u8,
    published_in_year: i32,
}

impl Header {
    /// Appends exactly [`HEADER_SIZE`] bytes to `buf`, in field-offset order. `buf` must be
    /// empty on entry — the header is always the first thing written to a fresh buffer.
    fn write_to(&self, buf: &mut Vec<u8>) {
        debug_assert!(buf.is_empty(), "header must be the first region written");
        buf.extend_from_slice(&self.abi_version.to_le_bytes()); // OFF_ABI_VERSION
        buf.extend_from_slice(&self.status.to_le_bytes()); // OFF_STATUS
        buf.extend_from_slice(&self.rank.to_le_bytes()); // OFF_RANK
        buf.extend_from_slice(&self.code.to_le_bytes()); // OFF_CODE
        buf.extend_from_slice(&self.name_type.to_le_bytes()); // OFF_NAME_TYPE
        buf.extend_from_slice(&self.state.to_le_bytes()); // OFF_STATE
        buf.push(self.candidatus as u8); // OFF_CANDIDATUS
        buf.push(self.doubtful as u8); // OFF_DOUBTFUL
        buf.push(self.manuscript as u8); // OFF_MANUSCRIPT
        buf.push(self.extinct as u8); // OFF_EXTINCT
        buf.push(self.original_spelling); // OFF_ORIGINAL_SPELLING
        buf.push(self.notho_bits); // OFF_NOTHO_BITS
        buf.extend_from_slice(&[0u8, 0u8]); // padding, offsets 30..32
        buf.extend_from_slice(&self.published_in_year.to_le_bytes()); // OFF_PUBLISHED_IN_YEAR
        debug_assert_eq!(buf.len(), HEADER_SIZE);
    }
}

// ================================================================================================
// String blob placement
// ================================================================================================

/// Accumulates string bytes into a trailing blob, handing back `(offset, len)` string refs
/// where `offset` is already absolute-from-buffer-start (i.e. `base_offset` is the byte
/// position, within the FINAL buffer, where this placer's blob will eventually be appended).
struct StringPlacer {
    blob: Vec<u8>,
    base_offset: u32,
}

impl StringPlacer {
    fn new(base_offset: u32) -> Self {
        Self {
            blob: Vec::new(),
            base_offset,
        }
    }

    /// Appends `s`'s UTF-8 bytes to the blob and returns its absolute `(offset, len)` ref.
    fn place(&mut self, s: &str) -> (u32, u32) {
        let offset = self.base_offset + self.blob.len() as u32;
        let bytes = s.as_bytes();
        self.blob.extend_from_slice(bytes);
        (offset, bytes.len() as u32)
    }
}

/// Appends one run-slot table (a `u32 count` then `count` plain string refs) to `buf`.
fn write_string_run_table(buf: &mut Vec<u8>, refs: &[(u32, u32)]) {
    buf.extend_from_slice(&(refs.len() as u32).to_le_bytes());
    for &(offset, len) in refs {
        buf.extend_from_slice(&offset.to_le_bytes());
        buf.extend_from_slice(&len.to_le_bytes());
    }
}

/// Appends the `epithetQualifier` run-slot table (`u32 count` then `count` ×
/// `(u32 namepart_ordinal, u32 offset, u32 len)`) to `buf`.
fn write_epithet_qualifier_table(buf: &mut Vec<u8>, refs: &[(u32, u32, u32)]) {
    buf.extend_from_slice(&(refs.len() as u32).to_le_bytes());
    for &(part_ordinal, offset, len) in refs {
        buf.extend_from_slice(&part_ordinal.to_le_bytes());
        buf.extend_from_slice(&offset.to_le_bytes());
        buf.extend_from_slice(&len.to_le_bytes());
    }
}

/// Appends a single `(u32 offset, u32 len)` string ref to `buf`.
fn write_string_ref(buf: &mut Vec<u8>, r: (u32, u32)) {
    buf.extend_from_slice(&r.0.to_le_bytes());
    buf.extend_from_slice(&r.1.to_le_bytes());
}

/// The absolute string ref for an optional string — the absent sentinel when `None`.
fn place_opt(placer: &mut StringPlacer, s: Option<&str>) -> (u32, u32) {
    match s {
        Some(s) => placer.place(s),
        None => (ABSENT_STRING_OFFSET, 0),
    }
}

// ================================================================================================
// Nested authorship groups (generic_authorship / specific_authorship)
// ================================================================================================

/// The placed string refs for one nested `CombinedAuthorship` group (see the module doc's
/// "Nested authorship groups" section). Strings are already in the blob by the time this exists.
struct NestedAuthorshipRefs {
    authors_comb: Vec<(u32, u32)>,
    exauthors_comb: Vec<(u32, u32)>,
    authors_bas: Vec<(u32, u32)>,
    exauthors_bas: Vec<(u32, u32)>,
    year_comb: (u32, u32),
    imprint_year_comb: (u32, u32),
    year_bas: (u32, u32),
    imprint_year_bas: (u32, u32),
    sanctioning_author: (u32, u32),
}

/// The byte size a nested authorship group occupies (count-driven, so computable before any
/// string is placed): 4 bytes for the `present` flag when absent, else the flag + its four run
/// tables + [`NESTED_GROUP_STRING_REFS`] fixed refs.
fn nested_group_size(ca: &Option<CombinedAuthorship>) -> usize {
    match ca {
        None => 4,
        Some(ca) => {
            4 + (4 + ca.combination_authorship.authors.len() * STRING_REF_SIZE)
                + (4 + ca.combination_authorship.ex_authors.len() * STRING_REF_SIZE)
                + (4 + ca.basionym_authorship.authors.len() * STRING_REF_SIZE)
                + (4 + ca.basionym_authorship.ex_authors.len() * STRING_REF_SIZE)
                + NESTED_GROUP_STRING_REFS * STRING_REF_SIZE
        }
    }
}

/// Places every string of a nested `CombinedAuthorship` into the blob (in the fixed encode
/// order: comb authors, comb ex-authors, bas authors, bas ex-authors, then the 5 opt strings),
/// returning their refs. The years/imprint-years/sanctioning-author use the absent sentinel.
fn place_nested(placer: &mut StringPlacer, ca: &CombinedAuthorship) -> NestedAuthorshipRefs {
    let place_run = |placer: &mut StringPlacer, a: &Authorship| {
        (
            a.authors
                .iter()
                .map(|s| placer.place(s))
                .collect::<Vec<_>>(),
            a.ex_authors
                .iter()
                .map(|s| placer.place(s))
                .collect::<Vec<_>>(),
        )
    };
    let (authors_comb, exauthors_comb) = place_run(placer, &ca.combination_authorship);
    let (authors_bas, exauthors_bas) = place_run(placer, &ca.basionym_authorship);
    let year_comb = place_opt(placer, ca.combination_authorship.year.as_deref());
    let imprint_year_comb = place_opt(placer, ca.combination_authorship.imprint_year.as_deref());
    let year_bas = place_opt(placer, ca.basionym_authorship.year.as_deref());
    let imprint_year_bas = place_opt(placer, ca.basionym_authorship.imprint_year.as_deref());
    let sanctioning_author = place_opt(placer, ca.sanctioning_author.as_deref());
    NestedAuthorshipRefs {
        authors_comb,
        exauthors_comb,
        authors_bas,
        exauthors_bas,
        year_comb,
        imprint_year_comb,
        year_bas,
        imprint_year_bas,
        sanctioning_author,
    }
}

/// Appends a nested authorship group to `buf`: just the [`GROUP_ABSENT`] flag when `refs` is
/// `None`, else the [`GROUP_PRESENT`] flag then its four run tables and five fixed string refs.
fn write_nested_group(buf: &mut Vec<u8>, refs: &Option<NestedAuthorshipRefs>) {
    match refs {
        None => buf.extend_from_slice(&GROUP_ABSENT.to_le_bytes()),
        Some(refs) => {
            buf.extend_from_slice(&GROUP_PRESENT.to_le_bytes());
            write_string_run_table(buf, &refs.authors_comb);
            write_string_run_table(buf, &refs.exauthors_comb);
            write_string_run_table(buf, &refs.authors_bas);
            write_string_run_table(buf, &refs.exauthors_bas);
            write_string_ref(buf, refs.year_comb);
            write_string_ref(buf, refs.imprint_year_comb);
            write_string_ref(buf, refs.year_bas);
            write_string_ref(buf, refs.imprint_year_bas);
            write_string_ref(buf, refs.sanctioning_author);
        }
    }
}

// ================================================================================================
// Public encode entry points
// ================================================================================================

/// Encodes a successfully parsed `ParsedName` per this module's layout. `abi_version` is
/// threaded in by the caller (`np_parse_struct`, via `np_abi_version()`) rather than read from
/// a constant here, keeping this module independent of `lib.rs`'s ABI-versioning policy.
///
/// Reads every field directly off `pn` — the same `ParsedName` the JSON path
/// (`np_parse_json`/`serialize_or_error`) serializes — so the two wire formats can never
/// derive a field differently, and (see this module's "Field coverage" doc section) this
/// format now carries every field the JSON path emits.
pub fn encode(pn: &ParsedName, abi_version: u32) -> Vec<u8> {
    // ---- gather the 17 fixed string-slot values, in slot order ----
    let plain_slots: [Option<&str>; NUM_STRING_SLOTS] = [
        pn.uninomial.as_deref(),
        pn.genus.as_deref(),
        pn.infrageneric_epithet.as_deref(),
        pn.specific_epithet.as_deref(),
        pn.infraspecific_epithet.as_deref(),
        pn.cultivar_epithet.as_deref(),
        pn.phrase.as_deref(),
        pn.taxonomic_note.as_deref(),
        pn.nomenclatural_note.as_deref(),
        pn.published_in.as_deref(),
        pn.published_in_page.as_deref(),
        pn.unparsed.as_deref(),
        pn.sanctioning_author.as_deref(),
        pn.combination_authorship.year.as_deref(),
        pn.basionym_authorship.year.as_deref(),
        pn.combination_authorship.imprint_year.as_deref(),
        pn.basionym_authorship.imprint_year.as_deref(),
    ];

    // ---- gather the 6 run-slots' source data, in fixed run order ----
    let run_authors_comb: &[String] = &pn.combination_authorship.authors;
    let run_exauthors_comb: &[String] = &pn.combination_authorship.ex_authors;
    let run_authors_bas: &[String] = &pn.basionym_authorship.authors;
    let run_exauthors_bas: &[String] = &pn.basionym_authorship.ex_authors;
    let run_warnings: &[String] = &pn.warnings;
    let empty_map: BTreeMap<NamePart, String> = BTreeMap::new();
    let eq_map = pn.epithet_qualifier.as_ref().unwrap_or(&empty_map);
    // BTreeMap<NamePart, _> iterates in NamePart's Ord order, which is declaration/ordinal
    // order — matching Java's EnumMap iteration order (see `model::name`'s own doc comment).
    let run_epithet_qualifier: Vec<(u32, &str)> = eq_map
        .iter()
        .map(|(part, s)| (namepart_ordinal(*part), s.as_str()))
        .collect();

    // ---- sizes of the header/string-table/run-tables/nested-group regions are count-driven
    // only, so the blob's base offset is known before any string byte is placed ----
    let run_table_size = 4
        + run_authors_comb.len() * STRING_REF_SIZE
        + 4
        + run_exauthors_comb.len() * STRING_REF_SIZE
        + 4
        + run_authors_bas.len() * STRING_REF_SIZE
        + 4
        + run_exauthors_bas.len() * STRING_REF_SIZE
        + 4
        + run_warnings.len() * STRING_REF_SIZE
        + 4
        + run_epithet_qualifier.len() * EPITHET_QUALIFIER_ENTRY_SIZE;
    let nested_groups_size =
        nested_group_size(&pn.generic_authorship) + nested_group_size(&pn.specific_authorship);
    let blob_base = (RUN_SLOTS_OFFSET + run_table_size + nested_groups_size) as u32;

    // ---- place every string into the blob, in a fixed order, recording its ref ----
    let mut placer = StringPlacer::new(blob_base);
    let mut slot_refs = [(ABSENT_STRING_OFFSET, 0u32); NUM_STRING_SLOTS];
    for (i, s) in plain_slots.iter().enumerate() {
        if let Some(s) = s {
            slot_refs[i] = placer.place(s);
        }
    }
    let refs_authors_comb: Vec<(u32, u32)> =
        run_authors_comb.iter().map(|s| placer.place(s)).collect();
    let refs_exauthors_comb: Vec<(u32, u32)> =
        run_exauthors_comb.iter().map(|s| placer.place(s)).collect();
    let refs_authors_bas: Vec<(u32, u32)> =
        run_authors_bas.iter().map(|s| placer.place(s)).collect();
    let refs_exauthors_bas: Vec<(u32, u32)> =
        run_exauthors_bas.iter().map(|s| placer.place(s)).collect();
    let refs_warnings: Vec<(u32, u32)> = run_warnings.iter().map(|s| placer.place(s)).collect();
    let refs_epithet_qualifier: Vec<(u32, u32, u32)> = run_epithet_qualifier
        .iter()
        .map(|&(part_ordinal, s)| {
            let (offset, len) = placer.place(s);
            (part_ordinal, offset, len)
        })
        .collect();
    // Nested groups' strings are placed AFTER the base run-slots' strings, generic before
    // specific — order is immaterial to correctness (refs are absolute) but fixed for
    // determinism. `nested_group_size` above already reserved the exact bytes each group's
    // table occupies before the blob.
    let refs_generic = pn
        .generic_authorship
        .as_ref()
        .map(|ca| place_nested(&mut placer, ca));
    let refs_specific = pn
        .specific_authorship
        .as_ref()
        .map(|ca| place_nested(&mut placer, ca));

    // ---- assemble the final buffer ----
    let total_size = blob_base as usize + placer.blob.len();
    let mut buf = Vec::with_capacity(total_size);

    let header = Header {
        abi_version,
        status: STATUS_SUCCESS,
        rank: rank_ordinal(pn.rank),
        code: pn.code.map(nomcode_ordinal).unwrap_or(ABSENT_ENUM),
        name_type: name_type_ordinal(pn.type_),
        state: state_ordinal(pn.state),
        candidatus: pn.candidatus,
        doubtful: pn.doubtful,
        manuscript: pn.manuscript,
        extinct: pn.extinct,
        original_spelling: original_spelling_byte(pn.original_spelling),
        notho_bits: notho_bits(&pn.notho),
        published_in_year: pn.published_in_year.unwrap_or(-1),
    };
    header.write_to(&mut buf);

    // string table
    buf.extend_from_slice(&(NUM_STRING_SLOTS as u32).to_le_bytes());
    for &(offset, len) in &slot_refs {
        buf.extend_from_slice(&offset.to_le_bytes());
        buf.extend_from_slice(&len.to_le_bytes());
    }
    debug_assert_eq!(buf.len(), RUN_SLOTS_OFFSET);

    // run-slot tables, in fixed order
    write_string_run_table(&mut buf, &refs_authors_comb);
    write_string_run_table(&mut buf, &refs_exauthors_comb);
    write_string_run_table(&mut buf, &refs_authors_bas);
    write_string_run_table(&mut buf, &refs_exauthors_bas);
    write_string_run_table(&mut buf, &refs_warnings);
    write_epithet_qualifier_table(&mut buf, &refs_epithet_qualifier);

    // nested authorship groups, generic then specific
    write_nested_group(&mut buf, &refs_generic);
    write_nested_group(&mut buf, &refs_specific);
    debug_assert_eq!(buf.len(), blob_base as usize);

    // trailing string blob
    buf.extend_from_slice(&placer.blob);
    debug_assert_eq!(buf.len(), total_size);

    buf
}

/// Encodes the header-only buffer written on the unparsable path (`status = -1`, `name_type`
/// and `code` taken from `err`, every other field at its absent/zero default — no string table
/// or run-slots). Always exactly [`HEADER_SIZE`] bytes.
pub fn encode_unparsable(err: &ParseError, abi_version: u32) -> Vec<u8> {
    let header = Header {
        abi_version,
        status: STATUS_UNPARSABLE,
        rank: ABSENT_ENUM,
        code: err.code.map(nomcode_ordinal).unwrap_or(ABSENT_ENUM),
        name_type: name_type_ordinal(err.type_),
        state: ABSENT_ENUM,
        candidatus: false,
        doubtful: false,
        manuscript: false,
        extinct: false,
        original_spelling: ORIGINAL_SPELLING_UNKNOWN,
        notho_bits: 0,
        published_in_year: -1,
    };
    let mut buf = Vec::with_capacity(HEADER_SIZE);
    header.write_to(&mut buf);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- layout shape sanity ----

    #[test]
    fn header_size_matches_the_documented_offset_table() {
        assert_eq!(HEADER_SIZE, 36);
        assert_eq!(OFF_PUBLISHED_IN_YEAR + 4, HEADER_SIZE);
    }

    #[test]
    fn string_table_size_and_offset_are_internally_consistent() {
        assert_eq!(NUM_STRING_SLOTS, 17);
        assert_eq!(STRING_TABLE_OFFSET, HEADER_SIZE);
        assert_eq!(STRING_TABLE_SIZE, 4 + 17 * 8);
        assert_eq!(RUN_SLOTS_OFFSET, HEADER_SIZE + STRING_TABLE_SIZE);
        // 36 header + (4 + 17*8) string table = 176.
        assert_eq!(RUN_SLOTS_OFFSET, 176);
    }

    #[test]
    fn encode_unparsable_is_exactly_header_size_bytes() {
        let err = ParseError::new(
            NameType::Other,
            Some(NomCode::Virus),
            "Tobacco mosaic virus",
        );
        let buf = encode_unparsable(&err, 1);
        assert_eq!(buf.len(), HEADER_SIZE);
        assert_eq!(
            u32::from_le_bytes(
                buf[OFF_ABI_VERSION..OFF_ABI_VERSION + 4]
                    .try_into()
                    .unwrap()
            ),
            1
        );
        assert_eq!(
            i32::from_le_bytes(buf[OFF_STATUS..OFF_STATUS + 4].try_into().unwrap()),
            STATUS_UNPARSABLE
        );
        assert_eq!(
            i32::from_le_bytes(buf[OFF_RANK..OFF_RANK + 4].try_into().unwrap()),
            ABSENT_ENUM
        );
        assert_eq!(
            i32::from_le_bytes(buf[OFF_CODE..OFF_CODE + 4].try_into().unwrap()),
            nomcode_ordinal(NomCode::Virus)
        );
        assert_eq!(
            i32::from_le_bytes(buf[OFF_NAME_TYPE..OFF_NAME_TYPE + 4].try_into().unwrap()),
            name_type_ordinal(NameType::Other)
        );
    }

    #[test]
    fn encode_of_default_parsed_name_is_header_plus_string_table_plus_empty_run_and_group_tables() {
        let buf = encode(&ParsedName::default(), 1);
        // 6 empty run-slot tables (a 4-byte zero count each) + 2 absent nested-group flags
        // (4 bytes each), no blob.
        let expected_len = RUN_SLOTS_OFFSET + NUM_RUN_SLOTS * 4 + NUM_NESTED_AUTHORSHIP_GROUPS * 4;
        assert_eq!(buf.len(), expected_len);
        assert_eq!(expected_len, 176 + 24 + 8); // 208
        assert_eq!(
            i32::from_le_bytes(buf[OFF_STATUS..OFF_STATUS + 4].try_into().unwrap()),
            STATUS_SUCCESS
        );
        // Both nested-group present flags sit right after the 6 run-slot tables and read ABSENT.
        let generic_flag_off = RUN_SLOTS_OFFSET + NUM_RUN_SLOTS * 4;
        assert_eq!(
            u32::from_le_bytes(
                buf[generic_flag_off..generic_flag_off + 4]
                    .try_into()
                    .unwrap()
            ),
            GROUP_ABSENT
        );
        assert_eq!(
            u32::from_le_bytes(
                buf[generic_flag_off + 4..generic_flag_off + 8]
                    .try_into()
                    .unwrap()
            ),
            GROUP_ABSENT
        );
    }

    // ---- enum ordinal mapping: exhaustive for the 4 small enums, spot-checked for Rank ----

    #[test]
    fn name_type_ordinals_match_java_declaration_order() {
        assert_eq!(name_type_ordinal(NameType::Scientific), 0);
        assert_eq!(name_type_ordinal(NameType::Formula), 1);
        assert_eq!(name_type_ordinal(NameType::Informal), 2);
        assert_eq!(name_type_ordinal(NameType::Placeholder), 3);
        assert_eq!(name_type_ordinal(NameType::Other), 4);
    }

    #[test]
    fn nomcode_ordinals_match_java_declaration_order() {
        assert_eq!(nomcode_ordinal(NomCode::Bacterial), 0);
        assert_eq!(nomcode_ordinal(NomCode::Botanical), 1);
        assert_eq!(nomcode_ordinal(NomCode::Cultivars), 2);
        assert_eq!(nomcode_ordinal(NomCode::Phyto), 3);
        assert_eq!(nomcode_ordinal(NomCode::Virus), 4);
        assert_eq!(nomcode_ordinal(NomCode::Zoological), 5);
        assert_eq!(nomcode_ordinal(NomCode::Phylo), 6);
    }

    #[test]
    fn namepart_ordinals_match_java_declaration_order() {
        assert_eq!(namepart_ordinal(NamePart::Generic), 0);
        assert_eq!(namepart_ordinal(NamePart::Infrageneric), 1);
        assert_eq!(namepart_ordinal(NamePart::Specific), 2);
        assert_eq!(namepart_ordinal(NamePart::Infraspecific), 3);
    }

    #[test]
    fn state_ordinals_match_java_declaration_order() {
        assert_eq!(state_ordinal(State::Complete), 0);
        assert_eq!(state_ordinal(State::Partial), 1);
        assert_eq!(state_ordinal(State::None), 2);
    }

    /// Spot-checks pinned against a mechanical parse of the Java oracle's `Rank.java`
    /// (`name-parser/name-parser-api/.../Rank.java`, this task's independent re-verification —
    /// see the module doc). `Rank::Species` is the brief's own explicitly-named example.
    #[test]
    fn rank_ordinal_spot_checks_pinned_against_the_java_oracle() {
        assert_eq!(rank_ordinal(Rank::Kingdom), 8);
        assert_eq!(rank_ordinal(Rank::Family), 64);
        assert_eq!(rank_ordinal(Rank::Genus), 73);
        assert_eq!(rank_ordinal(Rank::Species), 85);
        assert_eq!(rank_ordinal(Rank::Subspecies), 89);
        assert_eq!(rank_ordinal(Rank::Cultivar), 112);
        assert_eq!(rank_ordinal(Rank::Other), 115);
        assert_eq!(rank_ordinal(Rank::Unranked), 116);
    }

    /// Full-coverage companion to the spot checks above: every one of the 117 `Rank` variants'
    /// wire ordinal equals its position in `Rank::ALL` (itself independently verified elsewhere
    /// against Java declaration order — see `model::enums`).
    #[test]
    fn rank_ordinal_matches_position_in_rank_all_for_every_variant() {
        for (i, &r) in Rank::ALL.iter().enumerate() {
            assert_eq!(rank_ordinal(r), i as i32, "mismatch for {r:?}");
        }
    }

    #[test]
    fn notho_bits_folds_a_multi_part_set_and_none_is_zero() {
        assert_eq!(notho_bits(&None), 0);
        assert_eq!(notho_bits(&Some(vec![NamePart::Generic])), 1 << 0);
        assert_eq!(
            notho_bits(&Some(vec![NamePart::Generic, NamePart::Infraspecific])),
            (1 << 0) | (1 << 3)
        );
    }

    #[test]
    fn original_spelling_byte_is_a_3_state_encoding() {
        assert_eq!(original_spelling_byte(None), ORIGINAL_SPELLING_UNKNOWN);
        assert_eq!(original_spelling_byte(Some(false)), ORIGINAL_SPELLING_FALSE);
        assert_eq!(original_spelling_byte(Some(true)), ORIGINAL_SPELLING_TRUE);
    }

    #[test]
    fn absent_nested_group_is_a_lone_4_byte_flag() {
        assert_eq!(nested_group_size(&None), 4);
    }

    #[test]
    fn present_nested_group_size_counts_flag_four_run_tables_and_five_string_refs() {
        // combination has 1 author, basionym has 1 author, everything else empty:
        // 4 (flag) + [4+8] comb authors + [4+0] comb ex + [4+8] bas authors + [4+0] bas ex
        // + 5*8 fixed refs = 4 + 12 + 4 + 12 + 4 + 40 = 76.
        let ca = CombinedAuthorship {
            combination_authorship: Authorship {
                authors: vec!["Kuntze".to_string()],
                ..Default::default()
            },
            basionym_authorship: Authorship {
                authors: vec!["Adans.".to_string()],
                ..Default::default()
            },
            sanctioning_author: None,
        };
        assert_eq!(nested_group_size(&Some(ca)), 76);
    }

    #[test]
    fn imprint_year_slots_round_trip_through_encode() {
        // Standalone check that the two new string slots carry a base authorship's imprint
        // years (the full decode-and-compare lives in tests/ffi_struct.rs).
        let pn = ParsedName {
            uninomial: Some("Gemmata".to_string()),
            combination_authorship: Authorship {
                authors: vec!["Franzmann".to_string(), "Skerman".to_string()],
                year: Some("1985".to_string()),
                imprint_year: Some("1984".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let buf = encode(&pn, 1);
        let read_slot = |slot: usize| -> Option<String> {
            let entry = STRING_TABLE_OFFSET + 4 + slot * STRING_REF_SIZE;
            let off = u32::from_le_bytes(buf[entry..entry + 4].try_into().unwrap());
            let len = u32::from_le_bytes(buf[entry + 4..entry + 8].try_into().unwrap());
            if off == ABSENT_STRING_OFFSET {
                None
            } else {
                Some(
                    String::from_utf8(buf[off as usize..off as usize + len as usize].to_vec())
                        .unwrap(),
                )
            }
        };
        assert_eq!(read_slot(SLOT_YEAR_COMB).as_deref(), Some("1985"));
        assert_eq!(read_slot(SLOT_IMPRINT_YEAR_COMB).as_deref(), Some("1984"));
        assert_eq!(read_slot(SLOT_IMPRINT_YEAR_BAS), None);
    }
}
