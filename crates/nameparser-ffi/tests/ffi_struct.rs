// SPDX-License-Identifier: Apache-2.0

//! Rust-side C-ABI tests for `nameparser-ffi`'s flat fixed-layout struct entry point,
//! `np_parse_struct`. Implements a small, independent decoder that walks the raw bytes per
//! `layout`'s spec (using its published offset/size/sentinel constants and ordinal-mapping
//! functions as the single source of truth for "what does this byte mean" — but never calling
//! back into `layout::encode` itself, so an offset bug in the encoder isn't invisible to a
//! decoder built the same way) and asserts every decoded field equals the SAME
//! `nameparser::parse_name(...)` call `np_parse_struct` itself parses through.

use std::ffi::CString;

use nameparser::model::{Authorship, CombinedAuthorship, ParsedName};
use nameparser_ffi::layout;
use nameparser_ffi::np_parse_struct;

fn cstr(s: &str) -> CString {
    CString::new(s).expect("test literals never contain interior NUL bytes")
}

/// Generously large — every representative name below encodes in well under this many bytes.
/// The dedicated overflow test below is the only one that deliberately under-sizes the buffer.
const BIG_ENOUGH: usize = 8192;

/// Calls `np_parse_struct` with no authorship/rank/code hints and a generously-sized buffer,
/// asserting success and returning exactly the bytes written (truncated to the reported length).
fn parse_struct_success(name: &str) -> Vec<u8> {
    let name_c = cstr(name);
    let mut buf = vec![0u8; BIG_ENOUGH];
    let ret = unsafe {
        np_parse_struct(
            name_c.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            buf.as_mut_ptr(),
            buf.len(),
        )
    };
    assert!(ret >= 0, "expected success for {name:?}, got {ret}");
    buf.truncate(ret as usize);
    buf
}

// ---- little decoder, mirroring layout.rs's spec by walking raw bytes -------------------------

fn read_u32(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(buf[off..off + 4].try_into().unwrap())
}

fn read_i32(buf: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(buf[off..off + 4].try_into().unwrap())
}

/// Resolves one `(offset, len)` string ref against `buf`, honoring the absent sentinel.
fn read_string_ref(buf: &[u8], offset: u32, len: u32) -> Option<String> {
    if offset == layout::ABSENT_STRING_OFFSET {
        return None;
    }
    let start = offset as usize;
    let end = start + len as usize;
    Some(String::from_utf8(buf[start..end].to_vec()).expect("wire strings are always UTF-8"))
}

/// Reads one run-slot table (`u32 count` then `count` plain string refs) starting at `*cursor`,
/// advancing `*cursor` past it.
fn read_string_run(buf: &[u8], cursor: &mut usize) -> Vec<String> {
    let n = read_u32(buf, *cursor) as usize;
    *cursor += 4;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let offset = read_u32(buf, *cursor);
        let len = read_u32(buf, *cursor + 4);
        *cursor += layout::STRING_REF_SIZE;
        out.push(read_string_ref(buf, offset, len).expect("run-slot entries are never absent"));
    }
    out
}

/// Reads one 8-byte optional string ref at `*cursor`, advancing past it (honors the sentinel).
fn read_opt_string_ref(buf: &[u8], cursor: &mut usize) -> Option<String> {
    let offset = read_u32(buf, *cursor);
    let len = read_u32(buf, *cursor + 4);
    *cursor += layout::STRING_REF_SIZE;
    read_string_ref(buf, offset, len)
}

/// Reads one nested authorship group at `*cursor` (advancing past it), reconstructing the whole
/// `CombinedAuthorship` from its present-flag + 4 run tables + 5 string refs so it can be
/// compared with `==` against the real `ParsedName`'s own `Option<CombinedAuthorship>`.
fn read_nested_group(buf: &[u8], cursor: &mut usize) -> Option<CombinedAuthorship> {
    let present = read_u32(buf, *cursor);
    *cursor += 4;
    if present == layout::GROUP_ABSENT {
        return None;
    }
    assert_eq!(
        present,
        layout::GROUP_PRESENT,
        "nested group present flag must be GROUP_ABSENT (0) or GROUP_PRESENT (1)"
    );
    let authors_comb = read_string_run(buf, cursor);
    let exauthors_comb = read_string_run(buf, cursor);
    let authors_bas = read_string_run(buf, cursor);
    let exauthors_bas = read_string_run(buf, cursor);
    let year_comb = read_opt_string_ref(buf, cursor);
    let imprint_year_comb = read_opt_string_ref(buf, cursor);
    let year_bas = read_opt_string_ref(buf, cursor);
    let imprint_year_bas = read_opt_string_ref(buf, cursor);
    let sanctioning_author = read_opt_string_ref(buf, cursor);
    Some(CombinedAuthorship {
        combination_authorship: Authorship {
            authors: authors_comb,
            ex_authors: exauthors_comb,
            year: year_comb,
            imprint_year: imprint_year_comb,
        },
        basionym_authorship: Authorship {
            authors: authors_bas,
            ex_authors: exauthors_bas,
            year: year_bas,
            imprint_year: imprint_year_bas,
        },
        sanctioning_author,
    })
}

#[derive(Debug)]
struct DecodedHeader {
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

fn decode_header(buf: &[u8]) -> DecodedHeader {
    DecodedHeader {
        abi_version: read_u32(buf, layout::OFF_ABI_VERSION),
        status: read_i32(buf, layout::OFF_STATUS),
        rank: read_i32(buf, layout::OFF_RANK),
        code: read_i32(buf, layout::OFF_CODE),
        name_type: read_i32(buf, layout::OFF_NAME_TYPE),
        state: read_i32(buf, layout::OFF_STATE),
        candidatus: buf[layout::OFF_CANDIDATUS] != 0,
        doubtful: buf[layout::OFF_DOUBTFUL] != 0,
        manuscript: buf[layout::OFF_MANUSCRIPT] != 0,
        extinct: buf[layout::OFF_EXTINCT] != 0,
        original_spelling: buf[layout::OFF_ORIGINAL_SPELLING],
        notho_bits: buf[layout::OFF_NOTHO_BITS],
        published_in_year: read_i32(buf, layout::OFF_PUBLISHED_IN_YEAR),
    }
}

#[derive(Debug)]
struct Decoded {
    header: DecodedHeader,
    strings: [Option<String>; layout::NUM_STRING_SLOTS],
    authors_comb: Vec<String>,
    exauthors_comb: Vec<String>,
    authors_bas: Vec<String>,
    exauthors_bas: Vec<String>,
    warnings: Vec<String>,
    epithet_qualifier: Vec<(u32, String)>,
    generic_authorship: Option<CombinedAuthorship>,
    specific_authorship: Option<CombinedAuthorship>,
}

/// Decodes a full success-path buffer (header + string table + run-slots + nested groups +
/// blob) — mirrors `layout`'s documented traversal exactly: fixed-offset header, fixed-offset
/// 17-slot string table, the 6 run-slots walked sequentially from `RUN_SLOTS_OFFSET`, then the
/// 2 nested authorship groups (generic, specific).
fn decode(buf: &[u8]) -> Decoded {
    let header = decode_header(buf);

    let string_table_count = read_u32(buf, layout::STRING_TABLE_OFFSET) as usize;
    assert_eq!(
        string_table_count,
        layout::NUM_STRING_SLOTS,
        "string table count must always be the fixed slot count"
    );
    let mut strings_vec: Vec<Option<String>> = Vec::with_capacity(layout::NUM_STRING_SLOTS);
    for i in 0..layout::NUM_STRING_SLOTS {
        let entry_off = layout::STRING_TABLE_OFFSET + 4 + i * layout::STRING_REF_SIZE;
        let offset = read_u32(buf, entry_off);
        let len = read_u32(buf, entry_off + 4);
        strings_vec.push(read_string_ref(buf, offset, len));
    }
    let strings: [Option<String>; layout::NUM_STRING_SLOTS] = strings_vec.try_into().unwrap();

    let mut cursor = layout::RUN_SLOTS_OFFSET;
    let authors_comb = read_string_run(buf, &mut cursor);
    let exauthors_comb = read_string_run(buf, &mut cursor);
    let authors_bas = read_string_run(buf, &mut cursor);
    let exauthors_bas = read_string_run(buf, &mut cursor);
    let warnings = read_string_run(buf, &mut cursor);

    let eq_count = read_u32(buf, cursor) as usize;
    cursor += 4;
    let mut epithet_qualifier = Vec::with_capacity(eq_count);
    for _ in 0..eq_count {
        let part_ordinal = read_u32(buf, cursor);
        let offset = read_u32(buf, cursor + 4);
        let len = read_u32(buf, cursor + 8);
        cursor += layout::EPITHET_QUALIFIER_ENTRY_SIZE;
        let s =
            read_string_ref(buf, offset, len).expect("epithet qualifier entries are never absent");
        epithet_qualifier.push((part_ordinal, s));
    }

    let generic_authorship = read_nested_group(buf, &mut cursor);
    let specific_authorship = read_nested_group(buf, &mut cursor);

    Decoded {
        header,
        strings,
        authors_comb,
        exauthors_comb,
        authors_bas,
        exauthors_bas,
        warnings,
        epithet_qualifier,
        generic_authorship,
        specific_authorship,
    }
}

fn assert_abi_version_header(buf: &[u8]) {
    assert_eq!(
        read_u32(buf, layout::OFF_ABI_VERSION),
        nameparser_ffi::np_abi_version()
    );
}

/// The central comparison: every field this wire format carries, decoded from `decoded`, must
/// equal the corresponding field on `pn` — the SAME `ParsedName` `np_parse_struct` itself
/// parsed through. Applied to every representative-name test below, so each test gets full
/// field coverage, not just the one feature it was chosen to exercise.
fn assert_decoded_matches(name: &str, decoded: &Decoded, pn: &ParsedName) {
    assert_eq!(
        decoded.header.status,
        layout::STATUS_SUCCESS,
        "{name}: status"
    );
    assert_eq!(
        decoded.header.rank,
        layout::rank_ordinal(pn.rank),
        "{name}: rank"
    );
    assert_eq!(
        decoded.header.code,
        pn.code
            .map(layout::nomcode_ordinal)
            .unwrap_or(layout::ABSENT_ENUM),
        "{name}: code"
    );
    assert_eq!(
        decoded.header.name_type,
        layout::name_type_ordinal(pn.type_),
        "{name}: name_type"
    );
    assert_eq!(
        decoded.header.state,
        layout::state_ordinal(pn.state),
        "{name}: state"
    );
    assert_eq!(
        decoded.header.candidatus, pn.candidatus,
        "{name}: candidatus"
    );
    assert_eq!(decoded.header.doubtful, pn.doubtful, "{name}: doubtful");
    assert_eq!(
        decoded.header.manuscript, pn.manuscript,
        "{name}: manuscript"
    );
    assert_eq!(decoded.header.extinct, pn.extinct, "{name}: extinct");

    let expected_original_spelling = match pn.original_spelling {
        None => layout::ORIGINAL_SPELLING_UNKNOWN,
        Some(false) => layout::ORIGINAL_SPELLING_FALSE,
        Some(true) => layout::ORIGINAL_SPELLING_TRUE,
    };
    assert_eq!(
        decoded.header.original_spelling, expected_original_spelling,
        "{name}: original_spelling"
    );

    let expected_notho_bits = pn
        .notho
        .as_ref()
        .map(|parts| {
            parts
                .iter()
                .fold(0u8, |acc, &p| acc | (1u8 << layout::namepart_ordinal(p)))
        })
        .unwrap_or(0);
    assert_eq!(
        decoded.header.notho_bits, expected_notho_bits,
        "{name}: notho_bits"
    );
    assert_eq!(
        decoded.header.published_in_year,
        pn.published_in_year.unwrap_or(-1),
        "{name}: published_in_year"
    );

    assert_eq!(
        decoded.strings[layout::SLOT_UNINOMIAL],
        pn.uninomial,
        "{name}: uninomial"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_GENUS],
        pn.genus,
        "{name}: genus"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_INFRAGENERIC],
        pn.infrageneric_epithet,
        "{name}: infrageneric_epithet"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_SPECIFIC],
        pn.specific_epithet,
        "{name}: specific_epithet"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_INFRASPECIFIC],
        pn.infraspecific_epithet,
        "{name}: infraspecific_epithet"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_CULTIVAR],
        pn.cultivar_epithet,
        "{name}: cultivar_epithet"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_PHRASE],
        pn.phrase,
        "{name}: phrase"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_TAXONOMIC_NOTE],
        pn.taxonomic_note,
        "{name}: taxonomic_note"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_NOMENCLATURAL_NOTE],
        pn.nomenclatural_note,
        "{name}: nomenclatural_note"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_PUBLISHED_IN],
        pn.published_in,
        "{name}: published_in"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_PUBLISHED_IN_PAGE],
        pn.published_in_page,
        "{name}: published_in_page"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_UNPARSED],
        pn.unparsed,
        "{name}: unparsed"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_SANCTIONING_AUTHOR],
        pn.sanctioning_author,
        "{name}: sanctioning_author"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_YEAR_COMB],
        pn.combination_authorship.year,
        "{name}: combination year"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_YEAR_BAS],
        pn.basionym_authorship.year,
        "{name}: basionym year"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_IMPRINT_YEAR_COMB],
        pn.combination_authorship.imprint_year,
        "{name}: combination imprint_year"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_IMPRINT_YEAR_BAS],
        pn.basionym_authorship.imprint_year,
        "{name}: basionym imprint_year"
    );

    // The two nested authorship groups reconstruct whole `CombinedAuthorship`s, compared with
    // `==` against the real ParsedName's own Option<CombinedAuthorship> (both derive Eq).
    assert_eq!(
        decoded.generic_authorship, pn.generic_authorship,
        "{name}: generic_authorship"
    );
    assert_eq!(
        decoded.specific_authorship, pn.specific_authorship,
        "{name}: specific_authorship"
    );

    assert_eq!(
        decoded.authors_comb, pn.combination_authorship.authors,
        "{name}: combination authors"
    );
    assert_eq!(
        decoded.exauthors_comb, pn.combination_authorship.ex_authors,
        "{name}: combination ex-authors"
    );
    assert_eq!(
        decoded.authors_bas, pn.basionym_authorship.authors,
        "{name}: basionym authors"
    );
    assert_eq!(
        decoded.exauthors_bas, pn.basionym_authorship.ex_authors,
        "{name}: basionym ex-authors"
    );

    // `warnings` is a Java HashSet on the wire (unordered) — sort both sides before comparing,
    // matching this repo's own golden-test convention for this exact field.
    let mut decoded_warnings = decoded.warnings.clone();
    decoded_warnings.sort();
    let mut expected_warnings = pn.warnings.clone();
    expected_warnings.sort();
    assert_eq!(decoded_warnings, expected_warnings, "{name}: warnings");

    let expected_epithet_qualifier: Vec<(u32, String)> = pn
        .epithet_qualifier
        .as_ref()
        .map(|m| {
            m.iter()
                .map(|(part, s)| (layout::namepart_ordinal(*part), s.clone()))
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        decoded.epithet_qualifier, expected_epithet_qualifier,
        "{name}: epithet_qualifier"
    );
}

// ---- representative-name coverage --------------------------------------------------------

#[test]
fn binomial_with_combination_authorship_and_year() {
    let name = "Vulpes vulpes silaceus Miller, 1907";
    let pn = nameparser::parse_name(name, None, None, None).expect("must parse");
    let buf = parse_struct_success(name);
    assert_abi_version_header(&buf);
    let decoded = decode(&buf);
    assert_decoded_matches(name, &decoded, &pn);

    // Explicit spot checks pinning this test's specific purpose, on top of the generic sweep.
    assert_eq!(
        decoded.strings[layout::SLOT_GENUS].as_deref(),
        Some("Vulpes")
    );
    assert_eq!(
        decoded.strings[layout::SLOT_SPECIFIC].as_deref(),
        Some("vulpes")
    );
    assert_eq!(
        decoded.strings[layout::SLOT_INFRASPECIFIC].as_deref(),
        Some("silaceus")
    );
    assert_eq!(decoded.authors_comb, vec!["Miller".to_string()]);
    assert_eq!(
        decoded.strings[layout::SLOT_YEAR_COMB].as_deref(),
        Some("1907")
    );
}

#[test]
fn autonym_decodes_matching_specific_and_infraspecific_epithets() {
    let name = "Trimezia spathata (Klatt) Baker subsp. spathata";
    let pn = nameparser::parse_name(name, None, None, None).expect("must parse");
    assert!(
        pn.is_autonym(),
        "test name must actually exercise an autonym"
    );
    let buf = parse_struct_success(name);
    assert_abi_version_header(&buf);
    let decoded = decode(&buf);
    assert_decoded_matches(name, &decoded, &pn);

    assert_eq!(
        decoded.strings[layout::SLOT_SPECIFIC],
        decoded.strings[layout::SLOT_INFRASPECIFIC]
    );
    assert_eq!(decoded.authors_comb, vec!["Baker".to_string()]);
    assert_eq!(decoded.authors_bas, vec!["Klatt".to_string()]);
}

#[test]
fn hybrid_name_sets_notho_bits() {
    let name = "Rosa x rugotida Belder & Wijnands cv. 'Wageningen'";
    let pn = nameparser::parse_name(name, None, None, None).expect("must parse");
    assert!(pn.notho.is_some(), "test name must actually exercise notho");
    let buf = parse_struct_success(name);
    assert_abi_version_header(&buf);
    let decoded = decode(&buf);
    assert_decoded_matches(name, &decoded, &pn);

    assert_ne!(
        decoded.header.notho_bits, 0,
        "hybrid name must set at least one notho bit"
    );
    assert_eq!(
        decoded.strings[layout::SLOT_CULTIVAR].as_deref(),
        Some("Wageningen")
    );
}

#[test]
fn name_with_warnings_populates_the_warnings_run_slot() {
    let name = "Acranthera athroophlebia Bremek. var.";
    let pn = nameparser::parse_name(name, None, None, None).expect("must parse");
    assert!(
        !pn.warnings.is_empty(),
        "test name must actually carry a warning"
    );
    let buf = parse_struct_success(name);
    assert_abi_version_header(&buf);
    let decoded = decode(&buf);
    assert_decoded_matches(name, &decoded, &pn);

    assert!(!decoded.warnings.is_empty());
}

#[test]
fn ex_authors_populate_the_exauthors_run_slot() {
    let name = "Adenolepis calva Sch.Bip. ex Miq.";
    let pn = nameparser::parse_name(name, None, None, None).expect("must parse");
    assert!(
        !pn.combination_authorship.ex_authors.is_empty(),
        "test name must actually carry an ex-author"
    );
    let buf = parse_struct_success(name);
    assert_abi_version_header(&buf);
    let decoded = decode(&buf);
    assert_decoded_matches(name, &decoded, &pn);

    assert_eq!(decoded.exauthors_comb, vec!["Sch.Bip.".to_string()]);
    assert_eq!(decoded.authors_comb, vec!["Miq.".to_string()]);
}

/// Bonus coverage beyond the brief's explicit list: `epithetQualifier` is the one run-slot
/// with a distinct entry shape (`namepart_ordinal` + string ref, not just a string ref) — none
/// of the other representative names above ever populate it, so it would otherwise go
/// completely untested.
#[test]
fn epithet_qualifier_run_slot_decodes_namepart_ordinal_and_string() {
    let name = "Turritella aff. adulterata Deshayes 1820-1851";
    let pn = nameparser::parse_name(name, None, None, None).expect("must parse");
    assert!(
        pn.epithet_qualifier.is_some(),
        "test name must actually carry an epithet qualifier"
    );
    let buf = parse_struct_success(name);
    assert_abi_version_header(&buf);
    let decoded = decode(&buf);
    assert_decoded_matches(name, &decoded, &pn);

    assert_eq!(decoded.epithet_qualifier.len(), 1);
}

// ---- generic_authorship nested group (real corpus examples) ----

#[test]
fn generic_authorship_with_combination_and_basionym_authors() {
    // Cordia (Adans.) Kuntze sect. Salimori:
    // genericAuthorship.combination=[Kuntze], genericAuthorship.basionym=[Adans.].
    let name = "Cordia (Adans.) Kuntze sect. Salimori";
    let pn = nameparser::parse_name(name, None, None, None).expect("must parse");
    assert!(
        pn.generic_authorship.is_some(),
        "test name must actually carry a generic authorship"
    );
    let buf = parse_struct_success(name);
    assert_abi_version_header(&buf);
    let decoded = decode(&buf);
    assert_decoded_matches(name, &decoded, &pn);

    let generic = decoded
        .generic_authorship
        .as_ref()
        .expect("generic authorship must decode as present");
    assert_eq!(
        generic.combination_authorship.authors,
        vec!["Kuntze".to_string()]
    );
    assert_eq!(
        generic.basionym_authorship.authors,
        vec!["Adans.".to_string()]
    );
    // The base authorship stays empty for this name — the authorship is entirely in the group.
    assert!(decoded.authors_comb.is_empty());
    assert_eq!(decoded.specific_authorship, None);
}

#[test]
fn generic_authorship_combination_only() {
    // Centaurea L. subg. Jacea: genericAuthorship.combination=[L.].
    let name = "Centaurea L. subg. Jacea";
    let pn = nameparser::parse_name(name, None, None, None).expect("must parse");
    assert!(pn.generic_authorship.is_some());
    let buf = parse_struct_success(name);
    assert_abi_version_header(&buf);
    let decoded = decode(&buf);
    assert_decoded_matches(name, &decoded, &pn);

    let generic = decoded
        .generic_authorship
        .as_ref()
        .expect("generic authorship must decode as present");
    assert_eq!(
        generic.combination_authorship.authors,
        vec!["L.".to_string()]
    );
    assert!(generic.basionym_authorship.authors.is_empty());
}

// ---- specific_authorship nested group (real corpus examples) ----

#[test]
fn specific_authorship_on_a_cultivar() {
    // Acer campestre L. cv. 'Elsrijk' Broerse:
    // specificAuthorship.combination=[L.], base combination=[Broerse].
    let name = "Acer campestre L. cv. 'Elsrijk' Broerse";
    let pn = nameparser::parse_name(name, None, None, None).expect("must parse");
    assert!(
        pn.specific_authorship.is_some(),
        "test name must actually carry a specific authorship"
    );
    let buf = parse_struct_success(name);
    assert_abi_version_header(&buf);
    let decoded = decode(&buf);
    assert_decoded_matches(name, &decoded, &pn);

    let specific = decoded
        .specific_authorship
        .as_ref()
        .expect("specific authorship must decode as present");
    assert_eq!(
        specific.combination_authorship.authors,
        vec!["L.".to_string()]
    );
    // The base combination authorship is a DIFFERENT author than the nested specific one.
    assert_eq!(decoded.authors_comb, vec!["Broerse".to_string()]);
    assert_eq!(decoded.generic_authorship, None);
}

#[test]
fn specific_authorship_on_another_cultivar() {
    // Alnus elliptica Req. cv. 'itolanda' Door.:
    // specificAuthorship.combination=[Req.], base combination=[Door.].
    let name = "Alnus elliptica Req. cv. 'itolanda' Door.";
    let pn = nameparser::parse_name(name, None, None, None).expect("must parse");
    assert!(pn.specific_authorship.is_some());
    let buf = parse_struct_success(name);
    assert_abi_version_header(&buf);
    let decoded = decode(&buf);
    assert_decoded_matches(name, &decoded, &pn);

    let specific = decoded
        .specific_authorship
        .as_ref()
        .expect("specific authorship must decode as present");
    assert_eq!(
        specific.combination_authorship.authors,
        vec!["Req.".to_string()]
    );
    assert_eq!(decoded.authors_comb, vec!["Door.".to_string()]);
}

// ---- imprint years on the base authorship (real corpus examples) ----

#[test]
fn imprint_year_alongside_a_year_on_the_base_combination() {
    // Gemmata Franzmann & Skerman, 1985, 1984:
    // combinationAuthorship year=1985, imprintYear=1984.
    let name = "Gemmata Franzmann & Skerman, 1985, 1984";
    let pn = nameparser::parse_name(name, None, None, None).expect("must parse");
    assert_eq!(
        pn.combination_authorship.imprint_year.as_deref(),
        Some("1984"),
        "test name must actually carry an imprint year"
    );
    let buf = parse_struct_success(name);
    assert_abi_version_header(&buf);
    let decoded = decode(&buf);
    assert_decoded_matches(name, &decoded, &pn);

    assert_eq!(
        decoded.strings[layout::SLOT_YEAR_COMB].as_deref(),
        Some("1985")
    );
    assert_eq!(
        decoded.strings[layout::SLOT_IMPRINT_YEAR_COMB].as_deref(),
        Some("1984")
    );
}

#[test]
fn bracketed_imprint_year_with_no_regular_year() {
    // Anthoscopus Cabanis [1851]: combinationAuthorship imprintYear=1851, no year.
    let name = "Anthoscopus Cabanis [1851]";
    let pn = nameparser::parse_name(name, None, None, None).expect("must parse");
    assert_eq!(
        pn.combination_authorship.imprint_year.as_deref(),
        Some("1851")
    );
    assert_eq!(pn.combination_authorship.year, None);
    let buf = parse_struct_success(name);
    assert_abi_version_header(&buf);
    let decoded = decode(&buf);
    assert_decoded_matches(name, &decoded, &pn);

    assert_eq!(decoded.strings[layout::SLOT_YEAR_COMB], None);
    assert_eq!(
        decoded.strings[layout::SLOT_IMPRINT_YEAR_COMB].as_deref(),
        Some("1851")
    );
}

#[test]
fn unparsable_virus_returns_minus_one_with_header_and_name() {
    let name = "Tobacco mosaic virus";
    let err = nameparser::parse_name(name, None, None, None).expect_err("must be unparsable");

    let name_c = cstr(name);
    let mut buf = vec![0u8; 256];
    let ret = unsafe {
        np_parse_struct(
            name_c.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            buf.as_mut_ptr(),
            buf.len(),
        )
    };
    assert_eq!(ret, -1);

    let header = decode_header(&buf);
    assert_eq!(header.abi_version, nameparser_ffi::np_abi_version());
    assert_eq!(header.status, layout::STATUS_UNPARSABLE);
    assert_eq!(header.name_type, layout::name_type_ordinal(err.type_));
    assert_eq!(
        header.code,
        err.code
            .map(layout::nomcode_ordinal)
            .unwrap_or(layout::ABSENT_ENUM)
    );
    // rank/state have no source on the unparsable path (ParseError carries neither) — both
    // must be the absent sentinel, not left over from some other value.
    assert_eq!(header.rank, layout::ABSENT_ENUM);
    assert_eq!(header.state, layout::ABSENT_ENUM);
    // ABI 3: the error name rides the wire (here byte-identical to the input).
    assert_eq!(unparsable_wire_name(&buf), err.name);
}

/// Reads the ABI-3 unparsable-path error name (`u32` length + UTF-8 bytes at
/// [`layout::OFF_UNPARSABLE_NAME_LEN`]) out of a `-1` result buffer.
fn unparsable_wire_name(buf: &[u8]) -> String {
    let len = u32::from_le_bytes(
        buf[layout::OFF_UNPARSABLE_NAME_LEN..layout::OFF_UNPARSABLE_NAME_LEN + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    let start = layout::OFF_UNPARSABLE_NAME_LEN + 4;
    String::from_utf8(buf[start..start + len].to_vec()).unwrap()
}

#[test]
fn unparsable_otu_wire_name_is_the_canonicalized_uppercase_form() {
    // The whole point of ABI 3: the OTU/SH canonicalization the core applies to `ParseError.name`
    // must survive the FFI. Input is lowercase; the wire name is the uppercased canonical form.
    let name = "sh19186714.17fu";
    let err = nameparser::parse_name(name, None, None, None).expect_err("must be unparsable");
    assert_eq!(
        err.name, "SH19186714.17FU",
        "core canonicalizes SH ids to uppercase"
    );

    let name_c = cstr(name);
    let mut buf = vec![0u8; 256];
    let ret = unsafe {
        np_parse_struct(
            name_c.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            buf.as_mut_ptr(),
            buf.len(),
        )
    };
    assert_eq!(ret, -1);
    assert_eq!(unparsable_wire_name(&buf), "SH19186714.17FU");
}

#[test]
fn overflow_path_reports_needed_size_then_succeeds_with_exactly_that_buffer() {
    let name = "Vulpes vulpes silaceus Miller, 1907";
    let name_c = cstr(name);

    // First call: out_cap = 0 and out = null — a zero-length "write" must never dereference
    // `out`, and must report a usable `needed` size via the overflow encoding.
    let ret0 = unsafe {
        np_parse_struct(
            name_c.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
            0,
        )
    };
    assert!(ret0 <= -3, "expected an overflow code (<= -3), got {ret0}");
    let needed = (-ret0 - 3) as usize;
    assert!(needed > 0);

    // Second call: a buffer of EXACTLY `needed` bytes must succeed and decode correctly.
    let mut buf = vec![0u8; needed];
    let ret1 = unsafe {
        np_parse_struct(
            name_c.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            buf.as_mut_ptr(),
            buf.len(),
        )
    };
    assert_eq!(ret1, needed as i64);
    buf.truncate(ret1 as usize);

    let pn = nameparser::parse_name(name, None, None, None).expect("must parse");
    let decoded = decode(&buf);
    assert_decoded_matches(name, &decoded, &pn);
}

#[test]
fn np_abi_version_is_4() {
    assert_eq!(nameparser_ffi::np_abi_version(), 4);
}
