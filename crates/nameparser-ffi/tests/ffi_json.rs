// SPDX-License-Identifier: Apache-2.0

//! Rust-side C-ABI tests for `nameparser-ffi`'s JSON entry point. Calls the `extern "C"`
//! functions directly (the `rlib` crate-type — alongside `cdylib` — exposes them to normal
//! Rust test binaries), exactly as a future non-Rust caller would across the C ABI, just
//! without an actual FFI hop.

use std::ffi::{c_char, CStr, CString};

use nameparser::model::{NomCode, Rank};
use nameparser_ffi::{np_abi_version, np_free, np_parse_json};
use serde_json::Value;

fn cstr(s: &str) -> CString {
    CString::new(s).expect("test literals never contain interior NUL bytes")
}

/// Reads a `*mut c_char` previously returned by `np_parse_json` into an owned `String`,
/// without freeing it — callers still own the pointer afterwards and must free it themselves
/// (mirrors how a real caller must copy the bytes out before calling `np_free`).
unsafe fn read_and_copy(ptr: *mut c_char) -> String {
    assert!(!ptr.is_null(), "np_parse_json must not return null here");
    CStr::from_ptr(ptr)
        .to_str()
        .expect("output is always valid UTF-8 JSON")
        .to_string()
}

#[test]
fn np_abi_version_is_1() {
    assert_eq!(np_abi_version(), 1);
}

#[test]
fn parsable_name_all_null_hints_round_trips_the_expected_fields() {
    let name = cstr("Vulpes vulpes silaceus Miller, 1907");
    let ptr = unsafe {
        np_parse_json(
            name.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    let json = unsafe { read_and_copy(ptr) };

    let v: Value = serde_json::from_str(&json).expect("np_parse_json must return valid JSON");
    assert_eq!(v["genus"], "Vulpes");
    assert_eq!(v["specificEpithet"], "vulpes");
    assert_eq!(v["infraspecificEpithet"], "silaceus");
    assert_eq!(v["combinationAuthorship"]["year"], "1907");

    // np_free must not crash on a real pointer returned by np_parse_json.
    unsafe { np_free(ptr) };
}

#[test]
fn parsable_name_matches_the_core_serialization_byte_for_byte() {
    // The FFI must add nothing beyond nameparser's own ParsedName JSON: parse the SAME name
    // through the core crate directly and compare the two strings exactly.
    let raw = "Vulpes vulpes silaceus Miller, 1907";
    let name = cstr(raw);
    let ptr = unsafe {
        np_parse_json(
            name.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    let via_ffi = unsafe { read_and_copy(ptr) };
    unsafe { np_free(ptr) };

    let via_core = nameparser::parse(raw, None, None, None).expect("must parse");
    let expected = serde_json::to_string(&via_core).unwrap();

    assert_eq!(via_ffi, expected);
}

#[test]
fn unparsable_name_returns_the_error_envelope_with_expected_type_and_code() {
    let name = cstr("Tobacco mosaic virus");
    let ptr = unsafe {
        np_parse_json(
            name.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    let json = unsafe { read_and_copy(ptr) };
    unsafe { np_free(ptr) };

    // Matches testdata/expected-parse.jsonl's Java-oracle row for this exact name, and
    // nameparser-cli's own render_row test for the same error shape.
    assert_eq!(
        json,
        r#"{"error":{"type":"OTHER","code":"VIRUS","message":"Unparsable OTHER name: Tobacco mosaic virus"}}"#
    );

    let v: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["error"]["type"], "OTHER");
    assert_eq!(v["error"]["code"], "VIRUS");
}

#[test]
fn null_name_returns_a_non_null_error_json_not_a_crash() {
    let ptr = unsafe {
        np_parse_json(
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    assert!(!ptr.is_null());
    let json = unsafe { read_and_copy(ptr) };
    let v: Value = serde_json::from_str(&json).unwrap();
    assert!(v.get("error").is_some());
    unsafe { np_free(ptr) };
}

#[test]
fn np_free_of_null_is_a_no_op() {
    // Must not crash/panic/UB.
    unsafe { np_free(std::ptr::null_mut()) };
}

#[test]
fn explicit_rank_and_code_and_authorship_hints_are_honored() {
    let name = cstr("Abies alba");
    let authorship = cstr("Mill.");
    let rank = cstr("SPECIES");
    let code = cstr("BOTANICAL");
    let ptr = unsafe {
        np_parse_json(
            name.as_ptr(),
            authorship.as_ptr(),
            rank.as_ptr(),
            code.as_ptr(),
        )
    };
    let json = unsafe { read_and_copy(ptr) };
    unsafe { np_free(ptr) };

    let v: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["genus"], "Abies");
    assert_eq!(v["specificEpithet"], "alba");
    assert_eq!(v["rank"], "SPECIES");
    assert_eq!(v["code"], "BOTANICAL");
    assert_eq!(v["combinationAuthorship"]["authors"][0], "Mill.");
}

#[test]
fn rank_and_nomcode_from_name_pin_known_values_and_reject_nonsense() {
    assert_eq!(Rank::from_name("SPECIES"), Some(Rank::Species));
    assert_eq!(NomCode::from_name("BOTANICAL"), Some(NomCode::Botanical));
    assert_eq!(Rank::from_name("NONSENSE"), None);
}
